# Per-object local light selection

Roadmap item 9. Source build 11140890. Branch: `codex/object-local-lights`.
Base: `8eca150d90e7d0fa216823276c01825f8b1189d1`.
Merged into master on 2026-10-19. Source commit: `f7b7e53`.
Runtime acceptance and performance measurements remain open.

## Selection

The additive vertex pass builds a 64-bit mask from full renderer bounds.
Selection runs once per vertex. The flat mask is constant across each object's triangles.
Base passes do not run selection. No CPU selection, per-object buffer, material copy, or per-frame allocation was added.

The exporter reads mesh local bounds for static renderers.
See [recover_renderer](../../tools/reference/render_lighting.py).
The existing renderer attributes carry those bounds into [source_mesh.wgsl](../../web/assets/shaders/source_mesh.wgsl).

The selector transforms the full local box into a world box.
Absolute matrix columns cover rotation, reflection, scale, and shear.
The nearest point on that box bounds the distance from every fragment to each lamp.
A lamp is excluded only when that distance exceeds its range.
The test expands the box for floating-point error. Boundary contact retains the lamp.
Non-finite arithmetic retains the lamp. Skinned and morphed objects retain every otherwise eligible local lamp.

The mask also applies the existing light-kind, receiver-mask, and positive-color checks.
The fragment pass visits selected slots in ascending order.
The existing per-fragment range test, attenuation texture, cookie samples, and BRDF remain intact.
Sunlight keeps its existing path. Spotlight cones do not add another exclusion test.

Selection uses each draw's current object transform and current lamp uniform.
There is no retained selection state to invalidate during movement, streaming, origin shifts, or Load.

## Checks

Run the native shader check and WASM compile check:

```powershell
./scripts/check.ps1 -Test local_light_selection
```

The native check compiles the actual selector and fragment iteration with Naga 29.0.4.
It emits both stages for GLSL ES 3.00.
It also checks the 16 KiB uniform-block limit and excludes vertex texture bindings.
Naga was already in the renderer dependency graph. This change adds a native test dependency only.

Results on 2026-10-19, repeated on merged master before push:

- Rust format check passed.
- Targeted native shader test passed, 1 test with no failures.
- WASM library compile check passed.
- Native compilation reported the existing unused `startup_job_audit` method.
- WASM compilation reported four existing unused-variable warnings in `SourceReceiver`.

These checks do not execute the shader or prove identical pixels.

## Deferred acceptance

Release builds, full checks, and browser acceptance require an explicit request under AGENTS.md.
No release build or browser acceptance was run for this change.

Compare matched release A/B/A runs in exclusive benchmark windows.
Measure garage and driving average, p95, and p99 frame times.
Keep instrumented counts separate from timing.
Compare image output with moving lamps and moving objects.
Then check streaming, origin shifts, and repeated Save/Load.
Check real input, active physics, and submitted frames.

CPU selection cost is zero because selection runs in the vertex shader.
Vertex cost can exceed fragment savings on dense meshes or scenes with many overlapping lamps.
Net GPU cost, image equivalence, and a performance gain remain unverified.
