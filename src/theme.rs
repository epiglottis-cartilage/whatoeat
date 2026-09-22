#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Poster,
    Rhodes,
    Rhine,
    Penguin,
    Kazimierz,
    Control,
    Reclamation,
    Expedition,
}

impl Theme {
    pub const ALL: [Self; 8] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
        Self::Control,
        Self::Reclamation,
        Self::Expedition,
    ];

    pub const PALETTES: [Self; 5] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
    ];
    pub const INTERFACES: [Self; 3] = [Self::Control, Self::Reclamation, Self::Expedition];

    pub fn is_interface(self) -> bool {
        matches!(self, Self::Control | Self::Reclamation | Self::Expedition)
    }

    // Keep historical storage keys so existing selections survive display-name changes.
    pub fn key(self) -> &'static str {
        match self {
            Self::Poster => "poster",
            Self::Rhodes => "rhodes",
            Self::Rhine => "rhine",
            Self::Penguin => "penguin",
            Self::Kazimierz => "kazimierz",
            Self::Control => "control",
            Self::Reclamation => "reclamation",
            Self::Expedition => "expedition",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Poster => "原味海报",
            Self::Rhodes => "冰蓝终端",
            Self::Rhine => "苔绿纸页",
            Self::Penguin => "赤橙速递",
            Self::Kazimierz => "鎏金夜幕",
            Self::Control => "罗德岛 · 中枢",
            Self::Reclamation => "生息演算 · 营地",
            Self::Expedition => "集成战略 · 旅程",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Poster => "奶油纸张，荧光宣言。",
            Self::Rhodes => "深色底板，冰蓝强调。",
            Self::Rhine => "米白纸页，沉静苔绿。",
            Self::Penguin => "石墨底色，赤橙标记。",
            Self::Kazimierz => "暗紫夜幕，暖金线条。",
            Self::Control => "日常中枢，悬浮面板。",
            Self::Reclamation => "休整片刻，再次出发。",
            Self::Expedition => "每一顿，都是旅途的一站。",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.key() == key)
    }
}
