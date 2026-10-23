//! 顶部工具栏：路径输入、扫描控制、面包屑导航。

use crate::gpui::{self, 
    px, AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window};
use gpui_kit::component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, ButtonVariants as _},
    input::Input,
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
};
use rust_i18n::t;

use crate::app::SpaceSnifferApp;
use crate::views::{h_flex, v_flex};

pub(crate) fn toolbar(
    app: &mut SpaceSnifferApp,
    _window: &mut Window,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let border = cx.theme().border;
    let scanning = app.scan_stats().scanning;

    h_flex()
        .w_full()
        .px_3()
        .py_2()
        .gap_2()
        .items_center()
        .flex_wrap()
        .border_b_1()
        .border_color(border)
        .child(
            v_flex()
                .gap_0p5()
                .child(
                    gpui::div()
                        .text_base()
                        .font_semibold()
                        .child("SpaceSniffer-RS"),
                )
                .child(
                    gpui::div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Treemap · GPUI Kit"),
                ),
        )
        .child(gpui::div().w(px(1.)).h(px(24.)).bg(border))
        .child(
            gpui::div()
                .w(px(320.))
                .child(Input::new(&app.scan_path).cleanable(true).small()),
        )
        .child(
            Button::new("scan")
                .primary()
                .small()
                .label(t!("scan_button").to_string())
                .on_click(cx.listener(|this, _, window, cx| this.start_scan(window, cx))),
        )
        .child(
            Button::new("cancel")
                .ghost()
                .small()
                .label(t!("cancel_button").to_string())
                .disabled(!scanning)
                .on_click(cx.listener(|this, _, _window, cx| this.cancel_scan_from_ui(cx))),
        )
        .child(
            Button::new("browse")
                .ghost()
                .small()
                .label(t!("browse_button").to_string())
                .on_click(cx.listener(|this, _, window, cx| this.browse_for_folder(window, cx))),
        )
        .child(gpui::div().w(px(1.)).h(px(24.)).bg(border))
        .child(
            v_flex()
                .flex_1()
                .min_w(px(160.))
                .overflow_hidden()
                .child(breadcrumb(app, cx)),
        )
        .into_any_element()
}

fn breadcrumb(app: &SpaceSnifferApp, cx: &mut Context<SpaceSnifferApp>) -> Breadcrumb {
    let mut breadcrumb = Breadcrumb::new();
    let last_index = app.nav_names.len().saturating_sub(1);

    for (index, name) in app.nav_names.iter().enumerate() {
        let item = BreadcrumbItem::new(name.clone());
        if index < last_index {
            // 非当前层可点击跳转。
            let item = item.on_click(cx.listener(move |this, _, _window, cx| {
                this.navigate_to_level(index, cx);
            }));
            breadcrumb = breadcrumb.child(item);
        } else {
            breadcrumb = breadcrumb.child(item.disabled(true));
        }
    }

    breadcrumb
}
