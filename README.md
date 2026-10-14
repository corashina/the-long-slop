# My Summer Slop

Prepare a fixed Lada 2101 in the starting garage. Drive onto the desert road.
The home, car, and supplies come from installed Steam build 11140890.
Rust, Bevy, and Rapier run the browser game through WebAssembly.

## Export, build, and run

Start Docker Desktop. Install Node.js and Python 3.12 or later.
Keep Steam build 11140890 installed.
Prepare [host PowerShell and pinned ILSpy](tools/reference/README.md#host-managed-analysis-tools) before export.
Build the toolchain image, then export the complete library and selected runtime:

```powershell
python -m venv tools/reference/.venv
./tools/reference/.venv/Scripts/python.exe -m pip install -r tools/reference/requirements.txt
docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .
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

## Development asset library

The export preserves 183 source files, 262,784 objects, and 721,957 dependency links.
The library contains 5,121 converted assets, 257,587 native records, and 76 source-empty records.
Its 8,570 published files total 3,105,818,196 bytes.
Typed records and managed analysis use a separate ignored cache.
The asset branch selected 89 runtime files, totaling 73,261,688 bytes, before integration with source UI and interaction work.
Startup requests contain only selected runtime assets.

Use [export stages and verification commands](tools/reference/README.md) to access the library.
Read [coverage and support limits](docs/research/2026-10-10-full-asset-audit.md) before using an asset.
The library includes embedded Liberation Sans. The current shell uses self-hosted Liberation Sans and Arial.
Development availability does not add vehicles, audio, animation playback, or other gameplay.

## Controls

| Control | On foot | In the driver seat |
| --- | --- | --- |
| WASD | Move | Accelerate, brake, reverse, steer |
| Mouse | Look | Look |
| Shift / Ctrl | Run / crouch | — |
| Space | Jump | Toggle handbrake |
| E | Use cap, switch, or seat | Use control or leave seat |
| Hold E + mouse | Move an aimed mechanism | Move an aimed mechanism |
| F | Carry or remove a part; release to install; tap to drop | - |
| Hold F, then release | Charge and throw a held item | - |
| Hold F at the car body | Push while held | - |
| F at the steering grip | Latch grip; WASD pushes and steers; F releases | - |
| Mouse wheel / Tab | Change carry distance / recenter | - |
| Hold MMB / RMB + mouse | Rotate / move held item | - |
| Hold MMB and RMB + mouse | Roll held item | - |
| X | Split a wheel | - |
| Hold LMB | Fill an open container | Use an aimed switch |
| I | — | Tap ignition; hold starter |
| L | — | Cycle headlights: off, low beam, high beam |
| C | — | Switch first person / chase view |
| Backspace | Hide or show feedback | Hide or show feedback |
| Esc | Pause menu | Pause menu |

Remove the hubcap before removing its wheel. Removed parts enter your hands.
Move a held part close to a compatible empty slot. Tap F when the install prompt appears.
Aim at an empty wheel slot to install a wheel.
Some source items, including the fuel can, cannot rotate.

Carry a fuel can. Open the fuel cap. Aim at the opening and hold LMB to fill it.
Open carried containers before filling them at a supported supply.
Open containers spill when tilted. Supplies retain their checked fluid types and current stock.
The Lada starts with 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
Its capacities are 39 L fuel, 5 L oil, and 5 L coolant.

Use Recover in the pause menu. Use Controls for bindings and Fullscreen.
Aim at a tire and press F to dismount it. Release F to mount a held tire or rim.
Focus loss pauses play and clears input. Continue captures the mouse again.
Recovery preserves fluids, parts, condition, trip distance, cycle time, and lamp state.
An empty fuel tank requests the nearest supported physical supply. Water supplies do not provide gasoline.
Recovery waits for destination geometry, materials, collision, and clearance before moving the car.
Restart restores the garage, supplies, car, origin, trip distance, source cycle time, and lamp state.
Close the driver door before reversing through the garage exit.
Keyboard steering reaches full lock in 0.8 seconds. Release the key to return to center.

## Scope and evidence

Read the [approved interaction scope](docs/superpowers/specs/2026-10-10-interaction-parity-design.md), [garage scope](docs/superpowers/specs/2026-10-09-garage-start-design.md), and [verification](docs/verification.md).
Imported layout and recovered rules do not establish identical Unity physics or full-game parity.
Lighting follows the approved [day cycle scope](docs/superpowers/specs/2026-10-10-day-night-lighting-design.md).
The source cycle controls sun, moon, clouds, stars, fog, headlights, cabin lights, house lamps, and roadside lamps.
Pause freezes cycle time, lamp fades, and switch movement.
Restart restores the source startup offset of 240 seconds.
Selected source material behavior is reconstructed. Exact shadows, probe publication, postprocessing, and stained-glass capture remain deferred.
Live mirror reflections and audio remain outside this stage.
See [completed work and roadmap](docs/roadmap.md) and [the deferred rendering plan](docs/superpowers/plans/2026-10-11-rendering-completion.md).
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

## Source world

The current world uses the checked 5,000-metre saved settings profile and seed 42.
The source road ends after 50,000 segments. Off-road source grids have no finite extent.
Road, terrain, structures, props, and pole geometry use one source generation path.
The runtime retains changed objects for this session. Restart clears those changes.
Nested content also retains bounded first-call source history. This history can fill before all road segments are visited.

World admission keeps accepted support when a resource limit prevents new publication.
Native and WASM compilation do not establish source, driving, rebase, recovery, or revisit parity.
Read the current [world evidence and memory limits](docs/verification.md#source-world-implementation-2026-10-13).

Development tools can request `await window.measureWorld()` in the browser console.
This returns ownership counters, unique asset handles, timing, and available browser memory values.
The snapshot adds no player interface. Process and driver memory remain unavailable.
