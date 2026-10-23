//! 底部状态栏：总量、跳过数、颜色图例与条目计数。

use crate::gpui::{self, 
    prelude::FluentBuilder as _, px, AnyElement, Context, Hsla, IntoElement, ParentElement as _, Styled as _,};
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use rust_i18n::t;

use crate::app::SpaceSnifferApp;
use crate::colors;
use crate::util;
use crate::views::h_flex;

pub(crate) fn status_bar(
    app: &SpaceSnifferApp,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let border = cx.theme().border;
    let muted = cx.theme().muted_foreground;

    let stats = app.scan_stats();
    let total = app.root.as_ref().map(|root| root.size).unwrap_or(0);
    let items = app.current_node().map(|node| node.children.len()).unwrap_or(0);

    let legends = [
        (colors::DIR_COLOR, t!("legend_dir").to_string()),
        (colors::color_for_extension("pdf"), t!("legend_doc").to_string()),
        (colors::color_for_extension("rs"), t!("legend_code").to_string()),
        (colors::color_for_extension("png"), t!("legend_img").to_string()),
        (colors::color_for_extension("mp4"), t!("legend_vid").to_string()),
        (colors::color_for_extension("mp3"), t!("legend_aud").to_string()),
        (colors::color_for_extension("zip"), t!("legend_arc").to_string()),
        (colors::color_for_extension("exe"), t!("legend_exe").to_string()),
        (colors::color_for_extension(""), t!("legend_oth").to_string()),
    ];

    h_flex()
        .w_full()
        .px_3()
        .py_1()
        .gap_2()
        .items_center()
        .border_t_1()
        .border_color(border)
        .text_xs()
        .child(
            gpui::div()
                .font_semibold()
                .child(t!("total_label", total = util::format_size(total)).to_string()),
        )
        .when(stats.skipped > 0, |this| {
            this.child(gpui::div().text_color(muted).child(
                t!("dirs_skipped_label", skipped = stats.skipped).to_string(),
            ))
        })
        .child(gpui::div().w(px(1.)).h(px(12.)).bg(border))
        .children(
            legends
                .into_iter()
                .map(|(color, label)| legend_item(color, label, muted)),
        )
        .child(gpui::div().flex_1())
        .child(
            gpui::div()
                .text_color(muted)
                .child(t!("items_count_label", count = items).to_string()),
        )
        .into_any_element()
}

fn legend_item(color: Hsla, label: String, muted: Hsla) -> AnyElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            gpui::div()
                .w(px(9.))
                .h(px(9.))
                .rounded_sm()
                .bg(color),
        )
        .child(gpui::div().text_color(muted).child(label))
        .into_any_element()
}
