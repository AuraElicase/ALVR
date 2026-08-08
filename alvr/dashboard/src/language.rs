use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    English,
    SimplifiedChinese,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageOption {
    SystemDefault,
    English,
    SimplifiedChinese,
}

impl LanguageOption {
    fn locale(self) -> Locale {
        match self {
            Self::SystemDefault => detect_system_locale(),
            Self::English => Locale::English,
            Self::SimplifiedChinese => Locale::SimplifiedChinese,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SystemDefault => "system",
            Self::English => "en-US",
            Self::SimplifiedChinese => "zh-CN",
        }
    }

    fn from_str(value: &str) -> Self {
        match value.trim() {
            "en-US" => Self::English,
            "zh-CN" => Self::SimplifiedChinese,
            _ => Self::SystemDefault,
        }
    }
}

pub struct Language {
    translations: HashMap<String, String>,
}

impl Language {
    fn from_locale(locale: Locale) -> Self {
        let source = match locale {
            Locale::English => include_str!("../resources/languages/en-US.json"),
            Locale::SimplifiedChinese => include_str!("../resources/languages/zh-CN.json"),
        };

        Self {
            translations: serde_json::from_str(source).expect("invalid language file"),
        }
    }

    pub fn translate(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }
}

static LANGUAGE: OnceLock<RwLock<Language>> = OnceLock::new();
static LANGUAGE_OPTION: OnceLock<RwLock<LanguageOption>> = OnceLock::new();
static LANGUAGE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(path: PathBuf) {
    let option = fs::read_to_string(&path)
        .map(|value| LanguageOption::from_str(&value))
        .unwrap_or(LanguageOption::SystemDefault);
    LANGUAGE_OPTION.get_or_init(|| RwLock::new(option));
    LANGUAGE_PATH.set(path).ok();
    LANGUAGE.get_or_init(|| RwLock::new(Language::from_locale(option.locale())));
}

pub fn tr(key: &str) -> String {
    LANGUAGE
        .get_or_init(|| RwLock::new(Language::from_locale(detect_system_locale())))
        .read()
        .unwrap()
        .translate(key)
}

pub fn language_option() -> LanguageOption {
    *LANGUAGE_OPTION
        .get_or_init(|| RwLock::new(LanguageOption::SystemDefault))
        .read()
        .unwrap()
}

pub fn set_language_option(option: LanguageOption) {
    *LANGUAGE_OPTION
        .get_or_init(|| RwLock::new(option))
        .write()
        .unwrap() = option;

    if let Some(path) = LANGUAGE_PATH.get() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, option.as_str()).ok();
    }

    if let Some(language) = LANGUAGE.get() {
        *language.write().unwrap() = Language::from_locale(option.locale());
    }
}

fn detect_system_locale() -> Locale {
    let value = std::env::var("ALVR_LANGUAGE")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LANG"))
        .or_else(|_| browser_language())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if value.starts_with("zh") {
        Locale::SimplifiedChinese
    } else {
        Locale::English
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_language() -> Result<String, std::env::VarError> {
    eframe::web_sys::window()
        .map(|window| window.navigator().language())
        .ok_or(std::env::VarError::NotPresent)
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_language() -> Result<String, std::env::VarError> {
    Err(std::env::VarError::NotPresent)
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn missing_keys_fall_back_to_source_text() {
        let language = Language::from_locale(super::Locale::SimplifiedChinese);
        assert_eq!(language.translate("missing key"), "missing key");
        assert_eq!(language.translate("Settings"), "设置");
    }
}
