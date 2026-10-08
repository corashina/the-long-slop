# Desert driving in Rust and WebAssembly

Date: 2026-10-08, Europe/Warsaw
Status: Conversational design approved; written spec awaiting user review.

## Intent and agreed scope

The user initially requested an investigation of recent browser game ports and a browser implementation of The Long Drive, preferably in Rust and WebAssembly, using their Steam installation. They explicitly accepted a Rust/WASM recreation of its driving gameplay and selected a driving-first scope: desert road, car handling, cockpit/chase cameras, fuel, and roadside stops. The proposed Bevy/Rapier architecture and procedural assets were approved in chat.

Success is a playable local browser game with that driving loop. This deliverable is a recreation inspired by The Long Drive, not a claim of a completed decompilation or a behaviorally identical port. It runs its simulation and rendering on the user's machine. The Steam installation is a behavioral reference; the first build has its own code, geometry, textures, and audio.

This first sub-project is the driving slice. Walking, inventory, detachable parts, engine repair, multiplayer, Steam services, original save compatibility, extraction/import of original assets, mobile controls, and public deployment are outside its scope.

## Verified local context

- Workspace: `C:\Users\Corashina\Documents\Projects\the-long-drive-wasm`; empty at initial inspection, without a Git repository or applicable ancestor AGENTS.md files.
- Steam installation: `C:\Program Files (x86)\Steam\steamapps\common\The Long Drive`.
- Steam app ID: 1017180. Installed manifest build ID: 11140890. These identify the inspected installation, not a claim about the latest public game build.
- The `globalgamemanagers` header identifies Unity 2021.3.18f1.
- `MonoBleedingEdge` and `Managed/Assembly-CSharp.dll` are present; the latter is approximately 964 KB. This is favorable evidence for managed-code recovery in a separate original-game port investigation.
- The installation includes `steam_api64.dll`, NAudio, and desktop framework assemblies. Assembly strings reference WheelCollider, Rigidbody, System.Threading, and Steam APIs. These are dependency/reference evidence; call sites and active gameplay paths have not been audited.
- Node, npm, Python, and Git are available. Rust/Cargo, wasm-pack, Trunk, a Unity editor, .NET CLI, and an MSVC C++ toolset were not found in PATH or checked conventional installation locations. Rust build prerequisites must be provisioned and verified during implementation.

The installation inspection read metadata and file contents; it did not execute game assemblies or modify Steam files.

## Approaches considered

| Approach | Fit | Trade-off |
| --- | --- | --- |
| Rust/WASM recreation with Bevy and Rapier | Selected; matches the accepted driving-first goal | Original behavior must be implemented and tuned. It does not automatically preserve Unity semantics. |
| Recover Unity project and rebuild to WebGL/WASM | Possible separate fidelity investigation for this Mono build | Asset recovery, script references, shaders, native dependencies, and browser adaptation remain unverified substantial work. |
| Stream the native game into a browser | Feasible category of browser access | Requires a running native game and streaming service; does not produce a browser-native Rust/WASM implementation. |

Static binary recompilation demonstrated for a specific console platform is not an established general conversion route for this Win64 Unity game.

## Player experience

The player opens the locally served page, sees a start screen with controls, and starts a new drive with a fixed default world seed. The game begins seated in a weathered compact car on a desert road with enough fuel to reach the first roadside stop. The car has visible wheels and a simple modeled cockpit. The scene has dunes, scattered rocks and vegetation, roadside posts, and sparse refueling stations. Color, lighting, fog, and material variation evoke a dry, worn desert journey.

The road continues through deterministic chunks with gentle bends and grades. The car can leave the road, collide with terrain and solid props, brake, reverse, and recover after an overturn. Cockpit and chase cameras show the same physical car state. A compact HUD shows speed in km/h, fuel, distance traveled, and context-sensitive refueling instructions.

The engine runs during active driving while fuel remains. Fuel falls while the engine is running, with consumption increasing under throttle. At zero fuel, engine force becomes zero while braking, steering, and coasting still work. Holding the refuel key transfers fuel only below 1 km/h and within 6 meters of a station's refueling point. Refueling exceeds idle consumption and clamps at tank capacity. Each station can refill the tank; finite station stock is not part of this slice. Refueling is a seated-car interaction.

Pause freezes simulation, fuel consumption, and trip distance. Losing window focus clears pressed keys and pauses; return requires an explicit resume. A reset recovers the car onto a nearby road pose, clears its velocities, and preserves fuel and trip distance. Reset does not restore fuel. At zero fuel, reset instead places the car at the nearest station's refueling point; a new drive is also available. Restart restores the initial seed, fuel, and trip state. Trip distance accumulates horizontal physical travel and excludes reset/rebase teleports.

### Controls

| Control | Action |
| --- | --- |
| W / Up arrow | Throttle |
| S / Down arrow | Brake while moving forward; reverse after reaching a low-speed threshold |
| A / D or Left / Right arrows | Steer |
| Space | Handbrake |
| C | Switch cockpit/chase camera |
| E, held | Refuel when stopped within station range |
| R | Recover/reset vehicle |
| P / Escape | Pause/resume; Escape also releases mouse capture if used |

Buttons provide start, resume, restart, and fullscreen actions. Keyboard controls work without mandatory pointer lock. A camera look implementation, if included, must leave a visible route to release capture. HUD controls and loading errors remain readable outside the game canvas.

## Architecture and data flow

Use stable Rust targeting `wasm32-unknown-unknown`. Bevy supplies the application schedule, scene, cameras, meshes, materials, and browser renderer. Rapier supplies rigid-body integration, collision queries, and raycast vehicle behavior. Select and pin a mutually compatible published Bevy/Rapier combination in the implementation plan, verifying their release manifests rather than assuming the current development branches work together. The Rapier integration repository recently moved; use its current canonical project.

The first build targets WebGL2 for the browser renderer. A separate WebGPU build is optional after the WebGL2 acceptance checks pass; automatic fallback is not promised. The browser is supported if it can create the required WebGL2 context and execute the WASM build. The initial release is desktop keyboard/mouse only.

Keep these responsibilities in focused modules:

- `simulation`: input-to-control state, fuel, refueling eligibility, trip distance, and pause/reset rules; independent of DOM and rendering where practical.
- `vehicle`: chassis rigid body, collider, four wheel raycasts, suspension tuning, traction, speed-sensitive steering, engine/brake force application, and wheel pose updates. It owns the adapter between the chosen Rapier API and Bevy schedule.
- `world`: seeded road/terrain generation, chunk keys, station placement, and collider lifecycle. It returns chunk-local geometry and absolute-world metadata.
- `presentation`: original car/terrain/prop geometry, materials, lighting, fog, and cameras. It reads interpolated simulation poses and never advances gameplay.
- `browser`: startup, loading/error state, keyboard/focus events, minimal HTML shell, resize, fullscreen, and optional synthesized audio after a user gesture.
- `app`: composition and ordered scheduling of the modules.

Data flow is browser input -> control state -> fixed simulation step -> physics/fuel/trip state -> interpolated vehicle poses -> camera, world rendering, and HUD. Station interactions use authoritative physics location and speed. Rendering does not mutate fuel or physics.

Run gameplay and vehicle physics at a fixed 60 Hz with render interpolation. Clamp accumulated wall time after stalls and limit catch-up steps; resume must not simulate all elapsed background time. Physics timestep, unit conventions, wheel axes, and suspension parameters are explicit constants/configuration. Model one dynamic chassis and four raycast wheels, not four separate dynamic wheel bodies. Wheel queries exclude the chassis.

Road and nearby terrain receive colliders. Solid rocks, posts, and station structures receive simple colliders where they can be reached. Decorative distant scenery has no gameplay collisions. Chunk geometry and collision geometry derive from the same road/height samples so seams remain drivable.

## Long drives and world streaming

Generate chunks deterministically from seed and integer chunk coordinates. Road samples use shared global coordinates so adjacent chunks agree at their edges. Keep only a bounded neighborhood around the car; unload both render resources/entities and physics colliders outside that neighborhood. Avoid a road generator whose retained history grows with distance.

Place stations at deterministic intervals and provide enough initial fuel for the first station with a conservative reserve under ordinary road driving. Fuel consumption and station spacing are tuned together. The game must always expose a recovery/new-drive route if the player runs out of fuel.

Use a floating origin when local positions exceed a configured threshold. Keep absolute trip/world coordinates separately, with enough precision for long travel. Shift every affected body/collider, vehicle interpolation pose, camera, and active chunk consistently between physics steps. Update Rapier's query/broad-phase state as required by the selected integration. Rebasing must preserve velocities, wheel contact behavior, station eligibility, and distance. A near-origin coordinate shift must not count as distance traveled.

## Browser loading, errors, and storage

Serve static HTML, JavaScript glue, and WASM on localhost with correct MIME types. Use a reproducible build/serve script and document the exact launch command. Bind the development server to loopback by default. This slice has no network game service and does not upload Steam files.

Use browser APIs through appropriate Rust bindings for browser-only I/O. Do not assume native filesystem access, OS threads, or Win64 DLL loading. Keep the first build single-threaded to avoid shared-memory hosting requirements. Procedural first-build assets remove the need for a large initial asset download.

Show progress while loading and a useful error for WASM fetch/initialization failure or unavailable graphics. Release game-owned event handlers/resources on teardown if the runtime supports restarting within the page. Avoid a misleading interactive start button while initialization is incomplete.

Save/load persistence is outside the first slice. Reloading the page starts a new drive. Optional audio must be original/synthesized and start only after user interaction; missing audio support must not prevent driving.

## Verification and acceptance

The deliverable includes source, pinned dependency lockfiles, build/serve instructions, and a compiled local browser build. A build is complete only after the following evidence is collected:

1. Rust formatting and compilation checks pass. Focused simulation tests cover fuel bounds, zero-fuel thrust, refueling proximity/speed restrictions, pause, and reset behavior.
2. World tests verify deterministic generation, matching chunk boundaries, and bounded active chunks. A rebase test checks preserved relative poses/velocities and unchanged trip distance.
3. The WASM release/debug configuration selected for delivery compiles and loads from the documented localhost command. Loading failures and unsupported graphics have readable messages.
4. Browser play-testing confirms throttle, brake/reverse, steering, handbrake, both cameras, collisions, station refueling, fuel exhaustion/recovery, pause/resume, focus loss, reset, and restart. Verify visibly rendered 3D output, not only DOM text or a successful HTTP response.
5. An extended drive crosses multiple chunk boundaries and at least one floating-origin threshold with no disappearance, obvious seam failure, or persistent loss of vehicle contact. Track active chunk counts to substantiate bounded streaming.
6. Target smooth 60 FPS at 1280x720 on the actual test machine with modest scene settings. Record browser, renderer, machine context, and observed performance; do not report the target as a measured result. If performance is lower, optimize the measured bottleneck and document the actual limit.

Tests should validate these behaviors, not duplicate rendering implementation details. Toolchain or browser-access failures are reported as incomplete verification; an untested WASM build is not labeled ready.

## Implementation sequence

After written-spec review and implementation-plan approval:

1. Provision project-local Rust/WASM build tooling where practical, verify a compatible host linker/build environment, and pin the chosen crate versions.
2. Establish the minimal rendered browser application and loading/error shell.
3. Implement and test the car simulation on a small static track, then connect vehicle poses to visuals and cameras.
4. Add seeded desert road/terrain streaming and floating-origin handling.
5. Add fuel, stations, refueling, pause/reset/restart, and HUD.
6. Tune visual atmosphere and driving behavior, then build and perform browser acceptance checks.

Parallel agents may handle independent simulation/world/presentation work after their shared interfaces and ownership are defined. One integrator owns schedule ordering, physics integration, and final browser verification.

## Research findings and sources, checked 2026-10-08

### What the recent projects actually do

- Recent PSP Web Recomp work statically translates PSP MIPS machine code into C++, compiles it with Emscripten to WebAssembly, and implements platform services plus a WebGL2 renderer. The author's reported working titles share an engine; compatibility with arbitrary games is unproven. Its useful lessons are fixed guest timing, measured WASM/JavaScript crossings, asset streaming, and explicit replacement of platform services. [Developer repository](https://github.com/snuri00/psp-web-recomp).
- September coverage of GTA III/Vice City browser availability builds on older re3/reVC reverse-engineered C++ code. The browser distribution identifies itself as a reVC WebAssembly port and expects compatible game data. This is a source-port route, not generic Windows executable conversion. [reVC browser distribution](https://github.com/Carter54git/revcdos/blob/main/README.md), [re3/reVC source mirror](https://github.com/HeavyDutyGames/Re3).
- October 6 reporting claimed an AI-assisted GTA V browser port. A later static artifact investigation reports Emscripten glue, RAGE symbols, HTTP paging, shared memory, and WebGPU integration. That analysis is evidence about artifacts, not a reproducible source pipeline or verification of authorship, AI involvement, or full-game completeness. [Dated coverage](https://www.tomshardware.com/video-games/pc-gaming/gta-v-playable-in-browser-immediately-nuked-unofficial-webassembly-port-built-with-ai-gets-taken-down-within-hours-of-going-live), [artifact investigation](https://github.com/shadany7824/playgta5/blob/main/ENGINE_INVESTIGATION.md).
- A Unity IL2CPP case study distinguishes preserved Mono method bodies from IL2CPP exports containing stub methods, and describes manually reconstructed MiSide logic. Its measurements concern the examined games and must not be generalized into a recovery guarantee. [Author's case study](https://github.com/clericall/il2cpp-wasm-teardown).

### Implications for the installed game

Recovering a Unity Mono project and rebuilding with Unity is a plausible alternative because managed DLL code can be an input to Unity's IL2CPP -> C++ -> WebAssembly pipeline. Asset recovery does not recover Unity's native engine, and success for this game remains unverified. [Unity managed plug-ins](https://docs.unity3d.com/2021.3/Documentation/Manual/UsingDLL.html), [Unity WebGL pipeline and constraints](https://docs.unity3d.com/2021.3/Documentation/Manual/webgl-technical-overview.html).

AssetRipper offers decompiled, hybrid, and DLL export modes, with compile-error/experimental caveats. Recovering assets/scripts and adapting them for the browser are separate tasks. Native Windows DLLs require compatible replacements/source builds for WebGL. [AssetRipper export modes](https://github.com/AssetRipper/AssetRipper/blob/master/Source/AssetRipper.Export/Configuration/ScriptExportMode.cs), [Unity WebGL native plug-ins](https://docs.unity3d.com/2021.3/Documentation/Manual/webgl-native-plugins-with-emscripten.html).

The Rust recreation uses an independently implemented runtime and selected behaviors. Bevy has browser 3D examples; wgpu supports WebGL2 and WebGPU on WASM; Rapier provides raycast vehicle control. Rust's WASM target requires explicit browser I/O rather than ordinary native filesystem/thread assumptions. [Bevy WebGL2 example](https://bevy.org/examples/3d-rendering/3d-scene/), [wgpu](https://github.com/gfx-rs/wgpu), [Rapier vehicle controller](https://rapier.rs/docs/user_guides/rust/vehicle_controller/), [Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).

## Risks and scope limits

The main implementation risks are compatible Bevy/Rapier versions and browser build tooling, stable car tuning, terrain/collider seams, consistent floating-origin shifts, and browser performance. Resolve each with a small working checkpoint before adding the next subsystem.

The first build approximates the game's driving atmosphere and selected loop. It does not establish full original-game parity. A future asset importer or original Unity port needs its own scoped design and validation. No game executable or proprietary asset is part of this recreation's distribution.
