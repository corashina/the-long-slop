# Verification

Reference: installed Steam build 11140890, Unity 2021.3.18f1. Vehicle: fixed Lada 2101.
This file records the current checks and open limits. Git history keeps the dated record of each round.

## Checks

Last run: 2026-10-11, Europe/Warsaw.

| Check | Command | Result |
| --- | --- | --- |
| Rust format and lint | `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | Clean |
| Native suite | `cargo test --locked` | 111 passed; 9 installed-source gates skipped by default |
| Installed-source gates | `cargo test --locked -- --ignored` with the local export | Not run in this round |
| Browser modules | `node --test tests/web/*.test.mjs` | 14 passed |
| Exporter suite | `python -m unittest discover -s tools/reference/tests` | 8 passed; 3 need the installed build |
| Release WASM | `cargo build --locked --release --target wasm32-unknown-unknown --lib` | Passed |

`scripts/check.ps1` runs the Rust checks and browser modules in Docker. CI runs every row except the installed-source gates.
Source gates and the exporter need the installed build. They stay local.

## Evidence by feature

**Reference recovery.** The exporter follows the scene links to the home and Car02Full.
It converts handedness once and keeps nested scale and reflected wheel transforms.
The startup audit confirms 0.5 L gasoline, 2.5 L oil, and 3.6 L water. The car has 955 kg dry mass.

**Garage to road.** The source flow opens both garage leaves, enters the driver seat, and closes the door.
The car reverses onto the road lane in 5.65 seconds. All four wheels land within four more steps.
The parked car stays still for 90 seconds with all four wheels supported.

**Source UI.** Menus, action icons, prompts, and fluid text use the recovered sprites and layouts.
F press picks up, drops, or removes. F release mounts or fits. RMB moves a held item. MMB rotates it.
The speed, temperature, and fuel needles and the odometer use the source ranges and smoothing.

**World.** The `System.Random` and Perlin ports match .NET and Unity values.
The serialized seed is -1705217151. The world sits 578.897 meters lower so the home stays fixed.

**Part condition.** Each part keeps the source state from 0, new, to 4, rusty.
A crash is a forward speed change of 15 to 40 km/h in one step. A rusty body scales both by 0.2.
Twenty `partslotscript` slots come off. A part fits only into its own slot.

**Day cycle and headlights.** A cycle is 20 minutes of day and 5 of night. The game starts 4 minutes in.
L cycles off, low beam, and high beam. Each headlight fades at the source rate.
Unity light units have no exact Bevy match. These approximations apply:

- One Unity intensity unit is 8000 lux of sun light.
- One Unity ambient unit is 400 nits.
- A spot light matches Unity falloff at one fifth of its range. Near surfaces are brighter.
- Lens emission is the source emission color times 8000 / π nits.

**Sound.** Rust follows the source sound scripts. The browser plays the original clips through Web Audio.

## Not verified

- The continuous browser route from garage to road. The user waived it on 2026-10-10.
- A crash, a part fit, and sound in a live browser session. Native and source checks cover them.
- Live gameplay with pointer lock. The embedded browser rejects it.

## Limits

Rapier raycast suspension replaces Unity WheelCollider physics. Identical handling is not claimed.
The exporter keeps one LOD and does not reproduce Unity shader effects or live mirror reflections.
Browser text uses Liberation Sans and Arial, not Unity's SDF renderer.
The prompts do not show part condition or the headlight key.
Not ported: distant terrain, the `TerrainBiome` blend shader, building holes, power lines, grass,
street lamps, the interior light, rear lamps, stars, clouds, and the radio, horn, door squeak, pour sound, and Doppler shift.

## Performance

On 2026-10-10 at 1280 by 720 on an RTX 4070 Ti, garage frames averaged 7.82 ms. The 95th percentile was 8.2 ms.
The release WASM was 50.5 MB. The scene was ready about 3 seconds after reload over localhost.

## Research sources

The [Starting House](https://the-long-drive.fandom.com/wiki/Starting_House),
[Getting Started](https://the-long-drive.fandom.com/wiki/Getting_Started), and
[Lada 2101](https://the-long-drive.fandom.com/wiki/Lada_2101) wiki pages gave secondary context.
Installed-build evidence controls conflicting values. See `docs/research/2026-10-09-garage-reference.md`.
