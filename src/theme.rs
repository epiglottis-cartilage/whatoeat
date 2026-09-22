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
    Phantom,
    Island,
    Terminal,
    Strand,
    Frontline,
    Marathon,
    Valley,
}

impl Theme {
    pub const ALL: [Self; 16] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
        Self::Control,
        Self::Reclamation,
        Self::Expedition,
        Self::Automata,
        Self::Phantom,
        Self::Island,
        Self::Terminal,
        Self::Strand,
        Self::Frontline,
        Self::Marathon,
        Self::Valley,
    ];

    pub const PALETTES: [Self; 5] = [
        Self::Poster,
        Self::Rhodes,
        Self::Rhine,
        Self::Penguin,
        Self::Kazimierz,
    ];
    pub const INTERFACES: [Self; 11] = [
        Self::Control,
        Self::Reclamation,
        Self::Expedition,
        Self::Automata,
        Self::Phantom,
        Self::Island,
        Self::Terminal,
        Self::Strand,
        Self::Frontline,
        Self::Marathon,
        Self::Valley,
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
            Self::Phantom => "phantom",
            Self::Island => "island",
            Self::Terminal => "terminal",
            Self::Strand => "strand",
            Self::Frontline => "frontline",
            Self::Marathon => "marathon",
            Self::Valley => "valley",
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
            Self::Phantom => "P5 · 开饭预告",
            Self::Island => "动森 · 小岛食记",
            Self::Terminal => "辐射 · 补给终端",
            Self::Strand => "死亡搁浅 · 一餐之遥",
            Self::Frontline => "战地 1 · 休整时刻",
            Self::Marathon => "马拉松 · 食欲协议",
            Self::Valley => "星露谷 · 田园饭点",
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
            Self::Phantom => "今天的胃口，由自己宣告。",
            Self::Island => "慢慢生活，好好吃饭。",
            Self::Terminal => "先补给，再继续出发。",
            Self::Strand => "把温热的一餐，送给自己。",
            Self::Frontline => "休整之后，再次出发。",
            Self::Marathon => "接入日常，准备开饭。",
            Self::Valley => "日子慢慢过，饭要好好吃。",
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
            Self::Phantom => include_str!("../assets/themes/phantom.css"),
            Self::Island => include_str!("../assets/themes/island.css"),
            Self::Terminal => include_str!("../assets/themes/terminal.css"),
            Self::Strand => include_str!("../assets/themes/strand.css"),
            Self::Frontline => include_str!("../assets/themes/frontline.css"),
            Self::Marathon => include_str!("../assets/themes/marathon.css"),
            Self::Valley => include_str!("../assets/themes/valley.css"),
            _ => "",
        }
    }

    pub fn scene_svg(self) -> &'static str {
        match self {
            Self::Control => include_str!("../assets/themes/control-scene.svg"),
            Self::Reclamation => include_str!("../assets/themes/reclamation-scene.svg"),
            Self::Expedition => include_str!("../assets/themes/expedition-scene.svg"),
            Self::Phantom => include_str!("../assets/themes/phantom-scene.svg"),
            Self::Island => include_str!("../assets/themes/island-scene.svg"),
            Self::Strand => include_str!("../assets/themes/strand-scene.svg"),
            Self::Frontline => include_str!("../assets/themes/frontline-scene.svg"),
            Self::Marathon => include_str!("../assets/themes/marathon-scene.svg"),
            Self::Valley => include_str!("../assets/themes/valley-scene.svg"),
            _ => "",
        }
    }

    pub fn overview_svg(self) -> Option<&'static str> {
        match self {
            Self::Island => Some(include_str!("../assets/themes/island-picnic.svg")),
            Self::Terminal => Some(include_str!("../assets/themes/terminal-ration.svg")),
            Self::Strand => Some(include_str!("../assets/themes/strand-parcel.svg")),
            Self::Marathon => Some(include_str!("../assets/themes/marathon-nodes.svg")),
            Self::Valley => Some(include_str!("../assets/themes/valley-garden.svg")),
            _ => None,
        }
    }
}
