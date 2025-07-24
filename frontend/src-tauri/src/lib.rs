use std::fs;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use serde::{Deserialize, Serialize};

// Declare audio module
pub mod audio;
pub mod ollama;
pub mod analytics;
pub mod api;

use audio::{
    default_input_device, default_output_device, AudioStream,
    encode_single_audio,
};
use ollama::{OllamaModel};
use analytics::{AnalyticsClient, AnalyticsConfig};
use tauri::{Runtime, AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use log::{info as log_info, error as log_error, debug as log_debug};
use reqwest::multipart::{Form, Part};

static RECORDING_FLAG: AtomicBool = AtomicBool::new(false);
static mut MIC_BUFFER: Option<Arc<Mutex<Vec<f32>>>> = None;
static mut SYSTEM_BUFFER: Option<Arc<Mutex<Vec<f32>>>> = None;
static mut MIC_STREAM: Option<Arc<AudioStream>> = None;
static mut SYSTEM_STREAM: Option<Arc<AudioStream>> = None;
static mut IS_RUNNING: Option<Arc<AtomicBool>> = None;
static mut RECORDING_START_TIME: Option<std::time::Instant> = None;
static mut TRANSCRIPTION_TASK: Option<tokio::task::JoinHandle<()>> = None;
static mut ANALYTICS_CLIENT: Option<Arc<AnalyticsClient>> = None;
static mut ERROR_EVENT_EMITTED: bool = false;

// Audio configuration constants
const CHUNK_DURATION_MS: u32 = 30000; // 30 seconds per chunk for better sentence processing
const WHISPER_SAMPLE_RATE: u32 = 16000; // Whisper's required sample rate
const WAV_SAMPLE_RATE: u32 = 44100; // WAV file sample rate
const WAV_CHANNELS: u16 = 2; // Stereo for WAV files
const WHISPER_CHANNELS: u16 = 1; // Mono for Whisper API
const SENTENCE_TIMEOUT_MS: u64 = 1000; // Emit incomplete sentence after 1 second of silence
const MIN_CHUNK_DURATION_MS: u32 = 2000; // Minimum duration before sending chunk
const MIN_RECORDING_DURATION_MS: u64 = 2000; // 2 seconds minimum

#[derive(Debug, Deserialize)]
struct RecordingArgs {
    save_path: String,
}

#[derive(Debug, Serialize, Clone)]
struct TranscriptUpdate {
    text: String,
    timestamp: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptSegment {
    text: String,
    t0: f32,
    t1: f32,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    segments: Vec<TranscriptSegment>,
    buffer_size_ms: i32,
}

// Helper struct to accumulate transcript segments
#[derive(Debug)]
struct TranscriptAccumulator {
    current_sentence: String,
    sentence_start_time: f32,
    last_update_time: std::time::Instant,
    last_segment_hash: u64,
}

impl TranscriptAccumulator {
    fn new() -> Self {
        Self {
            current_sentence: String::new(),
            sentence_start_time: 0.0,
            last_update_time: std::time::Instant::now(),
            last_segment_hash: 0,
        }
    }

    fn add_segment(&mut self, segment: &TranscriptSegment) -> Option<TranscriptUpdate> {
        log_info!("Processing new transcript segment: {:?}", segment);
        
        // Update the last update time
        self.last_update_time = std::time::Instant::now();

        // Clean up the text (remove [BLANK_AUDIO], [AUDIO OUT] and trim)
        let clean_text = segment.text
            .replace("[BLANK_AUDIO]", "")
            .replace("[AUDIO OUT]", "")
            .trim()
            .to_string();
            
        if !clean_text.is_empty() {
            log_info!("Clean transcript text: {}", clean_text);
        }

        // Skip empty segments or very short segments (less than 1 second)
        if clean_text.is_empty() || (segment.t1 - segment.t0) < 1.0 {
            return None;
        }

        // Calculate hash of this segment to detect duplicates
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        segment.text.hash(&mut hasher);
        segment.t0.to_bits().hash(&mut hasher);
        segment.t1.to_bits().hash(&mut hasher);
        let segment_hash = hasher.finish();

        // Skip if this is a duplicate segment
        if segment_hash == self.last_segment_hash {
            return None;
        }
        self.last_segment_hash = segment_hash;

        // If this is the start of a new sentence, store the start time
        if self.current_sentence.is_empty() {
            self.sentence_start_time = segment.t0;
        }

        // Add the new text with proper spacing
        if !self.current_sentence.is_empty() && !self.current_sentence.ends_with(' ') {
            self.current_sentence.push(' ');
        }
        self.current_sentence.push_str(&clean_text);

        // Check if we have a complete sentence
        if clean_text.ends_with('.') || clean_text.ends_with('?') || clean_text.ends_with('!') {
            let sentence = std::mem::take(&mut self.current_sentence);
            let update = TranscriptUpdate {
                text: sentence.trim().to_string(),
                timestamp: format!("{:.1} - {:.1}", self.sentence_start_time, segment.t1),
                source: "Mixed Audio".to_string(),
            };
            log_info!("Generated transcript update: {:?}", update);
            Some(update)
        } else {
            None
        }
    }

    fn check_timeout(&mut self) -> Option<TranscriptUpdate> {
        if !self.current_sentence.is_empty() && 
           self.last_update_time.elapsed() > Duration::from_millis(SENTENCE_TIMEOUT_MS) {
            let sentence = std::mem::take(&mut self.current_sentence);
            let current_time = self.sentence_start_time + (SENTENCE_TIMEOUT_MS as f32 / 1000.0);
            let update = TranscriptUpdate {
                text: sentence.trim().to_string(),
                timestamp: format!("{:.1} - {:.1}", self.sentence_start_time, current_time),
                source: "Mixed Audio".to_string(),
            };
            Some(update)
        } else {
            None
        }
    }
}

async fn send_audio_chunk(chunk: Vec<f32>, client: &reqwest::Client, stream_url: &str) -> Result<TranscriptResponse, String> {
    log_debug!("Preparing to send audio chunk of size: {}", chunk.len());
    
    // Convert f32 samples to bytes
    let bytes: Vec<u8> = chunk.iter()
        .flat_map(|&sample| {
            let clamped = sample.max(-1.0).min(1.0);
            clamped.to_le_bytes().to_vec()
        })
        .collect();
    
    // Retry configuration
    let max_retries = 3;
    let mut retry_count = 0;
    let mut last_error = String::new();

    while retry_count <= max_retries {
        if retry_count > 0 {
            // Exponential backoff: wait 2^retry_count * 100ms
            let delay = Duration::from_millis(100 * (2_u64.pow(retry_count as u32)));
            log::info!("Retry attempt {} of {}. Waiting {:?} before retry...", 
                      retry_count, max_retries, delay);
            tokio::time::sleep(delay).await;
        }

        // Create fresh multipart form for each attempt since Form can't be reused
        let part = Part::bytes(bytes.clone())
            .file_name("audio.raw")
            .mime_str("audio/x-raw")
            .unwrap();
        let form = Form::new().part("audio", part);

        match client.post(stream_url)
            .multipart(form)
            .send()
            .await {
                Ok(response) => {
                    match response.json::<TranscriptResponse>().await {
                        Ok(transcript) => return Ok(transcript),
                        Err(e) => {
                            last_error = e.to_string();
                            log::error!("Failed to parse response: {}", last_error);
                        }
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    log::error!("Request failed: {}", last_error);
                }
            }

        retry_count += 1;
    }

    Err(format!("Failed after {} retries. Last error: {}", max_retries, last_error))
}

#[tauri::command]
async fn start_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    log_info!("Attempting to start recording...");
    
    if is_recording() {
        log_error!("Recording already in progress");
        return Err("Recording already in progress".to_string());
    }

    // Stop any existing transcription task first
    unsafe {
        if let Some(task) = TRANSCRIPTION_TASK.take() {
            log_info!("Stopping existing transcription task...");
            task.abort();
            // Give it a moment to clean up
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Initialize recording flag and buffers
    RECORDING_FLAG.store(true, Ordering::SeqCst);
    log_info!("Recording flag set to true");
    
    // Reset error event flag for new recording session
    unsafe {
        ERROR_EVENT_EMITTED = false;
    }


    // Store recording start time
    unsafe {
        RECORDING_START_TIME = Some(std::time::Instant::now());
    }

    // Initialize audio buffers
    unsafe {
        MIC_BUFFER = Some(Arc::new(Mutex::new(Vec::new())));
        SYSTEM_BUFFER = Some(Arc::new(Mutex::new(Vec::new())));
        log_info!("Initialized audio buffers");
    }
    
    // Get default devices
    let mic_device = Arc::new(default_input_device().map_err(|e| {
        log_error!("Failed to get default input device: {}", e);
        e.to_string()
    })?);
    
    let system_device = Arc::new(default_output_device().map_err(|e| {
        log_error!("Failed to get default output device: {}", e);
        e.to_string()
    })?);
    
    // Create audio streams
    let is_running = Arc::new(AtomicBool::new(true));
    
    // Create microphone stream
    let mic_stream = AudioStream::from_device(mic_device.clone(), is_running.clone())
        .await
        .map_err(|e| {
            log_error!("Failed to create microphone stream: {}", e);
            e.to_string()
        })?;
    let mic_stream = Arc::new(mic_stream);
    
    // Create system audio stream
    let system_stream = AudioStream::from_device(system_device.clone(), is_running.clone())
        .await
        .map_err(|e| {
            log_error!("Failed to create system stream: {}", e);
            e.to_string()
        })?;
    let system_stream = Arc::new(system_stream);

    unsafe {
        MIC_STREAM = Some(mic_stream.clone());
        SYSTEM_STREAM = Some(system_stream.clone());
        IS_RUNNING = Some(is_running.clone());
    }
    
    // Create HTTP client for transcription
    let client = reqwest::Client::new();
    
    // Get the transcript server URL from the store
    let store = app.store("store.json").map_err(|e| e.to_string())?;
    let stream_url = match store.get("transcriptServerUrl") {
        Some(url) => url.as_str().unwrap_or("http://127.0.0.1:8178/stream").to_string(),
        None => "http://127.0.0.1:8178/stream".to_string(),
    };
    log_info!("Using stream URL: {}", stream_url);

    // Start transcription task
    let app_handle = app.clone();
    
    // Create audio receivers
    let mut mic_receiver = mic_stream.subscribe().await;
    let mut mic_receiver_clone = mic_receiver.resubscribe();
    let mut system_receiver = system_stream.subscribe().await;
    
    // Create debug directory for chunks in temp
    let temp_dir = std::env::temp_dir();
    log_info!("System temp directory: {:?}", temp_dir);
    let debug_dir = temp_dir.join("meeting_minutes_debug");
    log_info!("Full debug directory path: {:?}", debug_dir);
    
    // Create directory and check if it exists
    fs::create_dir_all(&debug_dir).map_err(|e| {
        log_error!("Failed to create debug directory: {}", e);
        e.to_string()
    })?;
    
    if debug_dir.exists() {
        log_info!("Debug directory successfully created and exists");
    } else {
        log_error!("Failed to create debug directory - path does not exist after creation");
    }
    
    let chunk_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let chunk_counter_clone = chunk_counter.clone();
    
    // Create transcript accumulator
    let mut accumulator = TranscriptAccumulator::new();
    
    let device_config = mic_stream.device_config.clone();
    let _device_name = mic_stream.device.to_string();
    let sample_rate = device_config.sample_rate().0;
    let channels = device_config.channels();
    
    // Store the transcription task handle globally
    unsafe {
        TRANSCRIPTION_TASK = Some(tokio::spawn(async move {
            log_info!("Transcription task started");
            let chunk_samples = (WHISPER_SAMPLE_RATE as f32 * (CHUNK_DURATION_MS as f32 / 1000.0)) as usize;
            let min_samples = (WHISPER_SAMPLE_RATE as f32 * (MIN_CHUNK_DURATION_MS as f32 / 1000.0)) as usize;
            let mut current_chunk: Vec<f32> = Vec::with_capacity(chunk_samples);
            let mut last_chunk_time = std::time::Instant::now();
            
            log_info!("Mic config: {} Hz, {} channels", sample_rate, channels);
            
            // Use the global IS_RUNNING flag instead of local is_running
            while unsafe { 
                if let Some(is_running) = &IS_RUNNING {
                    let running = is_running.load(Ordering::SeqCst);
                    if !running {
                        log_debug!("Transcription task: IS_RUNNING is false, exiting loop");
                    }
                    running
                } else {
                    log_debug!("Transcription task: IS_RUNNING is None, exiting loop");
                    false
                }
            } {
                // Check for timeout on current sentence
                if let Some(update) = accumulator.check_timeout() {
                    if let Err(e) = app_handle.emit("transcript-update", update) {
                        log_error!("Failed to send timeout transcript update: {}", e);
                    }
                }

                // Collect audio samples
                let mut new_samples = Vec::new();
                let mut mic_samples = Vec::new();
                let mut system_samples = Vec::new();
                
                // Get microphone samples
                let mut got_mic_samples = false;
                while let Ok(chunk) = mic_receiver_clone.try_recv() {
                    got_mic_samples = true;
                    log_debug!("Received {} mic samples", chunk.len());
                    let chunk_clone = chunk.clone();
                    mic_samples.extend(chunk);
                    
                    // Store in global buffer
                    unsafe {
                        if let Some(buffer) = &MIC_BUFFER {
                            if let Ok(mut guard) = buffer.lock() {
                                guard.extend(chunk_clone);
                            }
                        }
                    }
                }
                // If we didn't get any samples, try to resubscribe to clear any backlog
                if !got_mic_samples {
                    log_debug!("No mic samples received, resubscribing to clear channel");
                    mic_receiver_clone = mic_stream.subscribe().await;
                }
                
                // Get system audio samples
                let mut got_system_samples = false;
                while let Ok(chunk) = system_receiver.try_recv() {
                    got_system_samples = true;
                    log_debug!("Received {} system samples", chunk.len());
                    let chunk_clone = chunk.clone();
                    system_samples.extend(chunk);
                    
                    // Store in global buffer
                    unsafe {
                        if let Some(buffer) = &SYSTEM_BUFFER {
                            if let Ok(mut guard) = buffer.lock() {
                                guard.extend(chunk_clone);
                            }
                        }
                    }
                }
                // If we didn't get any samples, try to resubscribe to clear any backlog
                if !got_system_samples {
                    log_debug!("No system samples received, resubscribing to clear channel");
                    system_receiver = system_stream.subscribe().await;
                }
                
                // Mix samples with debug info
                let max_len = mic_samples.len().max(system_samples.len());
                for i in 0..max_len {
                    let mic_sample = if i < mic_samples.len() { mic_samples[i] } else { 0.0 };
                    let system_sample = if i < system_samples.len() { system_samples[i] } else { 0.0 };
                    // Increase mic sensitivity by giving it more weight in the mix (80% mic, 20% system)
                    new_samples.push((mic_sample * 0.7) + (system_sample * 0.3));
                }
                
                log_debug!("Mixed {} samples", new_samples.len());
                
                // Add samples to current chunk
                for sample in new_samples {
                    current_chunk.push(sample);
                }
                
                // Check if we should send the chunk based on size or time
                let should_send = current_chunk.len() >= chunk_samples || 
                                (current_chunk.len() >= min_samples && 
                                 last_chunk_time.elapsed() >= Duration::from_millis(CHUNK_DURATION_MS as u64));
                
                if should_send {
                    log_info!("Should send chunk with {} samples", current_chunk.len());
                    let chunk_to_send = current_chunk.clone();
                    current_chunk.clear();
                    last_chunk_time = std::time::Instant::now();
                    
                    // Save debug chunks
                    let chunk_num = chunk_counter_clone.fetch_add(1, Ordering::SeqCst);
                    log_info!("Processing chunk {}", chunk_num);
                    
                    // Process chunk for Whisper API
                    let whisper_samples = if sample_rate != WHISPER_SAMPLE_RATE {
                        log_debug!("Resampling audio from {} to {}", sample_rate, WHISPER_SAMPLE_RATE);
                        resample_audio(
                            &chunk_to_send,
                            sample_rate,
                            WHISPER_SAMPLE_RATE,
                        )
                    } else {
                        chunk_to_send
                    };

                    // Send chunk for transcription
                    match send_audio_chunk(whisper_samples, &client, &stream_url).await {
                        Ok(response) => {
                            log_info!("Received {} transcript segments", response.segments.len());
                            for segment in response.segments {
                                log_info!("Processing segment: {} ({:.1}s - {:.1}s)", 
                                         segment.text.trim(), segment.t0, segment.t1);
                                // Add segment to accumulator and check for complete sentence
                                if let Some(update) = accumulator.add_segment(&segment) {
                                    // Emit the update
                                    if let Err(e) = app_handle.emit("transcript-update", update) {
                                        log_error!("Failed to emit transcript update: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log_error!("Transcription error: {}", e);
                            
                            // Check if this is a repeated error (could indicate server is down)
                            static mut ERROR_COUNT: u32 = 0;
                            static mut LAST_ERROR_TIME: Option<std::time::Instant> = None;
                            
                            unsafe {
                                let now = std::time::Instant::now();
                                if let Some(last_time) = LAST_ERROR_TIME {
                                    if now.duration_since(last_time).as_secs() < 30 {
                                        // Error within 30 seconds, increment counter
                                        ERROR_COUNT += 1;
                                        log_info!("Incremented ERROR_COUNT: {}", ERROR_COUNT);
                                    } else {
                                        // Reset counter if more than 30 seconds have passed
                                        ERROR_COUNT = 1;
                                        log_info!("Reset ERROR_COUNT to 1");
                                    }
                                } else {
                                    ERROR_COUNT = 1;
                                    log_info!("Initialized ERROR_COUNT to 1");
                                }
                                LAST_ERROR_TIME = Some(now);
                                
                                // If we have 1 error and haven't emitted event yet, emit event and stop recording
                                if ERROR_COUNT == 1 && !ERROR_EVENT_EMITTED {
                                    log_error!("Too many transcription errors ({}), stopping recording", ERROR_COUNT);
                                    log_info!("Emitting transcript-error event to frontend");
                                    // Determine specific error type for better user feedback
                                    let error_msg = if e.contains("Failed to connect") || e.contains("Connection refused") {
                                        "Transcription service is not available. Please check if the server is running.".to_string()
                                    } else if e.contains("timeout") {
                                        "Transcription service is not responding. Please check your connection.".to_string()
                                    } else {
                                        format!("Transcription service error: {}", e)
                                    };
                                    if let Err(emit_err) = app_handle.emit("transcript-error", error_msg) {
                                        log_error!("Failed to emit transcript error: {}", emit_err);
                                    }
                                    ERROR_EVENT_EMITTED = true; // Mark that we've emitted the event
                                    log_info!("Set RECORDING_FLAG and IS_RUNNING to false due to error");
                                    // Stop recording
                                    RECORDING_FLAG.store(false, Ordering::SeqCst);
                                    if let Some(is_running) = &IS_RUNNING {
                                        is_running.store(false, Ordering::SeqCst);
                                    }
                                    // Reset error counters
                                    ERROR_COUNT = 0;
                                    LAST_ERROR_TIME = None;
                                    ERROR_EVENT_EMITTED = false; // Reset for next recording session
                                    return; // Exit the transcription loop
                                }
                            }
                        }
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            
            // Emit any remaining transcript when recording stops
            if let Some(update) = accumulator.check_timeout() {
                if let Err(e) = app_handle.emit("transcript-update", update) {
                    log_error!("Failed to send final transcript update: {}", e);
                }
            }
            
            log_info!("Transcription task ended");
        }));
    }
    
    Ok(())
}

#[tauri::command]
async fn stop_recording(args: RecordingArgs) -> Result<(), String> {
    log_info!("Attempting to stop recording...");
    
    // Only check recording state if we haven't already started stopping
    if !RECORDING_FLAG.load(Ordering::SeqCst) {
        log_info!("Recording is already stopped");
        return Ok(());
    }

    // Check minimum recording duration
    let elapsed_ms = unsafe {
        RECORDING_START_TIME
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0)
    };

    if elapsed_ms < MIN_RECORDING_DURATION_MS {
        let remaining = MIN_RECORDING_DURATION_MS - elapsed_ms;
        log_info!("Waiting for minimum recording duration ({} ms remaining)...", remaining);
        tokio::time::sleep(Duration::from_millis(remaining)).await;
    }

    // First set the recording flag to false to prevent new data from being processed
    RECORDING_FLAG.store(false, Ordering::SeqCst);
    log_info!("Recording flag set to false");
    
    unsafe {
        // Stop the running flag for audio streams first
        if let Some(is_running) = &IS_RUNNING {
            // Set running flag to false first to stop the tokio task
            is_running.store(false, Ordering::SeqCst);
            log_info!("Set recording flag to false, waiting for streams to stop...");
            
            // Stop the transcription task
            if let Some(task) = TRANSCRIPTION_TASK.take() {
                log_info!("Stopping transcription task...");
                task.abort();
                // Give the task time to clean up
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            
            // Give the tokio task time to finish and release its references
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Stop mic stream if it exists
            if let Some(mic_stream) = &MIC_STREAM {
                log_info!("Stopping microphone stream...");
                if let Err(e) = mic_stream.stop().await {
                    log_error!("Error stopping mic stream: {}", e);
                } else {
                    log_info!("Microphone stream stopped successfully");
                }
            }
            
            // Stop system stream if it exists
            if let Some(system_stream) = &SYSTEM_STREAM {
                log_info!("Stopping system stream...");
                if let Err(e) = system_stream.stop().await {
                    log_error!("Error stopping system stream: {}", e);
                } else {
                    log_info!("System stream stopped successfully");
                }
            }
            
            // Clear the stream references
            MIC_STREAM = None;
            SYSTEM_STREAM = None;
            IS_RUNNING = None;
            TRANSCRIPTION_TASK = None;
            
            // Give streams time to fully clean up
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    // Get final buffers
    let mic_data = unsafe {
        if let Some(buffer) = &MIC_BUFFER {
            if let Ok(guard) = buffer.lock() {
                guard.clone()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };
    
    let system_data = unsafe {
        if let Some(buffer) = &SYSTEM_BUFFER {
            if let Ok(guard) = buffer.lock() {
                guard.clone()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };
    /*
    // Mix the audio and convert to 16-bit PCM
    let max_len = mic_data.len().max(system_data.len());
    let mut mixed_data = Vec::with_capacity(max_len);
    
    for i in 0..max_len {
        let mic_sample = if i < mic_data.len() { mic_data[i] } else { 0.0 };
        let system_sample = if i < system_data.len() { system_data[i] } else { 0.0 };
        mixed_data.push((mic_sample + system_sample) * 0.5);
    }

    if mixed_data.is_empty() {
        log_error!("No audio data captured");
        return Err("No audio data captured".to_string());
    }
    
    log_info!("Mixed {} audio samples", mixed_data.len());
    
    // Resample the audio to 16kHz for Whisper compatibility
    let original_sample_rate = 48000; // Assuming original sample rate is 48kHz
    if original_sample_rate != WHISPER_SAMPLE_RATE {
        log_info!("Resampling audio from {} Hz to {} Hz for Whisper compatibility", 
                 original_sample_rate, WHISPER_SAMPLE_RATE);
        mixed_data = resample_audio(&mixed_data, original_sample_rate, WHISPER_SAMPLE_RATE);
        log_info!("Resampled to {} samples", mixed_data.len());
    }
    
    // Convert to 16-bit PCM samples
    let mut bytes = Vec::with_capacity(mixed_data.len() * 2);
    for &sample in mixed_data.iter() {
        let value = (sample.max(-1.0).min(1.0) * 32767.0) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    
    log_info!("Converted to {} bytes of PCM data", bytes.len());

    // Create WAV header
    let data_size = bytes.len() as u32;
    let file_size = 36 + data_size;
    let sample_rate = WHISPER_SAMPLE_RATE; // Use Whisper's required sample rate (16000 Hz)
    let channels = 1u16; // Mono
    let bits_per_sample = 16u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    
    let mut wav_file = Vec::with_capacity(44 + bytes.len());
    
    // RIFF header
    wav_file.extend_from_slice(b"RIFF");
    wav_file.extend_from_slice(&file_size.to_le_bytes());
    wav_file.extend_from_slice(b"WAVE");
    
    // fmt chunk
    wav_file.extend_from_slice(b"fmt ");
    wav_file.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav_file.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
    wav_file.extend_from_slice(&channels.to_le_bytes()); // num channels
    wav_file.extend_from_slice(&sample_rate.to_le_bytes()); // sample rate
    wav_file.extend_from_slice(&byte_rate.to_le_bytes()); // byte rate
    wav_file.extend_from_slice(&block_align.to_le_bytes()); // block align
    wav_file.extend_from_slice(&bits_per_sample.to_le_bytes()); // bits per sample
    
    // data chunk
    wav_file.extend_from_slice(b"data");
    wav_file.extend_from_slice(&data_size.to_le_bytes());
    wav_file.extend_from_slice(&bytes);
    
    log_info!("Created WAV file with {} bytes total", wav_file.len());
    */
    // Create the save directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&args.save_path).parent() {
        if !parent.exists() {
            log_info!("Creating directory: {:?}", parent);
            if let Err(e) = std::fs::create_dir_all(parent) {
                let err_msg = format!("Failed to create save directory: {}", e);
                log_error!("{}", err_msg);
                return Err(err_msg);
            }
        }
    }

    /*
    // Save the recording
    log_info!("Saving recording to: {}", args.save_path);
    match fs::write(&args.save_path, wav_file) {
        Ok(_) => log_info!("Successfully saved recording"),
        Err(e) => {
            let err_msg = format!("Failed to save recording: {}", e);
            log_error!("{}", err_msg);
            return Err(err_msg);
        }
    }
    */
    
    // Clean up
    unsafe {
        MIC_BUFFER = None;
        SYSTEM_BUFFER = None;
        MIC_STREAM = None;
        SYSTEM_STREAM = None;
        IS_RUNNING = None;
        RECORDING_START_TIME = None;
        TRANSCRIPTION_TASK = None;
    }
    
    Ok(())
}

#[tauri::command]
fn is_recording() -> bool {
    RECORDING_FLAG.load(Ordering::SeqCst)
}

#[tauri::command]
fn read_audio_file(file_path: String) -> Result<Vec<u8>, String> {
    match std::fs::read(&file_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read audio file: {}", e))
    }
}

#[tauri::command]
async fn save_transcript(file_path: String, content: String) -> Result<(), String> {
    log::info!("Saving transcript to: {}", file_path);

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Write content to file
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    log::info!("Transcript saved successfully");
    Ok(())
}

// Analytics commands
#[tauri::command]
async fn init_analytics() -> Result<(), String> {
    let config = AnalyticsConfig {
        api_key:"phc_cohhHPgfQfnNWl33THRRpCftuRtWx2k5svtKrkpFb04".to_string(),
        host: Some("https://us.i.posthog.com".to_string()),
        enabled: true ,
    };
    
    let client = Arc::new(AnalyticsClient::new(config).await);
    
    unsafe {
        ANALYTICS_CLIENT = Some(client);
    }
    
    Ok(())
}

#[tauri::command]
async fn track_event(event_name: String, properties: Option<std::collections::HashMap<String, String>>) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_event(&event_name, properties).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn identify_user(user_id: String, properties: Option<std::collections::HashMap<String, String>>) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.identify(user_id, properties).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_meeting_started(meeting_id: String, meeting_title: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_meeting_started(&meeting_id, &meeting_title).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_recording_started(meeting_id: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_recording_started(&meeting_id).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_recording_stopped(meeting_id: String, duration_seconds: Option<u64>) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_recording_stopped(&meeting_id, duration_seconds).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_meeting_deleted(meeting_id: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_meeting_deleted(&meeting_id).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_search_performed(query: String, results_count: usize) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_search_performed(&query, results_count).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_settings_changed(setting_type: String, new_value: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_settings_changed(&setting_type, &new_value).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_feature_used(feature_name: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_feature_used(&feature_name).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn is_analytics_enabled() -> bool {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.is_enabled()
        } else {
            false
        }
    }
}

// Enhanced analytics commands for Phase 1
#[tauri::command]
async fn start_analytics_session(user_id: String) -> Result<String, String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.start_session(user_id).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn end_analytics_session() -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.end_session().await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}



#[tauri::command]
async fn track_daily_active_user() -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_daily_active_user().await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_user_first_launch() -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_user_first_launch().await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

// Summary generation analytics commands
#[tauri::command]
async fn track_summary_generation_started(model_provider: String, model_name: String, transcript_length: usize) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_summary_generation_started(&model_provider, &model_name, transcript_length).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_summary_generation_completed(model_provider: String, model_name: String, success: bool, duration_seconds: Option<u64>, error_message: Option<String>) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_summary_generation_completed(&model_provider, &model_name, success, duration_seconds, error_message.as_deref()).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_summary_regenerated(model_provider: String, model_name: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_summary_regenerated(&model_provider, &model_name).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_model_changed(old_provider: String, old_model: String, new_provider: String, new_model: String) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_model_changed(&old_provider, &old_model, &new_provider, &new_model).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn track_custom_prompt_used(prompt_length: usize) -> Result<(), String> {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.track_custom_prompt_used(prompt_length).await
        } else {
            Err("Analytics client not initialized".to_string())
        }
    }
}

#[tauri::command]
async fn is_analytics_session_active() -> bool {
    unsafe {
        if let Some(client) = &ANALYTICS_CLIENT {
            client.is_session_active().await
        } else {
            false
        }
    }
}

// Helper function to convert stereo to mono
fn stereo_to_mono(stereo: &[i16]) -> Vec<i16> {
    let mut mono = Vec::with_capacity(stereo.len() / 2);
    for chunk in stereo.chunks_exact(2) {
        let left = chunk[0] as i32;
        let right = chunk[1] as i32;
        let combined = ((left + right) / 2) as i16;
        mono.push(combined);
    }
    mono
}

pub fn run() {
    log::set_max_level(log::LevelFilter::Info);
    
    tauri::Builder::default()
        .setup(|_app| {
            log::info!("Application setup complete");

            // Trigger microphone permission request on startup
            if let Err(e) = audio::core::trigger_audio_permission() {
                log::error!("Failed to trigger audio permission: {}", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            is_recording,
            read_audio_file,
            save_transcript,
            init_analytics,
            track_event,
            identify_user,
            track_meeting_started,
            track_recording_started,
            track_recording_stopped,
            track_meeting_deleted,
            track_search_performed,
            track_settings_changed,
            track_feature_used,
            is_analytics_enabled,
            start_analytics_session,
            end_analytics_session,
            track_daily_active_user,
            track_user_first_launch,
            is_analytics_session_active,
            track_summary_generation_started,
            track_summary_generation_completed,
            track_summary_regenerated,
            track_model_changed,
            track_custom_prompt_used,
            api::api_get_meetings,
            api::api_search_transcripts,
            api::api_get_profile,
            api::api_save_profile,
            api::api_update_profile,
            api::api_get_model_config,
            api::api_save_model_config,
            api::api_get_api_key,
            api::api_get_transcript_config,
            api::api_save_transcript_config,
            api::api_get_transcript_api_key,
            api::api_delete_meeting,
            api::api_get_meeting,
            api::api_save_meeting_title,
            api::api_save_meeting_summary,
            api::api_get_summary,
            api::api_save_transcript,
            api::api_process_transcript,
            api::debug_store_contents,
            api::test_backend_connection,
            api::debug_backend_connection,
        ])
        .plugin(tauri_plugin_store::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Helper function to resample audio
fn resample_audio(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    
    let ratio = to_rate as f32 / from_rate as f32;
    let new_len = (samples.len() as f32 * ratio) as usize;
    let mut resampled = Vec::with_capacity(new_len);
    
    for i in 0..new_len {
        let src_idx = (i as f32 / ratio) as usize;
        if src_idx < samples.len() {
            resampled.push(samples[src_idx]);
        }
    }
    
    resampled
}
