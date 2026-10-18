//! Native adapter for the same renderer receipts and publication boundary used by WASM.
use super::*;
use crate::starting_area::GarageRuntime;

fn reference() -> ReferenceManifest {
    ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap()
}

#[test]
#[ignore = "requires the installed source export"]
fn structure_refresh_accumulates_both_foci_and_forced_refreshes() {
    let reference = reference();
    let mut run = GarageRuntime::new(&reference).unwrap();
    run.stream.request_at([0.; 3], [0.; 3]);
    for _ in 0..10000 {
        run.stream.prepare(100.);
        assert!(run.stream.failure.is_none(), "{:?}", run.stream.failure);
        if run.stream.startup.is_some() {
            break;
        }
    }
    assert!(run.stream.startup.is_some());
    run.stream.prepare(0.);
    assert_eq!(run.stream.plan_revision, run.stream.request.revision);
    assert_eq!(run.stream.structure_focus, [[0.; 3]; 2]);
    let (item, body) = run
        .stream
        .imported_baselines
        .iter()
        .find_map(|(id, _)| {
            let item = run.area.items.get(*id)?;
            item.inactive_pose.is_none().then_some(())?;
            Some((*id, item.body?))
        })
        .expect("imported item with a live physical body");
    let capture_and_prepare = |run: &mut GarageRuntime, player, vehicle, expected| {
        let mut pose = *run.physics.rapier().bodies[body].position();
        pose.translation.y += 0.125;
        run.physics.rapier_mut().bodies[body].set_position(pose, true);
        run.area.items.sync_poses(&run.physics);
        run.stream.request_at(player, vehicle);
        let captured = run.stream.changes.revision();
        run.stream.capture_changes(
            &reference,
            &mut run.area.items,
            &run.area.interaction,
            &run.physics,
        );
        assert_ne!(run.stream.changes.revision(), captured);
        assert!(
            run.stream
                .changes
                .get(super::super::ObjectKey::Imported(item))
                .is_some()
        );
        let revision = run.stream.request.revision;
        run.stream.prepare(0.);
        assert_ne!(run.stream.request.revision, revision);
        assert_eq!(run.stream.plan_revision, run.stream.request.revision);
        assert_eq!(run.stream.selection_changes, run.stream.changes.revision());
        assert_eq!(run.stream.structure_focus, expected);
        assert!(run.stream.failure.is_none(), "{:?}", run.stream.failure);
    };
    let vehicle = [0.; 3];
    for x in [0.25, 0.5, 0.75] {
        capture_and_prepare(&mut run, [x, 0., 0.], vehicle, [[0.; 3]; 2]);
    }
    capture_and_prepare(&mut run, [1., 0., 0.], vehicle, [[1., 0., 0.], vehicle]);
    capture_and_prepare(
        &mut run,
        [1., 0., 0.],
        [0., 0., 0.75],
        [[1., 0., 0.], vehicle],
    );
    capture_and_prepare(
        &mut run,
        [1., 0., 0.],
        [0., 0., 1.],
        [[1., 0., 0.], [0., 0., 1.]],
    );
    // A forced selection rebuild also moves the distance baseline.
    capture_and_prepare(
        &mut run,
        [1.25, 0., 0.],
        [0., 0., 1.],
        [[1., 0., 0.], [0., 0., 1.]],
    );
    run.stream.plan(true).unwrap();
    assert_eq!(run.stream.structure_focus, [[1.25, 0., 0.], [0., 0., 1.]]);
    capture_and_prepare(
        &mut run,
        [2., 0., 0.],
        [0., 0., 1.],
        [[1.25, 0., 0.], [0., 0., 1.]],
    );
    capture_and_prepare(
        &mut run,
        [2.25, 0., 0.],
        [0., 0., 1.],
        [[2.25, 0., 0.], [0., 0., 1.]],
    );
}

#[test]
#[ignore = "requires the installed source export"]
fn structure_publication_uses_nearest_focus_stable_ties_and_revision_guard() {
    let mut reference = reference();
    // Retain a checked source identity but remove collision from this ordering check.
    let prototype = std::sync::Arc::get_mut(&mut reference.world)
        .unwrap()
        .prototypes
        .iter_mut()
        .find(|p| p.role == crate::reference::WorldPrototypeRole::Structure)
        .unwrap();
    prototype.nodes.clear();
    prototype.bodies.clear();
    prototype.colliders.clear();
    let source = prototype.source.clone();
    let scale = prototype.prefab_pose.scale;
    let mut run = GarageRuntime::new(&reference).unwrap();
    let stream = &mut run.stream;
    stream.request.player = [0.; 3];
    stream.request.vehicle = [100., 0., 0.];
    stream.wanted_structures = [50., 101., -1.]
        .into_iter()
        .enumerate()
        .map(|(index, x)| {
            let mut p = replay_placement(&source, index as u32, [x, 0., 0.]);
            p.scale = scale;
            p
        })
        .collect();
    stream.reserve_prototype(source.clone(), 0, 0).unwrap();
    stream.bind_prototype(&source).unwrap();
    stream.bind_structure_recipe(&source).unwrap();
    for index in 0..stream.wanted_structures.len() {
        let id = stream.wanted_structures[index].id.clone();
        let mut claim = stream
            .reserve_instance_renderer(stream.request, &id, 0, 0, 0)
            .unwrap();
        stream
            .accept_structure_renderer(stream.request, &id, &mut claim)
            .unwrap();
    }
    assert_ne!(stream.plan_revision, stream.request.revision);
    stream.publish_structure(&mut run.physics, &[]).unwrap();
    assert!(
        stream.structures.is_empty(),
        "stale selection cannot publish"
    );
    stream.plan_revision = stream.request.revision;
    for ordinal in [1, 2, 0] {
        stream.publish_structure(&mut run.physics, &[]).unwrap();
        assert_eq!(
            stream.structures.last().unwrap().placement.id.ordinal,
            ordinal
        );
    }
    assert!(stream.ready_structures.is_empty());
}
fn views() -> [super::super::RequiredView; 1] {
    let planes = [
        [1., 0., 0., 150.],
        [-1., 0., 0., 150.],
        [0., 1., 0., 150.],
        [0., -1., 0., 150.],
        [0., 0., 1., 150.],
        [0., 0., -1., 150.],
    ];
    [super::super::RequiredView {
        camera: planes,
        shadow: crate::reference::shadow_candidates::DirectionalHull::new(planes, [1., 0., 0.])
            .unwrap(),
        mask: u32::MAX,
        light_mask: u32::MAX,
        origin: [0.; 3],
    }]
}
fn prepare_step(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    surface_claim: &mut Option<PipelineContinuation>,
) {
    let stream = &mut run.stream;
    stream.prepare(100.);
    if stream
        .ready
        .as_ref()
        .is_some_and(|w| !same_request(w.request, stream.request))
    {
        if let Some(mut claim) = surface_claim.take() {
            stream.discard_surface_renderer(&mut claim).unwrap();
        }
    }
    stream.select_required(&views());
    let sources: Vec<_> = reference
        .world
        .terrain
        .materials
        .iter()
        .map(|m| m.source.clone())
        .chain(reference.world.road.prototypes.iter().cloned())
        .collect();
    for source in sources {
        stream.reserve_prototype(source.clone(), 0, 0).unwrap();
        stream.bind_prototype(&source).unwrap();
    }
    let placement = stream
        .wanted_structures
        .iter()
        .find(|p| {
            (stream.structure_required(&p.id)
                || stream.lights.iter().any(|light| {
                    stream.publication_light(light, &stream.required.lights)
                        && light.owned_by(&p.id)
                }))
                && stream.contents_initialized(&p.id)
                && !stream.structures.iter().any(|s| s.placement.id == p.id)
                && !stream.ready_structures.contains(&p.id)
        })
        .cloned();
    if let Some(p) = placement {
        stream.reserve_prototype(p.prototype.clone(), 0, 0).unwrap();
        stream.bind_prototype(&p.prototype).unwrap();
        stream.bind_structure_recipe(&p.prototype).unwrap();
        let mut claim = stream
            .reserve_instance_renderer(stream.request, &p.id, 0, 0, 0)
            .unwrap();
        stream
            .accept_structure_renderer(stream.request, &p.id, &mut claim)
            .unwrap();
    }
    if let Some((request, surface)) = stream.prepared_surface() {
        let key = surface.key;
        if !stream.ready.as_ref().unwrap().renderer_reserved {
            let claim = stream.reserve_surface_renderer(request, key).unwrap();
            stream
                .accept_surface_renderer(request, key, &claim)
                .unwrap();
            *surface_claim = Some(claim);
        }
    }
    let lights = stream.required.lights.clone();
    stream.publish(&mut run.physics, &lights).unwrap();
    if stream.ready.is_none() {
        *surface_claim = None;
    }
    stream
        .sync_supplies(
            reference,
            &mut run.area.items,
            &mut run.area.interaction,
            &mut run.physics,
            &mut run.wheels,
        )
        .unwrap();
    assert!(stream.failure.is_none(), "{:?}", stream.failure);
    run.align_startup(reference).unwrap();
}

fn prepare_ready(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    let mut surface_claim = None;
    for step in 0..2000 {
        prepare_step(run, reference, &mut surface_claim);
        if run.stream.required_ready() {
            if let Some(mut claim) = surface_claim.take() {
                let key = run.stream.ready.as_ref().unwrap().published.key;
                assert!(!run.stream.required.terrain.contains(&key));
                assert!(
                    key.layer != WorldLayer::Road
                        || !run.stream.required.roads.contains(&(key.unit[0] as u32))
                );
                run.stream.discard_surface_renderer(&mut claim).unwrap();
            }
            return;
        }
        if step == 1999 {
            println!("stalled: {}", run.stream.startup_job_audit());
        }
    }
    panic!("required publication did not reach readiness");
}
fn withdraw_terrain(run: &mut GarageRuntime, key: SurfaceKey) {
    let index = run
        .stream
        .surfaces
        .iter()
        .position(|s| s.key == key)
        .unwrap();
    let mut surface = run.stream.surfaces.remove(index);
    run.physics
        .remove_collider(surface.collider.take().unwrap());
    surface.collision_bytes = 0;
    assert!(run.stream.ready.is_none());
    run.stream.ready = Some(PreparedSurface {
        pending_bytes: 0,
        renderer_ready: false,
        renderer_reserved: false,
        request: run.stream.request,
        published: surface,
    });
}
fn publish_with_receipts(run: &mut GarageRuntime) {
    let stream = &mut run.stream;
    let (request, surface) = stream.prepared_surface().unwrap();
    let key = surface.key;
    let mut claim = stream.reserve_surface_renderer(request, key).unwrap();
    for stage in ["geometry", "caster", "effect"] {
        let receipt = super::super::upload_receipt::UploadReceipt::default();
        receipt.register();
        claim.watch_upload(receipt.clone());
        assert!(
            stream
                .accept_surface_renderer(request, key, &claim)
                .is_err(),
            "{stage} must finish before renderer publication"
        );
        let lights = stream.required.lights.clone();
        stream.publication_available = true;
        stream.publish(&mut run.physics, &lights).unwrap();
        assert!(!stream.required_ready(), "{stage} must block readiness");
        receipt.complete();
    }
    stream
        .accept_surface_renderer(request, key, &claim)
        .unwrap();
    let lights = stream.required.lights.clone();
    stream.publication_available = true;
    stream.publish(&mut run.physics, &lights).unwrap();
}
#[test]
#[ignore = "requires the installed source export"]
fn required_publication_reaches_ready_and_withholds_dependencies() {
    let reference = reference();
    let mut run = GarageRuntime::new(&reference).unwrap();
    prepare_ready(&mut run, &reference);
    assert!(run.stream.required_ready());
    assert!(run.world_supported());
    println!(
        "ready: roads={} tiles={} structures={} lights={}; active bindings={} static={}",
        run.stream.required.roads.len(),
        run.stream.required.terrain.len(),
        run.stream.required.structures.len(),
        run.stream.required.lights.len(),
        run.stream.contents.active.len(),
        run.stream.static_controls.len()
    );
    let key = *run
        .stream
        .required
        .terrain
        .iter()
        .find(|k| k.layer == WorldLayer::Terrain)
        .unwrap();
    withdraw_terrain(&mut run, key);
    assert!(
        !run.stream.required_ready(),
        "unpublished geometry/collision must block readiness"
    );
    publish_with_receipts(&mut run);
    assert!(run.stream.required_ready());
    let light = run
        .stream
        .lights
        .iter()
        .find(|l| run.stream.required.lights.contains(&l.id))
        .unwrap();
    let index = run
        .stream
        .structures
        .iter()
        .position(|s| light.owned_by(&s.placement.id))
        .unwrap();
    let owner = run.stream.structures.remove(index);
    assert!(
        !run.stream.required_ready(),
        "required light owner must publish"
    );
    run.stream.structures.insert(index, owner);
    assert!(run.stream.required_ready());
    mechanism_binding_gate(&mut run, &reference);
    swept_boundaries(&mut run, &reference);
}

fn move_player(run: &mut GarageRuntime, position: [f32; 3]) {
    let old = run.area.player.body_pose(&run.physics).position;
    run.area
        .player
        .rebase(std::array::from_fn(|i| old[i] - position[i]));
    let pose = run.area.player.body_pose(&run.physics);
    let body = run.area.player.body_handle();
    run.physics.rapier_mut().bodies[body].set_position(crate::vehicle::to_rapier(pose), true);
    run.physics.refresh_queries();
}
fn swept_boundaries(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    use crate::simulation::{FrameInput, keys};
    use rapier3d::math::Vector;
    let original_player = run.area.player.body_pose(&run.physics).position;
    let original_car = run.physics.snapshot(run.vehicle).pose;
    let size = run.stream.tiles[0].size as f32;
    let boundary = size * 0.5;
    let key = SurfaceKey {
        layer: WorldLayer::Terrain,
        unit: [1, 0],
    };
    move_player(run, [boundary - 7.5, original_player[1] + 10., 0.]);
    run.started = true;
    assert!(run.world_supported());
    withdraw_terrain(run, key);
    let before = run.area.player.body_pose(&run.physics);
    assert!(
        run.stream
            .supports(window(run.stream.origin.absolute(before.position), 1.)),
        "player's current tile stays published"
    );
    assert!(
        !run.world_supported(),
        "player movement envelope reaches the unpublished neighbor"
    );
    for _ in 0..4 {
        run.step(
            reference,
            FrameInput {
                held: keys::RIGHT,
                ..Default::default()
            },
        );
    }
    assert_eq!(
        run.area.player.body_pose(&run.physics),
        before,
        "player must stop before unresolved collision"
    );
    publish_with_receipts(run);
    assert!(run.world_supported());
    run.step(
        reference,
        FrameInput {
            held: keys::RIGHT,
            ..Default::default()
        },
    );
    let after = run.area.player.body_pose(&run.physics);
    assert!(
        (after.position[0] - before.position[0]).abs()
            + (after.position[2] - before.position[2]).abs()
            > 1e-5,
        "player advances after collision publication"
    );

    let player = run.stream.origin.local([
        f64::from(boundary - 18.),
        f64::from(original_player[1] + 10.),
        0.,
    ]);
    move_player(run, player);
    let mut pose = original_car;
    pose.position = run.stream.origin.local([
        f64::from(boundary - 9.),
        f64::from(original_car.position[1] + 10.),
        0.,
    ]);
    run.physics.reset_preserving_wheels(run.vehicle, pose);
    run.stream.request_at(
        run.stream
            .origin
            .absolute(run.area.player.focus_position(&run.physics)),
        run.stream
            .origin
            .absolute(run.physics.snapshot(run.vehicle).pose.position),
    );
    prepare_ready(run, reference);
    run.sync_world_activity(reference).unwrap();
    run.physics.reset_preserving_wheels(run.vehicle, pose);
    assert_eq!(run.physics.snapshot(run.vehicle).pose, pose);
    let chassis = run.physics.chassis_handle(run.vehicle);
    assert!(
        run.physics.body_active(chassis),
        "vehicle must be active for the swept guard"
    );
    run.physics.rapier_mut().bodies[chassis].set_linvel(Vector::ZERO, true);
    withdraw_terrain(run, key);
    assert!(
        run.world_supported(),
        "stationary car remains in the prepared region"
    );
    run.physics.rapier_mut().bodies[chassis].set_linvel(Vector::new(120., 0., 0.), true);
    assert!(
        !run.world_supported(),
        "vehicle swept support includes its next step: pose={:?} velocity={:?} origin={:?} boundary={}",
        run.physics.snapshot(run.vehicle).pose.position,
        run.physics.rapier().bodies[chassis].linvel(),
        run.stream.origin.offset,
        boundary
    );
    let before = run.physics.snapshot(run.vehicle).pose;
    for _ in 0..4 {
        run.step(reference, FrameInput::default());
    }
    assert_eq!(
        run.physics.snapshot(run.vehicle).pose,
        before,
        "vehicle must stop before unresolved collision"
    );
    publish_with_receipts(run);
    assert!(run.world_supported());
    run.step(reference, FrameInput::default());
    assert!(
        run.physics.snapshot(run.vehicle).pose.position[0] > before.position[0] + 0.01,
        "vehicle advances after collision publication"
    );
}

fn mechanism_binding_gate(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    let prototype = reference
        .world
        .prototypes
        .iter()
        .find(|p| {
            reference
                .world
                .mechanisms
                .iter()
                .any(|m| m.prototype == p.source)
                && run
                    .stream
                    .contents
                    .rules
                    .component(&reference.world, &p.source, "buildingscript")
                    .unwrap()
                    .is_none()
        })
        .unwrap();
    let mut placement = run.stream.wanted_structures[0].clone();
    placement.prototype = prototype.source.clone();
    placement.id.layer = WorldLayer::Nested;
    placement.id.children.push((prototype.source.clone(), 999));
    placement.pose.position = run.stream.request.player;
    placement.bounds = window(placement.pose.position, 2.);
    placement.scale = prototype.prefab_pose.scale;
    let id = placement.id.clone();
    run.stream.wanted_structures.push(placement);
    run.stream.select_required(&views());
    assert!(run.stream.structure_required(&id));
    assert!(!run.stream.required_ready());
    run.stream
        .reserve_prototype(prototype.source.clone(), 0, 0)
        .unwrap();
    run.stream.bind_prototype(&prototype.source).unwrap();
    run.stream.bind_structure_recipe(&prototype.source).unwrap();
    let request = run.stream.request;
    let mut claim = run
        .stream
        .reserve_instance_renderer(request, &id, 0, 0, 0)
        .unwrap();
    run.stream
        .accept_structure_renderer(request, &id, &mut claim)
        .unwrap();
    run.stream.publication_available = true;
    let lights = run.stream.required.lights.clone();
    run.stream
        .publish_structure(&mut run.physics, &lights)
        .unwrap();
    run.stream
        .sync_static_controls(
            reference,
            &mut run.area.items,
            &mut run.area.interaction,
            &mut run.physics,
            &mut run.wheels,
        )
        .unwrap();
    let content = run
        .stream
        .static_controls
        .iter()
        .find(|c| c.content.placement.id == id)
        .unwrap();
    assert!(content.item.is_some());
    assert!(
        !content.bindings.is_empty(),
        "real mechanism must bind a control"
    );
    assert!(
        !run.stream.required_ready(),
        "unfinished control/interaction restoration blocks readiness"
    );
    run.stream
        .sync_supplies(
            reference,
            &mut run.area.items,
            &mut run.area.interaction,
            &mut run.physics,
            &mut run.wheels,
        )
        .unwrap();
    assert!(run.stream.bindings_ready(&id));
    assert!(
        run.stream.required_ready(),
        "control publication reaches terminal readiness"
    );
}

#[test]
#[ignore = "requires the installed source export"]
fn complete_probe_union_preserves_revision_and_fresh_cycles() {
    use crate::{
        reference::{
            loading_dispatch::{LoadingDispatch, LoadingKey},
            reflections::{ProbeJob, ProbeSchedule},
        },
        save::fresh_permit::PresentedFrames,
        world::{REQUIRED_VIEW_CAPACITY, RequiredView, RequiredViews},
    };
    let reference = reference();
    let mut run = GarageRuntime::new(&reference).unwrap();
    prepare_ready(&mut run, &reference);
    let tiny = |center: [f64; 3], mask| {
        let [x, y, z] = center.map(|v| v as f32);
        let planes = [
            [1., 0., 0., 0.25 - x],
            [-1., 0., 0., 0.25 + x],
            [0., 1., 0., 0.25 - y],
            [0., -1., 0., 0.25 + y],
            [0., 0., 1., 0.25 + z],
            [0., 0., -1., 0.25 - z],
        ];
        RequiredView {
            camera: planes,
            shadow: crate::reference::shadow_candidates::DirectionalHull::new(planes, [1., 0., 0.])
                .unwrap(),
            mask,
            light_mask: 0,
            origin: [0.; 3],
        }
    };
    // Published source structures provide distinct probe-exclusive geometry.
    let first_structure = run.stream.structures.iter().max_by(|a, b| {
        let a = a.placement.bounds.min[0].abs() + a.placement.bounds.min[2].abs();
        let b = b.placement.bounds.min[0].abs() + b.placement.bounds.min[2].abs();
        a.total_cmp(&b)
    });
    let center = first_structure
        .map(|s| {
            std::array::from_fn(|i| (s.placement.bounds.min[i] + s.placement.bounds.max[i]) * 0.5)
        })
        .expect("published source structures");
    let first = tiny(center, u32::MAX);
    let centers: Vec<_> = run
        .stream
        .structures
        .iter()
        .map(|s| {
            std::array::from_fn(|i| (s.placement.bounds.min[i] + s.placement.bounds.max[i]) * 0.5)
        })
        .collect();
    run.stream.select_required(&[first]);
    let first_ids = run.stream.required.structures.clone();
    let mut pair = None;
    for center in centers {
        let second = tiny(center, u32::MAX);
        run.stream.select_required(&[second]);
        let second_ids = &run.stream.required.structures;
        if first_ids.iter().any(|id| !second_ids.contains(id))
            && second_ids.iter().any(|id| !first_ids.contains(id))
        {
            pair = Some((second, second_ids.clone()));
            break;
        }
    }
    let (second, second_ids) = pair.expect("two probe-exclusive published regions");
    let seed = tiny([0.; 3], 0);
    let mut required = RequiredViews::new(seed);
    let build = |required: &mut RequiredViews, enabled: [bool; 2]| {
        required.reset(seed);
        required
            .append_probe_faces(&enabled, |probe, _| Ok([first, second][probe]))
            .unwrap();
    };
    build(&mut required, [true, true]);
    let storage = required.as_slice().as_ptr();
    run.stream.select_required(required.as_slice());
    let ids = run.stream.required.structures.clone();
    assert!(first_ids.iter().all(|id| ids.contains(id)));
    assert!(second_ids.iter().all(|id| ids.contains(id)));
    assert_eq!(required.as_slice().len(), 13);
    assert!(run.stream.required_ready());
    let revision = run.stream.required.revision;
    let key = LoadingKey {
        session: run.stream.request.session,
        scene: 1,
        extent: [800, 600],
    };
    let mut policy = LoadingDispatch::<2>::default();
    let mut schedule = [ProbeSchedule::default(), ProbeSchedule::default()];
    let mut probe = 0;
    for frame in 1..=16 {
        build(&mut required, [true, true]);
        run.stream.select_required(required.as_slice());
        assert_eq!(run.stream.required.structures, ids);
        assert_eq!(run.stream.required.revision, revision);
        assert_eq!(required.as_slice().as_ptr(), storage);
        policy.snapshot(key, frame, false, run.stream.required_ready(), revision, 3);
        let job = schedule[probe].job();
        if job == ProbeJob::Face(0) {
            policy.capture_started(key, frame, probe);
        }
        schedule[probe].complete(job).unwrap();
        if job == ProbeJob::RemainingMipsAndPublish {
            policy.publication_validated(key, probe);
            probe = (probe + 1) % 2;
        }
        assert_eq!(policy.fresh(key, revision, 3), frame == 16);
    }
    let mut presented = PresentedFrames::default();
    assert!(!presented.observe(key.session, key.scene, 16, 16));
    assert!(!presented.observe(key.session, key.scene, 17, 17));
    assert!(presented.observe(key.session, key.scene, 18, 18));
    build(&mut required, [true, false]);
    assert_eq!(required.as_slice().len(), 7);
    run.stream.select_required(required.as_slice());
    assert_ne!(run.stream.required.revision, revision);
    policy.snapshot(key, 19, false, true, run.stream.required.revision, 1);
    assert!(!policy.fresh(key, run.stream.required.revision, 1));
    while required.as_slice().len() < REQUIRED_VIEW_CAPACITY {
        required.push(seed).unwrap();
    }
    assert!(required.push(seed).is_err());
}

fn replay_placement(source: &SourceId, ordinal: u32, position: [f64; 3]) -> Placement {
    Placement {
        id: super::super::GeneratedId {
            layer: WorldLayer::Nested,
            generator: source.clone(),
            unit: [0, 0],
            ordinal,
            children: vec![(source.clone(), ordinal)],
        },
        prototype: source.clone(),
        pose: AbsolutePose {
            position,
            rotation: [0., 0., 0., 1.],
        },
        scale: [1.; 3],
        bounds: WorldBounds {
            min: position,
            max: position,
        },
        object_seed: 42,
    }
}
fn replay_snapshot(position: [f64; 3]) -> super::super::ObjectSnapshot {
    super::super::ObjectSnapshot {
        pose: AbsolutePose {
            position,
            rotation: [0., 0., 0., 1.],
        },
        condition: 0.5,
        food: None,
        temperature: None,
        fluid_quantities: None,
        valve: None,
        lamp: None,
        linear_velocity: [0.; 3],
        angular_velocity: [0.; 3],
        sleeping: false,
        held: false,
        attachment: None,
        mechanism: None,
    }
}
fn old_replay_keys(stream: &WorldStream) -> Vec<super::super::ObjectKey> {
    let radius = f64::from(stream.reference.distances["ItemSpawnDist"]);
    let mut result = Vec::new();
    let mut query = Vec::new();
    for focus in [stream.request.player, stream.request.vehicle] {
        stream.changes.old_keys_in(
            WorldBounds {
                min: [focus[0] - radius, f64::NEG_INFINITY, focus[2] - radius],
                max: [focus[0] + radius, f64::INFINITY, focus[2] + radius],
            },
            &mut query,
        );
        for key in query.drain(..) {
            if !result.contains(&key) {
                result.push(key);
            }
        }
    }
    result
}
fn assert_replay_selection(stream: &WorldStream) {
    let expected: Vec<_> = old_replay_keys(stream)
        .into_iter()
        .map(|key| {
            let change = stream.changes.get(&key).unwrap().clone();
            (key, change)
        })
        .collect();
    let actual: Vec<_> = stream
        .replay_indices
        .iter()
        .map(|&index| {
            let (key, change) = stream.changes.get_indexed(index);
            (key.clone(), change.clone())
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn replay_index_query_matches_old_order_and_exact_bounds() {
    use super::super::{ChangeRecords, ObjectKey};
    let source = SourceId {
        file: "replay-test".into(),
        path_id: 1,
    };
    let mut changes = ChangeRecords::default();
    let baseline = replay_snapshot([9000.; 3]);
    let positions = [
        [256., 0., 256.],
        [-1., 4., 0.],
        [0., 0., -1.],
        [0., 9000., 0.],
        [0., 0., 255.],
        [0., 0., 256.],
        [300., 0., 0.],
        [0., 0., 9000.],
    ];
    for (i, position) in positions.into_iter().enumerate() {
        let key = if i == 3 {
            ObjectKey::Imported(crate::reference::ItemId(3))
        } else {
            ObjectKey::Generated(replay_placement(&source, i as u32, position).id)
        };
        changes.record(key, replay_snapshot(position), &baseline);
    }
    let removed = ObjectKey::Generated(replay_placement(&source, 9, [0.; 3]).id);
    changes.record(removed.clone(), replay_snapshot([0.; 3]), &baseline);
    changes.remove(removed.clone());
    let bounds = [
        WorldBounds {
            min: [-1., f64::NEG_INFINITY, -1.],
            max: [256., f64::INFINITY, 256.],
        },
        WorldBounds {
            min: [0., -1., 0.],
            max: [300., 1., 256.],
        },
        WorldBounds {
            min: [0.; 3],
            max: [0.; 3],
        },
    ];
    let mut indices = Vec::new();
    let mut keys = Vec::new();
    for bounds in bounds {
        changes.indices_in(bounds, &mut indices);
        changes.old_keys_in(bounds, &mut keys);
        let actual: Vec<_> = indices
            .iter()
            .map(|&i| changes.get_indexed(i).0.clone())
            .collect();
        assert_eq!(actual, keys);
        assert!(!actual.contains(&removed));
    }
    assert!(changes.get(&removed).unwrap().snapshot.is_none());
    // Serialization must contain stable records only.
    let json = serde_json::to_value(&changes).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 1);
    assert!(json.get("records").is_some());
}

#[test]
#[ignore = "requires the installed source export"]
fn replay_selector_refreshes_set_replay_forget_and_removal() {
    use super::super::ObjectKey;
    let reference = reference();
    let mut stream = WorldStream::new(&reference, 42).unwrap();
    let source = &reference.world.road.prototypes[0];
    let radius = f64::from(stream.reference.distances["ItemSpawnDist"]);
    stream.request.vehicle = [radius, 0., 0.];
    let baseline = replay_snapshot([radius * 20.; 3]);
    let deleted = ObjectKey::Generated(replay_placement(source, 1, baseline.pose.position).id);
    stream.changes.record(
        deleted.clone(),
        replay_snapshot([radius * 10.; 3]),
        &baseline,
    );
    let imported = ObjectKey::Imported(crate::reference::ItemId(999999));
    stream.changes.record(
        imported.clone(),
        replay_snapshot([-radius, 9000., -radius]),
        &baseline,
    );
    let player = replay_placement(source, 2, [radius * 0.5, 0., 0.]);
    let vehicle = replay_placement(source, 3, [radius * 1.5, 0., 0.]);
    let last = replay_placement(source, 4, [0.; 3]);
    let player_key = ObjectKey::Generated(player.id.clone());
    let vehicle_key = ObjectKey::Generated(vehicle.id.clone());
    let last_key = ObjectKey::Generated(last.id.clone());
    stream.changes.record(
        player_key.clone(),
        replay_snapshot(player.pose.position),
        &baseline,
    );
    stream.changes.record(
        vehicle_key.clone(),
        replay_snapshot(vehicle.pose.position),
        &baseline,
    );
    let mut attached = replay_snapshot(last.pose.position);
    attached.attachment = Some(super::super::AttachmentSnapshot {
        owner: Some(ObjectKey::Generated(vehicle.id.clone())),
        slot: None,
        local_pose: last.pose,
    });
    stream.changes.record(last_key.clone(), attached, &baseline);
    stream.select_changed_destinations();
    assert_replay_selection(&stream);
    assert_eq!(old_replay_keys(&stream).last(), Some(&vehicle_key));
    assert_eq!(
        old_replay_keys(&stream)
            .iter()
            .filter(|k| **k == player_key)
            .count(),
        1
    );
    assert!(old_replay_keys(&stream).contains(&imported));
    // The last selected record moves into an unselected deleted slot.
    // Removing the selector revision check makes this selection assertion fail.
    stream
        .changes
        .record(deleted.clone(), baseline.clone(), &baseline);
    assert!(stream.changes.get(&deleted).is_none());
    let _ = stream.metrics(); // Metrics must not dereference the stale selection.
    stream.select_changed_destinations();
    assert_replay_selection(&stream);
    let revision = stream.change_revision;
    stream.changes.set_replay(&player_key, &player, &player.id);
    stream.changes.set_replay(&last_key, &last, &last.id);
    stream.initial_generation_ready = true;
    stream.select_changed_destinations();
    assert_ne!(stream.change_revision, revision);
    assert_replay_selection(&stream);
    let first = old_replay_keys(&stream)
        .iter()
        .find_map(|key| stream.changes.get(key).unwrap().replay.clone())
        .unwrap();
    assert_eq!(stream.replay_roots, vec![first]);
    let capacities = (
        stream.replay_indices.capacity(),
        stream.replay_query.capacity(),
    );
    stream.select_changed_destinations();
    assert_eq!(
        capacities,
        (
            stream.replay_indices.capacity(),
            stream.replay_query.capacity()
        )
    );
    assert_eq!(stream.replay_roots.len(), 1);
    let (_, selected) = stream
        .replay_indices
        .iter()
        .map(|&i| stream.changes.get_indexed(i))
        .find(|(key, _)| **key == last_key)
        .unwrap();
    assert_eq!(selected.instance.as_ref(), Some(&last.id));
    assert_eq!(
        selected
            .snapshot
            .as_ref()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .owner,
        Some(ObjectKey::Generated(vehicle.id.clone()))
    );
    stream.changes.remove(player_key.clone());
    stream.select_changed_destinations();
    assert_replay_selection(&stream);
    assert!(stream.changes.get(&player_key).unwrap().snapshot.is_none());
    assert_eq!(stream.replay_roots, vec![last]);
    assert!(stream.failure.is_none(), "{:?}", stream.failure);
}

#[test]
#[ignore = "requires the installed source export"]
fn replay_predicates_match_old_keys_for_instances_owners_and_removals() {
    use super::super::ObjectKey;
    let reference = reference();
    let mut stream = WorldStream::new(&reference, 42).unwrap();
    let source = &reference.world.road.prototypes[0];
    let far = f64::from(stream.reference.distances["ItemSpawnDist"]) * 10.;
    let placements: Vec<_> = (1..=6)
        .map(|i| replay_placement(source, i, [far, 0., 0.]))
        .collect();
    let baseline = replay_snapshot([far * 2.; 3]);
    let leaf = ObjectKey::Generated(replay_placement(source, 10, [0.; 3]).id);
    stream
        .changes
        .record(leaf.clone(), replay_snapshot([0.; 3]), &baseline);
    stream
        .changes
        .set_replay(&leaf, &placements[0], &placements[0].id);
    let mut attached = replay_snapshot([0.; 3]);
    attached.attachment = Some(super::super::AttachmentSnapshot {
        owner: Some(ObjectKey::Generated(placements[1].id.clone())),
        slot: None,
        local_pose: placements[1].pose,
    });
    stream.changes.record(
        ObjectKey::Imported(crate::reference::ItemId(999999)),
        attached,
        &baseline,
    );
    let removed = ObjectKey::Generated(placements[2].id.clone());
    stream
        .changes
        .record(removed.clone(), replay_snapshot([0.; 3]), &baseline);
    stream
        .changes
        .set_replay(&removed, &placements[2], &placements[2].id);
    stream.changes.remove(removed);
    // A changed destination outside both windows must not retain its source instance.
    let outside = ObjectKey::Generated(placements[4].id.clone());
    stream
        .changes
        .record(outside.clone(), replay_snapshot([far; 3]), &baseline);
    stream
        .changes
        .set_replay(&outside, &placements[4], &placements[4].id);
    let direct = ObjectKey::Generated(placements[5].id.clone());
    stream
        .changes
        .record(direct.clone(), replay_snapshot([0.; 3]), &baseline);
    stream
        .changes
        .set_replay(&direct, &placements[5], &placements[5].id);
    for placement in &placements {
        stream
            .contents
            .active
            .push(super::super::contents::ContentInstance::for_structure(
                placement.clone(),
            ));
    }
    let keys = old_replay_keys(&stream);
    let expected: Vec<_> = placements
        .iter()
        .filter(|p| {
            let selected = keys.iter().any(|key| {
                stream.changes.get(key).is_some_and(|c| {
                    c.instance.as_ref() == Some(&p.id)
                        || c.snapshot
                            .as_ref()
                            .and_then(|s| s.attachment.as_ref())
                            .and_then(|a| a.owner.as_ref())
                            == Some(&ObjectKey::Generated(p.id.clone()))
                })
            });
            stream
                .changes
                .get(ObjectKey::Generated(p.id.clone()))
                .is_none_or(|c| c.snapshot.is_some() && selected)
                && selected
        })
        .cloned()
        .collect();
    assert_eq!(
        expected,
        vec![
            placements[0].clone(),
            placements[1].clone(),
            placements[5].clone()
        ]
    );
    let history = stream.contents.state.capture_history();
    stream.prepare_contents();
    assert_eq!(stream.wanted_structures, expected);
    // Existing unbound distant placements exercise the earlier retention predicate too.
    stream.prepare_contents();
    assert_eq!(stream.wanted_structures, expected);
    assert_eq!(stream.contents.state.capture_history(), history);
    assert!(stream.failure.is_none(), "{:?}", stream.failure);
}

#[test]
#[ignore = "requires the installed source export"]
fn replay_restore_replaces_selection_and_saved_readiness_checks_revision() {
    use super::super::{ChangeRecords, ObjectKey};
    let reference = reference();
    let baseline = replay_snapshot([9000.; 3]);
    let old = ObjectKey::Imported(crate::reference::ItemId(999998));
    let replacement = ObjectKey::Imported(crate::reference::ItemId(999999));
    let mut candidate = WorldStream::new(&reference, 42).unwrap();
    candidate
        .changes
        .record(old.clone(), replay_snapshot([0.; 3]), &baseline);
    candidate.select_changed_destinations();
    let mut saved = crate::save::WorldSnapshot {
        generation: super::super::GenerationSnapshot {
            generator_contract: super::super::GENERATOR_CONTRACT,
            package_id: candidate.package_id.clone(),
            roads: candidate.roads.capture_index(),
            placements: candidate.placements.capture_index(),
        },
        seed: 42,
        changes: ChangeRecords::default(),
        spawn: candidate.contents.state.capture_history(),
    };
    saved
        .changes
        .record(replacement.clone(), replay_snapshot([0.; 3]), &baseline);
    assert_eq!(candidate.changes.revision(), saved.changes.revision());
    candidate.restore_save(&saved).unwrap();
    assert!(candidate.change_bounds.is_none());
    assert_eq!(candidate.change_revision, u64::MAX);
    let _ = candidate.metrics();
    candidate.select_changed_destinations();
    assert_replay_selection(&candidate);
    assert_eq!(old_replay_keys(&candidate), vec![replacement.clone()]);
    // The active stream owns its selection while the candidate is replaced and dropped.
    let mut run = GarageRuntime::new(&reference).unwrap();
    prepare_ready(&mut run, &reference);
    mechanism_binding_gate(&mut run, &reference);
    let owner = run
        .stream
        .static_controls
        .iter()
        .find(|c| c.restored && c.item.is_some())
        .unwrap();
    let item = owner.item.unwrap();
    let origin = owner.origin.clone();
    let instance = owner.content.placement.id.clone();
    let generated = run.area.items.object_key(item);
    assert!(matches!(&generated, ObjectKey::Generated(_)));
    let original = super::super::changes::capture(
        item,
        &run.area.items,
        &run.area.interaction,
        &run.physics,
        &reference,
    )
    .unwrap();
    let mut changed = original.clone();
    changed.condition = if original.condition < 0.5 { 0.75 } else { 0.25 };
    run.stream
        .changes
        .record(generated.clone(), changed, &original);
    run.stream
        .changes
        .set_replay(&generated, &origin, &instance);
    run.stream
        .static_controls
        .iter_mut()
        .find(|c| c.item == Some(item))
        .unwrap()
        .restored = false;
    run.stream.select_changed_destinations();
    assert!(old_replay_keys(&run.stream).contains(&generated));
    assert!(!run.stream.saved_bindings_ready(&run.area.items));
    run.stream
        .sync_supplies(
            &reference,
            &mut run.area.items,
            &mut run.area.interaction,
            &mut run.physics,
            &mut run.wheels,
        )
        .unwrap();
    assert_eq!(run.area.items.resolve_key(&generated), Some(item));
    assert!(
        run.stream
            .static_controls
            .iter()
            .any(|c| c.item == Some(item) && c.restored)
    );
    assert!(run.stream.saved_bindings_ready(&run.area.items));
    run.stream
        .changes
        .record(generated, original.clone(), &original);
    run.stream.select_changed_destinations();
    run.stream.changes.record(
        old.clone(),
        replay_snapshot(run.stream.request.player),
        &baseline,
    );
    assert!(run.stream.required_ready());
    assert!(!run.stream.saved_bindings_ready(&run.area.items));
    run.stream.select_changed_destinations();
    assert!(run.stream.saved_bindings_ready(&run.area.items));
    drop(candidate);
    saved.changes.remove(replacement);
    assert_eq!(old_replay_keys(&run.stream), vec![old]);
    assert!(run.stream.saved_bindings_ready(&run.area.items));
    let missing = ObjectKey::Generated(
        replay_placement(
            &reference.world.road.prototypes[0],
            900000,
            run.stream.request.player,
        )
        .id,
    );
    run.stream.changes.record(
        missing.clone(),
        replay_snapshot(run.stream.request.player),
        &baseline,
    );
    assert!(!run.stream.saved_bindings_ready(&run.area.items));
    run.stream.select_changed_destinations();
    assert!(!run.stream.saved_bindings_ready(&run.area.items));
    run.stream.changes.remove(missing);
    assert!(!run.stream.saved_bindings_ready(&run.area.items));
    run.stream.select_changed_destinations();
    assert!(run.stream.saved_bindings_ready(&run.area.items));
}
