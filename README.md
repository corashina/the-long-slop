# My Summer Slop

Prepare a fixed Lada 2101 in the starting garage. Drive onto the desert road.
The home, car, and supplies come from installed Steam build 11140890.
Rust, Bevy, and Rapier run the browser game through WebAssembly.

## Export, build, and run

Start Docker Desktop. Install Node.js and Python 3.12 or later.
Keep the selected Steam build installed. Export its assets once:

```powershell
python -m venv tools/reference/.venv
./tools/reference/.venv/Scripts/python.exe -m pip install -r tools/reference/requirements.txt
./tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'
./scripts/build.ps1
node scripts/serve.mjs
```

Open [the local game](http://127.0.0.1:4173). Wait for the scene. Select Start to capture the mouse.
Press Ctrl+C in the server terminal to stop it.
The exporter checks the Steam build and leaves Steam files unchanged.
Git excludes recovered assets, source analysis, and `web/pkg`.
A missing or invalid reference export stops loading.

Run formatting, native tests, and the release WASM check:

```powershell
./scripts/check.ps1
```

Docker caches use `tld-wasm-cargo` and `tld-wasm-target`.
The build script writes the browser package to `web/pkg`.

## Controls

| Control | On foot | In the driver seat |
| --- | --- | --- |
| WASD | Move | Accelerate, brake, reverse, steer |
| Mouse | Look | Look |
| Shift / Ctrl | Run / crouch | — |
| Space | Jump | Toggle handbrake |
| E | Use cap or seat | Leave seat |
| Hold E + mouse | Move an aimed door | Move an aimed door |
| F | Carry, drop, remove, or fit an aimed wheel or car part | — |
| Mouse wheel / hold RMB + mouse / hold MMB + mouse | Change carry distance / move / rotate a held item | — |
| Hold LMB | Fill an open container | — |
| I | — | Tap ignition; hold starter |
| L | — | Switch headlights: off, low beam, high beam |
| C | — | Switch first person / chase view |
| Backspace | Hide or show feedback | Hide or show feedback |
| Esc | Pause menu | Pause menu |

Carry a fuel can. Open the fuel cap. Aim at the opening and hold LMB to fill it.
Open carried containers before filling them at roadside pumps.
Open containers spill when tilted. Pumps retain unlimited gasoline stock.
The Lada starts with 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
Its capacities are 39 L fuel, 5 L oil, and 5 L coolant.

Use Recover in the pause menu. Use Controls for bindings and Fullscreen.
Aim at a tire and press F to dismount it. Release F to mount a held tire or rim.
Focus loss pauses play and clears input. Continue captures the mouse again.
Recovery preserves fluids, parts, condition, and trip distance.
A hard crash can knock wheels and parts off. Worn parts and a worn car body break first.
Carry a loose part to its own slot and press F to fit it.
The car does not run without its engine. Without its radiator it gets no cooling.
An empty fuel tank moves recovery to a station. Other recovery moves the car to the nearest road point.
Restart restores the garage, supplies, car, origin, and trip distance.
Close the driver door before reversing through the garage exit.

## Scope and evidence

Read [the approved scope](docs/superpowers/specs/2026-10-09-garage-start-design.md) and [verification](docs/verification.md).
Imported layout and recovered rules do not establish identical Unity physics or full-game parity.
Unity shader effects, live mirror reflections, and audio are not reproduced.
Terrain uses the source sand texture only. Biome blending and distant terrain are not reproduced.
The final browser walking route was skipped at the user's request. It remains unverified.
The build has no survival system, save system, multiplayer, or public deployment.

## Toolchain

`Cargo.lock` pins Rust dependencies. The Dockerfile pins the Rust image by digest.
Bevy 0.19.1 uses WebGL2. Rapier 0.36.0 handles native and browser physics.
wasm-bindgen crate and CLI use 0.2.129.
The server binds to `127.0.0.1:4173` and serves WASM with `application/wasm`.
The menus use self-hosted [Liberation Sans 2.1.5](https://github.com/liberationfonts/liberation-fonts/releases/tag/2.1.5).
Its license and provenance are in web/fonts/. Gameplay text uses Arial.
Source feedback replaces the persistent vehicle status card.
The Lada shows speed, fuel, temperature, warning lights, and distance on its imported instruments.
