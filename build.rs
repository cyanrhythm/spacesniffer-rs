fn main() {
    if cfg!(target_os = "windows") {
        // tauri-winres 内部委托 embed-resource 找资源编译器
        //（msvc → llvm-rc / rc.exe，gnu → windres），交叉编译也不用自己喂路径。
        //
        // 注意：**不要**在这里 set_manifest —— gpui 的 windows-manifest
        // feature 已经用 embed-resource 往 exe 里塞了 RT_MANIFEST，
        // 再加一个会撞同一个资源 ID。
        let mut res = tauri_winres::WindowsResource::new();
        res.set_icon("resources/icons/icon.ico");
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
