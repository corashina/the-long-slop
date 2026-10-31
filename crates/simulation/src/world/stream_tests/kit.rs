//! Test kit for headless scenario tests on the installed source export.
//! Run in release only. Debug builds overflow the stack.
//! The world uses the default game seed and the export date. Each frame is one fixed step (`FIXED_DT`).
//! Section sweeps use the whole kit. Some helpers have no caller yet.
#![allow(dead_code)]
use super::super::*;
use crate::audio::AudioEvent;
use crate::reference::ItemId;
use crate::sandbox::Sandbox;
use crate::save::{SaveSnapshot, SourceStamp};
use crate::simulation::{FIXED_DT, FrameInput};
use crate::starting_area::GarageRuntime;
use rapier3d::math::{Rotation, Vector};
use serde_json::Value;

/// Path of the installed export.
const MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../web/assets/reference/manifest.json"
);

thread_local! {
    /// Surface renderer claim kept across frames, as the WASM presentation keeps it.
    static CLAIM: std::cell::RefCell<Option<(WorldRequest, SurfaceKey)>> = const { std::cell::RefCell::new(None) };
    /// Sandbox state across `act` calls, as the `?sandbox` page keeps it.
    static SANDBOX: std::cell::RefCell<Sandbox> = std::cell::RefCell::new(Sandbox::default());
}

pub(crate) fn reference() -> ReferenceManifest {
    ReferenceManifest::from_runtime_json(&std::fs::read(MANIFEST).unwrap()).unwrap()
}

/// The save stamp of the installed export.
pub(crate) fn roadside_stamp(reference: &ReferenceManifest) -> SourceStamp {
    let digest = std::process::Command::new("sha256sum")
        .arg(MANIFEST)
        .output()
        .unwrap();
    assert!(digest.status.success());
    SourceStamp {
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
            .map(|h| crate::save::SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    }
}

/// The native adapter charges and hands over one structure claim, as the renderer ledger does.
pub(crate) fn accept_structure(stream: &mut WorldStream, id: &crate::world::GeneratedId) {
    stream.presentation.pending += 1;
    stream
        .accept_structure_renderer(stream.request, id)
        .unwrap();
}

pub(crate) fn views() -> [crate::world::RequiredView; 1] {
    let planes = [
        [1., 0., 0., 150.],
        [-1., 0., 0., 150.],
        [0., 1., 0., 150.],
        [0., -1., 0., 150.],
        [0., 0., 1., 150.],
        [0., 0., -1., 150.],
    ];
    [crate::world::RequiredView {
        camera: planes,
        shadow: crate::reference::shadow_candidates::DirectionalHull::new(planes, [1., 0., 0.])
            .unwrap(),
        mask: u32::MAX,
        light_mask: u32::MAX,
        origin: [0.; 3],
    }]
}

pub(crate) fn prepare_step(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    surface_claim: &mut Option<(WorldRequest, SurfaceKey)>,
) {
    prepare_step_for_views(run, reference, surface_claim, &views());
}

pub(crate) fn prepare_step_for_views(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    surface_claim: &mut Option<(WorldRequest, SurfaceKey)>,
    required_views: &[crate::world::RequiredView],
) {
    let stream = &mut run.stream;
    stream.prepare(100.);
    if stream
        .ready
        .as_ref()
        .is_some_and(|w| !same_request(w.request, stream.request))
    {
        if let Some((request, key)) = surface_claim.take() {
            stream.discard_surface_renderer(request, key).unwrap();
        }
    }
    stream.select_required(required_views);
    let sources: Vec<_> = reference
        .world
        .terrain
        .materials
        .iter()
        .map(|m| m.source.clone())
        .chain(reference.world.road.prototypes.iter().cloned())
        .collect();
    for source in sources {
        stream.hold_prototype(source.clone()).unwrap();
        stream.bind_prototype(&source).unwrap();
    }
    let placement = stream
        .wanted_structures
        .iter()
        .find(|p| {
            (stream.structure_required(&p.id)
                || stream.structure_replay_required(&p.id)
                || stream.lights.iter().any(|light| {
                    stream.publication_light(light, &stream.required.lights)
                        && light.owned_by(&p.id)
                }))
                && stream.contents_initialized(&p.id)
                && !stream.structures.iter().any(|s| s.placement.id == p.id)
                && !stream.ready_structures.contains(&p.id)
        })
        .cloned()
        // The native adapter reserves one instance claim and no grass child job.
        .filter(|_| stream.can_request_independent(1));
    if let Some(p) = placement {
        stream.hold_prototype(p.prototype.clone()).unwrap();
        stream.bind_prototype(&p.prototype).unwrap();
        stream.bind_structure_recipe(&p.prototype).unwrap();
        accept_structure(stream, &p.id);
    }
    if let Some((request, surface)) = stream.prepared_surface() {
        let key = surface.key;
        if !stream.ready.as_ref().unwrap().renderer_reserved {
            stream.reserve_surface_renderer(request, key).unwrap();
            stream.accept_surface_renderer(request, key).unwrap();
            *surface_claim = Some((request, key));
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
            &mut run.vehicles,
            &mut run.powertrain,
        )
        .unwrap();
    assert!(stream.failure.is_none(), "{:?}", stream.failure);
    run.align_startup(reference, [1, 1]).unwrap();
}

/// Publishes until the required world is ready. The run does not step.
pub(crate) fn prepare_ready(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    let mut surface_claim = None;
    for step in 0..2000 {
        prepare_step(run, reference, &mut surface_claim);
        if run.stream.required_ready() {
            if let Some((request, key)) = surface_claim.take() {
                assert!(!run.stream.required.terrain.contains(&key));
                assert!(
                    key.layer != WorldLayer::Road
                        || !run.stream.required.roads.contains(&(key.unit[0] as u32))
                );
                run.stream.discard_surface_renderer(request, key).unwrap();
            }
            return;
        }
        if step == 1999 {
            println!("stalled: {}", run.stream.startup_job_audit());
        }
    }
    panic!("required publication did not reach readiness");
}

/// Sets `run.vehicle` to the start car copy. Only the start car spawns on the `Startup` layer.
/// The game sets `run.vehicle` on the first seat. Tests drive the start car before that.
pub(crate) fn start_vehicle(run: &mut GarageRuntime) {
    let key = run
        .vehicles
        .keys()
        .find(|key| {
            matches!(key, crate::world::ObjectKey::Generated(id) if id.layer == WorldLayer::Startup)
        })
        .cloned()
        .expect("start car after the required publication");
    run.vehicle = Some(key);
}

/// A new game with the required world ready and the start car as `run.vehicle`. Not started.
pub(crate) fn ready(reference: &ReferenceManifest) -> GarageRuntime {
    let mut run = GarageRuntime::new_default(reference).unwrap();
    prepare_ready(&mut run, reference);
    start_vehicle(&mut run);
    run
}

/// A started new game at the garage spawn.
pub(crate) fn load_garage() -> (ReferenceManifest, GarageRuntime) {
    let reference = reference();
    assert_eq!(reference.build_id, 11140890);
    let mut run = ready(&reference);
    assert!(run.world_supported());
    run.started = true;
    run.paused = false;
    (reference, run)
}

/// One frame without input.
pub(crate) fn frame(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    frame_with(run, reference, FrameInput::default());
}

/// One frame: publication around the player focus, then one fixed step with `input`.
pub(crate) fn frame_with(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    input: FrameInput,
) {
    let mut claim = CLAIM.take();
    let focus = run
        .stream
        .origin
        .absolute(run.area.player.focus_position(&run.physics));
    let mut view = views();
    view[0].origin = focus;
    prepare_step_for_views(run, reference, &mut claim, &view);
    CLAIM.set(claim);
    run.step(reference, input);
    assert!(
        run.stream.frame().failure.is_none(),
        "{:?}",
        run.stream.frame().failure
    );
}

/// Frames for `seconds` of simulation time.
pub(crate) fn seconds(run: &mut GarageRuntime, reference: &ReferenceManifest, seconds: f32) {
    for _ in 0..(seconds / FIXED_DT) as usize {
        frame(run, reference);
    }
}

/// Frames until the player stands on the ground. At most 240 frames.
pub(crate) fn settle(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    for _ in 0..240 {
        if run.area.player.grounded() {
            return;
        }
        frame(run, reference);
    }
    panic!("player stays airborne");
}

/// Looks at `yaw` degrees right of the body and `pitch` degrees up.
pub(crate) fn look(run: &mut GarageRuntime, yaw: f32, pitch: f32) {
    let player = &mut run.area.player;
    let head = player.view(&run.physics).head_pose;
    let rotation = Rotation::from_array(player.body_pose(&run.physics).rotation)
        * Rotation::from_rotation_y(-yaw.to_radians())
        * Rotation::from_rotation_x(pitch.to_radians());
    let target = Vector::from_array(head.position) + rotation * -Vector::Z;
    player.look_at(target.to_array(), &run.physics);
}

/// Runs one sandbox action, such as `{"action":"seat","seat":"driver"}`.
/// Asserts `ok` and returns the reply `data`.
pub(crate) fn act(run: &mut GarageRuntime, reference: &ReferenceManifest, json: &str) -> Value {
    let reply = SANDBOX.with_borrow_mut(|sandbox| {
        sandbox.replies.clear();
        sandbox.queued.push(format!("[{json}]"));
        crate::sandbox::run(run, reference, sandbox);
        assert_eq!(sandbox.replies.len(), 1, "{json}: {:?}", sandbox.replies);
        sandbox.replies.pop().unwrap()
    });
    let mut reply: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(reply["ok"], true, "{json}: {}", reply["message"]);
    reply["data"].take()
}

/// The save of `run` after a JSON round trip, as the browser stores it.
pub(crate) fn save(run: &GarageRuntime, reference: &ReferenceManifest) -> SaveSnapshot {
    let saved = run
        .capture_save(
            reference,
            &roadside_stamp(reference),
            "2026-10-29T12:00:00Z",
        )
        .unwrap();
    serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap()
}

/// A runtime from `saved`. The restore is not done yet: call `restore`.
pub(crate) fn open(reference: &ReferenceManifest, saved: &SaveSnapshot) -> GarageRuntime {
    GarageRuntime::from_save(
        reference,
        &roadside_stamp(reference),
        saved,
        crate::world::GameMode::default(),
        reference.world.draw_distance.default,
    )
    .unwrap()
}

/// Publishes until the saved restore is done and `done` is true. Then starts the run.
pub(crate) fn restore(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    done: impl Fn(&GarageRuntime) -> bool,
) {
    for _ in 0..2000 {
        prepare_ready(run, reference);
        if run.advance_saved_restore(reference).unwrap() && !run.restore_pending() && done(run) {
            break;
        }
    }
    assert!(!run.restore_pending(), "restore stays pending");
    run.started = true;
    run.paused = false;
}

/// A started runtime from `saved` with the restore done.
pub(crate) fn load(reference: &ReferenceManifest, saved: &SaveSnapshot) -> GarageRuntime {
    let mut run = open(reference, saved);
    restore(&mut run, reference, |_| true);
    run
}

/// Saves and loads `run` the way the browser does.
pub(crate) fn save_load(run: &GarageRuntime, reference: &ReferenceManifest) -> GarageRuntime {
    load(reference, &save(run, reference))
}

/// Takes the queued audio events with their trigger names. The queue holds the last 64.
pub(crate) fn audio_events(run: &mut GarageRuntime) -> Vec<(String, AudioEvent)> {
    let audio = run.audio.as_mut().expect("audio state");
    let mut events = Vec::new();
    while let Some(event) = audio.pop_event() {
        let trigger = audio
            .binding(event.binding)
            .map_or_else(String::new, |b| b.trigger.clone());
        events.push((trigger, event));
    }
    events
}

/// The nearest item to the player whose name contains `name`, ignoring case.
pub(crate) fn find(run: &GarageRuntime, name: &str) -> ItemId {
    let player = Vector::from_array(run.area.player.body_pose(&run.physics).position);
    let name = name.to_lowercase();
    run.area
        .items
        .iter()
        .filter(|i| !i.removed)
        .filter(|i| {
            run.area
                .items
                .name(i.id)
                .is_some_and(|n| n.to_lowercase().contains(&name))
        })
        .min_by(|a, b| {
            let a = (Vector::from_array(a.pose.position) - player).length();
            let b = (Vector::from_array(b.pose.position) - player).length();
            a.total_cmp(&b)
        })
        .map(|i| i.id)
        .unwrap_or_else(|| panic!("no item named {name}"))
}

/// Spawns the first world prototype with each script class in front of the player. Returns
/// the placements once each binds.
pub(crate) fn spawn_scripts(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    classes: &[&str],
) -> Vec<crate::world::GeneratedId> {
    let rules = &run.stream.contents.rules;
    let sources: Vec<_> = classes
        .iter()
        .map(|class| {
            reference
                .world
                .prototypes
                .iter()
                .find(|p| {
                    rules
                        .component(&reference.world, &p.source, class)
                        .ok()
                        .flatten()
                        .is_some()
                })
                .unwrap_or_else(|| panic!("{class} prototype"))
                .source
                .clone()
        })
        .collect();
    let view = run.area.player.view(&run.physics).camera_pose;
    let rotation = rapier3d::math::Rotation::from_array(view.rotation);
    let forward = rotation * -rapier3d::math::Vector::Z;
    let right = rotation * rapier3d::math::Vector::X;
    for (i, source) in sources.iter().enumerate() {
        let p = rapier3d::math::Vector::from_array(view.position)
            + forward.with_y(0.).normalize() * 0.8
            + right.with_y(0.).normalize() * (0.4 * (i as f32 - 1.));
        let position = run
            .stream
            .origin
            .absolute((p - rapier3d::math::Vector::Y * 0.6).to_array());
        run.stream
            .add_startup_placement(
                source,
                crate::world::AbsolutePose {
                    position,
                    rotation: [0., 0., 0., 1.],
                },
            )
            .unwrap();
    }
    let parent = |run: &GarageRuntime, source: &crate::reference::SourceId| {
        run.stream
            .contents
            .active
            .iter()
            .find(|c| &c.content.placement.prototype == source)
            .map(|c| c.content.placement.id.clone())
    };
    let bound = |run: &GarageRuntime, class: &str, p: &crate::world::GeneratedId| match class {
        "musicboxscript" => run.stream.music_box(p).is_some(),
        _ => run.stream.dial(p).is_some(),
    };
    for _ in 0..600 {
        frame(run, reference);
        if sources
            .iter()
            .zip(classes)
            .all(|(s, c)| parent(run, s).is_some_and(|p| bound(run, c, &p)))
        {
            seconds(run, reference, 1.);
            return sources.iter().map(|s| parent(run, s).unwrap()).collect();
        }
    }
    panic!("{classes:?} do not bind");
}

/// The item of an active world content.
pub(crate) fn content_item(run: &GarageRuntime, id: &crate::world::GeneratedId) -> Option<ItemId> {
    run.stream
        .contents
        .active
        .iter()
        .find(|c| &c.content.placement.id == id)
        .and_then(|c| c.item)
}

/// Gives a test collider a trigger role, as `trigger_roles` publishes it.
pub(crate) fn add_trigger_role(
    run: &mut GarageRuntime,
    collider: rapier3d::prelude::ColliderHandle,
    role: crate::reference::TriggerRole,
) {
    run.stream.trigger_roles.insert(collider, role);
}
