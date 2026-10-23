//! CJK 字体：优先用编译进来的字体，否则回退到系统自带。
//!
//! GPUI 在 Windows 上会走 DirectWrite 的系统字体回退，即便不注册也能显示
//! 中文；这里主动注册一份，是为了让 treemap 画布里的自绘文本也稳定落到
//! 一个已知的中文字体上（见 [`cjk_family`]）。
//!
//! ⚠️ 族名必须以字体文件里的**真实**名字为准：内嵌字体作者把 `Hybrid`
//! 拼成了 `Hybird`，写"正确"拼写反而查不到。所以这里注册完再回查一次
//! [`crate::gpui::TextSystem::all_font_names`]，用实际注册成功的族名。

use std::{borrow::Cow, sync::OnceLock};

use crate::gpui::{App, SharedString};

static CJK_FAMILY: OnceLock<Option<SharedString>> = OnceLock::new();

#[cfg(embedded_font)]
const EMBEDDED_FONT: &[u8] = resources!("fonts/YaHeiMonacoHybrid.ttf");

/// 内嵌字体的族名（拼写以字体内部 name 表为准）。
#[cfg(embedded_font)]
const EMBEDDED_FAMILY: &str = "YaHei Monaco Hybird";

/// 注册字体，返回实际生效的字体族名。
pub fn install(cx: &mut App) -> Option<SharedString> {
    CJK_FAMILY
        .get_or_init(|| {
            let (hint, bytes) = candidate()?;
            cx.text_system().add_fonts(vec![bytes]).ok()?;

            let available = cx.text_system().all_font_names();
            Some(SharedString::from(pick_family(&available, hint)))
        })
        .clone()
}

/// 画布文本用的字体族名（未注册成功时为 `None`）。
pub fn cjk_family() -> Option<SharedString> {
    CJK_FAMILY.get().cloned().flatten()
}

/// 从已注册的族名里挑出目标字体：精确匹配优先，其次按前缀（去掉最后一个
/// 词）匹配，都对不上就直接用传入的名字——宁可交给系统 fallback，也不要
/// 静默失效。
fn pick_family(available: &[String], hint: &str) -> String {
    if let Some(name) = available.iter().find(|name| name.as_str() == hint) {
        return name.clone();
    }

    let stem = hint.rsplit_once(' ').map(|(stem, _)| stem).unwrap_or(hint);
    if let Some(name) = available.iter().find(|name| name.starts_with(stem)) {
        return name.clone();
    }

    hint.to_string()
}

#[cfg(embedded_font)]
fn candidate() -> Option<(&'static str, Cow<'static, [u8]>)> {
    Some((EMBEDDED_FAMILY, Cow::Borrowed(EMBEDDED_FONT)))
}

#[cfg(not(embedded_font))]
fn candidate() -> Option<(&'static str, Cow<'static, [u8]>)> {
    let paths: &[(&str, &str)] = if cfg!(target_os = "windows") {
        &[
            ("C:\\Windows\\Fonts\\msyh.ttc", "Microsoft YaHei"),
            ("C:\\Windows\\Fonts\\simhei.ttf", "SimHei"),
            ("C:\\Windows\\Fonts\\simsun.ttc", "SimSun"),
        ]
    } else if cfg!(target_os = "macos") {
        &[
            ("/System/Library/Fonts/PingFang.ttc", "PingFang SC"),
            (
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
                "Hiragino Sans GB",
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 内嵌字体的真实族名是 Hybird（字体作者的拼写问题）。
    #[test]
    fn picks_the_registered_spelling() {
        let available = vec![
            "Segoe UI".to_string(),
            "YaHei Monaco Hybird".to_string(),
            "Microsoft YaHei".to_string(),
        ];

        // 期望值写成正确拼写（Hybrid）也要落到实际注册名上
        assert_eq!(
            pick_family(&available, "YaHei Monaco Hybrid"),
            "YaHei Monaco Hybird"
        );
        // 直接写真实名，精确命中
        assert_eq!(
            pick_family(&available, "YaHei Monaco Hybird"),
            "YaHei Monaco Hybird"
        );
    }

    #[test]
    fn falls_back_to_the_hint_when_absent() {
        let available = vec!["Segoe UI".to_string()];
        assert_eq!(
            pick_family(&available, "YaHei Monaco Hybrid"),
            "YaHei Monaco Hybrid"
        );
    }
}
