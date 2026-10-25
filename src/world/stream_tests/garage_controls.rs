use crate::{
    interaction::feedback::ActionMask,
    player::PlayerState,
    reference::{ItemId, ReferenceManifest, TurnAction},
    simulation::{FrameInput, keys},
    starting_area::GarageRuntime,
    vehicle::VehiclePose,
};
use rapier3d::math::{Rotation, Vector};

#[path = "garage_controls/reactions.rs"]
mod reactions;

#[path = "garage_controls/instruments.rs"]
mod instruments;

#[path = "garage_controls/radio.rs"]
mod radio;

#[path = "garage_controls/restore.rs"]
mod restore;

#[path = "garage_controls/acceptance.rs"]
mod acceptance;

pub(super) fn load_garage() -> (ReferenceManifest, GarageRuntime) {
    let reference = super::reference();
    assert_eq!(reference.build_id, 11140890);
    assert_eq!(reference.vehicles.len(), 1);
    let mut run = GarageRuntime::new(&reference, crate::world::GameMode::default()).unwrap();
    super::prepare_ready(&mut run, &reference);
    assert!(run.world_supported());
    run.started = true;
    run.paused = false;
    (reference, run)
}

fn press(run: &mut GarageRuntime, reference: &ReferenceManifest, held: u32) {
    run.step(
        reference,
        FrameInput {
            held,
            pressed: held,
            ..FrameInput::default()
        },
    );
}

fn aim_handbrake(reference: &ReferenceManifest, run: &mut GarageRuntime) -> ItemId {
    let item = reference
        .mechanisms
        .iter()
        .find(|m| {
            m.turn
                .as_ref()
                .is_some_and(|t| matches!(t.action, TurnAction::ToggleHandbrake))
        })
        .unwrap()
        .item;
    let collider = reference
        .action_bindings
        .iter()
        .find(|binding| binding.use_owner == Some(item))
        .unwrap()
        .collider;
    let center = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == collider.0)
        .unwrap()
        .1
        .compute_aabb()
        .center();
    let view = [Vector::X, -Vector::X, Vector::Z, -Vector::Z]
        .into_iter()
        .flat_map(|axis| [0.15, 0.3, 0.6].map(move |distance| (axis, distance)))
        .map(|(axis, distance)| VehiclePose {
            position: (center + axis * distance).to_array(),
            rotation: Rotation::from_rotation_arc(-Vector::Z, -axis).to_array(),
        })
        .find(|view| {
            run.area
                .interaction
                .resolve(&run.physics, *view, &run.area.items)
                .use_target
                .is_some_and(|target| target.item == item)
        })
        .expect("source handbrake must resolve through its collider binding");
    let previous = run.area.player.body_handle();
    run.physics
        .edit_body(previous, |body| body.set_enabled(false));
    let view_rotation = Rotation::from_array(view.rotation);
    let head_offset =
        view_rotation * Vector::from_array(reference.player.posture.standing_head.translation);
    run.area.player = PlayerState::new(
        &reference.player,
        &reference.clothing.pose,
        VehiclePose {
            position: (Vector::from_array(view.position) - head_offset).to_array(),
            rotation: view_rotation.to_array(),
        },
        &mut run.physics,
    );
    run.area
        .interaction
        .bind_player_body(run.area.player.body_handle());
    let aim = run.area.interaction.resolve(
        &run.physics,
        run.area.player.view(&run.physics).camera_pose,
        &run.area.items,
    );
    assert_eq!(aim.use_target.unwrap().item, item);
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::SWITCH, 0);
    assert!(available.lmb_use);
    item
}

#[test]
fn g10_handbrake_press_priority() {
    let (reference, mut run) = load_garage();
    let lever = aim_handbrake(&reference, &mut run);
    let initial_handbrake = run.vehicles[0].handbrake;
    press(&mut run, &reference, keys::USE);
    assert_eq!(run.aim.use_target.unwrap().item, lever);
    let after_e_press = run.vehicles[0].handbrake;
    assert_eq!(after_e_press, !initial_handbrake);
    run.step(
        &reference,
        FrameInput {
            held: keys::USE,
            ..FrameInput::default()
        },
    );
    assert_eq!(run.vehicles[0].handbrake, after_e_press);
    run.step(
        &reference,
        FrameInput {
            released: keys::USE,
            ..FrameInput::default()
        },
    );
    assert_eq!(run.vehicles[0].handbrake, after_e_press);

    let (reference, mut run) = load_garage();
    aim_handbrake(&reference, &mut run);
    press(&mut run, &reference, keys::PRIMARY);
    let after_lmb_press = run.vehicles[0].handbrake;
    assert_eq!(after_lmb_press, !initial_handbrake);
    run.step(
        &reference,
        FrameInput {
            held: keys::PRIMARY,
            ..FrameInput::default()
        },
    );
    let after_held_frame = run.vehicles[0].handbrake;
    assert_eq!(after_held_frame, after_lmb_press);
    run.step(
        &reference,
        FrameInput {
            released: keys::PRIMARY,
            ..FrameInput::default()
        },
    );
    assert_eq!(run.vehicles[0].handbrake, after_lmb_press);

    let (reference, mut run) = load_garage();
    aim_handbrake(&reference, &mut run);
    press(&mut run, &reference, keys::USE | keys::PRIMARY);
    let after_combined_press = run.vehicles[0].handbrake;
    assert_eq!(after_combined_press, !initial_handbrake);

    let (reference, mut run) = load_garage();
    aim_handbrake(&reference, &mut run);
    let tool = reference.conditions.brushes[0].item;
    let point = run.area.items.get(tool).unwrap().pose.position;
    run.area
        .interaction
        .pickup_item(
            tool,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    assert_eq!(run.area.interaction.held(), Some(tool));
    assert!(run.area.equip_carried(&mut run.physics).unwrap());
    assert_eq!(run.area.interaction.equipment.item(), Some(tool));
    assert_eq!(run.area.interaction.held(), None);
    assert!(run.area.interaction.cleaning.reserves_lmb(
        run.area.interaction.equipment.item(),
        run.area.interaction.held(),
    ));
    press(&mut run, &reference, keys::PRIMARY);
    let after_reserved_lmb = run.vehicles[0].handbrake;
    assert_eq!(after_reserved_lmb, initial_handbrake);
}

#[test]
fn g10_light_shortcut_seat_roles() {
    let headlights = [0, 1, 2].map(|role| {
        let (reference, mut run) = load_garage();
        let seat = reference
            .seats
            .iter()
            .find(|seat| match role {
                0 => seat.driver,
                1 => !seat.driver && seat.driving2,
                _ => !seat.driver && !seat.driving2,
            })
            .unwrap();
        run.area.player.configure_seats(&reference.seats);
        let anchor = reference
            .anchors
            .iter()
            .find(|anchor| anchor.id == seat.anchor)
            .unwrap();
        assert!(run.area.player.enter(
            seat.id,
            VehiclePose {
                position: anchor.transform.translation,
                rotation: anchor.transform.rotation,
            },
            &mut run.physics,
        ));
        assert_eq!(run.area.player.driving(), seat.driver);
        assert_eq!(run.area.player.driving2(), seat.driving2);
        press(&mut run, &reference, keys::LIGHTS);
        run.lighting.headlights(seat.vehicle)
    });
    let [
        driver_headlights,
        secondary_control_headlights,
        ordinary_passenger_headlights,
    ] = headlights;
    assert_eq!(driver_headlights, 1);
    assert_eq!(secondary_control_headlights, 1);
    assert_eq!(ordinary_passenger_headlights, 0);
}
