mod common;
use my_summer_slop::reference::{ConsumptionEffect, FoodKind, NeedValues, ReferenceManifest};
#[test]
fn recovers_active_player_and_source_values() {
    let r = common::reference();
    assert_eq!(
        (
            r.survival.initial.hp,
            r.survival.initial.food,
            r.survival.initial.bowel
        ),
        (5., 0.75, 1.)
    );
    assert!((r.survival.initial.water - 1.8).abs() <= 1e-6);
    assert_eq!(
        r.survival.maximum,
        NeedValues {
            hp: 5.,
            food: 1.,
            water: 3.,
            bowel: 2.
        }
    );
    assert_eq!(r.survival.initial_bladder, [0., 0., 0.25, 0.5, 0., 0.]);
    assert_eq!(r.survival.bladder_capacity_liters, 2.);
    let sausage = r
        .survival
        .foods
        .iter()
        .find(|f| f.kind == FoodKind::Sausage)
        .unwrap();
    assert_eq!(sausage.bites, 6);
    assert_eq!(
        sausage.effect,
        ConsumptionEffect {
            food: 3.,
            water: -1.,
            bowel: 0.,
            hp: 0.375
        }
    );
    let turd = r
        .survival
        .foods
        .iter()
        .find(|f| f.kind == FoodKind::Turd)
        .unwrap();
    assert_eq!(turd.bites, 3);
    assert_eq!(
        turd.effect,
        ConsumptionEffect {
            food: 0.1,
            water: -1.,
            bowel: 0.2,
            hp: -2.
        }
    );
    assert_eq!(
        (
            r.sleep.hold_gain,
            r.sleep.sleep_real_seconds,
            r.sleep.sleep_cycle_seconds,
            r.sleep.cooldown_seconds
        ),
        (0.5, 8., 500., 800.)
    );
    assert_eq!(
        (
            r.weather.min_interval_seconds,
            r.weather.max_interval_seconds,
            r.weather.spawn_radius
        ),
        (1200., 5400., 15000.)
    );
    assert_eq!(r.expected_instances(), 3);
}
#[test]
fn rejects_missing_links_and_nonfinite_rates() {
    let mut r = common::reference();
    r.survival.food_loss = f32::NAN;
    assert!(r.validate().is_err());
    let mut r = common::reference();
    r.survival.foods[0].index = 6;
    assert!(r.validate().is_err());
    let mut r = common::reference();
    r.survival.foods[1].kind = FoodKind::Sausage;
    assert!(r.validate().is_err());
    let mut r = common::reference();
    r.weather.shelters[0].root = my_summer_slop::reference::BodyId(u32::MAX);
    assert!(r.validate().is_err());
    let mut r = common::reference();
    r.weather.particles[0].capacity = 0;
    assert!(r.validate().is_err());
    let mut r = common::reference();
    r.weather.particles[0].materials[0].textures[0].texture = u32::MAX;
    assert!(r.validate().is_err());
    for field in ["survival", "sleep", "weather"] {
        let mut value = serde_json::to_value(common::reference()).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn rejects_rebound_starter_and_shelter_links() {
    let mut r = common::reference();
    let other = r
        .items
        .iter_mut()
        .find(|i| i.id != r.survival.starter_food)
        .unwrap();
    other.kind = my_summer_slop::reference::ItemKind::Food;
    r.survival.starter_food = other.id;
    assert!(r.survival.validate(&r).is_err());

    let mut r = common::reference();
    r.survival.starter_parts[0][0] = r.survival.foods[0].root;
    assert!(r.survival.validate(&r).is_err());

    let mut r = common::reference();
    r.weather.shelters[0].root = r
        .items
        .iter()
        .find(|i| i.id == r.survival.starter_food)
        .unwrap()
        .body;
    assert!(r.weather.validate(&r).is_err());

    let mut r = common::reference();
    r.weather.shelters[0].root = r.weather.shelters[4].root;
    assert!(r.weather.validate(&r).is_err());

    let mut r = common::reference();
    r.weather.shelters[0].source.path_id = 24573;
    assert!(r.weather.validate(&r).is_err());
}
