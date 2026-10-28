//! Scene pixel sizing. Display coordinates and source projections stay native.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneScale {
    #[default]
    Native,
    ThreeQuarter,
}

impl SceneScale {
    pub fn from_factor(factor: f32) -> Result<Self, &'static str> {
        match factor {
            1. => Ok(Self::Native),
            0.75 => Ok(Self::ThreeQuarter),
            _ => Err("Scene resolution must be 100% or 75%"),
        }
    }

    pub fn extent(self, native: [u32; 2]) -> [u32; 2] {
        match self {
            Self::Native => native,
            Self::ThreeQuarter => native.map(|side| {
                if side == 0 {
                    0
                } else {
                    ((u64::from(side) * 3 / 4) as u32).max(1)
                }
            }),
        }
    }
}
