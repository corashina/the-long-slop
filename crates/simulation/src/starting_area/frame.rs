//! Renderer data of one tick.
use super::GarageRuntime;
use crate::boundary::{Handle, SimFrame, StateBlock, ValveRow};
use crate::interaction::items::ItemLocation;
use crate::reference::ReferenceManifest;

impl GarageRuntime {
    /// Writes this tick's poses and state into `frame`. `alpha` comes from the step clock.
    pub fn fill_frame(&mut self, reference: &ReferenceManifest, alpha: f32, frame: &mut SimFrame) {
        frame.changes.clear();
        let mut flags = 0;
        for (on, flag) in [
            (self.started, StateBlock::STARTED),
            (self.paused, StateBlock::PAUSED),
            (self.area.player.seated(), StateBlock::SEATED),
            (self.sleeping(), StateBlock::SLEEPING),
            (self.needs_output().dead, StateBlock::DEAD),
            (self.recovering, StateBlock::RECOVERING),
        ] {
            if on {
                flags |= flag;
            }
        }
        let area = &self.area;
        let stored_items = area.inventory.slots.map(|id| {
            id.filter(|id| {
                area.items
                    .get(*id)
                    .is_some_and(|i| matches!(i.location, ItemLocation::Stored(_)))
            })
            .map_or(0, |id| id.0 + 1)
        });
        frame.state = StateBlock {
            simulation_seconds: self.simulation_seconds,
            alpha,
            time_scale: self.clock_factors(reference).time_scale,
            flags,
            fov_degrees: area.player.view(&self.physics).fov_degrees,
            stored_items,
        };
        frame.valves.clear();
        frame.valves.extend(
            area.interaction
                .opening_valves()
                .map(|(id, valve)| ValveRow {
                    opening: id.0,
                    valve,
                }),
        );
        self.frame_poses.fill(&self.area, &self.physics, frame);
    }
    /// The renderer dropped a retired pose handle.
    pub fn release_pose(&mut self, handle: Handle) {
        self.frame_poses.release(handle);
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::{Change, PoseKey, SimFrame};
    use crate::reference::ReferenceManifest;
    use crate::simulation::FrameInput;
    use crate::starting_area::GarageRuntime;

    #[test]
    #[ignore = "requires the installed source export"]
    fn frame_poses_match_runtime_poses() {
        let reference = ReferenceManifest::from_runtime_json(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../web/assets/reference/manifest.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let mut run = GarageRuntime::new_default(&reference).unwrap();
        let mut frame = SimFrame::default();
        let mut keys = Vec::new();
        for _ in 0..10 {
            run.step(&reference, FrameInput::default());
            run.fill_frame(&reference, 0.5, &mut frame);
            for change in &frame.changes {
                if let Change::Spawn { handle, key } = change {
                    keys.push((*handle, *key));
                }
            }
        }
        assert_eq!(
            keys.len(),
            run.area.render_poses.len() + run.area.item_render_poses.len()
        );
        for (handle, key) in keys {
            let row = frame.current[handle.0 as usize];
            let pose = match key {
                PoseKey::Body(id) => run
                    .area
                    .render_poses
                    .iter()
                    .find(|p| p.0 == id)
                    .map(|p| p.2),
                PoseKey::Item(id) => run
                    .area
                    .item_render_poses
                    .iter()
                    .find(|p| p.0 == id)
                    .map(|p| p.2),
            }
            .unwrap();
            assert_eq!(row.pose(), pose);
        }
    }
}
