# Local startup and bounded texture uploads

Date: 2026-10-17, Europe/Warsaw.
Base: `ecd2ed3` on `codex/startup-physics`.
Status: Written specification for user review. Implementation has not started.

## Intent and scope

Reach a complete, playable garage within 30 seconds on this machine with cached assets.
Measure uncached downloads and first shader compilation separately.
Preserve build `11140890` geometry, texture detail, lighting, interactions, and source generation order.
Keep the player and vehicles on published collision.
Freeze simulation and retain the current poses during pause.

The earlier approved design, `4052a9c`, selected source-style initial generation and required-region readiness.
This specification applies that direction to the current loader and the new 30-second target.
Do not repeat completed work on manifest parsing, file receipts, or active light allocation.

The current branch contains separate fixes for source-parent spawn alignment and player pose synchronization.
Its installed-scene check confirms player ground contact and upright vehicle settling.
Browser movement and pause acceptance remain open because startup still fails.

## Current evidence

- The source record `level1:64771` sets `genAtStart = 100` and `genInOneFrame = 1`.
- The runtime waits for the complete 50,000-section index before local preparation.
- A cached test spent about 18 seconds on that index alone.
- The current package contains 1,977 `.smip` files.
- World texture uploads load and acknowledge individual chunks before starting the next chunk.
- Local collision became supported while `window_ready()` still waited for the larger selected world.
- The browser then failed texture admission for `sharedassets1.assets:21`.
- That check requested 208,666,528 GPU bytes against 203,970,300 available bytes.

These timings came from the diagnostic release build with LTO disabled and 16 codegen units.
The standard release build also passes compilation.
Neither result establishes playable startup or the 30-second target.

## Approach

Use source-style local startup and bounded texture batches.
This reduces initial work and removes repeated small-file upload waits.

Retaining full preload and adding a cache would preserve the large initial GPU requirement.
It would also retain first-run work and introduce another persistent representation.
Use the local startup approach already selected in the earlier design.

Do not reduce texture detail, source distances, object counts, effects, or graphics quality to meet the target.
Do not increase resource limits to pass a scene that has not been scoped to required content.

## Initial generation and required content

Export and consume the source's startup and continued-generation counts.
Generate the initial 100 road sections before first play.
Continue one section per ordinary application update until the finite source end.

Replace the global `indices_ready` gate with prepared ranges and a generation frontier.
Spatial queries must distinguish an unprepared range from a prepared range with no result.
Keep the existing sequential generator and a separate checkpoint replay state.
Preserve building selection pools, source identities, Unity random draws, and initialization effects.

Define one required-content set for startup.
Include authored garage objects, selected startup objects, local collision, active-view geometry, and influencing lights and casters.
Include source effects and controls required by those objects.
Do not omit visible startup objects or replace them with placeholders.

Prepare safe local collision before procedural content outside the required set.
Let the existing streamer prepare distant content after startup.
Keep physics inside supported bounds while the required range grows.
A player or vehicle must stop before an unresolved collision boundary.

Treat required content as explicit identities and ranges.
Do not infer readiness from total catalog completion or elapsed frame counts.
Use the same required-content contract for first start, New Game, Load, and recovery.

## Texture batch package

Keep existing checked raw mip chunks.
Add exported batch descriptors that group consecutive chunks from one texture layer.
Use a maximum decoded payload of one MiB and at most 32 chunk entries per batch.
Keep large row chunks separate when their decoded payload already fills that bound.
Group small tail mips into one batch.

Each descriptor records ordered source receipts, dimensions, mip levels, rows, and decoded byte ranges.
The exporter validates coverage, order, non-overlap, and exact byte totals.
Store descriptors as content-addressed `.stexbatch` files.
The file catalog covers descriptor sizes and hashes.
A new package identity covers the new descriptors.

Register `SourceTextureBatchLoader` through the existing source asset IO.
Its `SourceTextureBatch` asset contains the validated regions and their decoded payload.
Validate each receipt before decoding or publication.
Use one reusable bounded payload buffer per batch owner.
Read chunks in descriptor order and retain their source identities in failure messages.
Do not create a temporary GPU image for each raw mip chunk.

## Upload ownership

Keep one current upload batch and at most one prefetched batch.
Charge both owners against the existing job, CPU, and staging limits.
Their combined resource reservations must fit before either request starts.
Keep at most two source IO jobs active across fixed and world content.

Upload the current batch's row and mip ranges to its destination array.
Acknowledge the batch after the GPU submission completes.
Release its CPU payload and input handles after that acknowledgement.
Promote the prefetched batch only after the current owner releases its slot.

Use explicit queued, loading, ready, submitted, complete, and failed states.
Cancellation retains submitted ownership until its completion receipt arrives.
Session and request identities reject late results.
Share decoded batches through strong asset handles. Count shared bytes until the last owner releases them.
Shared source data must remain valid while another session uses it.

Remove the old per-chunk world-array upload path when this layer passes its checks.
Keep unrelated fixed-image loading on its existing path.

## First start, replacement, and pause

Reuse the already prepared fresh session for the first New Game commit.
Represent its preparation as a permit tied to that session and generation.
Discarding the permit must retain the prepared fresh scene.
Committing the permit enables the existing input transition without regenerating the same world.
Remove the duplicate initial-session preparation path.

Later New Game and Load operations retain the current candidate-and-commit lifecycle.
Keep the valid current scene while a replacement prepares or fails.
Apply the required-region contract to each candidate before commit.

Apply startup offsets to authoritative player state, physical bodies, and camera interpolation endpoints together.
Retain the existing pause guard and reset accumulated frame time on resume.
Loading and renderer preparation must not advance paused simulation.

## Saved world state

Increment save schema 5 to 6 for the generation frontier and partial index contract.
Save the exact frontier state, sparse checkpoints, seed, generator contract, and package identity.
Keep changed objects and initialization state under their existing owners.

Restore a saved range from its checked frontier and checkpoints.
Do not regenerate the entire finite road before restoring a distant location.
Prepare unresolved recovery searches while retaining the current scene.
Do not report a partial search as the globally nearest result.

Reject incompatible saves through the existing error path.
Preserve their stored bytes until the user replaces or deletes them.
Add no migration or old-format runtime path.

## Limits and failures

Retain the current shared limits: two jobs, 32 pending requests, and 32 MiB of staging.
Retain the current world and asset CPU and GPU limits.
Count shared ownership once and count real replacement overlap.

Wait only when an in-flight owner can release the required resource.
Report a permanent admission failure before its expensive work starts.
Include the source identity, stage, requested bytes, and available bytes.

Publish complete geometry, collision, interaction state, and required renderer work together.
Confirm two presented frames for the complete required scene before enabling input.
Keep failed startup disabled and preserve its measurements.

## Delivery and acceptance

Keep each layer working before adding the next layer.

1. Implement source frontiers, schema 6, required-region preparation, and local collision readiness as one complete layer.
2. Export batch descriptors and replace world-array chunk loading with bounded batch ownership.
3. Reuse the first prepared session and verify the complete browser lifecycle.

Implement only critical checks for checkpoint equivalence, batch byte coverage, resource ownership, cancellation, and save replacement.
Use the installed-scene spawn regression to retain the physics fixes.

Prime the package, then measure five normal cached starts with the standard release build.
Require each complete startup to take at most 30 seconds on this machine.
Report the median and maximum.
Include the first New Game commit and input response, excluding human menu delay.
Measure first-download and uncached shader preparation separately.

Verify walking, jumping, garage exit, vehicle settling, driving, and the initial generation boundary.
Pause during movement and in mid-air. Confirm unchanged simulation poses and step counts while paused.
Compare absolute source poses, including retained dormant poses.
Resume without a catch-up jump.
Verify repeated New Game, garage Load, a distant Load, and cancellation during texture preparation.
Do not call the task complete until browser timing and these runtime checks pass.
