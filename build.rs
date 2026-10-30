fn main() {
    if cfg!(target_os = "windows") {
        // tauri-winres 内部委托 embed-resource 找资源编译器
        //（msvc → llvm-rc / rc.exe，gnu → windres），交叉编译也不用自己喂路径。
        //
        // 注意：**不要**在这里 set_manifest —— gpui 的 windows-manifest
        // feature 已经用 embed-resource 往 exe 里塞了 RT_MANIFEST，
        // 再加一个会撞同一个资源 ID。
        let mut res = tauri_winres::WindowsResource::new();
        // 与 Cargo.toml 的 [package.metadata.packager] 保持一致：
        // 资源管理器「属性 → 详细信息」读的是这里的资源；安装包里的产品名/版本
        // 来自 packager 配置，两处要一起改。FileVersion / ProductVersion
        // 由 tauri-winres 取 CARGO_PKG_VERSION，不会与 Cargo.toml 脱节。
        res.set_icon("resources/icons/icon.ico")
            .set("CompanyName", "CYANRHYTHM")
            .set("FileDescription", "SpaceSniffer-RS")
            .set("InternalName", "spacesniffer-rs")
            .set("OriginalFilename", "spacesniffer-rs.exe")
            .set("ProductName", "SpaceSniffer-RS")
            .set("LegalCopyright", "Copyright (c) 2026 CYANRHYTHM")
            // 简体中文（0x0804）优先，探测不到就沿用默认语言。
            .set_language(0x0804);
        res.compile().unwrap();
    }

    println!("cargo::rustc-check-cfg=cfg(embedded_font)");

    if std::path::Path::new("resources/fonts/YaHeiMonacoHybrid.ttf").exists() {
        println!("cargo:rustc-cfg=embedded_font");
        println!("cargo:rerun-if-changed=resources/fonts/YaHeiMonacoHybrid.ttf");
    }

    if std::path::Path::new("resources/icons/icon.svg").exists() {
        println!("cargo:rerun-if-changed=resources/icons/icon.svg");
    }
}
