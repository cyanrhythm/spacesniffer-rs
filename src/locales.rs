
pub fn init() {
    let locale = detect();
    rust_i18n::set_locale(locale);
}

fn detect() -> &'static str {
    #[cfg(windows)]
    {
        let primary = unsafe { get_user_default_ui_lang_primary() };
        if primary == 0x04 {
            return "zh-CN";
        }
    }
    #[cfg(not(windows))]
    {
        for key in &["LANG", "LC_ALL", "LC_MESSAGES"] {
            if let Ok(v) = std::env::var(key) {
                let lower = v.to_lowercase();
                if lower.starts_with("zh") || lower.contains("zh_cn") || lower.contains("zh_tw") {
                    return "zh-CN";
                }
            }
        }
    }
    "en"
}

#[cfg(windows)]
unsafe fn get_user_default_ui_lang_primary() -> u16 {
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    GetUserDefaultUILanguage() & 0x00FF
}
