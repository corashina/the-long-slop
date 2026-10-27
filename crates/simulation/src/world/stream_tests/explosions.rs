//! X02 part 4: explosions. The record loads, a generated tank reads its explosion fields, a shot
//! tank explodes and a blast breaks and sets off what is near.
use super::drive_out::settle;
use crate::damage::BreakKind;
use crate::interaction::guns::Strike;
use crate::interaction::items::ItemLocation;
use crate::reference::ItemId;
use crate::simulation::FIXED_DT;
use rapier3d::math::Vector;

#[test]
#[ignore = "requires the installed source export"]
fn explosion_record_loads_and_an_explosive_tank_binds() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    let explosions = &reference.explosions;
    assert!(explosions.types.iter().all(|t| t.explosion_force > 0.));
    assert!(!explosions.explosive_fluids.is_empty());
    let interaction = &run.area.interaction;
    assert!(
        run.area.items.iter().any(|s| {
            s.fluid.as_ref().is_some_and(|f| f.total() > 0.)
                && interaction
                    .tank_explosion(s.id)
                    .is_some_and(|e| !e.never_explode)
        }),
        "an explosive generated tank binds near the start"
    );
}

/// Generated tanks near the start that a blast sets off: (tank, its content root).
fn explosive_tanks(run: &crate::starting_area::GarageRuntime) -> Vec<(ItemId, ItemId)> {
    let interaction = &run.area.interaction;
    let fluids = interaction.explosions.explosive_fluids();
    run.area
        .items
        .iter()
        .filter(|s| !s.removed && s.body.is_some())
        .filter_map(|s| {
            let e = interaction.tank_explosion(s.id)?;
            crate::interaction::explosions::tank_factor(e, s.fluid.as_ref()?, fluids, 1e6)?;
            let root = match s.location {
                ItemLocation::Attached(owner) => owner,
                _ => s.id,
            };
            let location = run.area.items.get(root)?.location;
            (location == ItemLocation::World).then_some((s.id, root))
        })
        .collect()
}

/// Moves the body of `item` to `at` and lets it settle.
fn place(
    run: &mut crate::starting_area::GarageRuntime,
    reference: &crate::reference::ReferenceManifest,
    item: ItemId,
    at: Vector,
) {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics.edit_body(body, |b| {
        b.set_translation(at, true);
        b.set_linvel(Vector::ZERO, true);
    });
    run.physics.refresh_queries();
    settle(run, reference, 0.1);
}

fn position(run: &crate::starting_area::GarageRuntime, item: ItemId) -> Vector {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics.rapier().bodies[body].translation()
}

fn removed(run: &crate::starting_area::GarageRuntime, item: ItemId) -> bool {
    let key = run.area.items.object_key(item);
    run.area.items.get(item).unwrap().removed
        && run
            .stream
            .changes
            .get(&key)
            .is_some_and(|c| c.snapshot.is_none())
}

#[test]
#[ignore = "requires the installed source export"]
fn a_shot_tank_explodes_and_breaks_a_nearby_bottle() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    let (tank, root) = *explosive_tanks(&run)
        .first()
        .expect("an explosive tank near the start");
    let bottle = run
        .stream
        .breakables
        .targets()
        .iter()
        .find_map(|t| match t.kind {
            BreakKind::Item(item)
                if item != root
                    && run.area.items.get(item).is_some_and(|s| {
                        !s.removed && s.location == ItemLocation::World && s.body.is_some()
                    }) =>
            {
                Some(item)
            }
            _ => None,
        })
        .expect("a breakable item near the start");
    let at = position(&run, root) + Vector::new(0.6, 0.3, 0.);
    place(&mut run, &reference, bottle, at);
    run.shot_explosive(Strike {
        item: tank,
        force: 1e6,
        explode: true,
        hole: None,
    });
    run.run_explosions(&reference, FIXED_DT);
    settle(&mut run, &reference, 0.1);
    assert!(
        removed(&run, root),
        "the tank explodes and its removal saves"
    );
    assert!(removed(&run, bottle), "the blast breaks the bottle");
}

#[test]
#[ignore = "requires the installed source export"]
fn a_blast_sets_off_a_second_tank() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    let tanks = explosive_tanks(&run);
    let (first, root) = tanks[0];
    let (_, second) = *tanks
        .iter()
        .find(|(_, r)| *r != root)
        .expect("two explosive tanks near the start");
    let at = position(&run, root) + Vector::new(0.8, 0.3, 0.);
    place(&mut run, &reference, second, at);
    run.shot_explosive(Strike {
        item: first,
        force: 1e6,
        explode: true,
        hole: None,
    });
    run.run_explosions(&reference, FIXED_DT);
    assert!(removed(&run, root));
    assert!(removed(&run, second), "the blast sets off the second tank");
}

#[test]
#[ignore = "requires the installed source export"]
fn a_shot_hole_facing_down_leaks_a_tank() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 1.);
    assert!(
        reference.guns.weapons.iter().any(|w| w.hithole),
        "a hithole gun"
    );
    let interaction = &run.area.interaction;
    let fluids = interaction.explosions.explosive_fluids();
    let tank = run
        .area
        .items
        .iter()
        .find(|s| {
            !s.removed
                && s.location == ItemLocation::World
                && s.body.is_some()
                && s.fluid.as_ref().is_some_and(|f| {
                    f.total() > 0.5 && !crate::interaction::explosions::is_explosive(f, fluids)
                })
                && interaction
                    .tank_explosion(s.id)
                    .is_some_and(|e| !e.no_shoot_hole && !e.always_explode)
        })
        .expect("a non-explosive tank with fluid near the start")
        .id;
    let amount = |run: &crate::starting_area::GarageRuntime| {
        run.area
            .items
            .get(tank)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .total()
    };
    let before = amount(&run);
    run.shot_explosive(Strike {
        item: tank,
        force: 1.,
        explode: false,
        hole: Some([position(&run, tank), -Vector::Y]),
    });
    assert_eq!(run.area.interaction.holes.count(tank), 1);
    settle(&mut run, &reference, 3.);
    assert!(amount(&run) < before, "the fluid leaks through the hole");
}
