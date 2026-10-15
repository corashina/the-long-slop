mod common;
use my_summer_slop::reference::ReferenceManifest;
use serde_json::{Value, json};
#[test]
fn checked_acquisition_graphs() {
    let r = common::reference();
    let a = &r.acquisition;
    assert_eq!(
        a.stations
            .iter()
            .map(|s| s.ordinary.len())
            .collect::<Vec<_>>(),
        [33, 37, 236]
    );
    assert_eq!(
        a.stations.iter().map(|s| s.misc4.len()).collect::<Vec<_>>(),
        [18, 6, 38]
    );
    assert_eq!(
        a.stations
            .iter()
            .map(|s| s.underground.len())
            .collect::<Vec<_>>(),
        [1, 1, 1]
    );
    let category = |id| a.categories.iter().find(|c| c.id == id).unwrap();
    assert_eq!(a.categories.len(), 86);
    assert_eq!(
        category(35).children,
        [32, 33, 34, 49, 50, 51, 58, 68, 70, 78, 89]
    );
    assert_eq!(
        category(32).children,
        [74, 172, 217, 218, 220, 245, 305, 306]
    );
    assert_eq!(category(32).weights, [1; 8]);
    assert_eq!(
        category(32).constructor_weights,
        Some(vec![10, 20, 70, 30, 10, 10, 10, 10])
    );
    assert_eq!(category(84).children, [217, 218, 245, 221]);
    assert_eq!(category(84).weights, [1; 4]);
    assert!(category(82).children.contains(&80));
    assert!(category(80).children.contains(&67));
    assert!(category(67).children.contains(&216));
    assert!(
        a.categories
            .iter()
            .filter(|c| !c.categories)
            .flat_map(|c| &c.prototypes)
            .any(Option::is_none)
    );
    let mut invalid = a
        .categories
        .iter()
        .flat_map(|c| c.invalid_children.iter().map(|i| i.target))
        .collect::<Vec<_>>();
    invalid.sort();
    assert_eq!(invalid, [186, 187, 265, 266, 303, 304, 386, 387]);
    assert_eq!(
        a.startup_packs
            .iter()
            .map(|p| p.source.path_id)
            .collect::<Vec<_>>(),
        [14079, 14082]
    );
    assert_eq!(a.r#box.disabled_starter_boxes.len(), 8);
    assert_eq!(a.packs[0].ammo.random_range, [20., 1.]);
    assert_eq!(a.shovel.motion.clips.len(), 6);
}
#[test]
fn rejects_malformed_acquisition_in_strict_and_runtime_paths() {
    let fixture: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    ReferenceManifest::from_runtime_json(include_bytes!("common/reference.json")).unwrap();
    let mut reversed_categories = fixture["acquisition"]["categories"]
        .as_array()
        .unwrap()
        .clone();
    reversed_categories.reverse();
    let mutations = [
        ("/acquisition/holiday_pumpkin/path_id", json!(82.0)),
        (
            "/acquisition/stations/0/ordinary/0/entries/0/selection/prototype/path_id",
            json!(75443.0),
        ),
        (
            "/acquisition/categories/31/prototypes/0/path_id",
            json!(
                fixture["acquisition"]["categories"][31]["prototypes"][0]["path_id"]
                    .as_i64()
                    .unwrap() as f64
            ),
        ),
        (
            "/acquisition/shovel/motion/clip_slots/0/path_id",
            json!(3998.0),
        ),
        (
            "/acquisition/shovel/motion/states/1/callback/path_id",
            json!(130091.0),
        ),
        (
            "/acquisition/stations/0/misc4/0/path_id",
            json!(
                fixture["acquisition"]["stations"][0]["misc4"][0]["path_id"]
                    .as_i64()
                    .unwrap() as f64
            ),
        ),
        ("/acquisition/categories", json!(reversed_categories)),
        ("/acquisition/stations/0/ordinary/0/chance", json!(0.9)),
        (
            "/acquisition/stations/0/ordinary/0/marker/node",
            json!(u32::MAX),
        ),
        (
            "/acquisition/stations/0/ordinary/0/entries/0/selection/source/path_id",
            json!(1),
        ),
        (
            "/acquisition/stations/0/ordinary/0/entries/0/selection/item_id",
            json!(230),
        ),
        (
            "/acquisition/stations/0/ordinary/0/entries/0/selection/prototype",
            json!({"file":"sharedassets1.assets","path_id":32825}),
        ),
        (
            "/acquisition/stations/0/ordinary/0/entries/0/initialization/mix_chance",
            json!(true),
        ),
        (
            "/acquisition/stations/0/underground/0/vertical",
            json!([-0.4, -0.15]),
        ),
        ("/acquisition/stations/0/underground/0/table", json!([])),
        ("/acquisition/stations/0/flyers/supported", json!(true)),
        ("/acquisition/stations/0/flyers/prefab/path_id", json!(1)),
        ("/acquisition/categories/0/children", json!([999])),
        ("/acquisition/box/release", json!([])),
        ("/acquisition/box/clips/0/path_id", json!(1)),
        ("/acquisition/box/content_counts/0/maximum", json!(999)),
        ("/acquisition/box/table", json!([])),
        (
            "/acquisition/box/disabled_starter_boxes/0/disable_spawn",
            json!(false),
        ),
        ("/acquisition/packs/0/ammo/random_range", json!([1., 20.])),
        ("/acquisition/packs/0/take_one_order", json!([])),
        ("/acquisition/startup_packs/0/group/path_id", json!(206)),
        ("/acquisition/holiday_startup/start", json!([10, 14])),
        ("/acquisition/holiday_pumpkin/path_id", json!(8094)),
        ("/acquisition/rules/integer_upper_exclusive", json!(false)),
        ("/acquisition/rules/flows/5/steps/0/lower/value", json!(0.)),
        (
            "/acquisition/rules/flows/5/steps/0/kind",
            json!("integer_range"),
        ),
        (
            "/acquisition/rules/flows/10/steps/2/upper/value",
            json!("category_total"),
        ),
        ("/acquisition/shovel/motion/parameters/0/id", json!(1)),
        (
            "/acquisition/shovel/motion/any_state_transitions/0/destination",
            json!(5),
        ),
        (
            "/acquisition/shovel/motion/states/1/callback/path_id",
            json!(1),
        ),
        ("/acquisition/shovel/motion/clips/0/tracks/0/node", json!(0)),
        (
            "/acquisition/shovel/motion/clips/0/tracks/0/values/0",
            json!([0., 0.]),
        ),
        (
            "/acquisition/shovel/motion/clips/0/tracks/0/coefficients/0/0/0",
            Value::Null,
        ),
        (
            "/acquisition/shovel/motion/clips/0/source_tracks/sha256",
            json!("0".repeat(64)),
        ),
        ("/acquisition/shovel/motion/clips", json!([])),
        ("/acquisition/evidence/methods", json!([])),
    ];
    for (path, value) in mutations {
        let mut broken = fixture.clone();
        *broken.pointer_mut(path).unwrap() = value;
        let bytes = serde_json::to_vec(&broken).unwrap();
        assert!(ReferenceManifest::from_json(&bytes).is_err(), "{path}");
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "{path}"
        );
    }
    for (path, field) in fixture["acquisition"]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| ("/acquisition", k.as_str()))
        .chain([
            ("/acquisition/shovel/motion/states/0", "callback"),
            ("/acquisition/box", "authored_spawn_position"),
            ("/acquisition/packs/0/ammo", "float_or_int"),
            (
                "/acquisition/stations/0/ordinary/0/entries/0/selection",
                "item_id",
            ),
        ])
    {
        let mut broken = fixture.clone();
        broken
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        let bytes = serde_json::to_vec(&broken).unwrap();
        assert!(
            ReferenceManifest::from_json(&bytes).is_err(),
            "missing {path}/{field}"
        );
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "missing {path}/{field}"
        );
    }
    let mut broken = fixture;
    broken["acquisition"]["shovel"]["motion"]["clips"][0]["extra"] = json!(0);
    let bytes = serde_json::to_vec(&broken).unwrap();
    assert!(ReferenceManifest::from_json(&bytes).is_err());
    assert!(ReferenceManifest::from_runtime_json(&bytes).is_err());
}

#[test]
fn rejects_crossed_startup_and_holiday_owners() {
    let r = common::reference();
    for owner in ["save", "body", "pickup", "active", "manual_start", "pose"] {
        let mut changed = r.clone();
        let h = &mut changed.acquisition.holiday_startup;
        match owner {
            "save" => h.save.path_id = 1,
            "body" => h.body.path_id = 1,
            "pickup" => h.pickup.path_id = 1,
            "active" => h.active = false,
            "manual_start" => h.manual_start = false,
            "pose" => h.pose = changed.acquisition.startup_packs[0].pose.clone(),
            _ => unreachable!(),
        }
        assert!(changed.acquisition.validate(&changed).is_err(), "{owner}");
    }
    let mut changed = r.clone();
    changed.acquisition.startup_packs[0].pose = r.acquisition.startup_packs[1].pose.clone();
    changed.acquisition.startup_packs[0].group_pose =
        r.acquisition.startup_packs[1].group_pose.clone();
    assert!(changed.acquisition.validate(&changed).is_err());
}

#[test]
fn rejects_shovel_contact_and_hand_input_changes() {
    let baseline: Value = serde_json::from_str(include_str!("common/reference.json")).unwrap();
    ReferenceManifest::from_json(baseline.to_string().as_bytes()).unwrap();
    for (field, value) in [
        ("force", json!(1)),
        ("hurt_food", json!(1)),
        ("all_particles", json!(0.1)),
    ] {
        let mut broken = baseline.clone();
        broken["acquisition"]["shovel"]["contact"][field] = value;
        assert!(ReferenceManifest::from_json(broken.to_string().as_bytes()).is_err());
        assert!(ReferenceManifest::from_runtime_json(broken.to_string().as_bytes()).is_err());
    }
    for field in ["right_hand_poses", "left_hand_poses"] {
        let mut broken = baseline.clone();
        broken["acquisition"]["shovel"][field] = json!([{"file":"level1","path_id":23118}]);
        assert!(ReferenceManifest::from_json(broken.to_string().as_bytes()).is_err());
        assert!(ReferenceManifest::from_runtime_json(broken.to_string().as_bytes()).is_err());
    }
}
