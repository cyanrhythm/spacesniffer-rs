//! CJK 字体：优先用编译进来的字体，否则回退到系统自带。
//!
//! GPUI 在 Windows 上会走 DirectWrite 的系统字体回退，即便不注册也能显示
//! 中文；这里主动注册一份，是为了让 treemap 画布里的自绘文本也稳定落到
//! 一个已知的中文字体上（见 [`cjk_family`]）。

use std::{borrow::Cow, sync::OnceLock};

use crate::gpui::{App, SharedString};

static CJK_FAMILY: OnceLock<Option<SharedString>> = OnceLock::new();

#[cfg(embedded_font)]
const EMBEDDED_FONT: &[u8] = resources!("fonts/YaHeiMonacoHybrid.ttf");

/// 注册字体，返回实际使用的字体族名。
pub fn install(cx: &mut App) -> Option<SharedString> {
    CJK_FAMILY
        .get_or_init(|| {
            let (family, bytes) = candidate()?;
            cx.text_system().add_fonts(vec![bytes]).ok()?;
            Some(SharedString::from(family))
        })
        .clone()
}

/// 画布文本用的字体族名（未注册成功时为 `None`）。
pub fn cjk_family() -> Option<SharedString> {
    CJK_FAMILY.get().cloned().flatten()
}

fn candidate() -> Option<(&'static str, Cow<'static, [u8]>)> {
    #[cfg(embedded_font)]
    {
        return Some(("YaHei Monaco Hybrid", Cow::Borrowed(EMBEDDED_FONT)));
    }

    #[cfg(not(embedded_font))]
    {
        let paths: &[(&str, &str)] = if cfg!(target_os = "windows") {
            &[
                ("C:\\Windows\\Fonts\\msyh.ttc", "Microsoft YaHei"),
                ("C:\\Windows\\Fonts\\simhei.ttf", "SimHei"),
                ("C:\\Windows\\Fonts\\simsun.ttc", "SimSun"),
            ]
        } else if cfg!(target_os = "macos") {
            &[
                ("/System/Library/Fonts/PingFang.ttc", "PingFang SC"),
                ("/System/Library/Fonts/Hiragino Sans GB.ttc", "Hiragino Sans GB"),
            ]
        } else {
            &[
                (
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "Noto Sans CJK SC",
                ),
                (
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                    "Noto Sans CJK SC",
                ),
                (
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "Noto Sans CJK SC",
                ),
                (
                    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                    "WenQuanYi Micro Hei",
                ),
            ]
        };

        for (path, family) in paths {
            if let Ok(data) = std::fs::read(path) {
                return Some((family, Cow::Owned(data)));
            }
        }
        None
    }
}
