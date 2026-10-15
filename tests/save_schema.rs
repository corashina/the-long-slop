use my_summer_slop::reference::{ReferenceManifest, SourceId};
use my_summer_slop::save::{
    ConditionSnapshot, IdentityMap, InstanceId, ItemLocationSave, SaveError, SaveSnapshot,
    SourceKey, SourceStamp,
};
use serde_json::{Value, json};

fn key(source: &SourceId) -> Value {
    json!({"file": source.file, "path_id": source.path_id.to_string()})
}
fn instance(source: &SourceId) -> Value {
    json!({"kind": "initial", "source": key(source)})
}
fn fixture() -> (ReferenceManifest, SourceStamp, SaveSnapshot) {
    let r = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    snapshot_for(r)
}
fn snapshot_for(r: ReferenceManifest) -> (ReferenceManifest, SourceStamp, SaveSnapshot) {
    let source = json!({"app_id": r.app_id, "build_id": r.build_id,
        "manifest_sha256": "a".repeat(64), "source_hashes": r.source_hashes});
    let item = |id| instance(&r.items.iter().find(|i| i.id == id).unwrap().source);
    let pose = json!({"position": [0., 0., 0.], "rotation": [0., 0., 0., 1.]});
    let local = pose.clone();
    let items: Vec<_> = r
        .items
        .iter()
        .map(|i| {
            let location = if let Some(s) = r.part_slots.iter().find(|s| s.part == Some(i.id)) {
                json!({"kind": "part_slot", "slot": key(&s.source)})
            } else if let Some(s) = r.wheel_slots.iter().find(|s| s.rim == i.id) {
                json!({"kind": "wheel_slot", "slot": key(&s.source)})
            } else if let Some(s) = r.wheel_slots.iter().find(|s| s.tire == i.id) {
                json!({"kind": "attached", "owner": item(s.rim)})
            } else if let Some(owner) = i.attached_to {
                json!({"kind": "attached", "owner": item(owner)})
            } else {
                json!({"kind": "world"})
            };
            json!({"instance": item(i.id), "pose": pose, "local_pose": local,
            "location": location,
            "temperature": r.vehicles.iter().find(|v| v.engine.item == i.id).map(|v| v.engine.initial_temperature),
            "fluids": r.tanks.iter().find(|t| t.item == i.id).map(|t| t.quantities_liters)})
        })
        .collect();
    let parts: Vec<_> = r
        .part_slots
        .iter()
        .map(|s| json!({"slot": key(&s.source), "part": s.part.map(item)}))
        .collect();
    let wheels: Vec<_> = r
        .wheel_slots
        .iter()
        .map(|s| json!({"slot": key(&s.source), "rim": item(s.rim), "rotation": 0., "rpm": 0., "previous_rotation": 0.}))
        .collect();
    let tires: Vec<_> = r
        .items
        .iter()
        .zip(&items)
        .filter(|(source, saved)| {
            matches!(source.kind, my_summer_slop::reference::ItemKind::Tire)
                && saved["location"]["kind"] == "attached"
        })
        .map(|(_, saved)| json!({"rim": saved["location"]["owner"], "tire": saved["instance"]}))
        .collect();
    let bodies: Vec<_> = r
        .items
        .iter()
        .zip(&items)
        .filter(|(source, saved)| {
            (r.parts.iter().any(|p| p.item == source.id)
                || r.wheel_parts.iter().any(|p| p.item == source.id))
                && saved["location"]["kind"] == "world"
        })
        .map(|(_, saved)| {
            json!({
                "identity": {"kind": "assembly", "root": saved["instance"]}, "pose": pose,
                "linear_velocity": [0.,0.,0.], "angular_velocity": [0.,0.,0.], "sleeping": false
            })
        })
        .collect();
    let controls: Vec<_> = r
        .mechanisms
        .iter()
        .map(|m| {
            json!({"source": key(&m.source),
        "degrees": m.rotation.as_ref().map_or([0.;2], |x| x.initial_degrees),
        "slide": m.slide.as_ref().map_or(0., |x| x.initial_value), "enabled": m.enabled,
        "turn_state": m.turn.as_ref().map_or(0, |x| x.initial_state)})
        })
        .collect();
    let openings: Vec<_> = r
        .openings
        .iter()
        .map(|o| json!({"source": key(&o.source), "valve": o.valve}))
        .collect();
    let controllers: Vec<_> = r
        .lighting
        .controllers
        .iter()
        .map(|c| {
            json!({"source": key(&c.source),
        "current_state": c.current_state, "requested_state": c.requested_state,
        "refreshed_state": c.current_state, "energy": c.energy, "enabled": c.enabled,
        "on": c.startup_on, "cabin_mode": c.cabin_input.as_ref().map(|x| x.startup_mode),
        "switch_motion": null, "switch_poses": c.animated_poses.iter().map(|p| json!({
            "node": key(&r.nodes.iter().find(|n| n.id == p.node).unwrap().source),
            "pose": {"position": p.states[0].translation, "rotation": p.states[0].rotation}
        })).collect::<Vec<_>>()})
        })
        .collect();
    let vehicles: Vec<_> = r.vehicles.iter().map(|v| json!({"instance": instance(&v.source), "body": {"identity": {"kind": "source", "source": key(&r.bodies.iter().find(|b| b.id == v.body).unwrap().source)},
            "pose": pose, "linear_velocity": [0.,0.,0.], "angular_velocity": [0.,0.,0.], "sleeping": false},
            "engine": {"ignition": false, "rpm": 0., "temperature": v.engine.initial_temperature,
                "elapsed": 0., "last_run_time": -1., "last_start_chance_time": 0., "random_start": 1.,
                "supplied_fuel": 0., "fan_on": false}, "gear": 0, "handbrake": true, "trip_m": 0., "requested_headlights": 0})).collect();
    let runtime = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    let (surfaces, washes) = runtime.area.conditions.capture_save(&r).unwrap();
    let tools = runtime.area.interaction.cleaning.capture_save(&r).unwrap();
    let value = json!({"conditions": ConditionSnapshot {surfaces,washes,tools}, "schema_version": 4, "driving_assists": r.driving.default_assists, "saved_at": "2026-10-12T10:00:00.000Z", "source": source,
        "world": {"seed": 42, "changes": my_summer_slop::world::ChangeRecords::default(), "spawn": my_summer_slop::world::spawn_state::SourceSpawnState::supplied_controlled_startup(&r.world.spawn_context, my_summer_slop::world::spawn_state::SpawnContext { current_car: r.vehicles[0].spawn_car_id, peaceful_enemies: 0, date: r.world.spawn_context.date, chance_bits: None }).capture_history()},
        "player": {"pose": pose, "yaw": 0., "pitch": 0., "seat": null, "crouched": false,
            "vertical_velocity": 0., "recoil_velocity": [0.,0.,0.], "running": false, "carry": null},
        "vehicles": vehicles,
        "bodies": bodies, "items": items, "attachments": {"parts": parts, "wheels": wheels, "tires": tires},
        "mechanisms": {"controls": controls, "openings": openings}, "progress": {"selected_vehicle": instance(&r.vehicles[0].source), "start_attempts": "0"},
        "lighting": {"cycle": {"elapsed_seconds": 0., "cycle_seconds": r.lighting.cycle.startup_offset_seconds,
            "evaluated_cycle_seconds": r.lighting.cycle.startup_offset_seconds, "cloud_coordinate": 0., "star_rotation": 0.},
            "controllers": controllers}, "view": {"camera": {"kind": "first_person"}}});
    let snapshot = SaveSnapshot::from_json(&serde_json::to_vec(&value).unwrap()).unwrap();
    (r, serde_json::from_value(source).unwrap(), snapshot)
}

#[test]
fn schema_rejects_version_source_and_invalid_links() {
    let (reference, source, snapshot) = fixture();
    snapshot.validate(&reference, &source).unwrap();
    let mut bad = snapshot.clone();
    bad.schema_version = 1;
    assert!(bad.validate(&reference, &source).is_err());
    let value = serde_json::to_value(&snapshot).unwrap();
    for field in ["unknown", "survival", "weather"] {
        let mut bad = value.clone();
        bad[field] = json!({});
        assert!(SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for changed in [
        json!({"manifest_sha256": "b".repeat(64)}),
        json!({"build_id": 1}),
    ] {
        let mut bad = value.clone();
        for (key, value) in changed.as_object().unwrap() {
            bad["source"][key] = value.clone();
        }
        assert!(
            SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap())
                .unwrap()
                .validate(&reference, &source)
                .is_err()
        );
    }
    let mut bad = value.clone();
    bad["source"]["source_hashes"][0]["sha256"] = json!("b".repeat(64));
    assert!(
        SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap())
            .unwrap()
            .validate(&reference, &source)
            .is_err()
    );
    let mut bad = snapshot.clone();
    bad.items.push(bad.items[0].clone());
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = value.clone();
    bad["items"][0]["location"] = json!({"kind":"attached", "owner": bad["items"][0]["instance"]});
    assert!(
        SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap())
            .unwrap()
            .validate(&reference, &source)
            .is_err()
    );
    let mut bad = value.clone();
    bad["attachments"]["parts"][0]["part"] = value["items"][0]["instance"].clone();
    assert!(
        SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap())
            .unwrap()
            .validate(&reference, &source)
            .is_err()
    );
    for attempts in ["01", "-1", "18446744073709551616"] {
        let mut bad = snapshot.clone();
        bad.progress.start_attempts = attempts.into();
        assert!(bad.validate(&reference, &source).is_err());
    }
    let mut bad = snapshot.clone();
    bad.attachments.wheels[0].rpm = f32::NAN;
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.attachments.wheels[0].previous_rotation = f32::INFINITY;
    assert!(bad.validate(&reference, &source).is_err());
    let mut missing = value.clone();
    missing["progress"]
        .as_object_mut()
        .unwrap()
        .remove("start_attempts");
    assert!(SaveSnapshot::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut bad = snapshot.clone();
    bad.items[0].pose.position[0] = f64::NAN;
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.items[0].pose.rotation = [0.; 4];
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.items
        .iter_mut()
        .find(|i| i.fluids.is_some())
        .unwrap()
        .fluids = Some([f32::MAX; 6]);
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.mechanisms.openings[0].valve = 2.;
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.items.pop();
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = value.clone();
    bad["player"]["pose"]["weather"] = json!({});
    assert!(SaveSnapshot::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = snapshot.clone();
    bad.source
        .source_hashes
        .push(bad.source.source_hashes[0].clone());
    assert!(bad.validate(&reference, &source).is_err());
    let mut incompatible = reference.clone();
    incompatible.part_slots[0].part_types = vec!["incompatible".into()];
    assert_eq!(
        snapshot.validate(&incompatible, &source),
        Err(SaveError::InvalidLinks("Incompatible part slot".into()))
    );
    let mut incompatible = reference.clone();
    incompatible.wheel_slots[0].rim_types = vec!["incompatible".into()];
    assert_eq!(
        snapshot.validate(&incompatible, &source),
        Err(SaveError::InvalidLinks("Incompatible wheel slot".into()))
    );
    // A detached assembly can come from a parented source body.
    let mut parented = reference.clone();
    let mut child = parented.bodies[0].clone();
    child.id = my_summer_slop::reference::BodyId(90003);
    child.source = SourceId {
        file: "fixture".into(),
        path_id: 90003,
    };
    child.parent_body = Some(parented.vehicles[0].body);
    parented
        .items
        .iter_mut()
        .find(|i| i.id == parented.vehicles[0].engine.item)
        .unwrap()
        .body = child.id;
    parented.bodies.push(child.clone());
    let mut independent = snapshot.clone();
    let mut body = independent.vehicles[0].body.clone();
    body.identity = my_summer_slop::save::BodyIdentity::Source {
        source: SourceKey::from(&child.source),
    };
    independent.bodies.push(body.clone());
    assert_eq!(
        independent.validate(&parented, &source),
        Err(SaveError::InvalidLinks(
            "Parented source body is not independent".into()
        ))
    );
    let engine_id = InstanceId::initial(
        &parented
            .items
            .iter()
            .find(|i| i.id == parented.vehicles[0].engine.item)
            .unwrap()
            .source,
    );
    let engine = independent
        .attachments
        .parts
        .iter_mut()
        .find(|s| s.part.as_ref() == Some(&engine_id))
        .unwrap()
        .part
        .take()
        .unwrap();
    independent
        .items
        .iter_mut()
        .find(|i| i.instance == engine)
        .unwrap()
        .location = ItemLocationSave::World;
    independent.bodies.last_mut().unwrap().identity =
        my_summer_slop::save::BodyIdentity::Assembly { root: engine };
    independent.validate(&parented, &source).unwrap();
    let mut cyclic = reference.clone();
    cyclic.part_slots[0].owner = cyclic.part_slots[0].part;
    assert_eq!(
        snapshot.validate(&cyclic, &source),
        Err(SaveError::InvalidLinks("Attachment cycle".into()))
    );
    let mut bad = snapshot.clone();
    bad.bodies.push(bad.vehicles[0].body.clone());
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.items[0].location = ItemLocationSave::Held;
    bad.items[1].location = ItemLocationSave::Held;
    assert!(bad.validate(&reference, &source).is_err());
    let mut bad = snapshot.clone();
    bad.vehicles.push(bad.vehicles[0].clone());
    assert!(bad.validate(&reference, &source).is_err());
}

#[test]
fn identity_preserves_large_signed_values() {
    let key = SourceKey {
        file: "sharedassets0.assets".into(),
        path_id: "-9223372036854775808".into(),
    };
    let restored: SourceKey = serde_json::from_slice(&serde_json::to_vec(&key).unwrap()).unwrap();
    assert_eq!(restored, key);
    let id = InstanceId::Generated {
        id: my_summer_slop::world::GeneratedId {
            layer: my_summer_slop::world::WorldLayer::Building,
            generator: SourceId {
                file: "fixture".into(),
                path_id: i64::MIN,
            },
            unit: [i64::MIN, i64::MAX],
            ordinal: u32::MAX,
            children: vec![],
        },
    };
    let restored: InstanceId = serde_json::from_slice(&serde_json::to_vec(&id).unwrap()).unwrap();
    assert_eq!(restored, id);
    let (mut reference, _, _) = fixture();
    reference.items[0].source.path_id = i64::MIN;
    reference.items[0].source.file = key.file.clone();
    let map = IdentityMap::new(&reference).unwrap();
    assert_eq!(
        map.item(&InstanceId::Initial { source: key }).unwrap(),
        reference.items[0].id
    );
    reference.items.reverse();
    let reordered = IdentityMap::new(&reference).unwrap();
    let key = SourceKey::from(&reference.items.last().unwrap().source);
    assert_eq!(
        reordered
            .item(&InstanceId::Initial { source: key })
            .unwrap(),
        reference.items.last().unwrap().id
    );
}

#[test]
fn fixed_source_child_cannot_become_an_independent_item() {
    let (mut reference, _, _) = fixture();
    let owner = reference
        .items
        .iter()
        .find(|i| {
            matches!(i.kind, my_summer_slop::reference::ItemKind::Food) && i.attached_to.is_none()
        })
        .unwrap()
        .id;
    let child = reference.items[0].id;
    assert!(!reference.parts.iter().any(|part| part.item == child));
    assert!(!reference.wheel_parts.iter().any(|part| part.item == child));
    reference.items[0].attached_to = Some(owner);
    let (reference, source, mut snapshot) = snapshot_for(reference);
    snapshot.validate(&reference, &source).unwrap();
    snapshot.items[0].location = ItemLocationSave::World;
    assert_eq!(
        snapshot.validate(&reference, &source),
        Err(SaveError::InvalidLinks(
            "Fixed source attachment changed".into()
        ))
    );
}
