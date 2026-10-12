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

## Verification

- `production_skoda_drives_switches_recovers_and_rebases` starts the Skoda and switches cars with its engine running.
  It drives, brakes, recovers, shifts the floating origin, and restarts.
- `lada_hood_fits_skoda_by_source_part_types` fits a Lada hood to the Skoda. Both cars use the source `mediumcar` type.
- No browser check covers the Skoda.

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
