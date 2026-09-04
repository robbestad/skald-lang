use super::types::Dictionary;

pub const LANGUAGE_PACK_FORMAT_VERSION: u32 = 1;

pub fn is_known_locale(locale: &str) -> bool {
    matches!(locale, "en-US" | "nb-NO" | "nn-NO")
}

pub fn builtin_pack_installed(locale: &str) -> bool {
    locale == "en-US"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSource {
    pub name: String,
    pub license: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub articles: String,
    pub numbers_verbal: String,
    pub case_title: String,
    pub rhyme: bool,
}

impl Capabilities {
    pub fn default_for_locale(locale: &str) -> Self {
        if locale == "en-US" {
            Self {
                articles: "en-indefinite".into(),
                numbers_verbal: "en".into(),
                case_title: "en".into(),
                rhyme: true,
            }
        } else {
            Self {
                articles: "none".into(),
                numbers_verbal: "none".into(),
                case_title: "none".into(),
                rhyme: false,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguagePack {
    pub id: String,
    pub locale: String,
    pub format_version: u32,
    pub content_version: String,
    pub capabilities: Capabilities,
    pub source: Option<PackSource>,
    pub dictionary: Dictionary,
}
