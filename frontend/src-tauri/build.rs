fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    
    #[cfg(target_os = "windows")]
    {
        // Add Windows-specific build configurations
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=avrt");
        println!("cargo:rustc-link-lib=ksuser");
    }
    
    tauri_build::build()
}
