# UI comparison

Checked on 2026-10-10. This report records analysis, not implementation.

## Scope

The user requested the original UI and UX for supported game features.
These include start, pause, controls, interaction prompts, fluids, and driving feedback.
The approved garage spawn and fixed Lada remain in scope.
Save/load, survival, other vehicles, multiplayer, audio, and deployment remain outside scope.

## Evidence

Reference installation: `C:\Program Files (x86)\Steam\steamapps\common\The Long Drive`.
The project selects Steam build `11140890`.
The current assembly SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.

I inspected recovered `mainscript`, `fpscontroller`, `tankcapscript`, `carscript`, and `settingsscript` code.
I recovered `mainmenuscript`, `menuhandler`, and `ls` with the existing ILSpy library.
I read Unity UI components with the project's validated `Source` reader.
The reader checks full byte consumption and MonoBehaviour header pointers.

Ignored research files:

- `.cache/reference/ui-layout.json`: serialized UI fields, colors, fonts, and RectTransforms.
- `.cache/reference/ui-assets.txt`: resolved font and sprite identities, menu ancestry, and meter components.
- `.cache/reference/mainmenuscript.cs`, `menuhandler.cs`, and `ls.cs`: recovered code.

I inspected the current start screen at `http://127.0.0.1:4173/` in the in-app browser.
It loaded the garage and enabled Start.
Start failed because this browser rejected pointer lock for the document.
This limits live gameplay inspection in this browser.
The existing `.cache/garage-clean.png` shows the gameplay HUD, but it is an earlier capture.
I compared that capture with the current HTML, CSS, JavaScript, and Rust bridge.
I did not run the original game or claim a pixel comparison with its live output.
Concurrent workspace changes renamed the game to My Summer Slop during this analysis.
That title change does not alter the layout findings. UI work must preserve the current project title.

## Comparison

| Surface | Current game | Recovered original | Gap |
| --- | --- | --- | --- |
| Start | Lower-left brown panel, serif heading, full control list | Separate main menu with large sans-serif buttons and submenus | Replace layout and move controls into a separate screen |
| Pause | Centered card with Resume and Restart | Menu overlay with Continue and separate Settings screen | Match supported menu actions and overlay layout |
| Fonts | Libre Baskerville and system UI | LiberationSans SDF for menu text; Arial for gameplay text | Replace serif and system styling with verified font styling |
| Menu art | Flat CSS panel | Serialized background and frame sprites | Export the source sprites |
| Crosshair | Fixed plus sign | Default reticle and context icons for use, pickup, mounting, seating, and filling | Export icons and use action state to select them |
| Prompts | One combined sentence inside the HUD panel | Separate lower-left `[key]: action` lines | Publish independent available actions |
| Fluid text | Liters per fluid beside the target label | Amount/capacity and mixture percentages below center | Match formatting, color, and visibility |
| Carry feedback | Always shows a carried-item label | Shows movement and rotation help while carrying | Show context help and match mouse controls |
| Driving | Constant speed, fuel, trip, RPM, and handbrake panel | Uses vehicle instruments; screen speed is a debug option | Animate imported instruments and remove the normal debug panel |
| Condition | Shows a target condition percentage | Provides a separate optional statistics display | Remove condition from normal interaction feedback |
| Browser actions | Pause and Fullscreen buttons during play | Uses the pause menu and desktop window controls | Put browser actions inside the menu |

## Serialized visual values

`mainmenuscript` is `level0:16795`. `menuhandler` is `level1:74691`.

The main menu canvas uses a `1920 x 1080` reference resolution.
Its CanvasScaler uses Expand mode.
The menu Continue button measures `350 x 90` reference units.
The Settings button measures `340 x 88` reference units.
Menu text resolves to `sharedassets0.assets:1366`, named `LiberationSans SDF`.
Gameplay text resolves to Unity's built-in Arial font.

The pause overlay color is RGBA `(180, 128, 70, 64)`.
Its frame uses `(180, 128, 70, 255)`.
The serialized overlay sprite is `sharedassets1.assets:4562`, named `ingamemenubg01`.
`menuhandler.RandomImage()` selects a background/frame pair from the serialized sprite arrays.
One serialized sprite does not establish the only runtime appearance.

The default crosshair uses `sharedassets1.assets:4540`, named `default`.
Its size is `35 x 35` reference units, with alpha `160/255`.
Use and fill icons measure `40 x 40` reference units.
Pickup and dismount icons measure `75 x 40` reference units.
`mainscript.CrossHairStuff()` selects icons from available actions and applies precedence rules.

The use prompt uses size `40` and white text.
The pickup prompt uses size `40` and RGB `(210, 210, 210)`.
Their source anchors place them at the lower left.
Carrying help uses the lower right.
Tank text uses size `55`, RGB approximately `(167, 255, 0)`, and a black shadow.
Its anchor is `(0.5, 0.35)` in Unity's bottom-origin coordinates.
This places it at 65 percent of the viewport height from the top.
The implementation must resolve parent transforms before using these values as screen pixels.

## Behavior findings

`mainscript` displays action prompts only when the action is available and help is enabled.
It hides gameplay prompts during menu states and alternate camera views.
It displays carrying help only while the player carries an item.
The current bridge supplies one action string, so it cannot represent the original action rows or icon precedence.

`tankcapscript.GetCapString()` displays Empty at amounts of `0.05 L` or less.
Otherwise, it displays amount/capacity or Full when the formatted amounts match.
It formats volume to one decimal place and removes a trailing `.0`.
It displays mixture components at one percent or more, rounded to whole percentages.
It can suppress text through `noText` and add a localized container name through `hasName`.
The current bridge supplies fluid amounts but omits capacity and these presentation flags.

Original default bindings include E for use, F for pickup/mounting, RMB for moving, and MMB for rotating.
The current game uses RMB for rotating and scroll for carry distance.
The current P pause binding also conflicts with the original P survival action.
The original uses Esc for its menu and Backspace to hide the HUD.
The later targeted trace found F press for dismount and F release for mount in `fpscontroller`.
Its part-slot flow also covers tire/rim assembly. The separate X assembly action has no matching source binding.

`carscript.METERUPD()` drives speed, fuel, temperature, and RPM instruments.
Fuel and temperature readings depend on ignition and use time-based smoothing.
`METERUPD2()` updates warning lights and distance counters.
The current presentation code does not update these instruments.
The later targeted extraction resolved Lada meter `level1:78501` and its three needles.
Speed uses transform `34042`, range `0..160 km/h`, and Unity Y angles `-64..64` degrees.
Temperature uses transform `34043`, range `10..130` degrees Celsius, and Y angles `-30..30` degrees.
Fuel uses transform `34041`, range `0..39 L`, and Y angles `-26..26` degrees.
The Lada has no RPM needle. Its odometer component is `level1:76207`.
Fuel, oil, and battery warning thresholds are `5 L`, `2 L`, `300 RPM`, and `800 RPM`, with source guards.
The complete extracted records are in ignored `.cache/reference/lada-ui.json`.

The source TMP menu font has a null `m_SourceFontFile` pointer.
Its LiberationSans SDF asset cannot supply a browser font file.
The design requires self-hosted Liberation Sans and records the text-renderer difference.

## Recommended direction

Use the source layout, sprites, fonts, and behavior for supported actions.
A CSS-only reskin would leave prompts, input bindings, fluid formatting, and instruments different.

Extend the existing reference exporter with the required UI assets and instrument links.
Keep generated original assets outside Git and record their source identities and hashes.
Replace the web shell with separate menu, controls, and gameplay feedback components.
Publish typed action flags and fluid presentation data from Rust.
Bind instrument nodes once during scene loading.
Update existing nodes without per-frame setup or collection allocation.

Retain browser loading, pointer-lock errors, focus-loss pause, recovery, and restart.
List browser-only actions in the menu and label them accurately.
Keep unsupported original menu actions absent.

Verification must cover menu transitions, input clearing, available-action visibility, fluid text, and instrument ranges.
Compare desktop and narrow-window screenshots with extracted source layout values.
Verify pointer lock and the full garage-to-driving route in a browser that supports capture.
