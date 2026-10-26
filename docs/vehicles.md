# Vehicles

The world loader spawns cars from car prototypes. The export allows the Lada and the bus pool.

| Car | Source root | Start | Engine |
| --- | --- | --- | --- |
| Lada 2101 | `sharedassets1` 82556 | `StartCar02` (`level1` GO 17687) | Front, petrol |
| Bus01, Bus02, Bus03 (bare and full) | Catalog category `bus` items with `carscript` | World spawn pools | Diesel |

The trailer bus (`bus02UtanfutoFull`, `utanfutoscript`) stays `other_vehicle` until the trailer PR.

## Model

- `car_prototypes` in `tools/reference/world.py` is the allow list. It reads the catalog at export time.
- The list holds the Lada and each `CAR_CATEGORIES` item root with `carscript`. Other car roots stay `other_vehicle`.
- An allowed car root exports as a world prototype, as loot does. Its `vehicle` record holds the `carscript` data.
- The record mass is dry `massScript` mass. Wheel parts, mounted parts and tank fluid (1 kg per liter) add at runtime.
- Engine and radiator records are on their parts. The car uses the engine part that is mounted on it.
- A car can have any wheel count. The radiator is optional.
- Every `meterscript` in the car prefab adds needles, lamps and a backlight. The odometer is optional.
- Diesel is data only. Each engine part has its `fuel_consumption` mix. Fuel suitability compares the tank mix with it.
- Pumps and cans take their fluids from the source tanks. Fluid 5 is diesel.
- `src/world/content_vehicles.rs` binds a car copy: chassis, `VehicleRuntime`, wheel slots, engine and radiator parts.
- `ObjectKey` keys each car copy. `GarageRuntime.vehicles`, `PhysicsWorld` and wheel slots use this key.
- One Rapier world holds every chassis. Every engine steps each frame. Only the seated driver's car reads keys.
- `GarageRuntime.vehicle` is the last seated car. The camera, HUD, and recovery follow it.
- Parts fit only slots with a matching source part type. The source rules decide cross-car fit.

## Placement and save

- The `world.startup` entry `StartCar02` has a prototype. The stream spawns it as a fixed placement on the `Startup` layer.
- The world generator can place any number of copies at any pose. Each copy has its own key.
- On retire, the car, engine, radiator and wheel slot states go to the world change records.
- On return, the copy respawns and takes its states back. A moved car returns at its last pose.
- The save keeps these records per key: pose, gear, ignition, handbrake, trip, headlights, engine state and fuel.
- Save schema 20. Older saves are rejected.

## Physics fix

Rapier 0.36 CCD sweeps disabled colliders. The Skoda has disabled source colliders below the ground.
CCD then clamped the chassis each step. The car moved at a fraction of its speed. The Lada lost half its motion.
Disabled colliders now have empty solver groups. CCD ignores them.

## Driving controls

The driving branch uses source bindings for both cars. Gear keys act with Automatic gears disabled.

| Action | Keys |
| --- | --- |
| Clutch | Left Ctrl, Right Ctrl, Numpad 0 |
| Shift up / down | Up arrow / Down arrow |
| Reverse | R, Numpad + |
| Neutral | 0, Numpad -, N |
| Forward gear | 1–6, Numpad 1–6, within the car's gear range |

Without Automatic clutch, depress the clutch past halfway before a shift.
Automatic clutch permits manual shifts. Its RPM clutch ramp runs inside automatic shifting.
Automatic gears requires Automatic clutch. In that mode, S requests reverse after stopping; W requests forward travel.
Both cars use neutral and both assists at New game startup.
At exact zero driven-wheel RPM, the endpoint-clamped port can retain neutral.
Source CalcTorque evaluates its curve without that wrapper. Native curve extrapolation remains unmeasured.
The serialized infinity value does not establish the public WrapMode mapping. Source boundary parity remains conditional.
Normal browser launch in both cars remains unverified.

The save stores each car's selected gear and engine state, plus both shared assist choices.
Load restores those values. Pause, capture loss, seat changes, and recovery clear input while keeping selected gears.
Seat entry suppresses pending shift edges. Seat exit clears ignition holds without a release toggle.
Automatic rules require a mounted engine; manual selection retains its source conditions.
Suspension validation accepts target `[0,1]` and nonnegative spring and damping values.
The gear display uses R, N, or a forward number. Browser acceptance of the seated display remains open.

## Wheel adapter limits

The drivetrain applies clutch engagement in the engine owner and each gear/final-drive ratio once.
The physics owner distributes axle torque among active driven wheels, then converts by installed wheel radius.
It supplies engine, service, and handbrake contributions to eligible wheels.
The stock Rapier controller excludes simultaneous motor force and brake impulse on one wheel.
Input sums therefore do not prove source combined-brake response.

Source tire stiffness and Rapier friction describe different models.
Tire/rim differences, empty hubs, suspension settling, and wheel spin require native measurements before further adapter work.
Task 4 lacks qualifying source observations. Task 5 remains unscheduled.

## Verification

The final fix on `codex/correct-driving-physics` follows review base `6f8118162ece6d9ee24c811719c0935af433487a`.
The controller owns its scoped re-review. Earlier full-suite results describe `05bbf7b`.
Native critical checks cover shift rules, clutch attenuation, signed RPM/stall, both-car seat transitions, and saved gears/assists.
The full native suite retains 108 known failures. These checks do not establish source handling parity.

The controller verified normal menu actions, paused save, full reload, restored choices, and visible New game assist defaults.
IAB refused pointer capture. Both-car driving, normal launch, recovery, restart, and handling-rate acceptance remain open.
The older `production_skoda_drives_switches_recovers_and_rebases` test describes a controlled Rust route with exported assets.
The current run passes Skoda movement checks, then fails the inactive Lada trip bound.
Parked drift and preparation sleep checks also fail. These ignored cases lack a complete prior baseline.
See [current verification](verification.md#driving-branch-verification-2026-10-14) for command results and retained evidence.

## Add the next car

1. Add the car's catalog category to `CAR_CATEGORIES` in `tools/reference/world.py`. Its roots then export with a `vehicle` record.
2. For a start car, add its startup placement next to `StartCar02` in `recover_world`. Check the pose against home colliders and the terrain.
3. Run the runtime export. The exporter stops when a source record is missing. Car `mountStuff` volumes export by themselves.
4. Check the prototype node count against the stream limits.
5. Run the export-based tests with the new manifest.

Expected work for other cars:

- Car03: the engine needs two-stroke mixed fuel. It has no radiator and no oil tank.
- Car06P: diesel works through its engine `fuel_consumption`. Add its category.
- Buses: door scripts (`busdoorscripts`) and the trailer come in later PRs.
- Car07: add the automatic gearbox modes from the source.
- Bikes: add balance physics. The four-wheel controller does not fit.
