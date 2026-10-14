# Cleaning and Painting Design

Date: 2026-10-14, Europe/Warsaw.
Status: scope approved in chat. Written specification awaits review.
Base: c189df3. Implementation requires an isolated worktree and an approved implementation plan.

## Goal and scope

The player can clean and paint eligible surfaces on the current Lada, Skoda, and garage items.
Use source brushes, sponges, and spray cans through the existing pickup and held-item controls.
Restore authored startup placement, condition changes, tool wear, paint use, sounds, and particles.
Preserve these changes through carry, part removal, attachment, origin shifts, recovery, save, and load.
Restart restores exported startup state.

This feature completes the Cleaning and painting row in `docs/roadmap.md` after acceptance.
It does not add inventory slots, new vehicles, loot generation, weapons, or a color selection interface.
Use only source-enabled startup tools. Recover inactive prototypes as evidence without adding an acquisition route.
Preserve other chats' files and worktrees.

## Source authority and evidence

Use Steam app 1017180, build 11140890, and Unity 2021.3.18f1.
Read the accepted library through its existing receipt and hash checks.
Keep Steam files, the accepted source archive, and recovered C# unchanged.
Keep generated assets and recovery records outside Git.

The accepted managed file is `.cache/reference-library/records/managed/Assembly-CSharp.cs`.
Its recorded SHA-256 is `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.
Check this hash before extracting implementation evidence.

| Source controller | Required behavior |
| --- | --- |
| `partconditionscript` | Repair permissions, progress, condition steps, color, material selection, and child propagation |
| `drotkefescript` | Cleaning type, target resolution, minimum force, repair rate, wear, and disabling |
| `objSlideOnColliderScript` | Contact motion, force, model pose, sound, and particles |
| `sprayscript` | Target ray, valve, color blend, paint use, particles, and sound |
| `ammoscript` and `colorscript` | Spray quantity, initial color, and ownership |
| `washtriggerscript` | Wash permissions, progress, object activation, and completed cleaning |
| `itemdatabase` and `mainscript` | Repair tables, cleaning tables, and shared repair rate |
| Player held-item handling | LMB precedence, pointer capture, tool aim, and release |

The selected `mainscript` record is `level1:64919`; its `repairValueIncrease` is 0.5.
The exporter currently selects condition materials once in `tools/reference/conditions.py`.
`ItemState.condition` and saved item condition are scalar values; they do not represent the complete source state.

Export startup tool identities, enabled ancestors, authored transforms, pickup links, bodies, colliders, and controller links.
Export source repair tables and every parameter used by the selected controllers.
Export condition renderers, materials, meshes, active-object variants, child links, and inheritance flags.
Include eligible wash targets in the selected scene hierarchies and their linked visible objects.
Resolve part-slot and `partconditionforward` links when the selected hierarchy uses them.
Fail export on unresolved required links, invalid indices, cycles, or unsupported in-scope data.
Name the source identity and field in each error. Do not replace missing evidence with guessed values.

## Runtime ownership and module boundaries

Add typed condition and tool records in `src/reference/conditions.rs` and `tools/reference/conditions.py`.
Use stable source identities for bindings and numeric indices for active updates.
Keep startup derivation and runtime material selection consistent with the same source flags.

Add `src/conditions.rs` for mutable condition, repair progress, color, and child propagation.
Store source condition integers, `state2`, and `state3` without collapsing them into a quality scalar.
Keep source type selection and material bindings as retained reference data.
Derive any physics quality value from the owning condition record when that source condition controls it.
Keep unrelated physical properties independent.

Add `src/interaction/cleaning.rs` for held tools, contact state, and source action rules.
Reuse `ItemStore`, carry physics, aim queries, input, and feedback.
Resolve hit collider ownership once during scene loading. Do not search scene hierarchies each frame.
Retain tool quantity, tool condition, and source color with their item identity.
Track wash-target progress and activation with the wash controller identity.

Add `src/presentation/conditions.rs` for renderer bindings, dirty updates, and tool presentation.
Use the existing source material, audio, and particle systems.
Keep rendering state separate from gameplay state. Rendering must not perform repairs or consume paint.

## Cleaning behavior

Hold LMB to use a held brush or sponge with pointer capture active.
Use source distance, layer masks, target ownership, model pose, and contact offset.
Apply source precedence: wash target, eligible installed part, ancestor condition, then explicit condition forwarding.
A blocking surface stops the query. A nearby unrelated condition is not an eligible target.

Match the source update order between tool pose, sampled contact motion, force, and repair.
Measure movement relative to the contacted target. Do not count target translation as rubbing.
Use source `InverseLerp(minSpeed, maxSpeed, speed)` for force and the strict minimum-force check.
The source speed is displacement per update, not metres per second.
Sample contact movement once per active presentation frame; do not repeat it in fixed-step catch-up updates.
Use the active frame delta for source repair accumulation.
Reset contact samples after load, recovery, origin shifts, target changes, or loss of contact.
Reset samples without adding repair progress or causing tool wear.

Apply `repairModifier * repairValueIncrease * deltaTime` only when source contact and force rules permit it.
Read permissions from the source table for the target property, condition, and cleaning type.
Normal repair accumulates progress; at progress >= 1, improve one condition step and reset progress to zero.
The type-1 damage branch has source precedence when `CanRepair(3)` permits it and condition is below 4.
That branch subtracts progress; at progress <= -1, worsen one condition step and reset progress to zero.
Do not retain threshold overflow or apply extra condition steps in the same source call.

On a completed repair or damage step, apply source tool wear when `canRuin` permits it.
Source wear increments tool condition through 4; the next wear event sets 5 and disables the tool.
Do not treat disabled tool state 5 as a normal surface material index.
Apply the source disabled model, prompt, force, sound, and particle behavior.

Wash targets use their source permissions and separate progress.
Match their strict progress > 1 completion rule, `neverRepair`, activation changes, and source destruction rule.
Apply tool wear only when the source wash completion calls it.

## Painting behavior

Hold LMB to fire the selected spray can while pointer capture is active.
Use the head ray for source hand use and the authored nozzle ray for source carried use.
Recover the existing carry-mode mapping before binding these two paths.
Use source distance and layer masks. Resolve ancestor condition, then the pickup's explicit condition link.
Apply painting only when `CanRepair(0)` permits it.

Update the valve with source clamped interpolation toward 1 while firing.
If paint remains, consume `paintLoss * valve * deltaTime`, including shots that miss an eligible surface.
Use the source quantity clamp and empty-can behavior.
Blend the target color toward the can color with the source clamped interpolation factor:
`colorLerp * deltaTime * InverseLerp(distance, 1, hitDistance)`.
Keep the source distance order. Do not replace it with a linear distance ratio.
Recover valve release, emission, volume, and can-color startup rules from the checked controllers.
Keep the can label, spray particles, and paint color consistent with that state.
Do not add infinite paint, instant recoloring, or a new control key.

## Condition refresh and presentation

Match source `Refresh` material selection, color flags, and child propagation.
Retain `UseMatList`, worn-material rules, material type overrides, `disableColor`, and `forceColor` behavior.
Retain child state, color, mesh, and active-object rules without propagating through unrelated parts.
Refresh detached parts through their own condition owner.

Prepare required condition variants, bindings, and mutable material instances before Start becomes available.
Retain per-owner instances so painting one part cannot recolor an unrelated part sharing the original material.
Update existing material values or handles only after state changes.
Reuse meshes, textures, materials, audio voices, particle pools, and scratch storage.
Do not create GPU assets, scene bindings, or collections during frame updates.
Keep source shadow, mirror, reflection, and motion bindings valid after material or mesh changes.
Include condition variants in the existing loading and renderer-readiness checks.

## Lifecycle and save state

Pause freezes repair, wear, paint use, valve updates, audio, and particles through the existing active-play gate.
Release, drop, and loss of pointer capture stop tool work and apply source release behavior.
Recovery preserves condition, color, repair progress, tool wear, and remaining paint.
Origin shifts preserve these records and reset contact-motion samples.
Restart rebuilds startup records and finishes renderer preparation before play resumes.

Extend the current save snapshot with condition, wash, and tool records keyed by stable source identity.
Save the current state integers, color, partial repair progress, tool wear, disabled state, and paint quantity.
Retain active wash targets and source destruction state without recreating cleaned objects on load.
Store semantic state; recompute transient contact, sound, particles, and firing state after load.
Restore condition state and presentation before enabling player input.
Reject duplicate identities, missing owners, invalid state indices, non-finite values, and invalid quantities.
Bump the save schema for the changed contract. Add no migration or old-schema reader.

## Working layers

1. Export and validate live condition records and variants. Confirm startup appearance and existing gameplay.
2. Add condition refresh and save round trips. Confirm isolated material changes and lifecycle behavior.
3. Add brush and sponge use, contact motion, wash targets, wear, and feedback. Confirm a complete cleaning route.
4. Add spray use, color blending, depletion, and feedback. Confirm a complete painting route.
5. Check both cars, removable parts, save/load, repeated restart, and renderer readiness. Update scope documents after acceptance.

Keep each layer usable before adding the next. Do not remove working paths before their replacement passes its checks.
Remove obsolete startup-only condition and duplicate state paths when the live system passes acceptance.

## Verification and acceptance

Use critical tests for source permission tables, repair thresholds, damage precedence, wear, and paint depletion.
Test child propagation, material isolation, invalid reference data, and save/load validation.
Run the project release build, required reference checks, and relevant existing interaction/save tests.

In the browser, pick up each source-enabled tool and use it on both cars and eligible garage items.
Check stationary contact, rubbing, moving targets, blocked rays, unsupported surfaces, tool wear, and an empty spray can.
Check source surface colors and condition variants through each supported step.
Remove and reinstall a changed part. Save a partly repaired surface and a worn tool.
Carry and drop a spray can with remaining paint.
Check pause, recovery, origin shift, save/load, and repeated restart without false rubbing or lost state.
Check visible tool poses, source sounds, particles, shadows, mirrors, and startup renderer readiness.
Compare matched source actions and views before claiming source parity.
Build success and synthetic tests alone do not prove source or visual parity.

Record evidence and limits in `docs/verification.md`.
Update `PRODUCT.md` and the roadmap only for accepted behavior.
Keep the row partial if a required tool, source rule, lifecycle check, or visible behavior remains unverified.
