# Source UI and interaction feedback

Date: 2026-10-10, Europe/Warsaw.
Status: The user approved this written spec in chat on 2026-10-10.

## Intent

Match The Long Drive's UI and UX for the features that My Summer Slop supports.
Use installed Steam build `11140890` as the source.
Preserve the project title, fixed Lada, garage spawn, and existing preparation-to-road loop.

The player must see source-style menus, action icons, prompts, fluid text, and working vehicle instruments.
The interface must describe actions that the simulation permits at that moment.
Remove the current brown HUD card, serif headings, constant vehicle readouts, and combined action sentence.

This spec supersedes UI and input clauses in the garage spec where they differ.
Keep its physics, assets, fluid rules, world state, recovery, and restart requirements.
Do not add saves, survival, weapons, other vehicles, multiplayer, audio, or deployment.
Do not add a debug HUD, graphics settings, control rebinding, localization system, or headlight gameplay.

## Reference authority

The [comparison](../research/2026-10-10-ui-reference.md) records the inspected source and current UI.
Required source links include `mainmenuscript`, `menuhandler`, `mainscript`, `settingsscript`, and `tankcapscript`.
Use `fpscontroller` for interaction precedence and `carscript` for instrument signals.
Use `meterscript`, `mutatoscript`, `forgoszamlap`, and `ledkijelzoscript` for instrument presentation.

The source menu uses LiberationSans SDF. Gameplay text uses Unity's built-in Arial.
The TMP asset contains no source font pointer, so it cannot supply a browser font file.
Self-host Liberation Sans from the [upstream font project](https://github.com/liberationfonts/liberation-fonts).
Ship its license and record its version and hash. Use Arial for gameplay text on the Windows target.
Match source font weight, size, alignment, wrapping, and shadow.
Record the browser text renderer as a difference from Unity's SDF renderer.
Remove the obsolete Libre Baskerville file and license when nothing references them.

Source sprites define the menu surfaces and crosshair icons. Do not redraw them from memory.
The main menu uses a `1920 x 1080` reference canvas with Expand scaling.
Resolve RectTransform ancestry and CanvasScaler settings separately for each exported canvas.
Do not apply main-menu scale rules to canvases with different source settings.
Convert bottom-origin Unity coordinates to top-origin browser coordinates once during export.
Keep pivot, anchors, local scale, image borders, sprite crop, and text alignment.

The reference establishes layout and rules. It does not establish identical live Unity rendering.
Do not claim pixel parity without a live original-game comparison.

## Export contract

Extend the existing reference exporter. Keep one export command and one validated manifest.
Add a focused `tools/reference/ui.py` module for UI assets and canvas layout.
Add a focused `tools/reference/instruments.py` module for the selected Lada instrument links.
Keep generated source assets and recovery evidence under ignored `web/assets/reference/`.

Export only the menu art, action icons, layouts, labels, and instrument data used by this scope.
Resolve assets through component pointers. Preserve source file IDs, object IDs, and hashes.
Export the source menu background/frame pairs and select one pair per menu opening, as the original does.
Use the garage as the menu backdrop. Do not build the original separate menu-world scene.
Keep the title My Summer Slop; do not import The Long Drive's title artwork.

Add required UI and instrument records to the current manifest schema.
Records must contain the source layout values, sprite paths, crop/border data, and instrument node IDs.
Instrument records must include signal, input range, angle range, axis, rest rotation, smoothing, and materials.
Export odometer digit-node links and the source digit mapping.
Export fluid presentation fields `hasName`, name, and `noText` for relevant caps.
Keep capacity in the existing tank records and link it through stable IDs.

Validate finite values, ranges, source links, hashes, and referenced node and material IDs.
Required missing data must stop export or loading with the failed field or asset name.
Replace native fixtures with the new schema. Do not accept older exports or add defaults for missing records.
Write the acceptance manifest only after all required exports and validation succeed.

## Components and data flow

Retain HTML controls over the Bevy canvas. Add no UI framework or runtime package.
Keep Rust responsible for simulation state and available actions.
Keep JavaScript responsible for menu state, input capture, and DOM presentation.
Keep Bevy responsible for imported vehicle instruments.

Use these focused components:

| Component | Responsibility | Inputs |
| --- | --- | --- |
| `src/interaction/feedback.rs` | Compute available actions and feedback from the current aim and held item | Interaction state, player, items, fluids, and reference data |
| `src/browser.rs` | Publish fixed feedback fields through the WASM bridge | Feedback record and runtime state |
| `web/menu.js` | Show menus and process Start, Continue, Restart, Recover, and Fullscreen | Loading state, runtime state, and capture result |
| `web/input.js` | Collect held, pressed, released, mouse, and scroll input | Browser events and menu/capture state |
| `web/feedback.js` | Update source action rows, icons, fluid text, and carrying help | Fixed feedback fields and exported UI data |
| `src/presentation/instruments.rs` | Update cached needle, lamp, and odometer entities | Vehicle speed, fluids, temperature, ignition, RPM, and trip |
| `web/main.js` | Load the export and game, then connect the components | Required assets and module exports |

Use a typed Rust action bitmask and a small context enum, rather than parsing action strings in JavaScript.
Include use, door adjustment, pickup, drop, mount, dismount, fill, cap tuning, seat entry, and seat exit.
Include blocked-action notices separately from available-action rows.
Publish fluid amount, capacity, six fluid quantities, and source presentation flags.
Use static labels for supported action names. Publish dynamic names only when their values change.
Keep existing simulation diagnostics available for native tests without displaying them during normal play.

Use the same action conditions in simulation and feedback. Do not duplicate permissibility rules in JavaScript.
Build entity and source-ID lookup tables once during scene loading.
Allocate action elements, formatting caches, and instrument bindings once.
Write DOM text only when the displayed value changes. Format mixtures only when displayed quantities change.
Avoid per-frame collection creation, entity searches, material clones, and event-listener setup.

## Menus and browser states

Keep the loaded garage visible behind the menu.
The start menu offers Start and Controls. Show loading and asset errors within this surface.
Keep Start disabled until the export, scene, and instrument bindings pass validation.
Use the source new-game slot for Start and the source Settings slot for Controls.
Remove buttons for unsupported original features rather than showing inactive actions.

The pause menu offers Continue, Controls, Restart, and Recover.
Use source menu button geometry and artwork for these supported actions.
Keep source anchors for Continue and Controls. Use vacant source menu slots for Restart and Recover.
Show trip distance on the menu using the source distance-label style.
Label it as session distance; the game has no persistent distance history.

The Controls screen separates on-foot and driving bindings using the source controls-list layout.
List only supported bindings. Include Fullscreen here as a browser action.
Provide Back to return to the originating menu.
Fullscreen failure must leave the controls screen open and show a short error there.

Use explicit loading, start-menu, playing, pause-menu, controls, and error states.
Record whether Controls opened from start or pause. Opening Controls must not start or resume simulation.
Esc returns from Controls to its originating menu. Esc pauses active play.
Continue and Start require a user gesture and successful pointer capture.
Capture failure keeps the originating menu visible and preserves a usable retry button.
Do not add a capture bypass or alternate input runtime.

Focus loss, document hiding, or capture loss pauses play and clears held and pending input.
Restart restores the garage through the existing reset action, then captures input through the same guarded path.
Recover uses the existing state-preserving recovery action while paused and keeps the pause menu open.
Continue resumes after recovery. Recovery must not invoke restart or clear the trip.

Use one DOM state transition function to hide stale gameplay feedback and restore keyboard focus.
Keep semantic buttons, tab order, visible focus, and existing loading/error announcements.
Do not trap focus on the canvas while a menu remains open.
Scale source layout within the viewport and permit controls-list scrolling in narrow or short windows.
Do not add touch controls or reduced-motion variants.

## Interaction feedback and input

Place action rows at the source lower-left anchors and carrying help at the lower right.
Use source `[key]: action` formatting, pale text, and black shadow.
Show each row only when its action is available.
Match `mainscript.CrossHairStuff()` icon precedence for supported states.
Use the default reticle when no supported context icon applies.
Hide gameplay feedback in menus and the chase view.
Hide ordinary target names and condition percentages unless the source's supported fluid display requires a name.
Show a blocked seat-exit or failed preparation notice without advertising that blocked action as available.

Use E for use, door adjustment, and seat entry/exit.
Use F for pickup, drop, mount, and dismount, with the source action precedence.
Target the tire or rim's source component when removing it from a wheel assembly.
Use the same F mount/dismount flow for tire/rim joining and splitting. Remove the separate X assembly action.
Use press for pickup/dismount and release for mounting, matching the recovered source flow.
Prevent the release after a new pickup from mounting or dropping that item again in the same gesture.

Use RMB with mouse movement to move a carried item and MMB with mouse movement to rotate it.
Retain scroll for the source carrying and valve-tuning paths where they apply.
Suppress ordinary view movement while the selected object manipulation consumes the mouse.
Keep Hold LMB for filling, I for ignition/starter, Space for seated handbrake, and C for seated camera.
Use WASD for walking, Shift for running, Ctrl for crouching, and Space for jumping.
Remove on-foot arrow movement aliases that do not appear in the recovered defaults.
Remove P pause and the R recovery shortcut. Recovery remains available in the pause menu.
Use Esc for pause and Backspace to hide/show gameplay feedback, as in the source.
Keep the HUD visibility choice for the session; restart must not change this interface preference.

## Fluid display

Place tank text at the source below-center anchor, with its green color, shadow, and font size.
Show it when the aimed fluid cap permits text and first-person gameplay feedback is visible.
Use the linked tank capacity, not the car's fuel capacity for every container.
Display Empty when the amount is at most `0.05 L`.
Otherwise, display amount/capacity or Full when the one-decimal formatted values match.
Remove a trailing `.0` from formatted volume.
Display each mixture component at one percent or more, rounded to a whole percent.
Preserve the source fluid ordering and names. Do not show component liters in the normal fluid display.
Use source `hasName` and `noText` behavior.
Keep the unlimited-pump notice separate from the finite-container display.

## Lada instruments

The selected instrument is `level1:78501`, under `Car02Full/Slots/KMHSlot/LadaKmh`.
The selected Lada contains three needles and one odometer. Do not create an RPM needle.

| Needle component | Transform | Input range | Unity Y-angle range |
| --- | --- | --- | --- |
| Speed `71099` | `34042` | 0 to 160 km/h | -64 to 64 degrees |
| Temperature `77005` | `34043` | 10 to 130 degrees Celsius | -30 to 30 degrees |
| Fuel `72439` | `34041` | 0 to 39 L | -26 to 26 degrees |

Read speed from the vehicle's existing speed signal and temperature from `EngineOutput.temperature`.
Fuel and temperature target zero with ignition off, as in `carscript.METERUPD()`.
They use the source first smoothing stage at `5 * dt`.
The selected needle scripts use ordinary value smoothing at `8 * dt`.
A serialized zero smoothing multiplier becomes one, following `mutatoscript.Start()`.
Clamp interpolation factors and input ranges as the source does.
Apply the existing handedness conversion to local rotations; do not rotate a converted node with unconverted Unity angles.

The odometer component is `level1:76207` and the meter uses `kilometerszamlalo`.
Drive its digit nodes through recovered `forgoszamlap` rules and existing session travel distance.
Restart returns it to the recovered initial value. Recovery and origin rebasing preserve it.

Use the source lamp material links and `carscript.LEDUPD()` rules for supported signals.
The source fuel threshold is `5 L`; the oil threshold is `2 L`.
Oil and battery test thresholds are `300 RPM` and `800 RPM`.
Retain source ignition and electronics gating rather than displaying these thresholds without their guards.
Keep the unsupported headlight signal off. Do not add headlight input or simulated lighting to this stage.
Preload required lamp materials and switch cached handles only when lamp state changes.
Bind instruments beneath the vehicle hierarchy so recovery and origin rebasing preserve their relative poses.

## Delivery layers and verification

Each layer must leave the game build usable before adding the next layer.

1. Export and validate source UI assets, layout, fluid flags, and instrument links.
2. Replace menus and the browser state flow while keeping the existing simulation usable.
3. Add typed interaction feedback and source input bindings, then replace the old HUD.
4. Bind and animate the Lada instruments. Remove obsolete UI files and bridge fields.

Add critical tests for action precedence, F press/release behavior, capture-state input clearing, and menu transitions.
Test Empty/Full thresholds, mixture percentage filtering, and tank-specific capacity.
Test needle endpoints, ignition gating, warning lights, odometer rollover, restart, recovery, and rebase preservation.
Validate real exported source links as well as synthetic native fixtures.
Run the project's formatting, native tests, release WASM check, exporter tests, and browser package build.

Inspect desktop and narrow-window menu layout and static feedback states against the extracted source records.
Keep source-derived values and screenshot evidence separate from live original-game parity claims.
The user previously waived the final continuous browser garage-to-road test; this spec preserves that waiver.
Do not repeat that route test or add a pointer-lock workaround.
Record the in-app browser's capture limitation and any resulting visual verification gaps.

Update PRODUCT.md, DESIGN.md, README controls, the game surface brief, and verification notes after implementation.
Keep these records consistent with the final components, bindings, source values, and remaining limits.
