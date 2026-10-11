# Verification record

The following sections record separate interaction and source UI checks.
Merge checks appear in the final section.

## Garage and interaction verification

Reference: installed Steam build 11140890. Vehicle: fixed Lada 2101.
Verification date: 2026-10-11, Europe/Warsaw. Final product commit: `d402a09`.
Final `d402a09` code passed fresh independent checks: 137 native tests and 31 installed-source tests.
Fresh independent formatting, release WASM, and 12 exporter tests also passed.
The earlier full checks and focused label regression remain recorded below.
Worktree: `C:/Users/Corashina/.codex/worktrees/interaction-parity/the-long-drive-wasm`.
Read the [approved interaction scope](superpowers/specs/2026-10-10-interaction-parity-design.md), [garage scope](superpowers/specs/2026-10-09-garage-start-design.md), and [decision record](interaction-decisions.md).

## Source and build checks

The exporter follows selected scene links to the home and Car02Full hierarchy.
It converts handedness once and preserves nested scale and reflected wheel transforms.
The manifest retains source IDs and input hashes. Generated assets stay outside version control.
Startup contains 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
The car has 955 kg dry mass and 6.6 kg initial fluid mass.

| Check | Result |
| --- | --- |
| Native suite | 137 passed |
| Installed-source gates | 31 passed |
| Exporter suite | 12 passed |
| HUD callback test | 1 passed |
| Formatting | Passed |
| Release WASM and browser package | Passed |
| Task, full-branch, and final code reviews | Passed |

The final label-only change passed its installed label regression, formatting, and WASM/package build.
The independent final exporter rerun passed all 12 tests in 22.016 seconds. Earlier logs remain preserved.
Native checks cover fluids, ignition, heat, collisions, carry, mechanisms, lamps, parts, push, throw, pause, recovery, and rebasing.
Source tests use the actual export. Some tests stage clear player positions.
They do not prove a continuous browser route.
The source route covers garage exit, nested parts, transferred hubcaps, engine and radiator operation, and lifecycle cancellation.
Carry regressions cover floor separation, recovered collision groups, installed-can pickup, recenter, and walking.

Final logs are in `.superpowers/sdd/2026-10-10-interaction-parity/`:
`final-fix-full.log`, `final-fix-python.log`, `final-fix-hud.log`, and `final-fix-report.md`.
Earlier RED/GREEN logs and screenshots remain in the ignored `.cache` directory.

## Package and browser conditions

The root-owned Windows server serves this worktree at `http://127.0.0.1:56191`.
The earlier 53a9a4e Windows preparation package matched its checkout at that time.
Its WASM was 50,916,156 bytes, SHA256 `b0f7325325a8ec276bdc07f91fc44de952b008d14d0b113a7a00ec5ef9035fd4`.
Final d402a09 WASM is 50,916,241 bytes, SHA256 `99bd67a677d5afbfd20771a5f04a14d960e563783c6d7e0e686541add455c4d4`.
The manifest is 3,849,428 bytes, SHA256 `ed43d7d49fcdb825c364b2a2ae4ac0b5043924dece3449bb1347ab863fa8c855`.
Served final Windows and Linux package bytes matched the final checkout.

Windows routes used Chrome 154.0.8037.95 with ANGLE SwiftShader.
Flags: `--use-angle=swiftshader --enable-unsafe-swiftshader`.
Playwright clicked Start and used normal pointer lock, keyboard, and mouse input.
The helper read public `gameState` and `gameKeys`. It did not inject game state.
Windows captures used 1280 by 720, with functional checks at 640 by 360 and 960 by 540.

The 53a9a4e browser closed before a closer grip retry. No close command was sent; the cause is unknown.
The helper read an already-closed page and exited. Evidence remains in `.cache/task8-53a9a4e-interrupted/`.
The earlier abd2f5c helper had read a partial command file. Atomic command writes now prevent that harness failure.
The d402a09 Windows session later lost its graphics context after a GPU process exit.
Windows headless pointer lock also confined the desktop cursor. Testing stopped, and only the owned browser/helper were closed.
Evidence remains in `.cache/task8-d402a09-context-lost/`. No Windows browser was launched afterward.

The active Windows harness entries `.cache/task8-session.cjs` and `.cache/task8-route-input.cjs` are retired.
Both fail before loading Playwright. Their scan and sequence callers inherit that failure.
Historical harness copies remain unchanged as evidence and are for inspection only.
The supported runner is `.cache/task8-run-linux.ps1`.
Its Linux helper rejects Windows, non-container Linux, and DISPLAY or WAYLAND_DISPLAY environments.
A normal `shutdown` command releases pointer lock and closes only the browser owned by that session.
Startup failures and session cleanup use the same owned-browser close path.
Four syntax checks and nine stub guard/cleanup checks passed. No real browser was launched for these checks.
Guard evidence: `.cache/task8-harness-guard-check.cjs` and `.cache/task8-harness-guard-check.log`.

Preparation-to-road continuity is unavailable. These split routes do not prove full continuous acceptance.

The remaining route used Linux Chromium 151.0.7922.34, Playwright 1.62.1, and ANGLE SwiftShader.
Image: `mcr.microsoft.com/playwright:v1.62.1-noble`.
Digest: `sha256:dcc5531e97840b9b5e794f2814476b21571c5124a3fca2267d73041f56e7580e`.
Docker used `--init`, `--network none`, and `--shm-size=2g`, with no host display mount.
The checkout and existing dependencies were read-only. A separate evidence directory was writable.
The server and browser ran inside the container. Root checked full desktop cursor bounds three times during Linux pointer lock.
Evidence: `.cache/cursor-linux-isolation.jsonl`, `.cache/task8-linux-isolation.txt`, and `.cache/task8-run-linux.ps1`.
Linux headless shell emitted opposing trusted mouse warps and failed calibration. It was closed.
Full Chromium new-headless passed the normal mouse-delta check in the same container.
A read-only native event observer recorded movement. It injected no events or game state.

## Action evidence

| Action | Browser evidence | Limit |
| --- | --- | --- |
| Can-cap pickup, carry, recenter, fill, drop | 53a9a4e Windows route | Direct crouched path passed after a blocked bench approach |
| Mounted hubcap, rim, engine removal and installation | 53a9a4e Windows route | Engine operation passed after installation |
| Charged throw and meter | 53a9a4e Windows route | Charge reached 0.45; release cleared state |
| Garage doors, window, mirrors, glovebox, visors | Paired Windows captures; Linux garage route | Window scroll was not proven |
| Cabin lamp | Windows bright/dark pair | No rendered Unity comparison |
| Vehicle headlights | Linux 1280 off/on pair with engine running | Two beam pools appeared on the brick wall after E and 60 steps |
| Side mirror label | d402a09 Windows route | Label-only package change |
| Ordinary push | d402a09 Windows route | F release stopped applied push |
| Latched grip | d402a09 Linux route | Release and look-away retained grip; next F released |
| Held-item pause | d402a09 Linux route | Fuel can remained held; charge and keys cleared |
| Restart with removed part | Windows removed door; Linux removed hubcap | Separate from held-item Restart |
| Restart with held item and shifted origin | Linux returned Fuel can | Cleared hands, origin, rebases, ignition, and keys; restored fuel 0.5 |
| Seat entry, start, garage exit | Windows seat actions; Linux driving exit | Driver door required visible closure |
| Recovery | d402a09 Linux route | Normal R preserved fuel and engine state |
| Origin shift | d402a09 Linux normal drive | One rebase at absolute x2102.222 during off-road driving |
| Return home | Physical Linux run after rebasing | Garage leaves remained open; loose-item persistence has native proof only |
| Covered loose-wheel pickup | Native installed-source regression | No browser proof |

Filling increased gasoline from 0.5 to 1.128319 L. Collision rules were unchanged after the first blocked approach.
The final held-can pause recorded charge 0.55, active throw, and keys 256 before Escape.
Pause cleared charge and keys. Resume retained the can without a delayed throw after 30 simulation steps.
A removed-part Restart restored fuel 0.5, origin 0, handbrake, and empty hands.
The first shifted-car reverse timed out. In the fresh route, the open driver door caught the garage frame.
A negative seated drag left it open. The opposite positive drag visibly closed it, then reverse cleared the garage.
The stage label `closed-door reverse` identifies a failed closure attempt, not proof of closure.
The passing image is `task8-linux-driver-opposite-close.png`; `task8-linux-verified-exit.png` shows the car fully outside.

Normal R aligned the car on the road at its current distance. R did not restore garage pose.
Initial travel followed negative Z. The car later changed direction near z-974 with only W held.
The cause was not isolated. The subsequent drive proves off-road travel and rebasing, not sustained road following.
At absolute [2102.222, 2.208, -965.356], the origin became [2048, 0, -1024], with finite vehicle state.
After another R, the engine was stopped and the car parked. Normal input returned the player home on foot.
At [32.683, 2.465, -69.490], the garage leaves remained open and the car remained parked remotely.
The passing capture is `task8-linux1280-returned-garage-leaves.png`.
The earlier `task8-linux1280-home-persistence.png` was a missed approach and proves no leaf state.
No Restart occurred during that return. No loose home item was moved after the preceding Restart.
The home capture therefore proves mechanism persistence. Loose-item persistence remains native/source proof.

The final headlight pair is `task8-linux1280-engine-headlight-off.png` and `task8-linux1280-engine-headlight-on.png`.
Both use the same parked view with the engine at 800 rpm. The controller also confirmed the visible beam change.
After capture, Escape paused the game with keys 0 and pointer lock released.
Linux Chromium closed normally with exit code 0. Its container exited with code 0 and was removed.
The root-owned Windows server remained available.

## Measurements and limits

Linux functional actions used 640 by 360, 480 by 270, and 320 by 180 for long travel.
The same browser was resized for 960 by 540 and 1280 by 720 captures.
At 1280 by 720, 30 RAF intervals had median 1833.3 ms and p95 2083.3 ms.
The range was 1699.9-2099.9 ms. This software-rendering result does not establish hardware or Windows performance.
Linux loading enabled Start in 5703 ms and acquired pointer lock in 11713 ms.
Its captured page and HTTP error arrays were empty. The Windows GPU failure is recorded separately.
Observed Windows request URLs map to 121,014,961 bytes of local files. This is not network transfer size.
The inventory includes 60,230,351 bytes of PNG textures and 50,916,241 bytes of WASM.
Route logs, screenshots, input history, and public-state snapshots remain in `.cache/task8-linux/` and `.cache/task8-linux-route.log`.

All screenshots show the port. No rendered Unity comparison was captured.
Recovered source records and native checks support source behavior; screenshots do not establish Unity visual parity.
Hardware ANGLE Direct3D11 stalled before Start for 90 seconds in the Windows setup.
The evidence does not identify the exact graphics operation. Software rendering does not establish hardware performance.

## Source rules and limits

Use, grab, fill, and push resolve recovered owners and masks.
General parts scan near the held part with source mask 513 and a 0.2 m base radius.
Source multipliers and nearest compatible slot selection follow recovered `partscript.Scan`.
Wheels retain their separate aim query. No general-part layer mask was widened.
The HUD names the resolved action owner. Remove the installed hubcap before its wheel.
Carry supports distance, recenter, translation, rotation, roll, and charged throw.
Source pickup rules can disable rotation; the garage fuel can has that rule.
Ordinary push stops on F release. Grip stays latched until next F, excess distance, owner loss, or lifecycle cancellation.
Changing aim does not release grip. Owner loss means an invalid physical owner or collider.
Pause, capture loss, recovery, and restart clear sampled input before another simulation step.

Rapier raycast suspension replaces Unity WheelCollider physics. Identical handling is not established.
Recovered dimensions, springs, damping, brakes, condition, and drivetrain ratios control the adapter.
The fixed garage spawn follows the user's request. Seeded wheel condition makes startup repeatable.
The exporter selects one high detail LOD and maps Unity materials to glTF.
Unity shader effects, live mirror reflections, runtime plate text, and audio remain outside this stage.
Selected Lada seats lack source pickup links. Plate slots start empty; no removable seats or plates were invented.
Earlier allocation checks measured Rapier movement allocations. The player wrapper added none above that path.
These results do not claim zero allocation for the game.

Read the [garage recovery record](research/2026-10-09-garage-reference.md) and [interaction comparison](research/2026-10-10-interaction-reference.md).
Installed-build evidence controls conflicting secondary descriptions.
The [decision record](interaction-decisions.md) preserves all twelve ledger rulings and their rework costs.

## Source UI verification

These checks cover the source UI branch before the interaction merge.
Its controls and headlight limits describe that branch.

Scope: approved current features in docs/superpowers/specs/2026-10-10-source-ui-design.md.
Source: installed Steam build 11140890.
Worktree: source-ui. Base: 743ef8b.
Date: 2026-10-10, Europe/Warsaw.

The UI now uses recovered menu sprites, canvas layouts, action icons, and fluid presentation flags.
The old serif shell, persistent vehicle card, X assembly action, and P/R shortcuts are removed.
F press picks up, drops, or dismounts. F release mounts.
RMB moves a held item. MMB rotates it.
Esc opens the menu. Backspace toggles feedback.
Recover remains in the pause menu. Fullscreen remains in Controls.

### Final checks

| Check | Result |
| --- | --- |
| Rust formatting and native suite | 109 passed; six installed-source gates skipped by default |
| Explicit installed-source gates | Six passed |
| Browser module suite | 14 passed |
| Python exporter suite | 11 passed |
| Complete installed-build export | Passed |
| Release WASM and browser package | Passed |
| Independent source review | One Important finding fixed; no Critical findings |
| Final diff and source-asset exclusions | Checked before the integration commit |

Commands use the source-ui worktree and its separate Cargo target volume.
The ignored .cache/check.ps1 and .cache/build.ps1 pin that volume.
The tracked scripts contain the same checks with the normal project cache.

- Python: unittest discover -s tools/reference/tests -v.
- Browser modules: node --test tests/web/menu.test.mjs tests/web/input.test.mjs tests/web/feedback.test.mjs.
- Native and release: .cache/check.ps1.
- Installed gates: cargo test --locked -- --ignored in the toolchain container.
- Export: tools.reference.export with the installed Steam root.
- Package: .cache/build.ps1.

Final logs are retained under ignored .cache/source-ui-verification.
They include final-check.log, final-source.log, final-python.log, final-node.log, final-export.log, and final-package.log.

### Review corrections

The independent reviewer found emission on the source warning lamps' Off materials.
Unity's _EMISSION keyword now controls both emission factor and texture export.
A focused regression test failed before the fix and passed afterward.
Regenerated Off materials have zero emission for fuel, oil, battery, and headlight2.
Their On materials retain the recovered emission values.

Browser startup exposed a late glTF material load that reloaded the vehicle scene.
The renderer now retains raw glTF handles during initial loading.
A startup adapter creates and caches the used lamp materials.
The final browser reaches the ready menu with the vehicle present.

Static review found clipped carrying text.
Its bottom alignment now follows the recovered lower-right text rectangle.
The confirmation capture shows all three lines above the bottom margin.

### Browser evidence

The final browser starts at http://127.0.0.1:4174/.
Desktop review used 1280 by 720 pixels. Narrow review used 390 by 720.
Start and Controls were inspected over the actual loaded garage.
Pause, failure, and fluid feedback were inspected with an isolated UI fixture.
That fixture contains no WASM runtime, simulated input, or product capture bypass.
It was removed after review.

Captures are under ignored .impeccable/review:

- source-ui-start-final.png: actual ready menu and vehicle.
- source-ui-controls-desktop.png and source-ui-controls-narrow.png: actual Controls.
- source-ui-pause-fixture.png: source pause geometry and session distance.
- source-ui-error-fixture.png: disabled actions and a readable failure.
- source-ui-fluid-fixture.png: source fluid text, fill icon, and complete carrying help.

The Impeccable detector ran once over web.
It reported three Arial warnings and one unrecorded error color.
Arial is the recovered gameplay font. The error color is now recorded in the palette.
Two files had invalid ellipsis bytes. They were repaired and validated as UTF-8.
The detector could not inspect those files before repair. It was not rerun.

The embedded browser rejects pointer lock.
Live pause and contextual gameplay remain covered by state and native tests, plus static layout review.
The approved final continuous browser route waiver remains.
No live pixel-parity claim is made.

### Instruments and limits

| Instrument | Source range | Unity Y angle |
| --- | --- | --- |
| Speed | 0 to 160 km/h | -64 to 64 degrees |
| Temperature | 10 to 130 degrees Celsius | -30 to 30 degrees |
| Fuel | 0 to 39 L | -26 to 26 degrees |

Needles use source signal smoothing at 5 times dt and value smoothing at 8 times dt.
The five odometer digits use base 10, one fractional digit, and source carry rules.
They use source X rotations and angle smoothing at 20 times dt.
The display starts from the recovered value and adds session travel.
Recovery and coordinate rebasing preserve it. Restart resets it.
The source fuel, oil, and battery guards use local ignition and recovered thresholds.
The unsupported headlight indicator remains off. The Lada has no RPM needle.

Entity and material binding occurs during loading.
Instrument updates use fixed arrays and cached handles.
Feedback retains display values and changes DOM text only when needed.
Browser text uses Liberation Sans and Arial. It differs from Unity's SDF renderer.
Liberation Sans 2.1.5 provenance and license remain in web/fonts.
Generated original assets remain excluded from Git.

### Execution rulings

- PowerShell briefs replace unavailable Bash bookkeeping helpers.
- A separate Cargo target cache prevents stale metadata from concurrent work.
- Menu synchronization uses three scalars to avoid a new frame object.
- Recovered Gas, empty, and (full) labels override illustrative plan wording.
- Cached startup material conversion replaces late subasset loading. Bevy's converter is private.
- Design documentation ran inline under the approved native workflow.

The design record uses the actual source palette, menu and gameplay fonts, canvas rules, and source text shadows.
Its sidecar records color metadata, motion, breakpoints, and standalone text components.
Original sprite previews require the ignored local export and remain outside the sidecar.
No independent-review Minor findings remain deferred.

## Local interaction merge, 2026-10-11

Master `5d4cc16` integrates interaction worktree commit `c2d0fe6`.
The merge keeps source menus, feedback, instruments, mechanisms, parts, carry, push, grip, and throw.
Source `fpscontroller.MountFunc` lines 5544–5557 install on F release.
Its callers at lines 3396 and 4325 require charge below the throw minimum.
Charged release over a valid slot throws. Blocked installation keeps the held part and clears charge.
Target names and source fluid labels use separate fields.
Wheel compatibility and occupancy share one rule for selection, feedback, and execution.

Headlight state now drives the high-beam indicator and odometer illumination.
Source evidence: `carscript.cs` lines 1197–1208, 2437–2445, and 2559–2584, plus `forgoszamlap.TurnOn/TurnOff`.
Digit material handles remain cached.

Fresh checks passed: 157 native tests, 31 installed-source tests, 20 browser unit tests, and 19 exporter tests.
Formatting, the release WASM build, and browser package generation passed.
The combined export contains 417 colliders and 127 items.

An isolated Linux Chromium 151.0.7922.34 check passed at 960 × 540.
Start enabled after 6 seconds. A trusted click captured the mouse. Physics advanced at least 12 steps.
Escape paused play and released capture. The browser and internal server then closed.
No page errors or gameplay asset HTTP errors occurred. The console recorded one 404 for the excluded favicon request.
Docker had no network or host display. Only the evidence directory was writable.
Evidence is in ignored `.cache/interaction-merge/`: logs, `smoke.json`, and three screenshots.
This check covers startup and pause. Earlier route and performance limits still apply.

Verified browser package SHA-256:

- WASM: `cd16e48a1495a9dff8fbf9265af03a0c2acdc9df96f0cf8cd7dfca2b55f203b2`.
- JavaScript: `1c6ac36bffef8185ec161b4535b222a22f04a587baf3a79d77f0e092a617a084`.
- Manifest: `826fc55277c235a062acacfd0a47cebe3dff3a297577cb0acd32c644ff6ac0c0`.

The managed interaction worktree and its local evidence remain available.
