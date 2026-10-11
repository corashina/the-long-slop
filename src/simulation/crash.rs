//! Source crash damage: `carscript.DamageStuff` and `partscript.WouldFallOff`.

use crate::reference::CrashReference;

/// Fall-off power scale for condition states zero to four.
const PART_WEAR: [f32; 5] = [1.0, 0.8, 0.6, 0.4, 0.2];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crash {
    /// Crash strength from zero to one.
    pub lerp: f32,
    /// Number of parts to knock off.
    pub count: usize,
}

/// `change_kmh` is the change of forward speed in one step.
/// `samples` are uniform in [0, 1): the speed threshold and the count range.
pub fn crash(
    rules: &CrashReference,
    change_kmh: f32,
    car_condition: u8,
    samples: [f32; 2],
) -> Option<Crash> {
    let wear = 1.0 - 0.8 * f32::from(car_condition.min(4)) / 4.0;
    let [low, high] = rules.fall_off_speed.map(|speed| speed * wear);
    let change = change_kmh * rules.multiplier;
    if change * rules.fall_off_multiplier <= low + (high - low) * samples[0] {
        return None;
    }
    let start = rules.fall_off_speed[0];
    let lerp = ((change - start) / (rules.max_fall_off_speed - start)).clamp(0.0, 1.0);
    let [fewest, most] = rules.fall_off_counts[usize::from(samples[1] >= 0.5)].map(f32::from);
    Some(Crash {
        lerp,
        count: (fewest + (most - fewest) * lerp) as usize,
    })
}

/// A mounted part falls off when the crash beats its worn power.
pub fn falls_off(power: f32, condition: u8, lerp: f32) -> bool {
    lerp > power
        * PART_WEAR
            .get(usize::from(condition))
            .copied()
            .unwrap_or(1.0)
}
