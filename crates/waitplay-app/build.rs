//! Windows: embeds the Vibe Play icon and version info in `waitplay.exe`, so Explorer, the Start menu, search and the taskbar show
//! the logo and the right name instead of a blank application icon.

fn main() {
    println!("cargo:rerun-if-changed=assets/vibeplay.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/vibeplay.ico");
        res.set("ProductName", "Vibe Play");
        res.set("FileDescription", "Vibe Play");
        res.set("InternalName", "waitplay");
        res.set("OriginalFilename", "waitplay.exe");
        if let Err(e) = res.compile() {
            // a missing resource compiler must not break a plain `cargo build`; the exe just has no icon then
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
