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
            Self::Rhodes => "罗德岛",
            Self::Rhine => "莱茵生命",
            Self::Penguin => "企鹅物流",
            Self::Kazimierz => "卡西米尔",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Poster => "奶油纸张，荧光宣言。",
            Self::Rhodes => "深色终端，冰蓝航线。",
            Self::Rhine => "纯白实验室，生命绿意。",
            Self::Penguin => "午夜街头，橙色速递。",
            Self::Kazimierz => "暗紫竞技场，金色荣光。",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.key() == key)
    }
}
