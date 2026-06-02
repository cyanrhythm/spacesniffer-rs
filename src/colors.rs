use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use egui::Color32;

use crate::scanner::FileEntry;

pub fn color_for_entry(entry: &FileEntry) -> Color32 {
    if entry.is_dir {
        return Color32::from_rgb(70, 130, 180);
    }

    let ext = entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    color_for_extension(&ext)
}

pub fn color_for_extension(ext: &str) -> Color32 {
    match ext {
        "pdf" | "doc" | "docx" | "odt" | "rtf" => Color32::from_rgb(200, 50, 50),
        "txt" | "md" | "rst" | "log" => Color32::from_rgb(100, 150, 200),
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb"
        | "php" | "swift" | "kt" | "scala" | "lua" | "pl" | "sh" | "bash" | "zsh" | "sql" => {
            Color32::from_rgb(80, 200, 120)
        }
        "html" | "htm" | "css" | "scss" | "less" | "xml" | "json" | "yaml" | "yml" | "toml" => {
            Color32::from_rgb(220, 150, 50)
        }
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "psd" => {
            Color32::from_rgb(200, 100, 200)
        }
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => Color32::from_rgb(220, 80, 80),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => Color32::from_rgb(100, 100, 220),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "iso" => {
            Color32::from_rgb(200, 200, 80)
        }
        "exe" | "dll" | "so" | "dylib" | "app" | "msi" | "deb" | "rpm" | "apk" => {
            Color32::from_rgb(160, 160, 160)
        }
        "ttf" | "otf" | "woff" | "woff2" | "eot" => Color32::from_rgb(180, 130, 80),
        _ if ext.is_empty() => Color32::from_rgb(140, 140, 140),
        _ => {
            let mut hasher = DefaultHasher::new();
            ext.hash(&mut hasher);
            let hash = hasher.finish();
            let hue = ((hash as f64) * 0.618033988749895).fract() as f32;
            let s = 0.65;
            let v = 0.75;
            let (r, g, b) = hsv_to_rgb(hue, s, v);
            Color32::from_rgb(r, g, b)
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
