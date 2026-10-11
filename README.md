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
| WASD / arrows | Move | Accelerate, brake, reverse, steer |
| Mouse | Look | Look |
| Shift / Ctrl | Run / crouch | — |
| Space | Jump | Toggle handbrake |
| E | Use cap or seat | Leave seat |
| Hold E + mouse | Move an aimed door | Move an aimed door |
| F | Carry, drop, remove, or install an aimed wheel | — |
| Mouse wheel / hold RMB + mouse | Change carry distance / rotate item | — |
| X | Split a wheel or join compatible rim and tire | — |
| Hold LMB | Fill an open container | — |
| I | — | Tap ignition; hold starter |
| C | — | Switch first person / chase view |
| R | Recover car | Recover car |
| P / Esc | Pause | Pause |

Carry a fuel can. Open the fuel cap. Aim at the opening and hold LMB to fill it.
Open carried containers before filling them at roadside pumps.
Open containers spill when tilted. Pumps retain unlimited gasoline stock.
The Lada starts with 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
Its capacities are 39 L fuel, 5 L oil, and 5 L coolant.

Focus loss pauses play and clears input. Resume captures the mouse again.
Recovery preserves fluids, parts, condition, and trip distance.
An empty fuel tank moves recovery to a station.
Restart restores the garage, supplies, car, origin, and trip distance.
Close the driver door before reversing through the garage exit.

## Scope and evidence

Read [the approved scope](docs/superpowers/specs/2026-10-09-garage-start-design.md) and [verification](docs/verification.md).
Imported layout and recovered rules do not establish identical Unity physics or full-game parity.
Selected source material behavior is reconstructed. Full source lighting, stained-glass capture, live mirrors, and audio remain incomplete.
See [completed work and roadmap](docs/roadmap.md) and [the deferred rendering plan](docs/superpowers/plans/2026-10-11-rendering-completion.md).
The final browser walking route was skipped at the user's request. It remains unverified.
The build has no survival system, save system, multiplayer, or public deployment.

## Toolchain

`Cargo.lock` pins Rust dependencies. The Dockerfile pins the Rust image by digest.
Bevy 0.19.1 uses WebGL2. Rapier 0.36.0 handles native and browser physics.
wasm-bindgen crate and CLI use 0.2.129.
The server binds to `127.0.0.1:4173` and serves WASM with `application/wasm`.
The headings use self-hosted [Libre Baskerville](https://github.com/google/fonts/tree/main/ofl/librebaskerville).
Its license is in `web/fonts/OFL.txt`.
