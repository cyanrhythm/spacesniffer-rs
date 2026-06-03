use std::path::Path;
use std::process::Command;

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
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

pub fn est_text_width(text: &str, font_size: f32) -> f32 {
    let mut cjk_count = 0.0f32;
    let mut ascii_count = 0.0f32;
    for ch in text.chars() {
        if ch > '\u{2e80}' {
            cjk_count += 1.0;
        } else {
            ascii_count += 1.0;
        }
    }
    ascii_count * font_size * 0.52 + cjk_count * font_size * 0.85
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
            .arg("/open,")
            .arg(target.as_os_str())
            .spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open")
            .args(["-R", &target.display().to_string()])
            .spawn()
    } else {
        Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
    };
    let _ = result;
}
