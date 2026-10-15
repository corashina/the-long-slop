# Project roadmap

Updated: 2026-10-15.

## Current status

### Inventory and clothing

We completed tasks 1 and 2 on `codex/inventory-clothing`. We stopped tasks 3ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Å“10 at the user's request.
Keep these eight tasks stopped until the user resumes them. Do not start another feature during integration.
Use Steam build `11140890` and its decompiled control flow as the source authority.

Task 1 retains checked inventory, item, clothing, player, and acquisition contracts.
Task 2 adds four slots, storage, hand retrieval, source icons, and saved ownership.
The master integration uses save schema 5 for inventory, driving, world changes, and condition state.
We reject older schemas without migration. We removed JSON test fixtures and their dependent checks.
We retain small independent checks and source verification.
The merged tree passed the release WASM build, 10 native checks, 53 exporter checks, and 21 browser interface checks.
Two installed-export checks remain ignored until we generate a matching merged package.
The acquisition check confirms unchanged source values and valid links to the current world tables.

| Task | Status | Remaining scope |
| --- | --- | --- |
| 3 | Stopped | Add explicit equip/release transitions, direct Tab behavior, and source item-action priority. |
| 4 | Stopped | Add station loot selection, source RNG order, item creation, stable identities, and persistence. Preserve empty and unsupported outcomes. |
| 5 | Stopped | Add source box contents and one-time release. Save release state to prevent duplicates. |
| 6 | Stopped | Add source shovel digging and buried-item recovery. Save burial and recovery state. |
| 7 | Stopped | Add supported wearables, cigarettes, and holiday startup. Keep disabled starter apparel disabled. |
| 8 | Stopped | Add both player skins, source pose and IK, wearable anchors, and camera visibility. Check mirrors, probes, and shadows. |
| 9 | Stopped | Add appearance controls and remaining HUD behavior. Prepare and commit profiles atomically. |
| 10 | Stopped | Compare the completed feature with the running source game. Record gameplay and visual limits. Use critical checks only. |

The branch passed its Task 2 release build, source checks, and browser inventory checks before integration.
Those results do not establish acceptance of the merged runtime package. Export and browser checks remain in the work order below.
A loose flashlight fell below the floor during a long browser run. We have not confirmed its cause.
Native source comparison, trusted desktop input, and full clothing behavior remain unverified.

### Earlier cleaning and painting integration

We merged Cleaning and painting into local master in `70a4604`, then fixed restored wheel quality in `a48cc2c`.
That merge implemented authored tools, live condition state, cleaning, spray paint, and save schema 4.
The inventory integration replaces schema 4 with schema 5.
Native library and WASM checks, release compilation, and wasm-bindgen passed for `a48cc2c`.
Those build results predate the later rendering cleanup and test reduction.
The retained native test targets compiled after we removed obsolete API tests and corrected retained fixture bindings.
Those native behavior checks failed because the former shared fixture lacked required world data.
The checked world probes, harness, and provenance are restored in `.cache/world-source`.
We retained the earlier runtime assets and WASM package. Merged package and active gameplay acceptance remain open.

The shared particle owner applies selected spray velocity, inherited motion, and masked collision.
Spray refraction and brush soft-particle depth fading remain deferred shared-renderer work and known visual differences.
The historical feature package passed release, export, readiness, New Game, and same-export Save/Load checks.
IAB showed `Mouse capture failed` after Continue. Active tool use and driving remain unverified.
See the [cleaning implementation](../src/interaction/cleaning.rs), [condition state](../src/conditions.rs), and [verification status](#verification-status).

### Checked world input recovery

Completed on 2026-10-15. Restored 95 capture and harness files, totaling 149,835,263 bytes, from the retained fast-startup capture.
The harness, input contract, and sample file retain their original checked hashes.
The local run pointer resolves inside this workspace. Verification no longer depends on the other worktree.
The unchanged probe tool accepts all five approved seeds and both retained runs.
Each run includes 55 road records, 20 terrain tiles, and 10,000 placement records.
The native primitive payload retains seven checked samples.
Installed and archived build `11140890` inputs match all 183 source-file hashes in each location.
Full accepted-library verification passed for 262,784 objects and their dependency graph, file receipts, and converted assets.

The recovery receipt records the source directory, relocated run pointer, provenance, and each restored file hash.
Receipt: `.cache/world-source/recovery-receipt.json`. Probe data and original source helpers remain in ignored storage.
This verifies the restored source inputs. Port gameplay acceptance remains open.
Wheel-response measurements remain a separate work item.

### Verification status

Before inventory integration, the reduced suite contained 162 declared cases. The cleanup removed 647 of the previous 809 cases.
Inventory integration removes the remaining JSON fixture and dependent checks. The table below records the earlier cleanup run.

| Suite | Declared cases | Last recorded result after test reduction |
| --- | --- | --- |
| Rust | 100 | Native targets compiled; 13 passed, 71 failed, and 13 remained ignored. Three WASM rendering cases did not run. |
| Python exporter tools | 40 | 40 passed through normal discovery. |
| JavaScript browser interface | 22 | 22 passed. |

The former shared fixture had no `world` field. Most recorded native failures stopped before behavior assertions.
Inventory integration removes that fixture and its dependent checks.
The [check runner](../scripts/check.ps1) names the retained JavaScript suites and runs native tests before the WASM build.
These results describe the recorded cleanup runs. We did not rerun the test suites during probe recovery.
Older test counts and the former six web failures describe the earlier, larger suite.
The reduced suite covers fewer formula, layout, media, and source-value combinations.

The documentation cleanup removed the detailed verification, research, plan, and audit files from the checkout.
Read the retained verification record from Git with `git show 6272b02:docs/verification.md`.
Historical reports establish only their recorded revisions. Current source links identify implementation owners, not runtime acceptance.

### Earlier source-world integration

Master contains the source-world integration and retained Bevy ownership patch after merge `79765e3`.
We completed nine source-world implementation layers and three integration merges.
The source-world scoped review passed with no findings. We installed its checked runtime inputs and WASM package before cleaning integration.
We reconciled master `6423b062` with source-world startup, collision, save identities, and renderer paths.
We fixed hidden grass preparation and generated attachment readiness in `8d4341f`.
We fixed the final review's texture receipt cleanup finding in `103bee7`.
The full integration review and scoped re-review have no open Critical or Important findings.
We preserved local transparent sorting and warmup filtering through the replacement renderer.
We added no features or dependencies during the final source-world merge.

The checked source-input gap is resolved. Fresh merged runtime export and package verification remain open.

### Changes in master

| Area | Implemented changes |
| --- | --- |
| Vehicles and interaction | Two imported cars, vehicle selection, carry, parts, mechanisms, and source recovery |
| Driving controls and transmission | Source pedal ramps, manual gears, clutch, two saved assists, control help, and gear feedback |
| Rendering | Shared lighting, shadows, probes, camera effects, transparent glass, live mirrors, and source plates |
| Audio | Source vehicle, interaction, weather, and Lada radio audio |
| Survival and weather | Existing consumables, survival state, sleep, weather, storm lighting, and particles |
| Portable flashlight | Source bindings, carried controls, storage restrictions, and shared light rendering |
| Cleaning and painting | Authored tools, cleaning, spray paint, wear, live conditions, and retained material updates |
| Save/load | Browser storage, fleet, fluids, items, conditions, tools, and scene replacement under strict schema 4 |
| Performance | Reduced camera light clustering and depth/shadow material bindings |
| Roadside stations | Checked source placement, selected prototypes, static collision, and terrain alignment |

We added left-button door dragging and suppress view movement during a drag.
The integration replaces the obsolete station generator with checked source-world placement and selected prototypes.

### Source-world changes

| Layer | Implemented changes |
| --- | --- |
| Source inputs | Checked build `11140890` records, generation rules, assets, and source receipts |
| Roads | Sequential random state, checkpoints, source positions, slopes, curves, and segment joins |
| Geometry | Road meshes, collision geometry, imported garage, and continuous garage exit |
| Terrain | Source height layers, road/building alignment, biomes, and distant terrain |
| Placement | Separate structures, stations, props, and lamp layers with source selection rules |
| Rendering | Source materials, terrain, grass, impostors, lamps, shadows, and selected prototype ownership |
| Streaming | Bounded jobs, requests, geometry, collision windows, and floating origin |
| Revisits | Stable identities, absolute poses, compact change records, attachment state, and source replay |
| Final corrections | Loader headroom, double-precision local transforms, and lazy painting assets |

We retain each changed object as a compact record. Active geometry has separate limits.
Changed-object records have no fixed count limit. Source checkpoints and first-call history share a 16 MiB limit.
Active CPU/GPU limits remain 128 MiB each. Prototype limits remain 256 MiB CPU and 512 MiB GPU.
Generation and presentation share 32 MiB of pending storage.

The documentation cleanup removed the source-world design, implementation plan, and research from the checkout.
Git history retains those documents. The [world contract](../src/reference/world.rs) and [world runtime](../src/world.rs) remain in master.
The [probe tool](../tools/reference/world_probe.py) requires checked inputs and provenance before accepting a new capture.

### Integration changes

We replaced station save identities with canonical generated identities, change records, and source spawn history.
The merged save schema 4 includes world, driving, and condition state. Browser database version remains separate.
We preserve source integer identities as decimal strings at the browser save boundary.
We apply the saved seed before generation and restore supported object state before physics participation.
We preserve imported fleet state, dormant absolute poses, recovery, and cross-owner attachments.
Vehicle selection updates future source spawn context. Stored first-call boundaries retain their original context.
We connect existing sausage and turd item definitions to generated state and source-owned geometry.

We completed hidden renderer preparation before scene replacement.
We preserve active scene owners until candidate uploads, pipelines, collision, and restored bindings are ready.
We added scoped GPU errors, completion receipts, separate mesh buffers, and combined memory charges.
We patched the existing Bevy 0.19.1 pipeline cache to own candidate preparation, cancellation, and retries.
We retain the original crate licenses and source provenance. The patch changes one upstream file.
Generated meshes require checked renderer bounds and flags. Selected ShadowsOnly meshes skip visible and motion passes.
We published 365 asset entries and recorded hashes for 7,890 runtime files, including the WASM package.
We retained master's latest depth/shadow bindings and connected candidate caster ownership.
The earlier source-world package passed native/WASM builds, packaging, 22 shader variants, JavaScript syntax, and scoped whitespace checks.
We preserved existing WASM warnings and unchanged upstream whitespace.

We verified all 7,890 installed runtime file hashes against their receipts before the cleaning merge.
Source-world runtime acceptance remains open. We skipped tests and runtime checks for that earlier merge.

### Verification limits

The merged source-world implementation passed native and release WASM builds, packaging, and static review.
These results establish compilation and reviewed structure. They do not establish source runtime parity.
The earlier source-world merge skipped tests and runtime checks. The retained-suite results above describe the later test reduction.
Earlier rendering checks below describe their recorded revisions.
Fixed source samples, collision seams, long drives, rebasing, revisits, and memory plateau remain unverified.
We have not measured retained objects or runtime memory for the merged world.
We removed obsolete generator paths from the integration. Acceptance of the replacement remains open.
We added no new storage engine, save migration, multiplayer, or deployment work.

## Driving integration: partial acceptance

We merged the driving work into local master with the source-world implementation.
The integration includes source pedal ramps, manual gears, clutch, and two saved driving assists.
Controls show the source keyboard bindings. Feedback shows the selected gear.
Seat changes clear transient driving input. Automatic rules require a mounted engine.
Suspension validation accepts source endpoints and rejects invalid values.
We retain both cars, source-world placement, generated identities, and renderer ownership.
Save schema 5 combines world records, selected gears, driving assists, condition state, and inventory ownership.
We reject versions 1ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Å“4 before reading their vehicle fields. No migration or old reader exists.

The control and transmission work passed earlier code review. Normal driving acceptance remains open.
The retained native targets now compile. Current fixture failures and recorded counts appear in [verification status](#verification-status).

The controller previously verified assist choices, dependency, paused save, reload, restore, and New game defaults.
IAB refused pointer capture. Normal launch, both-car driving, and 30/60/120 FPS routes remain unverified.
At zero wheel RPM, the port can retain neutral. Source curve extrapolation remains unverified.

Source response measurements remain a prerequisite. The earlier repository audit found none, and the user confirmed none were available.
Wheel-response work remains unscheduled. Tire/rim grip, empty hubs, suspension, wheel spin, and combined-brake response remain incomplete.
The adapter supplies summed brake inputs. The stock solver excludes simultaneous motor force and brake impulse per wheel.

Read [vehicle coverage and controls](vehicles.md), the [drivetrain](../src/simulation/drivetrain.rs), and the [wheel controller](../src/vehicle.rs).

## Work order

Recover source inputs and finish the matching package before expanding gameplay. Keep each layer working before adding the next.

1. **Recover checked world probes and provenance: complete.** Restored the unchanged build `11140890` capture and its local replay files.
   The probe tool accepts the recovered inputs. See [checked world input recovery](#checked-world-input-recovery).
2. **Export and build one matching package.** Generate runtime records, assets, and WASM from the merged code and accepted source inputs.
   Verify hashes, source identity, and candidate preparation. Record the package identity.
3. **Verify active gameplay.** Obtain mouse capture and check walking, preparation, both cars, cleaning, painting, Save/Load, recovery, and restart.
   Check collision seams, long drives, rebasing, revisits, retained objects, memory, and assembled audio.
4. **Measure source driving behavior.** Collect matched original-game measurements before changing tire, rim, hub, suspension, or braking response.
   Then verify launch, both-car driving, wheel spin, and 30/60/120 FPS behavior against those measurements.
5. **Complete save coverage.** Preserve needs, food progress, sleep, storms, toilet state, and radio state.
   Add save slots and autosave after these owners restore correctly. Keep one current schema without migrations.
6. **Expand gameplay in layers.** Add cargo, remaining items, the fleet, damage, weapons, and enemies in dependency order when authorized.
   Add clothing with inventory and towing with cargo. Connect settings, custom music, and remaining effects through their existing owners.
   Include save support and source comparison with each new feature.
7. **Add multiplayer last.** Synchronize stable local gameplay, object identities, and persistent state.

## Full-game feature gaps

Reference: installed Steam build `11140890`. This backlog records gaps beyond the implemented layers above.
The source-world integration is in master. Its runtime acceptance remains open.
Missing means no gameplay implementation. Partial means the current implementation covers selected behavior or content.
The backlog does not expand approved implementation scope. Each new feature needs a source-based specification.

| Feature | Status | Remaining work |
| --- | --- | --- |
| Source world generation | Implemented; runtime unverified | Checked probes are recovered. Build the merged package, then verify streaming, revisits, and memory. |
| Buildings and exploration | Implemented; runtime unverified | Check source buildings, interiors, landmarks, placement, content coverage, and collision in the matching package. |
| World population | Partial | Integrate source loot placement; add excluded abandoned vehicles, items, and encounters. |
| Full vehicle range | Partial | Add vehicles beyond the Lada and Skoda, including diesel, two-stroke, automatic, bus, and bike mechanics. |
| Driving physics | Partial | Verify launch and both-car driving; collect native source measurements before wheel, rim, hub, suspension, and combined-brake work. |
| Inventory and clothing | Partial; tasks 3ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Å“10 stopped | Four slots and hand retrieval implemented. See the eight stopped tasks above. |
| Cleaning and painting | Partial; merged | Verify active tools and lifecycle behavior in the matching package. Complete refraction and brush depth fading. |
| Cargo and towing | Partial | Add general cargo locking, attachment objects, towing hooks, and trailers. Current storage locking covers flashlights. |
| Full item range | Partial | Add remaining source food, tools, containers, and their actions. |
| Animals and enemies | Missing | Add rabbits, human enemies, movement, attacks, and spawn rules. |
| Weapons | Missing | Add firearms, ammunition, reload actions, melee weapons, and projectiles. |
| Damage and destruction | Partial | Add wounds, blood loss, combat damage, breakable objects, and death ragdolls. |
| Complete save state | Partial | Preserve needs, food progress, sleep, storms, toilet state, and radio state; add save slots and autosave. |
| Settings and game setup | Partial | Add seed entry, source game options, editable controls, graphics, audio, and player settings. |
| Custom radio music | Missing | Add user music playback through the existing audio owner. Source radio playback already exists. |
| Multiplayer | Missing | Add the reference build's multiplayer behavior and shared state. |
| Remaining visual effects | Partial | Add soft storm particles, brush depth fading, fluid refraction, and source death effects. |

Generated identities and change records are in master. Complete save state requires the remaining gameplay owners too.
Source placement and imported geometry do not provide excluded item actions, vehicle mechanics, or enemy behavior.

Follow the [work order](#work-order). Source-world integration is complete; package and runtime acceptance remain open.

Implementation owners: [vehicles](vehicles.md), [save contract](../src/save/schema.rs),
[survival](../src/survival.rs), [weather](../src/weather.rs), and [flashlight storage](../src/interaction/storage.rs).
The selected source has no supported rain scheduler or active snow scheduler. Add neither without source evidence.

## Remaining game comparison

Implemented features still need comparison with matched running source views and inputs.
Check continuous walking, preparation, seat entry, sustained driving, ordinary aiming, and lifecycle behavior in the matching merged package.
Check assembled audio on a listening device. Compare rendered frames, particles, and physics under matched source conditions.
Retain the world sample, seam, rebase, revisit, object-count, and memory checks listed above.
Existing build, review, and historical test results do not establish full-game parity.

## Earlier rendering completion

The following checkpoints describe the earlier rendering package. They do not establish acceptance of the current merged game.
The user approved source rendering completion in an isolated worktree based on `dccd516`.
On 2026-10-12, the user authorized merging rendering commit `67798ec` into local master `a5b9778`.
Keep the accepted gameplay, road, library, menus, instruments, and day cycle.
The earlier merged package passed its recorded checks on 2026-10-13.

### Historical rendering checkpoints

| Layer | Evidence | Checkpoint |
| --- | --- | --- |
| Texture storage | Source storage audit, corrected exports, startup and pause checks | Included in shadow checkpoint |
| Shadows and sky | Three camera views, four main cascades, issued draws, GPU validation, upright image | `8182153` |
| Reflection probe | Six 64x64 faces, two mip jobs, validated publication, resize | `1b77a1a` |
| Motion producer | Camera and rigid motion fixture, matching submission tokens, resize | `538810e` |
| Camera effects | AO, TAA, blur, exposure, bloom, grading, dither, callbacks, resize and steady storage | `6148e66` |
| Stained glass | Both real door materials, two ordered overlap captures, probe views, resize and steady storage | `6f84766` |
| Lifecycle and streaming | Cold restart, readiness, terminal faults, probe distance, two actual rebases, fixed motion slots | Final branch commit |

The implementation uses exact 8-sample HDR rendering on the accepted hardware.
The WebGL backend uses attachment-only multisample renderbuffers and explicit resolves.
Shader consumers read the original D24 depth storage through a supported unfiltered float view.

### Historical package checks

Task 7 connects scene restart, view suspension, current-scene readiness, probe distance activation, and failure handling.
Healthy startup, pause, resize, restart, missing assets, invalid pipelines, and context loss passed native checks.
Restart resets scene histories and retains compatible storage. Terminal failure stops extraction, rendering, and history updates.
Whole-branch review found one motion streaming fault. The fixed-slot correction passed three critical tests.
Its scoped review and real-body native stages passed. Motion retained 780 GPU slots through streaming and both rebases.
The earlier package passed its native, installed-source, browser-interface, and exporter suites before the later test reduction.
Seven final export checks passed. The final default release package built and passed browser startup and Load.
Controlled lifecycle checks passed six stages. Both vehicle records survived a manual save and full page reload.
Both probes published. Three main views completed, and three mirror submissions passed GPU validation.
The merged motion pool retained 953 slots through the checked scene replacements.
The checkout no longer contains the execution ledger or verification record. Read their historical versions from Git.
Current renderer owners include [postprocessing](../src/presentation/postprocessing.rs), [probes](../src/presentation/probes.rs),
and [mirror rendering](../src/presentation/mirrors.rs).

### Rendering limits

- No matched native frame establishes identical pixels.
- Exact native blend rounding remains unverified.
- Equal-distance transparent ties use target identities. Native runtime identities remain unknown.
- Nine exported primitives lack authored tangents.
- Logical GPU byte counts exclude driver memory and overhead.
- Native operating-system minimize cannot be automated with the available browser tools.
- The in-app browser refused pointer capture. Continuous mouse control remains unverified for the final package.

The installed restart caller reloads the active scene with `LoadSceneMode.Single`.
That source proof resolves the former reload-versus-reset uncertainty.

Source vehicle, interaction, and Lada radio audio are implemented. Assembled audible acceptance remains incomplete.
See the [audio owner](../src/audio.rs) and [browser audio route](../web/audio.js) for the current implementation.
Master includes save/load, two source cars, and live mirrors.
Survival, sleep, and weather are now implemented in master. Multiplayer and deployment remain outside scope.
