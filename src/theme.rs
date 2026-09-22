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
    Automata,
}

impl Theme {
    pub const ALL: [Self; 9] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
        Self::Control,
        Self::Reclamation,
        Self::Expedition,
        Self::Automata,
    ];

    pub const PALETTES: [Self; 5] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
    ];
    pub const INTERFACES: [Self; 4] = [
        Self::Control,
        Self::Reclamation,
        Self::Expedition,
        Self::Automata,
    ];

    pub fn is_interface(self) -> bool {
        Self::INTERFACES.contains(&self)
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
            Self::Automata => "automata",
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
            Self::Automata => "尼尔 · 日常档案",
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
            Self::Automata => "把平凡的每一餐，写入档案。",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.key() == key)
    }

    /// Only the selected interface stylesheet enters the DOM. All assets remain
    /// local and bundled, including when switching themes without a connection.
    pub fn stylesheet(self) -> &'static str {
        match self {
            Self::Control => include_str!("../assets/themes/control.css"),
            Self::Reclamation => include_str!("../assets/themes/reclamation.css"),
            Self::Expedition => include_str!("../assets/themes/expedition.css"),
            Self::Automata => include_str!("../assets/themes/automata.css"),
            _ => "",
        }
    }

    pub fn scene_svg(self) -> &'static str {
        match self {
            Self::Control => include_str!("../assets/themes/control-scene.svg"),
            Self::Reclamation => include_str!("../assets/themes/reclamation-scene.svg"),
            Self::Expedition => include_str!("../assets/themes/expedition-scene.svg"),
            _ => "",
        }
    }
}
