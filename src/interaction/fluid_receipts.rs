//! Source particle callbacks select one nearby cap per struck GameObject.
use super::fluid_contact::FluidCollision;
use super::*;
use crate::fluids::{FluidContainer, FluidSnapshot};
use std::ops::ControlFlow;

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

fn select_cap(
    source: OpeningId,
    position: Vector,
    caps: impl Iterator<Item = (OpeningId, Vector, f32)>,
) -> ControlFlow<Option<OpeningId>> {
    for (cap, center, radius) in caps {
        if cap == source {
            return ControlFlow::Break(None);
        }
        if (position - center).length() < radius * 0.1 {
            return ControlFlow::Break(Some(cap));
        }
    }
    ControlFlow::Continue(())
}

#[derive(Clone, Copy)]
enum ReceiptSource<'a> {
    Tank(&'a FluidContainer),
    Destination,
}

fn add_receipt(source: ReceiptSource<'_>, destination: &mut FluidContainer, liters: f32) {
    let entries = FluidSnapshot::from(match source {
        ReceiptSource::Tank(tank) => tank,
        ReceiptSource::Destination => &*destination,
    });
    for entry in entries.as_slice() {
        let current_source = match source {
            ReceiptSource::Tank(tank) => tank,
            ReceiptSource::Destination => &*destination,
        };
        let fraction =
            current_source.mixture().amount(entry.fluid_type as usize) / current_source.total();
        destination.change_one(liters * fraction, entry.fluid_type as usize);
    }
    destination.cleanup_one();
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
                let caps = tank.caps.iter().map(|cap| {
                    let opening = self
                        .openings
                        .iter()
                        .find(|o| o.reference.id == *cap)
                        .unwrap();
                    let pose = to_rapier(items.get(tank.item).unwrap().pose) * opening.local_pose;
                    (*cap, pose.translation, opening.reference.flow_radius)
                });
                if let ControlFlow::Break(target) = select_cap(source, event.position, caps) {
                    return target;
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
        let source_tank = self.openings[source_index].fluid_owner();
        let destination_tank = self.openings[destination_index].reference.tank;
        let Some(destination_fluid) = items.get(destination_tank).and_then(|i| i.fluid.as_ref())
        else {
            return false;
        };
        let receiver = &self.openings[destination_index];
        let valve = receiver.valve;
        let accepts = destination_fluid.total() < destination_fluid.capacity_liters && valve > 0.;
        let response = receiver.particles.as_ref().map(|p| (p.bounce, p.dampen));
        let emitter = &mut self.openings[source_index];
        if let Some(contact) = emitter.contact.as_mut() {
            if accepts {
                if let Some((bounce, dampen)) = response {
                    contact.receive_response(bounce, dampen, valve);
                }
            } else if let Some(particles) = emitter.particles.as_ref() {
                contact.restore_response(particles);
            }
        }
        // Old particles still change the response after the current spill stops.
        if !accepts || !(raw_poured > 0.) {
            return false;
        }
        let Some(_) = items.get(source_tank).and_then(|i| i.fluid.as_ref()) else {
            return false;
        };
        let liters = raw_poured * fluid_scale;
        if source_tank == destination_tank {
            add_receipt(
                ReceiptSource::Destination,
                items
                    .get_mut(destination_tank)
                    .unwrap()
                    .fluid
                    .as_mut()
                    .unwrap(),
                liters,
            );
        } else {
            let (source, destination) = items.get_pair_mut(source_tank, destination_tank).unwrap();
            add_receipt(
                ReceiptSource::Tank(source.fluid.as_ref().unwrap()),
                destination.fluid.as_mut().unwrap(),
                liters,
            );
        }
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
            let Some(contact) = self.openings[index].contact.as_ref() else {
                continue;
            };
            let raw_poured = contact.last_poured;
            group_callbacks(
                &mut self.callback_nodes,
                contact.collisions(),
                &self.light_nodes,
                physics,
            );
            let source = self.openings[index].reference.id;
            for callback in 0..self.callback_nodes.len() {
                let contact = self.openings[index].contact.as_ref().unwrap();
                if let Some(destination) = self.receipt_target(
                    source,
                    self.callback_nodes[callback],
                    contact.collisions(),
                    items,
                    physics,
                ) {
                    self.receive_fluid(source, destination, raw_poured, fluid_scale, items);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::{FluidContainer, FluidEntry};
    #[test]
    fn g04_receipts_isolated_selection() {
        let emitter = OpeningId(1);
        let first = OpeningId(2);
        let later = OpeningId(3);
        let center = Vector::ZERO;
        let caps = [(first, center, 2.), (later, center, 2.)];
        assert_eq!(
            select_cap(emitter, center, caps.into_iter()),
            ControlFlow::Break(Some(first))
        );
        assert_eq!(
            select_cap(emitter, Vector::X * 0.1999, caps.into_iter()),
            ControlFlow::Break(Some(first))
        );
        assert_eq!(
            select_cap(emitter, Vector::X * 0.2001, caps.into_iter()),
            ControlFlow::Continue(())
        );
        let stopped = [(emitter, Vector::X * 10., 2.), (later, center, 2.)];
        assert_eq!(
            select_cap(emitter, center, stopped.into_iter()),
            ControlFlow::Break(None)
        );
        let other_first = [(later, center, 2.), (emitter, center, 2.)];
        assert_eq!(
            select_cap(emitter, center, other_first.into_iter()),
            ControlFlow::Break(Some(later))
        );
    }

    #[test]
    fn g04_receipts_isolated_accounting() {
        let entries = [
            FluidEntry {
                fluid_type: 0,
                amount: 2.,
            },
            FluidEntry {
                fluid_type: 3,
                amount: 2.,
            },
        ];
        let mut source = FluidContainer::new(20., &entries);
        crate::fluids::spill(&mut source, 2.);
        assert!((source.mixture().amount(0) - 1.).abs() < 1e-6);
        assert!((source.mixture().amount(3) - 2. / 3.).abs() < 1e-6);
        let before = source.clone();
        let mut receiver = FluidContainer::new(20., &[]);
        add_receipt(ReceiptSource::Tank(&source), &mut receiver, 1. * 2.);
        assert_eq!(source, before);
        assert!((receiver.mixture().amount(0) - 1.2).abs() < 1e-6);
        assert!((receiver.mixture().amount(3) - 0.8).abs() < 1e-6);
        let mut self_receipt = FluidContainer::new(20., &entries);
        add_receipt(ReceiptSource::Destination, &mut self_receipt, 2.);
        assert!((self_receipt.mixture().amount(0) - 3.).abs() < 1e-6);
        assert!((self_receipt.mixture().amount(3) - 2.8).abs() < 1e-6);
        let mut limited = FluidContainer::new(1., &[]);
        add_receipt(ReceiptSource::Tank(&source), &mut limited, 2.);
        assert!((limited.total() - 1.).abs() < 1e-6);
    }

    #[test]
    #[ignore = "requires the installed single-Lada export"]
    fn g04_receipts_synthetic_drink_scales_raw_once() {
        let r = ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let mut run = crate::starting_area::GarageRuntime::new_default(&r).unwrap();
        let view = to_rapier(run.area.player.view(&run.physics).head_pose);
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
        }) = action.action
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
