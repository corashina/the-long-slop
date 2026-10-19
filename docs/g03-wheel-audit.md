# G03 wheel assembly

Date: 2026-10-19.
Garage assembly fixes are implemented. Browser acceptance is deferred by request.
Implementation branch: `codex/g03-wheel-assembly`.
Initial code base: `260476e92f5c48bedb700a71af278534a0f42107`.
Integration includes the current master and its single-Lada runtime export.

## Source rules

Steam build `11140890` is the behavior authority.
Evidence comes from `Assembly-CSharp.cs` and the complete serialized source records.

| Input | SHA-256 |
| --- | --- |
| Assembly-CSharp.dll | `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e` |
| Assembly-CSharp.cs | `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee` |
| Initial runtime manifest | `20c89ef5c9e7ce2ae922f2dc6fa30d88eb3528a470a4e8b446655f9af6bc6ecb` |

The integration manifest SHA-256 is `f1a86a6c91448e2c1d07dd90dae273cc136546d0e39f3e1b5cef963b2f7c9a8d`.

The Lada source identity is `level1:77839`.
The integration export contains only the garage Lada.

| Hub slot source | Rim item | Tire item | Hubcap slot source | Tire slot source |
| --- | --- | --- | --- | --- |
| level1:77840 | 46 | 48 | level1:66804 | level1:78999 |
| level1:77847 | 49 | 51 | level1:77392 | level1:68399 |
| level1:77862 | 43 | 45 | level1:65545 | level1:79643 |
| level1:77858 | 52 | 54 | level1:67799 | level1:78040 |

`partslotscript.CanCraft` requires an empty slot and intersecting part tags.
Main tags must intersect when both lists contain tags.
The garage hubs accept `mediumcar` rims with the `felni` part tag.
The tire slots have no main tag restriction and require `gumi`.
The exported garage tires therefore fit the exported garage rims.
There is no tire size rejection rule for these parts.

`partscript.cover_CantCraftIfCovered` blocks rim installation and removal while its hubcap is installed.
The tire parts have no cover restriction.
A hubcap on a loose rim therefore does not prevent tire assembly.

`partslotscript.Craft` disables the rim's listed colliders when the hub slot requires that state.
The rim pickup records include `GumiSLotCol` in that list.
Normal input therefore cannot use the tire slot while the wheel is mounted.
The source player also rejects a tire slot when its rim owner is held.

The source gameplay order is hubcap removal, wheel removal, wheel release, then tire removal or installation.
Wheel installation requires hubcap removal. Hubcap installation can follow wheel installation.
F removes a tire from a loose wheel. A held tire installs into a compatible empty tire slot.
The held rim beside loose tire path and the X shortcut were removed.

A complete wheel retains the rim identity.
The tire and hubcap retain their own identities, conditions, and owners.
Each selected rim and tire has 10 kg dry mass.
`massScript.Mass` sums the current hierarchy. `Craft` and `UnCraft` request mass updates.
These values do not establish source driving response.

## Implemented changes

| Files | Change |
| --- | --- |
| `src/interaction/wheels.rs` | Added shared preflight checks. Corrected collision enable state and collider poses during attachment. |
| `src/interaction.rs` | Added tire slot targets and assembly transactions. Validate before carry release. Removed the X action path. |
| `src/interaction/targets.rs` | Resolve source tire-slot colliders and the current tire after swaps. Enforce covered, occupied, and incompatible targets. |
| `src/interaction/feedback.rs` | Enforce loose-wheel tire actions and source pickup requirements. Removed reverse assembly and X feedback. |
| `src/starting_area.rs` | Route tire actions through the checked transactions. |
| `src/simulation.rs` | Removed only the obsolete input bit. Driving physics is unchanged. |
| `web/input.js`, `web/feedback.js`, `web/index.html`, `README.md` | Removed the X binding and help. Documented the F assembly controls. |
| `tests/wheel_assembly.rs` | Critical identity, condition, mass, collision, transform, and repeated-cycle checks. |
| `tests/wheel_input.rs` | Critical source target, input order, tire swap, hubcap mass, and rejection ownership checks. |
| `tests/web/input.test.mjs` | Check that X no longer sets an input bit. |
| `docs/roadmap.md` | Removed the completed G03 implementation row. Kept wheel browser acceptance under G16 and clarified the measurement rule. |

The assembly transactions preserve the held item and carry constraint when validation rejects an action.
Tire slot removal resolves the currently attached tire rather than the original serialized tire.
No allocations or new collider setup were added to per-frame paths.

The attachment API still supports direct source script operations on mounted rims for imports.
Player actions require a loose rim. The native stability test checks both API cases.
Mounted tire assembly now keeps tire collision disabled and uses the rim's body-relative pose.
Wheel installation sets correct body-relative collider poses before synchronization.
Loose components retain active collision and their separate body identities.

## Verification

The original native audit reported twelve mounted tire collision violations.
The corrected stability test passes three cycles for each of the four Lada wheels.
Identity, ownership, condition, mass, collider parent, collider enable state, and collider pose assertions pass.
The input test also uses the garage spare tire, item 231, on Lada rim 46.

| Terminal check | Result |
| --- | --- |
| Explicit `wheel_assembly` export test | Passed. Twelve wheel cycles. |
| Explicit `wheel_input` export test | Passed. Source rays, F input order, tire swap, rejected actions, mass, and conditions. |
| Native `fresh_session` target | Two passed. One export test ignored. |
| Web `input`, `save-session`, `save-bridge`, and `menu` | Sixteen passed after integration. |
| JavaScript syntax checks | Passed for `input.js` and `feedback.js`. |
| WASM library compile check | Passed. Exit code 0. |
| Formatting and whitespace checks | Passed. |
| Full checks and release build | Not run. |

The native compiler reports the existing unused `startup_job_audit` method.
The WASM compiler reports four existing warnings in `materials.rs`.
The first source audit attempt could not read a Windows junction through Docker.
The isolated test export now uses a verified manifest copy.

The critical export tests are ignored by default because they require build `11140890` assets.
Their explicit command is:

```powershell
docker run --rm `
  --mount "type=bind,source=$PWD,target=/workspace" `
  --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
  --mount 'type=volume,source=tld-wasm-target,target=/target' `
  --env 'CARGO_TARGET_DIR=/target/g03-wheel-assembly' `
  my-summer-slop-toolchain `
  cargo test --locked --test wheel_assembly --test wheel_input -- --ignored --nocapture
```

The other targeted commands are:

```powershell
./scripts/check.ps1 -Test fresh_session
./scripts/check.ps1 -WebTest input,save-session,save-bridge,menu
node --check web/input.js
node --check web/feedback.js
```

The session check also checks formatting and compiles the WASM library.
Logs and source records are summarized in `.cache/g03/`.
The baseline failure log is `wheel-audit-before-fix.log`.
Initial logs are `wheel-final-native.log`, `wheel-input-final.log`, `session-wasm-final.log`, and `web-final.log`.
Integration logs are `merge-native-wasm.log` and `merge-web.log`.

## Deferred acceptance

No Computer Use or browser automation was used for this implementation.
All seven requested browser actions remain deferred.
No assembly, fit rule, or transition has received browser verification.
G01 session start and G02 part handling remain unverified through normal browser input.
Native tests do not confirm physical contact response, rendered behavior, or browser mouse capture.
A matching browser package was not built or tested.

Source driving measurements are unavailable. They are required for G11 physics changes, not G03 garage assembly.
Tire grip, radius behavior, hubs, suspension, brakes, drivetrain, and vehicle physics were not changed.
No driving result is claimed.
