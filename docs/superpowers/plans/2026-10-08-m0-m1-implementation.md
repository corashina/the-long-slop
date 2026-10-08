# M0 + M1 Implementation Plan

> **For agentic workers:** execute this plan task by task, in order. Each task is TDD: write the failing test, watch it fail, implement, watch it pass, commit. Check the boxes (`- [ ]` → `- [x]`) as you go, in the same commit as the task. Formulas and numbers come from the specs; this plan references them by section instead of repeating them, so **open the referenced spec section before starting each task.**

**Goal:** a playable browser build with a first-person player on a fixed 4 km map, and the Laika car with every system in the Car Systems spec working and tested.

**Specs:**
- [Game Design & Architecture](../specs/2026-10-08-game-design.md), cited as **G§n**
- [Car Systems Design](../specs/2026-10-08-car-systems-design.md), cited as **C§n**

**Architecture:** three layers, `app → view → sim → shared` (G§2). `sim` is pure TypeScript plus Rapier, with no Three.js and no DOM, and runs headless in Vitest. Fixed 60 Hz tick, input as commands, all state plain JSON.

**Tech stack:** TypeScript 5 (strict), Vite 6, Three.js (`three/webgpu`), `@dimforge/rapier3d-compat`, `miniplex`, `lil-gui`, `idb-keyval`, Vitest, Playwright, ESLint 9 (flat config), Prettier, GitHub Actions. Pin exact versions in `package.json` when scaffolding (use the latest stable release of each at that time).

**Conventions:**
- Units: SI everywhere in `sim` (m, kg, s, N, N·m, rad, °C, L, A, V). km/h appears only in the view and in tests.
- Each system is a function `(world: SimWorld, dt: number) => void` in `src/sim/systems/<name>.ts`.
- Randomness only through `world.rng` (seeded, `mulberry32`), so tests are reproducible.
- One-shot effects for the view (sounds, particles) are pushed to `world.events`, never triggered from sim directly.
- Commit messages: `feat(sim): …`, `feat(view): …`, `test: …`, `chore: …`.
- Run `npm run check` (lint + typecheck + unit tests) before every commit.

**Milestone exits:**
- **M0 done** when Task 11 passes: an empty scene boots in the browser, the loop ticks, CI is green.
- **M1 done** when Task 60 passes: every scenario test in C§15.2 and every walkthrough in C§14 passes.

---

## Phase M0: Foundations

### Task 1: Scaffold the project

**Files:** `package.json`, `tsconfig.json`, `vite.config.ts`, `vitest.config.ts`, `eslint.config.js`, `.prettierrc`, `.gitignore`, `index.html`, `src/app/main.ts`, `tests/unit/sanity.test.ts`

- [ ] Create a Vite vanilla-TS project in the repo root (do not nest it in a subfolder).
- [ ] `tsconfig.json`: `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `target ES2022`, `moduleResolution bundler`, path alias `@sim/*`, `@view/*`, `@app/*`, `@shared/*`.
- [ ] ESLint flat config with `typescript-eslint` recommended-type-checked, plus the layer rule:

```js
{
  files: ['src/sim/**/*.ts'],
  rules: {
    'no-restricted-imports': ['error', { patterns: [
      { group: ['three', 'three/*', '@view/*', '@app/*'], message: 'sim must not depend on rendering or app code (G§2.1)' },
    ] }],
    'no-restricted-globals': ['error', 'window', 'document', 'navigator', 'localStorage', 'indexedDB'],
  },
},
{ files: ['src/view/**/*.ts'], rules: { 'no-restricted-imports': ['error', { patterns: [{ group: ['@app/*'] }] }] } },
```

- [ ] Scripts: `dev`, `build`, `preview`, `lint`, `typecheck` (`tsc --noEmit`), `test` (`vitest run`), `test:watch`, `e2e` (`playwright test`), `check` (`npm run lint && npm run typecheck && npm run test`).
- [ ] `tests/unit/sanity.test.ts`: `expect(1 + 1).toBe(2)`. Run `npm run check` and expect it to pass.
- [ ] Add a lint-fixture test proving the layer rule works: a file `tests/lint/sim-imports-three.fixture.ts` that ESLint is run against in `tests/unit/lint-rules.test.ts` with the ESLint Node API, asserting one `no-restricted-imports` error.
- [ ] Commit: `chore: scaffold Vite + TypeScript project with layer lint rules`.

### Task 2: CI

**Files:** `.github/workflows/ci.yml`

- [ ] A workflow on `pull_request` and `push` to `main`: checkout, setup-node (LTS, npm cache), `npm ci`, `npm run lint`, `npm run typecheck`, `npm run test`, `npm run build`. The e2e job is added in Task 11.
- [ ] Commit: `chore: add CI workflow`.

### Task 3: Shared math and RNG

**Files:** `src/shared/math.ts`, `src/shared/rng.ts`, `src/shared/units.ts`, `tests/unit/shared/*.test.ts`

- [ ] Tests first: `clamp`, `lerp`, `smoothstep(e0, e1, x)` (0 below, 1 above, 0.5 at the midpoint), `Vec3` helpers as plain `{x,y,z}` objects (`add`, `sub`, `scale`, `dot`, `cross`, `length`, `normalize`), quaternion rotate-vector, `mulberry32(seed)` produces the same sequence for the same seed, `kmhToMs` / `msToKmh`.
- [ ] Implement. No classes; plain objects and functions (JSON-serializable state rule, G§2.4).
- [ ] Commit: `feat(shared): math helpers, seeded RNG, unit conversions`.

### Task 4: Sim world, ECS and commands

**Files:** `src/sim/ecs.ts`, `src/sim/world.ts`, `src/sim/commands.ts`, `src/sim/events.ts`, `tests/unit/sim/world.test.ts`

- [ ] Define the component types from G§2.4 (empty bodies are fine for now) and `type Entity = Partial<Components> & { id: string }`. Create `createSimWorld(seed)` returning `{ ecs: miniplex World<Entity>, tick: number, time: number, rng, events: SimEvent[], worldOffset: {x,y,z} }`.
- [ ] `commands.ts`: the `Command` union from G§2.3.
- [ ] `world.ts`: `step(world, dt, commands)` calls the systems in the G§2.2 order. Start with each as a no-op stub in `src/sim/systems/`, and fill them in later tasks.
- [ ] Tests: `step` increments `tick` and `time`; `events` is cleared at the start of each step; the system call order matches G§2.2 (spy on the stubs).
- [ ] Commit: `feat(sim): world container, ECS, command types, step order`.

### Task 5: Fixed-step main loop

**Files:** `src/app/loop.ts`, `tests/unit/app/loop.test.ts`

- [ ] Tests with a fake clock: 16.67 ms frames give exactly 1 tick each; a 100 ms frame gives 5 ticks max (spiral guard, G§2.2); a frame dt above 250 ms is clamped; `alpha` is in [0, 1).
- [ ] Implement `createLoop({ step, render, now })`, which returns `start()` and `stop()`; the frame function is exported for tests.
- [ ] Commit: `feat(app): fixed 60 Hz loop with accumulator and spiral guard`.

### Task 6: Rapier in sim

**Files:** `src/sim/physics.ts`, `src/sim/systems/physicsStep.ts`, `tests/unit/sim/physics.test.ts`

- [ ] `initPhysics()` awaits `RAPIER.init()` (the compat build works in Node). It stores `world.physics = { rapier, world: new RAPIER.World({x:0, y:-9.81, z:0}), eventQueue }`, kept outside the serializable state: a non-enumerable property, or a side map keyed by the sim world.
- [ ] `Body` component helpers: `createDynamicBody(world, entity, desc)` and `removeBody`.
- [ ] `physicsStep` sets `integrationParameters.dt = dt`, steps, copies each body's translation and rotation into `Transform` (saving the previous ones first for interpolation), and drains contact-force events into `world.contactEvents` for the damage system.
- [ ] Test: a 1 kg box dropped from 10 m onto a ground cuboid comes to rest at the expected height within 3 s of ticks.
- [ ] Commit: `feat(sim): Rapier world, body helpers, physics step with transform sync`.

### Task 7: Renderer and view sync

**Files:** `src/view/renderer.ts`, `src/view/sync.ts`, `src/view/registry.ts`, `src/app/main.ts`

- [ ] `createRenderer(canvas)` uses `new WebGPURenderer({ antialias: true })` from `three/webgpu` and `await renderer.init()`. Log the active backend (`renderer.backend.isWebGPUBackend`) and expose it for the settings screen.
- [ ] `registry.ts`: a `Map<entityId, Object3D>` with `factory(entity) → Object3D` functions per entity kind.
- [ ] `sync.ts`: each frame, create objects for new entities, remove objects for gone entities, and set each transform to `lerp(prev, cur, alpha)` (position) and `slerp` (rotation).
- [ ] `main.ts`: boot Rapier, create the sim world with a ground plane and a falling box, create the renderer and a fixed camera, and start the loop. Manual check in `npm run dev`: the box falls and lands.
- [ ] Commit: `feat(view): WebGPU renderer, entity registry, interpolated sync`.

### Task 8: Input to commands

**Files:** `src/app/input/keyboard.ts`, `src/app/input/mouse.ts`, `src/app/input/bindings.ts`, `src/app/input/commands.ts`, `tests/unit/app/input.test.ts`

- [ ] `bindings.ts`: the default map from C§10.8, as data `{ action: KeyCode[] }`.
- [ ] `commands.ts`: `buildCommands(inputState, context: 'foot' | 'driver' | 'passenger') → Command[]`, called once per tick. Mouse deltas accumulate between ticks and are consumed once. Edge-triggered actions (E, L, Z…) fire once per press.
- [ ] Tests: pressing E across two ticks emits one `interact`; mouse delta is split correctly across ticks; in `driver` context WASD produce `drive`, and in `foot` they produce `move`.
- [ ] Pointer lock on canvas click; lost lock opens the pause menu (stub).
- [ ] Commit: `feat(app): input bindings and per-tick command building`.

### Task 9: Debug overlay

**Files:** `src/view/debug/overlay.ts`

- [ ] A `lil-gui` panel toggled with F3, which can register folders bound to live sim values (read-only monitors) and to tunable data objects (editable). Show FPS, tick ms and backend now; car folders are added in later tasks.
- [ ] Expose `window.__debug = { world }` in dev builds only (`import.meta.env.DEV`), for e2e tests.
- [ ] Commit: `feat(view): F3 debug overlay`.

### Task 10: Save skeleton

**Files:** `src/sim/save/snapshot.ts`, `src/sim/save/migrations.ts`, `src/app/save/storage.ts`, `tests/unit/sim/save.test.ts`

- [ ] `snapshot(world) → SaveFile { schemaVersion: 1, tick, time, worldOffset, entities: Entity[] (without runtime handles), consumedSpawns }`. `restore(save) → SimWorld` rebuilds Rapier bodies from `Transform` + `Body` descriptors.
- [ ] `migrations.ts`: `const migrations: Record<number, (s) => s>` and `migrate(save)` applies them in order. Test with a fake v0→v1 migration.
- [ ] `storage.ts` (app layer, because it uses IndexedDB): `save(slot)` copies the current slot to `backup` first, then writes; `load(slot)` falls back to `backup` on failure and reports which was used (G§6).
- [ ] Tests: snapshot → JSON → restore → snapshot is deep-equal; corrupt JSON triggers the backup path (storage tested with `fake-indexeddb`).
- [ ] Commit: `feat(sim): save snapshot, restore, migrations; IndexedDB storage with backup`.

### Task 11: E2E smoke and M0 exit

**Files:** `playwright.config.ts`, `tests/e2e/boot.spec.ts`, `.github/workflows/ci.yml`

- [ ] Playwright config: build + `vite preview` as webServer, Chromium using `executablePath` from env if set, with `--enable-unsafe-webgpu` and a WebGL fallback allowed.
- [ ] Test: page loads, `window.__debug.world.tick` increases over 1 s, no console errors.
- [ ] Add an `e2e` job to CI.
- [ ] Commit: `test: e2e boot smoke test`. **M0 complete.**

---

## Phase M1a: Player and world

### Task 12: Map source data and chunk baking

**Files:** `assets/map/heightmap.png` (generated), `assets/map/map.json`, `scripts/make-heightmap.ts`, `scripts/bake-chunks.ts`, `tests/unit/scripts/bake.test.ts`

- [ ] `make-heightmap.ts`: a deterministic script (seeded value noise, 3 octaves) that writes a 16-bit grayscale PNG, 4096 × 1024 px at 1 px = 1 m with ~60 m relief. It is committed as the authored base map, and the map can later be hand-edited in any image editor.
- [ ] `map.json`: the road spline control points (along the long axis), the home-base and gas-station positions, the two wreck positions with seeds, prop scatter rules (density per type, minimum distance from the road), and radio transmitter positions (C§10.7).
- [ ] `bake-chunks.ts`: for each 256 m chunk, sample heights at 4 m spacing (65 × 65), flatten the terrain under the road (7 m wide + 4 m smooth shoulder), split the road spline into per-chunk segments, place props with a seeded RNG per chunk, and assign items and structures to chunks. Output `public/chunks/{cx}_{cz}.json` (heights base64-encoded).
- [ ] Tests: road samples have height equal to the spline height ±0.05 m; no prop within 6 m of the road centreline; baking twice produces byte-identical output.
- [ ] Commit: `feat(world): heightmap, map layout, chunk baking script`.

### Task 13: WorldSource and chunk streaming

**Files:** `src/sim/worldgen/types.ts`, `src/sim/worldgen/fixedWorldSource.ts`, `src/sim/systems/worldStreaming.ts`, `tests/unit/sim/worldStreaming.test.ts`

- [ ] `interface WorldSource { getChunk(cx, cz): Promise<ChunkData> | ChunkData }` (G§4.2). `FixedWorldSource` takes a loader function (`fetch` in the app, `fs` in tests) injected from outside, so sim stays DOM-free.
- [ ] `worldStreaming`: keep chunks within radius 4 of the player loaded. Loading creates a Rapier heightfield collider and spawns untracked items whose spawn ids aren't in `consumedSpawns`. Unloading removes colliders and untracked items, and freezes vehicles (G§4.2).
- [ ] Async loads: request, then attach when ready on a later tick. Never block a tick.
- [ ] Tests: walking the player 600 m loads and unloads the expected chunk set; a tracked item survives unload/reload with its state; a vehicle in an unloaded chunk is frozen and resumes when reloaded.
- [ ] Commit: `feat(sim): WorldSource interface, fixed source, chunk streaming`.

### Task 14: Floating origin

**Files:** `src/sim/systems/worldStreaming.ts` (extend), `tests/unit/sim/floatingOrigin.test.ts`

- [ ] When the player is more than 2,000 m from the origin, shift every body, `Transform` (current and previous) and chunk collider by `-offset` (rounded to 256 m) in one tick, and add the offset to `worldOffset` (G§4.3). Emit an `originShift` event so the view shifts too.
- [ ] Tests: a moving car's velocity and relative positions are unchanged across a shift; absolute position (`worldOffset + position`) is continuous.
- [ ] Commit: `feat(sim): floating origin`.

### Task 15: Terrain, road and props in the view

**Files:** `src/view/world/terrain.ts`, `src/view/world/road.ts`, `src/view/world/props.ts`, `src/view/materials/palette.ts`, `assets/models/props/*.glb`, `assets/CREDITS.md`

- [ ] `palette.ts`: shared flat/vertex-colour node materials (G§2.5), with a condition-tier variant function used in Task 29.
- [ ] Terrain mesh per chunk from the heights, with vertex colours by slope and height (grass / dirt / rock).
- [ ] Road ribbon mesh from spline segments, with a centre-line and edge-line vertex colour.
- [ ] Props as `InstancedMesh` per type per chunk. Source low-poly trees, rocks and signs from Kenney / Quaternius CC0 packs, and record each file's source and licence in `assets/CREDITS.md`.
- [ ] Manual check: fly a debug camera along the road; chunks appear and disappear without hitches; draw calls stay under 300 (show `renderer.info` in the overlay).
- [ ] Commit: `feat(view): chunked terrain, road and instanced props`.

### Task 16: Player controller

**Files:** `src/sim/systems/playerMovement.ts`, `src/sim/player.ts`, `src/view/camera.ts`, `tests/unit/sim/player.test.ts`

- [ ] Rapier `KinematicCharacterController` with the G§3.1 numbers (capsule, speeds, autostep, slope, snap). Apply gravity and jump velocity manually.
- [ ] Riding moving bodies: if the ground collider's body is dynamic, add its point velocity at the feet (G§3.1).
- [ ] `look` commands set yaw and pitch (pitch clamped ±89°). The camera in the view follows the eye position with interpolation.
- [ ] Tests: walking 1 s at walk speed covers 1.6 m ±5%; the player can't climb a 50° slope; the player climbs a 0.3 m step; the player standing on a box moving at 2 m/s moves with it.
- [ ] Commit: `feat(sim): first-person player controller`.

### Task 17: Interaction raycast, prompts and rule table

**Files:** `src/sim/interaction/raycast.ts`, `src/sim/interaction/rules.ts`, `src/sim/systems/interaction.ts`, `src/view/hud/prompt.ts`, `tests/unit/sim/interaction.test.ts`

- [ ] Each tick, a 2.5 m ray from the eye along the look direction, excluding the player and held item. Store `player.target = { entityId, colliderTag, point } | null`.
- [ ] `Interactable` defines verbs and a prompt builder `(entity, world) → string`. The HUD shows the crosshair and prompt text (from `player.prompt`).
- [ ] Rule table per G§3.2: `{ id, held: (e) => boolean, target: (e, hit) => boolean, verb, durationS, onProgress?, apply }`. The interaction system finds the first matching rule while LMB is held, accumulates progress, and applies it. Expose `player.actionProgress` for the HUD ring.
- [ ] Tests: a target beyond 2.5 m is not hit; with two matching rules the first wins; progress resets when the target changes; `E` toggles a test switch entity.
- [ ] Commit: `feat(sim): interaction raycast, prompts, held×target rule table`.

### Task 18: Grab, hold, rotate, drop, throw

**Files:** `src/sim/interaction/grab.ts`, `tests/unit/sim/grab.test.ts`

- [ ] LMB on a `Grabbable`: attach it with a spring-damper drive toward the hold point (G§3.2). Implement this as a velocity-based drive: each tick set the body's linvel/angvel toward the target, capped by a force limit scaled by mass, so the item still collides.
- [ ] Wheel changes the distance (0.6–2.0 m), R + mouse rotates, RMB drops, and a long LMB press with no rule target throws.
- [ ] Tests: a held 20 kg jerry can reaches the hold point within 0.5 s; it can't be pushed through a wall; it drops with the player's velocity; a throw adds the expected impulse.
- [ ] Commit: `feat(sim): physical item holding, rotate, drop, throw`.

### Task 19: Sky and day/night

**Files:** `src/sim/systems/clock.ts`, `src/view/world/sky.ts`, `tests/unit/sim/clock.test.ts`

- [ ] Sim clock: `timeOfDay` in hours, advancing at 24 real min per day by default, start 08:00, and a debug "skip hours" command (G§4.4).
- [ ] View: sky node material, sun direction from the time, sun/moon directional light with 3 shadow cascades, hemisphere ambient scaled to 0.03 at midnight, fog colour matched to the sky, and stars at night.
- [ ] Test: 24 real minutes of ticks advance exactly one day; skipping hours wraps correctly.
- [ ] Commit: `feat: day/night clock and sky`.

### Task 20: Structures and basic items

**Files:** `src/sim/items/catalog.ts`, `src/sim/items/spawn.ts`, `src/view/world/structures.ts`, `assets/models/*`

- [ ] Item catalog: wrench (`Tool`), jerry can 20 L, oil canister 4 L, water bottle 1.5 L, as `Grabbable` + `LiquidContainer` (containers are filled in Task 23). Spawn the home-base items per G§4.1.
- [ ] Structures: the garage (static colliders, workbench) and the gas station (canopy, two pumps as entities, closed shop), built from CC0 low-poly kits or primitives.
- [ ] Manual check: walk from the garage to the gas station; pick up and carry each item.
- [ ] Commit: `feat: home base, gas station, basic items`.

### Task 21: Tracked items persistence

**Files:** `src/sim/worldgen/tracking.ts`, `tests/unit/sim/tracking.test.ts`

- [ ] Mark an item tracked on first grab, on moving more than 0.5 m, on liquid change, or on damage (G§4.2). Record its spawn id in `consumedSpawns`. Tracked items are saved; untracked ones are not.
- [ ] Tests: each trigger marks tracking; save/restore keeps tracked items and does not duplicate them from spawn data.
- [ ] Commit: `feat(sim): tracked item persistence`.

---

## Phase M1b: Liquids

### Task 22: LiquidMix

**Files:** `src/sim/liquids/mix.ts`, `tests/unit/sim/liquids/mix.test.ts`

- [ ] Tests first, per C§5.1 and C§15.1: `total`, `fraction` (0 for empty), `add`, proportional `take` (taking 1 L from {petrol 3, oil 1} takes 0.75 + 0.25), `take` more than available returns everything, results are never negative, and `addWithOverflow(container, mix) → spilled`.

```ts
it('take is proportional', () => {
  const { taken, rest } = take({ petrol: 3, oil: 1 }, 1);
  expect(taken.petrol).toBeCloseTo(0.75); expect(taken.oil).toBeCloseTo(0.25);
  expect(total(rest)).toBeCloseTo(3);
});
```

- [ ] Implement as pure functions returning new objects. Drop liquids under 1e-6 L.
- [ ] Commit: `feat(sim): LiquidMix maths`.

### Task 23: Caps, pouring, spilling

**Files:** `src/sim/liquids/container.ts`, `src/sim/systems/liquidTransfer.ts`, `src/sim/interaction/rules.ts` (add rows), `src/view/fx/liquid.ts`, `tests/unit/sim/liquids/pour.test.ts`

- [ ] `E` on a container toggles `capOpen` (prompt "Open cap" / "Close cap"; looking at an open opening shows the level, C§5.3).
- [ ] Pour rule: held open container + target open opening within 0.25 m horizontally and below → auto-tilt pose and flow at the container's rate (C§5.3 table), carrying the mix. Spill rule with no target. Detached open containers lying past 90° tilt leak.
- [ ] Emit `pour` / `spill` events with rate; the view draws a particle stream and plays glug/splash.
- [ ] Tests: rates per container type; a closed cap blocks; mix carried; a full target spills the excess; a tipped open can leaks; a closed can doesn't.
- [ ] Commit: `feat(sim): caps, pouring and spilling`.

### Task 24: Drain plugs

**Files:** `src/sim/liquids/drain.ts`, `tests/unit/sim/liquids/drain.test.ts`

- [ ] `drain` and `drainOpen` on containers (C§5.5); E toggles; 0.3 L/s into a held open container whose opening is within 0.25 m below, otherwise spill.
- [ ] Tests: drains into a held can; spills with no can; stops when empty.
- [ ] Commit: `feat(sim): drain plugs`.

### Task 25: Gas station pumps

**Files:** `src/sim/items/pump.ts`, `src/view/world/pumpHose.ts`, `tests/unit/sim/pump.test.ts`

- [ ] The pump entity has a nozzle item linked by a Rapier rope joint (3 m). The nozzle is an infinite source of `petrol` or `diesel` at 1.0 L/s via the pour rule. The pump display counts litres. A dropped nozzle more than 3 m away returns to the pump (G§4.5).
- [ ] View: hose as a tube along a catenary between the pump and the nozzle.
- [ ] Tests: pumping 10 s into an open jerry can gives 10 L of the right fuel; it stops when full.
- [ ] Commit: `feat: gas station pumps with hose nozzle`.

---

## Phase M1c: Car parts

### Task 26: Vehicle and part data

**Files:** `src/sim/vehicle/data/laika.ts`, `src/sim/vehicle/data/parts.ts`, `src/sim/vehicle/data/fluids.ts`, `src/sim/vehicle/data/electrics.ts`, `src/sim/vehicle/data/thermal.ts`, `src/sim/vehicle/data/damage.ts`, `tests/unit/sim/vehicle/data.test.ts`

- [ ] Encode C§2, C§3.2 (every slot with its tags, bolts, anchor, `requires`), C§3.4 (catalog), and every tunable referenced in C§5–C§11 into these files (C§17).
- [ ] Tests: the sum of part masses plus chassis plus full fluids is 1,060 ±20 kg; every slot accepts at least one catalog part; the torque curve is monotonic in rpm order; no duplicate slot paths.
- [ ] Commit: `feat(sim): Laika and part catalog data`.

### Task 27: Vehicle entity, slots, mass and centre of mass

**Files:** `src/sim/vehicle/vehicle.ts`, `src/sim/vehicle/slots.ts`, `src/sim/vehicle/mass.ts`, `src/view/vehicle/vehicleView.ts`, `tests/unit/sim/vehicle/slots.test.ts`

- [ ] `spawnVehicle(world, modelId, transform, partsSpec)` creates the chassis body and part entities, and installs parts (C§3.1: an installed part's collider joins the chassis as a compound child, and its body is removed).
- [ ] `recomputeMass(vehicle)` sets chassis additional mass and centre of mass from the installed parts and fluid litres (fuel 0.74, diesel 0.84, oil 0.88, water 1.0 kg/L), called whenever a part or fluid changes by more than 0.5 kg.
- [ ] View: the chassis mesh plus a mesh per installed part at its anchor (placeholder boxes are fine until Task 57).
- [ ] Tests: spawning the default Laika installs all slots; removing the engine moves the centre of mass rearward; draining 40 L of fuel lowers the mass by about 29.6 kg.
- [ ] Commit: `feat(sim): vehicle entity, slot installation, mass and COM`.

### Task 28: Bolting, attach, detach, access

**Files:** `src/sim/vehicle/bolting.ts`, `src/sim/interaction/rules.ts` (add rows), `src/view/hud/ghost.ts`, `tests/unit/sim/vehicle/bolting.test.ts`

- [ ] Rules (C§3.3): wrench + bolted part → unbolt bolt by bolt, 0.5 s each, keeping progress; wrench + loose part → bolt; empty hands + 0-bolt part → hold 0.5 s to remove; empty hands + fully unbolted part → grab, which detaches it as a dynamic item with the chassis velocity; held part near an empty compatible slot (0.4 m, rough alignment) → ghost preview → LMB snaps it in unbolted.
- [ ] Access: slot `requires` (the engine requires `coolantTank` empty), and the raycast is blocked by closed hood/door colliders naturally.
- [ ] Unbolted installed parts detach on a vertical acceleration over 2 g (C§3.1); the impact case is in Task 44.
- [ ] Tests: 4 bolts take 2.0 s; releasing halfway keeps 2 bolts; the wrong tag is refused; the engine can't be removed with the coolant tank present; the detach/attach round-trip restores mass.
- [ ] Commit: `feat(sim): bolting, part attach/detach, access rules`.

### Task 29: Condition tiers and visuals

**Files:** `src/sim/vehicle/condition.ts`, `src/view/materials/condition.ts`, `tests/unit/sim/vehicle/condition.test.ts`

- [ ] `tierOf(c)` per C§4.1, and prompts show the tier.
- [ ] View: per-tier material variants (gloss, satin, desaturated with noise, rusty) plus a broken mesh variant (a cheap vertex jitter is acceptable for M1).
- [ ] Tests: tier boundaries.
- [ ] Commit: `feat: condition tiers and visual variants`.

### Task 30: Wrecks

**Files:** `src/sim/vehicle/wrecks.ts`, `tests/unit/sim/vehicle/wrecks.test.ts`

- [ ] Spawn the two wrecks from `map.json` seeds per C§3.5 (each slot 60% present, conditions 0.05–0.5, 0–3 L old oil; wreck #2 always has `engine.laika15d`).
- [ ] Tests: the same seed gives the same wreck; wreck #2 has the diesel engine.
- [ ] Commit: `feat(sim): scavengeable wrecks`.

---

## Phase M1d: Car physics

### Task 31: Scenario test harness and test track

**Files:** `tests/scenario/harness.ts`, `tests/scenario/track.ts`

- [ ] `createTestTrack(kind: 'flat' | 'slope15' | 'slalom')` builds a minimal sim world (flat asphalt cuboid 3 km long, a 15° ramp, or a cone course) with a Laika. `drive(world, script: (t, car) => DriveCommand, seconds)` runs ticks headlessly. Helpers: `speedKmh(car)`, `distance()`, `fuelUsedL()`.
- [ ] Add `vitest.scenario.config.ts` with a longer timeout; `npm run test` runs unit + scenario.
- [ ] Commit: `test: headless vehicle scenario harness`.

### Task 32: Raycast suspension

**Files:** `src/sim/vehicle/suspension.ts`, `src/sim/systems/vehicleForces.ts`, `tests/unit/sim/vehicle/suspension.test.ts`

- [ ] Per C§9.1 and C§9.2: a ray per installed wheel (hub radius 0.12 m with no wheel), spring, damper (bump/rebound), anti-roll per axle, bump stop. Store compression, compression velocity and contact info in `WheelState`.
- [ ] Debug: draw rays and contacts in wireframe mode.
- [ ] Tests: the car settles at the expected static ride height (± 1 cm) with no oscillation after 2 s; removing a wheel drops that corner by about 0.17 m.
- [ ] Commit: `feat(sim): raycast suspension`.

### Task 33: Tyre model, steering, resistances

**Files:** `src/sim/vehicle/tyre.ts`, `src/sim/vehicle/steering.ts`, `src/sim/vehicle/aero.ts`, `tests/unit/sim/vehicle/tyre.test.ts`

- [ ] `tyreForces(Fz, s, α, μ) → {Fx, Fy}` per C§9.3, including friction circle and low-speed blending. Surface μ from collider user data. Wheel angular velocity integration.
- [ ] Steering per C§9.4 (speed-sensitive max angle, rate, self-centring, Ackermann). Drag and rolling resistance per C§9.5.
- [ ] Unit tests: the peak of `Fx` is at s = 0.10; the slide tail is 80%; the friction circle is respected; Ackermann inner angle > outer angle.
- [ ] Commit: `feat(sim): tyre model, steering, drag and rolling resistance`.

### Task 34: Brakes and handbrake

**Files:** `src/sim/vehicle/brakes.ts`, `tests/scenario/brakes.test.ts`

- [ ] Brake torques per C§8.4, with the standstill clamp.
- [ ] Scenario tests: 100→0 km/h in 40–50 m (start the car at 100 km/h by setting its velocity; the drivetrain is not needed); handbrake on the 15° slope moves < 5 cm in 30 s.
- [ ] Commit: `feat(sim): brakes and handbrake`.

---

## Phase M1e: Powertrain and electrics

### Task 35: Battery, loads, alternator, ignition

**Files:** `src/sim/vehicle/electrics.ts`, `src/sim/systems/electrical.ts`, `tests/unit/sim/vehicle/electrics.test.ts`

- [ ] Per C§6: battery V0, sag, capacity by condition, the load table, ignition positions (what each powers), alternator smoothstep and acceptance, `Vsys`, battery warning logic, deep-discharge condition loss.
- [ ] Tests (C§15.1 Battery and Ignition rows): headlights high for 8 h from a full 45 Ah battery reach 0; the Off position powers lights but not the radio; the alternator at 3,000 rpm covers 20 A of loads and charges; a missing battery with the engine running keeps the ignition alive.
- [ ] Commit: `feat(sim): battery, loads, alternator, ignition`.

### Task 36: Engine state machine and starting

**Files:** `src/sim/vehicle/engine.ts`, `src/sim/systems/engine.ts`, `tests/unit/sim/vehicle/engineStart.test.ts`

- [ ] States and transitions per C§7.1. Cranking speed from battery voltage under the 120 A load; preconditions with reason codes; a seeded roll every 0.5 s (C§7.2). Emit `starterCrank` / `starterClick` / `engineCatch` events.
- [ ] Tests: each reason code (`no_battery`, `weak_crank`, `no_fuel`, `bad_fuel`, `seized`, `no_engine`, `no_tank`); success rate over 10,000 seeded rolls is 0.3 + 0.7·c within ±1.5%; charge 0.5 can start and charge 0.3 can't.
- [ ] Commit: `feat(sim): engine state machine and starting`.

### Task 37: Torque, rev limiter, fuel

**Files:** `src/sim/vehicle/engine.ts` (extend), `src/sim/vehicle/fuel.ts`, `tests/unit/sim/vehicle/fuel.test.ts`

- [ ] Torque per C§7.4 (curve interpolation, condition, friction, idle controller holding 850 rpm, rev limiter, inertia). Fuel quality, misfires, stall, contamination damage and consumption per C§5.6. Fuel tank leak by condition.
- [ ] Tests: q thresholds; misfire rate ±10% over a long seeded run; consumption at a fixed 13.8 kW equals the formula; an empty tank stalls after 1 s; the fuel gauge reads total volume.
- [ ] Commit: `feat(sim): engine torque, rev limiter, fuel quality and consumption`.

### Task 38: Oil

**Files:** `src/sim/vehicle/oil.ts`, `tests/unit/sim/vehicle/oil.test.ts`

- [ ] λ, burn per km, friction heat, wear multiplier, 5 s seizure, oil warning (C§5.7).
- [ ] Tests: burn at c = 1 is 0.02 L/100 km and at c = 0 is 0.30; seizure exactly after 5 s at λ < 0.05; the light turns on at λ < 0.25 with the engine off and key On.
- [ ] Commit: `feat(sim): engine oil, lubrication and seizure`.

### Task 39: Heat and coolant

**Files:** `src/sim/vehicle/thermal.ts`, `src/sim/vehicle/coolant.ts`, `tests/unit/sim/vehicle/thermal.test.ts`

- [ ] The heat model per C§7.5 (Q_in, thermostat, airflow, fan, G, dT/dt), coolant κ, leak, boil-off, overheat damage, seizure chance above 130 °C, coolant warning (C§5.8).
- [ ] Tests driving `thermal.step` directly with a synthetic power and speed profile: steady state at 13.8 kW and 25 m/s is in 85–95 °C; with no coolant it passes 110 °C within 150 s; the fan turns on above 95 °C; boil-off above 110 °C; a rusty tank's leak rate matches the formula.
- [ ] Commit: `feat(sim): engine heat, thermostat, coolant and overheating`.

### Task 40: Auto-clutch and gearbox

**Files:** `src/sim/vehicle/clutch.ts`, `src/sim/vehicle/gearbox.ts`, `src/sim/systems/drivetrain.ts`, `tests/unit/sim/vehicle/gearbox.test.ts`

- [ ] Clutch engagement, slip and lock-up per C§8.1. Gearbox ratios, shift timer, automatic logic with hysteresis and kickdown, manual mode with the over-rev refusal, and R/N/D handling with the S-hold/W-hold rules (C§8.2).
- [ ] Tests (C§15.1 Auto-clutch and Gearbox rows).
- [ ] Commit: `feat(sim): auto-clutch and gearbox`.

### Task 41: Differential and full drivetrain integration

**Files:** `src/sim/systems/drivetrain.ts` (extend), `src/sim/vehicle/differential.ts`, `tests/scenario/performance.test.ts`

- [ ] The open differential (C§8.3) coupling the gearbox output to the rear wheels' angular velocities, with drivetrain efficiency 0.90. Solve the engine–clutch–wheel coupling with an implicit lock when the clutch is not slipping (treat engine plus reflected wheel inertia as one body) to stay stable at 60 Hz.
- [ ] Scenario tests (C§15.2): 0–100 in 16–22 s; top speed 130–150 km/h; 6–9 L/100 km at 90 km/h; 85–95 °C after 6 min at 90 km/h, holding 10 min; slalom stability. **Tune `data/*.ts` until these pass, and update the spec if any tunable changes.**
- [ ] Commit: `feat(sim): differential and drivetrain integration; performance scenarios`.

### Task 42: Roll start

**Files:** `src/sim/vehicle/engine.ts` (extend), `tests/scenario/rollStart.test.ts`

- [ ] Per C§7.3.
- [ ] Scenario test (C§14-E): full charge, headlights on for 8 simulated hours (fast-forward electrics only), can't crank; push to 15 km/h on the ramp in D with the key On; the engine catches; the battery then charges.
- [ ] Commit: `feat(sim): roll start`.

### Task 43: Debug overlay car folders

**Files:** `src/view/debug/carPanel.ts`

- [ ] Monitors for every value in G§2.6; editable folders bound to each `data/*.ts` tunable object; a "skip hours" button; a "set fluids" helper.
- [ ] Commit: `feat(view): car debug panels`.

---

## Phase M1f: Damage and wear

### Task 44: Collision damage, glass, detachment

**Files:** `src/sim/systems/collisionDamage.ts`, `src/sim/vehicle/damage.ts`, `tests/unit/sim/vehicle/damage.test.ts`

- [ ] Severity from contact-force events, falloff, bumper shielding, fragilities, glass shatter threshold by condition, detachment at 0 or when unbolted with severity > 1, door latch failure under 0.15 (C§11, C§4.3). Emit `impact` / `glassBreak` / `partDetached` events.
- [ ] Tests: severity < 2 does nothing; a frontal severity of 8 damages the bumper > coolant tank > engine (engine only above 8); a rear hit above 6 damages the fuel tank; a shattered window is removed; a part at 0 detaches with the chassis velocity.
- [ ] Commit: `feat(sim): collision damage, glass and detachment`.

### Task 45: Tyre pressure, punctures, flats, wheel change

**Files:** `src/sim/vehicle/tyre.ts` (extend), `tests/unit/sim/vehicle/puncture.test.ts`, `tests/scenario/wheelChange.test.ts`

- [ ] Pressure factor, the three puncture triggers, slow leak, flat effects, hub scraping, and the "pulls to one side" effect emerging from the flat's radius and μ (C§9.6).
- [ ] Tests: the bump-stop trigger; a random puncture rate over 10,000 km seeded; the flat radius; a scenario where the car with a flat front-left drifts left under straight steering, then a wheel swap via the bolting rules restores straight tracking.
- [ ] Commit: `feat(sim): tyre pressure, punctures, flats, wheel changes`.

### Task 46: Wear

**Files:** `src/sim/systems/wear.ts`, `tests/unit/sim/wear.test.ts`

- [ ] Distance- and slip-based wear per C§4.2.
- [ ] Tests: 1,000 km at 3,000 rpm costs the engine 0.2 condition; at 5,000 rpm it costs 1.5×.
- [ ] Commit: `feat(sim): wear over distance and slip`.

### Task 47: Flipping

**Files:** `src/sim/vehicle/flip.ts`, `tests/unit/sim/vehicle/flip.test.ts`

- [ ] The prompt and hold-E torque per C§11.5.
- [ ] Test: a car spawned upside down is upright on its wheels within 3 s of holding E.
- [ ] Commit: `feat(sim): flip overturned car`.

---

## Phase M1g: Cabin

### Task 48: Seats, entering and exiting

**Files:** `src/sim/vehicle/seats.ts`, `src/sim/systems/applyCommands.ts`, `tests/unit/sim/vehicle/seats.test.ts`

- [ ] Per G§3.3: E on an open door or seat sits (a seat part is required); input context switches; free-look limits; the exit side search; passenger seat.
- [ ] Tests: you can't sit without a driver's seat part; exit picks the left side, then the right if left is blocked by a wall; driving commands are ignored in the passenger seat.
- [ ] Commit: `feat(sim): seats, enter and exit`.

### Task 49: Driving input mapping

**Files:** `src/sim/systems/applyCommands.ts` (extend), `tests/unit/sim/vehicle/driveInput.test.ts`

- [ ] Map `drive` commands to throttle, brake and steer per C§8.2 (direction-aware in automatic), handbrake toggle vs hold, K for ignition, `carToggle` commands, missing steering wheel → steer ignored.
- [ ] Tests: S held 0.4 s at standstill selects R and then S throttles backwards; W brakes in R.
- [ ] Commit: `feat(sim): driving input semantics`.

### Task 50: Dashboard

**Files:** `src/sim/vehicle/dash.ts`, `src/view/vehicle/dashboard.ts`, `tests/unit/sim/vehicle/dash.test.ts`

- [ ] Sim: odometer and trip from the front-left wheel, the damped fuel needle, all warnings (C§10.2). View: needle meshes, odometer drums, warning lamp emissives, gear indicator.
- [ ] Tests: the odometer counts 1.000 km over 1 km driven; gauges are dead below 9 V; the warning truth table.
- [ ] Commit: `feat: dashboard gauges and warning lights`.

### Task 51: Lights

**Files:** `src/sim/vehicle/lights.ts`, `src/view/vehicle/lights.ts`, `tests/unit/sim/vehicle/lights.test.ts`

- [ ] Sim: per-lamp on/off and brightness (part, condition, circuit, voltage), blinker timing (1.5 Hz, 3 Hz with a missing bulb), auto-cancel after a turn (C§10.3). View: 2 headlight spot lights, emissives plus bloom for the rest, dome point light.
- [ ] Tests: brightness at 12.6 V vs 10 V; a missing left indicator doubles the blink rate; brake lights at brake > 0.05.
- [ ] Commit: `feat: vehicle lights`.

### Task 52: Doors, hood, trunk, windows, wipers

**Files:** `src/sim/vehicle/hinges.ts`, `src/sim/vehicle/windows.ts`, `src/view/vehicle/hinges.ts`, `tests/unit/sim/vehicle/hinges.test.ts`

- [ ] Hinge angle state with a spring/inertia response to chassis acceleration; E open/close; latch above 0.3 rad/s; failed latch below c = 0.15; window crank; wipers (C§10.4). Hinged collider children update with the angle.
- [ ] Tests: an unlatched door swings open under 0.5 g of braking; a closed door with a good latch stays shut at 1 g; the window moves 0.25 m in 2 s.
- [ ] Commit: `feat: doors, hood, trunk, windows, wipers`.

### Task 53: Glovebox and trunk storage

**Files:** `src/sim/vehicle/storage.ts`, `tests/scenario/storage.test.ts`

- [ ] Stow on lid close, unstow on open, inheriting chassis velocity; stowed mass is added to the chassis (C§10.5).
- [ ] Scenario tests: a stowed jerry can stays put through the slalom; with the trunk open, an unstowed jerry can falls out under hard acceleration.
- [ ] Commit: `feat(sim): glovebox and trunk storage`.

### Task 54: Mirrors

**Files:** `src/view/vehicle/mirrors.ts`

- [ ] 3 render targets at 256×128, round-robin at 30 Hz each, 150 m culling, per-mirror settings toggles, and the door mirror removed with its door (C§10.6). Manual perf check against the G§8 budget.
- [ ] Commit: `feat(view): rear-view and side mirrors`.

### Task 55: Radio

**Files:** `src/sim/vehicle/radio.ts`, `src/view/audio/radio.ts`, `assets/audio/radio/*`, `tests/unit/sim/vehicle/radio.test.ts`

- [ ] Sim: power, volume, frequency, signal formula and global playback position (C§10.7). View: Web Audio graph mixing the station track and a static noise buffer. Source 3 stations' tracks from CC0 music and credit them.
- [ ] Tests: the signal formula at the station frequency and 0.15 MHz off; it is off below 10 V; the playback position is consistent while the radio is off.
- [ ] Commit: `feat: car radio`.

---

## Phase M1h: Audio, assets and integration

### Task 56: Vehicle audio

**Files:** `src/view/audio/engine.ts`, `src/view/audio/vehicle.ts`, `src/view/audio/mixer.ts`, `assets/audio/*`

- [ ] Every row of C§12: the four-loop engine crossfade with pitch, load layer, misfire dip, exhaust rasp; starter; tyres; suspension; impacts; body; cabin; fluids; wind; the cabin low-pass filter. Driven by sim state plus `events`.
- [ ] Manual check: rev through the range in neutral; the crossfades have no audible gaps.
- [ ] Commit: `feat(view): vehicle audio`.

### Task 57: Final low-poly assets

**Files:** `assets/models/laika/*.glb`, `assets/CREDITS.md`, `src/view/vehicle/vehicleView.ts`

- [ ] Replace the placeholder boxes with a low-poly Laika: a CC0 base car model (Kenney Car Kit / Quaternius) split into the C§3.2 parts, or modelled in Blender. Every part is a separate node named after its slot so `vehicleView` can map them. Include broken variants where the mesh differs.
- [ ] Commit: `feat: low-poly Laika and part models`.

### Task 58: Settings, pause, keybinding UI

**Files:** `src/app/ui/pause.ts`, `src/app/ui/settings.ts`

- [ ] A DOM overlay pause menu (Esc / pointer lock lost) with Resume, Save, Load, and Settings: sensitivity, FOV, gearbox mode auto/manual, day length, mirrors on/off, master/radio volume, keybinding rebinding, and the backend shown. Settings are saved to `localStorage` in the app layer.
- [ ] Commit: `feat(app): pause menu and settings`.

### Task 59: Full save integration and autosave

**Files:** `src/sim/save/snapshot.ts` (extend), `src/app/save/autosave.ts`, `tests/unit/sim/saveCar.test.ts`

- [ ] Ensure the car and part state (C§16), tracked items, clock, and consumed spawns all round-trip. Autosave every 2 min and on `visibilitychange`/`pagehide` (G§6).
- [ ] Test: a fully populated world (a car mid-drive with mixed fluids, a detached door, a stowed item) snapshot → restore → snapshot is deep-equal, and one more tick after restore matches one tick without the save/load to 1e-6.
- [ ] Commit: `feat: full save integration and autosave`.

### Task 60: Walkthrough scenarios, playtest and M1 exit

**Files:** `tests/scenario/walkthroughs.test.ts`, `docs/playtest/m1-checklist.md`

- [ ] Automate C§14 A, B, C and E as scenario tests (D, F, G and H are covered by the unit/scenario tests from Tasks 44, 45, 28 and 30 plus the manual checklist).
- [ ] Write `docs/playtest/m1-checklist.md` from C§15.3 and run it in a real browser on both the WebGPU and the WebGL2 backends; record the results in the file.
- [ ] Performance pass against G§8 (fps, draw calls, download size); fix regressions.
- [ ] Update the Playwright smoke test to walk to the car, enter, start the engine and assert rpm > 0 (G§9).
- [ ] Commit: `test: M1 walkthrough scenarios and playtest checklist`. **M1 complete.**

---

## Dependency summary

```
M0: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11
M1a: 12 → 13 → 14 → 15;  16 → 17 → 18;  19;  20 → 21          (needs M0)
M1b: 22 → 23 → 24 → 25                                          (needs 17, 18, 20)
M1c: 26 → 27 → 28 → 29 → 30                                     (needs 17, 18, 22)
M1d: 31 → 32 → 33 → 34                                          (needs 27)
M1e: 35 → 36 → 37 → 38 → 39 → 40 → 41 → 42 → 43                 (needs 34, 23)
M1f: 44 → 45 → 46 → 47                                          (needs 41)
M1g: 48 → 49 → 50 → 51 → 52 → 53 → 54 → 55                      (needs 41)
M1h: 56 → 57 → 58 → 59 → 60                                     (needs everything)
```

M1b and M1c can run in parallel once Task 22 lands; M1f and M1g can run in parallel after Task 41.
