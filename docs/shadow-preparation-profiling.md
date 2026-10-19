# Shadow preparation profiling

`scripts/profile-shadow-preparation.mjs` measures a frozen release package in installed Chrome.
The runner requires Node.js and Playwright. `NODE_PATH` identifies the directory containing Playwright.
Each run uses one browser, a 1280 × 720 viewport, and device pixel ratio 1.
Software GPU samples are invalid.
Receipts include package hashes, the manifest hash, input state, physics progress, and submitted frames.
Failed startup also produces a receipt and a screenshot.

```powershell
node scripts/profile-shadow-preparation.mjs --assets PATH_TO_ASSETS --package PATH_TO_PACKAGE --variant baseline
node scripts/profile-shadow-preparation.mjs --assets PATH_TO_ASSETS --package PATH_TO_PACKAGE --variant candidate
node scripts/profile-shadow-preparation.mjs --assets PATH_TO_ASSETS --package PATH_TO_PACKAGE --variant baseline-recheck
```

`--profile` enables CPU measurements. These samples are separate from FPS samples.
`--out` selects the receipt directory. Its default is `.cache/shadow-preparation/reports`.
Each receipt filename is unique. An existing receipt causes an error.
`--seconds` selects the sample duration. Its default is 10 seconds.
`--timeout` selects the startup timeout in milliseconds. Its default is 180000.

`--scenario driving --save PATH_TO_SAVE` loads a frozen save and holds W during measurement.
The save must contain a seated player and a vehicle that can move with W.
The receipt contains the save hash. A stationary vehicle makes the driving sample invalid.
Builds, other game tabs, and other GPU measurements must be idle during FPS samples.

## Measurements

`shadow_preparation_profile(true)` clears and enables shadow measurements.
`shadow_preparation_audit()` returns their JSON representation.
`shadow_preparation_profile(false)` clears and disables measurements.
Disabled scopes do not read the clock or allocate memory.

| Stage | Measured work | Cache hits |
|---|---|---|
| `scene` | Main-world caster updates, views, and visibility | None |
| `view` | Per-view selection, fit, and split preparation | Complete view reuse |
| `selection` | Caster selection and projection bounds | None |
| `fit` | Cascade fitting | None |
| `splits` | Cascade split membership | None |
| `queue` | Shadow phase validation and updates | Unchanged bin entries |
| `cascades` | Cascade projection and uniform preparation | Unchanged view inputs |
| `collector` | Collector projection and uniform preparation | Unchanged view inputs and targets |
| `upload` | Uniform comparison and buffer writes | Equal uniform contents |

Each stage reports calls, total milliseconds, maximum milliseconds, cache hits, items, and uploaded bytes.
Timings overlap. For example, `scene` includes `view`. Their durations cannot be added.
`selection` counts examined casters. `splits` counts selected casters. `queue` counts eligible queue entries.
`upload.bytes` counts buffer writes after comparison, including initial allocation.

## Change tracking

Caster and view revisions retain their existing source inputs and validation.
Cascade keys also contain the session, scene, and light bias.
Collector keys also contain projection, view transform, clipping distances, strength, target size, and primary-view state.
Target replacement forces a new collector binding.

Scene replacement clears old shadow bins and specialization owners.
Pipeline readiness and mesh instance preparation still run each frame.
View extraction resets fade values, so cascade preparation restores fade values before using its cache.
Each active view still draws its shadow atlas. Shader animation does not depend on a caster bounds change.

Compilation and native tests do not establish visual correctness or a performance gain.
Runtime acceptance includes moving shadows, light changes, driving, streaming, and repeated Load.
