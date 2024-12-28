use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use lazy_static::lazy_static;
use log::{error, info, warn};
use realfft::RealFftPlanner;
use rubato::{FftFixedInOut, Resampler};
use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::{fmt, thread};
use tokio::sync::{broadcast, oneshot};

// Constants for audio processing
const CHUNK_DURATION_SECS: f32 = 0.1; // Smaller chunks for less latency
const SAMPLE_RATE: u32 = 48000;
const CHUNK_SIZE: usize = (CHUNK_DURATION_SECS * SAMPLE_RATE as f32) as usize;
const OVERLAP_RATIO: f32 = 0.25; // Less overlap to prevent stretching
const OVERLAP_SAMPLES: usize = (CHUNK_SIZE as f32 * OVERLAP_RATIO) as usize;
const FFT_SIZE: usize = 1024; // Smaller FFT size for better time resolution

const BUFFER_SIZE: usize = 1000; // Increased buffer size for better handling

lazy_static! {
    pub static ref LAST_AUDIO_CAPTURE: AtomicU64 = AtomicU64::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Debug, Deserialize)]
pub enum DeviceType {
    Input,
    Output,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Debug)]
pub struct AudioDevice {
    pub name: String,
    pub device_type: DeviceType,
}

impl AudioDevice {
    pub fn new(name: String, device_type: DeviceType) -> Self {
        AudioDevice { name, device_type }
    }

    #[allow(dead_code)]
    pub fn from_name(name: &str) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(anyhow!("Device name cannot be empty"));
        }

        let (name, device_type) = if name.to_lowercase().ends_with("(input)") {
            (
                name.trim_end_matches("(input)").trim().to_string(),
                DeviceType::Input,
            )
        } else if name.to_lowercase().ends_with("(output)") {
            (
                name.trim_end_matches("(output)").trim().to_string(),
                DeviceType::Output,
            )
        } else {
            return Err(anyhow!(
                "Device type (input/output) not specified in the name"
            ));
        };

        Ok(AudioDevice::new(name, device_type))
    }
}

impl fmt::Display for AudioDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.name,
            match self.device_type {
                DeviceType::Input => "input",
                DeviceType::Output => "output",
            }
        )
    }
}

// Audio buffer with overlap handling
#[derive(Default)]
pub struct AudioBuffer {
    samples: Vec<f32>,
    overlap: Vec<f32>,
    sample_rate: u32,
    target_rate: u32,
    resampler: Option<FftFixedInOut<f32>>,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, target_rate: u32) -> Self {
        let resampler = if sample_rate != target_rate {
            Some(FftFixedInOut::<f32>::new(
                sample_rate as usize,
                target_rate as usize,
                2048, // Smaller buffer for less latency
                1,    // Less overlap for resampling
            ).unwrap())
        } else {
            None
        };

        AudioBuffer {
            samples: Vec::new(),
            overlap: vec![0.0; OVERLAP_SAMPLES],
            sample_rate,
            target_rate,
            resampler,
        }
    }

    pub fn add_samples(&mut self, new_samples: &[f32]) -> Option<Vec<f32>> {
        // Convert to mono if needed
        let mono = audio_to_mono(new_samples, 1);
        
        // Resample if needed
        let resampled = if let Some(resampler) = &mut self.resampler {
            let mut output = vec![vec![0.0; resampler.output_frames_max()]];
            let waves_in = vec![mono.as_slice()];
            resampler.process_into_buffer(&waves_in, &mut output, None).unwrap();
            output.pop().unwrap()
        } else {
            mono
        };

        self.samples.extend(resampled);
        
        if self.samples.len() >= CHUNK_SIZE {
            // Extract chunk with overlap
            let mut chunk = self.samples.drain(..CHUNK_SIZE-OVERLAP_SAMPLES).collect::<Vec<_>>();
            
            // Apply fade-in/fade-out to overlap region
            for i in 0..OVERLAP_SAMPLES {
                let fade = 0.5 * (1.0 - (std::f32::consts::PI * i as f32 / OVERLAP_SAMPLES as f32).cos());
                chunk.push(self.overlap[i] * (1.0 - fade) + self.samples[i] * fade);
            }
            
            // Save overlap for next chunk
            if self.samples.len() >= OVERLAP_SAMPLES {
                self.overlap.copy_from_slice(&self.samples[..OVERLAP_SAMPLES]);
            }
            
            Some(chunk)
        } else {
            None
        }
    }
}

// Improved audio processing pipeline
pub fn process_audio_chunk(chunk: &[f32]) -> Result<Vec<f32>> {
    if chunk.is_empty() {
        return Ok(Vec::new());
    }

    // 1. Apply normalization
    let normalized = normalize_audio(chunk);
    
    // 2. Apply spectral subtraction with gentler noise reduction
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let ifft = planner.plan_fft_inverse(FFT_SIZE);
    
    let mut output = Vec::with_capacity(chunk.len());
    
    // Process in FFT-sized windows with overlap
    for window in normalized.chunks(FFT_SIZE/2) { // 50% overlap for FFT processing
        let mut fft_buffer = vec![0.0; FFT_SIZE];
        fft_buffer[..window.len()].copy_from_slice(window);
        
        // Apply Hanning window
        for i in 0..FFT_SIZE {
            let hann = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            fft_buffer[i] *= hann;
        }
        
        // Forward FFT
        let mut spectrum = vec![Complex::new(0.0, 0.0); FFT_SIZE/2 + 1];
        let mut scratch = vec![Complex::new(0.0, 0.0); FFT_SIZE];
        fft.process_with_scratch(&mut fft_buffer, &mut spectrum, &mut scratch)
            .map_err(|e| anyhow!("FFT error: {}", e))?;
        
        // Gentler spectral subtraction
        let noise_floor = spectrum.iter().map(|x| x.norm()).sum::<f32>() / spectrum.len() as f32 * 0.1; // Reduced noise threshold
        for bin in spectrum.iter_mut() {
            let magnitude = bin.norm();
            if magnitude > noise_floor {
                let factor = 1.0 - (noise_floor / magnitude).sqrt() * 0.5; // Gentler reduction
                *bin *= factor.max(0.3); // Higher minimum to preserve more signal
            } else {
                *bin *= 0.3; // Keep more of the low-amplitude signals
            }
        }
        
        // Inverse FFT
        let mut processed = vec![0.0; FFT_SIZE];
        ifft.process_with_scratch(&mut spectrum, &mut processed, &mut scratch)
            .map_err(|e| anyhow!("IFFT error: {}", e))?;
        
        // Remove Hanning window and scale
        for i in 0..FFT_SIZE {
            let hann = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
            processed[i] /= hann.max(0.01);
            processed[i] /= FFT_SIZE as f32;
        }
        
        // Only take the first half to handle overlap
        output.extend_from_slice(&processed[..window.len()]);
    }
    
    Ok(output)
}

// Update mix_buffers to handle system audio differently
pub fn mix_buffers(system_audio: &[f32], mic_audio: &[f32], system_gain: f32, mic_gain: f32) -> Result<Vec<f32>> {
    // For system audio, only apply minimal processing
    let processed_system = if !system_audio.is_empty() {
        normalize_audio(system_audio)
    } else {
        Vec::new()
    };

    // For mic audio, apply full processing
    let processed_mic = if !mic_audio.is_empty() {
        process_audio_chunk(mic_audio)?
    } else {
        Vec::new()
    };

    let max_len = processed_system.len().max(processed_mic.len());
    let mut mixed = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let system_sample = processed_system.get(i).copied().unwrap_or(0.0) * system_gain;
        let mic_sample = processed_mic.get(i).copied().unwrap_or(0.0) * mic_gain;
        
        // Soft limiting with bias towards preserving system audio
        let mixed_sample = system_sample + mic_sample * 0.8;
        let limited = if mixed_sample.abs() > 1.0 {
            mixed_sample.signum() * (1.0 - (-mixed_sample.abs()).exp())
        } else {
            mixed_sample
        };
        
        mixed.push(limited);
    }

    Ok(mixed)
}

// Audio processing functions
pub fn normalize_audio(audio: &[f32]) -> Vec<f32> {
    if audio.is_empty() {
        return Vec::new();
    }

    let rms = (audio.iter().map(|&x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    let peak = audio
        .iter()
        .fold(0.0f32, |max, &sample| max.max(sample.abs()));

    // Return the original audio if it's completely silent
    if rms < 1e-6 || peak < 1e-6 {
        return audio.to_vec();
    }

    let target_rms = 0.2;
    let target_peak = 0.95;

    let rms_scaling = target_rms / rms;
    let peak_scaling = target_peak / peak;

    // Use the smaller scaling factor to prevent clipping
    let scaling_factor = rms_scaling.min(peak_scaling);

    audio.iter().map(|&sample| sample * scaling_factor).collect()
}

pub fn audio_to_mono(audio: &[f32], channels: u16) -> Vec<f32> {
    if audio.is_empty() || channels == 0 {
        return Vec::new();
    }

    let chunk_size = channels as usize;
    let mut mono_samples = Vec::with_capacity(audio.len() / chunk_size);

    for chunk in audio.chunks(chunk_size) {
        // If we have an incomplete chunk at the end, pad with zeros
        if chunk.len() < chunk_size {
            let mut padded = chunk.to_vec();
            padded.resize(chunk_size, 0.0);
            let sum: f32 = padded.iter().sum();
            mono_samples.push(sum / channels as f32);
        } else {
            let sum: f32 = chunk.iter().sum();
            mono_samples.push(sum / channels as f32);
        }
    }

    mono_samples
}

// Simplified mixing function that focuses on clean audio
pub fn mix_buffers_simple(buffer1: &[f32], buffer2: &[f32], volume1: f32, volume2: f32) -> Vec<f32> {
    let max_len = buffer1.len().max(buffer2.len());
    let mut mixed = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let sample1 = buffer1.get(i).copied().unwrap_or(0.0) * volume1;
        let sample2 = buffer2.get(i).copied().unwrap_or(0.0) * volume2;
        
        // Simple mixing with soft clipping
        let mixed_sample = (sample1 + sample2) * 0.5;
        mixed.push(if mixed_sample > 1.0 {
            1.0
        } else if mixed_sample < -1.0 {
            -1.0
        } else {
            mixed_sample
        });
    }

    mixed
}

// Spectral subtraction for noise reduction
pub fn spectral_subtraction(input: &[f32], noise_profile: Option<&[f32]>) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    // If no noise profile provided, estimate from the first 100ms of audio
    let noise_est = noise_profile.unwrap_or(&input[..input.len().min(4800)]);
    
    // Simple spectral subtraction (time domain approximation)
    let mut output = Vec::with_capacity(input.len());
    let window_size = 1024;
    
    for chunk in input.chunks(window_size) {
        let mut processed = chunk.to_vec();
        let noise_power = noise_est.iter().map(|&x| x.powi(2)).sum::<f32>() / noise_est.len() as f32;
        
        for sample in processed.iter_mut() {
            let power = sample.powi(2);
            if power > noise_power {
                *sample *= (1.0 - (noise_power / power).sqrt());
            } else {
                *sample *= 0.1; // Residual noise floor
            }
        }
        
        output.extend(processed);
    }

    output
}

pub async fn get_device_and_config(
    audio_device: &AudioDevice,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();

    let is_output_device = audio_device.device_type == DeviceType::Output;
    let is_display = audio_device.to_string().contains("Display");

    let cpal_audio_device = if audio_device.to_string() == "default" {
        match audio_device.device_type {
            DeviceType::Input => host.default_input_device(),
            DeviceType::Output => host.default_output_device(),
        }
    } else {
        let mut devices = match audio_device.device_type {
            DeviceType::Input => host.input_devices()?,
            DeviceType::Output => host.output_devices()?,
        };

        #[cfg(target_os = "macos")]
        {
            if is_output_device {
                if let Ok(screen_capture_host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit)
                {
                    devices = screen_capture_host.input_devices()?;
                }
            }
        }

        devices.find(|x| {
            x.name()
                .map(|y| {
                    y == audio_device
                        .to_string()
                        .replace(" (input)", "")
                        .replace(" (output)", "")
                        .trim()
                })
                .unwrap_or(false)
        })
    }
    .ok_or_else(|| anyhow!("Audio device not found"))?;

    let config = if is_output_device && !is_display {
        cpal_audio_device.default_output_config()?
    } else {
        cpal_audio_device.default_input_config()?
    };
    Ok((cpal_audio_device, config))
}

pub async fn default_input_device() -> Result<AudioDevice> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No input device available"))?;
    let name = device.name()?;
    Ok(AudioDevice::new(name, DeviceType::Input))
}

pub async fn default_output_device() -> Result<AudioDevice> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(screen_capture_host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
            let mut devices = screen_capture_host.input_devices()?;
            if let Some(device) = devices.find(|d| {
                d.name()
                    .map(|name| name.contains("Display"))
                    .unwrap_or(false)
            }) {
                let name = device.name()?;
                return Ok(AudioDevice::new(name, DeviceType::Output));
            }
        }
    }

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("No output device available"))?;
    let name = device.name()?;
    Ok(AudioDevice::new(name, DeviceType::Output))
}

#[derive(Debug)]
pub enum PermissionError {
    ScreenRecordingDenied,
    ScreenRecordingNotDetermined,
    DeviceAccessDenied,
    Other(String),
}

impl fmt::Display for PermissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionError::ScreenRecordingDenied => write!(
                f,
                "Screen Recording permission denied. This is required to capture system audio on macOS."
            ),
            PermissionError::ScreenRecordingNotDetermined => write!(
                f,
                "Screen Recording permission not determined. Please grant permission when prompted."
            ),
            PermissionError::DeviceAccessDenied => write!(
                f,
                "Access to audio devices denied. Please check your system permissions."
            ),
            PermissionError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for PermissionError {}

pub fn trigger_audio_permission() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        info!("Checking Screen Recording permission...");
        
        let host = cpal::host_from_id(cpal::HostId::ScreenCaptureKit)
            .map_err(|e| {
                if e.to_string().contains("TCCs") {
                    PermissionError::ScreenRecordingDenied
                } else {
                    PermissionError::Other(format!("Failed to get ScreenCaptureKit host: {}", e))
                }
            })?;

        info!("Checking audio device access...");
        let mut devices = host
            .input_devices()
            .map_err(|e| {
                if e.to_string().contains("permission") {
                    PermissionError::DeviceAccessDenied
                } else {
                    PermissionError::Other(format!("Failed to get input devices: {}", e))
                }
            })?;

        let device = devices
            .next()
            .ok_or_else(|| PermissionError::Other("No input device found".to_string()))?;

        info!("Checking device configuration access...");
        device
            .default_input_config()
            .map_err(|e| {
                if e.to_string().contains("permission") {
                    PermissionError::DeviceAccessDenied
                } else {
                    PermissionError::Other(format!("Failed to get default input config: {}", e))
                }
            })?;

        info!("All permissions granted successfully!");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        info!("Permission check not required on this platform");
        Ok(())
    }
}

#[derive(Clone)]
pub struct AudioStream {
    device: Arc<AudioDevice>,
    pub device_config: cpal::SupportedStreamConfig,
    transmitter: Arc<broadcast::Sender<Vec<f32>>>,
    stream_control: mpsc::Sender<StreamControl>,
    pub stream_thread: Option<Arc<tokio::sync::Mutex<Option<thread::JoinHandle<()>>>>>,
    is_disconnected: Arc<AtomicBool>,
}

pub enum StreamControl {
    Stop(oneshot::Sender<()>),
}

impl AudioStream {
    pub async fn from_device(
        device: Arc<AudioDevice>,
        is_running: Arc<AtomicBool>,
    ) -> Result<Self> {
        let (tx, _) = broadcast::channel::<Vec<f32>>(BUFFER_SIZE);
        let tx_clone = tx.clone();
        let (cpal_audio_device, config) = get_device_and_config(&device).await?;
        let channels = config.channels();

        let is_running_weak = Arc::downgrade(&is_running);
        let is_disconnected = Arc::new(AtomicBool::new(false));
        let device_clone = device.clone();
        let config_clone = config.clone();
        let (stream_control_tx, stream_control_rx) = mpsc::channel();

        let is_disconnected_clone = is_disconnected.clone();
        let stream_control_tx_clone = stream_control_tx.clone();
        let stream_thread = Arc::new(tokio::sync::Mutex::new(Some(thread::spawn(move || {
            let device = device_clone;
            let device_name = device.to_string();
            let config = config_clone;
            let error_callback = move |err: StreamError| {
                if err
                    .to_string()
                    .contains("The requested device is no longer available")
                {
                    warn!(
                        "audio device {} disconnected. stopping recording.",
                        device_name
                    );
                    stream_control_tx_clone
                        .send(StreamControl::Stop(oneshot::channel().0))
                        .unwrap();

                    is_disconnected_clone.store(true, Ordering::Relaxed);
                } else {
                    error!("an error occurred on the audio stream: {}", err);
                    if err.to_string().contains("device is no longer valid") {
                        warn!("audio device disconnected. stopping recording.");
                        if let Some(arc) = is_running_weak.upgrade() {
                            arc.store(false, Ordering::Relaxed);
                        }
                    }
                }
            };

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => cpal_audio_device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &_| {
                            let mono = audio_to_mono(data, channels);
                            let normalized = normalize_audio(&mono);
                            let _ = tx.send(normalized);
                        },
                        error_callback,
                        None,
                    )
                    .expect("Failed to build input stream"),
                cpal::SampleFormat::I16 => cpal_audio_device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            let normalized = normalize_audio(&mono);
                            let _ = tx.send(normalized);
                        },
                        error_callback,
                        None,
                    )
                    .expect("Failed to build input stream"),
                cpal::SampleFormat::I32 => cpal_audio_device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i32], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            let normalized = normalize_audio(&mono);
                            let _ = tx.send(normalized);
                        },
                        error_callback,
                        None,
                    )
                    .expect("Failed to build input stream"),
                cpal::SampleFormat::I8 => cpal_audio_device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i8], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            let normalized = normalize_audio(&mono);
                            let _ = tx.send(normalized);
                        },
                        error_callback,
                        None,
                    )
                    .expect("Failed to build input stream"),
                _ => {
                    error!("unsupported sample format: {}", config.sample_format());
                    return;
                }
            };

            if let Err(e) = stream.play() {
                error!("failed to play stream for {}: {}", device.to_string(), e);
            }

            if let Ok(StreamControl::Stop(response)) = stream_control_rx.recv() {
                info!("stopped recording audio stream");
                stream.pause().ok();
                drop(stream);
                response.send(()).ok();
            }
        }))));

        Ok(AudioStream {
            device,
            device_config: config,
            transmitter: Arc::new(tx_clone),
            stream_control: stream_control_tx,
            stream_thread: Some(stream_thread),
            is_disconnected,
        })
    }

    pub async fn subscribe(&self) -> broadcast::Receiver<Vec<f32>> {
        self.transmitter.subscribe()
    }

    pub async fn stop(mut self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.stream_control.send(StreamControl::Stop(tx))?;
        rx.await?;

        if let Some(thread) = self.stream_thread.take() {
            if let Some(handle) = thread.lock().await.take() {
                handle.join().ok();
            }
        }

        Ok(())
    }
}
