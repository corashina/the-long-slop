# Vehicles

The game has two source cars. Both use the same code path.

| Car | Source root | Start | Engine |
| --- | --- | --- | --- |
| Lada 2101 | `level1` GO 17687, `StartCar02` | Garage spot | Front, petrol |
| Skoda (Car01) | `level1` GO 6890, `StartCar01` | Yard north of the garage | Rear, petrol |

## Selection

Car01 has complete start-car records and the same part set as the Lada.
It has four wheels, rear drive, a petrol engine, a radiator, and fuel, oil, and coolant tanks.
Its differences come from the source: torque, rpm, gears, mass, engine position, lights, and gauges.

Rejected cars:

- Car02k is a Lada wagon. It adds no mechanical difference.
- Car03 and Car03k use a two-stroke mixed-fuel engine without a radiator or an oil tank.
- Car06P, Bus01, and Bus02 use diesel.
- Car07 has an automatic gearbox and is not a start car.
- Bike01 and Bike03 need balance physics.

## Project change

In the source, all start cars share one garage spot. The game can show one start car.
The exporter moves the Skoda to Unity position (38.5, z 58.5) and turns it 180 degrees.
This spot is clear of home colliders and faces the road.

## Model

- `ReferenceManifest.vehicles` holds one record per car. Each record holds its engine and instruments.
- Seats, light controllers, high-beam lamps, part slots, and tanks name their car by index.
- One Rapier world holds every chassis. `PhysicsWorld` methods take the car index.
- `GarageRuntime.vehicles` holds engine, handbrake, gear, ignition, and trip per car.
- `GarageRuntime.vehicle` is the last seated car. The camera, HUD, and recovery follow it.
- Every engine steps each frame. Only the seated driver's car reads keys.
- Parts fit only slots with a matching source part type. The source rules decide cross-car fit.

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

1. Add the car to `CARS` in `tools/reference/export.py`: root GameObject, start-car name, and starter link.
2. Give it a parking spot with `relocate`. Check the spot against home colliders and the terrain.
3. Run the runtime export. The exporter stops when a source record is missing.
4. Run the production tests with the new manifest.

Expected work for each rejected car:

- Car03: the engine needs two-stroke mixed fuel. The manifest requires a radiator and an oil tank for each car. Make both optional.
- Diesel cars: the engine record keeps source fuel types. Check the station pumps and the start fluids.
- Car07: add the automatic gearbox modes from the source.
- Bikes: add balance physics. The four-wheel controller does not fit.
