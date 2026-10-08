# Desert Drive

A local Rust and WebAssembly driving game with procedural assets.
The current build renders a lit cube on desert ground.

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

## Toolchain

- Official Rust image: `tools/rust-image.txt`.
- Compiler: `rustc 1.99.0 (b940084d7 2026-09-28)`.
- Bevy: `0.19.1`, browser target only, WebGL2, engine multithreading disabled.
- Rapier: `0.36.0`, native and browser targets, parallel features disabled.
- wasm-bindgen crate and CLI: `0.2.129`.

`Cargo.lock` pins the dependency graph. The Dockerfile pins the image by digest.
The server binds to `127.0.0.1:4173` and serves WASM with `application/wasm`.
Keep Steam files outside this folder. This build uses its own geometry and materials.
