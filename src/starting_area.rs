//! Persistent imported home bodies and preparation state.
use crate::interaction::{
    InteractionAction, InteractionState, ResolvedAim,
    items::{ItemStore, node_pose},
};
use crate::player::PlayerState;
use crate::reference::{BodyId, ItemId, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::{
    DriveInput, FIXED_DT, FrameInput,
    engine::{EngineInput, EngineOutput, EngineState},
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
    pub bodies: HashMap<BodyId, RigidBodyHandle>,
    pub items: ItemStore,
    pub interaction: InteractionState,
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
            let handle = physics
                .rapier_mut()
                .insert_body(builder.pose(to_rapier(pose)));
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
            let rapier = physics.rapier_mut();
            let handle =
                rapier
                    .colliders
                    .insert_with_parent(collider, physical_parent, &mut rapier.bodies);
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
        for (_, collider) in physics.rapier_mut().colliders.iter_mut() {
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
        }
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
            let rapier = physics.rapier_mut();
            let handle = rapier.colliders.insert(builder);
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
            physics.rapier_mut().colliders[collider.handle].set_position_wrt_parent(local);
        }
        interaction.parts.bind(&mut items, physics)?;
        interaction.sync_parts(&mut items, physics)?;
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
            bodies,
            items,
            interaction,
            player,
            origin,
            attachments,
            render_poses,
            item_render_poses,
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
                physics.rapier_mut().bodies[*handle].set_additional_mass(mass, true);
                *current = mass;
            }
        }
    }
    pub fn sync_compound(&self, physics: &mut PhysicsWorld) {
        for record in &self.compound_colliders {
            if physics.rapier().colliders[record.handle]
                .parent()
                .and_then(|parent| physics.vehicle_of(parent))
                .is_none()
            {
                continue;
            }
            let local = record.local_pose(&self.interaction);
            let collider = &mut physics.rapier_mut().colliders[record.handle];
            if collider.position_wrt_parent() != Some(&local) {
                collider.set_position_wrt_parent(local);
            }
        }
    }
    pub fn commit_poses(&mut self, physics: &PhysicsWorld, teleport: bool) {
        for (id, previous, current) in &mut self.render_poses {
            *current =
                crate::vehicle::from_rapier(physics.rapier().bodies[self.bodies[id]].position());
            if teleport {
                *previous = *current;
            }
        }
        for (id, previous, current) in &mut self.item_render_poses {
            *current = self.items.get(*id).expect("bound visual item").pose;
            if teleport {
                *previous = *current;
            }
        }
    }

    /// Attached bodies without mechanisms follow the authoritative vehicle pose.
    pub fn sync_attachments(&self, physics: &mut PhysicsWorld, next: bool) {
        for &(handle, parent, local) in &self.attachments {
            let parent_pose = if next {
                *physics.rapier().bodies[parent].next_position()
            } else {
                *physics.rapier().bodies[parent].position()
            };
            let body = &mut physics.rapier_mut().bodies[handle];
            let pose = parent_pose * local;
            if !next {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
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
    pub engine_output: EngineOutput,
    pub handbrake: bool,
    /// Distance this vehicle has driven, in meters.
    pub trip_m: f64,
    ignition_held: bool,
    auto_gear: usize,
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
            ignition_held: false,
            auto_gear: vehicle
                .gear_ratios
                .iter()
                .position(|ratio| *ratio == 0.0)
                .expect("validated neutral"),
            recovery_bounds,
            previous: [0.0; 3],
        })
    }
}

pub struct GarageRuntime {
    pub lighting: crate::lighting::LightingState,
    pub physics: PhysicsWorld,
    pub area: StartingArea,
    pub stream: WorldStream,
    pub vehicles: Vec<VehicleRuntime>,
    /// The vehicle the player last sat in. The HUD and recovery follow it.
    pub vehicle: usize,
    pub paused: bool,
    pub started: bool,
    pub attempts: u64,
    pub rebases: u32,
    pub notice: &'static str,
    pub aim: ResolvedAim,
    pub wheels: crate::interaction::wheels::WheelAssemblies,
    steering: crate::simulation::input::KeyboardSteering,
    recovery_bodies: Vec<RigidBodyHandle>,
    light_owners: Vec<Option<ItemId>>,
}

impl GarageRuntime {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut physics = PhysicsWorld::new(reference, &initial_vehicle_poses(reference)?)?;
        let mut area = StartingArea::new(reference, &mut physics, [0.0; 3])?;
        let mut stream = WorldStream::new(DEFAULT_SEED);
        stream.configure_starting_area(reference);
        stream.sync_at(
            reference.area.player_spawn.translation.map(|v| v as f64),
            &mut physics,
        );
        crate::world::sync_station_supplies(
            &mut stream,
            &mut area.items,
            &mut area.interaction,
            &mut physics,
        );
        let mut wheels = crate::interaction::wheels::WheelAssemblies::new(reference)?;
        wheels.sync(&mut area.items, &mut physics)?;
        physics.refresh_queries();
        area.items.sync_poses(&physics);
        area.commit_poses(&physics, true);
        Ok(Self {
            physics,
            area,
            stream,
            lighting: crate::lighting::LightingState::new(&reference.lighting),
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
            steering: crate::simulation::input::KeyboardSteering::new(
                reference.vehicles[0].keyboard_steering_rate,
            ),
            recovery_bodies: Vec::with_capacity(reference.items.len()),
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

    pub fn advance_lighting(&mut self, reference: &ReferenceManifest, elapsed_seconds: f64) {
        if self.started && !self.paused {
            self.lighting
                .cycle
                .advance(&reference.lighting.cycle, elapsed_seconds);
        }
    }

    pub fn advance_light_fades(&mut self, reference: &ReferenceManifest, elapsed_seconds: f64) {
        if self.started && !self.paused {
            for (index, owner) in self.light_owners.iter().enumerate() {
                self.lighting.set_controller_mounted(
                    index,
                    owner.is_none_or(|item| self.area.items.mounted(item)),
                );
            }
            self.lighting
                .advance_fades(&reference.lighting, elapsed_seconds as f32);
        }
    }

    pub fn step(&mut self, reference: &ReferenceManifest, input: FrameInput) {
        if !self.started || self.paused {
            self.clear_input();
            return;
        }
        self.area
            .interaction
            .finish_cancelled_input(&mut self.area.player, &mut self.physics);
        for (index, vehicle) in self.vehicles.iter_mut().enumerate() {
            vehicle.previous = self
                .stream
                .origin
                .absolute(self.physics.snapshot(index).pose.position);
        }
        self.area.begin_step();
        let aim = self.area.interaction.resolve(
            &self.physics,
            self.area.player.view_pose(&self.physics),
            &self.area.items,
        );
        if !self.aim.same_actions(&aim)
            || input.pressed & (keys::GRAB | keys::USE | keys::ASSEMBLE) != 0
        {
            self.notice = "";
        }
        self.aim = aim;
        let seated = self.area.player.seat().is_some();
        let action = self.area.interaction.step(
            input,
            aim,
            &mut self.area.player,
            &mut self.area.items,
            &mut self.physics,
            FIXED_DT,
        );
        if action.is_none()
            && (input.pressed | input.released) & keys::GRAB != 0
            && let Some(block) = aim.installation_block.or(aim.grab_block)
        {
            self.notice = block.notice();
        }
        match action {
            Some(InteractionAction::LightControl(control)) => {
                self.lighting.use_control(&reference.lighting, control);
                self.notice = "";
            }
            Some(InteractionAction::WheelRemove(item)) => {
                if let Some(slot) = self.wheels.slot_for_item(item) {
                    self.notice =
                        match self
                            .wheels
                            .remove(slot, &mut self.area.items, &mut self.physics)
                        {
                            Ok(rim) => {
                                let point = aim.grab_target.map_or(
                                    self.area.items.get(rim).unwrap().pose.position,
                                    |target| target.point,
                                );
                                match self.area.interaction.pickup_item(
                                    rim,
                                    point,
                                    &self.area.player,
                                    &mut self.area.items,
                                    &mut self.physics,
                                ) {
                                    Ok(()) => "",
                                    Err(_) => "Cannot carry this wheel.",
                                }
                            }
                            Err(_) => "Cannot remove this wheel.",
                        };
                }
            }
            Some(InteractionAction::WheelInstall { item, target }) => {
                if self
                    .wheels
                    .can_install(item, target, &self.area.items)
                    .is_err()
                {
                    self.notice = "This wheel does not fit the slot.";
                } else {
                    self.area
                        .interaction
                        .drop_held(&mut self.area.items, &mut self.physics);
                    self.notice = if self
                        .wheels
                        .install(item, target, &mut self.area.items, &mut self.physics)
                        .is_ok()
                    {
                        if let Some(sensor) = self.wheels.sensor(target) {
                            self.area.interaction.bind_wheel_sensor(
                                sensor,
                                item,
                                &mut self.physics,
                            );
                        }
                        ""
                    } else {
                        "This wheel does not fit the slot."
                    };
                }
            }
            Some(InteractionAction::WheelSplit(rim)) => {
                self.area
                    .interaction
                    .drop_held(&mut self.area.items, &mut self.physics);
                if let Ok((_, tire)) =
                    self.wheels
                        .split(rim, &mut self.area.items, &mut self.physics)
                {
                    self.notice = "";
                    if input.pressed & keys::GRAB != 0 {
                        let point =
                            aim.focus_target.or(aim.grab_target).map_or(
                                self.area.items.get(tire).unwrap().pose.position,
                                |target| target.point,
                            );
                        if self
                            .area
                            .interaction
                            .pickup_item(
                                tire,
                                point,
                                &self.area.player,
                                &mut self.area.items,
                                &mut self.physics,
                            )
                            .is_err()
                        {
                            self.notice = "Cannot carry this tire.";
                        }
                    }
                } else {
                    self.notice = "Cannot dismount this tire.";
                }
            }
            Some(InteractionAction::WheelJoin { rim, tire }) => {
                self.area
                    .interaction
                    .drop_held(&mut self.area.items, &mut self.physics);
                self.notice = if self
                    .wheels
                    .assemble(rim, tire, &mut self.area.items, &mut self.physics)
                    .is_ok()
                {
                    ""
                } else {
                    "This tire does not fit the rim."
                };
            }
            _ => {}
        }
        if let Some(crate::interaction::mechanisms::ControlAction::ToggleHandbrake(lever)) =
            self.area.interaction.mechanisms.take_control_action()
            && let Some(vehicle) = reference
                .items
                .iter()
                .find(|item| item.id == lever)
                .and_then(|item| body_vehicle(reference, item.body))
        {
            self.vehicles[vehicle].handbrake = !self.vehicles[vehicle].handbrake;
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
        if (input.held & keys::USE != 0 && self.area.interaction.dragging_mechanism().is_some())
            || (input.held & keys::USE != 0 && self.area.interaction.tuning_opening().is_some())
            || (input.held & (keys::ROTATE | keys::TRANSLATE) != 0
                && self.area.interaction.held().is_some())
        {
            walking_input.mouse = [0.0; 2];
        }
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
        }
        let driving = self.area.player.driving();
        let steering = self.steering.step(
            if driving {
                self.area.player.seat()
            } else {
                None
            },
            input.held,
        );
        if driving && input.pressed & keys::LIGHTS != 0 {
            self.lighting
                .cycle_headlights(&reference.lighting, self.vehicle);
        }
        if driving && input.pressed & keys::SPACE != 0 {
            let vehicle = &mut self.vehicles[self.vehicle];
            vehicle.handbrake = !vehicle.handbrake;
        }
        for index in 0..self.vehicles.len() {
            self.step_vehicle(
                reference,
                index,
                input,
                driving && index == self.vehicle,
                steering,
            );
        }
        self.physics.step_physics();
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
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, false);
        for (index, vehicle) in self.vehicles.iter_mut().enumerate() {
            let current = self
                .stream
                .origin
                .absolute(self.physics.snapshot(index).pose.position);
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
        if self.stream.rebase(focus, &mut self.physics).is_some() {
            let old_origin = self.area.origin;
            self.area.sync_origin(self.stream.origin.offset);
            self.aim.rebase(std::array::from_fn(|i| {
                (self.area.origin[i] - old_origin[i]) as f32
            }));
            self.rebases += 1;
        }
        self.stream.sync_at(absolute, &mut self.physics);
        crate::world::sync_station_supplies(
            &mut self.stream,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.physics,
        );
    }

    /// Steps one engine and applies its forces. Only the driven vehicle reads keys.
    fn step_vehicle(
        &mut self,
        reference: &ReferenceManifest,
        index: usize,
        input: FrameInput,
        driving: bool,
        steering: f32,
    ) {
        let source = &reference.vehicles[index];
        let engine_reference = &source.engine;
        let snapshot = self.physics.snapshot(index);
        let speed = snapshot.speed_mps;
        let axis = ((driving && input.held & keys::FORWARD != 0) as u8 as f32)
            - ((driving && input.held & keys::BACK != 0) as u8 as f32);
        let direction = speed
            * snapshot
                .linear_velocity
                .iter()
                .map(|v| v * v)
                .sum::<f32>()
                .sqrt();
        let throttle =
            axis != 0.0 && (direction.abs() <= source.auto_move_min || direction * axis > 0.0);
        let vehicle = &mut self.vehicles[index];
        let ignition_held = driving
            && (input.held & keys::IGNITION != 0
                || (!source.dont_ignite_with_throttle_in_automatic
                    && throttle
                    && !vehicle.engine_output.running));
        let ignition_pressed = (driving && input.pressed & keys::IGNITION != 0)
            || (ignition_held && !vehicle.ignition_held);
        let ignition_released = !ignition_held
            && (vehicle.ignition_held || (driving && input.released & keys::IGNITION != 0));
        vehicle.ignition_held = ignition_held;
        if ignition_pressed {
            self.attempts = self.attempts.wrapping_add(1);
        }
        let radius = reference
            .wheel_slots
            .iter()
            .find(|slot| slot.id == source.wheel_slots[0])
            .expect("validated wheel slot")
            .radius;
        let rolling_rpm = self.physics.driven_wheel_rpm(index);
        let gear = crate::vehicle::select_gear(
            rolling_rpm,
            axis < 0.0,
            throttle,
            vehicle.auto_gear,
            engine_reference,
            source,
        );
        if throttle {
            vehicle.auto_gear = gear;
        }
        let ratio = source.gear_ratios[gear].abs() * source.final_drive_ratio;
        let wheel_rpm = rolling_rpm.abs() * ratio;
        let clutch = if ratio == 0.0 {
            0.0
        } else {
            ((vehicle.engine.rpm - engine_reference.auto_shift_clutch_rpm
                + engine_reference.auto_shift_clutch_rpm_range)
                / engine_reference.auto_shift_clutch_rpm_range)
                .clamp(0.0, 1.0)
        };
        let radiator_condition = ((1.0
            - self
                .area
                .items
                .get(engine_reference.coolant_tank)
                .map_or(1.0, |item| item.condition))
            * 4.0)
            .round() as u8;
        let was_running = vehicle.engine_output.running;
        vehicle.engine_output = vehicle.engine.step(
            EngineInput {
                engine_present: self.area.items.mounted(engine_reference.item),
                radiator_present: self.area.items.mounted(engine_reference.coolant_tank),
                ignition_held,
                ignition_pressed,
                ignition_released,
                throttle: throttle as u8 as f32,
                clutch,
                wheel_rpm,
                free_run: source.gear_free_run[gear],
                speed_kmh: speed * 3.6,
                ambient_temperature: engine_reference.initial_temperature,
                ambient_heat_modifier: engine_reference.globals.awake_heat_modifier,
                radiator_condition,
            },
            self.area.items.vehicle_fluids(index).expect("bound fluids"),
            engine_reference,
            start_sample(DEFAULT_SEED, self.attempts),
            FIXED_DT,
        );
        let output = vehicle.engine_output;
        self.area
            .items
            .get_mut(engine_reference.item)
            .expect("engine item")
            .temperature = Some(vehicle.engine.temperature);
        if output.running && !was_running {
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
            self.physics.rapier_mut().bodies[chassis].apply_impulse_at_point(
                impulse,
                engine.translation + offset,
                true,
            );
        }
        let pushed = self
            .area
            .interaction
            .push_steering()
            .filter(|(body, _)| self.physics.vehicle_of(*body) == Some(index))
            .map(|(_, steer)| steer);
        let drive = DriveInput {
            accelerate: driving && input.held & keys::FORWARD != 0,
            brake_reverse: driving && input.held & keys::BACK != 0,
            steer: pushed.unwrap_or(if driving { steering } else { 0.0 }),
            handbrake: self.vehicles[index].handbrake,
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
                local_ignition: output.ignition && electronics_fine,
                brake: crate::simulation::braking(drive, direction, source.auto_move_min) as u8
                    as f32,
                gear_ratio: source.gear_ratios[gear],
                electronics_output: electronics.electronics_output,
            },
        );
        let drive_count = self.physics.driven_wheel_count(index);
        self.physics.set_engine_brake(
            index,
            output.brake_torque_nm * ratio / (drive_count.max(1) as f32 * radius),
        );
        self.physics.apply_vehicle_forces(
            index,
            drive,
            crate::vehicle::drive_force(output, source, gear, radius, drive_count),
        );
    }

    pub fn clear_input(&mut self) {
        self.steering.clear();
        self.area.interaction.cancel_input();
    }

    pub fn restart(&mut self, reference: &ReferenceManifest) -> Result<(), ReferenceError> {
        self.area
            .interaction
            .drop_held(&mut self.area.items, &mut self.physics);
        *self = Self::new(reference)?;
        self.started = true;
        Ok(())
    }

    /// Moves the current vehicle, its cargo, and a walking player back to the road.
    pub fn recover(&mut self, reference: &ReferenceManifest) {
        let index = self.vehicle;
        self.clear_input();
        self.area
            .interaction
            .finish_cancelled_input(&mut self.area.player, &mut self.physics);
        self.aim = ResolvedAim::default();
        let held = self
            .area
            .interaction
            .held()
            .and_then(|item| self.area.items.get(item))
            .and_then(|item| item.body);
        let old_view = to_rapier(self.area.player.view_pose(&self.physics));
        let old_car = to_rapier(self.physics.snapshot(index).pose);
        let inverse = old_car.inverse();
        let bounds = self.vehicles[index].recovery_bounds;
        self.recovery_bodies.clear();
        for item in self.area.items.iter() {
            let Some(handle) = item.body else {
                continue;
            };
            if Some(handle) == held
                || self.physics.vehicle_of(handle).is_some()
                || self.recovery_bodies.contains(&handle)
            {
                continue;
            }
            let body = &self.physics.rapier().bodies[handle];
            if !body.is_dynamic() || !body.is_enabled() {
                continue;
            }
            let point = inverse * body.position().translation;
            if (0..3).all(|i| point[i] >= bounds.min[i] && point[i] <= bounds.max[i]) {
                self.recovery_bodies.push(handle);
            }
        }
        self.area
            .interaction
            .drop_held(&mut self.area.items, &mut self.physics);
        let old = self.physics.snapshot(index).pose;
        let fuel = self
            .area
            .items
            .get(reference.vehicles[index].engine.fuel_tank)
            .and_then(|item| item.fluid.as_ref())
            .map_or(0.0, |fluid| fluid.mixture.total());
        let distance = -self.stream.origin.absolute(old.position)[2];
        let pose = self
            .stream
            .reset_pose(distance, fuel <= 0.0, &mut self.physics);
        crate::world::sync_station_supplies(
            &mut self.stream,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.physics,
        );
        self.physics.reset(index, pose);
        let cargo_delta = to_rapier(pose) * inverse;
        for &body in &self.recovery_bodies {
            transport_body(&mut self.physics, body, cargo_delta);
        }
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
        self.area.items.sync_poses(&self.physics);
        if self.area.player.seat().is_none() {
            let old_player = self.area.player.body_handle();
            let standing = rapier3d::prelude::SharedShape::capsule_y(
                reference.player.standing_height * 0.5 - reference.player.standing_radius,
                reference.player.standing_radius,
            );
            let mut clear = None;
            for distance in [3.0, 4.0, 5.0, 6.0] {
                for side in [-1.0, 1.0] {
                    let mut candidate = to_rapier(pose);
                    candidate.translation += candidate.rotation
                        * rapier3d::math::Vector::new(
                            distance * side,
                            reference.player.standing_height * 0.5 - 1.1 + 0.05,
                            0.0,
                        );
                    if self
                        .physics
                        .queries(
                            rapier3d::prelude::QueryFilter::default()
                                .exclude_sensors()
                                .exclude_rigid_body(old_player),
                        )
                        .intersect_shape(candidate, standing.as_ref())
                        .next()
                        .is_none()
                    {
                        clear = Some(crate::vehicle::from_rapier(&candidate));
                        break;
                    }
                }
                if clear.is_some() {
                    break;
                }
            }
            if let Some(clear) = clear {
                self.physics.rapier_mut().remove_body(old_player);
                self.area.player = PlayerState::new(&reference.player, clear, &mut self.physics);
                self.area.player.configure_seats(&reference.seats);
                self.area
                    .interaction
                    .bind_player_body(self.area.player.body_handle());
            }
        }
        if let Some(body) = held {
            let player_delta =
                to_rapier(self.area.player.view_pose(&self.physics)) * old_view.inverse();
            transport_body(&mut self.physics, body, player_delta);
            self.area
                .interaction
                .sync_parts(&mut self.area.items, &mut self.physics)
                .expect("bound parts");
        }
        self.physics.refresh_queries();
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, true);
    }
}

fn transport_body(
    physics: &mut PhysicsWorld,
    handle: RigidBodyHandle,
    delta: rapier3d::math::Pose,
) {
    let pose = delta * *physics.rapier().bodies[handle].position();
    let body = &mut physics.rapier_mut().bodies[handle];
    body.set_position(pose, true);
    body.set_linvel(rapier3d::math::Vector::ZERO, true);
    body.set_angvel(rapier3d::math::Vector::ZERO, true);
}

fn start_sample(seed: u64, attempt: u64) -> f32 {
    (crate::world::hash(seed ^ attempt) >> 40) as f32 / 16_777_216.0
}
