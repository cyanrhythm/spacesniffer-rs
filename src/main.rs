#![windows_subsystem = "windows"]
rust_i18n::i18n!("locales");

/// 把 `resources/` 下的文件编进二进制。
#[macro_export]
macro_rules! resources {
    ($path:expr) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/", $path))
    };
}

/// GPUI Kit 把 GPUI 本体在根路径下重新导出，这里起个别名，
/// 让各模块照旧写 `gpui::...`，不用关心底层是哪个 crate。
pub(crate) use gpui_kit as gpui;

mod app;
mod colors;
mod fonts;
mod locales;
mod scanner;
mod treemap;
mod util;
mod views;

use crate::gpui::{px, size, AppContext as _, TitlebarOptions, WindowBounds, WindowOptions};

fn main() {
    locales::init();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            // GPUI Kit 的主题、Root 插件、菜单与提示层都在这里注册。
            gpui_kit::init(cx);
            fonts::install(cx);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1200.), px(800.)), cx)),
                window_min_size: Some(size(px(640.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("SpaceSniffer-RS".into()),
                    ..Default::default()
                }),
                app_id: Some("spacesniffer-rs".into()),
                ..Default::default()
            };

            // open_window 会自动挂上 gpui-base 的 Root，
            // 对话框 / 通知 / 菜单的浮层都靠它。
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| app::SpaceSnifferApp::new(window, cx))
            })
            .expect("failed to open window");
        });
}
