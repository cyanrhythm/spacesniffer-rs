use std::path::Path;

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
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

pub fn open_file_location(path: &Path) {
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path.as_os_str())
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .args(["-R", &path.display().to_string()])
            .spawn()
    } else {
        std::process::Command::new("xdg-open")
            .arg(path.as_os_str())
            .spawn()
    };
    let _ = result;
}
