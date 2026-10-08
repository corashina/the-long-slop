# My Summer Slop — Game Design & Architecture Spec

- **Date:** 2026-10-08
- **Status:** Approved design, awaiting written-spec review
- **Companion spec:** [Car Systems Design](./2026-10-08-car-systems-design.md) (the full vehicle specification)
- **Scope of this document:** the whole game's structure, the player, the world, persistence, testing, and the long-term roadmap. Milestone 1 (M1) is specified in implementable detail; later milestones are specified only as far as they constrain M1's architecture.

---

## 1. Vision

A browser game in the space between **The Long Drive** (TLD) and **My Summer Car** (MSC), leaning towards TLD: a first-person road-trip survival game where your car is the thing that keeps you alive, and every part of it is a physical object you can inspect, remove, refill, swap and break.

| Borrowed from The Long Drive (primary) | Borrowed from My Summer Car (reference) |
|---|---|
| Accessible controls: automatic gearbox, no clutch pedal | Depth of the part tree and fluid simulation |
| Long road, scavenging, survival loop | Diegetic interaction: physical switches, caps, bolts |
| Low-poly flat-colour art | Consequences for neglect (oil starvation, overheating) |
| Mixed-liquid containers (fuel/oil/water) | Dashboard gauges and warning lights as the only feedback |

### 1.1 Design pillars

1. **The car is a machine, not a health bar.** Every behaviour emerges from interacting parts and fluids (see the Car Systems spec, §11 interaction matrix).
2. **Everything is physical.** Parts, containers and tools are objects in the world; there are no menu-driven repairs.
3. **Accessible first.** Defaults are forgiving (automatic gearbox, auto-clutch, visible prompts). Realism lives in consequences, not in control complexity.
4. **Built to grow.** M1 is a fixed map, single player; architecture keeps the doors open for procedural worlds (M4) and co-op (M7).

### 1.2 Decisions log

| Topic | Decision |
|---|---|
| Platform | Web browser (desktop, keyboard + mouse). |
| Language | TypeScript (strict). |
| Rendering | Three.js, `WebGPURenderer` with automatic WebGL2 fallback. |
| Physics | Rapier (`@dimforge/rapier3d-compat`, Rust compiled to WASM). |
| Build / test | Vite, Vitest, Playwright (one smoke test), ESLint + Prettier, GitHub Actions. |
| ECS | `miniplex` (object-based ECS; fits deep nested car data better than typed-array ECS). |
| Debug UI | `lil-gui` overlay. |
| World | Fixed hand-built map now; procedural world with fixed home base later (M4). |
| Multiplayer | Single player now; co-op later (M7). Simulation is separable and serializable from day one. |
| Driving realism | TLD-style: automatic by default, optional manual shifting by key, no clutch pedal, no choke. |
| Art | Low-poly, flat / vertex colours. |
| Assets | **No assets ripped from TLD or MSC** (copyrighted; a web game redistributes every file). Use CC0 sources (Kenney, Quaternius, Poly Pizza CC0-filtered, Freesound CC0), procedural primitives, and own Blender models. Screenshots of the reference games may be kept privately as visual reference only. |
| Rejected | Rust/Bevy→WASM (slow iteration, immature web tooling, large bundles), raw WebGPU (would require writing a renderer), Unity-style behaviour scripts (tangles sim with rendering), Rapier's built-in vehicle controller (no drivetrain to hang fuel/oil/heat on). |

---

## 2. Architecture

### 2.1 Layers

```
src/
  app/    boot, main loop, input → commands, settings, menus, save slots
  view/   Three.js scene sync, materials, audio, HUD/prompt, particles, mirrors
  sim/    entities, components, systems, Rapier world, game rules (no Three.js, no DOM)
  shared/ math helpers, constants, types used by more than one layer
```

Dependency rule: `app → view → sim → shared`. **`sim` never imports from `view`, `app`, Three.js or any browser API.** This is enforced by an ESLint `no-restricted-imports` rule on `src/sim/**`. Consequences:

- The whole simulation runs in Node, so every rule can be unit-tested headlessly.
- A future co-op host can run `sim` authoritatively; clients run `view`.
- Save/load is a snapshot of `sim` state only.

Rapier lives in `sim` (it is a physics library, not a renderer). Its math types are converted to/from `shared` vector types at the boundary.

### 2.2 Main loop

```
every animation frame (variable dt):
  accumulator += min(frameDt, 0.25)
  ticks = 0
  while accumulator >= TICK (1/60 s) and ticks < 5:
      commands = input.drainCommandsForTick()
      sim.step(TICK, commands)
      accumulator -= TICK; ticks++
  alpha = accumulator / TICK
  view.render(sim.state, alpha)     // interpolates transforms between prev and current tick
```

- Fixed tick = **60 Hz**. Max 5 ticks per frame (spiral-of-death guard); excess time is dropped.
- `sim.step` order (each is a system in `sim/systems/`):
  1. `applyCommands` (player + car controls)
  2. `playerMovement`
  3. `interaction` (raycast, prompts, held item, `HeldItem × Target` rules)
  4. `liquidTransfer` (pouring, draining, pumping)
  5. `electrical` (battery, alternator, loads)
  6. `engine` (start logic, torque, fuel/oil consumption, heat)
  7. `drivetrain` (gearbox, auto-clutch, differential)
  8. `vehicleForces` (suspension, tyres, brakes, steering, aero)
  9. `physicsStep` (Rapier `world.step`)
  10. `collisionDamage` (consume contact events → part condition, detachments, punctures)
  11. `wear` (distance/time-based condition loss)
  12. `worldStreaming` (chunk load/unload, floating origin)
  13. `clock` (time of day)
  14. `events` (emit one-shot events for view: sounds, particles)

### 2.3 Commands (input seam)

Input never touches sim state. `app/input` turns keyboard/mouse state into a per-tick list of typed commands:

```ts
type Command =
  | { t: 'move'; x: number; z: number; sprint: boolean; crouch: boolean; jump: boolean }
  | { t: 'look'; yaw: number; pitch: number }
  | { t: 'interact' }                    // E
  | { t: 'primary'; phase: 'down' | 'hold' | 'up' }   // LMB
  | { t: 'secondary' }                   // RMB (drop)
  | { t: 'holdDistance'; delta: number } // wheel
  | { t: 'rotateHeld'; dx: number; dy: number }
  | { t: 'drive'; throttle: number; brake: number; steer: number; handbrake: boolean }
  | { t: 'shift'; dir: 1 | -1 }
  | { t: 'carToggle'; what: 'headlights' | 'indicatorL' | 'indicatorR' | 'hazards' | 'horn' | 'wipers' | 'dome' }
  | { t: 'exitSeat' };
```

Co-op later = send commands from clients to the host. Nothing else in `sim` changes.

### 2.4 State and entities

- All mutable state is **plain, JSON-serializable data** held in miniplex entities. No class instances with hidden state; no Three.js objects in sim components.
- Rapier bodies are referenced by handle; their transforms are copied into a `Transform` component after each physics step, and restored from it on load.
- Entity ids are stable strings (`car:laika#1`, `part:door-fl#17`, `item:jerrycan#4`) so save files and future network messages can refer to them.

Core components:

| Component | Data |
|---|---|
| `Transform` | position, rotation (quat), previous position/rotation (for interpolation) |
| `Body` | Rapier rigid-body handle, collider handles, mass |
| `Interactable` | prompt key, verbs available, interaction radius override |
| `Grabbable` | mass, hold offset, throwable |
| `LiquidContainer` | `LiquidMix`, capacity, cap open/closed, opening position |
| `Part` | type id, condition, bolted, slot path, type-specific state |
| `Vehicle` | root of a part tree; see Car Systems spec |
| `Player` | camera yaw/pitch, held entity, seat entity, `Needs` (reserved, unused in M1) |
| `Tool` | tool kind (`wrench`, later `screwdriver`, `jack`…) |
| `WorldItem` | origin chunk + spawn id (for persistence of moved items) |
| `Light`, `AudioEmitter` | data the view mirrors into Three.js / Web Audio |

### 2.5 View layer

- `view/sync.ts` iterates sim entities each frame and creates/updates/destroys Three.js objects via a registry keyed by entity id. Interpolation uses `alpha` from the loop.
- Materials: a small palette of shared `MeshStandardNodeMaterial`s (flat colours + vertex colours). Condition tiers swap a material variant (pristine shine, worn dull, rusty brown with noise).
- Props are `InstancedMesh` per prop type per chunk.
- The view only reads sim state and the per-tick `events` queue. It never mutates sim state.

### 2.6 Debug overlay

`lil-gui` panel (toggle F3) showing live car internals: RPM, gear, clutch slip, torque, engine temp, oil/coolant/fuel level + mix, battery charge/current, each wheel's compression/slip ratio/slip angle/load, part conditions. Also tunables (spring rates, grip, torque scale) editable at runtime. A wireframe toggle draws suspension rays and contact points.

---

## 3. Player

### 3.1 Movement

- Rapier `KinematicCharacterController`, capsule (radius 0.3 m, height 1.8 m, eye height 1.65 m).
- Walk 1.6 m/s, sprint 5 m/s, crouch 0.8 m/s (capsule height 1.1 m), jump 1.0 m apex.
- Autostep 0.35 m, max slope 45°, snap-to-ground 0.3 m.
- Rides on moving bodies: when standing on a dynamic body, the body's point velocity under the feet is added to the player's motion.
- Mouse look via Pointer Lock; pitch clamped ±89°. Sensitivity and FOV (default 75°) in settings.

### 3.2 Interaction

- Each tick: a ray from the camera, length **2.5 m**, against colliders with `Interactable` or `Grabbable`.
- The view shows a crosshair plus a prompt for the hit target, e.g. `Open door [E]`, `Unscrew cap [E]`, `Oil 3.1 / 4.0 L`, `Unbolt (hold LMB)`.
- **E** — activate: doors, hood, trunk, caps, switches, knobs, seats, ignition key.
- **LMB press** — pick up a `Grabbable`. Held items follow a target point in front of the camera through a spring-damper joint (stiffness scaled by mass), so they still collide.
- **Wheel** — move held item nearer/farther (0.6–2.0 m). **R + mouse** — rotate held item. **RMB** — drop. **LMB long press (>0.4 s) with nothing targeted** — throw (impulse 6 N·s, scaled down for heavy items).
- **LMB hold while holding an item and aiming at a target** — context action from a single rule table:

| Held item | Target | Action |
|---|---|---|
| Open liquid container (tilted) | Open liquid opening | Pour (see Car spec §5.3) |
| Open liquid container (tilted) | Nothing | Spill onto the ground |
| Wrench | Bolted part | Unbolt (0.5 s per bolt) |
| Wrench | Part resting in compatible slot | Bolt (0.5 s per bolt) |
| Part | Empty compatible slot | Snap into slot (unbolted) |
| Fuel nozzle | Open liquid opening | Pump from station |

- Rules are data (`sim/interaction/rules.ts`): `{ held: Predicate, target: Predicate, verb, durationS, apply(state) }`. New tools add rows, not code paths.

### 3.3 Seats

- E on a door (when open) or a seat sits you down; the player capsule is disabled and parented to the seat anchor. Camera free-look ±120° yaw, ±80° pitch.
- In the driver's seat, `drive` / `shift` / `carToggle` commands go to the car. The crosshair still works for cabin controls (key, stalks, radio, glovebox).
- E while targeting nothing in the cabin exits to the nearest free side (checks for a clear capsule position on that side, then the other, then the roof).
- Passenger seat is selectable; driving inputs are ignored there.

### 3.4 Inventory & needs

- M1: hands only (one held item). The car's trunk, bed and glovebox are physical storage.
- `Needs` component (hunger, thirst, fatigue) is defined but not simulated until M2.

---

## 4. World

### 4.1 M1 map

- Strip ~**4 km × 1 km**, terrain from one 16-bit heightmap (1 px = 1 m), max relief ~60 m, gentle hills.
- **Road:** two-lane asphalt (7 m wide) defined as a Catmull-Rom spline, terrain flattened under it with smooth shoulders. Runs the full length; ends at a dead end at the far edge (future attachment point for the endless road).
- **Home base** at the start: a garage (open shed) with a workbench, a wrench, a spare battery, a spare tyre, two jerry cans (one petrol, one empty), an oil canister (4 L), a water bottle set (2× 1.5 L), and the **Laika** parked inside.
- **Gas station** at ~2 km: two pumps (petrol, diesel), a canopy, a small shop building (closed in M1).
- **Wrecks:** 2 abandoned Laika shells along the road with a random subset of parts at low condition — the scavenging test case.
- Scatter: low-poly trees (3 kinds), rocks (3 kinds), road signs, distance markers every 500 m, fence segments.

### 4.2 Chunks

- Chunk size **256 m × 256 m**. Each chunk's data:

```ts
interface ChunkData {
  cx: number; cz: number;
  heights: Float32Array;        // 65×65 samples (4 m spacing) for collider + mesh
  roadSegments: SplineSegment[];
  props: PropSpawn[];           // instanced, static colliders where needed
  items: ItemSpawn[];           // dynamic grabbables / parts / vehicles with spawn ids
  structures: StructureSpawn[]; // garage, gas station
}
```

- Chunks within **~1 km** (4-chunk radius) of the player are loaded; others unloaded. Loading creates the terrain mesh, a Rapier heightfield collider, instanced props, and spawns items not yet tracked.
- **`WorldSource` interface:** `getChunk(cx, cz): ChunkData`. M1 ships `FixedWorldSource` (pre-baked chunk files built by a script from the heightmap + `map.json` layout). M4 adds `ProceduralWorldSource(seed)`; nothing else changes.
- **Persistence of moved items:** an item spawned from a chunk becomes **tracked** the first time it is grabbed, moved by physics beyond 0.5 m, poured from/into, or damaged. Tracked items are saved with full state and are not respawned from chunk data. Untracked items are recreated from spawn data on chunk load.
- **Vehicles are always tracked** and keep simulating while within the loaded radius. A vehicle in an unloading chunk is frozen (body set to sleep, kinematic) until its chunk reloads.

### 4.3 Floating origin

When the player is more than **2 km** from the origin, every body, chunk mesh and tracked transform is shifted by the player's offset (rounded to chunk size) in one tick. Sim keeps a `worldOffset` (float64) so saves store absolute positions. Built in M1 even though the map barely needs it, because M4 requires it.

### 4.4 Sky, time and lighting

- Day/night cycle: default **24 real minutes per in-game day**, configurable. Game starts at 08:00.
- Sky shader (Three.js `SkyMesh` / node sky), directional sun with one shadow cascade set (3 cascades, 200 m), moonlight at night, distance fog matched to sky colour, star dome at night.
- Nights are dark enough that headlights are necessary (ambient ≈ 0.03 at midnight).
- Weather: none in M1 (M6).

### 4.5 Gas station pumps

- Each pump has a nozzle on a hose (a Rapier rope joint, max length 3 m). Pick up the nozzle (LMB); while it is in an open opening and LMB is held, it pumps **1.0 L/s** of the pump's liquid. Releasing LMB stops; RMB drops the nozzle, which springs back to the pump when far away.
- The pump shows litres dispensed on its display. Fuel is free in M1 (money arrives in M3).

---

## 5. Car

Fully specified in the companion document: **[Car Systems Design](./2026-10-08-car-systems-design.md)**. Summary of M1 scope:

- One vehicle model, the **Laika** (a low-poly, Lada-like 4-door rear-wheel-drive sedan), plus two wreck shells.
- Part tree with ~40 parts, condition tiers, bolting with a wrench, detachment.
- Fluids: fuel, engine oil, coolant as mixed liquids, pouring and draining.
- Electrics: battery, alternator, ignition, starter, all lights, horn, radio, gauges.
- Engine, auto-clutch, 4-speed gearbox with automatic and manual modes, open differential, brakes.
- Custom raycast vehicle physics, tyre model, punctures, collision damage.
- Cabin: dashboard, window cranks, doors/hood/trunk, glovebox and trunk storage, mirrors, radio, seats.
- Sound: layered engine sound and all mechanical sounds.

---

## 6. Persistence

- **Save = JSON snapshot** of sim state: all entities with their components (except Rapier handles, which are rebuilt), `worldOffset`, clock, settings that affect gameplay, and the set of consumed spawn ids per chunk.
- `schemaVersion` integer at the root. `sim/save/migrations.ts` holds one pure function per version bump (`v1 → v2`, …). Loading runs migrations in order.
- Stored in **IndexedDB** (`idb-keyval`), one key per slot. M1 has one slot plus `backup`. Before writing a new save, the previous one is copied to `backup`.
- **Autosave** every 2 minutes and on `visibilitychange` → hidden / `pagehide`. Manual save from the pause menu.
- Loading failure (parse error, migration throws, validation fails) → offer the backup; if that fails, offer a new game. Never silently discard.

---

## 7. Robustness

| Failure | Handling |
|---|---|
| No WebGPU | `WebGPURenderer` falls back to WebGL2 automatically; settings show which backend is active. |
| Rapier WASM fails to load | Full-screen error with the message and a reload button. |
| Frame takes too long | Max 5 sim ticks per frame; extra time dropped (game slows rather than freezes). |
| Body falls out of the world | Any vehicle/item below terrain height − 50 m: items deleted (if untracked) or teleported to the nearest road point (vehicles, tracked items). Player respawns at the nearest road point. |
| NaN in physics | `vehicleForces` validates outputs; a NaN freezes that vehicle, logs the state to the console, and resets it upright on the spot. |
| Corrupt save | Backup slot (see §6). |
| Pointer lock lost | Pause menu opens. |

---

## 8. Performance budget

- 60 fps at 1080p on a mid-range laptop integrated GPU (e.g. Intel Iris Xe / Apple M1).
- < 300 draw calls, < 500k triangles visible.
- Initial download < 15 MB (gzip) for M1, including Rapier WASM (~1.5 MB) and audio.
- Mirrors are render targets at 256×128, updated at 30 Hz, individually toggleable.
- Sim tick budget: < 4 ms on the same hardware.

---

## 9. Testing strategy

- **Unit tests (Vitest, Node):** every sim system in isolation — liquid mixing maths, pour rates, fuel-quality thresholds, oil starvation → seizure, coolant loss → overheating, battery drain/charge balance, start conditions (each failure reason), auto-clutch engagement, gearbox shift logic, part attach/detach and mass recompute, condition tiers, damage distribution, save round-trip and migrations, chunk tracking rules.
- **Scenario tests (Vitest, Node, real Rapier):** a car on a flat test track driven by scripted commands. Assertions: 0–100 km/h in 16–22 s, top speed 130–150 km/h, braking 100→0 within 40–50 m, fuel use within 6–9 L/100 km at 90 km/h, engine reaches 85–95 °C and holds (full list in Car spec §15.2), car parked on 15° slope with handbrake doesn't move.
- **Smoke test (Playwright):** boot the game, wait for the first frame, walk to the car with scripted input, enter, start the engine, assert RPM > 0 via a `window.__debug` hook (dev builds only).
- **CI (GitHub Actions):** `lint`, `typecheck`, `test` (unit + scenario), `build`, smoke test, on every PR.

---

## 10. Repository layout (M1 target)

```
docs/
  superpowers/specs/      design specs (this file, car spec)
  superpowers/plans/      implementation plans
src/
  app/                    main.ts, loop.ts, input/, settings/, ui/ (menus)
  view/                   renderer.ts, sync.ts, materials/, audio/, hud/, mirrors/, debug/
  sim/
    world.ts              sim state container + step()
    ecs.ts                miniplex world + component types
    systems/              one file per system in §2.2
    interaction/          raycast, rules table
    liquids/              LiquidMix, transfer
    vehicle/              part catalog, slots, engine, drivetrain, electrical, tyres, damage
    worldgen/             WorldSource, FixedWorldSource, chunk streaming, floating origin
    save/                 snapshot, restore, migrations
  shared/                 math, units, constants
assets/
  models/ (glTF, CC0 sources listed in assets/CREDITS.md)
  audio/
  map/ heightmap.png, map.json
scripts/
  bake-chunks.ts          heightmap + map.json → chunk files
tests/
  unit/  scenario/  e2e/
```

---

## 11. Roadmap

Each milestone gets its own spec and plan when it starts. Only M0–M1 are planned in detail now.

| Milestone | Content | Constraints it puts on M1 |
|---|---|---|
| **M0 Foundations** | Vite + TS scaffold, loop, sim/view split, CI, debug overlay, save skeleton. | — |
| **M1 Car & world** (current) | Player, interaction, fixed map, the Laika with every system in the Car spec. | — |
| **M2 Survival** | Hunger, thirst, fatigue; food/drink items (which are liquids/items using existing systems); sleeping in the car. | `Needs` component reserved; liquids extensible. |
| **M3 Loot & economy** | Money, shops at gas stations, lootable buildings, more tools (jack, screwdriver), more item types. | Interaction rule table; pump has a price hook. |
| **M4 Procedural world** | `ProceduralWorldSource(seed)`, endless road, biomes, fixed home base as a hand-authored chunk set. | `WorldSource` interface, chunks, floating origin, tracked items. |
| **M5 More vehicles** | More car models (pickup, van, bus, moped), engine swaps across models, roof rack, trailers. | Data-driven part catalog and slot tags; vehicles defined as data. |
| **M6 Weather & hazards** | Rain/fog/wind, wipers matter, wet grip, night threats. | Wipers, hood-off cooling hook, tyre grip multiplier. |
| **M7 Co-op** | Host-authoritative co-op over WebRTC; clients send commands, host sends snapshots/diffs. | Commands seam, serializable state, stable entity ids. |
