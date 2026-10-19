use super::super::*;
use super::WheelAction;
use crate::reference::{
    AnchorId, PickupReference, PlayerReference, SourceId, WheelFrictionCurve, WheelHubReference,
    WheelSlotReference,
};
use crate::survival::NeedsOutput;
use items::ItemState;
use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

pub(super) const RIM: ItemId = ItemId(0);
pub(super) const TIRE: ItemId = ItemId(1);
pub(super) const SPARE: ItemId = ItemId(2);
pub(super) const SLOT: WheelSlotId = WheelSlotId(0);
pub(super) const IDENTITY: VehiclePose = VehiclePose {
    position: [0.; 3],
    rotation: [0., 0., 0., 1.],
};

pub(super) struct Fixture {
    pub interaction: InteractionState,
    pub wheels: wheels::WheelAssemblies,
    pub player: PlayerState,
    pub items: ItemStore,
    pub physics: PhysicsWorld,
    pub colliders: [ColliderHandle; 3],
    pub sensor: ColliderHandle,
}
impl Fixture {
    pub fn new(spare_fits: bool, mounted: bool, pickup: bool) -> Self {
        let reference = player_reference();
        let mut physics = PhysicsWorld::wheel_actions_fixture(slot_reference(), 100.);
        let rim_body = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .translation(Vector::new(1., -0.5, 0.))
                .additional_mass(12.),
        );
        let spare_body = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .translation(Vector::new(3., -0.5, 0.))
                .additional_mass(3.),
        );
        let colliders = std::array::from_fn(|i| {
            let body = if i == 2 { spare_body } else { rim_body };
            physics.insert_collider_with_parent(
                ColliderBuilder::ball(0.2)
                    .density(0.)
                    .user_data(((i as u128 + 1) << 64) | (i as u128 + 1)),
                body,
            )
        });
        physics.recompute_body_mass(rim_body);
        physics.recompute_body_mass(spare_body);
        let sensor = physics.insert_collider(
            ColliderBuilder::ball(0.3)
                .sensor(true)
                .user_data(SLOT_SENSOR_FLAG | ((SLOT.0 as u128) << 96) | 1),
        );
        let states = [RIM, TIRE, SPARE]
            .into_iter()
            .map(|id| {
                let body = if id == SPARE { spare_body } else { rim_body };
                ItemState {
                    id,
                    body: Some(body),
                    inactive_pose: None,
                    location: if id == TIRE {
                        ItemLocation::Attached(RIM)
                    } else {
                        ItemLocation::World
                    },
                    pose: crate::vehicle::from_rapier(physics.rapier().bodies[body].position()),
                    local_pose: IDENTITY,
                    generated_condition: None,
                    temperature: None,
                    fluid: None,
                    food: None,
                    removed: false,
                    pending_attachment: false,
                }
            })
            .collect();
        let items = ItemStore::wheel_actions_fixture(states);
        let player = PlayerState::new(
            &reference,
            VehiclePose {
                position: [0., 3., 3.],
                ..IDENTITY
            },
            &mut physics,
        );
        let rules = [RIM, TIRE, SPARE].map(|id| (id, pickup_reference(!pickup)));
        let mut interaction = InteractionState {
            cleaning: cleaning::CleaningTools::empty_for_wheel_actions(),
            sounds: [None; 64],
            sound_count: 0,
            next_generated_collider: 4,
            next_generated_light: 1,
            light_controls: HashMap::new(),
            held_controls: HashMap::new(),
            storage: storage::StorageLocks::empty_for_wheel_actions(),
            light_nodes: HashMap::new(),
            cabin_doors: Vec::new(),
            seat_action_latched: false,
            usable_light_controls: HashSet::new(),
            fill_opening: None,
            kinds: HashMap::from([
                (RIM, ItemKind::Rim),
                (TIRE, ItemKind::Tire),
                (SPARE, ItemKind::Rim),
            ]),
            slot_fits: HashMap::from([(SLOT, HashSet::from([RIM, SPARE]))]),
            tire_fits: HashSet::from([(RIM, TIRE)]),
            f_gesture_consumed: false,
            source_bindings: HashMap::new(),
            pickup_layers: HashMap::from([(RIM, 1), (TIRE, 1), (SPARE, 1)]),
            removal_layers: HashMap::from([(RIM, 1), (TIRE, 1), (SPARE, 1)]),
            usable_items: HashSet::new(),
            filling_items: HashSet::new(),
            collider_bindings: HashMap::new(),
            bodies: HashMap::new(),
            mechanisms: mechanisms::MechanismState::empty_for_wheel_actions(),
            parts: parts::PartAttachments::empty_for_wheel_actions(),
            attachments: attachments::PlateAttachments::empty_for_wheel_actions(),
            mechanism_bodies: HashMap::new(),
            openings: Vec::new(),
            seats: Vec::new(),
            carry: carry::CarryState::wheel_actions_fixture(reference.clone(), &rules),
            equipment: equipment::EquipmentState::empty_for_wheel_actions(),
            push: push::PushState::wheel_actions_fixture(reference.clone()),
            player_body: Some(player.body_handle()),
            reach: reference.aim_reach,
            action_masks: reference.action_masks,
            scroll_sensitivity: reference.scroll_sensitivity,
            supplies: HashSet::new(),
            source_supply_emission: HashMap::new(),
            tank_rules: HashMap::new(),
            slow_oil_modifier: 1.,
            drink_flow_liters_per_second: 1.,
            drink_contact_radius: 0.1,
            drink_collider_radius: 0.1,
            drink_collider_center: Vector::ZERO,
            pending_drink_contacts: Vec::new(),
            latched_mechanism: None,
            use_consumed: false,
            mechanism_release_distance_squared: 1.,
            tuning_opening: None,
        };
        interaction.bind_wheel_sensor(sensor, RIM, &mut physics);
        let wheels =
            wheels::WheelAssemblies::wheel_actions_fixture(RIM, TIRE, SPARE, SLOT, spare_fits);
        let mut fixture = Self {
            interaction,
            wheels,
            player,
            items,
            physics,
            colliders,
            sensor,
        };
        if mounted {
            fixture
                .wheels
                .install(RIM, SLOT, &mut fixture.items, &mut fixture.physics)
                .unwrap();
        }
        fixture
    }
    pub fn hold(&mut self, item: ItemId) {
        let point = self.items.get(item).unwrap().pose.position;
        self.interaction
            .pickup_item(
                item,
                point,
                &self.player,
                &mut self.items,
                &mut self.physics,
            )
            .unwrap();
    }
    pub fn complete(
        &mut self,
        action: WheelAction,
        input: FrameInput,
        aim: ResolvedAim,
    ) -> Option<&'static str> {
        self.interaction.complete_wheel_action(
            action,
            input,
            aim,
            &mut self.wheels,
            &self.player,
            &mut self.items,
            &mut self.physics,
        )
    }
    pub fn action(&mut self, action: WheelAction) -> Option<&'static str> {
        self.complete(action, FrameInput::default(), ResolvedAim::default())
    }
    pub fn body(&self, item: ItemId) -> RigidBodyHandle {
        self.items.get(item).unwrap().body.unwrap()
    }
    pub fn target(&self, item: ItemId, point: [f32; 3]) -> AimTarget {
        AimTarget {
            item,
            point,
            collider: self.colliders[item.0 as usize],
            distance: 1.,
            slot: None,
        }
    }
    pub fn event(&mut self) -> InteractionAction {
        let Some(InteractionSound::Action(action)) = self.interaction.pop_sound() else {
            panic!("missing completion")
        };
        action
    }
    pub fn no_event(&mut self) {
        assert!(self.interaction.pop_sound().is_none());
    }
    pub fn step(&mut self, input: FrameInput, aim: ResolvedAim) -> Option<InteractionAction> {
        self.interaction.step(
            input,
            aim,
            &mut self.player,
            &mut self.items,
            &mut self.physics,
            crate::simulation::FIXED_DT,
            NeedsOutput::default(),
            1.,
        )
    }
}
fn source() -> SourceId {
    SourceId {
        file: "wheel fixture".into(),
        path_id: 1,
    }
}
fn pickup_reference(cant_pickup: bool) -> PickupReference {
    PickupReference {
        source: source(),
        cant_pickup,
        cant_pickup_while_mounted: false,
        cant_pickup_while_seated: false,
        no_rotation: false,
        move_with_rotation: false,
        use_recenter_position: false,
        recenter_position: [0.; 3],
        use_recenter_rotation: false,
        recenter_rotation: [0., 0., 0., 1.],
        recenter_distance: 1.,
        can_inventory: false,
        can_hold_right_hand: false,
        disable_rh_fix: false,
        right_hand_position: [0.; 3],
        right_hand_rotation: [0., 0., 0., 1.],
        right_hand_grip: None,
        left_hand_grip: None,
        disable_when_stored: None,
        keep_when_stored: None,
        inventory_icon: None,
    }
}
fn player_reference() -> PlayerReference {
    PlayerReference {
        source: source(),
        capsule_source: source(),
        camera_source: source(),
        walk_speed: 3.,
        run_multiplier: 2.,
        jump_force: 3.,
        mass_kg: 70.,
        duck_multiplier: 0.5,
        ctrl_duck_value: 0.5,
        duck_lerp: 1.,
        head_lerp: 1.,
        crouch_height: 1.,
        capsule_radius: 0.3,
        capsule_height: 1.8,
        camera_height: 0.7,
        field_of_view_degrees: 60.,
        aim_reach: 4.,
        action_masks: ActionMasks {
            use_mask: 1,
            pickup_mask: 1,
            attach_mask: 1,
            push_mask: 1,
        },
        get_out_layer_mask: 1,
        exit_capsule_half_height: 0.5,
        exit_capsule_radius: 0.3,
        standing_radius: 0.3,
        standing_height: 1.8,
        crouch_radius: 0.3,
        exit_distance: 1.,
        exit_step_vertical: 0.1,
        exit_steps_vertical: 1,
        exit_step_horizontal: 0.1,
        exit_steps_horizontal: 1,
        source_force_interval: 0.02,
        throw_minimum: 1.,
        throw_gain: 1.,
        throw_multiplier: 1.,
        push_force: 1.,
        airborne_push_force: 1.,
        grip_force: 1.,
        grip_speed: 1.,
        grip_distance: 1.,
        carry_min_weight: 1.,
        carry_max_weight: 50.,
        carry_anchor_offset: [0., 0., -1.],
        carry_offset_radius: 2.,
        carry_drop_distance: 5.,
        carry_force_multiplier: 1.,
        carry_max_force: 100.,
        carry_min_distance: 0.1,
        carry_max_distance: 3.,
        obj_move_sensitivity: 1.,
        obj_rotation_sensitivity: 1.,
        look_sensitivity: [1.; 2],
        scroll_sensitivity: 1.,
        camera_culling_mask: 1,
    }
}
fn slot_reference() -> WheelSlotReference {
    let friction = WheelFrictionCurve {
        extremum_slip: 1.,
        extremum_value: 1.,
        asymptote_slip: 2.,
        asymptote_value: 0.5,
        stiffness: 1.,
    };
    WheelSlotReference {
        id: SLOT,
        source: source(),
        anchor: AnchorId(0),
        mount_anchor: AnchorId(1),
        wheel: RIM,
        tire: TIRE,
        rim: RIM,
        radius: 0.3,
        suspension_distance: 1.,
        center: [1., 0., 0.],
        spring: 100.,
        damper: 1.,
        target_position: 0.5,
        steer: false,
        drive: false,
        brake: true,
        handbrake: true,
        forward_friction: friction,
        sideways_friction: friction,
        wheel_mass_kg: 12.,
        rotational_damping: 1.,
        force_application_distance_m: 0.1,
        sideways_stiffness_modifier: 1.,
        hub: WheelHubReference {
            radius_m: 0.1,
            mass_kg: 1.,
            forward_stiffness: 1.,
            sideways_stiffness: 1.,
        },
        tire_condition_range: [0, 100],
        rim_condition_range: [0, 100],
        rim_main_types: Vec::new(),
        rim_types: vec!["fixture".into()],
    }
}
