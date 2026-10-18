# Shadow specialization lookup, 17 October 2026

Status: implementation is under test. There is no accepted FPS improvement yet.

The first keyed implementation used a standard hash map. Two settled active-garage samples measured 7.72 FPS. The unchanged baseline median was 8.73 FPS. This variant has no accepted gain. These exploratory runs used different save-attempt histories. The final comparison must use fresh sessions on both sides.

The current source uses a sorted, preallocated vector and binary search. It retains the original reservation limits. Cache hits neither allocate nor clone the layout or material owners. The entry retains both owners so their pointer identities cannot be reused. The matching rules include every mesh flag and both pointer identities.

The native oracle compares 105 entries and 192 hit or miss queries against the original linear predicate. It includes equal contents in distinct allocations. Both critical native checks pass. The sorted release compiled on 18 October in 4 minutes 44 seconds. The build had 11 existing warnings. Rust formatting, both native checks, and both FPS-counter checks passed. Browser and FPS measurements remain pending.

## Comparison baseline

The exploratory source baseline was 703d8e6eb3feb6e139d386e87551647db8ad2635. The pending final comparison uses merged master 75ebdffa9b1512deff9b4e6fb7bf690a1a182cd5 on both sides. The branch excludes the failed stencil experiment. Both benchmark artifacts use the same private source asset package.

Clean master 7eb1774 was rejected as a gameplay baseline. The renderer reported Ready, but New game stayed disabled after 247.75 seconds. Surface admission remained pending. The complete receipt is retained in `.cache/benchmark/results/baseline-startup-blocker.json`.

The retained 703 release package reached full readiness independently. New game enabled. A fresh session accepted input and advanced physics. This comparison does not establish a speedup on unmodified master.

## Measurement method

The canvas is 1280 by 720 pixels, with DPR 1. The document is visible. One game tab runs at a time. No compilation or tests overlap timed FPS windows.

Each accepted FPS window lasts ten seconds without WebGL hooks. The probe resumes gameplay and waits for generation and pending work to settle. Receipts contain the start and end state, world counts, frame intervals, GPU identity, and render submission tokens. Callbacks must agree with submitted frames. These are submitted frames, not monitor presentation measurements.

The baseline samples kept 14,567 generated objects, 54 roads, and 19 near terrain tiles. No world work was pending at either endpoint. Physics remained active. Separate WebGL counters recorded about 5,306 draws per active frame. Instrumented FPS is not used for acceptance.

Initial samples with streaming work or paused physics are excluded. A candidate sample with new generation work at its end is also excluded. All receipts remain available for review.

## Driving limitation

The normal save preparation path rejects `reference/food.gltf`. The constructed driving save fails before and after a fresh game start. The unmodified garage capture was not directly tested for preparation. The source trace indicates that normal Load shares the same unconditional availability check. The file exists. The source asset availability check requires a retained handle that the initial fixed-asset loader does not own.

The driving fixture is unaccepted. No driving FPS claim is made. This error also exists before the cache change and remains outside this branch.

## Local artifacts

- `.cache/benchmark/provenance.json`: exact revisions, package hashes, manifest hash, lock hash, and release profile.
- `.cache/benchmark/results/`: complete observation receipts.
- `.cache/benchmark/playable-baseline/pkg/`: unchanged 703 release.
- `.cache/benchmark/std-candidate/pkg/`: rejected hash-map release.
- `.cache/candidate-build.log`: hash-map release compilation.

Source integration status: The shadow-cache branch includes current master and preserves its batching and reflection changes. It adds the sorted cache, diagnostic count, and critical cache checks. Runtime FPS remains unmeasured. No improvement is claimed.

## Manual test, 18 October

The sorted option 5 release was served at http://127.0.0.1:4191/. That local server is now stopped. This uses branch `codex/shadow-cache`, revision `9364acc`, based on `75ebdff`. Later master merges are excluded. The release WASM SHA256 is `b61d39dbf8fe1bb19ca8f8df98ef920a7e996e78aff4defd48042a15ba947a4b`.

Restart the local server with:

```powershell
Set-Location 'C:\Users\Corashina\.codex\worktrees\shadow-cache\the-long-drive-wasm'
$env:PORT='4191'
node scripts/serve.mjs
```

No benchmark input hook or WebGL counter hook runs in this manual session. No FPS improvement is claimed from startup counters.
