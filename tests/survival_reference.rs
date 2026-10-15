mod common;
use my_summer_slop::reference::{FoodKind, ReferenceManifest};
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
