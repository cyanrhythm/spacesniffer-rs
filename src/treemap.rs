use egui::{pos2, Rect};

pub fn compute_treemap(items: &[(usize, f64)], bounds: Rect) -> Vec<(usize, Rect)> {
    if items.is_empty() {
        return Vec::new();
    }

    let total: f64 = items.iter().map(|(_, s)| s).sum();
    if total <= 0.0 {
        return Vec::new();
    }

    let mut sorted: Vec<(usize, f64)> = items.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut result = Vec::new();
    squarify(&sorted, 0, bounds, total, &mut result);
    result
}

fn squarify(
    sorted: &[(usize, f64)],
    start: usize,
    bounds: Rect,
    total: f64,
    result: &mut Vec<(usize, Rect)>,
) {
    if start >= sorted.len() || total <= 0.0 {
        return;
    }

    let items = &sorted[start..];

    if items.len() == 1 {
        result.push((items[0].0, bounds));
        return;
    }

    let w = bounds.width() as f64;
    let h = bounds.height() as f64;

    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let horizontal = w >= h;

    // Determine how many items fit in the first row/column
    let mut row_sum = 0.0;
    let mut row_end = 0;

    for (i, item) in items.iter().enumerate() {
        let test_sum = row_sum + item.1;
        let cur_worst = row_aspect_ratio(&items[..i], row_sum, total, w, h, horizontal);
        let new_worst = row_aspect_ratio(&items[..=i], test_sum, total, w, h, horizontal);

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
        w,
        h,
        horizontal,
        result,
    );

    // Recurse on remaining items and remaining space
    let remaining_total = total - row_sum;
    let remaining_rect = if horizontal {
        let row_w = w * row_sum / total;
        Rect::from_min_max(
            pos2(bounds.min.x + row_w as f32, bounds.min.y),
            bounds.max,
        )
    } else {
        let row_h = h * row_sum / total;
        Rect::from_min_max(
            pos2(bounds.min.x, bounds.min.y + row_h as f32),
            bounds.max,
        )
    };

    squarify(sorted, start + row_end, remaining_rect, remaining_total, result);
}

fn row_aspect_ratio(
    row: &[(usize, f64)],
    row_sum: f64,
    total: f64,
    w: f64,
    h: f64,
    horizontal: bool,
) -> f64 {
    if row.is_empty() || row_sum <= 0.0 || total <= 0.0 || w <= 0.0 || h <= 0.0 {
        return f64::MAX;
    }

    let mut max_ar = 0.0f64;

    for &(_, size) in row {
        if size <= 0.0 {
            continue;
        }

        let ar = if horizontal {
            let iw = w * row_sum / total;
            let ih = h * size / row_sum;
            (iw / ih).max(ih / iw)
        } else {
            let iw = w * size / row_sum;
            let ih = h * row_sum / total;
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
    bounds: Rect,
    w: f64,
    h: f64,
    horizontal: bool,
    result: &mut Vec<(usize, Rect)>,
) {
    if horizontal {
        let row_w = w * row_sum / total;
        let mut y = bounds.min.y as f64;

        for &(idx, size) in row {
            if size <= 0.0 {
                continue;
            }
            let item_h = h * size / row_sum;
            let rect = Rect::from_min_max(
                pos2(bounds.min.x, y as f32),
                pos2(bounds.min.x + row_w as f32, (y + item_h) as f32),
            );
            result.push((idx, rect));
            y += item_h;
        }
    } else {
        let row_h = h * row_sum / total;
        let mut x = bounds.min.x as f64;

        for &(idx, size) in row {
            if size <= 0.0 {
                continue;
            }
            let item_w = w * size / row_sum;
            let rect = Rect::from_min_max(
                pos2(x as f32, bounds.min.y),
                pos2((x + item_w) as f32, bounds.min.y + row_h as f32),
            );
            result.push((idx, rect));
            x += item_w;
        }
    }
}
