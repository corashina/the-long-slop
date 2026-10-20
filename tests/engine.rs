//! Engine save boundaries checked against the installed build 11140890 export.
use my_summer_slop::{
    fluids::{FluidContainer, FluidMixture, transfer},
    reference::ReferenceManifest,
    save::{
        CameraSave, FluidSnapshot, IdentityMap, SaveError, SaveSnapshot, SourceHash, SourceStamp,
    },
    starting_area::GarageRuntime,
};

fn installed_source() -> (ReferenceManifest, SourceStamp) {
    let path = "web/assets/reference/manifest.json";
    let bytes = std::fs::read(path).unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(reference.build_id, 11140890);
    assert_eq!(reference.vehicles.len(), 1);
    assert_eq!(reference.vehicles[0].engine.source.file, "level1");
    assert_eq!(reference.vehicles[0].engine.source.path_id, 68282);
    let digest = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|hash| SourceHash {
                file: hash.file.clone(),
                sha256: hash.sha256.clone(),
            })
            .collect(),
    };
    (reference, stamp)
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn fluid_save_order_and_validation() {
    let (reference, stamp) = installed_source();
    let mut game = GarageRuntime::new(&reference).unwrap();
    let fuel_id = reference.vehicles[0].engine.fuel_tank;
    let container = game
        .area
        .items
        .get_mut(fuel_id)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap();
    *container = FluidContainer::new(container.capacity_liters, FluidMixture::default());
    for (index, liters) in [(3, 3.), (0, 2.)] {
        let mut quantities_liters = [0.; 6];
        quantities_liters[index] = liters;
        let mut source = FluidContainer::new(liters, FluidMixture { quantities_liters });
        assert_eq!(transfer(&mut source, container, liters), liters);
    }
    let saved = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let ids = IdentityMap::new(&reference).unwrap();
    let fuel = saved
        .items
        .iter()
        .position(|item| ids.item(&item.instance).unwrap() == fuel_id)
        .unwrap();
    let fluids = saved.items[fuel].fluids.as_ref().unwrap();
    assert_eq!(fluids.order, [3, 0]);
    assert_eq!(fluids.quantities_liters, [2., 0., 0., 3., 0., 0.]);
    let decoded = SaveSnapshot::from_json(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let restored = GarageRuntime::from_save(&reference, &stamp, &decoded).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
        .area
        .items
        .capture_save(
            &ids,
            &restored.stream.origin,
            &restored.physics,
            &restored.area.interaction,
            &reference,
        )
        .unwrap();
    assert_eq!(recaptured[fuel].fluids, saved.items[fuel].fluids);
    for order in [
        vec![3, 0, 0],
        vec![3, 0, 6],
        vec![0, 1, 2, 3, 4, 5, 0],
        vec![3],
    ] {
        let mut invalid = saved.clone();
        invalid.items[fuel].fluids.as_mut().unwrap().order = order;
        assert!(invalid.validate(&reference, &stamp).is_err());
    }
    for amount in [
        -1.,
        f32::NAN,
        reference
            .tanks
            .iter()
            .find(|tank| tank.item == fuel_id)
            .unwrap()
            .capacity_liters
            + 1.,
    ] {
        let mut invalid = saved.clone();
        invalid.items[fuel]
            .fluids
            .as_mut()
            .unwrap()
            .quantities_liters[0] = amount;
        assert!(invalid.validate(&reference, &stamp).is_err());
    }
    game.area
        .items
        .get_mut(fuel_id)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[1] = 1.;
    let quantity_only = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert_eq!(
        quantity_only.items[fuel].fluids.as_ref().unwrap().order,
        [3, 0, 1]
    );
    let restored = GarageRuntime::from_save(&reference, &stamp, &quantity_only).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
        .area
        .items
        .capture_save(
            &ids,
            &restored.stream.origin,
            &restored.physics,
            &restored.area.interaction,
            &reference,
        )
        .unwrap();
    assert_eq!(recaptured[fuel].fluids, quantity_only.items[fuel].fluids);
    let mut retained_zero = saved.clone();
    retained_zero.items[fuel]
        .fluids
        .as_mut()
        .unwrap()
        .order
        .push(5);
    retained_zero.validate(&reference, &stamp).unwrap();
    let restored = GarageRuntime::from_save(&reference, &stamp, &retained_zero).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
        .area
        .items
        .capture_save(
            &ids,
            &restored.stream.origin,
            &restored.physics,
            &restored.area.interaction,
            &reference,
        )
        .unwrap();
    assert_eq!(recaptured[fuel].fluids.as_ref().unwrap().order, [3, 0, 5]);
    let mut invalid = saved.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.fluids.is_none())
        .unwrap()
        .fluids = Some(FluidSnapshot {
        quantities_liters: [0.; 6],
        order: vec![],
    });
    assert!(invalid.validate(&reference, &stamp).is_err());
    let mut invalid = saved.clone();
    invalid.schema_version = 6;
    assert!(invalid.validate(&reference, &stamp).is_err());
    assert!(GarageRuntime::from_save(&reference, &stamp, &invalid).is_err());
    assert!(matches!(
        SaveSnapshot::from_json(br#"{"schema_version":6}"#),
        Err(SaveError::IncompatibleSources(_))
    ));
    assert!(
        serde_json::from_str::<FluidSnapshot>(
            r#"{"quantities_liters":[0,0,0,0,0,0],"order":[],"extra":true}"#
        )
        .is_err()
    );
}
