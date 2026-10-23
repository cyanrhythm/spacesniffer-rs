//! 界面层：每个函数返回一个独立元素，供 `SpaceSnifferApp::render` 组合。

mod states;
mod status_bar;
mod toolbar;
mod treemap_view;

pub(crate) use gpui_kit::component::{h_flex, v_flex};

pub(crate) use states::{error_state, landing_state, scanning_state};
pub(crate) use status_bar::status_bar;
pub(crate) use toolbar::toolbar;
pub(crate) use treemap_view::{overlays, treemap_panel};
