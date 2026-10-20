//! Source particle callbacks select one nearby cap per struck GameObject.
use super::fluid_contact::FluidCollision;
use super::*;

pub(super) struct ReceiptTank {
    item: ItemId,
    node: NodeId,
    caps: Vec<OpeningId>,
}

impl ReceiptTank {
    pub(super) fn retained_cap_bytes(&self) -> usize {
        super::memory::vector(&self.caps)
    }
}

pub(super) fn tank_nodes(reference: &ReferenceManifest) -> Vec<ReceiptTank> {
    let mut tanks: Vec<_> = reference
        .tanks
        .iter()
        .map(|tank| ReceiptTank {
            item: tank.item,
            node: reference
                .items
                .iter()
                .find(|i| i.id == tank.item)
                .unwrap()
                .node,
            caps: tank.openings.clone(),
        })
        .collect();
    tanks.sort_unstable_by_key(|tank| tank.node.0);
    tanks
}

fn group_callbacks(
    nodes: &mut Vec<NodeId>,
    events: &[FluidCollision],
    collider_nodes: &HashMap<ColliderId, NodeId>,
    physics: &PhysicsWorld,
) {
    nodes.clear();
    for event in events {
        if let Some(node) = collision_node(event, collider_nodes, physics)
            && !nodes.contains(&node)
        {
            nodes.push(node);
        }
    }
    nodes.sort_unstable_by_key(|node| node.0);
}

fn collision_node(
    event: &FluidCollision,
    nodes: &HashMap<ColliderId, NodeId>,
    physics: &PhysicsWorld,
) -> Option<NodeId> {
    let collider = physics.rapier().colliders.get(event.collider)?;
    nodes
        .get(&ColliderId((collider.user_data >> 64) as u32))
        .copied()
}

impl InteractionState {
    fn receipt_target(
        &self,
        source: OpeningId,
        game_object: NodeId,
        events: &[FluidCollision],
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> Option<OpeningId> {
        let root = self.mechanisms.root_node(game_object)?;
        for tank in &self.receipt_tanks {
            if self.mechanisms.root_node(tank.node) != Some(root)
                || items
                    .get(tank.item)
                    .is_none_or(|i| i.inactive_pose.is_some())
            {
                continue;
            }
            for event in events {
                if collision_node(event, &self.light_nodes, physics) != Some(game_object) {
                    continue;
                }
                for cap in &tank.caps {
                    if *cap == source {
                        return None;
                    }
                    let opening = self
                        .openings
                        .iter()
                        .find(|o| o.reference.id == *cap)
                        .unwrap();
                    let pose = to_rapier(items.get(tank.item)?.pose) * opening.local_pose;
                    if (event.position - pose.translation).length()
                        < opening.reference.flow_radius * 0.1
                    {
                        return Some(*cap);
                    }
                }
            }
        }
        None
    }

    fn receive_fluid(
        &mut self,
        source: OpeningId,
        destination: OpeningId,
        raw_poured: f32,
        fluid_scale: f32,
        items: &mut ItemStore,
    ) -> bool {
        let source_index = self
            .openings
            .iter()
            .position(|o| o.reference.id == source)
            .unwrap();
        let destination_index = self
            .openings
            .iter()
            .position(|o| o.reference.id == destination)
            .unwrap();
        let source_tank = self.openings[source_index].reference.tank;
        let destination_tank = self.openings[destination_index].reference.tank;
        let Some(destination_fluid) = items.get(destination_tank).and_then(|i| i.fluid.as_ref())
        else {
            return false;
        };
        if !(destination_fluid.total() < destination_fluid.capacity_liters
            && self.openings[destination_index].valve > 0.)
        {
            return false;
        }
        let Some(source_fluid) = items.get(source_tank).and_then(|i| i.fluid.as_ref()) else {
            return false;
        };
        let entries = crate::fluids::FluidSnapshot::from(source_fluid);
        for entry in entries.as_slice() {
            // A self-receipt reads the fraction after each earlier addition.
            let source_fluid = items.get(source_tank).unwrap().fluid.as_ref().unwrap();
            let fraction =
                source_fluid.mixture().amount(entry.fluid_type as usize) / source_fluid.total();
            items
                .get_mut(destination_tank)
                .unwrap()
                .fluid
                .as_mut()
                .unwrap()
                .change_one(
                    raw_poured * fluid_scale * fraction,
                    entry.fluid_type as usize,
                );
        }
        items
            .get_mut(destination_tank)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap()
            .cleanup_one();
        self.openings[destination_index].flow = Some((true, 1.));
        self.openings[destination_index].fill_called = true;
        true
    }

    pub(super) fn apply_fluid_receipts(
        &mut self,
        items: &mut ItemStore,
        physics: &PhysicsWorld,
        fluid_scale: f32,
    ) {
        for index in 0..self.openings.len() {
            let Some(contact) = self.openings[index].contact.take() else {
                continue;
            };
            if contact.last_poured > 0. {
                group_callbacks(
                    &mut self.callback_nodes,
                    contact.collisions(),
                    &self.light_nodes,
                    physics,
                );
                let source = self.openings[index].reference.id;
                for callback in 0..self.callback_nodes.len() {
                    if let Some(destination) = self.receipt_target(
                        source,
                        self.callback_nodes[callback],
                        contact.collisions(),
                        items,
                        physics,
                    ) {
                        self.receive_fluid(
                            source,
                            destination,
                            contact.last_poured,
                            fluid_scale,
                            items,
                        );
                    }
                }
            }
            self.openings[index].contact = Some(contact);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::{FluidContainer, FluidEntry};
    #[test]
    fn g04_receipts_live_fractions_and_sleep_scale() {
        let r = ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let mut run = crate::starting_area::GarageRuntime::new(&r).unwrap();
        let source = r.openings.iter().find(|o| o.tank == ItemId(116)).unwrap();
        let destination = r
            .openings
            .iter()
            .find(|o| o.tank == r.vehicles[0].engine.fuel_tank)
            .unwrap();
        run.area.items.get_mut(source.tank).unwrap().fluid = Some(FluidContainer::new(
            20.,
            &[
                FluidEntry {
                    fluid_type: 0,
                    amount: 2.,
                },
                FluidEntry {
                    fluid_type: 3,
                    amount: 2.,
                },
            ],
        ));
        crate::fluids::spill(
            run.area
                .items
                .get_mut(source.tank)
                .unwrap()
                .fluid
                .as_mut()
                .unwrap(),
            2.,
        );
        run.area.items.get_mut(destination.tank).unwrap().fluid =
            Some(FluidContainer::new(40., &[]));
        let before = run.area.items.get(source.tank).unwrap().fluid.clone();
        let remaining = before.as_ref().unwrap().mixture();
        assert!((remaining.amount(0) - 1.).abs() < 1e-6);
        assert!((remaining.amount(3) - 2. / 3.).abs() < 1e-6);
        run.area
            .interaction
            .openings
            .iter_mut()
            .find(|o| o.reference.id == destination.id)
            .unwrap()
            .valve = 1.;
        run.area
            .interaction
            .tank_rules
            .get_mut(&source.tank)
            .unwrap()
            .no_fill_from = true;
        run.area
            .interaction
            .tank_rules
            .get_mut(&destination.tank)
            .unwrap()
            .no_fill_this = true;
        assert!(run.area.interaction.receive_fluid(
            source.id,
            destination.id,
            1.,
            2.,
            &mut run.area.items
        ));
        assert_eq!(run.area.items.get(source.tank).unwrap().fluid, before);
        let fluid = run
            .area
            .items
            .get(destination.tank)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap();
        assert!((fluid.mixture().amount(0) - 1.2).abs() < 1e-6);
        assert!((fluid.mixture().amount(3) - 0.8).abs() < 1e-6);
    }
    #[test]
    fn g04_receipts_callback_groups_caps_and_current_roots() {
        use rapier3d::prelude::ColliderBuilder;
        let r = ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let mut run = crate::starting_area::GarageRuntime::new(&r).unwrap();
        let receiver = r.vehicles[0].engine.fuel_tank;
        let cap = r.openings.iter().find(|o| o.tank == receiver).unwrap();
        let emitter = r
            .openings
            .iter()
            .find(|o| o.tank == ItemId(116))
            .unwrap()
            .id;
        let chassis = run.physics.chassis_handle(0);
        let node = r.items.iter().find(|i| i.id == receiver).unwrap().node;
        let other_node = r
            .items
            .iter()
            .find(|i| i.id == r.vehicles[0].engine.item)
            .unwrap()
            .node;
        let handles: Vec<_> = (0..3)
            .map(|i| {
                run.physics.insert_collider_with_parent(
                    ColliderBuilder::ball(0.01).user_data(((90000 + i) as u128) << 64 | 123),
                    chassis,
                )
            })
            .collect();
        let nodes = &mut run.area.interaction.light_nodes;
        nodes.insert(ColliderId(90000), node);
        nodes.insert(ColliderId(90001), node);
        nodes.insert(ColliderId(90002), other_node);
        let position = to_rapier(run.area.items.get(receiver).unwrap().pose)
            * run
                .area
                .interaction
                .openings
                .iter()
                .find(|o| o.reference.id == cap.id)
                .unwrap()
                .local_pose;
        let mut events = [FluidCollision {
            collider: handles[0],
            position: position.translation,
        }; 3];
        events[1].collider = handles[1];
        events[2].collider = handles[2];
        let state = &mut run.area.interaction;
        let capacity = state.callback_nodes.capacity();
        group_callbacks(
            &mut state.callback_nodes,
            &events[..2],
            &state.light_nodes,
            &run.physics,
        );
        let callbacks_for_two_colliders_on_one_node = state.callback_nodes.len();
        assert_eq!(callbacks_for_two_colliders_on_one_node, 1);
        group_callbacks(
            &mut state.callback_nodes,
            &events,
            &state.light_nodes,
            &run.physics,
        );
        let callbacks_for_two_nodes_on_one_body = state.callback_nodes.len();
        assert_eq!(callbacks_for_two_nodes_on_one_body, 2);
        for _ in 0..256 {
            group_callbacks(
                &mut state.callback_nodes,
                &events,
                &state.light_nodes,
                &run.physics,
            );
        }
        assert_eq!(state.callback_nodes.capacity(), capacity);
        state.receipt_tanks.retain(|t| t.item == receiver);
        let first = state
            .openings
            .iter()
            .position(|o| o.reference.id == cap.id)
            .unwrap();
        let second_id = OpeningId(90000);
        let second = OpeningState {
            reference: OpeningRules {
                id: second_id,
                tank: receiver,
                cap: cap.cap,
                flow_liters_per_second: 1.,
                flow_radius: cap.flow_radius,
                tilt_degrees: 0.,
                angle_disabled: true,
                spill_disabled: true,
                tube_connected: false,
                can_toggle: true,
                can_tune: false,
                tune_rate: 0.,
            },
            local_pose: state.openings[first].local_pose,
            tilt_axis: Vector::Y,
            valve: 1.,
            flow: None,
            fill_called: false,
            empty_pour: false,
            particles: None,
            contact: None,
        };
        state.openings.push(second);
        state.receipt_tanks[0].caps = vec![cap.id, second_id];
        state.openings[first].valve = 0.;
        let selected_with_closed_first_cap =
            state.receipt_target(emitter, node, &events, &run.area.items, &run.physics);
        assert_eq!(selected_with_closed_first_cap, Some(cap.id));
        assert!(!state.receive_fluid(emitter, cap.id, 1., 1., &mut run.area.items));
        state.openings[first].valve = 1.;
        run.area.items.get_mut(receiver).unwrap().fluid = Some(FluidContainer::new(
            1.,
            &[FluidEntry {
                fluid_type: 0,
                amount: 1.,
            }],
        ));
        let selected_with_full_first_cap =
            state.receipt_target(emitter, node, &events, &run.area.items, &run.physics);
        assert_eq!(selected_with_full_first_cap, Some(cap.id));
        assert!(!state.receive_fluid(emitter, cap.id, 1., 1., &mut run.area.items));
        let mut boundary = [events[0]];
        boundary[0].position += Vector::X * cap.flow_radius * 0.0999;
        assert_eq!(
            state.receipt_target(emitter, node, &boundary, &run.area.items, &run.physics),
            Some(cap.id)
        );
        boundary[0].position = position.translation + Vector::X * cap.flow_radius * 0.1001;
        assert_eq!(
            state.receipt_target(emitter, node, &boundary, &run.area.items, &run.physics),
            None
        );
        state.receipt_tanks[0].caps = vec![emitter, second_id];
        let selected_after_emitter_cap =
            state.receipt_target(emitter, node, &events, &run.area.items, &run.physics);
        assert_eq!(selected_after_emitter_cap, None);
        state.receipt_tanks[0].caps = vec![second_id, cap.id];
        let selected_other_cap_on_emitter_tank =
            state.receipt_target(cap.id, node, &events, &run.area.items, &run.physics);
        assert_eq!(selected_other_cap_on_emitter_tank, Some(second_id));
        run.area.items.get_mut(receiver).unwrap().fluid = Some(FluidContainer::new(
            20.,
            &[
                FluidEntry {
                    fluid_type: 0,
                    amount: 2.,
                },
                FluidEntry {
                    fluid_type: 3,
                    amount: 2.,
                },
            ],
        ));
        assert!(state.receive_fluid(cap.id, second_id, 2., 1., &mut run.area.items));
        let fluid = run
            .area
            .items
            .get(receiver)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap();
        assert!((fluid.mixture().amount(0) - 3.).abs() < 1e-6);
        assert!((fluid.mixture().amount(3) - 2.8).abs() < 1e-6);
        let tank_node = r
            .items
            .iter()
            .find(|i| i.id == r.vehicles[0].engine.oil_tank)
            .unwrap()
            .node;
        let mechanism = &mut state.mechanisms;
        let original_root = mechanism.root_node(tank_node).unwrap();
        mechanism.set_item_pose(
            r.vehicles[0].engine.item,
            run.area.items.get(r.vehicles[0].engine.item).unwrap().pose,
        );
        assert_eq!(
            mechanism.root_node(tank_node),
            Some(original_root),
            "a cached external pose does not detach a mounted tank"
        );
        mechanism.set_part_root(other_node, Some(chassis));
        assert_eq!(mechanism.root_node(tank_node), Some(other_node));
        mechanism.mount_part(other_node, node, [0., 0., 0., 1.]);
        assert_eq!(mechanism.root_node(tank_node), Some(original_root));
        let portable = r.items.iter().find(|i| i.id == ItemId(116)).unwrap().node;
        assert_eq!(mechanism.root_node(portable), Some(portable));
    }
    #[test]
    fn g04_receipts_synthetic_drink_scales_raw_once() {
        let r = ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let mut run = crate::starting_area::GarageRuntime::new(&r).unwrap();
        let view = to_rapier(run.area.player.view_pose(&run.physics));
        let center = view.translation + view.rotation * run.area.interaction.drink_collider_center;
        let source = ItemId(116);
        run.area.items.get_mut(source).unwrap().fluid = Some(FluidContainer::new(
            4.,
            &[FluidEntry {
                fluid_type: 0,
                amount: 4.,
            }],
        ));
        let opening = run
            .area
            .interaction
            .openings
            .iter_mut()
            .find(|o| o.reference.tank == source)
            .unwrap();
        let id = opening.reference.id;
        let cap = to_rapier(run.area.items.get(source).unwrap().pose) * opening.local_pose;
        let emitter = cap.inverse() * Pose::from_translation(center);
        opening.valve = 1.;
        opening.reference.angle_disabled = true;
        opening.reference.spill_disabled = false;
        opening.reference.tube_connected = false;
        opening.reference.flow_liters_per_second = 1.;
        let particles = opening.particles.as_mut().unwrap();
        particles.local_pose.translation = emitter.translation.to_array();
        particles.local_pose.rotation = emitter.rotation.to_array();
        particles.min_rate = 600.;
        particles.max_rate = 600.;
        particles.max_speed = 1.;
        particles.cone_radius = 0.;
        particles.gravity_multiplier = 0.;
        let action = run.area.interaction.step(
            FrameInput::default(),
            ResolvedAim::default(),
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            crate::simulation::FIXED_DT,
            run.survival.output(),
            2.,
        );
        let Some(InteractionAction::DrinkContact {
            source: actual_source,
            poured_liters,
            ..
        }) = action
        else {
            panic!("synthetic player sphere must receive the emitted particles");
        };
        assert_eq!(actual_source, source);
        assert_eq!(poured_liters, crate::simulation::FIXED_DT * 2.);
        assert_eq!(
            run.area
                .interaction
                .fluid_particles()
                .find(|(cap, _)| *cap == id)
                .unwrap()
                .1
                .last_poured,
            crate::simulation::FIXED_DT
        );
    }
}
