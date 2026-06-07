fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.compile().unwrap();
    }

    println!("cargo::rustc-check-cfg=cfg(embedded_font)");

    if std::path::Path::new("fonts/YaHeiMonacoHybrid.ttf").exists() {
        println!("cargo:rustc-cfg=embedded_font");
        println!("cargo:rerun-if-changed=fonts/YaHeiMonacoHybrid.ttf");
    }
}
