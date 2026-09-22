use std::{collections::BTreeMap, sync::LazyLock};

/// Language is derived from the device, never stored with meals or backups.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    Chinese,
    #[default]
    English,
}

static ENGLISH: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/locales/en.json"))
        .expect("bundled English catalog must be valid")
});

impl Language {
    pub fn from_preferences<'a>(tags: impl IntoIterator<Item = &'a str>) -> Self {
        for tag in tags {
            let base = tag.trim().split(['-', '_', '.', '@']).next().unwrap_or("");
            if base.eq_ignore_ascii_case("zh") {
                return Self::Chinese;
            }
            if base.eq_ignore_ascii_case("en") {
                return Self::English;
            }
        }
        Self::English
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }

    pub fn text(self, source: &str) -> &str {
        if self == Self::Chinese {
            source
        } else {
            ENGLISH.get(source).map(String::as_str).unwrap_or(source)
        }
    }

    pub fn format(self, source: &str, values: &[(&str, String)]) -> String {
        let mut result = self.text(source).to_owned();
        for (key, value) in values {
            result = result.replace(&format!("{{{key}}}"), value);
        }
        result
    }

    /// Translate app-owned error/notice text, leaving external diagnostic details intact.
    pub fn message(self, source: &str) -> String {
        let translated = self.text(source);
        if translated != source || self == Self::Chinese {
            return translated.to_owned();
        }
        if let Some((prefix, detail)) = source.split_once('：') {
            let key = format!("{prefix}：");
            if let Some(english) = ENGLISH.get(&key) {
                return format!("{english}{}", self.message(detail));
            }
        }
        source.to_owned()
    }
}
