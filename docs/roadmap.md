# Project roadmap

Updated: 2026-10-15. Source authority: Steam build `11140890` and its decompiled control flow.

## Current state

Master contains the source world, two cars, driving controls, rendering, audio, survival, weather, cleaning, painting, and inventory.
Save schema 5 holds world, driving, condition, and inventory state. Older schemas are rejected without migration.
Builds and static review pass. Runtime acceptance of the merged package is open.
Checked world inputs are restored in `.cache/world-source` (receipt: `recovery-receipt.json`).
Historical verification records are in Git: `git show 6272b02:docs/verification.md`.

## Work order

Keep each layer working before you add the next.

1. **Recover checked world probes and provenance.** Complete.
2. **Export and build one matching package.** Generate records, assets, and WASM from merged code. Verify hashes and record the package identity.
3. **Verify active gameplay.** Get mouse capture. Check walking, both cars, cleaning, painting, Save/Load, restart, seams, long drives, rebasing, revisits, memory, and audio.
4. **Measure source driving.** Collect original-game measurements before you change tire, rim, hub, suspension, or brake response. Then verify 30/60/120 FPS driving.
5. **Complete save coverage.** Save needs, food progress, sleep, storms, toilet state, and radio state. Then add save slots and autosave.
6. **Expand gameplay in layers.** Add cargo, items, fleet, damage, weapons, and enemies when authorized. Include save support and source comparison with each feature.
7. **Add multiplayer last.**

## Inventory and clothing tasks

Tasks 1 and 2 are done: four slots, storage, hand retrieval, source icons, and saved ownership.
Tasks 3-10 are stopped at the user's request. Do not resume them until the user says so.

| Task | Remaining scope |
| --- | --- |
| 3 | Equip/release transitions, direct Tab behavior, source item-action priority. |
| 4 | Station loot selection, source RNG order, item creation, stable identities, persistence. |
| 5 | Source box contents and one-time release, saved. |
| 6 | Shovel digging and buried-item recovery, saved. |
| 7 | Wearables, cigarettes, holiday startup. Keep disabled starter apparel disabled. |
| 8 | Both player skins, pose and IK, wearable anchors, camera visibility. |
| 9 | Appearance controls and remaining HUD behavior. |
| 10 | Compare with the running source game. |

## Feature gaps

Missing means no implementation. Partial means selected behavior only. Each new feature needs a source-based specification.

| Feature | Status | Remaining work |
| --- | --- | --- |
| World generation | Done; runtime unverified | Verify streaming, revisits, and memory in the matching package. |
| Buildings and exploration | Done; runtime unverified | Check buildings, interiors, landmarks, and collision. |
| World population | Partial | Source loot placement, abandoned vehicles, items, encounters. |
| Vehicle range | Partial | Vehicles beyond the Lada and Skoda. See [vehicles](vehicles.md). |
| Driving physics | Partial | Source measurements first, then wheel, rim, hub, suspension, and combined brakes. |
| Inventory and clothing | Partial | Tasks 3-10 above. |
| Cleaning and painting | Partial | Verify tools in the package. Add spray refraction and brush depth fading. |
| Cargo and towing | Partial | Cargo locking, attachments, towing hooks, trailers. |
| Item range | Partial | Remaining food, tools, containers, and actions. |
| Animals and enemies | Missing | Rabbits, human enemies, movement, attacks, spawn rules. |
| Weapons | Missing | Firearms, ammunition, reload, melee, projectiles. |
| Damage and destruction | Partial | Wounds, blood loss, combat damage, breakable objects, death ragdolls. |
| Save state | Partial | Work order step 5. |
| Settings and game setup | Partial | Seed entry, game options, controls, graphics, audio, player settings. |
| Custom radio music | Missing | User music through the existing audio owner. |
| Multiplayer | Missing | Shared state. Last in the work order. |
| Visual effects | Partial | Soft storm particles, brush depth fading, fluid refraction, death effects. |

The source has no supported rain or snow scheduler. Do not add either without source evidence.

## Known limits

- A loose flashlight fell below the floor during a long browser run. Cause unknown.
- The in-app browser refuses pointer capture, so mouse control and driving are unverified.
- At zero wheel RPM the port can keep neutral. Source curve extrapolation is unverified.
- The stock solver does not apply motor force and brake impulse to one wheel at the same time.
- No matched native frame proves identical pixels. Nine exported primitives lack tangents.
