#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Poster,
    Rhodes,
    Rhine,
    Penguin,
    Kazimierz,
}

impl Theme {
    pub const ALL: [Self; 5] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
    ];

    // Keep historical storage keys so existing selections survive display-name changes.
    pub fn key(self) -> &'static str {
        match self {
            Self::Poster => "poster",
            Self::Rhodes => "rhodes",
            Self::Rhine => "rhine",
            Self::Penguin => "penguin",
            Self::Kazimierz => "kazimierz",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Poster => "原味海报",
            Self::Rhodes => "冰蓝终端",
            Self::Rhine => "苔绿纸页",
            Self::Penguin => "赤橙速递",
            Self::Kazimierz => "鎏金夜幕",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Poster => "奶油纸张，荧光宣言。",
            Self::Rhodes => "深色底板，冰蓝强调。",
            Self::Rhine => "米白纸页，沉静苔绿。",
            Self::Penguin => "石墨底色，赤橙标记。",
            Self::Kazimierz => "暗紫夜幕，暖金线条。",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.key() == key)
    }
}
