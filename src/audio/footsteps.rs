//! Source footsteps: `fpscontroller` step triggers and `stepSoundScript.Step`.
use super::{AudioAction, AudioKind, AudioState, SurvivalAudioEvent};
use crate::reference::{FootstepAudioReference, StepSurface};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, STEP_SURFACE_MASK, STEP_SURFACE_SHIFT, to_rapier};
use rapier3d::prelude::{Collider, ColliderHandle, QueryFilter, Ray, Vector};

const ROAD: usize = 0;
const DESERT: usize = 1;
const WINTER: usize = 2;
const GRASS: usize = 3;
const WOOD: usize = 4;

/// Surface index in source order. `biome` runs only for sand.
pub fn surface(
    code: u8,
    membership: u32,
    sand_layers: &[u32],
    biome: impl FnOnce() -> u8,
) -> usize {
    // otherStepSoundScript, then sand layer or tag, then tag Wood, then road.
    // Codes 1 to 5 are the surfaces in source order.
    if (1..=5).contains(&code) {
        return code as usize - 1;
    }
    if code == StepSurface::Sand as u8 || sand_layers.iter().any(|l| membership & (1 << l) != 0) {
        return match biome() {
            1 => WINTER,
            2 => GRASS,
            _ => DESERT,
        };
    }
    if code == StepSurface::WoodTag as u8 {
        WOOD
    } else {
        ROAD
    }
}

/// Source `Step` modifier: duck value, plus 0.15 while run is held.
pub fn gain(volume: f32, posture: f32, run: bool, step_gain: f32) -> f32 {
    volume * (posture + if run { 0.15 } else { 0. }) * step_gain
}

/// Clip and gain of a step on a hit collider `(user_data, membership)`. `sample` is in 0..1.
fn step_sound(
    r: &FootstepAudioReference,
    hit: Option<(u128, u32)>,
    posture: f32,
    run: bool,
    sample: f32,
    biome: impl FnOnce() -> u8,
) -> Option<(u32, f32)> {
    let (data, membership) = hit?;
    let code = ((data & STEP_SURFACE_MASK) >> STEP_SURFACE_SHIFT) as u8;
    let s = &r.surfaces[surface(code, membership, &r.sand_layers, biome)];
    let index = ((sample * s.clips.len() as f32) as usize).min(s.clips.len() - 1);
    Some((s.clips[index], gain(s.volume, posture, run, r.step_gain)))
}

#[derive(Default)]
pub struct FootstepState {
    last_grounded: bool,
}

impl FootstepState {
    /// True when the source plays a step: a walk bob beat or a grounded change, not seated.
    fn step(&mut self, seated: bool, grounded: bool, beat: bool) -> bool {
        let changed = grounded != self.last_grounded;
        self.last_grounded = grounded;
        !seated && (beat || changed)
    }
}

impl AudioState {
    /// Run after the walk bob step. `biome` takes an absolute position.
    pub fn footsteps(
        &mut self,
        area: &StartingArea,
        physics: &PhysicsWorld,
        run: bool,
        biome: impl FnOnce([f64; 3]) -> u8,
    ) {
        let player = &area.player;
        let seated = player.seated();
        if !self
            .footsteps
            .step(seated, player.grounded(), player.footstep_beat())
        {
            return;
        }
        let r = &self.reference.footsteps;
        let body = to_rapier(player.body_pose(physics));
        let origin = body.transform_point(Vector::from_array(r.local_position));
        let own = player.body_handle();
        let mask = r.ray_mask;
        let floor = |_: ColliderHandle, c: &Collider| {
            !c.is_sensor()
                && c.parent() != Some(own)
                && c.collision_groups().memberships.bits() & mask != 0
        };
        let hit = physics
            .queries(QueryFilter::default().predicate(&floor))
            .cast_ray(
                &Ray::new(origin, body.rotation * -Vector::Y),
                r.ray_distance,
                true,
            )
            .map(|(handle, _)| {
                let c = &physics.rapier().colliders[handle];
                (c.user_data, c.collision_groups().memberships.bits())
            });
        let position = std::array::from_fn(|i| area.origin[i] + f64::from(origin[i]));
        let sample = self.sample();
        let Some((clip, gain)) = step_sound(
            &self.reference.footsteps,
            hit,
            player.posture(),
            run,
            sample,
            || biome(position),
        ) else {
            return;
        };
        let [low, high] = self.reference.shared.random_pitch;
        let pitch = low + (high - low) * self.sample();
        self.survival_event(SurvivalAudioEvent {
            session: self.session,
            emitter: 0,
            kind: AudioKind::Footstep,
            action: AudioAction::Start,
            clip,
            gain,
            pitch,
            position,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::FootstepSurfaceReference;

    const SAND: [u32; 2] = [17, 19];

    fn at(code: Option<StepSurface>, membership: u32, biome: u8) -> usize {
        surface(code.map_or(0, |s| s as u8), membership, &SAND, || biome)
    }

    #[test]
    fn surface_follows_source_order() {
        use StepSurface::*;
        let sand_layer = 1 << 17;
        for (code, index) in [
            (Road, ROAD),
            (Desert, DESERT),
            (Winter, WINTER),
            (Grass, GRASS),
        ] {
            assert_eq!(
                at(Some(code), sand_layer, 1),
                index,
                "override wins over sand"
            );
        }
        assert_eq!(at(Some(Wood), sand_layer, 1), WOOD);
        assert_eq!(at(Some(WoodTag), 1, 0), WOOD);
        assert_eq!(
            at(Some(WoodTag), 1 << 19, 0),
            DESERT,
            "sand wins over tag Wood"
        );
        assert_eq!(at(None, 1, 0), ROAD);
        for (biome, index) in [(0, DESERT), (1, WINTER), (2, GRASS)] {
            assert_eq!(at(Some(Sand), 1, biome), index);
            assert_eq!(at(None, sand_layer, biome), index);
        }
        let mut sampled = false;
        surface(StepSurface::Road as u8, sand_layer, &SAND, || {
            sampled = true;
            0
        });
        assert!(!sampled, "biome only for sand");
    }

    #[test]
    fn gain_uses_duck_value_and_run() {
        assert!((gain(0.2, 1., false, 1.) - 0.2).abs() < 1e-6);
        assert!((gain(0.2, 0.5, true, 2.) - 0.26).abs() < 1e-6);
        let surface = |volume| FootstepSurfaceReference {
            name: String::new(),
            volume,
            clips: vec![10, 11],
        };
        let r = FootstepAudioReference {
            surfaces: [0.2, 0.2, 0.2, 0.2, 2.].map(surface),
            step_gain: 1.,
            ray_distance: 0.25,
            ray_mask: 164353,
            local_position: [0., -0.8343, 0.],
            sand_layers: SAND.to_vec(),
            method: Vec::new(),
        };
        let wood = crate::vehicle::step_surface_bits(Some(StepSurface::Wood));
        let (clip, g) = step_sound(&r, Some((wood | 5, 1)), 1., true, 0.99, || 0).unwrap();
        assert_eq!(clip, 11);
        assert!((g - 2.3).abs() < 1e-6);
        assert_eq!(
            step_sound(&r, None, 1., false, 0., || 0),
            None,
            "no ray hit"
        );
    }

    #[test]
    fn steps_on_beat_and_grounded_change_but_not_seated() {
        let mut s = FootstepState::default();
        assert!(s.step(false, true, false), "landing");
        assert!(!s.step(false, true, false));
        assert!(s.step(false, true, true), "beat");
        assert!(s.step(false, false, false), "jump");
        assert!(!s.step(true, true, true), "seated");
        assert!(s.last_grounded);
    }
}
