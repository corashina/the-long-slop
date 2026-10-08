# Desert Driving WASM Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved desert-driving recreation as a local Rust/WASM browser game.

**Architecture:** Bevy renders the scene. Rapier owns the physics world and four raycast wheels. Rust modules own controls, fuel, terrain, and trip state.

**Tech Stack:** Rust stable, Bevy 0.19.1, rapier3d 0.36.0, wasm-bindgen, Docker, static localhost server.

**Spec:** `docs/superpowers/specs/2026-10-08-desert-driving-wasm-design.md`, approved in chat on 2026-10-08.

## Global Constraints

- Target `wasm32-unknown-unknown`; use WebGL2 and one simulation thread.
- Run physics at a fixed 60 Hz with render interpolation.
- Allow refueling only below 1 km/h and within 6 meters of the station point.
- Pause must freeze fuel, physics, and trip distance. Clear input on focus loss.
- Reset must preserve fuel and distance. At zero fuel, reset places the car at a station.
- Use procedural assets. Keep Steam files outside the build and distribution.
- Keep world chunks bounded. Rebase physics, scenery, cameras, and interpolation poses together.
- Follow `AGENTS.md`: use established libraries, avoid per-frame allocation, and implement critical tests only.
- Follow the stop-slop skill for UI text and documentation. Use short instructions.
- Implement the spec's scope. Leave walking, inventory, multiplayer, persistence, and asset import for separate work.
- Preserve the existing untracked `AGENTS.md`; exclude it from implementation commits unless the user requests inclusion.

## Review Focus

- Tab focus loss while steering: clear input and require resume. Verify in Task 4.
- Stalls and resume: clamp elapsed time and avoid a long catch-up drive. Verify in Task 2 and Task 5.
- Reset or rebase: exclude teleports from trip distance and preserve fuel. Verify in Task 3 and Task 4.
- Empty fuel near an unloaded station: load the reset destination before resuming physics. Verify in Task 4.
- Missing WASM or graphics support: show a readable error and disable Start. Verify in Task 1 and Task 5.

## File Map

| File | Responsibility |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | Rust package and pinned dependencies |
| `src/lib.rs` | Expose simulation, vehicle, and world modules for native tests |
| `src/main.rs`, `src/app.rs` | WASM entry, Bevy setup, and ordered systems |
| `src/simulation.rs` | Input, fuel, drive state, and trip rules |
| `src/vehicle.rs` | Rapier world, vehicle controller, body poses, and collider lifecycle |
| `src/world.rs` | Seeded road/terrain samples, chunk data, stations, and origin |
| `src/presentation/mod.rs` | Scene assets, streamed chunk entities, and pose synchronization |
| `src/presentation/car.rs` | Procedural car and wheel/cockpit geometry |
| `src/presentation/camera.rs` | Cockpit and chase cameras |
| `src/browser.rs` | Browser input, focus, UI state, and optional synthesized sound |
| `web/index.html`, `web/style.css`, `web/main.js` | Loading/start screen, HUD, pause controls, and WASM loader |
| `scripts/build.ps1`, `scripts/check.ps1`, `scripts/serve.mjs` | Build, checks, and loopback server |
| `tools/Dockerfile`, `tools/rust-image.txt` | Container toolchain and immutable base-image reference |
| `tests/vehicle.rs`, `tests/world.rs`, `tests/simulation.rs` | Critical native behavior tests |
| `docs/verification.md`, `README.md` | Measured checks and launch instructions |

## Build Decisions

Docker 29.8.1 responds on this machine. No usable Rust/MSVC host toolchain was found. Use Docker for compilation and native tests.

Resolve the official `rust:latest` image to its repository digest during Task 1. Record the digest and compiler version. Use that digest in the Dockerfile. Install the WASM target, rustfmt, and a wasm-bindgen CLI version matching the resolved crate version. Pin the Rust crate dependencies with `Cargo.lock`.

Use `rapier3d = "=0.36.0"` without parallel features. Use `bevy = "=0.19.1"` only for the WASM target. Select required rendering/window/input features from its published manifest. Disable default features, audio decoding, gamepads, and engine multithreading. Enable WebGL2 and avoid WebGPU.

Keep Rapier in the common library for native physics tests. Convert its vector/quaternion values into plain arrays at the rendering boundary. This avoids a second Bevy physics scheduler.

Build scripts mount this workspace as `/workspace`. Use project-specific Docker cache volumes for Cargo and target files. Copy the finished WASM/glue to `web/pkg`. Bind the static server to `127.0.0.1:4173`. Exclude `.git`, caches, and output from Docker context and Git commits.

Use the documented Rapier 0.36 API. Its vehicle controller accepts `QueryPipelineMut`; older examples use a different signature. Inspect the pinned source before integrating it.

Sources: [Bevy 0.19.1](https://docs.rs/bevy/0.19.1/bevy/), [Rapier 0.36 vehicle API](https://docs.rs/rapier3d/0.36.0/rapier3d/control/struct.DynamicRayCastVehicleController.html).

## Task 1: A Rendered Browser Build

**Files:** Cargo files, `src/lib.rs`, `src/main.rs`, `src/app.rs`, web shell, scripts, tools, `.gitignore`, `.dockerignore`, `README.md`.

**Interfaces:** `app::run()` starts Bevy on canvas `#game`. The loader exposes Start only after Rust signals readiness. Build/check scripts return nonzero on failure.

- [ ] Resolve and record the official Rust image digest. Build the project toolchain container.
- [ ] Create the Rust package with common Rapier and WASM-only Bevy/browser dependencies.
- [ ] Create a lit ground plane, camera, and visible cube. Connect rendering to `#game`.
- [ ] Create the web loader, loading state, Start control, and readable initialization errors.
- [ ] Implement `scripts/build.ps1`, `scripts/check.ps1`, and `scripts/serve.mjs` with propagated exit codes and correct WASM MIME type.
- [ ] Run `./scripts/build.ps1` and `node scripts/serve.mjs`. Open `http://127.0.0.1:4173` and verify rendered geometry.
- [ ] Temporarily request a missing WASM URL. Verify an error and disabled Start; restore the valid URL.
- [ ] Commit this working layer as `feat: establish Rust WASM browser build`.

This task needs a real build/load check. A unit test of static HTML text would not verify it.

## Task 2: Drive a Physical Car on a Static Track

**Files:** `src/simulation.rs`, `src/vehicle.rs`, car/camera presentation, app systems, `tests/vehicle.rs`.

**Interfaces:**

- `DriveInput { accelerate: bool, brake_reverse: bool, steer: f32, handbrake: bool }`, with a cleared default.
- `VehiclePose { position: [f32; 3], rotation: [f32; 4] }` and `VehicleSnapshot { pose: VehiclePose, wheels: [VehiclePose; 4], speed_mps: f32, linear_velocity: [f32; 3], angular_velocity: [f32; 3], wheels_contact: [bool; 4] }`.
- `PhysicsWorld::new(pose: VehiclePose) -> Self` creates the world, chassis, and wheels.
- `PhysicsWorld::step(&mut self, input: DriveInput, engine_enabled: bool)` advances exactly 1/60 second.
- `PhysicsWorld::snapshot(&self) -> VehicleSnapshot` reads current state without allocation.
- `PhysicsWorld::reset(&mut self, pose: VehiclePose)` clears velocities and wheel transient state.
- `PhysicsWorld::insert_mesh(&mut self, vertices: &[[f32; 3]], indices: &[[u32; 3]]) -> ColliderHandle` adds static geometry.
- `PhysicsWorld::remove_collider(&mut self, handle: ColliderHandle)` removes it from physics/query state.
- `CameraMode::{Cockpit, Chase}` selects the camera behavior.

- [ ] Write `tests/vehicle.rs`: settle on flat ground, accelerate, brake, reverse, and reset. Assert finite poses, support, speed change, and zero reset velocities.

Define the test helper `flat_track() -> PhysicsWorld` using `new` and `insert_mesh`. Settle the car for 120 steps.

```rust
#[test]
fn reset_stops_the_chassis() {
    let mut world = flat_track();
    let start = world.snapshot().pose;
    let input = DriveInput { accelerate: true, ..DriveInput::default() };
    for _ in 0..180 { world.step(input, true); }
    assert!(world.snapshot().speed_mps.abs() > 1.0);
    world.reset(start);
    assert_eq!(world.snapshot().linear_velocity, [0.0; 3]);
    assert_eq!(world.snapshot().angular_velocity, [0.0; 3]);
}
```
- [ ] Run `./scripts/check.ps1 -Test vehicle`. Confirm the missing interfaces fail before implementing them.
- [ ] Implement one dynamic chassis and four raycast wheels using Rapier. Exclude the chassis from wheel queries.
- [ ] Tune mass, suspension, traction, steering, braking, and reverse on the static track. Use meters, seconds, and kilograms.
- [ ] Add the procedural compact car, wheel transforms, cockpit, and both cameras.
- [ ] Order input, fixed physics, interpolation, camera, and HUD systems. Cap elapsed frame time at 100 ms.
- [ ] Run the vehicle tests and WASM build. Play-test driving, handbrake, collision, and camera changes.
- [ ] Verify a simulated one-second stall cannot advance one second of physics in one frame.
- [ ] Commit as `feat: add raycast vehicle driving and cameras`.

## Task 3: Stream the Desert Road

**Files:** `src/world.rs`, world presentation, vehicle collider management, app streaming, `tests/world.rs`.

**Interfaces:**

- `WorldOrigin { offset: [f64; 3] }` tracks the absolute origin.
- `ChunkKey(i64)` identifies a 128-meter longitudinal road section.
- `RoadSample { center: [f64; 3], tangent: [f32; 3] }` describes an absolute road point.
- `road_sample(seed: u64, distance_m: f64) -> RoadSample` defines shared samples for terrain, road, and reset.
- `ChunkData { key: ChunkKey, vertices: Vec<[f32; 3]>, indices: Vec<[u32; 3]>, stations: Vec<Station> }` stores generated chunk-local geometry.
- `Station { id: i64, absolute_point: [f64; 3] }` identifies a refueling point.
- `generate_chunk(seed: u64, key: ChunkKey) -> ChunkData` allocates only during chunk generation.
- `active_chunks(distance_m: f64) -> [ChunkKey; 11]` selects three behind and seven ahead, plus the current chunk.
- `nearest_station(seed: u64, distance_m: f64) -> Station` works without a retained world history.
- `PhysicsWorld::rebase(&mut self, shift: [f32; 3])` shifts local physics poses while preserving velocities.

- [ ] Write seam/determinism tests using adjacent chunks from the same seed. Assert matching boundary positions within 0.001 meters.

Derive `PartialEq` for chunk data and stations. In tests, define `seam_vertices(chunk: &ChunkData, end: bool) -> Vec<[f64; 3]>`.
The helper selects boundary vertices, converts them to absolute coordinates, and sorts them by position.

```rust
#[test]
fn adjacent_chunks_share_the_seam() {
    let a = generate_chunk(42, ChunkKey(0));
    let b = generate_chunk(42, ChunkKey(1));
    assert_eq!(a, generate_chunk(42, ChunkKey(0)));
    let left = seam_vertices(&a, true);
    let right = seam_vertices(&b, false);
    assert!(!left.is_empty());
    assert_eq!(left.len(), right.len());
    for (p, q) in left.iter().zip(&right) {
        for axis in 0..3 { assert!((p[axis] - q[axis]).abs() < 0.001); }
    }
}
```
- [ ] Write a rebase test: compare relative wheel/chassis poses and velocities before and after a 4096-meter shift.
- [ ] Run `./scripts/check.ps1 -Test world`. Confirm the tests fail before implementing the missing behavior.
- [ ] Generate a gently curved road, dunes, sparse props, and stations every 640 meters. Use global samples at shared edges.
- [ ] Stream eleven active chunks. Remove physics colliders and entities when unloading; reuse shared meshes/materials where possible.
- [ ] Rebase above 2048 local meters between physics steps. Shift physics, chunk entities, interpolation buffers, and cameras together.
- [ ] Load station/reset destination chunks before moving the chassis there. Reset interpolation poses after a teleport.
- [ ] Run world tests and build. Drive across seams and a rebase threshold; record active chunk counts.
- [ ] Commit as `feat: stream seeded desert road and roadside stations`.

## Task 4: Fuel, Stops, Pause, and Browser Controls

**Files:** `src/simulation.rs`, `src/browser.rs`, app coordination, web UI, `tests/simulation.rs`.

**Interfaces:**

- `FuelState { liters: f32, capacity_liters: f32 }`; start with 12 liters and capacity 40 liters.
- `FuelState::consume(&mut self, throttle: f32, dt: f32)` uses 0.001 liters/second idle plus 0.009 under full throttle.
- `FuelState::refuel(&mut self, dt: f32)` transfers 4 liters/second and clamps at capacity.
- `can_refuel(speed_kmh: f32, distance_m: f32) -> bool` requires absolute speed below 1 and distance at most 6.
- `DriveState { fuel: FuelState, trip_m: f64, paused: bool, started: bool }` owns session state.
- `DriveState::record_travel(&mut self, previous: [f64; 3], current: [f64; 3], teleported: bool)` accumulates horizontal travel.
- `DriveState::restart(&mut self)` restores initial state; `reset` uses the vehicle API and preserves this state.
- The browser shell consumes speed, fuel, distance, pause state, and refueling eligibility. It sends start/resume/restart actions and held keys.

- [ ] Write tests for fuel bounds, zero-fuel engine gating, refueling thresholds, pause, and unchanged state after reset/rebase.

```rust
#[test]
fn refuel_requires_near_station_and_low_speed() {
    assert!(can_refuel(0.0, 6.0));
    assert!(!can_refuel(1.0, 6.0));
    assert!(!can_refuel(-1.0, 6.0));
    assert!(!can_refuel(0.0, 6.01));
}

#[test]
fn fuel_stays_within_tank_limits() {
    let mut fuel = FuelState { liters: 0.001, capacity_liters: 40.0 };
    fuel.consume(1.0, 10.0);
    assert_eq!(fuel.liters, 0.0);
    fuel.refuel(100.0);
    assert_eq!(fuel.liters, 40.0);
}
```
- [ ] Include zero-fuel reset to an unloaded station and assert correct refueling range after destination loading.
- [ ] Run `./scripts/check.ps1 -Test simulation`. Confirm failure before adding the missing functions.
- [ ] Implement fuel consumption/refueling. Pass `engine_enabled = fuel.liters > 0` into the vehicle step.
- [ ] Implement trip state, station prompts, reset/restart, and the specified keyboard mappings.
- [ ] Add Start, Resume, Restart, Fullscreen, and pause UI. Keep the HUD readable over the scene.
- [ ] Clear key state and pause on blur/visibility loss. Resume only on explicit action.
- [ ] Run simulation tests and build. Verify refueling, empty-tank recovery, pause, focus loss, and restart in the browser.
- [ ] Commit as `feat: add fuel and roadside driving loop`.

## Task 5: Finish and Verify the Playable Build

**Files:** presentation and browser modules, web styles, README, `docs/verification.md`.

**Interfaces:** Retain the defined module contracts. Tune geometry, materials, camera placement, and physics values without adding a second behavior path.

- [ ] Apply warm desert lighting, fog, weathered car materials, terrain variation, and sparse roadside details.
- [ ] Use the Impeccable skill for the loading/start screen and HUD. Apply the stop-slop skill to final UI text.
- [ ] Add original synthesized engine sound only if browser audio integration is small and verified; audio remains optional under the spec.
- [ ] Run `./scripts/check.ps1` for formatting, critical native tests, and WASM compilation.
- [ ] Start the loopback server. Play-test each control and game action against the spec's acceptance list.
- [ ] Test missing WASM and denied/unavailable WebGL2. Confirm useful error text and disabled Start.
- [ ] Drive across multiple chunks and an origin shift. Verify the visible car, terrain, wheel contact, station prompts, and bounded chunk count.
- [ ] Measure frame times at 1280x720. Record browser, renderer, hardware context, and observed performance.
- [ ] Fix measured failures. Re-run only the affected checks plus the final build if code changes.
- [ ] Write launch instructions and evidence in `docs/verification.md`. Mark unavailable checks as incomplete.
- [ ] Obtain an independent final code review. Resolve actionable findings before committing the delivery.
- [ ] Commit as `feat: finish and verify browser desert driving slice`.

## Execution and Review

Task 1 establishes the working product. Task 2 defines the physics interface. Tasks 3 and 4 share app integration; one integrator must order their updates.

Independent agents can implement world generation and pure fuel/session rules after Task 2. Assign distinct files. A reviewer must check seams, reset behavior, and origin shifts after integration. Keep browser verification with one owner.

Recommend subagent-driven execution because physics, streaming, and browser timing have separate failure modes. Preserve the user-approved design and check each working layer.

## Plan Self-Review

- Scope: Tasks 1-5 cover the spec's first driving slice and acceptance checks.
- Interfaces: Physics owns collision state. World owns absolute coordinates. Simulation owns fuel and distance. Rendering reads snapshots.
- Tests: Native tests cover critical state and physics. Browser checks cover rendered behavior and focus/error paths.
- Tooling: Docker access was verified. Image pull, WASM tools, and crate compilation remain implementation checks.
- Status: Written plan awaits user review and execution choice. No product code has been written.
