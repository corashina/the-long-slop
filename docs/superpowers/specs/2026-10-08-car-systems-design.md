# My Summer Slop — Car Systems Design Spec

- **Date:** 2026-10-08
- **Status:** Approved design, awaiting written-spec review
- **Parent spec:** [Game Design & Architecture](./2026-10-08-game-design.md)
- **Scope:** every system of the M1 vehicle, the **Laika**, at implementable detail: parts, condition, fluids, electrics, engine, drivetrain, physics, damage, cabin, sound, how they interact, and how to test them.

All numbers are **starting tunables**. They live in data files (`src/sim/vehicle/data/*.ts`), are editable live in the debug overlay, and are validated by the scenario tests in §15. Where a number is tuned later, update this spec in the same PR.

---

## 1. Feature checklist (The Long Drive parity)

What a player can do with the car in M1. "Later" items are explicitly out of M1 scope and noted with their milestone.

| # | Feature | M1 | Section |
|---|---|---|---|
| 1 | Enter/exit driver and passenger seats | ✅ | §10.1 |
| 2 | Turn ignition key through Off / Accessory / On / Start | ✅ | §6.3 |
| 3 | Engine start depends on battery, fuel, oil, engine condition | ✅ | §7.2 |
| 4 | Drive: throttle, brake, steer, handbrake | ✅ | §8, §9 |
| 5 | Automatic gearbox (R/N/D) + optional manual mode | ✅ | §8.2 |
| 6 | Roll / push start with a dead battery | ✅ | §7.3 |
| 7 | Refuel from gas station pump with a hose nozzle | ✅ | §5.4 |
| 8 | Refuel / top up from jerry cans and bottles by pouring | ✅ | §5.3 |
| 9 | Wrong fuel or contaminated fuel has consequences | ✅ | §5.6 |
| 10 | Engine oil level and quality; oil burning; seizure | ✅ | §5.7 |
| 11 | Coolant level, leaks, overheating, boiling | ✅ | §5.8, §7.5 |
| 12 | Drain fuel, oil and coolant via drain plugs | ✅ | §5.5 |
| 13 | Battery charge/drain, alternator charging, dead battery | ✅ | §6 |
| 14 | Headlights low/high, tail, brake, reverse, turn signals, hazards, dome light | ✅ | §10.3 |
| 15 | Horn | ✅ | §10.2 |
| 16 | Radio with tuning, volume, stations, static | ✅ | §10.7 |
| 17 | Wipers (visual; matter in M6) | ✅ | §10.4 |
| 18 | Doors, hood, trunk open/close/slam; window cranks | ✅ | §10.4 |
| 19 | Glovebox and trunk storage that carries items | ✅ | §10.5 |
| 20 | Rear-view and side mirrors | ✅ | §10.6 |
| 21 | Dashboard: speedo, tacho, odometer, trip, fuel, temp, warning lights | ✅ | §10.2 |
| 22 | Every part removable/installable with a wrench | ✅ | §3 |
| 23 | Part condition tiers with visual + functional effect | ✅ | §4 |
| 24 | Scavenging parts from wrecks | ✅ | §3.5 |
| 25 | Petrol ↔ diesel engine swap | ✅ | §3.4 |
| 26 | Wheel change with a spare | ✅ | §9.6 |
| 27 | Tyre punctures and flats | ✅ | §9.6 |
| 28 | Collision damage, glass breaking, parts falling off | ✅ | §11 |
| 29 | Flip an overturned car | ✅ | §11.5 |
| 30 | Cargo slides out of an open trunk | ✅ | §10.5 |
| 31 | Layered engine sound reacting to RPM/load/misfire | ✅ | §12 |
| 32 | Paint / spray cans | Later (M3) | — |
| 33 | Car jack, tyre pump, screwdriver | Later (M3) | — |
| 34 | Other car models, cross-model swaps | Later (M5) | — |
| 35 | Roof rack, trailers, towing | Later (M5) | — |
| 36 | Wet grip, wipers clear rain | Later (M6) | — |
| 37 | Cassette tapes / custom music | Later (M3) | — |

---

## 2. The Laika (vehicle definition)

A low-poly, Lada-2101-like four-door rear-wheel-drive sedan.

| Property | Value |
|---|---|
| Total mass (all parts, full fluids) | ~1,060 kg |
| Bare chassis mass (shell + gearbox + axles + interior trim) | 640 kg |
| Wheelbase / track | 2.42 m / 1.36 m |
| Wheel radius (13" rim, 175/70 tyre) | 0.29 m |
| Ground clearance | 0.17 m |
| Drag area (Cd·A) | 0.95 m² |
| Drivetrain | Front engine, rear-wheel drive, open differential |
| Top speed (target) | ~140 km/h |
| 0–100 km/h (target) | 16–22 s |

The vehicle is defined as data (`src/sim/vehicle/data/laika.ts`): chassis mass/inertia, collider shapes, slot list (§3.2) with anchor transforms, wheel geometry, suspension and brake tunables, gearbox ratios, dashboard layout. New models in M5 are new data files.

### 2.1 Starting state (home base)

| Item | State |
|---|---|
| All parts | Good, condition random in 0.60–0.80, all bolted |
| Fuel tank | 15 L petrol |
| Engine oil | 3.6 L motor oil |
| Coolant | 5.0 L water |
| Battery | charge 0.80 |
| Gearbox | N, automatic mode; handbrake on; ignition Off |
| Odometer | random 80,000–180,000 km |

---

## 3. Part tree

### 3.1 Concepts

- **Slot:** a named attachment point on the chassis or on another part. Has an anchor transform, a list of accepted **tags**, and a **bolt count**.
- **Part:** a physical item with `typeId`, `tags`, `mass`, `condition`, `bolted`, and type-specific `state`.
- **Installed** = the part sits in a slot. It is no longer its own rigid body: its collider is added as a compound collider to the chassis body, and its mass is added to the chassis.
- **Detached** = the part is its own dynamic rigid body with a `Grabbable` component.
- **Bolted** = installed and every bolt tightened. An installed but unbolted part functions, but detaches on any impact with severity > 1 m/s (§11) or when the car's vertical acceleration exceeds 2 g (big bump).

### 3.2 Laika slot list

| Slot path | Accepts tags | Bolts | Part mass (kg) | Function | If missing or broken (condition 0) |
|---|---|---|---|---|---|
| `engine` | `engine:laika-mount` | 4 | 120 (petrol) / 135 (diesel) | Torque, heat, alternator, oil sump | No propulsion, no charging, no oil/coolant circuit |
| `engine/oilCap` | — (integral) | — | — | Oil filler opening | — |
| `battery` | `battery:12v` | 2 | 14 | Stores charge; powers everything | No electrics at all (except an already-running engine whose alternator keeps the ignition alive) |
| `coolantTank` | `radiator:laika` | 2 | 6 | Holds coolant; rejects engine heat | No cooling beyond air cooling → overheats in minutes |
| `fuelTank` | `fueltank:laika` | 4 | 10 | Holds fuel | Engine starves and stops |
| `exhaust` | `exhaust:laika` | 3 | 12 | Noise reduction | Engine much louder; no performance effect |
| `hood` | `hood:laika` | 4 | 15 | Covers engine bay | Engine bay exposed (cooling hook for M6 rain) |
| `trunkLid` | `trunklid:laika` | 4 | 12 | Covers trunk | Trunk always open; cargo can slide out |
| `door.fl` / `door.fr` / `door.rl` / `door.rr` | `door:laika:fl` etc. | 2 each | 20 each | Closes cabin | None functionally; lighter car; more wind noise |
| `door.*/window` | `window:laika:*` | 0 (snap) | 4 each | Glass; crankable | Wind noise |
| `windshield` | `windshield:laika` | 0 (snap) | 10 | Glass | Wind noise; wipers do nothing |
| `bumper.front` / `bumper.rear` | `bumper:laika:*` | 2 each | 8 each | Absorbs impacts first (§11) | Impacts go straight to the next parts |
| `wheel.fl` / `.fr` / `.rl` / `.rr` | `wheel:13in` | 4 each | 15 each | Contact with ground | That corner drops onto the hub (§9.6) |
| `headlight.l` / `.r` | `headlight:laika` | 2 each | 2 each | Low/high beam | That lamp is dark |
| `taillight.l` / `.r` | `taillight:laika` | 2 each | 1 each | Tail, brake, reverse lamps | Those lamps dark |
| `indicator.fl` / `.fr` / `.rl` / `.rr` | `indicator:laika` | 0 (snap) | 0.3 each | Turn signal | That lamp dark; blink rate doubles (like a real flasher relay) |
| `seat.driver` / `seat.passenger` | `seat:laika:front` | 4 each | 15 each | Sitting | Cannot sit there → cannot drive without a driver's seat |
| `seat.rear` | `seat:laika:rear` | 2 | 20 | Rear bench; also cargo surface | — |
| `steeringWheel` | `steeringwheel:laika` | 1 | 2 | Steering | Steering input ignored; wheels self-centre |
| `radio` | `radio:din` | 0 (snap) | 1.5 | Radio | No radio |

Total installed part mass with fluids ≈ 420 kg, chassis 640 kg → ~1,060 kg.

### 3.3 Bolting

- Hold **wrench** + aim at a part + hold LMB → a radial progress ring; each bolt takes **0.5 s**. Releasing LMB keeps progress per bolt (bolts are individually tracked: `boltsTight: number`).
- Part counts as `bolted` when `boltsTight === bolts`. Parts with 0 bolts (glass, indicators, radio) snap in/out by hand: aim + hold LMB 0.5 s with empty hands to remove.
- To **remove**: unbolt all bolts, then grab it (LMB with empty hands). To **install**: hold the part near an empty compatible slot (within 0.4 m of the anchor, roughly aligned) → a ghost preview shows → press LMB to snap it in (unbolted), then bolt it.
- **Access rules:** some slots are reachable only when another part is open or absent:
  - `engine`, `battery`, `coolantTank`, oil cap: hood open or missing.
  - `fuelTank`, `exhaust`, drain plugs: from below / beside (crouch); no jack needed in M1.
  - `engine` removal additionally requires `coolantTank` removed (it sits in front of the engine).
  - Seats, steering wheel, radio: from inside the cabin (door open).
  Access is enforced by the interaction raycast hitting blocking colliders, plus an explicit `requires` list on the slot for the engine case.

### 3.4 Part variants and swaps

Variants share slot tags but differ in stats. M1 catalog:

| Type id | Tags | Notes |
|---|---|---|
| `engine.laika13` | `engine:laika-mount`, fuel `petrol` | 47 kW @ 5,600 rpm, 89 N·m @ 3,400, redline 6,000 |
| `engine.laika15d` | `engine:laika-mount`, fuel `diesel` | 37 kW @ 4,200 rpm, 92 N·m @ 2,200, redline 4,600; found in wreck #2 |
| `battery.std45` | `battery:12v` | 45 Ah |
| `wheel.steel13` | `wheel:13in` | Steel rim, 175/70R13 tyre |

Engine type determines which fuel it needs (§5.6). Swapping is purely physical: remove one engine, install the other. Oil and coolant stay with the **engine** (oil) and the **coolant tank** (coolant), because the liquid containers are owned by those parts.

### 3.5 Wrecks

Two wreck shells on the map. Each is a Laika chassis spawned with a seeded random subset of parts (each slot 60% chance present), conditions in 0.05–0.5, fluids empty except a random 0–3 L of old oil in the engine. Wreck #2 always has the diesel engine. Wrecks are vehicles like any other: if you complete one, it drives.

---

## 4. Condition

### 4.1 Tiers

`condition ∈ [0, 1]`, continuous in sim. The view maps it to tiers:

| Range | Tier | Look |
|---|---|---|
| 0.80–1.00 | **Pristine** | Base colour, glossy |
| 0.55–0.80 | **Good** | Base colour, satin |
| 0.25–0.55 | **Worn** | Desaturated, scuffed (vertex-colour noise) |
| 0.01–0.25 | **Rusty** | Brown/orange, rough |
| 0 | **Broken** | Rusty + cracked/bent mesh variant; functions as missing |

The prompt shows the tier when looking at a part (`Battery — Worn`).

### 4.2 Sources of condition loss

| Source | Affects | Rate |
|---|---|---|
| Distance driven | engine, tyres, exhaust, coolant tank | engine 0.0002/km × rpmFactor (1 + max(0, rpm − 4,000)/2,000); tyres 0.0003/km; exhaust 0.0001/km; coolant tank 0.0001/km |
| Tyre slip | that tyre | 0.002 per second with slip ratio > 0.3 or slip angle > 0.25 rad |
| Impacts | parts near contact | §11 |
| Overheating | engine | §7.5 |
| Poor lubrication | engine | §5.7 |
| Contaminated fuel | engine | §5.6 |
| Deep discharge | battery | −0.01 each time charge reaches 0 |

### 4.3 Functional effect per part

| Part | Effect of condition `c` |
|---|---|
| Engine | Torque × (0.55 + 0.45·c); start chance per attempt 0.3 + 0.7·c; oil burn (§5.7); fuel efficiency ÷ (0.7 + 0.3·c) |
| Battery | Internal resistance 0.010 + 0.040·(1 − c) Ω; usable capacity 45 Ah × (0.4 + 0.6·c) |
| Coolant tank | Heat rejection × (0.4 + 0.6·c); leak (§5.8) |
| Fuel tank | Leak 0.5 L/h × max(0, 0.25 − c)/0.25 when c < 0.25 |
| Exhaust | Loudness × (1 + 1.5·(1 − c)) |
| Tyres | Grip × (0.6 + 0.4·c); random puncture chance (§9.6) |
| Lights | Brightness × (0.5 + 0.5·c) |
| Glass | Shatters below a smaller impact threshold as c drops (§11) |
| Doors / hood / trunk | At c < 0.15 the latch fails: opens on impacts > 1.5 m/s |
| Radio | Extra static × (1 − c) |
| Seats / steering wheel / bumpers / indicators | Cosmetic, plus §11 impact absorption for bumpers |

---

## 5. Fluids

### 5.1 `LiquidMix`

```ts
type Liquid = 'petrol' | 'diesel' | 'oil' | 'water';   // extensible (M2: 'juice', 'alcohol', …)

interface LiquidMix { [liquid: string]: number }       // litres per liquid

interface LiquidContainer {
  mix: LiquidMix;
  capacity: number;          // litres
  capOpen: boolean;          // openings are sealed when false
  opening: Vec3;             // local position of the opening (filler neck / bottle mouth)
  drain?: Vec3;              // local position of a drain plug, if any
  drainOpen?: boolean;
}

total(mix) = Σ litres
fraction(mix, l) = mix[l] / total(mix)   // 0 when empty
```

Pure functions in `sim/liquids/mix.ts`: `total`, `fraction`, `add(mix, other)`, `take(mix, litres) → {taken, rest}` (proportional), `scale`. Every function is unit-tested; volumes never go negative, and adding beyond capacity overflows (the excess spills).

### 5.2 Containers in M1

| Container | Capacity | Owner | Opening |
|---|---|---|---|
| Fuel tank | 45 L | `fuelTank` part | Filler neck rear-left, behind a flap + screw cap |
| Engine oil | 4.0 L | `engine` part | Oil filler cap on the valve cover (hood must be open) |
| Coolant | 6.0 L | `coolantTank` part | Radiator cap (hood must be open) |
| Jerry can | 20 L | item | Screw cap |
| Oil canister | 4 L | item | Screw cap |
| Water bottle | 1.5 L | item | Screw cap |
| Pump nozzle | ∞ source | gas station | — |

### 5.3 Pouring

- Both the source opening and the target opening must be **open** (caps are toggled with E; the fuel flap opens with the cap).
- The held source must be tilted: its opening must point below horizontal (the player tilts with R + mouse; a default "pour pose" auto-tilts when LMB is held over a valid target, so players need not fiddle).
- The target opening must be within **0.25 m** horizontally and below the source opening.
- Flow rate: **0.5 L/s** for jerry cans and canisters, 0.25 L/s for bottles, 1.0 L/s for pump nozzles. Flow carries the source's mix proportionally (`take` then `add`).
- If the target is full, the excess spills (visual particles, liquid lost).
- Tilting an open container over nothing spills at the same rate.
- A detached, open container lying on its side leaks at the same rate (so you learn to close caps).

### 5.4 Gas station

See the parent spec §4.5. The nozzle is a held item whose "contents" are an infinite source of one liquid; it uses the same pour rule.

### 5.5 Draining

- Fuel tank, engine sump and coolant tank have a **drain plug** (E to open/close; reachable from beside the car while crouched).
- An open drain plug flows at 0.3 L/s. Any open container held under it (opening within 0.25 m below) receives the flow; otherwise it spills.
- Draining is how players fix contaminated tanks (§14 scenario B).

### 5.6 Fuel

- Engine fuel type: petrol or diesel (§3.4). **Fuel quality** `q = fraction(tankMix, engineFuel)`.
- The engine draws fuel from the tank proportionally (it consumes the mix, not just the correct fuel).

| q | Behaviour |
|---|---|
| ≥ 0.85 | Normal |
| 0.60–0.85 | Misfires: power factor `0.5 + 0.5·(q − 0.6)/0.25`; random misfire events at `(0.85 − q)·8` per second (torque drop for 0.1 s, audio stutter) |
| < 0.60 | Stalls within 1–2 s; cannot start |

- **Damage from contamination:** while running with `q < 0.95`, engine condition −0.001/s × (1 − q).
- **Consumption:** `fuelFlow (L/s) = (0.8 / 3600) + 0.405 × P_kW / 3600 / (0.7 + 0.3·c_engine)` for petrol; diesel uses 0.6 L/h idle and 0.33 L/kWh. Here `P_kW` is the engine's current output power (≥ 0). This gives about 7 L/100 km at a steady 90 km/h.
- **Fuel gauge** reads `total(tankMix)` regardless of what is in it: a tank full of water reads full.
- Fuel tank empty (`total < 0.05 L`) → engine stops (sputters for 1 s first).

### 5.7 Engine oil

- **Lubrication** `λ = clamp(total(oil)/4.0, 0, 1) × fraction(oilMix, 'oil')`.
- **Oil burn:** `(0.02 + 0.28·(1 − c_engine)²)` L per 100 km driven, removed as oil.

| λ | Behaviour |
|---|---|
| ≥ 0.5 | Normal |
| 0.05–0.5 | Extra friction heat `(0.5 − λ) × 40 kW × rpm/6,000`; engine wear rate × `1 + 20·(0.5 − λ)/0.5`; ticking sound |
| < 0.05 while running | Seizure timer: after **5 s**, engine condition → 0, state `seized`, engine stops with a bang; wheels lock briefly if in gear (clutch disengages after 0.3 s) |

- Oil warning light: `λ < 0.25` (lit when ignition On, whether running or not).

### 5.8 Coolant

- **Coolant factor** `κ = clamp(total(coolant)/6.0, 0, 1)^1.5 × fraction(coolantMix, 'water')`. Any non-water liquid in the radiator reduces cooling proportionally (oil in the radiator is bad).
- **Leak:** `max(0, 0.55 − c_tank) × 4` L per 100 km (Worn and worse tanks leak; a rusty 0.1 tank loses ~1.8 L/100 km). Leaking drips spawn a small puddle decal under the car.
- **Boil-off:** when engine temperature > 110 °C, coolant is lost at 0.5 L/min with steam particles from the hood.
- Coolant warning light: `total(coolant) < 1.5 L` (25%).

---

## 6. Electrics

### 6.1 Battery

- State: `charge ∈ [0, 1]`, `condition`.
- Open-circuit voltage `V0 = 11.8 + 0.9 × charge`.
- Internal resistance `R = 0.010 + 0.040 × (1 − c)` Ω.
- Terminal voltage under load `V = V0 − I_net × R` when discharging.
- Usable capacity `Ah = 45 × (0.4 + 0.6·c)`.
- Each tick: `charge -= I_net × dt / 3600 / Ah` (negative `I_net` charges). Clamp to [0, 1]; reaching 0 costs condition (§4.2).
- No self-discharge in M1.

### 6.2 Loads (amps at 12 V)

| Load | A | Condition |
|---|---|---|
| Ignition, fuel pump, dash | 4.0 | Key ≥ On |
| Starter motor | 120 | Key = Start |
| Headlights low / high | 9 / 11 | Light switch; both lamps present scale the load (−50% per missing lamp) |
| Tail + dash illumination | 2.0 | Lights ≥ low |
| Brake lights | 4.0 | Brake > 0.05 |
| Reverse lights | 2.0 | In R |
| Turn signals / hazards | 1.5 / 3.0 (average) | Active |
| Dome light | 0.5 | Door open or switched on |
| Radio | 1.5 | Key ≥ Accessory and on |
| Horn | 4.0 | Pressed |
| Wipers | 3.0 | On |
| Radiator fan | 8.0 | Engine temperature > 95 °C and key ≥ On |

Example: headlights (11 A) left on for 8 hours drain 88 Ah, about two full batteries, so the battery is dead by morning.

### 6.3 Ignition

Key positions cycle by clicking the key (E) or holding a key shortcut (default `K` tap = next position, `K` hold from On = Start):

| Position | Powers |
|---|---|
| **Off** | Nothing except lights, horn, hazards, dome (wired direct, as on old cars) |
| **Accessory** | + radio, wipers |
| **On** | + ignition circuit, fuel pump, dash gauges/warnings, fan |
| **Start** (momentary, held) | + starter motor; radio and wipers cut out while cranking |

Releasing Start returns to On. Turning to Off stops a running engine.

### 6.4 Alternator

- Built into the engine part (no separate part in M1).
- Output capacity `I_alt = 45 A × smoothstep(900, 2,000, rpm)` while the engine is running.
- **Net battery current:** `I_net = Σ loads − I_alt_delivered`, where the alternator delivers `min(I_alt, Σ loads + acceptance)` and `acceptance = 1 + 20 × (1 − charge)` A.
- If the battery is missing or at 0 charge but the engine is running and `I_alt ≥ ignition load`, the engine keeps running (system voltage from the alternator).
- **Battery warning light:** key ≥ On and (engine not running, or `I_alt < Σ loads`).

### 6.5 Voltage effects

- System voltage `Vsys` = battery terminal voltage, or 14.2 V while the alternator covers all loads.
- Lights brightness × `clamp((Vsys − 9) / 3.6, 0, 1)` → lamps visibly dim on a weak battery.
- Gauges and warning lights work only when `Vsys > 9 V`. Radio cuts out below 10 V. Horn pitch drops below 11 V.

---

## 7. Engine

### 7.1 State machine

```
            key=Start & can crank          catches (§7.2)
   OFF ─────────────────────────▶ CRANKING ─────────────────▶ RUNNING
    ▲  ◀──── key released / no crank ──┘                        │
    │                                                           │ key Off, fuel starvation,
    │◀───────────────────────── STALLING (0.5–1.5 s spin-down) ◀┘ q < 0.6, rpm < 400 in gear
    │
 SEIZED (terminal until a different engine is installed; condition 0)
```

### 7.2 Starting

While key = Start:

1. **Cranking speed** `ω_crank = 300 rpm × clamp((V − 8.5)/3, 0, 1)` where `V` is the battery terminal voltage with the 120 A starter load. Examples: full, good battery ≈ 300 rpm; charge 0.5 ≈ 255 rpm; charge 0.3 ≈ 237 rpm (won't catch); rusty battery ≈ 0 (click only).
2. **Preconditions to catch:** engine present and not seized; fuel tank present; `q ≥ 0.6`; fuel `≥ 0.05 L`; `ω_crank ≥ 250 rpm`.
3. Every **0.5 s** of qualifying cranking, roll `p = 0.3 + 0.7·c_engine` (diesel: `0.2 + 0.7·c`; cold diesel below 10 °C: × 0.5). Success → RUNNING at 900 rpm.
4. The starter sound speed follows `ω_crank`; a battery too weak to crank gives a solenoid click.

### 7.3 Roll start

In RUNNING-capable conditions (preconditions 2, except the cranking speed) with key ≥ On, gearbox in a forward gear, and road speed > 10 km/h, the drivetrain spins the engine. Every 0.5 s roll `p` as above; success → RUNNING. This works in automatic mode too (D at speed).

### 7.4 Torque

```
T_curve(rpm) = table (petrol laika13):
  rpm:   800  1500  2500  3400  4500  5600  6000
  N·m:    52    70    84    89    86    80    72
T_eng = T_curve(rpm) × throttleEff × (0.55 + 0.45·c) × fuelFactor(q) × misfire
      − T_friction(rpm, λ)
T_friction = 6 + 0.002·rpm  (N·m), × (1 + 3·max(0, 0.5 − λ))
throttleEff = max(throttle, idleThrottle)   // an idle controller holds 850 rpm when throttle ≈ 0
```

- Rev limiter: fuel cut above redline (6,000 petrol, 4,600 diesel), resume 200 rpm below.
- Engine inertia: 0.15 kg·m².
- Power `P_kW = T_eng × rpm × 2π / 60 / 1000` (used for fuel consumption and heat).

### 7.5 Heat

Engine temperature `T` (°C), ambient `T_amb = 20` (M6 varies it). Thermal mass `C = 60 kJ/K`.

```
Q_in  (kW) = 6 (idle base) + 1.2 × P_kW + friction extra (§5.7)
o     = clamp((T − 85)/10, 0, 1)                         // thermostat opening
air   = 0.35 + 0.65 × min(1, speed / 25 m/s) (+0.30 if fan running)
G     = 0.03 + o × 0.8 × κ × air × (0.4 + 0.6·c_tank)     // kW/K; 0.03 = bare-block cooling
Q_out (kW) = G × (T − T_amb)
dT/dt = (Q_in − Q_out) / C
```

When the engine is off, `Q_in = 0` and it cools toward ambient. Missing coolant tank: `κ = 0`.

Behaviour this produces (checked in §15):

- Warm-up to ~87 °C in about 3–5 minutes of driving.
- Steady ~88–90 °C cruising, ~97 °C at sustained full throttle (fan engages above 95 °C).
- With no coolant: past 110 °C within ~2 minutes of driving.

| T | Effect |
|---|---|
| > 95 °C | Fan on (if powered) |
| > 110 °C | Engine condition −0.002/s × (T − 110)/10; coolant boil-off (§5.8); steam; temperature needle in the red |
| > 130 °C | Each second: 5% chance to seize |

### 7.6 Stalling

RUNNING → STALLING when: key Off; fuel empty; `q < 0.6` (after 1–2 s); rpm < 400 (only possible if the auto-clutch is overwhelmed, e.g. pushing against a wall in gear with a low-condition engine); seizure.

---

## 8. Drivetrain

### 8.1 Auto-clutch

There is no clutch pedal. The clutch is modelled as a torque-limited coupling between engine and gearbox input:

- Engagement `e = smoothstep(1,000, 1,500, rpm_engine)` when a gear is engaged and throttle > 0.05, or `e = 1` once `rpm_gearboxInput > 1,200` (locked up at speed).
- `e = 0` in N, while shifting, or when rpm would fall below 700 (anti-stall).
- Max clutch torque `T_clutch = e × 250 N·m`. If the torque difference across the clutch exceeds `T_clutch`, it slips (both sides receive ±T_clutch); otherwise engine and gearbox input are locked together.

### 8.2 Gearbox

| Gear | Ratio |
|---|---|
| R | −3.53 |
| 1 | 3.67 |
| 2 | 2.10 |
| 3 | 1.36 |
| 4 | 1.00 |
| Final drive | 4.10 |

- Shift time **0.35 s** (torque cut, clutch disengaged).
- Drivetrain efficiency 0.90.

**Automatic mode (default).** The selector is R / N / D:

- Upshift when `rpm > 2,500 + 2,800 × throttle`. Downshift when `rpm < 1,300 + 1,200 × throttle`. Kickdown when throttle > 0.9 and the lower gear keeps rpm < 5,200. Minimum 1.0 s between automatic shifts (hysteresis).
- **W/S semantics:** in D, W = throttle and S = brake. At standstill (< 1 km/h), holding S for 0.4 s selects R; in R, S = throttle (backwards) and W = brake; holding W at standstill for 0.4 s selects D. N is reached only via the gear lever (E on the lever cycles R/N/D).

**Manual mode (settings toggle).** The selector is R / N / 1 / 2 / 3 / 4:

- Shift up/down with **Shift / Ctrl** while driving (Shift and Ctrl are free in the seat because walking is disabled).
- W = throttle, S = brake always.
- Shifting from N down to R requires < 3 km/h. Shifting to a gear that would over-rev (> redline + 300) is refused with a grinding sound.

### 8.3 Differential

Open differential on the rear axle: equal torque to both rear wheels; wheel speeds independent. (A lifted, spinning rear wheel takes all the speed, a real and fun open-diff behaviour.)

### 8.4 Brakes

- Footbrake max torque: 1,200 N·m per front wheel, 600 N·m per rear wheel (front-biased), scaled by the brake input.
- **Handbrake:** 1,500 N·m on each rear wheel. Holds the car on a 15° slope.
- No ABS: locking wheels slide (tyre model handles it; tyre wear from slip).
- At standstill with brake applied, wheels are held by a static-friction clamp to prevent creeping.

---

## 9. Vehicle physics

### 9.1 Overview

The chassis is a single Rapier dynamic rigid body (compound collider: shell + installed parts). Wheels are **not** rigid bodies. Each wheel is simulated in `vehicleForces`:

```
for each wheel:
  ray from hub anchor along chassis-down, length = restLength + radius
  if hit:
     compression, compressionVelocity → suspension force Fz (spring + damper + anti-roll)
     contact frame (normal, forward, side) from steering angle
     wheel angular velocity ω integrates: drive torque − brake torque − tyre Fx × r
     slip ratio, slip angle → tyre forces Fx, Fy (§9.3), scaled by Fz and μ
     apply Fz·n + Fx·fwd + Fy·side at contact point to chassis
  else:
     wheel spins freely (drive − brake − bearing friction)
```

Rapier `castRay` against terrain, road, props and other vehicles (excluding own chassis). Sub-stepping: tyre forces are computed once per 60 Hz tick; if instability shows in testing, `vehicleForces` runs 2 sub-steps per tick.

### 9.2 Suspension

| Tunable | Front | Rear |
|---|---|---|
| Rest length (travel) | 0.18 m | 0.18 m |
| Spring rate | 28 kN/m | 24 kN/m |
| Damping bump / rebound | 2.5 / 3.5 kN·s/m | 2.5 / 3.5 kN·s/m |
| Anti-roll stiffness | 9 kN/m (difference) | 5 kN/m |
| Bump stop | Stiff spring 200 kN/m in the last 0.02 m | same |

### 9.3 Tyre model

Simplified, Pacejka-shaped:

```
slipRatio  s = (ω·r − v_x) / max(|v_x|, 1.0)
slipAngle  α = atan2(v_y, |v_x|)
Fx0 = Fz · μ · shape(s / 0.10, 0.80)     // peak at 10% slip, falls to 80% of peak when sliding
Fy0 = −Fz · μ · shape(α / 0.14, 0.85)    // peak at ~8°, falls to 85%
shape(x, tail) = x ≤ 1 ? sin(x·π/2) : lerp(1, tail, min(1, (x−1)/2))
friction circle: if √(Fx0² + Fy0²) > Fz·μ, scale both down to fit
low-speed: below 2 m/s blend toward a velocity-damping model to stop jitter at rest
```

`μ = μ_surface × pressureFactor × (0.6 + 0.4·c_tyre)`

| Surface | μ_surface |
|---|---|
| Asphalt | 1.00 |
| Dirt / gravel shoulder | 0.70 |
| Grass | 0.60 |
| Sand (future biomes) | 0.45 |

Surface comes from the collider's user data (road mesh vs. terrain material map).

### 9.4 Steering

- Max wheel angle 35° at standstill, linearly reducing to 8° at 120 km/h.
- Input rate 2.5 rad/s toward the target; self-centres at 3.5 rad/s when released.
- Ackermann geometry (inner wheel turns more), track 1.36 m, wheelbase 2.42 m.
- The steering wheel mesh rotates 3.5 turns lock to lock.

### 9.5 Resistances

- Aerodynamic drag `F = 0.5 × 1.225 × 0.95 × v²` (≈ 0.58·v² N).
- Rolling resistance `F = 0.013 × Fz` per wheel (×5 on a flat tyre).
- These yield a top speed of ~140 km/h in 4th, close to the rev limiter.

### 9.6 Tyres: pressure, punctures, flats, wheel removal

- Tyre `pressure` (bar), nominal 2.0. `pressureFactor = 1 − 0.4·|p − 2.0|/2.0`, clamped ≥ 0.4.
- **Puncture** (pressure → 0 over 3 s, hiss sound) when:
  - that wheel's suspension hits its bump stop with compression velocity > 4 m/s (curbs, rocks), with chance `0.2 + 0.6·(1 − c_tyre)`; or
  - an impact (§11) with severity > 8 m/s within 0.6 m of the wheel; or
  - randomly: `0.02 × (1 − c_tyre)²` per km.
- **Slow leak:** tyres below c = 0.25 lose 0.05 bar per km.
- **Flat** (p < 0.3): effective radius −0.06 m, μ × 0.4, rolling resistance × 5, flapping sound, the car pulls toward that side. Cannot be repaired in M1; swap the wheel (a pump arrives in M3).
- **Missing wheel:** the hub ray uses radius 0.12 m and a high-friction, no-drive "hub scraping" contact (μ 0.8, sparks, scrape sound). The corner drops ~0.17 m. The car still drives, badly.
- **Wheel change:** unbolt 4 bolts (2 s), pull the wheel (15 kg item), place the spare at the hub, bolt it. With no jack in M1, the car simply rests on the hub while the wheel is off.

---

## 10. Cabin, controls and feedback

### 10.1 Seats and camera

Covered in the parent spec §3.3. The driver's view sees the dashboard, mirrors and the road; free-look lets the player check the passenger side and the rear seats.

### 10.2 Dashboard

Gauges are physical meshes (needles, LCD-free odometer drums) driven by sim values, readable only when `Vsys > 9 V` and key ≥ On (odometer drums always readable):

| Instrument | Source |
|---|---|
| Speedometer (0–180 km/h) | Front-left wheel speed (as on real cars; lies on a spinning or locked wheel) |
| Tachometer (0–8,000 rpm, red from 6,000) | Engine rpm |
| Odometer + trip (click trip with E to reset) | Distance integrated from the same wheel |
| Fuel gauge (E–F) | `total(fuelTankMix) / 45`; needle damped (5 s time constant) |
| Temperature gauge (C–H) | Engine temperature 40–130 °C |
| Warning: battery | §6.4 |
| Warning: oil | §5.7 |
| Warning: coolant | §5.8 |
| Warning: handbrake | Handbrake on |
| Indicator arrows + tick sound | Turn signals / hazards |
| High beam | High beam on |
| Gear display (auto: R N D; manual: R N 1–4) | Gearbox state |

There is **no on-screen HUD** for car state; the debug overlay (F3) is a developer tool.

### 10.3 Lights

| Lamp | On when | Visual |
|---|---|---|
| Headlights low | Switch ≥ low | Two spot lights (range 40 m, 50°), emissive lens |
| Headlights high | Switch = high | Range 90 m, 35° |
| Tail lights | Switch ≥ low | Emissive red |
| Brake lights | Brake > 0.05 | Brighter emissive red |
| Reverse lights | Gear R | Emissive white + small point light |
| Turn signals | Stalk / hazards | Emissive amber at 1.5 Hz (3 Hz if a bulb is missing) |
| Dome | A door open, or dome switch | Point light in cabin |

Each lamp needs its part present with c > 0, a powered circuit, and brightness scales with voltage (§6.5) and condition (§4.3). To fit the performance budget, only the headlights cast real spot lights (max 2 per car, shadows off); other lamps are emissive with a bloom pass.

**Controls:** `L` cycles off → low → high → off (also the light stalk, E). `Z` / `X` left/right signal (stalk; auto-cancels after the steering returns past centre). `C` hazards (dash button). `H` horn (horn pad).

### 10.4 Doors, hood, trunk, windows, wipers

- Doors, hood and trunk lid are installed parts whose meshes hinge around their anchor. **While installed they are kinematic children of the chassis** (animated hinge angle, not physics joints) to keep the chassis simulation stable. The hinge angle is sim state, so players can push doors with their body or with car motion (the angle integrates a simple spring/inertia model driven by chassis acceleration).
- **E** opens or closes. Closing above 0.3 rad/s latches with a slam sound. Doors swing on hard acceleration if unlatched.
- **Window cranks:** E on the crank to start or stop winding; the window moves 0.25 m in 2 s. Open windows: wind noise.
- **Wipers:** dash switch off / slow / fast. Animated; power draw 3 A; no gameplay effect until M6 rain.

### 10.5 Storage: glovebox and trunk

- Storage volumes are box triggers in the chassis. Lids (glovebox door, trunk lid) are interactables.
- **Stowing:** when a lid closes, every grabbable item fully inside the volume becomes **stowed**: its rigid body is disabled and it is attached as a fixed child of the chassis (no jitter, no physics cost, mass added to the chassis). When the lid opens, stowed items become dynamic again, inheriting the chassis velocity.
- **Open trunk while driving:** items stay dynamic and can slide or fall out.
- The **rear seat** and the floor are just physical surfaces; items there slide around while driving.

### 10.6 Mirrors

- A rear-view mirror and two door mirrors, each a `RenderTarget` 256×128 updated at 30 Hz (round-robin, one mirror per frame at 60 fps), rendered from a mirrored camera with distance culling at 150 m.
- Each mirror can be turned off in settings; a door mirror on a missing door is gone.

### 10.7 Radio

- Controls (E on each knob): power/volume knob (click toggles power, scroll adjusts volume while targeted), tuning knob (scroll sweeps 88.0–108.0 MHz, 0.1 MHz steps).
- **Stations:** 3 stations at fixed frequencies, each with a playlist of CC0/royalty-free tracks and a transmitter position on the map. Playback position is global: `worldTime mod playlistLength`, so a station keeps "playing" while the radio is off.
- **Signal:** `tuneMatch = 1 − min(1, |f − f_station| / 0.3 MHz)`; `range = clamp(1 − dist / 6 km, 0, 1)`; `signal = tuneMatch × range`. Output = music × signal + static × (1 − signal) × (1 + (1 − c_radio)).
- Power from key ≥ Accessory and `Vsys > 10 V`.

### 10.8 Complete control map (defaults, rebindable)

| Context | Key | Action |
|---|---|---|
| On foot | WASD / Shift / Ctrl / Space | Move / sprint / crouch / jump |
| On foot | E / LMB / RMB / Wheel / R + mouse | Interact / grab, use / drop / hold distance / rotate held |
| Driver | W / S | Throttle / brake (direction-aware in automatic, §8.2) |
| Driver | A / D | Steer |
| Driver | Space | Handbrake (toggle on tap, hold for momentary) |
| Driver | Shift / Ctrl | Gear up / down (manual mode) |
| Driver | K (tap / hold) | Ignition next position / start |
| Driver | L / Z / X / C / H | Lights / left / right / hazards / horn |
| Driver | E (aiming) | Cabin controls; E aiming at nothing exits |
| Any | F3 / Esc | Debug overlay / pause |

---

## 11. Damage and detachment

### 11.1 Impact detection

Rapier contact-force events on the chassis (threshold set so light touches are ignored). For each event:

```
severity (m/s) = impulse (N·s) / chassisMass        // velocity change of the car
point          = contact point in chassis space
```

Impacts with `severity < 2 m/s` (about 7 km/h) do nothing.

### 11.2 Damage distribution

For every installed part, with anchor distance `d` from the impact point:

```
dmg = fragility(part) × (severity − 2) × falloff(d)
falloff(d) = max(0, 1 − d / 1.0 m)
shielding: parts behind a bumper (in the impact direction) take × 0.4 while the bumper's condition > 0
```

| Part | Fragility (condition per m/s above threshold) |
|---|---|
| Bumpers | 0.08 |
| Hood, trunk lid, doors | 0.05 |
| Headlights, taillights, indicators | 0.15 |
| Coolant tank (front) | 0.06 |
| Engine (only if severity > 8) | 0.03 |
| Fuel tank (rear, only if severity > 6) | 0.04 |
| Wheels (tyre) | 0.03, plus puncture (§9.6) |
| Glass | Shatters (condition → 0, removed, glass particles) when severity > `6 × (0.5 + 0.5·c)` within 1 m |

### 11.3 Detachment

- A part detaches (becomes its own dynamic body with the chassis velocity plus a small impulse) when its condition reaches 0 (doors, hood, trunk lid, bumpers, lights fall off), or when it is unbolted and severity > 1 m/s.
- Detached broken parts are still items: they can be carried, but count as broken (function as missing) if reinstalled.

### 11.4 Consequence chains

Damage feeds the other systems automatically: a dented coolant tank leaks (§5.8) → the engine overheats (§7.5); a punctured fuel tank leaks (§4.3); smashed headlights make night driving dangerous. See §13.

### 11.5 Flipping

When the chassis roll or pitch exceeds 100° and speed < 1 m/s, the body shows `Push to flip [hold E]`. Holding E for 1.5 s applies a torque that rolls the car back onto its wheels (and a small upward impulse). This is a deliberate gameplay concession, as in The Long Drive.

### 11.6 Player damage

None in M1 (no player health until M2). Being hit by the car pushes the player capsule.

---

## 12. Sound

All via Web Audio with positional `PannerNode`s. Inside the cabin, a low-pass filter (cutoff 1.2 kHz) applies to outside sources, lifted when the door near the listener is open.

| Sound | Driver |
|---|---|
| Engine | Four loops (idle ~850, low ~2,000, mid ~3,500, high ~5,500 rpm) crossfaded by rpm and pitch-shifted to match; a load layer adds intensity with throttle; misfire = 80 ms gain dip + pop; seizure = bang + grind. Exhaust condition/absence raises gain and adds a rasp layer. |
| Starter | Loop pitched by `ω_crank`; solenoid click when it can't crank. |
| Tyres | Squeal gain from slip ratio/angle on asphalt; gravel crunch on dirt; flat flap. |
| Suspension | Knock on bump stop. |
| Impacts | Three crunch samples by severity; glass shatter. |
| Body | Door / hood / trunk open, close, slam; window crank; latch. |
| Cabin | Indicator tick, ignition key, light stalk click, handbrake ratchet, gear lever, horn (pitch from voltage), wipers. |
| Fluids | Pour glug, spill splash, drain drip, pump hum, steam hiss, puncture hiss. |
| Wind | Gain from speed; louder with windows open or doors/windshield missing. |
| Radio | §10.7 |

Sounds are CC0 (Freesound CC0 filter) or synthesized; credits in `assets/CREDITS.md`.

---

## 13. Interaction matrix

How each system affects the others. Read the row as "this system → affects the column".

| From ↓ / To → | Engine | Electrics | Fuel | Oil | Coolant / heat | Drivetrain / physics | Lights / cabin | Sound |
|---|---|---|---|---|---|---|---|---|
| **Engine** | — | Alternator charges battery and powers loads (§6.4) | Consumes fuel by power (§5.6) | Burns oil by condition (§5.7) | Produces heat by power (§7.5) | Torque through clutch (§8.1) | Gauges (rpm, temp) | Engine loops, misfires |
| **Electrics** | Starter cranking speed decides starts (§7.2); fan cools (§7.5) | — | Fuel pump needs key On | — | Fan needs power | — | Lamps dim with voltage; radio and gauges cut out (§6.5) | Horn pitch, starter speed |
| **Fuel** | Quality → power, misfire, stall, damage (§5.6) | — | — | — | Misfire lowers heat input | Lower power | Fuel gauge (reads volume, not quality) | Stutter |
| **Oil** | λ → friction torque, wear, seizure (§5.7) | — | — | — | Friction heat (§5.7) | Seizure locks drivetrain briefly | Oil warning light | Ticking, seizure bang |
| **Coolant / heat** | Overheat damages engine; seizure risk (§7.5) | Fan load 8 A | — | — | — | — | Temp gauge, coolant light, steam | Steam hiss |
| **Drivetrain / physics** | Load sets rpm; roll start (§7.3) | — | — | — | Speed → airflow cooling (§7.5) | — | Speedometer, odometer | Tyres, wind, suspension |
| **Parts / condition** | Torque, start chance, efficiency (§4.3) | Battery capacity and resistance | Tank leaks | Oil burn rate | Radiator capacity, leaks | Grip; mass and centre of mass; missing wheels | Lamp brightness; missing lamps | Exhaust loudness |
| **Damage** | Engine condition | Battery (if hit) | Tank leaks | — | Radiator leaks | Punctures; detached parts change mass | Lamps break; glass breaks | Crunches |
| **Player actions** | Key, throttle | Lights left on drain the battery | Pour, pump, drain | Pour, drain | Pour, drain | Drive, swap wheels | Switches | — |

---

## 14. Gameplay scenarios (acceptance walkthroughs)

Each scenario is reproduced as an automated scenario test where possible (§15) and as a manual playtest checklist.

**A. Out of fuel.** Drive until the tank is empty → engine sputters 1 s and stops → fuel gauge at E. Walk back to the home base with the jerry can, fill it at the gas station or use the spare, open the fuel flap and cap, pour → start.

**B. Diesel in a petrol car.** Fill 20 L diesel into a tank holding 10 L petrol (q = 0.33) → misfire, then stall, then won't start. Fix: open the drain plug, catch the fuel in a jerry can (or spill it), refill with petrol until q ≥ 0.85 → starts. Engine lost ~0.001·(1 − q) condition per second while it was running badly.

**C. Forgot the oil.** Drain the oil and drive: oil warning light on, ticking sound, rising temperature from friction heat; after λ < 0.05 for 5 s → bang, seized. Only fix: install another engine (the diesel from wreck #2, which also means switching to diesel fuel).

**D. Radiator smashed in a crash.** A frontal impact at ~30 km/h (severity ~8): bumper absorbs most of it, the coolant tank drops to Worn and leaks; the temperature climbs over the following kilometres; the coolant light comes on; the player tops up with water bottles or swaps in the radiator from a wreck.

**E. Headlights left on overnight.** Park with the low beams on, ignition Off, wait 8 in-game hours (skip time in the debug overlay) → battery flat → solenoid click only. Fix: push the car downhill in D with the key On (roll start), or swap the spare battery from the garage. Driving recharges at up to 20 A acceptance.

**F. Rusty tyre on a long trip.** A tyre at c = 0.1 → random puncture (or a curb hit) → hiss, flap, the car pulls → stop, unbolt the wheel, fit the spare from the trunk, bolt.

**G. Scavenging.** Drive to wreck #1, open its hood, unbolt its battery with the wrench, carry it to the Laika's trunk, close the trunk (stowed), drive home.

**H. Engine swap.** Remove the coolant tank, unbolt and lift out the petrol engine, carry in the diesel engine from wreck #2, bolt it in, reinstall the coolant tank, pour 4 L oil into the new engine, drain the petrol, fill with diesel → starts (diesel start chance).

---

## 15. Testing

### 15.1 Unit tests (Vitest, pure sim)

| Area | Tests |
|---|---|
| `LiquidMix` | add/take proportional; never negative; overflow spills; fraction of empty = 0 |
| Pouring | rate per container type; requires both caps open; carries mix; spill with no target |
| Fuel | q thresholds 0.85 / 0.60; misfire rate; consumption at a fixed power equals the formula; gauge reads volume |
| Oil | λ formula; burn rate per km at c = 1 and c = 0; seizure after 5 s at λ < 0.05 |
| Coolant / heat | steady-state temperature at 90 km/h within 85–95 °C; no-coolant passes 110 °C within 150 s of cruise; boil-off above 110 °C; leak by tank condition |
| Battery | V0 and sag formulas; 11 A for 8 h from a full charge reaches 0; alternator acceptance; warning-light logic |
| Ignition | each position's powered loads |
| Start | each failing precondition blocks the start with a reason code (`no_battery`, `weak_crank`, `no_fuel`, `bad_fuel`, `seized`, `no_engine`, `no_tank`); success probability matches 0.3 + 0.7·c over 10,000 seeded rolls |
| Roll start | catches at > 10 km/h in gear, not in N |
| Auto-clutch | no stall pulling away at idle; lock-up above 1,200 rpm input |
| Gearbox | auto up/downshift thresholds, hysteresis, kickdown, S-hold selects R, manual over-rev refusal |
| Parts | attach/detach updates mass and centre of mass; bolts tracked individually; access rules (engine needs the coolant tank removed); tags enforce compatibility |
| Condition | tier mapping; wear rates per km |
| Damage | severity below 2 → nothing; falloff; bumper shielding; detachment at 0; glass threshold |
| Tyres | pressure factor; puncture triggers; flat radius and grip |
| Radio | signal formula |
| Save | snapshot → restore round-trip of a fully populated car is lossless |

### 15.2 Scenario tests (Vitest, real Rapier, flat test track)

| Scenario | Pass criteria |
|---|---|
| Acceleration | 0–100 km/h in 16–22 s, automatic, full throttle |
| Top speed | 130–150 km/h |
| Braking | 100→0 km/h in 40–50 m on asphalt |
| Fuel economy | 6–9 L/100 km at a steady 90 km/h |
| Warm-up | 85–95 °C after 6 min at 90 km/h and holds for 10 min |
| Handbrake | parked on a 15° slope, moves < 5 cm in 30 s |
| Stability | 60 s of slalom with no NaN, no tunnelling, and no flip at 60 km/h |
| Dead battery | §14-E reproduced: headlights 8 h → cannot crank → roll start succeeds |
| Bad fuel | §14-B reproduced: stalls, then starts after drain and refill |
| No oil | §14-C reproduced: seizes within expected time |

### 15.3 Manual playtest checklist

Every walkthrough in §14, plus: every switch, knob and lamp; every part removed and reinstalled once; the flip interaction; cargo sliding out of an open trunk; mirrors; radio static by distance.

---

## 16. Data schemas (sim)

```ts
interface VehicleState {
  id: string;                          // 'car:laika#1'
  modelId: 'laika';
  slots: Record<SlotPath, string | null>;  // part entity id in each slot
  controls: {
    throttle: number; brake: number; steer: number; handbrake: boolean;
    key: 'off' | 'acc' | 'on' | 'start';
    lights: 'off' | 'low' | 'high'; indicator: 'none' | 'left' | 'right'; hazards: boolean;
    horn: boolean; wipers: 0 | 1 | 2; dome: boolean;
    gearMode: 'auto' | 'manual';
  };
  engine: { state: 'off' | 'cranking' | 'running' | 'stalling' | 'seized'; rpm: number; tempC: number; seizeTimer: number; crankTimer: number };
  gearbox: { selector: 'R' | 'N' | 'D' | 1 | 2 | 3 | 4; gear: number; shiftTimer: number; lastAutoShift: number };
  clutch: { engagement: number };
  wheels: WheelState[];                // ω, steer angle, compression, slip, contact surface
  electrics: { vsys: number; altAmps: number; loadAmps: number };
  dash: { odometerKm: number; tripKm: number; fuelNeedle: number };
  radio: { on: boolean; volume: number; freq: number };
  doors: Record<'fl' | 'fr' | 'rl' | 'rr' | 'hood' | 'trunk' | 'glovebox', { angle: number; angVel: number; latched: boolean }>;
  windows: Record<'fl' | 'fr' | 'rl' | 'rr', number>;   // 0 = closed, 1 = open
  stowed: string[];                    // item ids attached to the chassis
}

interface PartState {
  id: string; typeId: string; tags: string[];
  condition: number; boltsTight: number; bolts: number;
  installedIn: { vehicleId: string; slot: SlotPath } | null;
  liquid?: LiquidContainer;            // engine (oil), coolantTank, fuelTank
  battery?: { charge: number };
  tyre?: { pressure: number };
}
```

All of it is plain JSON and covered by the save round-trip test.

---

## 17. Tunables index

Every number in this document lives in one of:

| File | Contents |
|---|---|
| `sim/vehicle/data/laika.ts` | Chassis, slots, wheel geometry, suspension, brakes, steering, aero, gear ratios |
| `sim/vehicle/data/parts.ts` | Part catalog: masses, bolts, tags, engine curves, battery capacities |
| `sim/vehicle/data/fluids.ts` | Container capacities, flow rates, fuel/oil/coolant thresholds |
| `sim/vehicle/data/electrics.ts` | Loads, alternator, battery model constants |
| `sim/vehicle/data/thermal.ts` | Heat model constants |
| `sim/vehicle/data/damage.ts` | Thresholds, fragilities, puncture chances, wear rates |

The debug overlay exposes all of them live. Tuned values are committed back to these files and this spec is updated in the same PR.
