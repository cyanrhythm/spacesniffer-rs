#![windows_subsystem = "windows"]
rust_i18n::i18n!("locales");
use std::sync::Arc;

mod app;
mod colors;
mod locales;
mod scanner;
mod treemap;
mod util;


fn main() -> eframe::Result {
    locales::init();

    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "SpaceSnifferX",
        options,
        Box::new(|cc| {
            configure_cjk_font(&cc.egui_ctx);
            Ok(Box::new(app::SpaceSnifferApp::new()))
        }),
    )
}

fn load_icon() -> Arc<egui::IconData> {
    let svg_data = include_bytes!("../resources/icon.svg");
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .expect("Failed to parse icon.svg");
    let size = tree.size();
    let scale = 256.0 / size.width().max(size.height());
    let w = (size.width() * scale).ceil() as u32;
    let h = (size.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Arc::new(egui::IconData {
        rgba: pixmap.data().to_vec(),
        width: w,
        height: h,
    })
}

fn configure_cjk_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(data) = try_load_cjk_font() {
        fonts
            .font_data
            .insert("cjk".to_owned(), egui::FontData::from_owned(data).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn try_load_cjk_font() -> Option<Vec<u8>> {
    let paths: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ]
    };

    for path in paths {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}
