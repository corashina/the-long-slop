use crate::reference::reflections::PROBE_COUNT;
#[cfg(target_arch = "wasm32")]
use bevy::ecs as bevy_ecs;

#[cfg_attr(target_arch = "wasm32", derive(bevy::prelude::Resource))]
pub struct ProbeActivation {
    pub enabled: [bool; PROBE_COUNT],
    pub checks: u32,
    pub distance_squared: [f32; PROBE_COUNT],
    scene: u32,
    elapsed: f32,
    last_check: f32,
}
impl Default for ProbeActivation {
    fn default() -> Self {
        Self {
            enabled: [false; PROBE_COUNT],
            checks: 0,
            distance_squared: [0.; PROBE_COUNT],
            scene: 0,
            elapsed: 0.,
            last_check: 0.,
        }
    }
}

impl ProbeActivation {
    pub fn update(
        &mut self,
        scene: u32,
        slots: [bool; PROBE_COUNT],
        delta: f32,
        distances: [f32; PROBE_COUNT],
    ) {
        if self.scene != scene {
            self.scene = scene;
            // reflectionsscript.Start registers each authored probe and calls Upd(BReflection).
            self.enabled = slots;
            self.elapsed = 0.;
            self.last_check = 0.;
            self.checks = 0;
        }
        self.elapsed += delta;
        self.distance_squared = distances;
        if self.elapsed - self.last_check >= 5. {
            self.last_check = self.elapsed;
            self.enabled =
                std::array::from_fn(|index| slots[index] && self.distance_squared[index] < 2500.);
            self.checks += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::reflections::{
        ProbeJob, ProbeSchedule, pending_probe_index, probe_slots,
    };
    #[test]
    fn one_authored_probe_never_enables_an_absent_slot() {
        let slots = probe_slots(1).unwrap();
        assert_eq!(slots, [true, false]);
        let mut activation = ProbeActivation::default();
        assert_eq!(activation.enabled, [false, false]);
        activation.update(10, probe_slots(2).unwrap(), 0., [0.; PROBE_COUNT]);
        assert_eq!(activation.enabled, [true, true]);
        for scene in [11, 12] {
            activation.update(scene, slots, 0., [1., 0.]);
            assert_eq!(activation.enabled, [true, false]);
            assert_eq!(activation.checks, 0);
            activation.update(scene, slots, 5., [2500., 0.]);
            assert_eq!(activation.enabled, [false, false]);
            activation.update(scene, slots, 5., [1., 0.]);
            assert_eq!(activation.enabled, [true, false]);
            assert_eq!(activation.checks, 2);
        }
    }
    #[test]
    fn two_authored_probes_keep_registration_and_five_second_cadence() {
        let slots = probe_slots(2).unwrap();
        let mut activation = ProbeActivation::default();
        activation.update(1, slots, 4.9, [2600., 1.]);
        assert_eq!(activation.enabled, [true, true]);
        assert_eq!(activation.checks, 0);
        activation.update(1, slots, 0.1, [2600., 1.]);
        assert_eq!(activation.enabled, [false, true]);
        assert_eq!(activation.checks, 1);
        activation.update(1, slots, 11., [1., 2500.]);
        assert_eq!(activation.enabled, [true, false]);
        assert_eq!(
            activation.checks, 2,
            "long frames perform only one distance check"
        );
        activation.update(2, slots, 0., [3000., 3000.]);
        assert_eq!(activation.enabled, [true, true]);
        assert_eq!(activation.checks, 0);
    }
    #[test]
    fn publication_completion_retains_a_registered_source_index() {
        for count in [1, 2] {
            let slots = std::array::from_fn(|index| index < count);
            let source_positions = vec![[1., 2., 3.]; count];
            let mut probes = [ProbeSchedule::default(), ProbeSchedule::default()];
            let mut index = 0;
            for expected in 0..count {
                assert_eq!(index, expected);
                for _ in 0..8 {
                    let job = probes[index].job();
                    probes[index].complete(job).unwrap();
                    index = pending_probe_index(index, &probes, slots);
                    assert!(slots[index]);
                    assert_eq!(source_positions[index], [1., 2., 3.]);
                }
            }
            for _ in 0..3 {
                index = pending_probe_index(index, &probes, slots);
                assert!(slots[index]);
                assert_eq!(source_positions[index], [1., 2., 3.]);
                assert!(probes[index].published());
            }
            assert_eq!(probes[count - 1].job(), ProbeJob::Face(0));
        }
    }
    #[test]
    fn one_probe_required_views_and_readiness_do_not_wait_for_absent_slot() {
        use crate::reference::loading_dispatch::{LoadingDispatch, LoadingKey};
        use crate::reference::shadow_candidates::DirectionalHull;
        use crate::world::{RequiredView, RequiredViews};
        let slots = probe_slots(1).unwrap();
        let camera = [
            [1., 0., 0., 1.],
            [-1., 0., 0., 1.],
            [0., 1., 0., 1.],
            [0., -1., 0., 1.],
            [0., 0., 1., 1.],
            [0., 0., -1., 1.],
        ];
        let first = RequiredView {
            camera,
            shadow: DirectionalHull::new(camera, [1., 0., 0.]).unwrap(),
            mask: 1,
            light_mask: 1,
            origin: [0.; 3],
        };
        let mut views = RequiredViews::new(first);
        let sources = [first];
        views
            .append_probe_faces(&slots, |index, _| Ok(sources[index]))
            .unwrap();
        assert_eq!(views.as_slice().len(), 7);
        let key = LoadingKey {
            session: 1,
            scene: 1,
            extent: [800, 600],
        };
        let mut loading = LoadingDispatch::<PROBE_COUNT>::default();
        let mask = slots
            .iter()
            .enumerate()
            .fold(0, |mask, (i, enabled)| mask | (u32::from(*enabled) << i));
        loading.snapshot(key, 4, false, true, 1, mask);
        assert!(!loading.fresh(key, 1, mask));
        loading.capture_started(key, 4, 0);
        loading.publication_validated(key, 0);
        assert!(loading.fresh(key, 1, mask));
    }
    #[test]
    fn probe_inventory_rejects_overflow() {
        assert!(probe_slots(0).is_err());
        assert!(probe_slots(PROBE_COUNT + 1).is_err());
    }
}
