//! Treemap 主视图。
//!
//! 所有矩形都用一块 `canvas` 自绘：一次 prepaint 算布局 + shape 文本，
//! paint 阶段只往 scene 里塞 quad，几万个格子也不会变成几万个 element。

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::gpui::{
    self, anchored, canvas, deferred, div, fill, font, point, px, AnyElement, App, Bounds, Context,
    Font, FontFallbacks, FontWeight, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, Point, ShapedLine, SharedString, StatefulInteractiveElement as _,
    Styled as _, TextAlign, TextRun, Window,
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
/// 矮于这个高度只放得下一行名字，放不下「名字 + 大小」。
const MIN_TWO_LINE_HEIGHT: f32 = 36.0;

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
    labels: Vec<(ShapedLine, Pixels)>,
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
        |bounds, frame: TreemapFrame, window, cx| paint_frame(frame, bounds, window, cx),
    );

    // 点击/移动只挂在画布这一层：信息卡和角标是它的**兄弟**而不是子节点，
    // 否则点卡片会冒泡到画布的 on_click，按坐标命中下面的格子并钻进目录。
    let surface =
        div()
            .id("treemap-surface")
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

    let mut panel = div()
        .id("treemap-panel")
        .relative()
        .size_full()
        .child(surface);

    // ---- 扫描中的角标 ----
    if stats.scanning {
        panel = panel.child(
            div()
                .id("scan-badge")
                // occlude：挡住指针，别让下面画布的 hitbox 也收到事件
                .occlude()
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

    // 悬停信息卡不在这里画：它要跟着指针走，交给 `overlays()` 用
    // `deferred(anchored(..))` 贴边翻转（见该函数）。

    panel.into_any_element()
}

/// 覆盖层：悬停信息卡、右键菜单。都用窗口坐标定位，由 `deferred` 最后绘制。
pub(crate) fn overlays(
    app: &SpaceSnifferApp,
    cx: &mut Context<SpaceSnifferApp>,
) -> Vec<AnyElement> {
    let mut layers = Vec::new();

    // ---- 悬停信息卡：跟着指针，稍微偏右下，贴边时自动翻转 ----
    if let (Some(pointer), Some(entry)) = (app.hover_pos, app.hovered_entry()) {
        let muted = cx.theme().muted_foreground;
        let overlay = cx.theme().popover;
        let border = cx.theme().border;

        layers.push(
            deferred(
                anchored()
                    .position(pointer + point(px(14.), px(18.)))
                    .snap_to_window_with_margin(px(8.))
                    .child(
                        div()
                            .max_w(px(420.))
                            .rounded_md()
                            .bg(overlay)
                            .border_1()
                            .border_color(border)
                            .shadow_md()
                            .p_3()
                            .child(info_card(entry, muted)),
                    ),
            )
            // 比右键菜单低一层，菜单打开时压在卡片上面
            .with_priority(gpui_kit::base::POPUP_PRIORITY - 1)
            .into_any_element(),
        );
    }

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

    // 0 字节条目（空目录、空文件、Windows 目录联接）在按比例分配里权重为 0，
    // 会被直接跳过——于是既看不见也点不到。给它们一小块保底面积：
    // 单个最多 0.4%，全部加起来最多 10%，按个数摊。
    let zero_count = cells.iter().filter(|cell| cell.size == 0).count();
    let zero_weight = if total == 0 || zero_count == 0 {
        0.0
    } else {
        total as f64 * (0.10 / zero_count as f64).min(0.004)
    };

    let items: Vec<(usize, f64)> = cells
        .iter()
        .map(|cell| {
            let weight = if total == 0 {
                1.0
            } else if cell.size == 0 {
                zero_weight
            } else {
                cell.size as f64
            };
            (cell.index, weight)
        })
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

        // 标签：名字 + 大小两行，整体在格子里居中；格子太矮就只写名字。
        let mut labels = Vec::new();
        if rect.size.width.as_f32() >= MIN_LABEL_WIDTH
            && rect.size.height.as_f32() >= MIN_LABEL_HEIGHT
        {
            let two_lines = rect.size.height.as_f32() >= MIN_TWO_LINE_HEIGHT;
            let height = rect.size.height.as_f32();
            let name_size = (height / if two_lines { 6.0 } else { 4.0 }).clamp(10.0, 15.0);
            let text_color = colors::label_color(color);
            let max_width = rect.size.width - px(6.);
            let name = util::truncate_name(&cell.name, 48);

            if let Some(line) = fit_label(
                window,
                &name,
                &text_font,
                px(name_size),
                text_color,
                max_width,
            ) {
                labels.push((line, px(name_size)));

                // 0 字节不写「0 B」：这些格子多半是没权限读的目录
                //（扫描时被跳过、大小记 0），写死 0 B 会误导。
                //
                // 大小这一行不做截断：放得下就画完整，放不下就不画
                //（「30.8 ..」这种半截比没有更难看）。
                if two_lines && cell.size > 0 {
                    let size_size = (name_size * 0.82).max(9.0);
                    let size_text: SharedString = util::format_size(cell.size).into();
                    let line = shape_line(window, size_text, &text_font, px(size_size), text_color);
                    if line.width() <= max_width {
                        labels.push((line, px(size_size)));
                    }
                }
            }
        }

        painted.push(PaintedCell {
            rect,
            fill: color,
            labels,
        });
    }

    TreemapFrame {
        cells: painted,
        empty: None,
    }
}

fn paint_frame(frame: TreemapFrame, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    // 先铺一层深色底：每个格子的填充各自内缩 0.5px，缝隙露出这层底色。
    // 等于用「1 个背景 quad + 每格 1 个 fill」换回细网格——
    // 之前每格再画一条 1px 描边会被拆成 4 条 strip，几千个格子就是几万个
    // primitive，是整帧最贵的开销（实测 10.9ms → 3.2ms）。
    window.paint_quad(fill(bounds, colors::rgb(18, 18, 20)));

    for cell in &frame.cells {
        let width = cell.rect.size.width.as_f32();
        let height = cell.rect.size.height.as_f32();

        // 亚像素格子看不见，白占一个 primitive。
        if width < 1.0 || height < 1.0 {
            continue;
        }

        let rect = if width > 1.5 && height > 1.5 {
            cell.rect.inset(px(0.5))
        } else {
            cell.rect
        };
        window.paint_quad(fill(rect, cell.fill));

        // 多行标签作为一个整体垂直居中，每行在格子宽度内水平居中。
        let total_height: f32 = cell.labels.iter().map(|(_, height)| height.as_f32()).sum();
        let mut y = cell.rect.center().y - px(total_height / 2.0);

        for (line, line_height) in &cell.labels {
            let _ = line.paint(
                point(cell.rect.origin.x, y),
                *line_height,
                TextAlign::Center,
                Some(cell.rect.size.width),
                window,
                cx,
            );
            y += *line_height;
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

/// 排版一段能放下就行的标签。
///
/// 先用字宽估算裁一刀（省掉大部分 shaping），再按实际宽度逐步收缩：
/// 之前估算偏宽就直接放弃，长中文名会整块空白，现在最多缩到 4 个字符。
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

    let total = text.chars().count();
    let estimate = util::est_text_width(text, font_size.as_f32());
    let mut limit = if estimate <= max_width.as_f32() {
        total
    } else {
        ((max_width.as_f32() / estimate.max(1.0)) * total as f32).floor() as usize
    }
    .clamp(4.min(total), total);

    loop {
        let candidate = util::truncate_name(text, limit.max(4));
        let line = shape_line(window, candidate.into(), text_font, font_size, color);

        if line.width() <= max_width {
            return Some(line);
        }
        if limit <= 4 {
            return None;
        }

        // 按实际超宽比例收缩，至少减 1，几轮就能收敛。
        let ratio = (max_width.as_f32() / line.width().as_f32()).clamp(0.35, 0.9);
        limit = ((limit as f32 * ratio).floor() as usize)
            .min(limit - 1)
            .max(4);
    }
}
