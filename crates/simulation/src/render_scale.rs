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

#[cfg(test)]
mod tests {
    use super::SceneScale;

    #[test]
    fn scene_scale_preserves_native_and_bounds_reduced_extents() {
        assert_eq!(SceneScale::default().extent([1280, 720]), [1280, 720]);
        let scale = SceneScale::from_factor(0.75).unwrap();
        assert_eq!(scale.extent([1280, 720]), [960, 540]);
        assert_eq!(scale.extent([1279, 719]), [959, 539]);
        assert_eq!(scale.extent([1, 1]), [1, 1]);
        assert_eq!(scale.extent([0, 0]), [0, 0]);
        assert_eq!(scale.extent([u32::MAX; 2]), [3221225471; 2]);
        for factor in [0., 0.5, f32::NAN, f32::INFINITY] {
            assert!(SceneScale::from_factor(factor).is_err());
        }
    }
}
