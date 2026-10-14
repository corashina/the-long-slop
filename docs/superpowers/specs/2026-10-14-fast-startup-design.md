# Fast startup and streamed source content

Date: 2026-10-14, Europe/Warsaw.
Base: master `ee9d545847f15ffa95017d55fa4f42a0489e89bb`.
Status: The user approved the design direction. This written specification awaits review.

## Goal

Reach a playable garage within five seconds after a cached page load.
Measure first-download time separately.
Preserve the source world, selected vehicles, texture detail, lighting, interactions, and saved changes.
Use installed Steam build `11140890` as the behavior authority.

Load the garage and its required surroundings first.
Fetch other source content as the player approaches it or requests it through an interaction.
Keep the finite 50,000-section road and its sequential generation rules.

This design replaces startup requirements that wait for the complete road index or complete content catalog.
It also replaces permanent GPU slot reservations for disabled lights.
Other source-world and lighting requirements still apply.

## Evidence and limits

The following observations describe the inspected export and code at the base commit.
They identify work and allocation errors. They do not assign measured shares of the reported seven minutes.

| Observation | Evidence |
| --- | --- |
| Startup fetches and checks 7,184 world dependency files in sequence | `web/world-assets.js::admitWorldAssets` |
| Startup reads a 136,571,197-byte manifest before starting WASM | `web/main.js::load`; installed export |
| World source records occupy about 61 MiB of that JSON | Installed export, compact JSON measurement |
| Startup waits for 50,000 prepared road sections | `src/world/stream.rs::prepare`, `available_external_jobs`, and `window_ready` |
| The source prepares 100 sections at new-game startup | `level1:64771`, field `genAtStart`; `roadGenScript.FStart` |
| The source continues road generation during updates | `roadGenScript.Update`; serialized `genInOneFrame = 1` |
| Fixed light definitions reserve 27 light entries and 66 shadow faces | `src/presentation/lighting.rs`; installed lighting records |
| Only one of those fixed lights starts enabled | Installed lighting records; runtime controllers can change these states |
| The reported error combines light-entry and shadow-face failures | `src/presentation/world/lamps.rs::source_lamp_admission` |

The message's number 14 counts requested world lights. It does not state a hardware light limit.
The exact failed capacity and active light states require a runtime trace.

## Package and export contract

Keep one runtime package format and one loading path.
Replace the large `manifest.json` with a compact startup index at the same entry path.
Regenerate the installed export and test fixtures. Reject the old contract.

The startup index contains:

- Build identity, source hashes, package identity, and contract version.
- UI settings, startup scene references, world settings, and source generation parameters.
- Compact selection tables needed to choose content without loading its construction records.
- A catalog of metadata chunks, with canonical paths, byte lengths, and SHA-256 hashes.

Export required startup construction records into a startup metadata chunk.
Export other prototype construction records into shared or prototype metadata chunks.
Each chunk contains the records needed for its materials, collision, lights, LOD, and initialization.
Represent shared records once and reference them by stable source identity.
Group cyclic local references within one chunk. Keep chunk ownership edges acyclic.
Keep item selection weights and source IDs available before a random selection needs detailed construction data.
Export runtime field projections instead of complete serialized source records.
Retain fields used by supported behavior and resolve required transforms during export.
Exclude editor data and unused backreferences that would pull the complete scene graph into a chunk.
Check the projected data against the retained source evidence during export.

The exporter owns the complete source graph and validates its links before publishing the package.
Keep raw records and detailed evidence in ignored development storage.
The browser must not fetch that complete graph to start the garage.
Retain source identities and evidence hashes in compact runtime records.

Export encoded byte lengths, decoded dimensions, buffer lengths, and loader reservation estimates with each asset receipt.
Derive these values from the produced files during export.
Validate PNG layout, glTF structure, and dependency closure during export.

Root package identity covers the complete chunk catalog and its hashes.
Chunk hashes cover their asset receipts.
Keep save identity tied to this complete package, including content that has not loaded yet.

## Runtime ownership and loading

| Owner | Responsibility |
| --- | --- |
| Existing exporter | Validate source data; publish compact records, chunks, and receipts |
| Browser bootstrap | Fetch the startup index and initialize WASM in parallel |
| Runtime reference store | Own loaded immutable chunks and resolve stable source identities |
| Existing asset loaders | Reserve, fetch, validate, and decode selected assets |
| World stream | Request construction records and assets for the required region |
| Lighting owner | Map CPU light identities to current GPU allocations |
| Existing loading panel | Report required work, timings, and resource reservations |

Use an explicit reference state: pending, ready, or failed.
An unresolved chunk must not appear as an empty record or an absent source object.
Keep its region unpublished until its required records and assets are ready.
Keep controller state, generated identities, and saved changes independent of chunk residency.
Resolve record loads after source selection.
Retrying a load must not repeat random draws or initialization effects.
Retain referenced chunks while active objects or pending jobs use their records.
Release unused construction data through the existing retirement lifecycle.

Remove the full-catalog `admitWorldAssets` scan.
Build input-size and reservation tables from exported receipts without fetching asset payloads.
Remove the separate selected-glTF fetch used to inspect metadata before its real load.
Use exported construction and reservation data instead.

For a selected asset, reserve its metadata, decoded payload, staging window, and GPU allocation before fetching.
Read at most the declared encoded size. Reject excess or truncated input.
Check its SHA-256 and format limits using the same bytes passed to the decoder.
Reuse Bevy's PNG decoding and mesh support through explicit source loaders.
Do not register a source loader as a replacement for unrelated default loaders.

Keep normal browser HTTP caching. Key immutable content by package identity and asset hash.
Revalidate the small startup index so a new package selects new content.
Add no second catalog-wide verification cache.
Cancel stale loads by session and request identity. Do not publish late results.

## Initial world and continued generation

Export and use `genAtStart` and `genInOneFrame` from the selected source records.
Prepare the source's 100-section initial road sequence for a new game.
Prepare the terrain, collision, structures, and selected items required around the garage.
Allow compact numeric placement tables needed by source selection and recovery.
Keep detailed prototype graphs outside those tables.

Replace `indices_ready` as a global end-of-world gate with explicit prepared ranges and generation frontiers.
Keep spatial queries within prepared ranges.
Continue sequential generation from the current frontier using the source's batching and update rules.
Prepare the required surroundings before the player reaches their collision boundary.
Do not derive a fresh seed for each segment or advance a replay through the live random stream.

Keep sparse road and placement checkpoints.
Store the current frontier even when it falls between sparse checkpoints.
Use a separate replay state to revisit prepared regions.
Preserve building selection pools and shared Unity random ordering.

Keep supported recovery behavior, including nearest-supply queries.
Prepare missing recovery information as a cancellable job while retaining the current scene.
Do not label a partial search result as the globally nearest supply.

Retain the existing two-job and staging limits.
Reuse generation buffers and limit work per update.
Increase lookahead through preparation, without publishing incomplete collision or source state.

## Lights and shadow allocation

Retain CPU definitions and controller bindings for supported lights.
Separate their stable source identities from GPU slots.
Build the GPU set from current controller outputs, positive intensity, culling masks, and receiver influence.
Include active main-camera and mirror receivers.
Use receiver bounds and light influence, rather than the whole camera-frustum AABB alone.

Disabled lights consume no active light entry or shadow face.
Apply the global road-lamp state before deciding which road lights need GPU resources.
Allocate six faces for an enabled shadowed point light and one for an enabled shadowed spot light.
Keep the sun in its dedicated cascade path.
Release stale allocations before admitting replacements where ownership permits it.
Count real old/new GPU ownership during save replacement and deferred retirement.

Keep the existing uniform and atlas pool sizes for this allocation repair.
Treat them as limits on simultaneous GPU work, not total scene definitions.
Use reusable slot tables; change allocations when light membership changes.
Preserve source range, intensity, cookies, shadow resolution, and shadow behavior.
Do not omit enabled influencing lights or take an arbitrary first-N subset to pass admission.

Check device capabilities before loading heavy scene assets.
Check required light and shadow allocations as soon as the required scene descriptors are ready.
A genuine capacity failure must name the failed resource, requested count, available count, and relevant source IDs.
Keep the current valid scene during a failed replacement. Reject an invalid initial scene before its asset decoding starts.
The supported day, night, vehicle, garage, and nearby-world cases must pass these checks before delivery.
Actual capacity exhaustion in those cases blocks completion; increasing constants alone is not an accepted repair.

## Ready state and saved games

Use this startup sequence:

1. Fetch the startup index and initialize WASM.
2. Resolve the startup metadata and source-selected construction records.
3. Prepare the initial generation range and check renderer allocations.
4. Load required assets and publish complete scene, collision, and interaction state.
5. Prepare pipelines and effects required by the startup view.
6. Confirm presented frames and enable Continue.

Gate readiness on the current required region, its controls, and effects visible in the startup view.
Prepare distant material variants and other content when their regions need them.
Keep physics outside unresolved regions and retain the existing input gate during loading.
Do not hide missing startup objects or delay their visible effects to improve the measured result.

Add generation frontiers and sparse checkpoints to saved world state.
Bind them to the world seed, package identity, and generator contract version.
Validate checkpoint ordering, bounds, numeric state, and saved identities before committing a load.
Restore a distant location from its checked checkpoints and prepare its required region.
Avoid replaying the entire road as a prerequisite for that load.
Preserve changed objects and source initialization state through the existing save owners.

Increment save schema version 3 for the new world-state contract.
Reject older save versions with the existing incompatible-save message. Provide no migration path.
Keep an incompatible stored save intact unless the user replaces or deletes it.

## Resource and failure rules

Preserve existing CPU, GPU, staging, pending-work, and object limits.
Charge loaded metadata and shared chunks once per actual ownership.
Reserve decode and upload overlap before starting a job.
Release texture CPU reservations after the GPU completion receipt.

Wait for in-flight work when it can release the resource a pending job needs.
Distinguish that wait from a single asset or scene that cannot fit the configured budget.
Report a permanent failure before starting its expensive work.
Keep the source ID, file, stage, and requested/available values in diagnostics.

Progress counts only the current required work.
Show completed and required files, bytes, generation range, elapsed stage time, and resource reservations.
Retain the last measurements after failure.
Use explicit completion events; remove readiness conditions that depend on unused content.

## Measurement and acceptance

Record phase durations from navigation through the first complete, ready scene.
Include index fetch and validation, WASM fetch and initialization, metadata parsing, generation, decoding, upload, and pipeline preparation.
Record required request count, transferred bytes, cache hits, and peak logical CPU/GPU/staging reservations.
Record browser, renderer, viewport, seed, package identity, and release build identity with each trace.

Prime the package once, then run five normal cached reloads on the user's current machine.
Require each startup to reach the complete ready scene within 5.0 seconds.
Exclude human menu delay. Record the first presented frames and input response after Continue.
Report median and maximum durations.
Measure first-download and hard-refresh runs separately, including their transferred bytes.
Do not report compiled code or a menu alone as a successful game startup.

Measure cached save loading from Load request to committed, ready scene.
Use garage and distant-road fixtures with saved checkpoints and a small fixed set of changed objects.
Target 5.0 seconds for those fixtures. Report large save payload costs separately.

Implement only tests needed to protect this change:

- Assert startup requests exclude unused catalog payloads and the full source-record graph.
- Reject a selected asset with a wrong hash, wrong size, or invalid decoded dimensions before publication.
- Compare initial and checkpoint-restored generation with checked source samples and the existing sequential generator.
- Test enabled-light and shadow-face accounting, controller changes, mirror receivers, and save replacement overlap.
- Reproduce the reported 14-light case with its controller states and exact light/shadow diagnostics.
- Run timed cached starts, garage exit, driving across the initial generation boundary, and a distant saved load.
- Exercise day/night changes, garage lamps, vehicle lights, and repeated New Game/Load without lost source effects.

Preserve existing source validation gates. Passing startup timing does not establish full source parity.
Record existing unrelated test failures separately and compare them with the base commit.

## Delivery layers

Implement in a separate worktree based on current master.
Keep each layer working before adding the next layer.

1. Add phase measurements, reproduce the light case, and repair active light allocation and diagnostics.
2. Export size and reservation receipts. Replace full-catalog checks with validation during selected asset loading.
3. Split startup and prototype metadata. Load construction records through the runtime reference store.
4. Replace complete-road startup gating with source initial generation, continued preparation, and saved checkpoints.
5. Limit pipeline preparation to required content and verify the cached-start acceptance runs.

Remove each replaced runtime path when its layer passes checks.
Use the existing exporter, runtime dependencies, and loading panel.
Keep a custom archive format, new rendering backend, quality reduction, and unrelated gameplay changes outside this work.

## Design review

The approved direction covers the complete startup path and the reported light allocation failure.
The written specification adds package ownership, saved checkpoint rules, and measurable delivery gates.
The next step is user review of this file, followed by an implementation plan.
