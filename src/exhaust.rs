//! Source exhaust smoke rules: `fluidcontainer.GetBurnColor` and `carscript` smoke rate.
use crate::fluids::FluidContainer;
use crate::particles::{ParticleOutput, ParticleState};
use crate::reference::ParticleReference;
use crate::simulation::dials::inverse_lerp;
use crate::vehicle::PhysicsWorld;
use rapier3d::math::Rotation;

/// Source `carscript` sets the smoke start color this often while the engine runs.
const COLOR_INTERVAL: f32 = 2.;

pub struct ExhaustInput<'a> {
    pub running: bool,
    pub rpm: f32,
    pub tank: Option<&'a FluidContainer>,
    /// Absolute position of the car root node.
    pub root_absolute: [f64; 3],
    /// Rotation of the car root node.
    pub rotation: Rotation,
}

/// Per-car `carscript.psSmoke` systems.
#[derive(Debug)]
pub struct ExhaustState {
    systems: Vec<ParticleState>,
    max_rpm: f32,
    smoke_rate: f32,
    next_color: f32,
    elapsed: f32,
}
impl ExhaustState {
    pub fn new(systems: &[ParticleReference], max_rpm: f32, smoke_rate: f32, seed: u64) -> Self {
        Self {
            systems: systems
                .iter()
                .enumerate()
                .map(|(i, r)| ParticleState::new(r, seed.wrapping_add(i as u64)))
                .collect(),
            max_rpm,
            smoke_rate,
            next_color: 0.,
            elapsed: 0.,
        }
    }
    pub fn advance(
        &mut self,
        dt: f32,
        input: ExhaustInput,
        colors: &[[f32; 4]; 6],
        physics: &PhysicsWorld,
    ) {
        self.elapsed += dt;
        let color = (input.running && self.elapsed >= self.next_color).then(|| {
            self.next_color = self.elapsed + COLOR_INTERVAL;
            input.tank.map_or([0.; 4], |tank| burn_color(tank, colors))
        });
        let strength = smoke_rate(input.running, input.rpm, self.max_rpm, self.smoke_rate);
        for system in &mut self.systems {
            if color.is_some() {
                system.start_color = color;
            }
            system.rotation = input.rotation;
            system.advance(dt, input.root_absolute, strength, physics);
        }
    }
    pub fn particles(&self, system: usize) -> &[ParticleOutput] {
        self.systems[system].particles()
    }
}

/// Source `GetBurnColor`: each fluid adds its color by share; each later fluid halves the sum.
pub fn burn_color(tank: &FluidContainer, colors: &[[f32; 4]; 6]) -> [f32; 4] {
    let total = tank.total();
    let mut color = [0.; 4];
    if !(total > 0. && total.is_finite()) {
        return color;
    }
    for (i, entry) in tank.entries().enumerate() {
        let weight = inverse_lerp(0., 0.15, entry.amount / total);
        for (c, f) in color.iter_mut().zip(colors[entry.fluid_type as usize]) {
            *c += f * weight;
            if i != 0 {
                *c *= 0.5;
            }
        }
    }
    color
}

/// Source smoke emission rate: scales with rpm while the engine runs.
pub fn smoke_rate(running: bool, rpm: f32, max_rpm: f32, rate: f32) -> f32 {
    if running {
        inverse_lerp(0., max_rpm, rpm) * rate
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::FluidEntry;
    use crate::reference::{
        GradientReference, ParticleColor, ParticleCurve, SourceId, TorqueCurve, Transform,
    };

    const COLORS: [[f32; 4]; 6] = [
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
        [0.5, 0.5, 0.5, 1.],
        [0.2, 0.4, 0.6, 0.8],
        [0., 0., 0., 0.],
    ];

    fn tank(entries: &[(u8, f32)]) -> FluidContainer {
        let entries: Vec<_> = entries
            .iter()
            .map(|&(fluid_type, amount)| FluidEntry { fluid_type, amount })
            .collect();
        FluidContainer::new(100., &entries)
    }

    fn close(a: [f32; 4], b: [f32; 4]) {
        for i in 0..4 {
            assert!((a[i] - b[i]).abs() < 1e-5, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn single_fluid_gives_its_color() {
        close(burn_color(&tank(&[(3, 10.)]), &COLORS), COLORS[3]);
    }

    #[test]
    fn later_fluids_halve_the_sum_in_stored_order() {
        // Shares 0.9 and 0.1: weights 1 and 2/3.
        let w = 0.1 / 0.15;
        let c = burn_color(&tank(&[(0, 9.), (1, 1.)]), &COLORS);
        close(c, [0.5, 0.5 * w, 0., 0.5 * (1. + w)]);
        // Three fluids at 50/25/25: all weights 1.
        let c = burn_color(&tank(&[(0, 2.), (1, 1.), (2, 1.)]), &COLORS);
        close(c, [0.25, 0.25, 0.5, 0.25 + 0.25 + 0.5]);
    }

    #[test]
    fn weight_ramps_to_fifteen_percent() {
        let c = burn_color(&tank(&[(2, 1.), (0, 0.)]), &COLORS);
        close(c, [0., 0., 0.5, 0.5]);
        let c = burn_color(&tank(&[(2, 92.5), (0, 7.5)]), &COLORS);
        close(c, [0.25, 0., 0.5, 0.75]);
        let c = burn_color(&tank(&[(2, 85.), (0, 15.)]), &COLORS);
        close(c, [0.5, 0., 0.5, 1.]);
    }

    #[test]
    fn empty_tank_gives_no_color() {
        assert_eq!(burn_color(&tank(&[]), &COLORS), [0.; 4]);
        assert_eq!(burn_color(&tank(&[(0, 0.)]), &COLORS), [0.; 4]);
    }

    #[test]
    fn smoke_rate_follows_rpm_while_running() {
        assert_eq!(smoke_rate(false, 3000., 3000., 20.), 0.);
        assert_eq!(smoke_rate(true, 3000., 3000., 20.), 20.);
        assert_eq!(smoke_rate(true, 1500., 3000., 20.), 10.);
        assert_eq!(smoke_rate(true, 6000., 3000., 20.), 20.);
    }

    fn curve(scalar: f32) -> ParticleCurve {
        let flat = || TorqueCurve {
            keys: Vec::new(),
            pre_wrap_mode: 0,
            post_wrap_mode: 0,
        };
        ParticleCurve {
            mode: 0,
            min_scalar: scalar,
            scalar,
            min_curve: flat(),
            max_curve: flat(),
        }
    }

    fn smoke(capacity: u32, lifetime: f32) -> ParticleReference {
        let gradient = || GradientReference {
            mode: 0,
            colors: Vec::new(),
            alphas: Vec::new(),
        };
        ParticleReference {
            source: SourceId {
                file: "level1".into(),
                path_id: 1,
            },
            role: "exhaust_0_0".into(),
            capacity,
            rate_over_time: 1.,
            rate_over_distance: 0.,
            duration: 5.,
            simulation_space: 2,
            local_pose: Transform {
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            },
            module_flags: Default::default(),
            curves: [
                ("InitialModule.startSpeed", curve(5.)),
                ("InitialModule.startLifetime", curve(lifetime)),
                ("InitialModule.startSize", curve(1.)),
                ("InitialModule.gravityModifier", curve(0.)),
                ("InitialModule.startRotation", curve(0.)),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            colors: [(
                "InitialModule.startColor".into(),
                ParticleColor {
                    mode: 0,
                    min_color: [1.; 4],
                    max_color: [1.; 4],
                    min_gradient: gradient(),
                    max_gradient: gradient(),
                },
            )]
            .into(),
            vectors: [
                ("ShapeModule.m_Scale".into(), [1.; 3]),
                ("ShapeModule.m_Rotation".into(), [0.; 3]),
            ]
            .into(),
            scalars: [
                ("ShapeModule.type", 4.),
                ("ShapeModule.angle", 10.),
                ("ShapeModule.radiusThickness", 1.),
                ("ShapeModule.radius.value", 0.1),
                ("ShapeModule.randomPositionAmount", 0.),
                ("ringBufferMode", 0.),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            flags: [("gameObject.active".into(), true)].into(),
            links: Default::default(),
            materials: Vec::new(),
            limits: Vec::new(),
        }
    }

    fn input(running: bool, tank: &FluidContainer) -> ExhaustInput<'_> {
        ExhaustInput {
            running,
            rpm: 6600.,
            tank: Some(tank),
            root_absolute: [10., 1., -3.],
            rotation: Rotation::IDENTITY,
        }
    }

    #[test]
    fn max_rpm_emits_smoke_rate_per_second() {
        let physics = PhysicsWorld::empty(0, 0);
        let fuel = tank(&[(0, 10.)]);
        for (capacity, expected) in [(2000, 175), (100, 100)] {
            let mut exhaust = ExhaustState::new(&[smoke(capacity, 15.)], 6600., 175., 1);
            for _ in 0..60 {
                exhaust.advance(1. / 60., input(true, &fuel), &COLORS, &physics);
            }
            let count = exhaust.particles(0).len() as i32;
            assert!((count - expected).abs() <= 1, "{count} != {expected}");
        }
    }

    #[test]
    fn stopped_engine_emits_none_and_smoke_lives_out() {
        let physics = PhysicsWorld::empty(0, 0);
        let fuel = tank(&[(0, 10.)]);
        let mut exhaust = ExhaustState::new(&[smoke(2000, 2.)], 6600., 175., 1);
        for _ in 0..4 {
            exhaust.advance(0.25, input(true, &fuel), &COLORS, &physics);
        }
        let count = exhaust.particles(0).len();
        assert!(count > 0);
        for _ in 0..3 {
            exhaust.advance(0.25, input(false, &fuel), &COLORS, &physics);
        }
        assert_eq!(
            exhaust.particles(0).len(),
            count,
            "no emission, no early death"
        );
        for _ in 0..6 {
            exhaust.advance(0.25, input(false, &fuel), &COLORS, &physics);
        }
        assert!(exhaust.particles(0).is_empty(), "lifetime ends");
    }

    #[test]
    fn start_color_follows_tank_every_two_seconds() {
        let physics = PhysicsWorld::empty(0, 0);
        let red = tank(&[(0, 10.)]);
        let blue = tank(&[(2, 10.)]);
        let mut exhaust = ExhaustState::new(&[smoke(2000, 15.)], 6600., 175., 1);
        exhaust.advance(0.25, input(false, &red), &COLORS, &physics);
        assert_eq!(
            exhaust.systems[0].start_color, None,
            "stopped engine keeps color"
        );
        exhaust.advance(0.25, input(true, &red), &COLORS, &physics);
        assert_eq!(exhaust.systems[0].start_color, Some(COLORS[0]));
        assert_eq!(exhaust.particles(0)[0].color, COLORS[0]);
        for _ in 0..7 {
            exhaust.advance(0.25, input(true, &blue), &COLORS, &physics);
        }
        assert_eq!(exhaust.systems[0].start_color, Some(COLORS[0]));
        exhaust.advance(0.25, input(true, &blue), &COLORS, &physics);
        assert_eq!(exhaust.systems[0].start_color, Some(COLORS[2]));
    }
}
