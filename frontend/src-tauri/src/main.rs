use log;
use env_logger;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    log::info!("Starting application...");
    
    // Request Windows permissions at startup
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = app_lib::windows_permissions::request_windows_permissions() {
            log::error!("Failed to request Windows permissions: {}", e);
        } else {
            log::info!("Windows permissions requested successfully");
        }
    }
    
    app_lib::run();
}
