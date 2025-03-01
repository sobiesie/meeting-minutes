fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    
    #[cfg(target_os = "windows")]
    {
        // Embed the manifest resource on Windows
        embed_resource::compile("app.rc");
        println!("cargo:rerun-if-changed=app.manifest");
        println!("cargo:rerun-if-changed=app.rc");
    }
    
    tauri_build::build()
}
