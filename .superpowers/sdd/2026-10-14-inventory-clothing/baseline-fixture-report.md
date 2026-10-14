# Baseline fixture repair report

Status: DONE.
Base: 49ae29edef2900ad47f4894c4f349f06917d3095.
Worktree: C:/Users/Corashina/.codex/worktrees/inventory-clothing/the-long-drive-wasm.

## Scope and authority

The brief authorized current-contract test scenarios and source-key lookup. Root approved the scenario design before implementation.
Root later authorized one production fix: unanimated Switch controllers must keep idle motion after use.
Root also approved current GPU size assertions and a disallowed shelter-root negative case.
I changed no renderer behavior, source library, primary checkout, dependency version, build script, or manifest export.
I created no subagents and made no merge or push.

## Fixture design

common::reference loads the source fixture without changing house geometry.
common::containers adds named CAN, DESTINATION, HINGE, SUPPLY, and CHILD actors with IDs 50000 through 50004.
Each actor has explicit body, node, collider, pickup, and action records. Container actors also have tanks, caps, anchors, and particle records.
These records use current typed contracts. They add no obsolete scene IDs or production compatibility paths.
The scenarios retain source contracts for unrelated systems needed by GarageRuntime.
physics_reference supplies controlled chassis and cabin boxes for each vehicle. It uses fresh collider IDs and test source keys.
Vehicle worlds use initial_vehicle_poses for the selected fleet.
Tests identify source actors through source keys or their owning vehicle, seat, wheel, and part records.
Save tests find source-sorted vehicle snapshots by InstanceId rather than runtime index.

## Changed expectations and evidence

| Files | Old assumption | Current test contract |
| --- | --- | --- |
| src/reference/lighting_scene.rs | Appending a probe produced index 1; source 59660 was unused. | Mutate the appended index. Select an unused source and node. Retain duplicate and bounds rejection. |
| tests/common/mod.rs, tests/common/scenarios.rs, tests/vehicle.rs | Collider zero was a synthetic chassis; one vehicle pose was enough. | Keep the source house intact. Build explicit fleet physics scenarios with current initial poses. |
| tests/interaction.rs, tests/lighting_interaction.rs | Items 1, 2, 4, 18, and 19 represented synthetic containers and controls. | Use named controlled actors. Resolve source seats, wheels, lights, and cap owners from their contracts. |
| tests/starting_area.rs, tests/mechanisms.rs | Old IDs and mechanism index zero selected the test hinge. | Build fresh controlled hinges and child nodes. Keep collision pose, limits, cancellation, and compound pose checks. |
| tests/parts.rs, tests/wheels.rs, tests/feedback.rs | Wheels had no covers; seat pickup flags allowed removal; exits were blocked by incidental geometry. | Remove source covers before wheel operations. Set removable-seat flags. Add an explicit exit blocker. |
| tests/input.rs, tests/starting_area.rs | Frame elapsed time owned fades and cycle time. | Advance accepted fixed steps. Check paused state, resumed fades, exact elapsed steps, and the six-step cap. |
| tests/lighting.rs | 40 lights and 16,048 uniform bytes; decimal f64 step literals. | Assert 42 lights, 100 shadows, 16,368 bytes, Rust/WGSL equality, and the 16,384-byte limit. Derive time from the actual f32 delta. |
| tests/save_capture.rs, tests/save_roundtrip.rs, tests/save_schema.rs, tests/save_world.rs | Duplicate engine slots, fixed part indices, two assembly bodies, and exact rebased floats. | Use real engine slots, named children, and source-key lookup. Assert the exact assembly-root set, including initial loose assemblies and removed covers. |
| tests/plate_interaction.rs, tests/save_capture.rs | Quaternion recomposition and rebasing were bit-exact. | Apply bounded float comparisons only to poses. Keep identities, counts, membership, velocities, and discrete state exact. |
| tests/audio.rs | Invalid required clips were optional playback failures. | Reject invalid required timing in both manifest loaders. Permit runtime loading with invalid playback voices, then reject the optional playback contract. |
| tests/audio.rs | First matching source records belonged to synthetic action actors; source faucet orientation matched a test can. | Bind audio to fresh actor source keys. Use two controlled caps on one destination and retain non-looping fill/spill checks. |
| tests/consumption.rs | Items 3 and 18 were drink sources; digestion did not affect the bite-edge test. | Use controlled water sources. Derive pending food from one bite and 12 fixed digestion steps. Keep source food tests on the source fixture. |
| tests/consumption.rs | Particle contact debited emitted fluid again; reported mixed-fluid debit was 0.54444444 L. | Assert zero contact debit and unchanged source fluid. Assert 0.5 L reported debit under current per-fluid source-volume caps. |
| tests/reference.rs | Tank zero was the 39 L vehicle tank; 40 L exceeded its capacity. | Find the primary vehicle fuel tank. Set overflow relative to the selected tank capacity. |
| tests/lighting_reference.rs, tests/survival_reference.rs | Three unique scene assets; the second vehicle was not a valid shelter owner. | Assert four unique scene assets. Test a valid body outside home and vehicle roots as the negative case. |
| tests/plate_reference.rs | Glyph zero had width, so x=1024 exceeded the atlas. | Set x beyond the actual atlas extent, including zero-width glyphs. |
| tests/rendering.rs | The cloned material had no extra texture defaults. | Clear inherited texture bindings before building the test material's complete active-slot contract. |
| tests/player.rs | The source chassis started with identity rotation. | Reset the controlled chassis pose before checking forward and sideways exit angles. |
| tests/world.rs | Node 999 was free; only the test room supplied interior geometry. | Use fresh node/collider IDs and remove source interior colliders from the controlled room scenario. Keep exact boundary ray checks. |

The source contract has four UniqueScene assets: IDs 1 through 4. Prototypes remain excluded from the instance count.
The resource-bound test now counts fleet colliders, streamed terrain/props/stations, and three support chunks per vehicle.
It still checks 11 active chunks, four wheel contacts, removal of the returning vehicle's three support chunks, and stable repeat counts.

The mixed-fluid debit follows src/fluids.rs drink_transfer. Gas reports 0.1 L. Blood acceptance exceeds its remaining 0.4 L source volume.
The method caps reported blood debit at 0.4 L. Diesel accepts zero after the bladder fills. The total is 0.5 L.
The test retains water filtering, gas and blood retention, and the 2 L bladder bound.

## Required audio contract

src/reference/audio.rs:702-773 defines AudioGateBinding, AudioGateClip, and AudioGameplayReference.
Required clips contain id, source, frames, frequency, and duration_seconds.
Validation requires positive IDs, source path IDs, frames, frequency, and finite positive duration matching frames/frequency within 1e-6.
The ten required survival roles each need valid clip links and a positive ordered pitch range.
Thus corrupt clips are source-contract failures. Corrupt playback voices remain distinct from required gameplay timing.
Native audio tests and the existing web audio owner tests retain playback-failure coverage.

## Authorized production fix

LightingState::use_control set SwitchMotion::Initial for switches without animated poses.
The source flashlight regression then failed immediate save capture with InvalidLinks("Switch motion without animated poses").
The red evidence is baseline-fixture-flashlight-red.log.
The fix starts switch motion only when animated_poses is nonempty. It retains the on/off toggle and refresh_switch call.
The regression uses the unmodified source reference. It toggles an unanimated flashlight, checks on state and idle motion, then captures and loads immediately.
Save validation remains unchanged. Existing animated switch, fade, and audio tests cover the adjacent paths.

## Float bounds

Pose tests now allow f32 rounding from subtraction, composition, and quaternion normalization.
Position tolerance is 4 * f32::EPSILON * max(abs(before), abs(after), abs(rebase shift), 1), per axis.
The rebase test uses shifts [2048, 0, -4096], so horizontal limits are 0.0009765625 and 0.001953125 meters.
Observed drift included x=0.000003814697266 and z=0.00006103515625 meters in baseline-fixture-runtime-2.log.
Quaternion component tolerance is 8 * f32::EPSILON, about 9.54e-7. The comparison accepts equivalent opposite quaternion signs.
Observed identity recomposition had w=1.0000002 and tiny vector components, about 2.38e-7 maximum error.
No runtime transforms or saved coordinates changed for these tests.
Cycle time tests use f64::from(0.02_f32), preserving exact comparisons to the actual input representation.

## Verification

All commands ran in the isolated worktree. Docker used the pinned my-summer-slop-toolchain image and CARGO_BUILD_JOBS=2.
The cargo target came from scripts/cargo-target.ps1. Logs remain in this plan directory.

| Command or stage | Result | Evidence |
| --- | --- | --- |
| Previous focused baseline | 65 pass, 102 fail, 52 existing ignores | baseline-report.md and baseline-native-focused.log |
| New flashlight regression before fix | Fails required save validation | baseline-fixture-flashlight-red.log |
| Focused interaction, vehicle, lighting, parts, audio, input, save, and lifecycle repairs | Passed after fixture repairs | baseline-fixture-interaction-2.log, baseline-fixture-runtime-4.log, later logs |
| All native targets with --no-fail-fast | Exposed 13 additional failing target groups | baseline-fixture-native-2.log |
| Additional target repair runs | Final consumption and lighting run passes | baseline-fixture-extra.log through baseline-fixture-extra-5.log |
| Full scripts/check.ps1 | Exit 0: format passes; JavaScript 91/91; native 354 pass, 0 fail, 66 existing ignores; release WASM passes | baseline-fixture-check.log |
| Post-review focused verification | Format passes; 73 pass, 0 fail, 6 existing ignores | baseline-fixture-review-check.log |
| Valid source digestion and input check | Format passes; 23 pass, 0 fail, 5 existing ignores | baseline-fixture-digestion-check.log |
| git diff --check | Pass | Tool output |

Native command pattern: cargo test --locked --no-fail-fast with the named --test targets in each log.
Final focused command: cargo fmt --all -- --check and cargo test --locked --test interaction --test consumption --test save_schema --test save_roundtrip.
The release WASM build finished in 8 minutes 50 seconds.
The post-review commands cover test-only corrections made after the full check compiled its native targets.
The production code stayed unchanged throughout final verification.
The full check used a process-local PowerShell docker function that inserts --env CARGO_BUILD_JOBS=2 for docker run.
I did not alter scripts/check.ps1.

## Self-review and limits

I reviewed changed assertions, actor ownership, source lookup, fixture uniqueness, and the production Switch guard.
I removed one unused edit to a source action binding from the cap-surface scenario.
I kept unrelated food-source tests on common::reference and made the startup-rotation scenario use the named HINGE actor.
I added explicit toggle-state and idle-motion assertions to the flashlight regression.
I removed a temporary zero-digestion override because survival validation requires a positive delay_speed.
The bite test uses the source rate and checks 0.5 * (1 - FIXED_DT * delay_speed)^12 pending food.
I found no remaining production changes outside the authorized guard.
Existing installed-source ignores remain. These native scenarios and build checks do not prove browser behavior or source visual parity.
The separate renderer investigation remains outside this repair.

## Commits

- 3f6fca929ef44d3581dc5472e08b6a3316c90b94: Repair baseline scenarios and idle unanimated light switches.
- This report and the retained final verification logs form the following documentation commit.
