# Final lighting fix report

Base: `4d7b2f355e3f7f3f3265f1a12880ab03fef84dbd`.
Branch: `codex/day-night-lighting`.
Worktree: `C:/Users/Corashina/.codex/worktrees/day-night-lighting/the-long-drive-wasm`.

I applied the implementer template and the verification skill.
I applied [stop-slop](https://github.com/hardikpandya/stop-slop/blob/main/SKILL.md) to the controls row and this report.

## Changes

- I1: Require the bindings that lighting setup and reset consume.
- M1: Add `L` to the README driver-seat controls.
- Add six failure tests with 17 malformed manifest cases.
- Keep the installed export and checked fixture unchanged.

The validator names the source file, source ID, and required field in each new error.
The new checks run during loading, before setup or reset.
They add no frame work or fallback path.

## Source and consumer checks

| Required contract | Consumer | Check and error field |
| --- | --- | --- |
| Sky floats `_SunSize`, `_SunSizeConvergence` | `SkyState::new`, `src/presentation/sky.rs:56-57` | Require both typed float keys; retain finite-value validation. Error uses sky material `sharedassets1.assets:566` and `floats.<property> missing`. |
| First star texture `_MainTex` | `bind_sky`, `src/presentation/sky.rs:275` | Require `_MainTex` at index zero; retain texture ID, scale, and offset validation. Error uses `sharedassets1.assets:431 textures[0]._MainTex missing`. |
| Cloud `_ScatterMap0`, `_ScatterMap1`, `_DensityMap`, `_TextureMap` | `bind_sky`, `src/presentation/sky.rs:385-405` | Require each property on both cloud materials. Retain texture ID, scale, offset, and geometry checks. Errors use material IDs `sharedassets1.assets:44` or `:43` and `textures.<property> missing`. |
| Camera roles `Cam`, `farCamera` | `bind_lighting`, `src/presentation/lighting.rs:574-583` | Require both roles. Retain four-camera count, unique roles, finite planes, valid FOV, and nonzero masks. Error uses `level1:56810 cameras.<role> missing`. |
| `Headlight`, `Rear` controller families | `LightingState::reset`, `src/lighting.rs:702-719`; `update_vehicle`, `src/lighting.rs:823-827` | Require at least one of each family. Retain typed kind, state bounds, node, beam-cookie, and lamp-link checks. Error uses `level1:56810 controllers.<family> missing`. |
| One road light | `road_lamp_output`, `src/lighting.rs:954`; stream binding, `src/presentation/lighting.rs:1106` | Require one entry before either consumer indexes it. Retain light, prototype node, and road controller links. Error uses `sharedassets1.assets:144809 road lights[0] required`. |

I checked the exported source shader declarations for the required material property names.
The procedural sky declares both float properties.
The particle shader declares `_MainTex`.
The cloud shader declares the four required 2D texture properties.
Rust deserialization retains the typed float, texture-link, camera, and controller records.

I traced the other setup lookups and indexes.
Existing checks cover texture IDs, flare atlases, sky asset/root links, cloud nodes, gradients, and two cloud/star records.
Existing checks also cover fixed array lengths, controller states, animated poses, cabin modes, and lamp links.
The road first-light index lacked a count check, so I included it in I1.
The existing renderer already requires one light per road prototype.

`app::run` calls `ReferenceManifest::from_json` before `GarageRuntime::new` and renderer setup.
`from_json` calls `manifest.validate`, which calls this lighting validator.
I kept that loading path and the consumers unchanged.

## Failure test coverage

| Test | Malformed cases |
| --- | --- |
| `rejects_missing_required_sky_float` | Remove each consumed float key. |
| `rejects_missing_required_star_binding` | Remove the binding; rename its property while retaining its texture. |
| `rejects_missing_required_cloud_binding` | Remove each of four bindings from each of two cloud materials. Keep the texture records. |
| `rejects_missing_required_camera_role` | Rename each consumed role to another unique role. Keep four valid cameras. |
| `rejects_missing_required_controller_family` | Remove all Headlight controllers; remove all Rear controllers. |
| `rejects_missing_required_road_light` | Remove the road light list while retaining the light record. |

Each case asserts an error with its source identity and required field.
The tests mutate fresh copies of the valid fixture.

## Commands and evidence

All Rust commands use this Docker invocation and the existing checkout target:

```powershell
docker run --rm --mount type=bind,source=C:/Users/Corashina/.codex/worktrees/day-night-lighting/the-long-drive-wasm,target=/workspace --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target --env CARGO_TARGET_DIR=/target/0aeee55818e76061 my-summer-slop-toolchain <command>
```

I captured Docker output with `2>&1 | Tee-Object -FilePath <log>`.
Logs remain in `.cache/reference/`, outside Git.

### RED

Command: `cargo test --locked --test lighting_reference`.
Log: `.cache/reference/final-fix-red.log`.
Docker exit: `1`.

```text
rejects_missing_required_road_light ... FAILED
rejects_missing_required_star_binding ... FAILED
rejects_missing_required_controller_family ... FAILED
rejects_missing_required_cloud_binding ... FAILED
rejects_missing_required_sky_float ... FAILED
rejects_missing_required_camera_role ... FAILED
missing setup binding must fail validation: ()
test result: FAILED. 9 passed; 6 failed; 2 ignored
```

The failures confirm that the old validator accepted each malformed contract family.

### GREEN

Command: `sh -c 'cargo fmt --all && cargo test --locked --test lighting_reference --test lighting'`.
Log: `.cache/reference/final-fix-green.log`.
Docker exit: `0`.

```text
tests/lighting.rs: test result: ok. 21 passed; 0 failed
tests/lighting_reference.rs: test result: ok. 15 passed; 0 failed; 2 ignored
```

All six new failure tests passed after the validator changes.

### Final checks

Command:

```sh
cargo test --locked && cargo test --locked -- --ignored && cargo fmt --all -- --check && cargo build --locked --release --target wasm32-unknown-unknown --lib && wasm-bindgen --target web --out-dir web/pkg --out-name my_summer_slop /target/0aeee55818e76061/wasm32-unknown-unknown/release/my_summer_slop.wasm
```

Log: `.cache/reference/final-fix-gates.log`.
Docker exit: `0`.

| Check | Result |
| --- | --- |
| Full native run | 153 passed; zero failed; 15 installed cases ignored. |
| Installed run | 15 passed; zero failed. Both installed lighting checks passed. |
| Formatting | `cargo fmt --all -- --check` passed. |
| Release WASM | Passed in 59.54 seconds. |
| Matching bindgen | Passed. CLI and locked crate use `0.2.129`. |
| Patch whitespace | `git diff --check` passed. |

The test and build output contains no compiler warnings.
The house pose test printed its existing over-60-second progress message, then passed.
Git printed its configured LF-to-CRLF warning during diff inspection.

The generated pair remains outside Git:

| File | SHA-256 |
| --- | --- |
| `web/pkg/my_summer_slop.js` | `7c4f2044f7c990fc6b415b40669965e51f7059ba76ee023a5ac6b46118f49fed` |
| `web/pkg/my_summer_slop_bg.wasm` | `6a73f7fcfc86aeb4c5aad3cb35d389ef8afd46ef997df11174d15308c0f99be9` |

I notified the parent when this pair was ready.
I held source and WASM unchanged for the parent's normal load/night-low browser check.
The parent checked this matched pair after my readiness message.

| Browser check | Parent result |
| --- | --- |
| `http://127.0.0.1:4175/` | Ready; Start enabled; no `ERROR` entries. |
| `http://127.0.0.1:4176/?phase=night&control=dashboard&beams=low` | Driver seated; ignition on; engine stopped; paused after 55 steps. Visible wall beams and horizontal occlusion; no `ERROR` entries. |

The parent loaded these URLs in order after the final package build.
The parent found no errors after `finalFixBrowserStart` and confirmed the visible `L` row.
Capture: `C:/Users/Corashina/.codex/visualizations/2026/10/10/01a12681-8e80-7e00-b838-1e5b879912f2/lighting-final-fix-night-low.jpg`.
These checks confirm normal loading and the night-low view after the validation fix.
They do not establish new original-image parity claims.

I ran the native suite once because the shared validator affects runtime construction across the product.
I ran installed validation once against the existing export.
The installed lighting check also reads the source links and verifies exported file hashes.
I did not rerun Python tests because this fix changes neither the exporter nor the source producer.

## Files

- `src/reference/lighting.rs`
- `tests/lighting_reference.rs`
- `README.md`
- `.superpowers/sdd/2026-10-10-day-night-lighting/final-fix-report.md`

## Self-review and concerns

I reviewed the diff against I1 and M1.
I kept the checks beside their existing validation blocks.
The validator was already large; this fix adds fixed required-field checks without a new abstraction.
I found no further change needed within this scope.
I preserved other targets, processes, servers, and export files.
The parent owns the normal load/night-low browser check and the one scoped review.
Existing rendering and original-image comparison limits remain unchanged.
