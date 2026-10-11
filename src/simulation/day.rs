//! Source day cycle: `napszakvaltakozas.FixedUpdate` and its four phases.

use crate::reference::DayCycleReference;
use rapier3d::math::Rotation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Directional light rotation in game space. The light shines along local -Z.
    pub sun_rotation: [f32; 4],
    pub sun_color: [f32; 3],
    /// Unity light intensity.
    pub sun_intensity: f32,
    /// Unity flat ambient value.
    pub ambient: f32,
    pub fog_color: [f32; 3],
}

/// Advances the cycle clock. One cycle is a day and a night.
pub fn advance(rules: &DayCycleReference, seconds: f32, dt: f32) -> f32 {
    (seconds + dt).rem_euclid(rules.day_seconds + rules.night_seconds)
}

/// `terrain` is the `TerrainGenerator.GetTerrainColor` value at the player.
pub fn sky(rules: &DayCycleReference, t: f32, terrain: [f32; 4]) -> Sky {
    let (day, dawn, dusk) = (rules.day_seconds, rules.dawn_seconds, rules.dusk_seconds);
    let day_fog = biome(&rules.day_fog_colors, terrain);
    let night_fog = biome(&rules.night_fog_colors, terrain);
    // Phase order: night, dusk, day, dawn. Each value follows its source phase method.
    let (angle, yaw, intensity, color, ambient, fog) = if t > day {
        let lerp = inverse_lerp(day, day + rules.night_seconds, t);
        let moon = mix(rules.moon_angles[0], rules.moon_angles[1], lerp);
        (
            moon,
            0.0,
            rules.night_intensity,
            gradient(rules, 1.0),
            rules.night_ambient,
            night_fog,
        )
    } else if t > day - dusk {
        let lerp = inverse_lerp(day - dusk, day, t);
        (
            mix(rules.day_angles[1], rules.midnight_angles[1], lerp),
            mix(60.0, 0.0, lerp),
            mix(rules.day_intensity, 0.0, lerp),
            gradient(rules, lerp),
            mix(rules.day_ambient, rules.night_ambient, lerp),
            mix3(day_fog, night_fog, inverse_lerp(0.0, 0.5, lerp)),
        )
    } else if t > dawn {
        let lerp = inverse_lerp(dawn, day - dusk, t);
        let noon = inverse_lerp(0.45, 0.35, lerp).max(inverse_lerp(0.55, 0.7, lerp));
        (
            mix(rules.day_angles[0], rules.day_angles[1], lerp),
            60.0,
            mix(rules.noon_intensity, rules.day_intensity, noon),
            gradient(rules, 0.0),
            rules.day_ambient,
            day_fog,
        )
    } else {
        let lerp = inverse_lerp(0.0, dawn, t);
        (
            mix(rules.midnight_angles[0], rules.day_angles[0], lerp),
            mix(0.0, 60.0, lerp),
            mix(0.0, rules.day_intensity, lerp),
            gradient(rules, 1.0 - lerp),
            mix(rules.night_ambient, rules.day_ambient, lerp),
            mix3(night_fog, day_fog, inverse_lerp(0.5, 1.0, lerp)),
        )
    };
    // Unity euler order applies X, then Y. The export flips Z, so X and Y change sign.
    let unity =
        Rotation::from_rotation_y(yaw.to_radians()) * Rotation::from_rotation_x(angle.to_radians());
    let game = Rotation::from_xyzw(-unity.x, -unity.y, unity.z, unity.w);
    Sky {
        sun_rotation: game.to_array(),
        sun_color: color,
        sun_intensity: intensity,
        ambient,
        fog_color: fog,
    }
}

/// `napszakvaltakozas.LerpColor`: R picks desert, B picks winter.
fn biome(colors: &[[f32; 3]; 3], terrain: [f32; 4]) -> [f32; 3] {
    mix3(
        mix3(colors[2], colors[0], terrain[0]),
        colors[1],
        terrain[2],
    )
}

/// Unity `Gradient.Evaluate` in blend mode.
fn gradient(rules: &DayCycleReference, time: f32) -> [f32; 3] {
    let keys = &rules.sun_colors;
    match keys.iter().position(|key| key.time >= time) {
        Some(0) => keys[0].color,
        Some(index) => {
            let (a, b) = (&keys[index - 1], &keys[index]);
            mix3(a.color, b.color, inverse_lerp(a.time, b.time, time))
        }
        None => keys[keys.len() - 1].color,
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| mix(a[i], b[i], t))
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}
