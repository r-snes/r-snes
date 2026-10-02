//! This build script is used to set the application icon for the R-SNES emulator on Windows.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("./assets/r-snes.ico");
        res.compile().unwrap();
    }
}
