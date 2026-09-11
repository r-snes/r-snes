fn main() {
    // Check the *target* OS, not cfg!(windows): build scripts run on the host,
    // so cfg!(windows) would be wrong when cross-compiling.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("logo.ico");
        res.compile().expect("failed to embed Windows resources");
    }
}
