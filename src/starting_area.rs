//! Persistent imported home bodies and preparation state.
pub mod recovery;
use crate::interaction::{
    InteractionAction, InteractionState, ResolvedAim,
    items::{ItemStore, node_pose},
};
use crate::player::PlayerState;
use crate::reference::{BodyId, ItemId, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::{
    DriveInput, FIXED_DT, FrameInput,
    engine::{EngineInput, EngineOutput, EngineState, InstalledEngineInput},
    ignition::{IgnitionInput, IgnitionState},
    keys,
};
use crate::vehicle::{
    PhysicsWorld, VEHICLE_COLLIDER_FLAG, VehiclePose, collider_builder, to_rapier,
};
use crate::world::{DEFAULT_SEED, WorldStream};
use rapier3d::prelude::{
    Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder, RigidBodyHandle,
};
use std::collections::HashMap;

mod survival;
pub use survival::DrinkResult;
pub(crate) use survival::active_part as active_food_part;

struct CompoundCollider {
    handle: crate::vehicle::ColliderHandle,
    body: BodyId,
    root: BodyId,
    fixed_pose: rapier3d::math::Pose,
    shape_pose: rapier3d::math::Pose,
}

impl CompoundCollider {
    fn local_pose(&self, interaction: &InteractionState) -> rapier3d::math::Pose {
        interaction
            .mechanisms
            .body_local_pose(self.body, self.root)
            .unwrap_or(self.fixed_pose)
            * self.shape_pose
    }
}

pub struct StartingArea {
    pub conditions: crate::conditions::ConditionStore,
    pub plates: crate::plates::PlateState,
    _plate_draw: crate::plates::StartupDraw,
    pub bodies: HashMap<BodyId, RigidBodyHandle>,
    pub items: ItemStore,
    pub interaction: InteractionState,
    pub inventory: crate::inventory::InventoryState,
    pub pose: crate::player::pose::PlayerPoseState,
    inventory_offsets: [Option<(ItemId, VehiclePose)>; 4],
    inventory_clutch_conflict: bool,
    pub player: PlayerState,
    pub origin: [f64; 3],
    attachments: Vec<(RigidBodyHandle, RigidBodyHandle, rapier3d::math::Pose)>,
    pub render_poses: Vec<(BodyId, VehiclePose, VehiclePose)>,
    pub item_render_poses: Vec<(ItemId, VehiclePose, VehiclePose)>,
    compound_colliders: Vec<CompoundCollider>,
    portable_mass: Vec<(RigidBodyHandle, f32, Vec<ItemId>, f32)>,
}

impl StartingArea {
    pub fn new(
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
        origin: [f64; 3],
    ) -> Result<Self, ReferenceError> {
        let mut plate_draw = crate::plates::StartupDraw::new(DEFAULT_SEED);
        let plates =
            crate::plates::PlateState::new(&reference.plates, &mut |a, b| plate_draw.draw(a, b))?;
        let conditions = crate::conditions::ConditionStore::new(reference)?;
        let mut items = ItemStore::new(reference)?;
        let mut interaction = InteractionState::new(reference)?;
        let mut bodies = HashMap::with_capacity(reference.bodies.len());
        let mut compound_colliders = Vec::new();
        for (index, vehicle) in reference.vehicles.iter().enumerate() {
            bodies.insert(vehicle.body, physics.chassis_handle(index));
        }
        for body in &reference.bodies {
            if reference.vehicles.iter().any(|v| v.body == body.id) {
                continue;
            }
            let pose = VehiclePose {
                position: std::array::from_fn(|i| body.pose.translation[i] - origin[i] as f32),
                rotation: body.pose.rotation,
            };
            let builder = if body.parent_body.is_some() {
                RigidBodyBuilder::kinematic_position_based()
            } else if body.dynamic {
                RigidBodyBuilder::dynamic()
                    .ccd_enabled(true)
                    .additional_mass(body.mass_kg)
                    .linear_damping(body.linear_damping)
                    .angular_damping(body.angular_damping)
            } else {
                RigidBodyBuilder::fixed()
            };
            let inactive_flashlight = reference
                .flashlights
                .instances
                .iter()
                .any(|instance| instance.node == body.node && !instance.startup_active);
            let handle =
                physics.insert_body(builder.enabled(!inactive_flashlight).pose(to_rapier(pose)));
            bodies.insert(body.id, handle);
        }
        for record in &reference.colliders {
            if reference
                .vehicles
                .iter()
                .any(|v| v.colliders.contains(&record.id))
            {
                continue;
            }
            let item = reference
                .action_bindings
                .iter()
                .find(|binding| binding.collider == record.id)
                .and_then(|binding| binding.use_owner)
                .or_else(|| collider_item(reference, record.node));
            let vehicle = body_vehicle(reference, record.body);
            let flag = if vehicle.is_some() {
                VEHICLE_COLLIDER_FLAG
            } else {
                0
            };
            let builder = collider_builder(&record.geometry)?
                .friction(record.friction)
                .restitution(record.restitution)
                .sensor(record.sensor)
                .enabled(record.enabled)
                .solver_groups(crate::vehicle::solver_groups(record.enabled))
                .density(0.0)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(record.membership),
                    Group::from_bits_truncate(record.filter),
                    InteractionTestMode::And,
                ))
                .user_data(
                    flag | ((record.id.0 as u128) << 64)
                        | item.map_or(0, |item| item.0 as u128 + 1),
                );
            let owner = bodies[&record.body];
            let mut collider = builder.build();
            let local = *collider.position();
            let physical_parent = match vehicle.map(|v| physics.chassis_handle(v)) {
                Some(chassis) if owner != chassis => {
                    collider.set_position(
                        physics.rapier().bodies[chassis].position().inverse()
                            * *physics.rapier().bodies[owner].position()
                            * local,
                    );
                    chassis
                }
                _ => owner,
            };

            let handle = physics.insert_collider_with_parent(collider, physical_parent);
            if physical_parent != owner {
                let source = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == record.body)
                    .unwrap();
                let root = reference.vehicles[vehicle.unwrap()].body;
                let parent = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == root)
                    .unwrap();
                compound_colliders.push(CompoundCollider {
                    handle,
                    body: record.body,
                    root,
                    fixed_pose: to_rapier(VehiclePose {
                        position: parent.pose.translation,
                        rotation: parent.pose.rotation,
                    })
                    .inverse()
                        * to_rapier(VehiclePose {
                            position: source.pose.translation,
                            rotation: source.pose.rotation,
                        }),
                    shape_pose: local,
                });
            }
        }
        // The vehicle constructor owns these shapes. Add only their aim identity.
        physics.edit_colliders(|_, collider| {
            if collider
                .parent()
                .is_some_and(|parent| reference.vehicles.iter().any(|v| bodies[&v.body] == parent))
            {
                let source = (collider.user_data >> 64) as u32;
                if let Some(record) = reference
                    .colliders
                    .iter()
                    .find(|record| record.id.0 == source)
                {
                    let item = reference
                        .action_bindings
                        .iter()
                        .find(|binding| binding.collider == record.id)
                        .and_then(|binding| binding.use_owner)
                        .or_else(|| collider_item(reference, record.node));
                    collider.user_data = (collider.user_data & !(u32::MAX as u128))
                        | item.map_or(0, |id| id.0 as u128 + 1);
                }
            }
        });
        for (&id, &handle) in &bodies {
            items.bind_body(id, handle);
            interaction.bind_body(id, handle);
        }
        for slot in &reference.wheel_slots {
            let builder = rapier3d::prelude::ColliderBuilder::ball(slot.radius)
                .sensor(true)
                .enabled(false)
                .density(0.0)
                .user_data(
                    VEHICLE_COLLIDER_FLAG
                        | crate::vehicle::SLOT_SENSOR_FLAG
                        | ((slot.id.0 as u128) << 96)
                        | (slot.wheel.0 as u128 + 1),
                );

            let handle = physics.insert_collider(builder);
            interaction.bind_wheel_sensor(handle, slot.rim, physics);
        }
        for (handle, collider) in physics.rapier().colliders.iter() {
            let id = crate::reference::ColliderId((collider.user_data >> 64) as u32);
            if id.0 != 0 {
                interaction.bind_collider(id, handle);
            }
        }
        interaction.rebase(origin.map(|v| v as f32));
        interaction.sync_mechanisms(physics);
        for collider in &compound_colliders {
            let local = collider.local_pose(&interaction);
            physics.edit_collider(collider.handle, |collider| {
                collider.set_position_wrt_parent(local)
            });
        }
        interaction.parts.bind(&mut items, physics)?;
        interaction.attachments.bind(&mut items, physics)?;
        interaction.storage.bind(&mut items, physics)?;
        interaction.sync_parts(&mut items, physics)?;
        let starter = reference.survival.starter_food;
        if let Some(food) = items.get(starter).and_then(|item| item.food) {
            let body = items
                .get(starter)
                .and_then(|item| item.body)
                .ok_or_else(|| ReferenceError("Starter food body missing".into()))?;
            for &handle in physics.rapier().bodies[body].colliders().to_vec().iter() {
                let collider_id = (physics.rapier().colliders[handle].user_data >> 64) as u32;
                if let Some(record) = reference
                    .colliders
                    .iter()
                    .find(|entry| entry.id.0 == collider_id)
                {
                    let enabled = record.enabled
                        && survival::active_part(
                            reference,
                            record.node,
                            &reference.survival.starter_parts,
                            food.bite,
                        );
                    physics.edit_collider(handle, |collider| collider.set_enabled(enabled));
                }
            }
        }
        physics.refresh_queries();
        items.sync_poses(physics);
        let spawn = &reference.area.player_spawn;
        let mut player = PlayerState::new(
            &reference.player,
            VehiclePose {
                position: std::array::from_fn(|i| spawn.translation[i] - origin[i] as f32),
                rotation: spawn.rotation,
            },
            physics,
        );
        player.configure_seats(&reference.seats);
        interaction.bind_player_body(player.body_handle());
        let attachments = reference
            .bodies
            .iter()
            .filter_map(|record| {
                let parent = record.parent_body?;
                if reference
                    .mechanisms
                    .iter()
                    .any(|m| m.driven_nodes.contains(&record.node))
                {
                    return None;
                }
                let parent_record = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == parent)
                    .expect("validated parent");
                let source = to_rapier(VehiclePose {
                    position: record.pose.translation,
                    rotation: record.pose.rotation,
                });
                let parent_source = to_rapier(VehiclePose {
                    position: parent_record.pose.translation,
                    rotation: parent_record.pose.rotation,
                });
                Some((
                    bodies[&record.id],
                    bodies[&parent],
                    parent_source.inverse() * source,
                ))
            })
            .collect();
        let render_poses = bodies
            .iter()
            .map(|(&id, &handle)| {
                let pose = crate::vehicle::from_rapier(physics.rapier().bodies[handle].position());
                (id, pose, pose)
            })
            .collect();
        let item_render_poses = items
            .iter()
            .map(|item| (item.id, item.pose, item.pose))
            .collect();
        let portable_mass = reference
            .bodies
            .iter()
            .filter_map(|body| {
                if !body.dynamic || reference.vehicles.iter().any(|v| v.body == body.id) {
                    return None;
                }
                let tanks: Vec<_> = reference
                    .tanks
                    .iter()
                    .filter(|tank| {
                        tank.role == crate::reference::TankRole::Portable
                            && reference
                                .items
                                .iter()
                                .any(|item| item.id == tank.item && item.body == body.id)
                    })
                    .map(|tank| tank.item)
                    .collect();
                if tanks.is_empty() {
                    return None;
                }
                let volume: f32 = tanks
                    .iter()
                    .map(|id| {
                        items
                            .get(*id)
                            .unwrap()
                            .fluid
                            .as_ref()
                            .unwrap()
                            .mixture
                            .total()
                    })
                    .sum();
                Some((bodies[&body.id], body.mass_kg - volume, tanks, body.mass_kg))
            })
            .collect();
        Ok(Self {
            conditions,
            inventory: crate::inventory::InventoryState::new(&reference.inventory),
            inventory_offsets: [None; 4],
            inventory_clutch_conflict: reference.driving.bindings.iter().any(|binding| {
                binding.action == crate::reference::DrivingAction::Clutch
                    && binding.codes.iter().any(|code| code == "Digit1")
            }),
            pose: crate::player::pose::PlayerPoseState::new(reference)?,
            bodies,
            items,
            interaction,
            player,
            origin,
            attachments,
            render_poses,
            item_render_poses,
            plates,
            _plate_draw: plate_draw,
            compound_colliders,
            portable_mass,
        })
    }

    pub fn sync_origin(&mut self, origin: [f64; 3]) {
        let shift = std::array::from_fn(|i| (origin[i] - self.origin[i]) as f32);
        self.player.rebase(shift);
        self.interaction.rebase(shift);
        self.items.rebase(shift);
        for (_, previous, current) in &mut self.render_poses {
            for i in 0..3 {
                previous.position[i] -= shift[i];
                current.position[i] -= shift[i];
            }
        }
        for (_, previous, current) in &mut self.item_render_poses {
            for i in 0..3 {
                previous.position[i] -= shift[i];
                current.position[i] -= shift[i];
            }
        }
        self.origin = origin;
    }

    pub fn begin_step(&mut self) {
        for (_, previous, current) in &mut self.render_poses {
            *previous = *current;
        }
        for (_, previous, current) in &mut self.item_render_poses {
            *previous = *current;
        }
    }
    pub fn sync_portable_mass(&mut self, physics: &mut PhysicsWorld) {
        for (handle, dry, tanks, current) in &mut self.portable_mass {
            if !physics.body_active(*handle) {
                continue;
            }
            let mass = *dry
                + tanks
                    .iter()
                    .map(|id| {
                        self.items
                            .get(*id)
                            .expect("portable tank")
                            .fluid
                            .as_ref()
                            .expect("portable fluid")
                            .mixture
                            .total()
                    })
                    .sum::<f32>();
            if (mass - *current).abs() > 0.0001 {
                physics.set_body_mass(*handle, mass);
                *current = mass;
            }
        }
    }
    pub fn sync_compound(&self, physics: &mut PhysicsWorld) {
        for record in &self.compound_colliders {
            if self
                .interaction
                .attachments
                .owns_collider(crate::reference::ColliderId(
                    (physics.rapier().colliders[record.handle].user_data >> 64) as u32,
                ))
                || self
                    .interaction
                    .storage
                    .owns_collider(crate::reference::ColliderId(
                        (physics.rapier().colliders[record.handle].user_data >> 64) as u32,
                    ))
            {
                continue;
            }
            if physics.rapier().colliders[record.handle]
                .parent()
                .is_some_and(|body| !physics.body_active(body))
            {
                continue;
            }
            if physics.rapier().colliders[record.handle]
                .parent()
                .and_then(|parent| physics.vehicle_of(parent))
                .is_none()
            {
                continue;
            }
            let local = record.local_pose(&self.interaction);
            physics.edit_collider(record.handle, |collider| {
                if collider.position_wrt_parent() != Some(&local) {
                    collider.set_position_wrt_parent(local);
                }
            });
        }
    }
    pub fn commit_poses(&mut self, physics: &PhysicsWorld, teleport: bool) {
        let stored = self.inventory.slots.map(|id| {
            id.and_then(|id| self.items.get(id))
                .filter(|i| {
                    matches!(
                        i.location,
                        crate::interaction::items::ItemLocation::Stored(_)
                    )
                })
                .and_then(|i| i.body)
        });
        for (id, previous, current) in &mut self.render_poses {
            if !physics.body_active(self.bodies[id]) && !stored.contains(&Some(self.bodies[id])) {
                continue;
            }
            *current =
                crate::vehicle::from_rapier(physics.rapier().bodies[self.bodies[id]].position());
            if teleport {
                *previous = *current;
            }
        }
        for (id, previous, current) in &mut self.item_render_poses {
            if self
                .items
                .get(*id)
                .is_some_and(|item| item.inactive_pose.is_some())
            {
                continue;
            }
            *current = self.items.get(*id).expect("bound visual item").pose;
            if teleport {
                *previous = *current;
            }
        }
    }

    /// Attached bodies without mechanisms follow the authoritative vehicle pose.
    pub fn sync_attachments(&self, physics: &mut PhysicsWorld, next: bool) {
        for &(handle, parent, local) in &self.attachments {
            if !physics.body_active(handle) || !physics.body_active(parent) {
                continue;
            }
            let parent_pose = if next {
                *physics.rapier().bodies[parent].next_position()
            } else {
                *physics.rapier().bodies[parent].position()
            };
            physics.edit_body(handle, |body| {
                let pose = parent_pose * local;
                if !next {
                    body.set_position(pose, false);
                }
                body.set_next_kinematic_position(pose);
            });
        }
    }
}

/// Returns the vehicle whose chassis owns the body or one of its parents.
pub fn body_vehicle(reference: &ReferenceManifest, mut id: BodyId) -> Option<usize> {
    loop {
        if let Some(index) = reference.vehicles.iter().position(|v| v.body == id) {
            return Some(index);
        }
        id = reference
            .bodies
            .iter()
            .find(|body| body.id == id)
            .and_then(|body| body.parent_body)?;
    }
}

pub fn collider_item(reference: &ReferenceManifest, mut node: NodeId) -> Option<ItemId> {
    if let Some(seat) = reference
        .seats
        .iter()
        .find(|seat| seat.hit_nodes.contains(&node))
    {
        return Some(seat.item);
    }
    loop {
        if let Some(item) = reference.items.iter().find(|item| item.node == node) {
            return Some(item.id);
        }
        node = reference
            .nodes
            .iter()
            .find(|record| record.id == node)?
            .parent?;
    }
}

pub fn initial_vehicle_poses(
    reference: &ReferenceManifest,
) -> Result<Vec<VehiclePose>, ReferenceError> {
    reference
        .vehicles
        .iter()
        .map(|vehicle| node_pose(reference, vehicle.root))
        .collect()
}

/// Engine and control state of one vehicle.
pub struct VehicleRuntime {
    pub engine: EngineState,
    pub ignition: IgnitionState,
    pub engine_output: EngineOutput,
    pub handbrake: bool,
    /// Distance this vehicle has driven, in meters.
    pub trip_m: f64,
    pub drivetrain: crate::simulation::drivetrain::DrivetrainState,
    recovery_bounds: crate::reference::Bounds,
    /// Absolute position at the start of the current step.
    previous: [f64; 3],
}

impl VehicleRuntime {
    fn new(
        reference: &ReferenceManifest,
        vehicle: &crate::reference::VehicleReference,
    ) -> Result<Self, ReferenceError> {
        let mut recovery_bounds = crate::reference::Bounds {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        };
        for id in &vehicle.colliders {
            let record = reference
                .colliders
                .iter()
                .find(|record| record.id == *id)
                .expect("vehicle shape");
            if record.sensor || !record.enabled {
                continue;
            }
            let aabb = collider_builder(&record.geometry)?.build().compute_aabb();
            for i in 0..3 {
                recovery_bounds.min[i] = recovery_bounds.min[i].min(aabb.mins[i]);
                recovery_bounds.max[i] = recovery_bounds.max[i].max(aabb.maxs[i]);
            }
        }
        Ok(Self {
            engine: EngineState::new(&vehicle.engine),
            engine_output: EngineOutput::default(),
            handbrake: vehicle.handbrake,
            trip_m: 0.0,
            ignition: IgnitionState::default(),
            drivetrain: crate::simulation::drivetrain::DrivetrainState::new(vehicle),
            recovery_bounds,
            previous: [0.0; 3],
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod engine_tests;
mod inventory;
pub use inventory::InventoryFrame;
mod save;

pub struct GarageRuntime {
    pub driving_assists: crate::reference::DrivingAssists,
    pub survival: crate::survival::SurvivalState,
    pub sleep: crate::sleep::SleepState,
    ambient: crate::audio::AmbientState,
    pub urine: crate::interaction::fluid_contact::FluidContact,
    urine_reference: crate::reference::FluidParticleReference,
    urine_emission: [f32; 2],
    sleep_camera: Option<crate::sleep::SleepCamera>,
    pub sleep_camera_allowed: bool,
    pub audio_motion_revision: u32,
    pub simulation_seconds: f64,
    survival_draw: u64,
    previous_speed: Option<f32>,
    death_protected: bool,
    death_announced: bool,
    pub audio_gameplay: crate::reference::AudioGameplayReference,

    pub audio: Option<crate::audio::AudioState>,
    pub persistent_world: crate::save::PersistentWorld,
    pending_restore: Option<save::PendingRestore>,
    pub lighting: crate::lighting::LightingState,
    pub physics: PhysicsWorld,
    pub area: StartingArea,
    pub stream: WorldStream,
    pub vehicles: Vec<VehicleRuntime>,
    pub recovering: bool,
    pub startup_aligned: bool,
    recovery_held_bodies: Vec<RigidBodyHandle>,
    /// The vehicle the player last sat in. The HUD and recovery follow it.
    pub vehicle: usize,
    pub paused: bool,
    pub started: bool,
    pub attempts: u64,
    pub rebases: u32,
    pub notice: &'static str,
    pub aim: ResolvedAim,
    pub wheels: crate::interaction::wheels::WheelAssemblies,
    driving_input: crate::simulation::input::KeyboardDriving,
    recovery_bodies: Vec<RigidBodyHandle>,
    light_owners: Vec<Option<ItemId>>,
}

impl GarageRuntime {
    pub fn set_driving_assists(
        &mut self,
        mut requested: crate::reference::DrivingAssists,
    ) -> Result<(), ReferenceError> {
        if self.started && !self.paused {
            return Err(ReferenceError("Driving assists require pause".into()));
        }
        requested.automatic_gears &= requested.automatic_clutch;
        self.driving_assists = requested;
        self.clear_input();
        Ok(())
    }
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        Self::new_candidate(reference, true, DEFAULT_SEED as i32)
    }
    fn new_candidate(
        reference: &ReferenceManifest,
        stream_world: bool,
        seed: i32,
    ) -> Result<Self, ReferenceError> {
        let mut physics = PhysicsWorld::new(reference, &initial_vehicle_poses(reference)?)?;

        let mut area = StartingArea::new(reference, &mut physics, [0.0; 3])?;
        let mut stream = WorldStream::new(reference, seed)?;
        stream.select_source_car(reference.vehicles[0].spawn_car_id);
        if stream_world {
            stream.request_at(
                reference.area.player_spawn.translation.map(f64::from),
                physics.snapshot(0).pose.position.map(f64::from),
            );
        }
        let mut wheels = crate::interaction::wheels::WheelAssemblies::new(reference)?;
        wheels.refresh_conditions(&area.conditions, &area.items);
        wheels.sync(&mut area.items, &mut physics)?;
        area.interaction
            .cleaning
            .sync_wash_colliders(&area.conditions, &mut physics);
        physics.refresh_queries();
        area.items.sync_poses(&physics);
        area.commit_poses(&physics, true);
        stream.initialize_imported_changes(reference, &area.items, &area.interaction, &physics);
        let lighting = crate::lighting::LightingState::new(&reference.lighting);
        let audio = crate::reference::AudioReference::from_value(&reference.audio, reference)
            .ok()
            .map(|r| {
                let mut state = crate::audio::AudioState::new(r, 1);
                state.bind_game(reference, &area, &physics);
                state.sync_lamps(&reference.lighting, &lighting);
                state
            });
        let urine_visual = reference
            .particles
            .iter()
            .find(|p| p.role == "urine")
            .unwrap();
        let urine_splash = reference
            .particles
            .iter()
            .find(|p| p.role == "urine_splash")
            .unwrap();
        let urine_reference = crate::particles::fluid_reference(urine_visual);
        let urine = crate::interaction::fluid_contact::FluidContact::new(
            &urine_reference,
            urine_visual,
            urine_splash,
            0x70697373,
        );
        Ok(Self {
            driving_assists: reference.driving.default_assists,
            urine,
            urine_reference,
            urine_emission: [0.; 2],
            ambient: crate::audio::AmbientState::new(&reference.ambient),
            survival: crate::survival::SurvivalState::new(&reference.survival),
            sleep: crate::sleep::SleepState::new(&reference.sleep),
            sleep_camera: None,
            sleep_camera_allowed: true,
            audio_motion_revision: 0,
            simulation_seconds: 0.,
            survival_draw: 0,
            previous_speed: None,
            death_protected: true,
            death_announced: false,
            audio_gameplay: crate::reference::AudioGameplayReference::from_value(&reference.audio)?,
            audio,
            persistent_world: crate::save::PersistentWorld::new(reference),
            pending_restore: None,
            physics,
            area,
            stream,
            recovering: false,
            startup_aligned: false,
            lighting,
            vehicles: reference
                .vehicles
                .iter()
                .map(|vehicle| VehicleRuntime::new(reference, vehicle))
                .collect::<Result<_, _>>()?,
            vehicle: 0,
            paused: false,
            started: false,
            attempts: 0,
            rebases: 0,
            notice: "",
            aim: ResolvedAim::default(),
            wheels,
            driving_input: crate::simulation::input::KeyboardDriving::new(
                reference.vehicles[0].keyboard_steering_rate,
                reference.driving.pedal_rate,
            ),
            recovery_bodies: Vec::with_capacity(reference.items.len()),
            recovery_held_bodies: Vec::with_capacity(reference.items.len()),
            light_owners: reference
                .lighting
                .controllers
                .iter()
                .map(|control| {
                    let mut node = Some(control.node);
                    while let Some(id) = node {
                        if let Some(part) = reference.parts.iter().find(|part| part.node == id) {
                            return Some(part.item);
                        }
                        node = reference
                            .nodes
                            .iter()
                            .find(|node| node.id == id)
                            .unwrap()
                            .parent;
                    }
                    None
                })
                .collect(),
        })
    }

    fn use_light_control(
        &mut self,
        reference: &ReferenceManifest,
        control: crate::reference::LightControlId,
    ) {
        match self.stream.use_generated_light(control) {
            Ok(false) => self.lighting.use_control(&reference.lighting, control),
            Ok(true) => {}
            Err(error) => self.stream.fail(error),
        }
        self.area
            .interaction
            .completed_action(InteractionAction::LightControl(control));
        if let Some(audio) = &mut self.audio
            && reference.lighting.controllers.iter().any(|c| {
                c.id == control
                    && c.kind == crate::reference::LightControllerKind::Headlight
                    && c.vehicle == Some(audio.vehicle_index)
            })
        {
            audio.light_switch();
        }
        self.notice = "";
    }

    fn sync_tool_audio(&mut self, reference: &ReferenceManifest) {
        if let Some(audio) = &mut self.audio {
            for (b, s) in reference
                .conditions
                .brushes
                .iter()
                .zip(self.area.interaction.cleaning.signals())
            {
                if let Some(voice) = b.voice {
                    audio.voice(voice, s.playing, s.voice_gain, 1.);
                }
            }
            for (b, s) in reference.conditions.sprays.iter().zip(
                self.area
                    .interaction
                    .cleaning
                    .signals()
                    .iter()
                    .skip(reference.conditions.brushes.len()),
            ) {
                if let Some(voice) = b.voice {
                    audio.voice(voice, s.playing, s.voice_gain, 1.);
                }
            }
        }
    }

    pub fn clock_factors(&self, reference: &ReferenceManifest) -> crate::sleep::ClockFactors {
        self.sleep.clock_factors(&reference.sleep)
    }

    fn advance_ambient(&mut self, reference: &ReferenceManifest, dt: f32) {
        let gains = self
            .ambient
            .gains(&reference.ambient, self.lighting.cycle.output(), dt);
        if let Some(audio) = &mut self.audio {
            let position = self
                .stream
                .origin
                .absolute(self.area.player.body_pose(&self.physics).position);
            audio.source_loop(crate::audio::AudioKind::AmbientDay, 0, gains[0], position);
            audio.source_loop(crate::audio::AudioKind::AmbientNight, 0, gains[1], position);
        }
    }

    fn advance_light_fades(&mut self, reference: &ReferenceManifest, dt: f32) {
        self.stream.advance_generated_lamps(dt);
        for (index, owner) in self.light_owners.iter().enumerate() {
            self.lighting.set_controller_mounted(
                index,
                owner.is_none_or(|item| self.area.items.mounted(item)),
            );
        }
        self.lighting.advance_fades(&reference.lighting, dt);
    }

    pub fn step(&mut self, reference: &ReferenceManifest, mut input: FrameInput) {
        self.wheels
            .refresh_conditions(&self.area.conditions, &self.area.items);
        if let Err(error) = self.sync_world_activity(reference) {
            self.stream.fail(ReferenceError(error.to_string()));
        }
        if !self.started || self.paused || self.recovering || !self.world_supported() {
            self.clear_input();
            #[cfg(target_arch = "wasm32")]
            if crate::app::cpu_profile::enabled() {
                crate::browser::game_cpu_clear_input();
            }
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if crate::app::cpu_profile::enabled() {
            crate::browser::game_cpu_input_consumed();
        }
        let factors = self.clock_factors(reference);
        let sleep = self.sleep.output();
        self.lighting.cycle.advance(
            &reference.lighting.cycle,
            crate::lighting::CycleAdvance {
                simulation_delta: FIXED_DT,
                cycle_extra: FIXED_DT * factors.cycle_extra_per_sim_second,
                cloud_scale: factors.cloud_scale,
                sleep_progress: (sleep.phase == crate::sleep::SleepPhase::Sleeping)
                    .then_some(sleep.progress),
            },
        );
        self.urine_emission = [0.; 2];
        let sleep_held = input.held & keys::SLEEP != 0;
        if self.sleeping() {
            input = FrameInput::default();
        } else {
            self.process_toilet(reference, input, FIXED_DT, factors.fluid_scale);
        }
        let needs = self.needs_output();
        if needs.dead {
            input = FrameInput {
                mouse: input.mouse.map(|v| v * needs.look_scale),
                ..FrameInput::default()
            };
            self.clear_input();
        }
        self.area
            .interaction
            .finish_cancelled_input(&mut self.area.player, &mut self.physics);
        for (index, vehicle) in self.vehicles.iter_mut().enumerate() {
            vehicle.previous = self.persistent_world.absolute_vehicle_position(
                &self.physics,
                &self.stream.origin,
                index,
            );
        }

        self.area.begin_step();
        let aim = if self.sleeping() {
            ResolvedAim::default()
        } else {
            self.area.interaction.resolve(
                &self.physics,
                self.area.player.view_pose(&self.physics),
                &self.area.items,
            )
        };
        if !self.aim.same_actions(&aim) || input.pressed & (keys::GRAB | keys::USE) != 0 {
            self.notice = "";
        }
        self.aim = aim;
        let previous_seat = self.area.player.seat();
        let seated = previous_seat.is_some();
        let outcome = self.area.interaction.step(
            input,
            aim,
            &mut self.area.player,
            &mut self.area.items,
            &mut self.physics,
            FIXED_DT,
            self.survival.output(),
            factors.fluid_scale,
        );
        let action = outcome.action;
        if action.is_none()
            && (input.pressed | input.released) & keys::GRAB != 0
            && let Some(block) = aim.installation_block.or(aim.grab_block)
        {
            self.notice = block.notice();
        }
        match action {
            Some(InteractionAction::Consume(item)) => {
                if self.consume(reference, item).is_err() {
                    self.notice = "Cannot eat this item.";
                }
            }
            Some(InteractionAction::Drink {
                source,
                requested_liters,
            }) => {
                if self.drink(reference, source, requested_liters).is_err() {
                    self.notice = "Cannot drink from this item.";
                }
            }
            Some(InteractionAction::DrinkContact {
                source,
                poured_liters,
                fractions,
            }) => {
                if self
                    .drink_contact(reference, source, poured_liters, fractions)
                    .is_err()
                {
                    self.notice = "Cannot drink this fluid.";
                }
            }
            Some(InteractionAction::LightControl(control)) => {
                self.use_light_control(reference, control)
            }
            Some(InteractionAction::Wheel(action)) => {
                if let Some(notice) = self.area.interaction.complete_wheel_action(
                    action,
                    aim,
                    &mut self.wheels,
                    &self.area.player,
                    &mut self.area.items,
                    &mut self.physics,
                ) {
                    self.notice = notice;
                }
            }
            _ => {}
        }
        while let Some((source, poured_liters, fractions)) =
            self.area.interaction.take_drink_contact()
        {
            if self
                .drink_contact(reference, source, poured_liters, fractions)
                .is_err()
            {
                self.notice = "Cannot drink this fluid.";
            }
        }
        if let Some(action) = self.area.interaction.mechanisms.take_control_action() {
            match action {
                crate::interaction::mechanisms::ControlAction::ToggleHandbrake(lever) => {
                    if let Some(vehicle) = reference
                        .items
                        .iter()
                        .find(|item| item.id == lever)
                        .and_then(|item| body_vehicle(reference, item.body))
                    {
                        self.vehicles[vehicle].handbrake = !self.vehicles[vehicle].handbrake;
                    }
                }
                crate::interaction::mechanisms::ControlAction::Radio(control) => {
                    if let Some(audio) = &mut self.audio {
                        audio.radio_control(control);
                    }
                }
                _ => {}
            }
        }
        for (index, control) in reference.lighting.controllers.iter().enumerate() {
            if control.cabin_input.is_some() {
                let doors = self
                    .area
                    .interaction
                    .cabin_door_values(index, &self.area.items);
                self.lighting
                    .update_cabin(&reference.lighting, index, doors);
            }
        }
        self.area.sync_portable_mass(&mut self.physics);
        if seated
            && input.pressed & keys::USE != 0
            && matches!(action, None | Some(InteractionAction::Seat(_)))
            && self.area.interaction.dragging_mechanism().is_none()
            && self.area.interaction.tuning_opening().is_none()
        {
            self.notice = if self.area.player.seat().is_some() {
                "Look toward a clear exit."
            } else {
                ""
            };
        } else if input.mouse != [0.0; 2] {
            self.notice = "";
        }
        let mut walking_input = input;
        walking_input.mouse = outcome.look_mouse;
        self.area
            .player
            .step(walking_input, &mut self.physics, FIXED_DT);
        self.area.sync_attachments(&mut self.physics, true);
        self.area.sync_compound(&mut self.physics);
        self.area
            .interaction
            .sync_parts(&mut self.area.items, &mut self.physics)
            .expect("bound parts");
        if let Some(vehicle) = self.area.player.seat_vehicle() {
            self.vehicle = vehicle;
            self.stream
                .select_source_car(reference.vehicles[vehicle].spawn_car_id);
        }
        let vehicle_input = if previous_seat != self.area.player.seat() {
            self.clear_driving_input();
            FrameInput::default()
        } else {
            input
        };
        let driving = self.area.player.driving();
        let controls = self.driving_input.step(
            if driving {
                self.area.player.seat()
            } else {
                None
            },
            vehicle_input.held,
        );
        if driving && input.pressed & keys::LIGHTS != 0 {
            if let Some(audio) = &mut self.audio
                && audio.vehicle_index == self.vehicle
            {
                audio.light_switch();
            }
            self.lighting
                .cycle_headlights(&reference.lighting, self.vehicle);
        }
        if (driving || self.area.player.driving2()) && input.pressed & keys::SPACE != 0 {
            let vehicle = &mut self.vehicles[self.vehicle];
            vehicle.handbrake = !vehicle.handbrake;
        }
        for index in 0..self.vehicles.len() {
            self.step_vehicle(
                reference,
                index,
                vehicle_input,
                driving && index == self.vehicle,
                controls,
            );
        }
        self.advance_urine(reference, FIXED_DT);
        self.advance_ambient(reference, FIXED_DT);
        self.physics.step_physics();
        self.sample_survival_speed(reference);
        self.area.interaction.sync_mechanisms(&mut self.physics);
        self.area.sync_attachments(&mut self.physics, false);
        self.area.sync_compound(&mut self.physics);
        self.area
            .interaction
            .sync_parts(&mut self.area.items, &mut self.physics)
            .expect("bound parts");
        self.wheels
            .sync(&mut self.area.items, &mut self.physics)
            .expect("validated wheel bindings");
        self.physics.refresh_queries();
        if self
            .area
            .interaction
            .storage
            .advance(
                &mut self.area.items,
                &mut self.physics,
                &self.area.interaction.parts,
                &self.area.interaction.mechanisms,
                FIXED_DT,
            )
            .expect("bound storage")
        {
            self.wheels
                .sync(&mut self.area.items, &mut self.physics)
                .expect("validated wheel bindings");
        }
        self.area.sync_inventory();
        self.area.sync_equipment(reference, &mut self.physics);
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, false);
        if let Some(audio) = &mut self.audio {
            audio.fixed_step(&self.area, &self.physics, FIXED_DT);
            while let Some(sound) = self.area.interaction.pop_sound() {
                match sound {
                    crate::interaction::InteractionSound::Action(action) => {
                        audio.action(action, audio.game_seconds)
                    }
                    crate::interaction::InteractionSound::Cap {
                        id,
                        previous,
                        current,
                        tune,
                    } => audio.cap(id, previous, current, tune),
                }
            }
            let index = audio.vehicle_index;
            let output = self.vehicles[index].engine_output;
            let engine_active = self
                .area
                .items
                .get(reference.vehicles[index].engine.item)
                .and_then(|item| item.body)
                .is_some_and(|body| self.physics.body_active(body));
            let active = self.physics.body_active(self.physics.chassis_handle(index));
            let car_controls = self.area.player.seat_vehicle() == Some(index)
                && (self.area.player.driving() || self.area.player.driving2());
            audio.vehicle(
                crate::audio::VehicleAudioInput {
                    rpm: output.rpm,
                    ignition: active && self.vehicles[index].ignition.ignition,
                    running: engine_active && output.running,
                    starter: engine_active && output.starter,
                    horn: active && car_controls && input.held & keys::HORN != 0,
                    ambient_temperature: self.lighting.cycle.output().ambient_temperature,
                    electronics_output: audio.reference.radio.electronics_output,
                },
                FIXED_DT,
                audio.game_seconds,
            );
        }
        for (index, vehicle) in self.vehicles.iter_mut().enumerate() {
            let current = self.persistent_world.absolute_vehicle_position(
                &self.physics,
                &self.stream.origin,
                index,
            );
            let previous = vehicle.previous;
            vehicle.trip_m +=
                ((current[0] - previous[0]).powi(2) + (current[2] - previous[2]).powi(2)).sqrt();
        }

        let focus = if self.area.player.seat().is_some() {
            self.physics.snapshot(self.vehicle).pose.position
        } else {
            self.area.player.focus_position(&self.physics)
        };
        let absolute = self.stream.origin.absolute(focus);
        self.stream.request_at(
            absolute,
            self.persistent_world.absolute_vehicle_position(
                &self.physics,
                &self.stream.origin,
                self.vehicle,
            ),
        );
        self.persistent_world
            .sync(
                reference,
                &mut self.area,
                &mut self.wheels,
                &self.stream,
                &mut self.physics,
            )
            .expect("persistent world bindings");
        if self.stream.rebase(focus, &mut self.physics).is_some() {
            let old_origin = self.area.origin;
            self.area.sync_origin(self.stream.origin.offset);
            self.urine
                .rebase(rapier3d::math::Vector::from_array(std::array::from_fn(
                    |i| (self.area.origin[i] - old_origin[i]) as f32,
                )));
            if let Some(audio) = &mut self.audio {
                audio.rebase(std::array::from_fn(|i| {
                    (self.area.origin[i] - old_origin[i]) as f32
                }));
            }
            self.aim.rebase(std::array::from_fn(|i| {
                (self.area.origin[i] - old_origin[i]) as f32
            }));
            self.rebases += 1;
        }
        self.stream.capture_changes(
            reference,
            &mut self.area.items,
            &self.area.interaction,
            &self.physics,
        );
        self.persistent_world
            .sync(
                reference,
                &mut self.area,
                &mut self.wheels,
                &self.stream,
                &mut self.physics,
            )
            .expect("persistent world bindings");
        self.advance_light_fades(reference, FIXED_DT);
        self.advance_needs(reference, FIXED_DT, factors.need_scale);
        let was_sleeping = self.sleeping();
        let output = self.sleep.step(
            &reference.sleep,
            crate::sleep::SleepInput {
                held: sleep_held,
                active: !self.paused,
                camera_allowed: self.sleep_camera_allowed || self.area.player.seat().is_none(),
                dead: self.needs_output().dead,
            },
            self.simulation_seconds,
            FIXED_DT,
        );
        self.sync_sleep_transition(reference, was_sleeping);
        self.lighting.cycle.set_sleep_progress(
            &reference.lighting.cycle,
            (output.phase == crate::sleep::SleepPhase::Sleeping).then_some(output.progress),
        );
    }

    pub fn instrument_sample(
        &self,
        reference: &ReferenceManifest,
        index: usize,
    ) -> crate::simulation::instruments::InstrumentSample {
        let amount = |id: ItemId| {
            self.area
                .items
                .get(id)
                .and_then(|i| i.fluid.as_ref())
                .map_or(0., |f| f.mixture.total())
        };
        let vehicle = &reference.vehicles[index];
        let state = &self.vehicles[index];
        let r = &vehicle.instruments;
        let engine_present = self.area.items.mounted(vehicle.engine.item);
        crate::simulation::instruments::InstrumentSample {
            speed_kmh: self.physics.snapshot(index).speed_mps * 3.6,
            fuel_liters: amount(vehicle.engine.fuel_tank),
            oil_liters: amount(vehicle.engine.oil_tank),
            temperature: if engine_present {
                state.engine_output.temperature
            } else {
                0.
            },
            rpm: if engine_present {
                state.engine_output.rpm
            } else {
                0.
            },
            headlights: self.lighting.headlights(index),
            local_ignition: state.ignition.ignition
                && r.electronics_output > r.electronics_off_threshold,
            distance_m: state.trip_m,
        }
    }

    /// Steps one engine and applies its forces. Only the driven vehicle reads keys.
    fn step_vehicle(
        &mut self,
        reference: &ReferenceManifest,
        index: usize,
        input: FrameInput,
        driving: bool,
        controls: crate::simulation::input::DrivingControls,
    ) {
        let chassis_active = self.physics.body_active(self.physics.chassis_handle(index));
        let engine_present = self
            .area
            .items
            .mounted(reference.vehicles[index].engine.item);
        let engine_active = self
            .area
            .items
            .get(reference.vehicles[index].engine.item)
            .and_then(|item| item.body)
            .is_some_and(|body| self.physics.body_active(body));
        if !chassis_active && !engine_active {
            return;
        }
        let sleep_modifier = self.clock_factors(reference).fluid_scale;
        let source = &reference.vehicles[index];
        let engine_reference = &source.engine;
        let snapshot = self.physics.snapshot(index);
        let speed = snapshot.speed_mps;
        let driving = driving && chassis_active;
        let controls = if driving {
            controls
        } else {
            Default::default()
        };
        let direction = speed
            * snapshot
                .linear_velocity
                .iter()
                .map(|v| v * v)
                .sum::<f32>()
                .sqrt();
        let rolling_rpm = self.physics.driven_wheel_rpm(index);
        let vehicle = &mut self.vehicles[index];
        let drivetrain = vehicle.drivetrain.step(
            controls,
            input.pressed,
            self.driving_assists,
            engine_present.then_some(vehicle.engine_output),
            vehicle.ignition.ignition,
            rolling_rpm,
            direction,
            source,
            &reference.driving,
            driving,
        );
        let gear = drivetrain.gear;
        let throttle = drivetrain.throttle;
        let brake = drivetrain.brake;
        let car_controls = chassis_active
            && (driving
                || (self.area.player.driving2() && self.area.player.seat_vehicle() == Some(index)));
        let ignition_held =
            car_controls && (input.held & keys::IGNITION != 0 || drivetrain.ignition_request);
        let ignition_pressed = (car_controls && input.pressed & keys::IGNITION != 0)
            || (ignition_held && !vehicle.ignition.held());
        let ignition_released = !ignition_held
            && (vehicle.ignition.held() || (car_controls && input.released & keys::IGNITION != 0));
        if ignition_pressed {
            self.attempts = self.attempts.wrapping_add(1);
        }
        let radiator_condition = ((1.0
            - self
                .area
                .conditions
                .item_quality(engine_reference.coolant_tank)
                .unwrap_or(1.0))
            * 4.0)
            .round() as u8;
        let was_ignition = vehicle.ignition.ignition;
        vehicle.ignition.step(
            IgnitionInput {
                held: ignition_held,
                pressed: ignition_pressed,
                released: ignition_released,
            },
            engine_reference.starter_delay_seconds,
            engine_reference.electronics_enabled,
            FIXED_DT,
        );
        let was_running = vehicle.engine_output.running;
        if engine_active {
            vehicle.engine_output = vehicle.engine.step(
                EngineInput {
                    installed: engine_present.then_some(InstalledEngineInput {
                        ignition: vehicle.ignition.ignition,
                        starter: vehicle.ignition.starter,
                        throttle,
                        clutch: drivetrain.engagement,
                        wheel_rpm: drivetrain.wheel_rpm,
                        free_run: drivetrain.free_run,
                        speed_kmh: speed * 3.6,
                        radiator_condition: self
                            .area
                            .items
                            .mounted(engine_reference.coolant_tank)
                            .then_some(radiator_condition),
                    }),
                    ambient_temperature: self.lighting.cycle.output().ambient_temperature,
                    ambient_heat_modifier: engine_reference.globals.outside_temperature_modifier
                        * ((self.lighting.cycle.output().ambient_temperature
                            - reference.lighting.cycle.temp_nights[0])
                            / (reference.lighting.cycle.temp_days[0]
                                + reference.lighting.cycle.temp_add_noons[0]
                                - reference.lighting.cycle.temp_nights[0]))
                            .clamp(0., 1.),
                    sleep_modifier,
                },
                self.area.items.vehicle_fluids(index).expect("bound fluids"),
                engine_reference,
                |draw| start_sample(DEFAULT_SEED, draw),
                FIXED_DT,
            );
            self.area
                .items
                .get_mut(engine_reference.item)
                .expect("engine item")
                .temperature = Some(vehicle.engine.temperature);
        }
        let output = vehicle.engine_output;
        if vehicle.ignition.ignition != was_ignition
            && let Some(audio) = &mut self.audio
            && audio.vehicle_index == index
        {
            audio.ignition_changed(&self.area);
        }
        if chassis_active && engine_present && output.running && !was_running {
            let engine = to_rapier(
                self.area
                    .items
                    .get(engine_reference.item)
                    .expect("engine item")
                    .pose,
            );
            let sequence = self.attempts.wrapping_mul(4);
            let factor = 0.85 + 0.3 * start_sample(DEFAULT_SEED, sequence);
            let vertical = 2.0 * start_sample(DEFAULT_SEED, sequence.wrapping_add(1)) - 1.0;
            let angle =
                std::f32::consts::TAU * start_sample(DEFAULT_SEED, sequence.wrapping_add(2));
            let radius = start_sample(DEFAULT_SEED, sequence.wrapping_add(3)).cbrt();
            let horizontal = (1.0 - vertical * vertical).sqrt();
            let offset = rapier3d::math::Vector::new(
                horizontal * angle.cos(),
                vertical,
                horizontal * angle.sin(),
            ) * radius;
            let impulse = engine.rotation
                * rapier3d::math::Vector::Y
                * engine_reference.starting_push_force
                * factor
                * FIXED_DT;
            let chassis = self.physics.chassis_handle(index);
            self.physics.edit_body(chassis, |body| {
                body.apply_impulse_at_point(impulse, engine.translation + offset, true)
            });
        }
        let pushed = self
            .area
            .interaction
            .push_steering()
            .filter(|(body, _)| self.physics.vehicle_of(*body) == Some(index))
            .map(|(_, steer)| steer);
        let drive = DriveInput {
            throttle,
            brake,
            steer: pushed.unwrap_or(controls.steer),
            handbrake: vehicle.handbrake as u8 as f32,
        };
        let electronics = reference
            .lighting
            .controllers
            .iter()
            .find(|c| {
                c.kind == crate::reference::LightControllerKind::Rear && c.vehicle == Some(index)
            })
            .expect("validated rear lights");
        let electronics_fine = electronics.electronics_source.is_none()
            || electronics.electronics_output > electronics.electronics_threshold;
        self.lighting.update_vehicle(
            &reference.lighting,
            index,
            crate::lighting::VehicleLightSample {
                local_ignition: vehicle.ignition.ignition && electronics_fine,
                brake,
                gear_ratio: source.gear_ratios[gear],
                electronics_output: electronics.electronics_output,
            },
        );
        if chassis_active {
            self.physics.apply_vehicle_forces(
                index,
                drive,
                if engine_present {
                    crate::simulation::drivetrain::transmitted_torques(output, source, gear)
                } else {
                    Default::default()
                },
            );
        }
    }

    pub fn initialize_storage_frame(&mut self, dt: f32) -> Result<bool, ReferenceError> {
        let changed = self.area.interaction.storage.initialize_frame(
            &mut self.area.items,
            &mut self.physics,
            &self.area.interaction.parts,
            &self.area.interaction.mechanisms,
            if self.paused { 0.0 } else { dt },
        )?;
        if changed {
            self.wheels.sync(&mut self.area.items, &mut self.physics)?;
            self.area.items.sync_poses(&self.physics);
            self.area.commit_poses(&self.physics, true);
        }
        Ok(changed)
    }

    pub fn clear_input(&mut self) {
        self.sleep.cancel_charge();
        self.clear_driving_input();
        self.area.interaction.cancel_input();
    }

    fn clear_driving_input(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.starter_enabled = false;
        }
        self.driving_input.clear();
        for vehicle in &mut self.vehicles {
            vehicle.ignition.clear_input();
            vehicle.engine.clear_input();
            vehicle.engine_output.starter = false;
        }
        self.area.interaction.cancel_input();
        self.area.inventory.cancel_input();
    }

    /// Moves the current vehicle, its cargo, and a walking player back to the road.
    pub fn recover(&mut self, reference: &ReferenceManifest) {
        if self.needs_output().dead {
            return;
        }
        self.interrupt_sleep(reference);
        self.audio_motion_revision = self.audio_motion_revision.wrapping_add(1);
        self.previous_speed = None;
        self.death_protected = true;
        let position = if self
            .physics
            .body_active(self.physics.chassis_handle(self.vehicle))
        {
            self.persistent_world.absolute_vehicle_position(
                &self.physics,
                &self.stream.origin,
                self.vehicle,
            )
        } else {
            self.stream
                .origin
                .absolute(self.area.player.focus_position(&self.physics))
        };
        let empty = self
            .area
            .items
            .get(reference.vehicles[self.vehicle].engine.fuel_tank)
            .and_then(|item| item.fluid.as_ref())
            .is_some_and(|fluid| fluid.mixture.total() <= 0.0);
        match self.stream.recovery_target(position, empty) {
            Ok(_) => {
                self.recovering = true;
                self.clear_input();
            }
            Err(error) => self.stream.fail(error),
        }
    }
    pub fn world_supported(&self) -> bool {
        if self.restore_pending()
            || !self.startup_aligned
            || !self.stream.initial_generation_ready()
        {
            return false;
        }
        let supported = |point: [f32; 3], padding: f64| {
            let center = self.stream.origin.absolute(point);
            self.stream.supports(crate::world::WorldBounds {
                min: center.map(|v| v - padding),
                max: center.map(|v| v + padding),
            })
        };
        supported(self.area.player.focus_position(&self.physics), 8.)
            && (0..self.physics.vehicle_count()).all(|index| {
                let handle = self.physics.chassis_handle(index);
                !self.physics.body_active(handle)
                    || supported(
                        self.physics.snapshot(index).pose.position,
                        8. + self.physics.rapier().bodies[handle].linvel().length() as f64
                            * FIXED_DT as f64,
                    )
            })
    }
    /// Call after source publication, before deciding whether the simulation can advance.
    pub fn sync_world_activity(
        &mut self,
        reference: &ReferenceManifest,
    ) -> Result<(), crate::save::SaveError> {
        if self.startup_aligned {
            self.persistent_world.sync(
                reference,
                &mut self.area,
                &mut self.wheels,
                &self.stream,
                &mut self.physics,
            )?;
        }
        Ok(())
    }
    pub fn align_startup(&mut self, reference: &ReferenceManifest) -> Result<bool, ReferenceError> {
        if self.startup_aligned {
            return Ok(true);
        }
        let frame = self.stream.frame();
        let Some(startup) = frame.startup else {
            return Ok(false);
        };
        for (id, offset) in startup.body_offsets(reference) {
            let handle = self.area.bodies[&id];
            self.physics.edit_body(handle, |body| {
                let mut pose = *body.position();
                pose.translation.y += offset;
                body.set_position(pose, true);
            });
        }
        self.area
            .player
            .align_startup(startup.player_offset, &mut self.physics);
        self.physics.refresh_queries();
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, true);
        self.startup_aligned = true;
        Ok(true)
    }
}

fn transport_body(
    physics: &mut PhysicsWorld,
    handle: RigidBodyHandle,
    delta: rapier3d::math::Pose,
) {
    let pose = delta * *physics.rapier().bodies[handle].position();
    physics.edit_body(handle, |body| {
        body.set_position(pose, true);
        body.set_linvel(rapier3d::math::Vector::ZERO, true);
        body.set_angvel(rapier3d::math::Vector::ZERO, true);
    });
}

fn start_sample(seed: u64, attempt: u64) -> f32 {
    (startup_hash(seed ^ attempt) >> 40) as f32 / 16_777_216.0
}

fn startup_hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
