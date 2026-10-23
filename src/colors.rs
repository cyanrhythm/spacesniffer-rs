//! 文件类型 → 颜色的映射。
//!
//! 输出 GPUI 的 [`Hsla`]，方便直接用于 `paint_quad` / 文本运行。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::gpui::{hsla, Hsla, Rgba};

use crate::scanner::FileEntry;

/// 目录统一使用的蓝灰色（steelblue）。
pub const DIR_COLOR: Hsla = hsla(0.5756, 0.4400, 0.4902, 1.0);

pub fn color_for_entry(entry: &FileEntry) -> Hsla {
    if entry.is_dir {
        return dir_shade(&entry.name);
    }

    let ext = entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    color_for_extension(&ext)
}

/// 目录统一是蓝色，但按名字哈希微调明度：同级目录不会糊成一整块，
/// 又不至于抢走"目录 = 蓝"的语义。
pub fn dir_shade(name: &str) -> Hsla {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let t = (hasher.finish() % 1000) as f32 / 1000.0;
    let lightness = (DIR_COLOR.l + (t - 0.5) * 0.18).clamp(0.3, 0.68);

    Hsla {
        l: lightness,
        ..DIR_COLOR
    }
}

pub fn color_for_extension(ext: &str) -> Hsla {
    match ext {
        "pdf" | "doc" | "docx" | "odt" | "rtf" => rgb(200, 50, 50),
        "txt" | "md" | "rst" | "log" => rgb(100, 150, 200),
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb"
        | "php" | "swift" | "kt" | "scala" | "lua" | "pl" | "sh" | "bash" | "zsh" | "sql" => {
            rgb(80, 200, 120)
        }
        "html" | "htm" | "css" | "scss" | "less" | "xml" | "json" | "yaml" | "yml" | "toml" => {
            rgb(220, 150, 50)
        }
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "psd" => {
            rgb(200, 100, 200)
        }
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => rgb(220, 80, 80),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => rgb(100, 100, 220),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "iso" => rgb(200, 200, 80),
        "exe" | "dll" | "so" | "dylib" | "app" | "msi" | "deb" | "rpm" | "apk" => {
            rgb(160, 160, 160)
        }
        "ttf" | "otf" | "woff" | "woff2" | "eot" => rgb(180, 130, 80),
        _ if ext.is_empty() => rgb(140, 140, 140),
        _ => {
            let mut hasher = DefaultHasher::new();
            ext.hash(&mut hasher);
            let hash = hasher.finish();
            let hue = ((hash as f64) * 0.618033988749895).fract() as f32;
            hsla(hue, 0.65, 0.55, 1.0)
        }
    }
}

/// 按比例降低亮度，用于表达"进入得越深颜色越暗"。
pub fn shade(color: Hsla, factor: f32) -> Hsla {
    Hsla {
        l: (color.l * factor).clamp(0.0, 1.0),
        ..color
    }
}

/// 悬停高亮：向白色混合。
pub fn hover_tint(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l + amount * (1.0 - color.l)).clamp(0.0, 1.0),
        s: (color.s * (1.0 - amount * 0.5)).clamp(0.0, 1.0),
        ..color
    }
}

/// 在给定背景上挑一个对比度够高的文字颜色。
pub fn label_color(background: Hsla) -> Hsla {
    let rgba = Rgba::from(background);
    let luminance = 0.299 * rgba.r + 0.587 * rgba.g + 0.114 * rgba.b;
    if luminance > 0.62 {
        rgb(24, 24, 27)
    } else {
        rgb(255, 255, 255)
    }
}

/// `r,g,b` (0-255) → Hsla。
pub fn rgb(r: u8, g: u8, b: u8) -> Hsla {
    Hsla::from(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: Hsla, b: Hsla) -> bool {
        u32::from(Rgba::from(a)) == u32::from(Rgba::from(b))
    }

    #[test]
    fn directories_use_the_blue_tone() {
        let entry = FileEntry {
            name: "src".into(),
            path: "src".into(),
            size: 1,
            children: Vec::new(),
            is_dir: true,
            is_symlink: false,
        };
        let color = color_for_entry(&entry);
        // 色相仍然是蓝色那一档，只是明度略有浮动。
        assert!((color.h - DIR_COLOR.h).abs() < 0.001);
        assert!((color.l - DIR_COLOR.l).abs() <= 0.1);
    }

    #[test]
    fn sibling_directories_are_distinguishable() {
        assert_ne!(dir_shade("target").l, dir_shade("package").l);
    }

    #[test]
    fn known_extensions_are_stable() {
        assert!(same(color_for_extension("rs"), color_for_extension("rs")));
        assert!(!same(color_for_extension("rs"), color_for_extension("png")));
    }

    #[test]
    fn shading_keeps_hue_and_darkens() {
        let c = rgb(200, 100, 50);
        let dark = shade(c, 0.7);
        assert_eq!(c.h, dark.h);
        assert!(dark.l < c.l);
    }

    #[test]
    fn labels_contrast_with_their_background() {
        let light = rgb(230, 230, 230);
        let dark = rgb(20, 20, 20);
        assert!(label_color(light).l < 0.5);
        assert!(label_color(dark).l > 0.5);
    }
}
