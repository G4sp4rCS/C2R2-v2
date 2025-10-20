fn main() {
    let manifest_path = "../agent.manifest";
    let mut res = winres::WindowsResource::new();
    res.set_manifest_file(manifest_path);
    res.compile().unwrap();
}
