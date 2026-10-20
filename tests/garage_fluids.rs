//! Critical G04 persistence checks against the installed single-Lada export.
use my_summer_slop::{
    fluids::{FluidContainer, FluidEntry},
    reference::ReferenceManifest,
    save::{CameraSave, SourceHash, SourceStamp},
    starting_area::GarageRuntime,
};

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_ordered_save_load_cleans_once_before_detached_mass_sync() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let digest = std::process::Command::new("sha256sum")
        .arg("web/assets/reference/manifest.json")
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: r.app_id,
        build_id: r.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: r
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let mut run = GarageRuntime::new(&r).unwrap();
    let oil = r.vehicles[0].engine.oil_tank;
    let engine = r.vehicles[0].engine.item;
    run.area
        .interaction
        .parts
        .remove(engine, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    let capacity = run
        .area
        .items
        .get(oil)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .capacity_liters;
    let entries = [
        FluidEntry {
            fluid_type: 3,
            amount: -0.5,
        },
        FluidEntry {
            fluid_type: 0,
            amount: 0.,
        },
        FluidEntry {
            fluid_type: 2,
            amount: -0.25,
        },
        FluidEntry {
            fluid_type: 1,
            amount: capacity + 1.,
        },
    ];
    run.area.items.get_mut(oil).unwrap().fluid = Some(FluidContainer::new(capacity, &entries));
    let saved = run
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T12:00:00Z")
        .unwrap();
    let restored = GarageRuntime::from_save(&r, &stamp, &saved).unwrap();
    let tank = restored
        .area
        .items
        .get(oil)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap();
    assert_eq!(tank.entries().collect::<Vec<_>>(), entries[1..]);
    let body = restored.area.items.get(engine).unwrap().body.unwrap();
    assert_ne!(body, restored.physics.chassis_handle(0));
    let mass = restored.physics.rapier().bodies[body].mass();
    // A second source load removes the retained zero, without changing mass.
    let recaptured = restored
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T12:00:00Z")
        .unwrap();
    let second = GarageRuntime::from_save(&r, &stamp, &recaptured).unwrap();
    let second_tank = second.area.items.get(oil).unwrap().fluid.as_ref().unwrap();
    assert_eq!(second_tank.entries().collect::<Vec<_>>(), entries[2..]);
    let second_body = second.area.items.get(engine).unwrap().body.unwrap();
    assert!((second.physics.rapier().bodies[second_body].mass() - mass).abs() < 0.001);
    let third_save = second
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T12:00:00Z")
        .unwrap();
    let third = GarageRuntime::from_save(&r, &stamp, &third_save).unwrap();
    let third_body = third.area.items.get(engine).unwrap().body.unwrap();
    assert!((third.physics.rapier().bodies[third_body].mass() - mass - 0.25).abs() < 0.001);
    // World reactivation restores complete state without a logical source load.
    let snapshot = second.area.items.generated_snapshot(oil, None).unwrap();
    let mut reactivated = GarageRuntime::from_save(&r, &stamp, &saved).unwrap();
    reactivated
        .area
        .items
        .restore_generated(oil, &snapshot)
        .unwrap();
    assert_eq!(
        reactivated
            .area
            .items
            .get(oil)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .entries()
            .collect::<Vec<_>>(),
        entries[2..]
    );
}
