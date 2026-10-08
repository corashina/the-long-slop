# Verification record

Date: 2026-10-08. Branch: `codex/desert-driving`.
The controller owns browser checks. The records below distinguish browser observations from native coverage.

## Build

Run from the project folder:

```powershell
./scripts/check.ps1
./scripts/build.ps1
node scripts/serve.mjs
```

Open [the local game](http://127.0.0.1:4173). Select Start after the scene loads.
The server binds to loopback. The build writes WASM and JavaScript to `web/pkg`.

- Rust: `1.99.0 (b940084d7 2026-09-28)`.
- Rust image: `rust@sha256:6ff07edce8775d0f64be7aba9197229407301bddf2054d62c27b541a6238a181`.
- Bevy: `0.19.1`. Rapier: `0.36.0`. wasm-bindgen crate and CLI: `0.2.129`.
- Cargo.lock pins dependencies. The runtime uses one simulation thread and WebGL2.
- Final presentation check: formatting, 23 native tests, and release WASM compilation passed.
- `node --check web/main.js` and `git diff --check` passed.

## Critical native tests

The suite contains nine vehicle tests, seven world tests, and seven simulation tests.

| Area | Coverage |
| --- | --- |
| Vehicle | Wheel contact, acceleration, braking, reverse, steering, handbrake, chassis collision, and velocity reset |
| Timing | Fixed 60 Hz steps; a one-second stall permits six catch-up steps |
| World | Deterministic samples, shared seams, lateral overlap, bounded chunks and colliders, and destination loading |
| Origin | Preserved absolute position, rotation, wheel offsets, velocities, contact, and trip distance after a shift |
| Fuel | Bounds, idle/throttle rates, zero-fuel engine gating, and restored thrust after refueling |
| Refueling | Below 1 km/h, within 6 meters, and rejection of vertical motion |
| Session | Pause freezes physics, fuel, and trip; recovery preserves fuel/trip; restart restores initial values |
| Empty recovery | Load a previously absent station, reset the car there, and refuel through the real physics integration |

## Browser observations before presentation refinement

The controller used Chrome 155 at 1280 × 720, ANGLE, NVIDIA RTX 4070 Ti, and D3D11.
These checks used the same simulation and world code as the presentation build.

| Check | Observed result |
| --- | --- |
| Throttle | Five seconds of W reached 84.16 km/h and Z −67.85 m, with four wheel contacts |
| Brake/reverse | Five seconds of S braked, then reached signed −28.21 km/h |
| Steering | A changed X to −14.16 m while coasting |
| Handbrake | Space stopped the car after 2.5 seconds, with four contacts |
| Cameras | C showed the steering wheel and hood; a second C restored Chase |
| Recovery | R restored the car to the road with zero speed |
| Stall | A one-second JavaScript stall added seven steps including the next ordinary frame; contact stayed stable |
| Refuel | At 5.217 m from the station, two seconds of E increased fuel from 11.8866 L to 19.9289 L |
| Pause | Holding W and E for two seconds changed neither position, fuel, trip, nor steps |
| Resume | The car coasted without a stale accelerator input |
| Focus lifecycle | A synthetic window blur cleared input and paused; Resume restored play |
| Fullscreen | Entered and exited; Escape paused the game |
| Restart | Restored origin zero, fuel 12 L, and trip zero |
| Missing WASM | A blocked WASM request showed a readable error and reload instruction; Start stayed disabled |

## Extended browser drive

The controller drove through multiple chunks and lateral terrain bands, then crossed an origin threshold.

- The run reached 3106 meters of trip distance.
- X reached 2184.48 m. Local X became 136.48 m after origin X shifted to 2048 m.
- The car and terrain stayed visible with four wheel contacts.
- The active count stayed at eleven chunks. Observed collider counts ranged from 13 to 61.
- Recovery excluded the 2.4-kilometer teleport from trip distance.
- Native tests bound colliders at 63 and compare shared terrain triangles within 0.001 m.

## Final browser gates

The controller checked the final presentation build after the font fix.

| Check | Status |
| --- | --- |
| Refined scene, start, HUD, pause, cockpit, and narrow layout | Passed; all six views show no new clipping or overlap |
| Simulated unavailable WebGL2 | Passed through the actual loader with a null probe result |
| Blocked WASM request | Passed; readable reload instructions and disabled Start |
| Visible browser collision | Passed against a solid roadside rock; four wheel contacts remained |
| Frame times at 1280 × 720 | Controller measured browser-frame pacing; see below |
| Independent final code review | Pending |

The controller recaptured desktop start, HUD, cockpit, and pause, plus narrow start and pause.
The UI reviewer accepted the font fix and approved these six views.
The browser confirmed that self-hosted Libre Baskerville loaded.

For graphics failure, the controller used an own-page runtime fixture to make the WebGL2 probe return null.
The actual loader showed `This browser cannot use WebGL2` and kept Start disabled.
Reload restored the normal browser API and scene.
This checks the unavailable-graphics error path. It does not test real hardware denial.

For loading failure, the controller blocked `*desert_drive_bg.wasm` through browser network controls.
The actual loader showed `Failed to fetch`, reload instructions, and disabled Start.
The controller cleared the block and reloaded the normal game.

The controller drove into a solid rock using W for two seconds, then D+W for 400 ms, then W for 800 ms.
The car stopped and rebounded at X 3.4736 m, Z −22.307 m, and signed −1.161 km/h.
The wheel-contact mask remained 15, which means all four wheels retained contact.

The controller recorded 720 animation-frame intervals during driving across chunk changes at 1280 × 720.
The sample took 5.0001 seconds and averaged 143.997 frames/s.
Median interval: 6.9 ms. 95th percentile: 7.1 ms. Maximum: 7.2 ms.
Machine: Intel Core i7-11700K and NVIDIA RTX 4070 Ti.
Browser/renderer: Chrome 155, ANGLE, and D3D11.
These measurements describe browser-frame pacing on this machine, not a universal game benchmark.
The sample exceeded the 60 FPS target. It does not measure GPU execution time or long-session performance.
The controller did not run natural tank exhaustion, which requires 20 minutes at full throttle.
Native tests use the real physics integration to check zero-fuel thrust, station recovery, and resumed refueling.
That coverage does not certify a natural browser tank-exhaustion run.

This build has no audio. Audio remains optional under the approved scope.
