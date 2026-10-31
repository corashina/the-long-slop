//! ROADMAP test pass: Player body and outfit (PB rows).
use crate::interaction::items::ItemLocation;
use crate::reference::{ItemId, ReferenceManifest};
use crate::simulation::{FIXED_DT, FrameInput, player_keys};
use crate::starting_area::GarageRuntime;
use crate::world::stream::tests::kit::*;
use rapier3d::math::Vector;

/// Kills the player with the full HP damage, as a hard crash does.
fn kill(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    run.survival
        .damage_instant(&reference.survival, reference.survival.maximum.hp);
    frame(run, reference);
    assert!(run.needs_output().dead);
}

/// One frame with `input`: the inventory frame, as the driver runs it, then one fixed step.
fn input(run: &mut GarageRuntime, reference: &ReferenceManifest, input: FrameInput) {
    let view = run.area.player.view(&run.physics).head_pose;
    let fixed = run
        .advance_inventory_frame(reference, input, FIXED_DT, view)
        .fixed_input;
    frame_with(run, reference, fixed);
}

/// The root name of item prototype `prototype`.
fn root_name(reference: &ReferenceManifest, prototype: usize) -> &str {
    let root = reference.item_prototypes[prototype].root;
    reference
        .nodes
        .iter()
        .find(|n| n.id == root)
        .map_or("", |n| n.name.as_str())
}

fn worn(run: &GarageRuntime) -> Vec<ItemId> {
    (0..crate::interaction::wear::WEAR_SLOTS)
        .filter_map(|slot| run.area.wear.item(slot))
        .collect()
}

/// Source `fpscontroller` `removeSunglassesUp` and `Death2`: X takes off the first edible worn
/// item first, then the first worn slot, into the hand. Save and load keep worn items. Death takes
/// each one off without pickup.
#[test]
#[ignore = "requires the installed source export"]
fn tp_pb13_pb15_x_takes_off_the_cigarette_first_worn_items_save_and_death_drops_each() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    // One wearable per slot, then the cigarette.
    let mut names = Vec::new();
    for slot in 0..crate::interaction::wear::WEAR_SLOTS as u32 {
        let w = reference
            .clothing
            .wearables
            .iter()
            .find(|w| w.type_index == slot && w.lenses.is_empty() == (slot != 0))
            .or_else(|| {
                reference
                    .clothing
                    .wearables
                    .iter()
                    .find(|w| w.type_index == slot)
            })
            .unwrap_or_else(|| panic!("a wearable of type {slot}"));
        let prototype = reference
            .item_prototypes
            .iter()
            .position(|p| p.id == w.prototype)
            .unwrap();
        names.push(root_name(&reference, prototype).to_string());
    }
    let cigarette = root_name(&reference, run.area.cigarettes.cigarette_prototype).to_string();
    let wear = |run: &mut GarageRuntime, name: &str| -> ItemId {
        let id = act(
            run,
            &reference,
            &format!(r#"{{"action":"wear","name":"{name}"}}"#),
        );
        ItemId(id.as_u64().unwrap() as u32)
    };
    let cig = wear(&mut run, &cigarette);
    let cig_slot = run
        .area
        .wear
        .slot(&run.area.interaction, cig)
        .expect("the cigarette has a wear slot");
    for (slot, name) in names.iter().enumerate() {
        if slot != cig_slot {
            wear(&mut run, name);
        }
    }
    assert_eq!(
        worn(&run).len(),
        names.len(),
        "one item per slot: {names:?}"
    );
    let take_off = |run: &mut GarageRuntime| -> Option<ItemId> {
        let before = worn(run);
        input(
            run,
            &reference,
            FrameInput {
                player_released: player_keys::TAKE_OFF,
                ..Default::default()
            },
        );
        let off = before.into_iter().find(|id| !worn(run).contains(id))?;
        assert_eq!(
            run.area.interaction.held(),
            Some(off),
            "the item goes to the hand"
        );
        run.area
            .interaction
            .drop_held(&mut run.area.items, &mut run.physics);
        frame(run, &reference);
        Some(off)
    };
    assert_eq!(
        take_off(&mut run),
        Some(cig),
        "the cigarette comes off first"
    );
    let first = (0..names.len())
        .find_map(|slot| run.area.wear.item(slot))
        .unwrap();
    assert_eq!(take_off(&mut run), Some(first), "then the first worn slot");
    wear(&mut run, &cigarette);
    let count = worn(&run).len();
    let mut loaded = save_load(&run, &reference);
    seconds(&mut loaded, &reference, 0.5);
    assert_eq!(
        worn(&loaded).len(),
        count,
        "worn items stay worn after load"
    );
    // Death: each worn item comes off and lies in the world.
    let on = worn(&loaded);
    kill(&mut loaded, &reference);
    assert!(worn(&loaded).is_empty(), "death takes every item off");
    for id in on {
        let item = loaded.area.items.get(id).unwrap();
        assert_eq!(item.location, ItemLocation::World);
        assert!(loaded.physics.rapier().bodies[item.body.unwrap()].is_dynamic());
    }
}

/// Source `fpscontroller.SpawnRagdol` and `mainscript` `died` frame: the ragdoll takes the joints
/// of the record and the speed of the parent body. The camera follows its head. `DeadV.weight`
/// and `deathSens` run over 14 s, then the menu opens. A loaded save has no ragdoll.
#[test]
#[ignore = "requires the installed source export"]
fn tp_pb9_pb10_pb11_pb12_death_ragdoll_follows_fades_14_s_and_a_load_clears_it() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    assert_eq!(
        reference.survival.death_seconds, 14.,
        "mainscript InverseLerp(0, 14)"
    );
    let saved = save(&run, &reference);
    // Die while the player body moves.
    let body = run.area.player.body_handle();
    let speed = Vector::new(3., 0., 0.);
    run.physics.edit_body(body, |b| b.set_linvel(speed, true));
    kill(&mut run, &reference);
    let ragdoll = &run.area.ragdoll;
    assert!(ragdoll.active(), "the ragdoll spawns");
    let r = &reference.clothing.ragdoll;
    let joints = run
        .physics
        .rapier()
        .impulse_joints
        .iter()
        .filter(|(_, j)| {
            ragdoll.bodies().contains(&j.body1()) || ragdoll.bodies().contains(&j.body2())
        })
        .collect::<Vec<_>>();
    assert_eq!(joints.len(), r.joints.len(), "one joint per record joint");
    assert!(
        joints.iter().all(|(_, j)| !j.data.limit_axes.is_empty()),
        "each joint has limits"
    );
    // Head follow and fade.
    let head = |run: &GarageRuntime| {
        run.area
            .ragdoll
            .head_pose(&run.physics)
            .unwrap()
            .translation
    };
    let th = |run: &GarageRuntime| {
        Vector::from_array(run.area.player.view(&run.physics).head_pose.position)
    };
    seconds(&mut run, &reference, 7.);
    let needs = run.needs_output();
    assert!(
        (th(&run) - head(&run)).length() < 0.05,
        "the camera stays in the head"
    );
    let progress = needs.death_seconds as f32 / 14.;
    assert!(
        (needs.look_scale - 0.8 * (1. - progress)).abs() < 1e-3,
        "deathSens fades"
    );
    assert!(!needs.menu_due);
    seconds(&mut run, &reference, 7.5);
    let needs = run.needs_output();
    assert!(
        needs.menu_due && needs.look_scale == 0.,
        "the menu opens at 14 s"
    );
    let loaded = load(&reference, &saved);
    assert!(
        !loaded.area.ragdoll.active(),
        "a loaded save has no ragdoll"
    );
    assert!(!loaded.needs_output().dead);
}

/// Source `SpawnRagdol`: `ragdolRBs[i].velocity = GetComponentInParent<Rigidbody>().velocity`.
/// Seated, the parent body is the car.
#[test]
#[ignore = "requires the installed source export"]
fn tp_pb9_the_ragdoll_carries_the_speed_of_the_car() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    act(&mut run, &reference, r#"{"action":"seat","seat":"driver"}"#);
    seconds(&mut run, &reference, 0.5);
    let car = run.area.player.seat_body().expect("a seat body");
    let speed = Vector::new(0., 3., 0.);
    run.physics.edit_body(car, |b| b.set_linvel(speed, true));
    run.survival
        .damage_instant(&reference.survival, reference.survival.maximum.hp);
    frame(&mut run, &reference);
    let bodies = run.area.ragdoll.bodies();
    assert!(!bodies.is_empty());
    let car_speed = run.physics.rapier().bodies[car].linvel();
    for b in bodies {
        let v = run.physics.rapier().bodies[*b].linvel();
        assert!(
            (v - car_speed).length() < 0.5,
            "the ragdoll takes the car speed {car_speed:?}: {v:?}"
        );
    }
}
