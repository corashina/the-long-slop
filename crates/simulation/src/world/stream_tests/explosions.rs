//! X02 part 4: explosions. The record loads and a generated tank reads its explosion fields.
use super::drive_out::settle;

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
