mod common;

use common::{CAN, SUPPLY};
use my_summer_slop::reference::ConsumptionEffect;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;
use my_summer_slop::survival::ToiletInput;
use my_summer_slop::vehicle::{VehiclePose, to_rapier};

fn drink_fixture() -> my_summer_slop::reference::ReferenceManifest {
    let mut r = common::containers();
    r.survival.initial.water = 1.;
    let supply = common::tank_mut(&mut r, SUPPLY);
    supply.no_drink = true;
    supply.quantities_liters = [0., 0., 0., 100., 0., 0.];
    let opening = r.openings.iter_mut().find(|o| o.tank == SUPPLY).unwrap();
    opening.angle_disabled = true;
    opening.particles.local_pose.translation = [0.; 3];
    opening.particles.local_pose.rotation = [0., 0., 0., 1.];
    r
}

#[test]
fn one_primary_press_across_steps_takes_one_bite() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let id = reference.survival.starter_food;
    let item = runtime.area.items.get(id).unwrap();
    let body = item.body.unwrap();
    let local = item.local_pose;
    let mut position = runtime.area.player.view_pose(&runtime.physics).position;
    position[2] -= 0.7;
    let pose = VehiclePose {
        position,
        rotation: [0., 0., 0., 1.],
    };
    runtime.physics.rapier_mut().bodies[body]
        .set_position(to_rapier(pose) * to_rapier(local).inverse(), true);
    runtime.area.items.sync_poses(&runtime.physics);
    let point = runtime.area.items.get(id).unwrap().pose.position;
    runtime
        .area
        .interaction
        .pickup_item(
            id,
            point,
            &runtime.area.player,
            &mut runtime.area.items,
            &mut runtime.physics,
        )
        .unwrap();
    let available = runtime.area.interaction.available_actions(
        &Default::default(),
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
        runtime.survival.output(),
    );
    assert_ne!(
        available.actions.0 & my_summer_slop::interaction::feedback::ActionMask::CONSUME,
        0
    );
    let mut input = FrameInput::default();
    input.merge(FrameInput {
        held: keys::PRIMARY,
        pressed: keys::PRIMARY,
        ..Default::default()
    });
    for _ in 0..12 {
        runtime.step(&reference, input.take_step());
    }
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 1);
    let fraction = my_summer_slop::simulation::FIXED_DT * reference.survival.delay_speed;
    let expected_pending = 0.5 * (1. - fraction).powi(12);
    assert!((runtime.survival.pending().food - expected_pending).abs() < 1e-6);
    input.merge(FrameInput {
        released: keys::PRIMARY,
        ..Default::default()
    });
    runtime.step(&reference, input.take_step());
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 1);
    input.merge(FrameInput {
        held: keys::PRIMARY,
        pressed: keys::PRIMARY,
        ..Default::default()
    });
    runtime.step(&reference, input.take_step());
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 2);
}

#[test]
fn final_bite_removes_all_bindings() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let id = reference.survival.starter_food;
    let body = runtime.area.items.get(id).unwrap().body.unwrap();
    let colliders = runtime.physics.rapier().bodies[body].colliders().to_vec();
    for _ in 0..6 {
        assert!(runtime.consume(&reference, id).unwrap());
    }
    assert!(runtime.area.items.get(id).unwrap().removed);
    assert!(runtime.physics.rapier().bodies.get(body).is_none());
    for collider in colliders {
        assert!(runtime.physics.rapier().colliders.get(collider).is_none());
    }
    assert_eq!(runtime.survival.pending().food, 3.);
    assert_eq!(runtime.survival.pending().hp, 0.375);
    assert_eq!(runtime.survival.pending().water, -1.);
    assert_eq!(runtime.area.interaction.held(), None);
    assert!(!runtime.consume(&reference, id).unwrap());
}

#[test]
fn full_bowel_rejects_bite_without_mutation() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.survival.queue(ConsumptionEffect {
        bowel: 1.,
        ..Default::default()
    });
    let id = reference.survival.starter_food;
    let before = runtime.survival.pending();
    assert!(!runtime.consume(&reference, id).unwrap());
    assert_eq!(runtime.survival.pending(), before);
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 0);
}

#[test]
fn requested_drink_effect_precedes_actual_debit() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = CAN;
    runtime
        .area
        .items
        .get_mut(source)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0., 0., 0., 0.01, 0., 0.];
    let result = runtime.drink(&reference, source, 0.1).unwrap();
    assert_eq!(result.requested_liters, 0.1);
    assert!((result.debited_liters - 0.01).abs() < 0.000001);
    assert!((result.effect.water - 0.1).abs() < 0.000001);
    assert!((runtime.survival.pending().water - 0.1).abs() < 0.000001);
    assert!(
        runtime
            .area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total()
            <= 0.000001
    );
}

#[test]
fn near_full_bladder_filters_water_and_retains_other_fluid() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = CAN;
    runtime.survival.bladder.mixture.quantities_liters = [0., 0., 0., 1.9, 0., 0.];
    runtime
        .area
        .items
        .get_mut(source)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.1, 0., 0., 0.4, 0.4, 0.1];
    let result = runtime.drink(&reference, source, 1.).unwrap();
    assert!((result.effect.water - 0.4).abs() < 0.000001);
    // Blood acceptance exceeds its remaining source volume; reported debit caps at 0.4 L.
    assert!((result.debited_liters - 0.5).abs() < 0.000001);
    let bladder = &runtime.survival.bladder.mixture;
    assert!(bladder.amount(3) < 1.9);
    assert!(bladder.amount(4) > 0.);
    assert!(bladder.amount(0) > 0.);
    assert!(bladder.total() <= 2.000001);
}

#[test]
fn full_bladder_blocks_effects_and_source_debit() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = CAN;
    runtime.survival.bladder.mixture.quantities_liters = [0., 0., 0., 2., 0., 0.];
    let before = runtime
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture;
    let pending = runtime.survival.pending();
    let result = runtime.drink(&reference, source, 0.1).unwrap();
    assert_eq!(result.debited_liters, 0.);
    assert_eq!(result.effect, ConsumptionEffect::default());
    assert_eq!(
        runtime
            .area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture,
        before
    );
    assert_eq!(runtime.survival.pending(), pending);
}

#[test]
fn no_drink_source_has_no_effect() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let before = runtime.survival.pending();
    assert!(runtime.drink(&reference, SUPPLY, 0.1).is_err());
    assert_eq!(runtime.survival.pending(), before);
}

#[test]
fn particle_contact_does_not_debit_source_twice() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = SUPPLY;
    assert!(runtime.drink(&reference, source, 0.1).is_err());
    let emitted = {
        let fluid = runtime
            .area
            .items
            .get_mut(source)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap();
        my_summer_slop::fluids::spill(fluid, 0.1)
    };
    let before = runtime
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture;
    let result = runtime
        .drink_contact(&reference, source, emitted, [0., 0., 0., 1., 0., 0.])
        .unwrap();
    assert_eq!(result.debited_liters, 0.);
    assert!(result.effect.water > 0.);
    assert_eq!(
        runtime
            .area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture,
        before
    );
}

#[test]
fn open_particle_contact_emits_once_and_reaches_bladder() {
    let mut reference = drink_fixture();
    let opening = reference
        .openings
        .iter_mut()
        .find(|opening| opening.tank == SUPPLY)
        .unwrap();
    opening.valve = 1.;
    let opening = opening.clone();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = SUPPLY;
    let item = reference
        .items
        .iter()
        .find(|item| item.id == source)
        .unwrap();
    let body = reference
        .bodies
        .iter()
        .find(|body| body.id == item.body)
        .unwrap();
    let anchor = reference
        .anchors
        .iter()
        .find(|anchor| anchor.id == opening.anchor)
        .unwrap();
    let from_transform = |transform: &my_summer_slop::reference::Transform| {
        to_rapier(VehiclePose {
            position: transform.translation,
            rotation: transform.rotation,
        })
    };
    let item_pose = my_summer_slop::interaction::items::node_pose(&reference, item.node).unwrap();
    let local = to_rapier(item_pose).inverse()
        * from_transform(&body.pose)
        * from_transform(&anchor.transform);
    let outlet = (to_rapier(runtime.area.items.get(source).unwrap().pose) * local).translation;
    let mouth = runtime.area.player.view_pose(&runtime.physics).position;
    runtime.area.player.rebase([
        mouth[0] - outlet.x,
        mouth[1] - outlet.y,
        mouth[2] - outlet.z,
    ]);
    let before = runtime
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    let action = runtime.area.interaction.step(
        FrameInput::default(),
        Default::default(),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1. / 60.,
        runtime.survival.output(),
        1.,
    );
    let my_summer_slop::interaction::InteractionAction::DrinkContact {
        source: contacted,
        poured_liters,
        fractions,
    } = action.unwrap()
    else {
        panic!("Expected particle contact");
    };
    assert_eq!(contacted, source);
    assert!(poured_liters > 0.);
    let emitted_total = runtime
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!((before - emitted_total - poured_liters).abs() < 0.00001);
    let result = runtime
        .drink_contact(&reference, source, poured_liters, fractions)
        .unwrap();
    assert!(result.effect.water > 0.);
    assert_eq!(
        runtime
            .area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        emitted_total
    );
}

#[test]
fn closed_opening_cannot_create_contact_action() {
    let reference = drink_fixture();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let pending = runtime.survival.pending();
    assert_eq!(
        runtime
            .area
            .interaction
            .opening_valve(my_summer_slop::reference::OpeningId(SUPPLY.0)),
        0.
    );
    runtime.step(&reference, FrameInput::default());
    assert_eq!(runtime.survival.pending(), pending);
}

#[test]
fn defecation_spawns_one_three_group_turd_with_harmful_bites() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let player_position = runtime.area.player.body_pose(&runtime.physics).position;
    let output = runtime.survival.toilet_step(
        &reference.survival,
        ToiletInput {
            burp_pressed: true,
            ..Default::default()
        },
        1. / 60.,
        [0., 1.],
        1.,
        1.,
    );
    assert!(output.defecated);
    let id = runtime
        .apply_toilet_output(&reference, output)
        .unwrap()
        .unwrap();
    assert_eq!(
        runtime.area.items.get(id).unwrap().pose.position,
        player_position
    );
    assert_eq!(
        runtime.area.items.get(id).unwrap().pose.rotation,
        [0., 0., 0., 1.]
    );
    assert_eq!(
        runtime
            .area
            .items
            .get(id)
            .unwrap()
            .food
            .unwrap()
            .prototype_index,
        1
    );
    assert_eq!(reference.survival.foods[1].parts.len(), 3);
    let initial = runtime.survival.pending();
    for _ in 0..3 {
        assert!(runtime.consume(&reference, id).unwrap());
    }
    assert!(runtime.area.items.get(id).unwrap().removed);
    let effect = runtime.survival.pending();
    assert!((effect.food - initial.food - 0.1).abs() < 0.000001);
    assert!((effect.water - initial.water + 1.).abs() < 0.000001);
    assert!((effect.bowel - initial.bowel - 0.2).abs() < 0.000001);
    assert!((effect.hp - initial.hp + 2.).abs() < 0.000001);
    assert!(
        runtime
            .apply_toilet_output(&reference, Default::default())
            .unwrap()
            .is_none()
    );
}
