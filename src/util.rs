use std::path::Path;
use std::process::{Command, Stdio};

/// 1024 进制的可读大小。
///
/// 规则：
/// - 整数不带小数点（`1023 MiB`、`1 GiB`）；
/// - 四舍五入后如果顶到 1024，就进位到下一个单位
///   （`1023.96 MiB` → `1 GiB`，而不是 `1024.0 MiB`）；
/// - 非整数保留一位小数（`1.5 KiB`）。
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    let mut unit = 0usize;
    let mut value = bytes as f64;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    // 先按一位小数四舍五入，再看是否该进位。
    loop {
        let rounded = (value * 10.0).round() / 10.0;
        if rounded >= 1024.0 && unit + 1 < UNITS.len() {
            value = rounded / 1024.0;
            unit += 1;
            continue;
        }
        value = rounded;
        break;
    }

    let number = if value.fract() == 0.0 {
        format!("{}", value as u64)
    } else {
        format!("{value:.1}")
    };

    format!("{number} {}", UNITS[unit])
}

pub fn truncate_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        name.to_string()
    } else {
        let limit = max_chars.saturating_sub(2);
        let truncated: String = name.chars().take(limit).collect();
        truncated + ".."
    }
}

/// 粗估一段文本的像素宽度：CJK 按 0.85em、ASCII 按 0.52em 算。
///
/// 只用来在 `shape_line` 之前先裁一刀，避免每帧为超长文件名反复排版。
pub fn est_text_width(text: &str, font_size: f32) -> f32 {
    let mut wide = 0.0f32;
    let mut narrow = 0.0f32;
    for ch in text.chars() {
        if ch > '\u{2e80}' {
            wide += 1.0;
        } else {
            narrow += 1.0;
        }
    }
    narrow * font_size * 0.52 + wide * font_size * 0.85
}

pub fn dirs_or_home() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .or_else(|_| {
                std::env::var("HOMEDRIVE").map(|d| if !d.ends_with('\\') { d + "\\" } else { d })
            })
            .unwrap_or_else(|_| "C:\\".to_string())
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    }
}

pub fn open_file_location(path: &Path, is_dir: bool) {
    let target = if is_dir {
        path.to_path_buf()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.to_path_buf())
    };

    let result = if cfg!(target_os = "windows") {
        Command::new("explorer")
            .arg(if is_dir { "/open," } else { "/select," })
            .arg(path.as_os_str())
            .stdin(Stdio::null()) // 显式禁用标准流
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open")
            .args(["-R", &target.display().to_string()])
            .spawn()
    } else {
        Command::new("xdg-open").arg(target.as_os_str()).spawn()
    };
    let _ = result;
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    #[test]
    fn small_sizes_stay_in_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn whole_numbers_have_no_decimal_point() {
        assert_eq!(format_size(1023 * MIB), "1023 MiB");
        assert_eq!(format_size(GIB), "1 GiB");
        assert_eq!(format_size(1024), "1 KiB");
    }

    #[test]
    fn fractions_keep_one_decimal() {
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(1023 * MIB + MIB / 2), "1023.5 MiB");
    }

    /// 关键规则：四舍五入顶到 1024 就进位，而不是显示 `1024.0 MiB`。
    #[test]
    fn rounding_carries_into_the_next_unit() {
        // 1 MiB - 1 字节 ≈ 1023.999 KiB
        assert_eq!(format_size(MIB - 1), "1 MiB");
        // 1 GiB - 1 字节 ≈ 1023.999 MiB
        assert_eq!(format_size(GIB - 1), "1 GiB");
        // 1 TiB - 1 字节
        assert_eq!(format_size(GIB * 1024 - 1), "1 TiB");
        // 正好 1024 MiB
        assert_eq!(format_size(1024 * MIB), "1 GiB");
    }

    #[test]
    fn top_unit_does_not_overflow() {
        assert_eq!(format_size(u64::MAX), "16 EiB");
    }

    #[test]
    fn truncate_keeps_short_names_and_marks_long_ones() {
        assert_eq!(truncate_name("abc", 10), "abc");
        assert_eq!(truncate_name("abcdefghij", 6), "abcd..");
    }
}
