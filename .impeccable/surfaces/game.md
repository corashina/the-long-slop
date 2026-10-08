# Game surface

Mode: Experience.

The user approved a Rust/WASM driving recreation with procedural assets.
The first slice includes car handling, desert roads, two cameras, fuel, and roadside stops.
The user requested implementation and continuation.

## Direction

Refine the existing warm desert scene. Keep the live game view dominant.
Use worn rust paint, sand, a hazy sky, and dark readable controls.
Keep the start instructions, HUD, pause menu, and recovery paths clear.
Preserve physics and the approved scope.

This is code-led refinement of an existing Bevy game. No approved bitmap comp exists.
The approved spec and existing scene define the direction.

## Quality bar

- The player can see the scene before starting.
- The player can read controls and status at desktop and narrow window sizes.
- The cockpit shows the road through the front glass.
- Controls support keyboard focus and loading/error states.
- The graphics use simple original geometry; original Steam assets are outside this slice.
- The target is smooth 60 FPS on the test machine.

## Evidence

The current screenshots are in `.impeccable/review`:
`desktop-start.jpg`, `desktop-hud.jpg`, `desktop-cockpit.jpg`, `desktop-pause.jpg`, `narrow-start.jpg`, and `narrow-pause.jpg`.
The mechanical HTML/CSS/JS detector returned an empty findings array.
The browser collision test stopped the car at a solid roadside rock.
The 720-interval moving sample at 1280x720 measured 143.997 browser frames/second.
Median interval: 6.9 ms. 95th percentile: 7.1 ms. Maximum: 7.2 ms.
Chrome 155 used WebGL2 through ANGLE/NVIDIA RTX 4070 Ti. CPU: Intel i7-11700K.

Natural browser tank exhaustion was not run. Native physics/fuel integration covers zero-fuel behavior.
Audio is optional and omitted from this build.
