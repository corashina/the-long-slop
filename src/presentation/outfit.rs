//! Player outfit (`settingsplayercustomscript.outfitclass`). The browser stores it and sends it as JSON.
//! Only the presentation reads it. The game logic does not use it.
use crate::reference::{CharacterReference, NodeId};
use bevy::ecs as bevy_ecs;
use bevy::prelude::Resource;

/// `enabledCharacter` 0: no body.
pub const MODEL_OFF: u8 = 0;
/// `enabledCharacter` 1: the camera does not draw the body, but it casts its shadow.
pub const MODEL_SHADOW: u8 = 1;
/// `enabledCharacter` 2: visible body.
pub const MODEL_VISIBLE: u8 = 2;

#[derive(Resource, Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outfit {
    /// `enabledCharacter`.
    pub model: u8,
    /// `selectedCharacter`.
    pub character: usize,
    pub characters: Vec<CharacterOutfit>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterOutfit {
    /// `noReset`. The browser resets a character that is not edited.
    pub edited: bool,
    pub segments: Vec<SegmentOutfit>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentOutfit {
    pub enabled: bool,
    pub hue: f32,
    pub brightness: f32,
    pub saturation: f32,
}

impl Outfit {
    /// `FullReset`: visible model, character 0 and the segment defaults.
    pub fn defaults(characters: &[CharacterReference]) -> Self {
        Self {
            model: MODEL_VISIBLE,
            character: 0,
            characters: characters
                .iter()
                .map(|c| CharacterOutfit {
                    edited: false,
                    segments: c
                        .segments
                        .iter()
                        .map(|s| SegmentOutfit {
                            enabled: s.default_enabled,
                            hue: s.default_hsv[0],
                            brightness: s.default_hsv[2],
                            saturation: s.default_hsv[1],
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// True when the counts match the characters and each value is in range.
    pub fn valid(&self, characters: &[CharacterReference]) -> bool {
        let unit = |v: f32| (0. ..=1.).contains(&v);
        self.model <= MODEL_VISIBLE
            && self.character < characters.len()
            && self.characters.len() == characters.len()
            && self.characters.iter().zip(characters).all(|(o, c)| {
                o.segments.len() == c.segments.len()
                    && o.segments
                        .iter()
                        .all(|s| unit(s.hue) && unit(s.brightness) && unit(s.saturation))
            })
    }
}

impl SegmentOutfit {
    /// `GetColor`: `Color.HSVToRGB(color, saturation, brightness)`.
    pub fn color(&self) -> [f32; 3] {
        hsv_to_rgb(self.hue, self.saturation, self.brightness)
    }
}

/// Unity `Color.HSVToRGB(H, S, V)` (HDR, no clamp).
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    if s == 0. {
        return [v; 3];
    }
    if v == 0. {
        return [0.; 3];
    }
    let h6 = h * 6.;
    let sector = h6.floor();
    let f = h6 - sector;
    let p = v * (1. - s);
    let q = v * (1. - s * f);
    let t = v * (1. - s * (1. - f));
    match sector as i32 {
        0 | 6 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 | -1 => [v, p, q],
        _ => [0.; 3],
    }
}

/// Map `nodes` of character 0 to character 1 by node name.
/// `from` and `to` are the named skin joints of characters 0 and 1.
/// A node in `to`, or a node not in `from`, maps to itself. None when a name has no match.
pub fn map_by_name(
    nodes: &[NodeId],
    from: &[(NodeId, &str)],
    to: &[(NodeId, &str)],
) -> Option<Vec<NodeId>> {
    nodes
        .iter()
        .map(|node| {
            if to.iter().any(|(n, _)| n == node) {
                return Some(*node);
            }
            let Some((_, name)) = from.iter().find(|(n, _)| n == node) else {
                return Some(*node);
            };
            to.iter().find(|(_, n)| n == name).map(|(n, _)| *n)
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
/// Read the outfit that the browser sends at start and after each change.
pub(crate) fn poll(
    mut outfit: bevy::prelude::ResMut<Outfit>,
    reference: bevy::prelude::Res<crate::app::RuntimeReference>,
) {
    use bevy::prelude::DetectChangesMut;
    let Some(json) = crate::browser::game_outfit() else {
        return;
    };
    match serde_json::from_str::<Outfit>(&json) {
        Ok(value) if value.valid(&reference.0.clothing.characters) => {
            outfit.set_if_neq(value);
        }
        Ok(_) => bevy::log::warn!("Outfit: values do not match the characters"),
        Err(error) => bevy::log::warn!("Outfit: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_matches_unity() {
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6);
        // Male hair: `RGBToHSV(defColor)` back to `defColor`.
        assert!(close(
            hsv_to_rgb(0.074_561_42, 0.542_857_25, 0.274_509_8),
            [0.274_509_8, 0.192_156_85, 0.125_490_15]
        ));
        assert!(close(hsv_to_rgb(0.5, 0., 0.3), [0.3; 3]));
        assert!(close(hsv_to_rgb(0.5, 1., 0.), [0.; 3]));
        // H = 1 is sector 6, the same color as H = 0.
        assert!(close(hsv_to_rgb(1., 1., 1.), [1., 0., 0.]));
        assert!(close(hsv_to_rgb(1. / 3., 1., 1.), [0., 1., 0.]));
        assert!(close(hsv_to_rgb(2. / 3., 1., 1.), [0., 0., 1.]));
        assert!(close(hsv_to_rgb(0.75, 0.5, 0.8), [0.6, 0.4, 0.8]));
    }

    #[test]
    fn joint_names_map_all_joints() {
        let names: Vec<String> = (0..53).map(|i| format!("bone_{i}")).collect();
        let from: Vec<_> = (0..53)
            .map(|i| (NodeId(i), names[i as usize].as_str()))
            .collect();
        // Character 1 lists its joints in another order.
        let to: Vec<_> = (0..53)
            .rev()
            .map(|i| (NodeId(100 + i), names[i as usize].as_str()))
            .collect();
        let rig: Vec<_> = from.iter().map(|(n, _)| *n).collect();
        let mapped = map_by_name(&rig, &from, &to).unwrap();
        assert_eq!(mapped, (100..153).map(NodeId).collect::<Vec<_>>());
        // A shared node and a node outside character 0 map to themselves.
        assert_eq!(
            map_by_name(&[NodeId(120), NodeId(500)], &from, &to),
            Some(vec![NodeId(120), NodeId(500)])
        );
        assert_eq!(map_by_name(&rig, &from, &to[1..]), None);
    }
}
