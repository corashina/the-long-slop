//! X02 part 3: gun shots. A revolver shot breaks a bottle. Shotgun pellets push an item.
use super::drive_out::{frame, settle};
use super::melee::{CROWBAR, place, spawn, weapon};
use super::*;
use crate::damage::BreakKind;
use crate::interaction::items::ItemLocation;
use crate::reference::{GunKind, GunReference, ItemId};
use crate::simulation::{FIXED_DT, FrameInput, keys};
use crate::vehicle::to_rapier;
use rapier3d::math::{Pose, Vector};

/// Frames from one trigger press to the next.
const PRESS_FRAMES: usize = 20;

/// Spawns the gun prototype at the player, puts it in the right hand and fills its ammo.
fn equip(run: &mut GarageRuntime, reference: &ReferenceManifest, gun: &GunReference) -> ItemId {
    let prototype = reference
        .item_prototypes
        .iter()
        .position(|p| p.id == gun.prototype)
        .unwrap();
    let pickup = reference.item_prototypes[prototype]
        .pickups
        .iter()
        .position(|p| p.pickup.source == gun.pickup)
        .unwrap();
    let pose = run.area.player.body_pose(&run.physics);
    let item = crate::starting_area::spawned::spawn_prototype(
        reference,
        &mut run.area,
        &mut run.physics,
        prototype,
        pose,
    )
    .unwrap()[pickup];
    super::g08_pickup(run, item);
    assert!(run.area.equip_carried(&mut run.physics).unwrap());
    settle(run, reference, 1.);
    super::g08_frame(run, reference, FrameInput::default(), FIXED_DT);
    let rt = run
        .area
        .interaction
        .guns
        .runtime_mut(item)
        .expect("gun runtime");
    rt.ammo.count = rt.ammo.maximum.max(1.);
    item
}

/// `Th` in physics space.
fn view(run: &GarageRuntime) -> Pose {
    to_rapier(run.area.player.view(&run.physics).head_pose)
}

/// One frame of repeated trigger presses.
fn trigger_frame(run: &mut GarageRuntime, reference: &ReferenceManifest, index: usize) {
    let input = if index % PRESS_FRAMES == 0 {
        FrameInput {
            held: keys::PRIMARY,
            pressed: keys::PRIMARY,
            ..Default::default()
        }
    } else {
        FrameInput::default()
    };
    super::g08_frame(run, reference, input, FIXED_DT);
}

#[test]
#[ignore = "requires the installed source export"]
fn revolver_shot_breaks_a_bottle() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    let records = &reference.breakables;
    let (target, bottle) = run
        .stream
        .breakables
        .targets()
        .iter()
        .enumerate()
        .filter_map(|(i, t)| match t.kind {
            BreakKind::Item(item) => Some((i, t.record, item)),
            BreakKind::Lamp { .. } => None,
        })
        .filter(|(_, record, item)| {
            let r = &records[*record];
            r.source != reference.acquisition.r#box.breakable
                && !(r.no_destroy || r.no_collider || r.no_shoot)
                && r.health <= 6.
                && run
                    .area
                    .items
                    .get(*item)
                    .is_some_and(|s| s.location == ItemLocation::World && !s.buried)
        })
        .map(|(i, _, item)| (i, item))
        .next()
        .expect("bottle near the start");
    let revolver = reference
        .guns
        .weapons
        .iter()
        .find(|w| matches!(w.kind, GunKind::Revolver { .. }))
        .expect("revolver");
    let item = equip(&mut run, &reference, revolver);
    let pickup = &run.area.interaction.carry.rule(item).unwrap().source;
    assert!(run.area.interaction.guns.weapon_for(pickup).is_some());
    for i in 0..5 * PRESS_FRAMES {
        if run.stream.breakables.targets()[target].destroyed {
            break;
        }
        let view = view(&run);
        place(&mut run, bottle, view * Vector::new(0., 0., -1.5));
        trigger_frame(&mut run, &reference, i);
        frame(&mut run, &reference);
    }
    assert!(
        run.stream.breakables.targets()[target].destroyed,
        "the revolver shot breaks the bottle"
    );
}

#[test]
#[ignore = "requires the installed source export"]
fn shotgun_pellets_push_an_item() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    let shotgun = reference
        .guns
        .weapons
        .iter()
        .find(|w| w.shot_count > 1)
        .expect("shotgun");
    equip(&mut run, &reference, shotgun);
    let target = spawn(&mut run, &reference, weapon(&reference, CROWBAR));
    let body = run.area.items.get(target).unwrap().body.unwrap();
    let mut pushed = false;
    for i in 0..5 * PRESS_FRAMES {
        let view = view(&run);
        if i % PRESS_FRAMES == 0 {
            place(&mut run, target, view * Vector::new(0., 0., -1.));
        }
        trigger_frame(&mut run, &reference, i);
        let forward = view.rotation * -Vector::Z;
        let velocity = run.physics.rapier().bodies[body].linvel();
        if i % PRESS_FRAMES == 0 && velocity.dot(forward) > 0.1 {
            pushed = true;
            break;
        }
        frame(&mut run, &reference);
    }
    assert!(pushed, "the pellets push the item away from the gun");
}
