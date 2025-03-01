use anyhow::Result;
use log::{info, error};

#[cfg(target_os = "windows")]
pub fn request_windows_permissions() -> Result<()> {
    info!("Requesting Windows permissions...");
    
    // On Windows 10/11, we can't programmatically request permissions directly.
    // Instead, we'll try to access the devices which will trigger the OS permission prompts
    
    // For microphone permissions
    let result = try_access_microphone();
    if let Err(e) = &result {
        error!("Failed to access microphone: {}", e);
    }
    
    // For screen capture permissions
    let result_screen = try_access_screen_capture();
    if let Err(e) = &result_screen {
        error!("Failed to access screen capture: {}", e);
    }
    
    // Even if we failed, return Ok since the user might grant permissions later
    Ok(())
}

#[cfg(target_os = "windows")]
fn try_access_microphone() -> Result<()> {
    // Use cpal to try to access the default input device
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
                    error!("Failed to get default input config: {}", e);
                    
                    // Show the user a message about how to enable permissions
                    info!("To enable microphone permissions:");
                    info!("1. Go to Windows Settings > Privacy & Security > Microphone");
                    info!("2. Ensure 'Microphone access' is turned On");
                    info!("3. Ensure this app is allowed to access your microphone");
                    
                    Err(anyhow::anyhow!("Microphone permission denied"))
                }
            }
        },
        None => {
            error!("No default input device found");
            Err(anyhow::anyhow!("No microphone device found"))
        }
    }
}

#[cfg(target_os = "windows")]
fn try_access_screen_capture() -> Result<()> {
    // Screen capture permissions are more complex and might require 
    // Windows Graphics Capture API or similar
    
    // For now, just log a message about how to enable screen capture
    info!("To enable screen capture permissions:");
    info!("1. When prompted by Windows, click 'Yes' to allow screen recording");
    info!("2. If not prompted, go to Windows Settings > Privacy & Security > Screen Recording");
    info!("3. Ensure this app is allowed to access your screen");
    
    // We can't easily test this permission, so just return Ok
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn request_windows_permissions() -> Result<()> {
    // No-op on non-Windows platforms
    Ok(())
}
