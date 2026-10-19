# Opaque shadow material batching

Roadmap item 7 groups compatible opaque shadow casters across primary surface materials.
The branch is `codex/opaque-shadow-material-batching`. It remains unmerged.
The baseline is `8eca150d90e7d0fa216823276c01825f8b1189d1`.
Steam build `11140890` remains the behavior reference.

## Eligibility and ownership

Only prepared `SourceMaterial` casters can share a material key.
They must use the standard opaque path, ordinary depth and blend states, and a LOD fade of exactly 1.
Alpha-test, premultiplied-alpha, and alpha-blend flags exclude sharing.
Other material types retain their material keys. This includes grass, impostors, terrain, and specular materials.

The checked `source_caster.wgsl` fragment returns before alpha sampling for these opaque casters.
Their surface color, textures, and UV values do not change shadow coverage.
The vertex position and normal bias use per-instance mesh data and the cascade uniform.

Material extraction retains the eligibility state with its existing revision.
Queueing shares a key only when the prepared caster material has that same revision.
Missing, changed, and cancelled preparation retains the material key.
The next queue pass moves affected entities between retained bins through the existing key comparison.

Each bin still requires identical pipeline, draw function, mesh, indexed state, and exact cascade mask.
Bevy supplies each entity's live transform through its existing instance buffer.
The draw command binds its representative entity's prepared material.
All shared members have equivalent opaque caster behavior with that binding.

The change adds no steady-frame allocations or renderer setup.
It does not alter caster selection, offscreen admission, shaders, camera depth, or surface materials.

## Checks

The baseline passed `shadow_cache` and `source_shadow_preparation`: three tests, zero failures.
Candidate native tests passed: six tests, zero failures.
The locked, offline WASM compile check passed.
Formatting and staged whitespace checks passed.
The native check reported one existing unused-method warning.
The WASM check reported five warnings in unchanged `SourceReceiver` code.
A temporary copy with the old material-specific behavior failed all three new tests, as expected.

Commands ran through the project Docker toolchain:

```text
cargo fmt --all -- --check
cargo test --locked --offline --test opaque_shadow_batching --test shadow_cache --test source_shadow_preparation
cargo check --locked --offline --target wasm32-unknown-unknown --lib
```

The new native tests exercise the production eligibility function and batch key.
They check alpha and fade exclusions, preparation revisions, geometry and pipeline separation, and all 15 nonzero cascade masks.
Their fixtures compare exact entity and transform membership across all four cascades.
These fixtures do not execute Bevy draws or a browser.

## Runtime acceptance

Full checks were not requested.
Release builds, browser acceptance, and matched A/B/A measurements were deferred in the approved design.
No draw-count, FPS, or frame-time gain is verified.
Existing atlas counters still compare submitted instance counts with required counts for each cascade.

Remaining checks include garage and driving, each cascade, offscreen casters, material changes, and LOD transitions.
Also check mirrors, streaming, origin changes, real input, active physics, submitted frames, and Save/Load.
Record draw counts separately from uninstrumented FPS and average, p95, and p99 frame times.
