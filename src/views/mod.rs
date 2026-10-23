//! 界面层：每个函数返回一个独立元素，供 `SpaceSnifferApp::render` 组合。

use crate::gpui::{self, App};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    ActiveTheme as _, StyledExt as _,
};

mod states;
mod status_bar;
mod toolbar;
mod treemap_view;

pub(crate) use gpui_kit::component::{h_flex, v_flex};

pub(crate) use states::{error_state, landing_state, scanning_state};
pub(crate) use status_bar::status_bar;
pub(crate) use toolbar::toolbar;
pub(crate) use treemap_view::{overlays, treemap_panel};

/// 主操作按钮。
///
/// GPUI Kit 的 `primary` 在深色主题下是 `neutral-50`（近白）填充——
/// 铺在 `neutral-950` 的黑色面板上会非常刺眼。深色下改用主题自带的
/// `secondary`（`neutral-800` 表面 + 一圈 border），既压得住黑底，
/// 又和 ghost 按钮区分得开。
pub(crate) fn cta_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<gpui::SharedString>,
    cx: &App,
) -> Button {
    let button = Button::new(id).label(label);

    if cx.theme().mode.is_dark() {
        // 深色下没有色彩对比可用，靠「深色表面 + 描边 + 加粗字」来保住主次。
        button.secondary().font_semibold()
    } else {
        button.primary()
    }
}
