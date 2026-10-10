# Interaction comparison

Date: 2026-10-10, Europe/Warsaw.
Stage: Research for the requested interaction update. Product code has not changed.
Reference: installed The Long Drive Steam build 11140890.
Base commit: d4cdefc.

## Intent

The user wants useful interactions with the current home, car, and supplies.
The user requested a separate worktree and permitted subagents.
Two subagents inspected recovered code and the current interaction path.

## Confirmed gaps

| Gap | Current implementation | Recovered source |
| --- | --- | --- |
| Pickup uses the use target | `src/interaction.rs:358` selects with the use mask | `.cache/reference/fpscontroller.cs:4318` uses useLayer; line 4645 uses pickupLayer |
| Can caps reject pickup | `src/starting_area.rs:409` selects the nearest item; `src/interaction.rs:788` rejects Cap | `fpscontroller.cs:4647` searches parent pickupable components |
| Wheel centers can select inactive hubcaps | Decoration items 23, 26, 29, and 32 own colliders; wheel rewriting covers wheel-part colliders | `fpscontroller.cs:4330` searches the part owner before removal |
| Carry movement lacks translation and roll | `src/interaction.rs:848` changes distance; line 857 changes yaw and pitch | `fpscontroller.cs:4118` rotates; line 4121 rolls; line 4148 moves; line 4167 changes distance |
| Wheel removal leaves the wheel on the ground | `src/starting_area.rs:546` removes without pickup | `fpscontroller.cs:4338` removes, then line 4340 picks up |
| Cabin controls have no runtime actions | `src/interaction.rs:32` has no generic slider or switch action | `fpscontroller.cs:4383`, 4405, and 4424 handle rotation, sliding, and switches |
| Export omits cabin mechanisms | `tools/reference/manifest.py:79` filters home controls; lines 81 and 90 restrict mechanisms | Recovered usablescript records include windows, mirrors, glovebox, ashtray, and visors |
| Part removal covers wheels | `tools/reference/manifest.py:132` exports wheel parts; runtime accepts selected item kinds | `fpscontroller.cs:4330` handles part slots; `partslotscript.cs:248` checks compatibility |
| Push and throw have no input path | `src/interaction.rs:573` processes F presses; no push or charged-throw action exists | `fpscontroller.cs:3502` charges throws; line 5149 applies force; line 4891 handles pushing |

Source masks: use 12747265; pickup 8388609; attachment 8552449.
The generated parameter evidence records these values under `level1:80125`.
The manifest contains 19 decoration items. This count includes the home root and several car parts.
A decoration label does not prove that the original object has a gameplay action.

## Existing scope

The approved garage specification covers doors, caps, fluids, seats, carrying, and wheels.
It permits home decoration as geometry. It excludes survival, weapons, other vehicles, saves, multiplayer, and deployment.
The exporter also omits window and mirror adjustment. Audio remains outside the current stage.
The new request can extend interactions for the selected home and Lada.
Any extension needs explicit scope in the new design.

## Proposed layers for review

1. Resolve use, pickup, and attachment owners from source components and masks.
2. Use the same resolved actions for input and prompts.
3. Complete carry movement and place removed wheels in hand.
4. Add recovered rotation, sliding, and switch actions for the current home and cabin.
5. Add source-compatible removal and installation for the current Lada parts.
6. Add push and throw using recovered rules.

Each layer must leave the preparation and driving flow working.
Keep walls as aim blockers. Resolve parents on the hit object; do not select targets through walls.
Keep item identity, fluids, condition, and mass across attachment changes.
Cache component bindings during load. Avoid per-frame allocations and repeated setup.
Remove obsolete item-kind action filters after explicit bindings replace them.

## Alternatives

Repairing selected targets costs less but leaves the requested missing mechanisms inactive.
Adding source component bindings supports the current home and car without full-game expansion.
Full-game interaction parity would include excluded systems and exceeds this task's proposed scope.

## Verification needed

Critical checks must cover cap-to-container pickup and hubcap-to-wheel removal.
Check occlusion, wrong-slot rejection, part identity, fluid conservation, restart, pause, and origin changes.
Check detached engine and radiator effects on vehicle operation.
Run formatting, native tests, exporter tests, installed-source checks, and the WASM build.
Verify a continuous browser route with common aim positions, then drive through the garage exit.
Existing source tests stage clear player positions. They do not establish ordinary browser aiming or a continuous walking route.

## Research limits

The input bits match between JavaScript and Rust. Research found no global input-loss defect.
This comparison uses recovered code and exported assets. It does not establish identical Unity physics.
No product tests or browser checks ran during this research stage.
The worktree is `C:/Users/Corashina/.codex/worktrees/interaction-parity/the-long-drive-wasm`.
Recovered C# and assets remain ignored in the primary checkout.
