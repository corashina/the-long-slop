//! Renderer data of one tick.
use super::GarageRuntime;
use crate::boundary::{
    BudgetRow, Change, CigaretteRow, ConditionRow, Handle, PlaneRow, PoseRow, SimFrame, StateBlock,
    ValveRow,
};
use crate::interaction::cigarettes::EmberPhase;
use crate::interaction::items::ItemLocation;
use crate::reference::{ItemId, ReferenceManifest};
use crate::vehicle::VehiclePose;
use crate::world::{ObjectKey, WorldLayer};

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
            (
                self.stream.frame().failure.is_some(),
                StateBlock::WORLD_FAILED,
            ),
        ] {
            if on {
                flags |= flag;
            }
        }
        let needs = self.needs_output();
        let dead_weight = if needs.dead {
            (needs.death_seconds as f32 / reference.survival.death_seconds).clamp(0., 1.)
        } else {
            0.
        };
        let area = &self.area;
        let stored_items = area.inventory.slots.map(|id| {
            id.filter(|id| {
                area.items
                    .get(*id)
                    .is_some_and(|i| matches!(i.location, ItemLocation::Stored(_)))
            })
            .map_or(0, |id| id.0 + 1)
        });
        let c = &area.cigarettes;
        let lit = |id: &ItemId| {
            c.cigarette(*id)
                .is_some_and(|s| s.ember.phase != EmberPhase::Start)
        };
        // One ember light: the worn cigarette, then the held one, then any lit one.
        let ember_light = c
            .mouth
            .item
            .filter(lit)
            .or(area.interaction.held().filter(lit))
            .or_else(|| c.cigarettes.iter().map(|(id, _)| *id).find(lit));
        let anchor = self.sky_anchor(reference);
        let offset = self.stream.origin.offset;
        frame.state = StateBlock {
            simulation_seconds: self.simulation_seconds,
            origin: area.origin,
            cycle_elapsed_seconds: self.lighting.cycle.elapsed_seconds(),
            session: self.stream.frame().request.session,
            selection_revision: self.stream.required_content().selection_revision,
            alpha,
            time_scale: self.clock_factors(reference).time_scale,
            flags,
            fov_degrees: area.player.view(&self.physics).fov_degrees,
            stored_items,
            body_pose: PoseRow::new(area.player.body_pose(&self.physics), 0),
            body_root: PoseRow::new(area.body.root_local(), 0),
            joint_scale: area.body.joint_scale(),
            sky_anchor: std::array::from_fn(|i| (anchor[i] - offset[i]) as f32),
            ember_light: ember_light.map_or(0, |id| id.0 + 1),
            dead_weight,
            motion_revision: self.audio_motion_revision,
            far_plane: self.distances.cam2_far_plane,
            sleep_camera: self
                .sleep_camera()
                .map_or(PoseRow::default(), |camera| PoseRow {
                    position: self.stream.origin.local(camera.absolute_position),
                    flags: PoseRow::ACTIVE,
                    rotation: camera.rotation,
                }),
            seat_car: match (area.player.seat_vehicle(), self.vehicle.as_ref()) {
                (Some(_), Some(key)) => {
                    PoseRow::new(self.physics.snapshot(key).pose, PoseRow::ACTIVE)
                }
                _ => PoseRow::default(),
            },
            prepared_roads: self.stream.metrics().prepared_roads as u32,
            pad: 0,
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
        frame.cigarettes.clear();
        frame
            .cigarettes
            .extend(c.cigarettes.iter().map(|(id, state)| {
                let out = state.ember.output(&c.rates.ember);
                CigaretteRow {
                    item: id.0,
                    blend: state.part_blend(),
                    color: out.color,
                    light_intensity: out.light_intensity,
                }
            }));
        self.fill_particles(reference, frame);
        self.fill_conditions(reference, frame);
        self.fill_followed_car(frame);
        self.fill_hud(reference, frame);
        frame.budgets.clear();
        if !self.started {
            let stream = &self.stream;
            frame.budgets.extend(
                stream
                    .presentation_budgets(stream.presentation_allocation())
                    .map(|b| BudgetRow {
                        used: b.used as f64,
                        limit: b.limit as f64,
                    }),
            );
        }
        if let Some(aims) = self.home_cable(reference) {
            frame.changes.push(Change::HomeCable(aims));
        }
        frame.planes.clear();
        frame
            .planes
            .extend(self.sky().planes().iter().map(|plane| PlaneRow {
                id: plane.id,
                prefab: plane.prefab as u32,
                offset: plane.offset.to_array(),
                heading: plane.heading.to_array(),
            }));
        let lighting = &self.lighting;
        frame.lighting.cycle.clone_from(lighting.cycle.output());
        frame.lighting.road_lamp = lighting.road_lamp_output(&reference.lighting);
        frame.lights.clear();
        frame.lights.extend_from_slice(lighting.light_outputs());
        frame.lamp_materials.clear();
        frame
            .lamp_materials
            .extend_from_slice(lighting.material_outputs());
        frame.control_poses.clear();
        frame
            .control_poses
            .extend_from_slice(lighting.control_poses());
        frame.node_visibility.clear();
        frame
            .node_visibility
            .extend_from_slice(lighting.visibility_outputs());
        self.frame_poses
            .fill(reference, &self.area, &self.physics, frame);
        frame.ragdoll.clear();
        let area = &self.area;
        if area.ragdoll.active() {
            frame
                .ragdoll
                .extend(reference.clothing.ragdoll.bodies.iter().map(|b| {
                    area.ragdoll.node_pose(&self.physics, b.node).map_or_else(
                        PoseRow::default,
                        |f| {
                            let pose = VehiclePose {
                                position: f.position.to_array(),
                                rotation: f.rotation.to_array(),
                            };
                            PoseRow::new(pose, PoseRow::VISIBLE)
                        },
                    )
                }));
        }
    }
    fn fill_conditions(&mut self, reference: &ReferenceManifest, frame: &mut SimFrame) {
        let store = &mut self.area.conditions;
        frame.conditions.clear();
        if self.conditions_sent {
            while let Some(id) = store.take_changed() {
                frame.conditions.push(ConditionRow::new(id, store.get(id)));
            }
        } else {
            while store.take_changed().is_some() {}
            frame
                .conditions
                .extend(store.iter().map(|(id, s)| ConditionRow::new(id, s)));
            self.conditions_sent = true;
        }
        frame.washes.clear();
        frame
            .washes
            .extend(store.washes.iter().map(|w| u8::from(w.visible())));
        let cleaning = &self.area.interaction.cleaning;
        frame.brush_poses.clear();
        frame.brush_poses.extend(
            (0..reference.conditions.brushes.len())
                .map(|i| PoseRow::new(cleaning.model_pose(i, &self.area.items), 0)),
        );
        frame.spray_colors.clear();
        frame.spray_colors.extend(
            reference
                .conditions
                .sprays
                .iter()
                .map(|t| cleaning.get(t.item).unwrap().color.unwrap()),
        );
    }
    /// Source probe follow: the world car the player last sat in, else the start car.
    fn fill_followed_car(&mut self, frame: &mut SimFrame) {
        let car = match self.vehicle.as_ref() {
            Some(key @ ObjectKey::Generated(id)) => self.vehicles.get(key).map(|v| (id, v)),
            Some(_) => None,
            None => self.vehicles.iter().find_map(|(key, vehicle)| match key {
                ObjectKey::Generated(id) if id.layer == WorldLayer::Startup => Some((id, vehicle)),
                _ => None,
            }),
        }
        .map(|(id, v)| (id, v.prototype as u32));
        if car != self.followed_car.as_ref().map(|(id, p)| (id, *p)) {
            self.followed_car = car.map(|(id, p)| (id.clone(), p));
            frame
                .changes
                .push(Change::FollowedCar(self.followed_car.clone()));
        }
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
                if let Change::Spawn { handle, key, .. } = change {
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
