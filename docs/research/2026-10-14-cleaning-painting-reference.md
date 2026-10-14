# Cleaning and painting source records

Date: 2026-10-14. Steam app 1017180, build 11140890, Unity 2021.3.18f1.

We implemented startup records, live condition state, cleaning, and spray painting on `codex/cleaning-painting`.
Reviewed code head: `6ba18866f20e00126feafcf4aa1863b3b50bde8c`.
We retain Partial status until active gameplay and matched source checks pass.
Build checks do not prove source parity.

## Source authority

The accepted library is a read-only input from the primary checkout.
`LibraryReader` checks the manifest, catalog, typed records, and selected payloads.
`load_snapshot` and `verify_library` passed before recovery.
The production `derive_reference` API writes to the isolated worktree.
Its atomic promotion and `verify_reference` checks passed.

| Input | SHA-256 |
|---|---|
| Accepted library manifest | 92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99 |
| Source catalog | ba572df2c46e84c2839c805655c252acb3ef8964a776c2eeb378bb3de0b1b13f |
| Source manifest | 254830925305facc97113479ecdea6f154389ed354afa42aa996ead492d5c04d |
| Managed C# | 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee |

## Startup tools

Terrain controller `level1:64898` enables GameObject `level1:117` through `EnableAfterStart`.
That group is serialized inactive. The six selected children are serialized active.
Recovery adds those roots before material, geometry, condition, lighting, and audio recovery.
It preserves their source transforms, pickup, body, and save links.
The duster and player PointAtObject controllers stay outside this scope.

| Tool | Controller | GameObject | State links |
|---|---|---|---|
| Brush 1 | level1:66615 | level1:10842 | slide 68227; condition 77707 |
| Brush 2 | level1:66617 | level1:10846 | slide 68224; condition 77709 |
| Sponge 1 | level1:66616 | level1:10838 | slide 68226; condition 71298 |
| Sponge 2 | level1:66619 | level1:10849 | slide 68225; condition 71299 |
| Spray 1 | level1:73834 | level1:10841 | ammo 78225; color 73209 |
| Spray 2 | level1:73835 | level1:10840 | ammo 78223; color 73210 |

Brush cleaning type is 1. Sponge cleaning type is 2.
Both use `canRuin=true`, `repairModifier=1`, distance 2, and mask 8552449.
Minimum displacement is 0.009999999776482582.
Maximum displacement and minimum force are 0.10000000149011612.
The shared repair rate is 0.5 from `mainscript level1:64919`.
Sprays use distance 3, mask 8552449, colorLerp 0.25, and paintLoss 4.5.
Their valveSpeed is 10. Their particle rate is 250.
Both ammo controllers use float quantity 100 and maximum 100.
The source save startup excludes sprays from ammo randomization.

The existing fixed build hash policy selects unset colors.
Spray 73834 selects `car10paint`, index 9: RGBA [0.6509804129600525, 0.125490203499794, 0.11764706671237946, 1].
Spray 73835 selects `busfem`, index 1: RGBA [0.9019607901573181, 0.9019607901573181, 0.9019607901573181, 1].
Records retain own colors, type palettes, the selected palette, and the selected index.
Native Unity random sequence parity is untested.

## Condition contract

`ReferenceManifest.conditions` is required. It has no schema fallback.
The export contains 122 surfaces, 58 wash controllers, four brushes or sponges, and two sprays.
The early scene query found 119 conditions. Existing flashlight and spare-wheel roots add three conditions.
They are level1:76004, level1:71403, and level1:78094.
All condition owners belong to the cars or item scene. No prototype conditions are exported.
The export has 518 collider bindings, 663 colliders, and 232 items.
Surface records retain startup state, state2, state3, progress, color, palettes, material rules, and child flags.
They bind checked material states, meshes, linked objects, and renderer nodes.
All selected UseMatList, UseMesList, and UseGoList flags are false.
Their empty source lists remain valid. Supplied variant bindings still receive strict validation.
Tool-disabled state 5 belongs only to its disabled brush controller.

Repair permissions come from the checked `itemdatabase.repairProperties` initializer: eight tables, five states, seven types.
Cleaning permissions come from `itemdatabase.cleanProperties`: four tables, seven types.
These tables are absent from serialized itemdatabase records.
Recovery checks the managed hash and decompilation receipt before reading the scoped class initializers.

Collider records keep ancestor, pickup, forwarded, installed-slot, and wash owners separate.
General parts use PartSlotId. Road wheels use WheelSlotId.
Tire slots use their rim ItemId and the checked wheel_parts tire_slot_anchor.
The source part-slot controller identity remains in each installed-slot binding.
Validation rejects conflicting slot kinds. Wheel slots do not enter the general-part table.
The selected closure has no partconditionforward controller.

Variants reuse existing source material keys, shaders, textures, meshes, audio voices, and particle records.
Asset mesh_count supports checked mesh index validation.
Particle resources enter existing dependency and path checks.
`conditions-evidence.json` has a byte count and SHA-256 receipt in `library-selection.json`.
Verification checks this receipt and equality with runtime condition records.

## Historical Task 1 checks and limits

Python source checks passed: three tests.
Rust condition checks passed: seven tests.
Rust reference checks passed: sixteen tests; two installed-export tests are ignored by that target.
The production export runs the Rust manifest validator on real records.
The final package passed the release build. Browser evidence is recorded below.

Existing full-check failures remain outside this layer.
The web input assertion expects old toilet bit values.
Global formatting fails in unrelated files. The full native suite uses obsolete APIs.
The controller approved preserving those failures.
The renderer prerequisite preserves source sort metadata in streamed recipes.
That separate fix passed independent review and browser startup checks.
Active input and source pixel parity need separate browser evidence.

## Historical Task 1 export and package

The final release build passed in 6 minutes 12 seconds, exit 0.
It includes all final validation fixes. The contract is frozen.
The real export passed final contract and resource verification after those fixes.

| File | SHA-256 |
|---|---|
| manifest.json | e36c3d4d8cb983645b273085f4e2e8f3b563de9cb3a18eb1c2c9d7a1de6b3563 |
| conditions-evidence.json | 962b9b9fd8acf0b7fa6ebdf36bf08307d58ca34289966703c7e5e9daa1c0ba4f |
| library-selection.json | 95cd2ceabeb48e1157fe46c1a824911af991999e9aa4791eea32dbbad3db5397 |
| items.gltf | 971d71969c9cb162c0e1e5fa4ffa7513d058633bad37614d956025d87ff716dd |
| items.bin | 5f4e6192b517ee8ca07615357e56d5241e139f0cac72ecae2fc3b71cb2527a06 |
| my_summer_slop.js | 8ed305a677311519837b790023f91c9c07124e1dc61fdd7578ab2633383e9261 |
| my_summer_slop_bg.wasm | 5936d71bb5db56655b825ee3432726193993d06696452c872e6bb155f271fc8e |

The controller proved garage and New Game readiness on the preceding package with the same source schema.
Captured console errors were empty. Individual visible pickup, walking, and driving remain unverified because IAB cannot provide pointer lock.
At Task 1, the controller had not checked the final package. Later package evidence follows below.

## Integrated runtime rules

The runtime stores surface, wash, and tool state under checked source identities.
We removed duplicate scalar ownership. We retain the checked startup initializer.
Schema 2 stores surface state, state2, state3, color, and partial repair progress.
It stores wash activation, destruction, progress, tool wear, disabled state, and can quantity/color.
The loader rejects old schemas, source mismatches, invalid palettes, completed surface steps, and inconsistent disabled tools.
Load clears contact and valve output. Recovery preserves semantic state; New Game restores exported startup values.

Brushes and sponges use the held hand-use ray and checked cleaning type.
Minimum force, displacement, repair rate, tool wear, and wash activation follow the checked records.
We guard stationary contact against progress from visual settling. This port invariant needs matched source comparison.
Worn tools retain the slide model and hide the source scrub node.
Wash collider activation follows the source object state and each authored collider enable flag.
Wash ON and object activation remain separate states.

Carried spray uses the checked nozzle pose. Fire opens the valve, consumes paint, then casts its ray.
The last shot can paint after depletion. It starts no new sound or particles after quantity reaches zero.
Spray selects an ancestor condition, a current checked installed-slot owner, or an explicit pickup condition.
It does not use brush wash lookup. An empty slot cannot paint its former owner.
Painting checks permission column zero and the clamped distance factor before blending the saved color.
Part removal and installation keep that part's condition owner.

Both tool rays include enabled source-mask sensors.
The exporter checks `PhysicsManager globalgamemanagers:10 m_QueriesHitTriggers=true`.
We found 25 selected non-wash sensors in brush mask 8552449.
Ammo 78225 and 78223 use hideLookAtCount=false, hideLookAtCountInInventory=true, and noPerShow=true.
Look-at quantity follows the aimed can, independent of the held tool.
The HUD displays bare rounded quantity. Its cache follows the same integer boundary.
Carried cans hide quantity and the native Paint prompt.
Brush, Sponge, and Paint are port item labels; Wash and Scrub are checked native prompts.

The renderer prepares two retained material banks per owner before readiness.
Candidates use the inactive bank; failed candidates preserve live state and bindings.
Color updates write retained receiver and caster buffers through RenderQueue.
The existing weather and audio owners prepare and update tool effects.
The implementation adds no frame material, buffer, binding, or effect pool setup.

## Final acceptance package and limits

The final Task 4 fix package uses code head `6ba1886`.
We reuse its successful release build for documentation changes under the approved build reuse ruling.
Task 5 checks package hashes before and after source verification.
See [verification](../verification.md) for final hashes, commands, counts, and baseline failures.

The controller reached garage readiness, New Game, and current-export Load with enabled controls.
The controller's fresh Continue attempt showed `Mouse capture failed` and retained pause.
Chrome remains absent from tool inventory; IAB cannot capture the pointer.
The current error-level log query returned no entries. Startup event capture remains incomplete.
Bevy can publish warnings at log level; this query does not prove warning-free operation.

Earlier Task 2 diagnostics proved candidate isolation, material isolation, and retained GPU identities on their named packages.
Across Load and New Game, they retained 1118 meshes, 122 owners, and 168 receivers.
They retained 1680 condition materials, 2266 source materials, and transparent capacity 5812.
Those packages predate spray effects. We have no final-package active particle or binding growth measurement.
We removed cleaning diagnostic routes before the final default package.

The required route remains open: six tools, eligible surfaces on both cars, changed removable parts, and sustained driving.
It includes wheels, doors, fluids, lights, plates, storage, death gating, recovery, save/load, restart, and origin changes.
Matched controls, surfaces, tool state, camera, main view, shadows, mirrors, reflections, and audio remain open.
The fixed build hash color policy does not reproduce Unity's random sequence.
Whole-branch review and branch integration remain controller-owned gates.
