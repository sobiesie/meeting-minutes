use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::{debug, error, info};
use tokio::sync::broadcast::error::TryRecvError;

mod audio;
use audio::{mix_buffers, AudioStream, AudioBuffer};

// Adjusted volume levels for better mixing
const SYSTEM_VOLUME: f32 = 0.5;  // Balanced system audio
const MIC_VOLUME: f32 = 0.5;     // Balanced mic volume

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting audio capture...");

    // Check permissions first
    audio::trigger_audio_permission()?;

    // Get default devices
    let system_device = Arc::new(audio::default_output_device().await?);
    let mic_device = Arc::new(audio::default_input_device().await?);

    info!("Using system audio device: {}", system_device);
    info!("Using microphone device: {}", mic_device);

    // Setup WAV writer with higher quality settings
    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create("output.wav", spec)?;
    let is_running = Arc::new(AtomicBool::new(true));

    // Create audio streams
    let system_stream = AudioStream::from_device(system_device, is_running.clone()).await?;
    let mic_stream = AudioStream::from_device(mic_device, is_running.clone()).await?;

    // Subscribe to audio streams
    let mut system_rx = system_stream.subscribe().await;
    let mut mic_rx = mic_stream.subscribe().await;

    // Create audio buffers for chunk-based processing with resampling
    let mut system_buffer = AudioBuffer::new(48000, spec.sample_rate);
    let mut mic_buffer = AudioBuffer::new(48000, spec.sample_rate);
    
    // Record for 10 seconds
    let duration = Duration::from_secs(10);
    let start = std::time::Instant::now();
    let mut total_samples = 0;
    let mut empty_buffers = 0;
    let mut mixed_buffers = 0;

    while start.elapsed() < duration {
        // Try to receive from both streams
        let system_samples = match system_rx.try_recv() {
            Ok(samples) => {
                debug!("Received {} system audio samples", samples.len());
                samples
            }
            Err(TryRecvError::Empty) => {
                empty_buffers += 1;
                vec![]
            }
            Err(TryRecvError::Lagged(n)) => {
                info!("System audio stream lagged, missed {} samples", n);
                continue;
            }
            Err(TryRecvError::Closed) => {
                info!("System audio stream closed");
                break;
            }
        };

        let mic_samples = match mic_rx.try_recv() {
            Ok(samples) => {
                debug!("Received {} microphone samples", samples.len());
                samples
            }
            Err(TryRecvError::Empty) => {
                empty_buffers += 1;
                vec![]
            }
            Err(TryRecvError::Lagged(n)) => {
                info!("Microphone audio stream lagged, missed {} samples", n);
                continue;
            }
            Err(TryRecvError::Closed) => {
                info!("Microphone audio stream closed");
                break;
            }
        };

        // Add samples to buffers and process when we have enough for a chunk
        if !system_samples.is_empty() {
            if let Some(system_chunk) = system_buffer.add_samples(&system_samples) {
                if let Some(mic_chunk) = mic_buffer.add_samples(&mic_samples) {
                    mixed_buffers += 1;
                    let mixed = mix_buffers(&system_chunk, &mic_chunk, SYSTEM_VOLUME, MIC_VOLUME)?;
                    
                    // Write mixed samples
                    for &sample in &mixed {
                        writer.write_sample(sample)?;
                        total_samples += 1;
                    }
                }
            }
        } else if !mic_samples.is_empty() {
            if let Some(mic_chunk) = mic_buffer.add_samples(&mic_samples) {
                mixed_buffers += 1;
                let mixed = mix_buffers(&[], &mic_chunk, SYSTEM_VOLUME, MIC_VOLUME)?;
                
                // Write mixed samples
                for &sample in &mixed {
                    writer.write_sample(sample)?;
                    total_samples += 1;
                }
            }
        }

        // Small sleep to prevent busy waiting
        sleep(Duration::from_millis(1)).await;
    }

    info!("Recording finished. Processed {} samples", total_samples);
    info!("Empty buffers: {}, Mixed buffers: {}", empty_buffers, mixed_buffers);

    // Stop streams
    system_stream.stop().await?;
    mic_stream.stop().await?;

    // Finalize WAV file
    writer.finalize()?;
    Ok(())
}
