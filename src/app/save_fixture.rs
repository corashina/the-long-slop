//! Controlled setup and observations for browser save acceptance. Default builds exclude this module.
use super::*;
use crate::save::SaveError;
use serde_json::{Value, json};

pub(super) fn process(world: &mut World, kind: &str) -> Result<String, SaveError> {
    let reference = world.resource::<RuntimeReference>().0.clone();
    let mut drive = world.resource_mut::<Drive>();
    let g = &mut drive.runtime;
    let error = |text: &str| SaveError::ReconstructionFailure(text.into());
    if !g.paused {
        return Err(error("Pause before fixture setup"));
    }
    let value: Value = match kind {
        "FixtureSetupFleet" => {
            if g.vehicles.len() != 2 || g.area.player.seat().is_some() {
                return Err(error("Fleet fixture needs two cars and a walking player"));
            }
            g.vehicle = 1;
            for index in 0..g.vehicles.len() {
                let vehicle = &mut g.vehicles[index];
                let mut engine = vehicle.engine.capture_save();
                engine.ignition = true;
                engine.rpm = 800. + index as f32 * 300.;
                engine.temperature = 40. + index as f32 * 20.;
                engine.elapsed = 10.;
                engine.last_run_time = 9.9;
                vehicle.engine_output = vehicle.engine.restore_save(&engine);
                vehicle.trip_m = 1234.5 + index as f64 * 400.;
                vehicle.handbrake = index == 0;
                g.area
                    .items
                    .get_mut(reference.vehicles[index].engine.item)
                    .unwrap()
                    .temperature = Some(engine.temperature);
                for _ in 0..=index {
                    g.lighting.cycle_headlights(&reference.lighting, index);
                }
            }
            json!({"vehicles":g.vehicles.len(),"selected":crate::save::InstanceId::initial(&reference.vehicles[g.vehicle].source)})
        }
        "FixtureSetupDriving" => {
            let road = crate::world::road_sample(g.stream.seed, 2044.);
            let pose = VehiclePose {
                position: [road.center[0] as f32, road.center[1] as f32 + 0.65, -2044.],
                rotation: [0., 0., 0., 1.],
            };
            g.physics.reset_preserving_wheels(g.vehicle, pose);
            g.stream.sync_at(road.center, &mut g.physics);
            g.area.sync_attachments(&mut g.physics, false);
            g.area
                .interaction
                .sync_parts(&mut g.area.items, &mut g.physics)?;
            g.wheels.sync(&mut g.area.items, &mut g.physics)?;
            g.physics.refresh_queries();
            let seat = reference.seats.iter().find(|s| s.driver).unwrap();
            let anchor = reference
                .anchors
                .iter()
                .find(|a| a.id == seat.anchor)
                .unwrap();
            if !g.area.player.enter(
                seat.id,
                VehiclePose {
                    position: anchor.transform.translation,
                    rotation: anchor.transform.rotation,
                },
                &mut g.physics,
            ) {
                return Err(error("Source driver seat is blocked"));
            }
            g.vehicles[g.vehicle].handbrake = false;
            g.area.items.sync_poses(&g.physics);
            g.area.commit_poses(&g.physics, true);
            json!({"position": pose.position, "velocity": g.physics.snapshot(g.vehicle).linear_velocity})
        }
        "FixtureDetachAndCarry" => {
            let slot = &reference.wheel_slots[0];
            let cover = g
                .area
                .interaction
                .parts
                .covering_item(slot.rim)
                .ok_or_else(|| error("Source wheel cover is missing"))?;
            g.area
                .interaction
                .parts
                .remove(cover, &mut g.area.items, &mut g.physics)?;
            g.area.interaction.parts.detach_mechanism(
                cover,
                &mut g.area.interaction.mechanisms,
                &mut g.physics,
            );
            let rim = g
                .wheels
                .remove(slot.id, &mut g.area.items, &mut g.physics)?;
            let item = g.area.items.get(cover).unwrap();
            let body = item.body.unwrap();
            let local = crate::vehicle::to_rapier(item.local_pose);
            let view = crate::vehicle::to_rapier(g.area.player.view_pose(&g.physics));
            let point =
                view.translation + view.rotation * rapier3d::math::Vector::new(0., 0., -0.35);
            let mut pose = *g.physics.rapier().bodies[body].position();
            pose.translation = point - pose.rotation * local.translation;
            g.physics.rapier_mut().bodies[body].set_position(pose, true);
            g.physics.refresh_queries();
            g.area.items.sync_poses(&g.physics);
            let point = point.to_array();
            g.area.interaction.pickup_item(
                cover,
                point,
                &g.area.player,
                &mut g.area.items,
                &mut g.physics,
            )?;
            g.area
                .interaction
                .sync_parts(&mut g.area.items, &mut g.physics)?;
            g.area.items.sync_poses(&g.physics);
            g.area.commit_poses(&g.physics, true);
            json!({"removedPart":cover.0,"removedRim":rim.0,"supportCount":g.physics.wheel_support_count(g.vehicle)})
        }
        "FixtureTransferFluid" => {
            let source = reference
                .tanks
                .iter()
                .find(|t| {
                    t.role == crate::reference::TankRole::Portable
                        && g.area
                            .items
                            .get(t.item)
                            .and_then(|i| i.fluid.as_ref())
                            .is_some_and(|f| f.mixture.quantities_liters[0] > 0.25)
                })
                .ok_or_else(|| error("No portable gasoline supply"))?
                .item;
            let target = reference.vehicles[g.vehicle].engine.fuel_tank;
            let mut supply = g.area.items.get(source).unwrap().fluid.clone().unwrap();
            let destination = g
                .area
                .items
                .get_mut(target)
                .unwrap()
                .fluid
                .as_mut()
                .unwrap();
            let before = [supply.mixture.total(), destination.mixture.total()];
            let transferred = crate::fluids::transfer(&mut supply, destination, 0.25);
            let after = [supply.mixture.total(), destination.mixture.total()];
            g.area.items.get_mut(source).unwrap().fluid = Some(supply);
            g.area.sync_portable_mass(&mut g.physics);
            let physics = g.physics.rapier_mut();
            for (_, body) in physics.bodies.iter_mut() {
                if body.is_dynamic() {
                    body.recompute_mass_properties_from_colliders(&physics.colliders);
                }
            }
            json!({"source":source.0,"target":target.0,"before":before,"after":after,"transferred":transferred})
        }
        "FixtureSetupCycle" => {
            let cycle = &reference.lighting.cycle;
            let total = f64::from(cycle.day_seconds + cycle.night_seconds);
            let current = g.lighting.cycle.capture_save().cycle_seconds;
            g.lighting.cycle.advance(cycle, total - current - 0.05);
            let controller = reference
                .lighting
                .controllers
                .iter()
                .find(|c| c.kind == crate::reference::LightControllerKind::House)
                .unwrap();
            g.lighting.use_control(&reference.lighting, controller.id);
            g.lighting.advance_fades(&reference.lighting, 0.001);
            json!({"cycle":g.lighting.cycle.capture_save()})
        }
        "FixtureInspect" => {
            let physics = g.physics.rapier();
            let mut duplicate = 0;
            let mut bad_owners = 0;
            let mut ownership_changes = Vec::new();
            for item in &reference.items {
                let state = g.area.items.get(item.id).unwrap();
                if let (Some(source), Some(owner)) = (g.area.bodies.get(&item.body), state.body) {
                    if *source != owner
                        && physics.bodies[*source].is_enabled()
                        && !physics.bodies[*source].colliders().is_empty()
                    {
                        let former_is_chassis = g.physics.vehicle_of(*source).is_some();
                        if !former_is_chassis {
                            duplicate += 1;
                        }
                        let source_record =
                            reference.bodies.iter().find(|b| b.id == item.body).unwrap();
                        let collider_ids: Vec<_> = reference
                            .parts
                            .iter()
                            .filter(|p| p.item == item.id)
                            .flat_map(|p| p.colliders.iter())
                            .chain(
                                reference
                                    .wheel_parts
                                    .iter()
                                    .filter(|p| p.item == item.id)
                                    .flat_map(|p| p.colliders.iter()),
                            )
                            .collect();
                        let mut checked = 0;
                        for id in collider_ids {
                            let matches: Vec<_> = physics
                                .colliders
                                .iter()
                                .filter(|(_, c)| ((c.user_data >> 64) as u32) == id.0)
                                .collect();
                            if matches.len() != 1 {
                                bad_owners += 1;
                            }
                            for (_, collider) in matches {
                                checked += 1;
                                if collider.parent() != Some(owner) {
                                    bad_owners += 1;
                                }
                            }
                        }
                        ownership_changes.push(json!({"item":crate::save::InstanceId::initial(&item.source),"formerBody":crate::save::SourceKey::from(&source_record.source),"formerIsChassis":former_is_chassis,"formerEnabled":physics.bodies[*source].is_enabled(),"formerColliderCount":physics.bodies[*source].colliders().len(),"currentEnabled":physics.bodies[owner].is_enabled(),"currentColliderCount":physics.bodies[owner].colliders().len(),"checkedItemColliders":checked,"currentRootItems":g.area.items.iter().filter(|i|i.body==Some(owner)&&matches!(i.location,crate::interaction::items::ItemLocation::World|crate::interaction::items::ItemLocation::Held)).filter_map(|i|reference.items.iter().find(|r|r.id==i.id)).map(|i|crate::save::InstanceId::initial(&i.source)).collect::<Vec<_>>() }));
                    }
                }
            }
            for (handle, body) in physics.bodies.iter() {
                for collider in body.colliders() {
                    if physics.colliders[*collider].parent() != Some(handle) {
                        bad_owners += 1;
                    }
                }
            }
            let held = g.area.interaction.held();
            let held_body = held
                .and_then(|id| g.area.items.get(id))
                .and_then(|i| i.body);
            let carry_joints = held_body.map_or(0, |body| {
                physics
                    .impulse_joints
                    .iter()
                    .filter(|(_, joint)| joint.body1() == body || joint.body2() == body)
                    .count()
            });
            json!({"duplicateAssemblyBodies":duplicate,"flaggedOwnershipChanges":ownership_changes,"badColliderOwners":bad_owners,"bodyCount":physics.bodies.len(),"colliderCount":physics.colliders.len(),"wheelSupport":g.physics.wheel_support_count(g.vehicle),"wheelContacts":g.physics.snapshot(g.vehicle).wheels_contact,"carryJoints":carry_joints,"held":held.map(|i|i.0),"heldPosition":held.and_then(|id|g.area.items.get(id)).map(|i|i.pose.position),"vehiclePosition":g.stream.origin.absolute(g.physics.snapshot(g.vehicle).pose.position),"velocity":g.physics.snapshot(g.vehicle).linear_velocity,"rebases":g.rebases,"origin":g.stream.origin.offset})
        }
        _ => return Err(error("Unknown fixture request")),
    };
    drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
    drive.previous = drive.current;
    drive.rendered = drive.current;
    Ok(value.to_string())
}
