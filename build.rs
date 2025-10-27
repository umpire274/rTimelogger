#[cfg(target_os = "windows")]
fn main() {
    use winresource::WindowsResource;

    // Assicurati che res/rtimelogger.ico esista
    let mut res = WindowsResource::new();
    res.set_icon("res/rtimelogger.ico")
        .set("FileDescription", "rTimelogger CLI")
        .set("ProductName", "rTimelogger")
        .set("OriginalFilename", "rtimelogger.exe")
        .set("FileVersion", env!("CARGO_PKG_VERSION"))
        .set("ProductVersion", env!("CARGO_PKG_VERSION"))
        .compile()
        .expect("Failed to embed icon resource");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
