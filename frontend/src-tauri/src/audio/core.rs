use super::audio_processing::audio_to_mono; 
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use lazy_static::lazy_static;
use log::{ error, info, warn, debug};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, thread};
use tokio::sync::{broadcast, oneshot};
lazy_static! {
    pub static ref LAST_AUDIO_CAPTURE: AtomicU64 = AtomicU64::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioTranscriptionEngine {
    Deepgram,
    WhisperTiny,
    WhisperDistilLargeV3,
    WhisperLargeV3Turbo,
    WhisperLargeV3,
}

impl fmt::Display for AudioTranscriptionEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioTranscriptionEngine::Deepgram => write!(f, "Deepgram"),
            AudioTranscriptionEngine::WhisperTiny => write!(f, "WhisperTiny"),
            AudioTranscriptionEngine::WhisperDistilLargeV3 => write!(f, "WhisperLarge"),
            AudioTranscriptionEngine::WhisperLargeV3Turbo => write!(f, "WhisperLargeV3Turbo"),
            AudioTranscriptionEngine::WhisperLargeV3 => write!(f, "WhisperLargeV3"),
        }
    }
}

impl Default for AudioTranscriptionEngine {
    fn default() -> Self {
        AudioTranscriptionEngine::WhisperLargeV3Turbo
    }
}

#[derive(Clone, Debug)]
pub struct DeviceControl {
    pub is_running: bool,
    pub is_paused: bool,
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

pub fn parse_audio_device(name: &str) -> Result<AudioDevice> {
    AudioDevice::from_name(name)
}

// Platform-specific audio device configurations
#[cfg(target_os = "windows")]
fn configure_windows_audio(host: &cpal::Host) -> Result<Vec<AudioDevice>> {
    let mut devices = Vec::new();
    
    // Get WASAPI devices
    if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
        // Add output devices (including loopback)
        if let Ok(output_devices) = wasapi_host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    // For Windows, we need to mark output devices specifically for loopback
                    devices.push(AudioDevice::new(format!("{} (output)", name), DeviceType::Output));
                }
            }
        }

        // Add input devices from WASAPI
        if let Ok(input_devices) = wasapi_host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDevice::new(format!("{} (input)", name), DeviceType::Input));
                }
            }
        }
    }
    
    // If WASAPI failed, try default host as fallback
    if devices.is_empty() {
        debug!("WASAPI device enumeration failed, falling back to default host");
        // Add regular input devices
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDevice::new(format!("{} (input)", name), DeviceType::Input));
                }
            }
        }

        // Add output devices
        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDevice::new(format!("{} (output)", name), DeviceType::Output));
                }
            }
        }
    }
    
    Ok(devices)
}

#[cfg(target_os = "linux")]
fn configure_linux_audio(host: &cpal::Host) -> Result<Vec<AudioDevice>> {
    let mut devices = Vec::new();
    
    // Add input devices
    for device in host.input_devices()? {
        if let Ok(name) = device.name() {
            devices.push(AudioDevice::new(name, DeviceType::Input));
        }
    }
    
    // Add PulseAudio monitor sources for system audio
    if let Ok(pulse_host) = cpal::host_from_id(cpal::HostId::Pulse) {
        for device in pulse_host.input_devices()? {
            if let Ok(name) = device.name() {
                // Check if it's a monitor source
                if name.contains("monitor") {
                    devices.push(AudioDevice::new(
                        format!("{} (System Audio)", name),
                        DeviceType::Output
                    ));
                }
            }
        }
    }
    
    Ok(devices)
}

pub async fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    // Platform-specific device enumeration
    #[cfg(target_os = "windows")]
    {
        devices = configure_windows_audio(&host)?;
    }

    #[cfg(target_os = "linux")]
    {
        devices = configure_linux_audio(&host)?;
    }

    #[cfg(target_os = "macos")]
    {
        // Existing macOS implementation
        for device in host.input_devices()? {
            if let Ok(name) = device.name() {
                devices.push(AudioDevice::new(name, DeviceType::Input));
            }
        }

        // Filter function to exclude macOS speakers and AirPods for output devices
        fn should_include_output_device(name: &str) -> bool {
            !name.to_lowercase().contains("speakers") && !name.to_lowercase().contains("airpods")
        }

        if let Ok(host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
            for device in host.input_devices()? {
                if let Ok(name) = device.name() {
                    if should_include_output_device(&name) {
                        devices.push(AudioDevice::new(name, DeviceType::Output));
                    }
                }
            }
        }

        for device in host.output_devices()? {
            if let Ok(name) = device.name() {
                if should_include_output_device(&name) {
                    devices.push(AudioDevice::new(name, DeviceType::Output));
                }
            }
        }
    }

    // Add any additional devices from the default host
    if let Ok(other_devices) = host.devices() {
        for device in other_devices {
            if let Ok(name) = device.name() {
                if !devices.iter().any(|d| d.name == name) {
                    devices.push(AudioDevice::new(name, DeviceType::Output));
                }
            }
        }
    }

    Ok(devices)
}

pub fn default_input_device() -> Result<AudioDevice> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No default input device found"))?;
    Ok(AudioDevice::new(device.name()?, DeviceType::Input))
}

pub fn default_output_device() -> Result<AudioDevice> {
    #[cfg(target_os = "macos")]
    {
        // ! see https://github.com/RustAudio/cpal/pull/894
        if let Ok(host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
            if let Some(device) = host.default_input_device() {
                if let Ok(name) = device.name() {
                    return Ok(AudioDevice::new(name, DeviceType::Output));
                }
            }
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No default output device found"))?;
        return Ok(AudioDevice::new(device.name()?, DeviceType::Output));
    }

    #[cfg(target_os = "windows")]
    {
        // Try WASAPI host first for Windows
        if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
            if let Some(device) = wasapi_host.default_output_device() {
                if let Ok(name) = device.name() {
                    return Ok(AudioDevice::new(name, DeviceType::Output));
                }
            }
        }
        // Fallback to default host if WASAPI fails
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No default output device found"))?;
        return Ok(AudioDevice::new(device.name()?, DeviceType::Output));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No default output device found"))?;
        return Ok(AudioDevice::new(device.name()?, DeviceType::Output));
    }
}

pub fn trigger_audio_permission() -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No default input device found"))?;

    let config = device.default_input_config()?;

    // Build and start an input stream to trigger the permission request
    let stream = device.build_input_stream(
        &config.into(),
        |_data: &[f32], _: &cpal::InputCallbackInfo| {
            // Do nothing, we just want to trigger the permission request
        },
        |err| error!("Error in audio stream: {}", err),
        None,
    )?;

    // Start the stream to actually trigger the permission dialog
    stream.play()?;

    // Sleep briefly to allow the permission dialog to appear
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Stop the stream
    drop(stream);

    Ok(())
}

#[derive(Clone)]
pub struct AudioStream {
    pub device: Arc<AudioDevice>,
    pub device_config: cpal::SupportedStreamConfig,
    transmitter: Arc<tokio::sync::broadcast::Sender<Vec<f32>>>,
    stream_control: mpsc::Sender<StreamControl>,
    stream_thread: Option<Arc<tokio::sync::Mutex<Option<thread::JoinHandle<()>>>>>,
    is_disconnected: Arc<AtomicBool>,
}

enum StreamControl {
    Stop(oneshot::Sender<()>),
}

impl AudioStream {
    pub async fn from_device(
        device: Arc<AudioDevice>,
        is_running: Arc<AtomicBool>,
    ) -> Result<Self> {
        info!("Initializing audio stream for device: {}", device.to_string());
        let (tx, rx) = broadcast::channel::<Vec<f32>>(1000);
        // Keep one receiver alive to prevent "channel closed" errors
        let _keep_alive_rx = rx;
        let tx_clone = tx.clone();
        
        // Get device and config with improved error handling
        let (cpal_audio_device, config) = match get_device_and_config(&device).await {
            Ok((device, config)) => (device, config),
            Err(e) => {
                error!("Failed to get device and config: {}", e);
                return Err(anyhow!("Failed to initialize audio device: {}", e));
            }
        };
        
        // Verify we can actually get input config for input devices
        if device.device_type == DeviceType::Input {
            match cpal_audio_device.default_input_config() {
                Ok(conf) => info!("Default input config: {:?}", conf),
                Err(e) => {
                    error!("Failed to get default input config: {}", e);
                    
                    // On Windows, we might still be able to use the device with our custom config
                    #[cfg(not(target_os = "windows"))]
                    return Err(anyhow!("Failed to get default input config: {}", e));
                    
                    #[cfg(target_os = "windows")]
                    warn!("Continuing with custom config despite default config error on Windows");
                }
            }
        }
        
        let channels = config.channels();
        info!("Audio config - Sample rate: {}, Channels: {}, Format: {:?}", 
            config.sample_rate().0, channels, config.sample_format());

        let is_running_weak_2 = Arc::downgrade(&is_running);
        let is_disconnected = Arc::new(AtomicBool::new(false));
        let device_clone = device.clone();
        let config_clone = config.clone();
        let (stream_control_tx, stream_control_rx) = mpsc::channel();

        let is_disconnected_clone = is_disconnected.clone();
        let stream_control_tx_clone = stream_control_tx.clone();
        let stream_thread = Arc::new(tokio::sync::Mutex::new(Some(thread::spawn(move || {
            let device = device_clone;
            let device_name = device.to_string();
            let device_name_clone = device_name.clone();  // Clone for the closure
            let config = config_clone;
            info!("Starting audio stream thread for device: {}", device_name);
            let error_callback = move |err: StreamError| {
                if err
                    .to_string()
                    .contains("The requested device is no longer available")
                {
                    warn!(
                        "audio device {} disconnected. stopping recording.",
                        device_name_clone
                    );
                    stream_control_tx_clone
                        .send(StreamControl::Stop(oneshot::channel().0))
                        .unwrap();

                    is_disconnected_clone.store(true, Ordering::Relaxed);
                } else if err.to_string().to_lowercase().contains("permission denied") || 
                         err.to_string().to_lowercase().contains("access denied") {
                    error!("Permission denied for audio device {}. Please check microphone permissions.", device_name_clone);
                    if let Some(arc) = is_running_weak_2.upgrade() {
                        arc.store(false, Ordering::Relaxed);
                    }
                } else {
                    error!("an error occurred on the audio stream: {}", err);
                    if err.to_string().contains("device is no longer valid") {
                        warn!("audio device disconnected. stopping recording.");
                        if let Some(arc) = is_running_weak_2.upgrade() {
                            arc.store(false, Ordering::Relaxed);
                        }
                    }
                }
            };

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    match cpal_audio_device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &_| {
                            let mono = audio_to_mono(data, channels);
                            debug!("Received audio chunk: {} samples", mono.len());
                            if let Err(e) = tx.send(mono) {
                                error!("Failed to send audio data: {}", e);
                                if e.to_string().contains("channel closed") {
                                    debug!("Audio channel closed, this is expected during shutdown");
                                } else {
                                    error!("Unexpected error sending audio data: {}", e);
                                }
                            }
                        },
                        error_callback.clone(),
                        None,
                    ) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to build input stream: {}", e);
                            return;
                        }
                    }
                }
                cpal::SampleFormat::I16 => {
                    match cpal_audio_device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            debug!("Received audio chunk: {} samples", mono.len());
                            if let Err(e) = tx.send(mono) {
                                error!("Failed to send audio data: {}", e);
                                if e.to_string().contains("channel closed") {
                                    debug!("Audio channel closed, this is expected during shutdown");
                                } else {
                                    error!("Unexpected error sending audio data: {}", e);
                                }
                            }
                        },
                        error_callback.clone(),
                        None,
                    ) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to build input stream: {}", e);
                            return;
                        }
                    }
                }
                cpal::SampleFormat::I32 => {
                    match cpal_audio_device.build_input_stream(
                        &config.into(),
                        move |data: &[i32], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            debug!("Received audio chunk: {} samples", mono.len());
                            if let Err(e) = tx.send(mono) {
                                error!("Failed to send audio data: {}", e);
                                if e.to_string().contains("channel closed") {
                                    debug!("Audio channel closed, this is expected during shutdown");
                                } else {
                                    error!("Unexpected error sending audio data: {}", e);
                                }
                            }
                        },
                        error_callback.clone(),
                        None,
                    ) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to build input stream: {}", e);
                            return;
                        }
                    }
                }
                cpal::SampleFormat::I8 => {
                    match cpal_audio_device.build_input_stream(
                        &config.into(),
                        move |data: &[i8], _: &_| {
                            let mono = audio_to_mono(bytemuck::cast_slice(data), channels);
                            debug!("Received audio chunk: {} samples", mono.len());
                            if let Err(e) = tx.send(mono) {
                                error!("Failed to send audio data: {}", e);
                                if e.to_string().contains("channel closed") {
                                    debug!("Audio channel closed, this is expected during shutdown");
                                } else {
                                    error!("Unexpected error sending audio data: {}", e);
                                }
                            }
                        },
                        error_callback.clone(),
                        None,
                    ) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to build input stream: {}", e);
                            return;
                        }
                    }
                }
                _ => {
                    error!("unsupported sample format: {}", config.sample_format());
                    return;
                }
            };

            if let Err(e) = stream.play() {
                error!("failed to play stream for {}: {}", device.to_string(), e);
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("permission") {
                    error!("Permission error detected. Please check microphone permissions");
                } else if err_str.contains("busy") {
                    error!("Device is busy. Another application might be using it");
                }
                return;
            }
            info!("Audio stream started successfully for device: {}", device_name);
            if let Ok(StreamControl::Stop(response)) = stream_control_rx.recv() {
                info!("stopping audio stream...");
                // First stop the stream
                if let Err(e) = stream.pause() {
                    error!("failed to pause stream: {}", e);
                }
                // Close the stream to release OS resources
                drop(stream);
                // Signal completion
                response.send(()).ok();
                info!("audio stream stopped and cleaned up");
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

    pub async fn stop(&self) -> Result<()> {
        // Mark as disconnected first
        self.is_disconnected.store(true, Ordering::Release);
        
        // Send stop signal and wait for confirmation
        let (tx, _rx) = oneshot::channel();
        self.stream_control.send(StreamControl::Stop(tx))?;

        // Wait for thread to finish
        if let Some(thread_arc) = &self.stream_thread {
            let thread_arc = thread_arc.clone();
            let thread_handle = tokio::task::spawn_blocking(move || {
                let mut thread_guard = thread_arc.blocking_lock();
                if let Some(join_handle) = thread_guard.take() {
                    join_handle
                        .join()
                        .map_err(|_| anyhow!("failed to join stream thread"))
                } else {
                    Ok(())
                }
            });

            thread_handle.await??;
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn get_windows_device(audio_device: &AudioDevice) -> Result<(cpal::Device, cpal::SupportedStreamConfig)> {
    info!("Getting Windows audio device: {}", audio_device.name);
    
    // Try WASAPI host first
    let wasapi_host = match cpal::host_from_id(cpal::HostId::Wasapi) {
        Ok(host) => {
            info!("Successfully created WASAPI host");
            host
        },
        Err(e) => {
            error!("Failed to create WASAPI host: {}. Falling back to default host.", e);
            cpal::default_host()
        }
    };

    // Log available devices for debugging
    info!("Available input devices:");
    match wasapi_host.input_devices() {
        Ok(devices) => {
            for device in devices {
                if let Ok(name) = device.name() {
                    info!("  - Input device: {}", name);
                }
            }
        },
        Err(e) => error!("Failed to enumerate input devices: {}", e)
    };
    
    info!("Available output devices:");
    match wasapi_host.output_devices() {
        Ok(devices) => {
            for device in devices {
                if let Ok(name) = device.name() {
                    info!("  - Output device: {}", name);
                }
            }
        },
        Err(e) => error!("Failed to enumerate output devices: {}", e)
    };

    match audio_device.device_type {
        DeviceType::Input => {
            for device in wasapi_host.input_devices()? {
                if let Ok(name) = device.name() {
                    if name == audio_device.name {
                        // Try to get default input config
                        match device.default_input_config() {
                            Ok(default_config) => {
                                info!("Using default input config: {:?}", default_config);
                                return Ok((device, default_config));
                            },
                            Err(e) => {
                                warn!("Failed to get default input config: {}. Trying supported configs...", e);
                                
                                // Try to find a supported configuration
                                if let Ok(supported_configs) = device.supported_input_configs() {
                                    for config in supported_configs {
                                        // Prefer configurations with F32 format
                                        if config.sample_format() == cpal::SampleFormat::F32 {
                                            let config = config.with_max_sample_rate();
                                            info!("Using alternative input config: {:?}", config);
                                            return Ok((device, config));
                                        }
                                    }
                                    
                                    // If no F32 format found, try any supported format
                                    if let Ok(supported_configs) = device.supported_input_configs() {
                                        if let Some(config) = supported_configs.into_iter().next() {
                                            let config = config.with_max_sample_rate();
                                            info!("Using fallback input config: {:?}", config);
                                            return Ok((device, config));
                                        }
                                    }
                                }
                                
                                return Err(anyhow!("No compatible input configuration found for device: {}", name));
                            }
                        }
                    }
                }
            }
        }
        DeviceType::Output => {
            for device in wasapi_host.output_devices()? {
                if let Ok(name) = device.name() {
                    if name == audio_device.name {
                        // For output devices, we want to use them in loopback mode
                        let supported_configs = device.supported_output_configs()?;
                        
                        // Try to find a config that supports f32 format
                        for config in supported_configs {
                            if config.sample_format() == cpal::SampleFormat::F32 {
                                let config = config.with_max_sample_rate();
                                return Ok((device, config));
                            }
                        }
                        
                        // If no f32 config found, use default
                        let default_config = device
                            .default_output_config()
                            .map_err(|e| anyhow!("Failed to get default output config: {}", e))?;
                        return Ok((device, default_config));
                    }
                }
            }
        }
    }

    Err(anyhow!("Device not found: {}", audio_device.name))
}

#[cfg(target_os = "windows")]
pub fn ensure_windows_audio_permissions() -> Result<()> {
    info!("Checking Windows audio permissions...");
    
    // On Windows, we can try to access the default input device to trigger permission prompts
    let host = cpal::default_host();
    
    match host.default_input_device() {
        Some(device) => {
            info!("Default input device found: {}", device.name()?);
            
            // Try to get the default input config to verify permissions
            match device.default_input_config() {
                Ok(config) => {
                    info!("Default input config: {:?}", config);
                    Ok(())
                },
                Err(e) => {
                    if e.to_string().to_lowercase().contains("permission") || 
                       e.to_string().to_lowercase().contains("access denied") {
                        error!("Permission error detected: {}. Please check microphone permissions in Windows Settings.", e);
                        
                        // Inform the user about how to enable permissions
                        info!("To enable microphone permissions:");
                        info!("1. Go to Windows Settings > Privacy & Security > Microphone");
                        info!("2. Ensure 'Microphone access' is turned On");
                        info!("3. Ensure this app is allowed to access your microphone");
                        
                        Err(anyhow!("Microphone permission denied. Please check Windows Settings."))
                    } else {
                        warn!("Non-permission error with audio device: {}", e);
                        Ok(()) // Continue despite error if it's not permission-related
                    }
                }
            }
        },
        None => {
            warn!("No default input device found");
            Ok(()) // Continue anyway
        }
    }
}

pub async fn get_device_and_config(
    audio_device: &AudioDevice,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig)> {
    #[cfg(target_os = "windows")]
    {
        // Check permissions first on Windows
        if let Err(e) = ensure_windows_audio_permissions() {
            warn!("Windows permission check warning: {}", e);
            // Continue anyway, as the user might have manually granted permissions
        }
        
        return get_windows_device(audio_device);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let host = cpal::default_host();
        
        match audio_device.device_type {
            DeviceType::Input => {
                for device in host.input_devices()? {
                    if let Ok(name) = device.name() {
                        if name == audio_device.name {
                            let default_config = device
                                .default_input_config()
                                .map_err(|e| anyhow!("Failed to get default input config: {}", e))?;
                            return Ok((device, default_config));
                        }
                    }
                }
            }
            DeviceType::Output => {
                #[cfg(target_os = "macos")]
                {
                    if let Ok(host) = cpal::host_from_id(cpal::HostId::ScreenCaptureKit) {
                        for device in host.input_devices()? {
                            if let Ok(name) = device.name() {
                                if name == audio_device.name {
                                    let default_config = device
                                        .default_input_config()
                                        .map_err(|e| anyhow!("Failed to get default input config: {}", e))?;
                                    return Ok((device, default_config));
                                }
                            }
                        }
                    }
                }

                #[cfg(target_os = "linux")]
                {
                    // For Linux, we use PulseAudio monitor sources for system audio
                    if let Ok(pulse_host) = cpal::host_from_id(cpal::HostId::Pulse) {
                        for device in pulse_host.input_devices()? {
                            if let Ok(name) = device.name() {
                                if name == audio_device.name {
                                    let default_config = device
                                        .default_input_config()
                                        .map_err(|e| anyhow!("Failed to get default input config: {}", e))?;
                                    return Ok((device, default_config));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Err(anyhow!("Device not found: {}", audio_device.name))
    }
}
