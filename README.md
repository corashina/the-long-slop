# My Summer Slop

Prepare a fixed Lada 2101 in the starting garage. Drive onto the desert road.
A source Skoda waits in the yard north of the garage. Read [vehicles](docs/vehicles.md).
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

Open [the local game](http://127.0.0.1:4173). Wait for the scene. Select New game, then Continue to capture the mouse.
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
It also prepares Brotli downloads in `.cache/http-compression`. Keep that directory with the server deployment.
After replacing an export without rebuilding WASM, prepare its downloads and restart the server:

```powershell
node scripts/prepare-http-compression.mjs
```

The server negotiates Brotli for WASM and current package JSON. It keeps images and package receipts unchanged.
Run `node scripts/measure-startup-downloads.mjs` to check transfer bytes, decoded hashes, and conditional cache requests.
This check measures HTTP transfers. It does not measure playable startup.

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
Development availability does not add further vehicles, audio, animation playback, or other gameplay.

## Controls

| Control | On foot | In the driver seat |
| --- | --- | --- |
| WASD | Move | Accelerate, brake, steer |
| Mouse | Look | Look |
| Shift | Run | — |
| Space | Jump | Toggle handbrake |
| E | Use cap, switch, or seat | Use control or leave seat |
| Hold LMB + mouse | Move an aimed mechanism | Move an aimed mechanism |
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
| Left Ctrl / Right Ctrl / Numpad 0 | Crouch with Ctrl | Clutch |
| Up arrow / Down arrow | — | Shift up / down |
| R / Numpad + | — | Select reverse |
| 0 / Numpad - / N | — | Select neutral |
| 1–6 / Numpad 1–6 | — | Select a forward gear, when available |
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
The Skoda starts with the same fluids. Its capacities are 32 L fuel, 5 L oil, and 5 L coolant.

Controls contains Automatic clutch and Automatic gears. Automatic gears requires Automatic clutch.
With Automatic gears, S requests reverse after the car stops. W requests forward travel.
Disable Automatic gears to select gears. Without Automatic clutch, press the clutch past halfway before a shift.
The gear display shows R, N, or the forward gear number while seated.
Save stores both cars' gears and engine state, plus the two assist choices. Load restores them.
New game restores neutral and enables both assists. Recovery and input clearing keep the selected gears.

Use Recover in the pause menu. Use Controls for bindings and Fullscreen.
Aim at a tire and press F to dismount it. Release F to mount a held tire or rim.
Focus loss pauses play and clears input. Continue captures the mouse again.
Recovery preserves fluids, parts, condition, trip distance, cycle time, and lamp state.
An empty fuel tank requests the nearest supported physical supply. Water supplies do not provide gasoline.
Recovery waits for destination geometry, materials, collision, and clearance before moving the car.
Restart restores the garage, supplies, car, origin, trip distance, source cycle time, and lamp state.
