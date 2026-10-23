/// 初始化语言。
///
/// 注意有两个 i18n 实例：应用自己用 `rust-i18n 3`，而 gpui-component 用的是
/// `rust-i18n 4`，两者互相看不见对方的 locale。所以设置一次不够，组件库里
/// 的输入框右键菜单、对话框按钮等文案要单独同步一份。
pub fn init() {
    let locale = detect();
    rust_i18n::set_locale(locale);
    gpui_kit::component::set_locale(locale);
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
