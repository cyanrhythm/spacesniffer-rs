//! 主区域的三种空状态：欢迎页、扫描中、出错。

use crate::gpui::{self, 
    prelude::FluentBuilder as _, px, AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    progress::Progress,
    spinner::Spinner,
    ActiveTheme as _, Sizable as _, StyledExt as _,
};
use rust_i18n::t;

use crate::app::SpaceSnifferApp;
use crate::views::v_flex;

pub(crate) fn landing_state(
    _app: &mut SpaceSnifferApp,
    _window: &mut Window,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;

    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            gpui::div()
                .text_2xl()
                .font_semibold()
                .child(t!("landing_heading").to_string()),
        )
        .child(
            gpui::div()
                .max_w(px(420.))
                .text_center()
                .text_color(muted)
                .child(t!("landing_instruction").to_string()),
        )
        .child(
            Button::new("landing-browse")
                .primary()
                .label(t!("landing_browse_button").to_string())
                .on_click(cx.listener(|this, _, window, cx| this.browse_for_folder(window, cx))),
        )
        .into_any_element()
}

pub(crate) fn scanning_state(
    app: &mut SpaceSnifferApp,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let stats = app.scan_stats();

    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            gpui::div()
                .text_xl()
                .font_semibold()
                .child(t!("scanning_heading").to_string()),
        )
        .child(
            gpui::div()
                .text_color(muted)
                .child(t!("scanning_files_dirs", files = stats.files, dirs = stats.dirs).to_string()),
        )
        .when(!stats.current_path.is_empty(), |this| {
            this.child(
                gpui::div()
                    .max_w(px(520.))
                    .text_xs()
                    .text_color(muted)
                    .truncate()
                    .child(t!("scanning_current", path = &stats.current_path).to_string()),
            )
        })
        .child(
            gpui::div()
                .w(px(320.))
                .child(Progress::new("scan-progress").loading(true)),
        )
        .into_any_element()
}

pub(crate) fn error_state(
    app: &mut SpaceSnifferApp,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let danger = cx.theme().danger;
    let message = app.error_message.clone().unwrap_or_default();

    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            gpui::div()
                .text_color(danger)
                .max_w(px(520.))
                .text_center()
                .child(t!("error_label", msg = &message).to_string()),
        )
        .child(
            Button::new("retry")
                .primary()
                .label(t!("retry_button").to_string())
                .on_click(cx.listener(|this, _, window, cx| this.retry(window, cx))),
        )
        .into_any_element()
}

/// 扫描遮罩里的小徽标：转圈 + 计数。
pub(crate) fn scanning_badge(app: &SpaceSnifferApp) -> AnyElement {
    let stats = app.scan_stats();

    v_flex()
        .gap_1()
        .child(
            crate::views::h_flex()
                .gap_2()
                .items_center()
                .child(Spinner::new().small())
                .child(
                    gpui::div()
                        .text_xs()
                        .child(t!("scanning_files_dirs", files = stats.files, dirs = stats.dirs).to_string()),
                ),
        )
        .into_any_element()
}
