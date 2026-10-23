//! Treemap 主视图。
//!
//! 所有矩形都用一块 `canvas` 自绘：一次 prepaint 算布局 + shape 文本，
//! paint 阶段只往 scene 里塞 quad，几万个格子也不会变成几万个 element。

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::gpui::{self, 
    InteractiveElement as _, StatefulInteractiveElement as _, 
    anchored, canvas, deferred, div, fill, font, outline, px, AnyElement, App, BorderStyle, Bounds,
    Context, Font, FontFallbacks, FontWeight, Hsla, IntoElement, MouseButton, ParentElement as _,
    Pixels, Point, ShapedLine, SharedString, Styled as _, TextAlign, TextRun, Window,
};
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use rust_i18n::t;

use crate::app::{CellLayout, SpaceSnifferApp};
use crate::colors;
use crate::scanner::FileEntry;
use crate::treemap;
use crate::util;
use crate::views::v_flex;

const PADDING: f32 = 2.0;
const MIN_LABEL_WIDTH: f32 = 34.0;
const MIN_LABEL_HEIGHT: f32 = 18.0;

/// 传给 canvas prepaint 的一份快照，避免在绘制闭包里再碰实体状态。
struct CellInfo {
    index: usize,
    name: SharedString,
    size: u64,
    color: Hsla,
}

/// 一个已经算好的待绘制格子。
struct PaintedCell {
    rect: Bounds<Pixels>,
    fill: Hsla,
    border: Hsla,
    labels: Vec<(ShapedLine, Point<Pixels>, Pixels)>,
}

struct TreemapFrame {
    cells: Vec<PaintedCell>,
    empty: Option<(ShapedLine, Point<Pixels>, Pixels)>,
}

/// 主区域：Treemap 画布 + 悬浮信息卡 + 扫描徽标。
pub(crate) fn treemap_panel(
    app: &SpaceSnifferApp,
    cx: &mut Context<SpaceSnifferApp>,
) -> AnyElement {
    let Some(node) = app.current_node() else {
        return div().size_full().into_any_element();
    };

    let border = cx.theme().border;
    let muted = cx.theme().muted_foreground;
    let overlay = cx.theme().popover;
    let font_family = cx.theme().font_family.clone();
    let stats = app.scan_stats();

    let cells: Rc<Vec<CellInfo>> = Rc::new(
        node.children
            .iter()
            .enumerate()
            .map(|(index, child)| CellInfo {
                index,
                name: child.name.clone().into(),
                size: child.size,
                color: colors::color_for_entry(child),
            })
            .collect(),
    );

    let hovered = app.hovered_cell;
    let depth = app.nav_stack.len();
    let layout_slot: Rc<RefCell<CellLayout>> = app.cell_layout.clone();
    let bounds_slot = app.canvas_bounds.clone();
    let cjk = crate::fonts::cjk_family();

    let prepaint_cells = cells.clone();
    let prepaint_family = font_family.clone();

    let treemap_canvas = canvas(
        move |bounds, window, _cx| {
            prepaint_frame(
                bounds,
                prepaint_cells,
                hovered,
                depth,
                layout_slot,
                bounds_slot,
                prepaint_family,
                cjk,
                muted,
                window,
            )
        },
        |_bounds, frame: TreemapFrame, window, cx| paint_frame(frame, window, cx),
    );

    let mut panel = div()
        .id("treemap-panel")
        .relative()
        .size_full()
        .on_mouse_move(
            cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                this.on_mouse_move(event, cx)
            }),
        )
        .on_click(cx.listener(|this, event: &gpui::ClickEvent, _window, cx| {
            this.on_left_click(event, cx)
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                this.on_right_click(event.position, window, cx)
            }),
        )
        .child(treemap_canvas.size_full());

    // ---- 扫描中的角标 ----
    if stats.scanning {
        panel = panel.child(
            div()
                .absolute()
                .top_2()
                .right_2()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(overlay)
                .border_1()
                .border_color(border)
                .child(super::states::scanning_badge(app)),
        );
    }

    // ---- 悬停信息卡 ----
    if let Some(child) = app.hovered_cell.and_then(|i| node.children.get(i)) {
        panel = panel.child(
            div()
                .absolute()
                .right_3()
                .bottom_3()
                .max_w(px(420.))
                .rounded_md()
                .bg(overlay)
                .border_1()
                .border_color(border)
                .shadow_md()
                .p_3()
                .child(info_card(child, muted)),
        );
    }

    panel.into_any_element()
}

/// 覆盖层：右键菜单等浮层。用窗口坐标定位，交给 `deferred` 在最后绘制。
pub(crate) fn overlays(
    app: &SpaceSnifferApp,
    _cx: &mut Context<SpaceSnifferApp>,
) -> Vec<AnyElement> {
    let mut layers = Vec::new();

    if let Some(menu) = app.context_menu.clone() {
        layers.push(
            deferred(
                anchored()
                    .position(app.context_pos)
                    .snap_to_window_with_margin(px(8.))
                    .child(menu),
            )
            .with_priority(gpui_kit::base::POPUP_PRIORITY)
            .into_any_element(),
        );
    }

    layers
}

fn info_card(child: &FileEntry, muted: Hsla) -> AnyElement {
    let kind = if child.is_symlink {
        // 链接不跟随，大小按 0 记，这里如实说明。
        t!("tooltip_link_type").to_string()
    } else if child.is_dir {
        t!("tooltip_dir_type", items = child.children.len()).to_string()
    } else if let Some(ext) = child.path.extension().and_then(|e| e.to_str()) {
        t!("tooltip_extension", ext = ext).to_string()
    } else {
        t!("tooltip_file_type").to_string()
    };

    v_flex()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .truncate()
                .child(child.name.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(muted)
                .child(t!("tooltip_size", size = util::format_size(child.size)).to_string()),
        )
        .child(div().text_xs().text_color(muted).child(kind))
        .child(
            div()
                .text_xs()
                .text_color(muted)
                .truncate()
                .child(t!("tooltip_path", path = child.path.display().to_string()).to_string()),
        )
        .into_any_element()
}

// ------------------------------------------------------------------ 绘制

#[allow(clippy::too_many_arguments)]
fn prepaint_frame(
    bounds: Bounds<Pixels>,
    cells: Rc<Vec<CellInfo>>,
    hovered: Option<usize>,
    depth: usize,
    layout_slot: Rc<RefCell<CellLayout>>,
    bounds_slot: Rc<Cell<Option<Bounds<Pixels>>>>,
    font_family: SharedString,
    cjk: Option<SharedString>,
    muted: Hsla,
    window: &mut Window,
) -> TreemapFrame {
    bounds_slot.set(Some(bounds));

    let text_font = text_font(&font_family, &cjk);
    let inner = bounds.inset(px(PADDING));

    if cells.is_empty() {
        *layout_slot.borrow_mut() = Rc::new(Vec::new());
        return TreemapFrame {
            cells: Vec::new(),
            empty: empty_label(window, &text_font, inner, muted),
        };
    }

    let total: u64 = cells.iter().map(|cell| cell.size).sum();
    let items: Vec<(usize, f64)> = cells
        .iter()
        .map(|cell| (cell.index, if total == 0 { 1.0 } else { cell.size as f64 }))
        .collect();

    let layout = treemap::compute_treemap(&items, inner);
    *layout_slot.borrow_mut() = Rc::new(layout.clone());

    let darken = 1.0 - (depth as f32 * 0.06).min(0.4);
    let mut painted = Vec::with_capacity(layout.len());

    for (index, rect) in layout {
        let Some(cell) = cells.get(index) else {
            continue;
        };

        let base = colors::shade(cell.color, darken);
        let color = if hovered == Some(index) {
            colors::hover_tint(base, 0.35)
        } else {
            base
        };

        let mut labels = Vec::new();
        if rect.size.width.as_f32() >= MIN_LABEL_WIDTH
            && rect.size.height.as_f32() >= MIN_LABEL_HEIGHT
        {
            let font_size = (rect.size.height.as_f32() / 4.0).clamp(10.0, 15.0);
            let text_color = colors::label_color(color);
            let max_width = rect.size.width - px(8.);
            let name = util::truncate_name(&cell.name, 40);

            if let Some(line) = fit_label(
                window,
                &name,
                &text_font,
                px(font_size),
                text_color,
                max_width,
            ) {
                let origin = Point {
                    x: rect.origin.x + px(4.),
                    y: rect.center().y - px(font_size / 2.0),
                };
                labels.push((line, origin, px(font_size)));
            }
        }

        painted.push(PaintedCell {
            rect,
            fill: color,
            border: colors::shade(color, 0.55),
            labels,
        });
    }

    TreemapFrame {
        cells: painted,
        empty: None,
    }
}

fn paint_frame(frame: TreemapFrame, window: &mut Window, cx: &mut App) {
    for cell in &frame.cells {
        window.paint_quad(fill(cell.rect, cell.fill));
        window.paint_quad(outline(cell.rect, cell.border, BorderStyle::Solid));

        for (line, origin, line_height) in &cell.labels {
            let _ = line.paint(*origin, *line_height, TextAlign::Left, None, window, cx);
        }
    }

    if let Some((line, origin, line_height)) = &frame.empty {
        let _ = line.paint(*origin, *line_height, TextAlign::Left, None, window, cx);
    }
}

fn empty_label(
    window: &mut Window,
    text_font: &Font,
    area: Bounds<Pixels>,
    color: Hsla,
) -> Option<(ShapedLine, Point<Pixels>, Pixels)> {
    let text: SharedString = t!("empty_directory").to_string().into();
    let font_size = px(14.);
    let line = shape_line(window, text, text_font, font_size, color);
    let origin = Point {
        x: area.center().x - px(line.width().as_f32() / 2.0),
        y: area.center().y - px(font_size.as_f32() / 2.0),
    };
    Some((line, origin, font_size))
}

fn text_font(family: &SharedString, cjk: &Option<SharedString>) -> Font {
    let mut text_font = font(family.clone());
    text_font.weight = FontWeight::NORMAL;
    if let Some(cjk) = cjk {
        text_font.fallbacks = Some(FontFallbacks::from_fonts(vec![cjk.to_string()]));
    }
    text_font
}

fn shape_line(
    window: &mut Window,
    text: SharedString,
    text_font: &Font,
    font_size: Pixels,
    color: Hsla,
) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font: text_font.clone(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };

    window
        .text_system()
        .shape_line(text, font_size, &[run], None)
}

/// 先按字宽估算裁一刀，再取真正 shape 出来宽度合适的版本。
fn fit_label(
    window: &mut Window,
    text: &str,
    text_font: &Font,
    font_size: Pixels,
    color: Hsla,
    max_width: Pixels,
) -> Option<ShapedLine> {
    if text.is_empty() || max_width <= px(0.) {
        return None;
    }

    let estimate = util::est_text_width(text, font_size.as_f32());
    let drawn = if estimate <= max_width.as_f32() {
        text.to_string()
    } else {
        let char_count = text.chars().count().max(1);
        let keep = ((max_width.as_f32() / estimate.max(1.0)) * char_count as f32).floor() as usize;
        util::truncate_name(text, keep.clamp(2, char_count))
    };

    if drawn.is_empty() {
        return None;
    }

    let line = shape_line(window, drawn.into(), text_font, font_size, color);
    (line.width() <= max_width).then_some(line)
}
