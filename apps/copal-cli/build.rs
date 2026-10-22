//! Windows only: puts the crystal icon and version info into the .exe.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/copal.ico")
            .set("FileDescription", "Copal CLI")
            .set("ProductName", "Copal")
            .set(
                "LegalCopyright",
                "MIT License, hakiguru and Copal contributors",
            );
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
