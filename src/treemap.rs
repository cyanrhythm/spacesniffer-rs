//! 平方化 Treemap 布局算法（Bruls, Huizing, van Wijk）。
//!
//! 纯几何计算，只依赖 GPUI 的 `Bounds<Pixels>` 作为输出容器，
//! 不涉及任何渲染或状态。

use crate::gpui::{point, px, size, Bounds, Pixels};

/// 一个待布局的最小矩形，内部用 f64 计算以避免 f32 累积误差。
#[derive(Clone, Copy, Debug)]
struct RectF {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl RectF {
    fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    fn to_bounds(self) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(self.x as f32), px(self.y as f32)),
            size: size(px(self.w as f32), px(self.h as f32)),
        }
    }
}

/// 计算 `items`（索引 + 权重）在 `bounds` 内的平方化布局。
///
/// 返回的矩形按输入索引输出，顺序与调用方传入的 `items` 一致（不保证有序）。
pub fn compute_treemap(
    items: &[(usize, f64)],
    bounds: Bounds<Pixels>,
) -> Vec<(usize, Bounds<Pixels>)> {
    if items.is_empty() {
        return Vec::new();
    }

    let total: f64 = items.iter().map(|(_, s)| s).sum();
    if total <= 0.0 {
        return Vec::new();
    }

    let mut sorted: Vec<(usize, f64)> = items.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let root = RectF::new(
        bounds.origin.x.as_f32() as f64,
        bounds.origin.y.as_f32() as f64,
        bounds.size.width.as_f32() as f64,
        bounds.size.height.as_f32() as f64,
    );

    let mut result = Vec::with_capacity(items.len());
    squarify(&sorted, 0, root, total, &mut result);
    result
}

fn squarify(
    sorted: &[(usize, f64)],
    start: usize,
    bounds: RectF,
    total: f64,
    result: &mut Vec<(usize, Bounds<Pixels>)>,
) {
    if start >= sorted.len() || total <= 0.0 || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return;
    }

    let items = &sorted[start..];

    if items.len() == 1 {
        result.push((items[0].0, bounds.to_bounds()));
        return;
    }

    let horizontal = bounds.w >= bounds.h;

    // 贪心地往同一行/列里塞元素，直到长宽比不再变好。
    let mut row_sum = 0.0;
    let mut row_end = 0;

    for (i, item) in items.iter().enumerate() {
        let test_sum = row_sum + item.1;
        let cur_worst = row_aspect_ratio(&items[..i], row_sum, total, &bounds, horizontal);
        let new_worst = row_aspect_ratio(&items[..=i], test_sum, total, &bounds, horizontal);

        if i == 0 || new_worst <= cur_worst {
            row_sum = test_sum;
            row_end = i + 1;
        } else {
            break;
        }
    }

    if row_end == 0 {
        row_end = 1;
        row_sum = items[0].1;
    }

    layout_row(
        &items[..row_end],
        row_sum,
        total,
        bounds,
        horizontal,
        result,
    );

    // 剩余元素在剩下的空间里递归布局。
    let remaining_total = total - row_sum;
    let remaining_rect = if horizontal {
        let row_w = bounds.w * row_sum / total;
        RectF::new(bounds.x + row_w, bounds.y, bounds.w - row_w, bounds.h)
    } else {
        let row_h = bounds.h * row_sum / total;
        RectF::new(bounds.x, bounds.y + row_h, bounds.w, bounds.h - row_h)
    };

    squarify(
        sorted,
        start + row_end,
        remaining_rect,
        remaining_total,
        result,
    );
}

fn row_aspect_ratio(
    row: &[(usize, f64)],
    row_sum: f64,
    total: f64,
    bounds: &RectF,
    horizontal: bool,
) -> f64 {
    if row.is_empty() || row_sum <= 0.0 || total <= 0.0 {
        return f64::MAX;
    }

    let mut max_ar: f64 = 0.0;

    for &(_, item_size) in row {
        if item_size <= 0.0 {
            continue;
        }

        let ar = if horizontal {
            let iw = bounds.w * row_sum / total;
            let ih = bounds.h * item_size / row_sum;
            if ih <= 0.0 {
                continue;
            }
            (iw / ih).max(ih / iw)
        } else {
            let iw = bounds.w * item_size / row_sum;
            let ih = bounds.h * row_sum / total;
            if iw <= 0.0 {
                continue;
            }
            (iw / ih).max(ih / iw)
        };

        if ar > max_ar {
            max_ar = ar;
        }
    }

    max_ar
}

fn layout_row(
    row: &[(usize, f64)],
    row_sum: f64,
    total: f64,
    bounds: RectF,
    horizontal: bool,
    result: &mut Vec<(usize, Bounds<Pixels>)>,
) {
    if row_sum <= 0.0 {
        return;
    }

    if horizontal {
        let row_w = bounds.w * row_sum / total;
        let mut y = bounds.y;

        for &(idx, item_size) in row {
            if item_size <= 0.0 {
                continue;
            }
            let item_h = bounds.h * item_size / row_sum;
            result.push((idx, RectF::new(bounds.x, y, row_w, item_h).to_bounds()));
            y += item_h;
        }
    } else {
        let row_h = bounds.h * row_sum / total;
        let mut x = bounds.x;

        for &(idx, item_size) in row {
            if item_size <= 0.0 {
                continue;
            }
            let item_w = bounds.w * item_size / row_sum;
            result.push((idx, RectF::new(x, bounds.y, item_w, row_h).to_bounds()));
            x += item_w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Bounds<Pixels> {
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(100.), px(100.)),
        }
    }

    #[test]
    fn empty_input_produces_no_rects() {
        assert!(compute_treemap(&[], bounds()).is_empty());
        assert!(compute_treemap(&[(0, 0.0), (1, 0.0)], bounds()).is_empty());
    }

    #[test]
    fn single_item_fills_the_canvas() {
        let rects = compute_treemap(&[(7, 42.0)], bounds());
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, 7);
        let b = rects[0].1;
        assert!((b.size.width.as_f32() - 100.0).abs() < 0.01);
        assert!((b.size.height.as_f32() - 100.0).abs() < 0.01);
    }

    #[test]
    fn areas_are_proportional_to_weights() {
        let items = [(0, 50.0), (1, 30.0), (2, 20.0)];
        let rects = compute_treemap(&items, bounds());
        assert_eq!(rects.len(), 3);

        let total_area: f32 = rects
            .iter()
            .map(|(_, b)| b.size.width.as_f32() * b.size.height.as_f32())
            .sum();
        assert!(
            (total_area - 10_000.0).abs() < 1.0,
            "total area {total_area}"
        );

        for (idx, rect) in &rects {
            let weight = items.iter().find(|(i, _)| i == idx).unwrap().1 as f32;
            let expected = 10_000.0 * weight / 100.0;
            let area = rect.size.width.as_f32() * rect.size.height.as_f32();
            assert!(
                (area - expected).abs() < 40.0,
                "item {idx}: area {area} vs {expected}"
            );
        }
    }

    #[test]
    fn rects_stay_inside_bounds() {
        let items: Vec<(usize, f64)> = (0..17).map(|i| (i, (i as f64 + 1.0) * 3.7)).collect();
        for (_, rect) in compute_treemap(&items, bounds()) {
            assert!(rect.origin.x.as_f32() >= -0.01);
            assert!(rect.origin.y.as_f32() >= -0.01);
            assert!(rect.origin.x.as_f32() + rect.size.width.as_f32() <= 100.01);
            assert!(rect.origin.y.as_f32() + rect.size.height.as_f32() <= 100.01);
        }
    }
}
