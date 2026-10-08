# Desert Drive

A local Rust and WebAssembly driving game with procedural assets.
Drive a weathered compact car along an endless desert road.
Switch between chase and cockpit views. Stop at roadside pumps to refuel.
This build recreates a selected driving loop from The Long Drive with original code and procedural assets.

## Build and run

Install Docker Desktop and Node.js. Start Docker Desktop.
Run these commands from this folder in PowerShell:

```powershell
./scripts/build.ps1
node scripts/serve.mjs
```

Open [the local game](http://127.0.0.1:4173). Wait for the scene, then select Start.
Press Ctrl+C in the server terminal to stop the server.

Run formatting, native tests, and the browser compile check:

```powershell
./scripts/check.ps1
```

Docker stores Cargo and build caches in `tld-wasm-cargo` and `tld-wasm-target`.
The build script writes the browser package to `web/pkg`.
Git excludes the package. Build it before starting the server.

## Controls

| Key | Action |
| --- | --- |
| W / Up | Accelerate |
| S / Down | Brake, then reverse |
| A / D or Left / Right | Steer |
| Space | Handbrake |
| C | Switch camera |
| Hold E | Refuel below 1 km/h, within 6 meters of a station point |
| R | Recover the car |
| P / Esc | Pause or resume |

Select Pause for Resume and Restart. Select Fullscreen to enter or leave fullscreen.
Focus loss pauses the game and clears held keys. Select Resume to continue.
Recovery preserves fuel and trip distance. With an empty tank, recovery places the car at a station.
Restart restores 12 liters of fuel and clears the trip. Reload starts a new drive.

The tank holds 40 liters. The engine consumes fuel during active driving, including idle.
At zero fuel, you can coast, brake, and steer. Recover to reach a pump, then hold E to refuel.

## Verification

Read [the verification record](docs/verification.md) for native tests, browser observations, and incomplete checks.
The first build supports desktop keyboards and WebGL2. It has no save system or sound.
Walking, inventory, multiplayer, and original game asset import remain outside this build.

## Toolchain

- Official Rust image: `tools/rust-image.txt`.
- Compiler: `rustc 1.99.0 (b940084d7 2026-09-28)`.
- Bevy: `0.19.1`, browser target only, WebGL2, engine multithreading disabled.
- Rapier: `0.36.0`, native and browser targets, parallel features disabled.
- wasm-bindgen crate and CLI: `0.2.129`.

`Cargo.lock` pins the dependency graph. The Dockerfile pins the image by digest.
The server binds to `127.0.0.1:4173` and serves WASM with `application/wasm`.
Keep Steam files outside this folder. This build uses its own geometry and materials.
The headings use self-hosted Libre Baskerville from [Google Fonts](https://github.com/google/fonts/tree/main/ofl/librebaskerville).
Read its SIL Open Font License in `web/fonts/OFL.txt`.
