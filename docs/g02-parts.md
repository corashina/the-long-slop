# G02 Lada part verification

Scope: build 11140890, one Lada, and the current garage.
Integration target: `master`. Browser acceptance remains open.

Changed files:

- `src/interaction/parts.rs`: return only valid installation targets.
- `src/interaction/targets.rs`: use the simplified scan result.
- `tests/garage_parts.rs`: critical part, plate, and input checks.
- `docs/roadmap.md`: record fixed seats and keep acceptance open.
- `docs/g02-parts.md`: source evidence, checks, and limits.

## Source rules

Authority: `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.

| Source | Rule |
| --- | --- |
| `partscript.Scan`, line 47690 | Find the nearest compatible slot inside the source scan sphere. Return no target when every slot rejects the part. |
| `partslotscript.CanCraft`, line 47963 | Require an empty slot, matching main and part types, and no installed cover. |
| Player interaction, lines 17640–17680 | Apply pickup, seat, slot, and cover rules before removal. The slot hit and part hit use distinct source branches. |
| `MountFunc`, line 18862 | Install on the mount input release. Drop the carried object before mounting it. |
| Player interaction, line 16723 | A later pickup input release can drop a held object when no attachment action consumes it. |
| `partslotscript.Craft` and `UnCraft`, lines 47820–47961 | Move the existing part. Preserve child objects. Transfer rigid body and collision ownership. Update vehicle mass. |
| `massScript`, line 32355 | Include attached child mass and each active tank's fluid amount. |
| `attachablescript`, line 3891 | Plates retain their separate attachment path. |

The engine and radiator have no source requirement to remove each other first.
The hood affects physical access to their interaction surfaces.
No extra removal order, tool requirement, or fluid drain is added.

The Lada seats are fixed source controls.
For example, `level1` GameObject 17433 contains Transform 33209,
BoxCollider 55592, and `seatscript` 74076.
It contains no `pickupable` or `partscript` component.
Seat removal is a prohibited action in this scope.

The front plate target belongs to `FrontBumper`.
The rear plate target belongs to `Car02Full`.
The front plate follows a removed bumper. The rear plate stays with the car body.

## Reproduced defect

A rear bumper near incompatible front slots could not be dropped.
The scan returned a failed-slot blocker even though no installation was possible.
The blocker cancelled the carry input.

The scan now returns only a valid installation target, as the source does.
Invalid slots cannot attach the part. They also do not trap it in the player's hands.
The obsolete scan blocker return value was removed.

## Checks

`tests/garage_parts.rs` contains installed-export checks for:

- Three removal and installation cycles for each supported general part.
- Fluid retention, stable identities, mass conservation, and collision ownership.
- The front plate on its removed bumper, the rear plate on the body, and repeated plate cycles.
- Incompatible installation, repeated attachment rejection, and fixed seats.
- The input sequence that drops a part beside incompatible slots.

These native checks do not establish browser acceptance.
They require `--ignored` and an installed source export.
The cycle helper calls the existing vehicle mass synchronization after each transition.
Gameplay update timing and detached collision response remain unverified.

The corrected input regression failed against the original scan and passed with the change.
That comparison used only Lada objects from the earlier export, mounted read-only.
The final single-Lada run passed all three tests with zero failures.
It covered 20 general parts with three removal and installation cycles each.
It also covered both plates with three cycles each and rejected fixed-seat removal.
The engine and radiator retained their fluid containers and mixtures through each cycle.
Part bodies and collider counts stayed stable. Vehicle and detached-part mass remained conserved.
Source plate owners stayed correct. Repeated installation did not create duplicate target members.
The final WASM compile check and development build passed.
The build used the development profile with optimization level 1 and no debug information.
Four existing unused-variable warnings remain in presentation code.

The isolated single-Lada runtime export passed its existing validation.
The package receipt is `.cache/g02/premerge-2026-10-19/checks/package-receipt.json`.
It records the manifest, main WASM, terrain WASM, and JavaScript hashes.
The build input hashes did not change during the final compilation.

Commands executed in the existing Docker toolchain:

```sh
cargo fmt --all -- --check
cargo test --locked --test garage_parts -- --ignored --nocapture
cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop -p terrain-worker
cargo build --locked --target wasm32-unknown-unknown --lib -p my-summer-slop -p terrain-worker
```

The WASM commands used `CARGO_PROFILE_DEV_OPT_LEVEL=1` and `CARGO_PROFILE_DEV_DEBUG=0`.
`wasm-bindgen` then generated both development browser modules.

The first native run failed because the new runtime manifest was not ready.
An early input test also used a private carry field. The test now uses the public interaction API.
The corrected input test reproduced the stuck-part defect before the product change.
The isolated export first lacked the retained world samples.
The samples were copied into the worktree and passed their existing validation.
The queued native run timed out before asset export completed. It ran no behavior tests.

## Integration checks, 2026-10-19

The G02 source was checked with master at `63e9493` before integration.
Formatting passed. All three G02 native tests and the shared wheel-input test passed.
The WASM compile check passed for the main library and terrain worker.
The native check reported one existing unused-method warning in world streaming.
The WASM check reported the same four presentation warnings.

```sh
cargo fmt --all -- --check
cargo test --locked --test garage_parts --test wheel_input -- --ignored --nocapture
cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop -p terrain-worker
```

Browser acceptance, full checks, and release builds remain deferred.

## Browser status

The earlier main-checkout package reached New game and the pause menu.
Continue failed in the embedded browser because pointer lock was rejected.
That package still contained two cars and does not establish acceptance for this worktree.

G02 browser transitions remain unverified.
Computer Use was stopped with Escape before worktree browser acceptance.
G01 session start, mouse capture, and active input remain unconfirmed for this package.
Full checks and release builds are deferred.

| Part or action | Native check | Browser check |
| --- | --- | --- |
| Engine and radiator removal and installation | Passed, three cycles each, fluids retained | Not run |
| Four doors, hood, trunk, bumpers, mirrors, headlights, and supported interior parts | Passed, three cycles each | Not run |
| Front plate on the removed bumper and rear plate on the car body | Passed, three cycles each | Not run |
| Fixed seats | Removal rejected | Not run |
| Incompatible fit, repeated installation, and drop beside incompatible slots | Passed | Not run |

The pre-merge package and check logs are retained under `.cache/g02/premerge-2026-10-19`.
This package predates later master changes. It does not verify the merged browser package.
