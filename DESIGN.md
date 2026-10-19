---
name: My Summer Slop
description: Source menus and feedback over the imported garage.
colors:
  text: "#ffffff"
  prompt: "#d2d2d2"
  controls: "#c8c8c8"
  fluid: "#a7ff00"
  menu: "#b4804640"
  frame: "#b48046"
  shadow: "#000000"
  error: "#4b211d"
typography:
  menu:
    fontFamily: '"Liberation Sans", sans-serif'
    fontWeight: 400
  gameplay:
    fontFamily: 'Arial, sans-serif'
    fontWeight: 400
  heading:
    fontFamily: '"Liberation Sans", sans-serif'
    fontSize: "clamp(24px, 3.2vw, 60px)"
    fontWeight: 400
  narrow-controls:
    fontFamily: 'Arial, sans-serif'
    fontSize: "16px"
    lineHeight: 1.5
spacing:
  inset: "16px"
  action-gap: "12px"
  control-gap: "16px"
  row-gap: "8px"
components:
  menu-button:
    typography: "{typography.menu}"
  fullscreen:
    backgroundColor: "{colors.menu}"
    textColor: "{colors.text}"
    padding: "8px 16px"
    height: "44px"
  fluid:
    textColor: "{colors.fluid}"
    typography: "{typography.gameplay}"
  feedback:
    textColor: "{colors.prompt}"
    typography: "{typography.gameplay}"
---

# Design System: My Summer Slop

## Overview

**Creative North Star: "The Long Drive source UI"**

Use installed Steam build 11140890 and the approved source UI spec.
Keep the project title and garage backdrop.
The exporter supplies sprites, layout records, colors, and font sizes.

**Key Characteristics:**

- Source menu art over the live scene.
- Contextual action rows and fluid text.
- Instruments on the imported Lada dashboard.

## Colors

Use source sprite tints and pale text over the scene.
Use bright green for fluid text and source action labels.
Use black text shadows for contrast.
The error panel uses the recorded dark red.

### Scene lighting

Use the installed build's material and lighting records for imported objects.
Use checked source TerrainBiome equations and source road material bindings.
Source sky, clouds, stars, ambient light, and fog follow the active cycle.
Source lamp choices retain authored material links, emission, cookies, and shadow flags.
Use one `SourceLighting` owner and one shared material cache.
Keep cycle values in persistent uniform buffers. Select prepared lamp handles during updates.
Retain imported scene nodes and shared assets through recovery and restart.
Replace light proxies and road pole instances on restart.
Keep loading markers on warmup instances only.

The renderer uses WebGL2. Only the sun casts shadows, through its four-cascade atlas.
Lamps light receivers without shadow maps or shadow caster copies.
Read `docs/verification.md` before claiming source pixel parity.

## Typography

Use self-hosted Liberation Sans 2.1.5 for menus. Keep its license and provenance.
Use Arial for gameplay text and controls lists.
Resolve source font sizes through canvas and parent scales.
Use the heading clamp for the project title and Controls heading.
Use 16-pixel controls text in narrow windows.

Browser text rendering differs from Unity's SDF renderer.

## Layout

Resolve source anchors, pivots, scales, and canvas settings at resize.
Use the recorded 1920 by 1080 canvas and Expand scaling.
Use source action-left, key-right controls lists.
At widths below 800 pixels, stack controls and keep lists scrollable.
Use 44-pixel menu buttons, a 16-pixel inset, and a 12-pixel gap in narrow windows.

Place action rows at lower left and carrying help at lower right.
Keep all carrying lines above the source bottom margin.
Center fluid text below the center icon.
Hide feedback in menus and chase view.

## Elevation & Depth

Source sprite pixels provide button depth.
Tint them once during loading. Preserve alpha and source color.
Use a black text shadow of 2px 2px 2px.
Use 3px 3px 2px for fluid text.
Keep keyboard focus visible with a two-pixel white outline.
Use source directional illumination and cycle fog for scene depth.

## Shapes

Use source button, background, frame, and icon images.
Keep their serialized geometry and authored silhouettes.
Fullscreen uses a plain rectangular border.

## Components

### Menus

Choose a source background/frame pair when opening the start or pause menu.
Show Start and Controls before play.
Show Continue, Controls, Restart, and Recover while paused.
Put Fullscreen in Controls. Keep loading and capture errors readable.

Source buttons brighten on hover and darken when pressed.
Disabled buttons use 45 percent opacity.

### Feedback

Show available actions only.
Mount replaces Drop at a compatible slot.
Backspace changes feedback visibility for the session. Restart preserves that choice.

Show source container names, amount/capacity, and whole mixture percentages.
Use original English labels Gas, empty, and (full).
Filter components below one percent.
Keep tank-specific capacity and source noText/hasName flags.

### Instruments

Bind Lada speed, temperature, and fuel needles through source node IDs.
Use source ranges and smoothing.
Update local rotations with one handedness conversion.
Keep the five-digit odometer and warning lights on the imported dashboard.
Cache entities and materials during loading.
Change lamp handles only when their state changes.

## Do's and Don'ts

- Use exported source UI records for supported actions.
- Check required assets and bindings before Start.
- Preserve the approved route-test waiver.
- Keep the live scene visible behind controls.
- Keep keyboard focus visible.
- Use tabular HUD numbers.
- Keep menus within the approved product scope.
- Keep the Lada dashboard free of an RPM needle.

## Reference library availability

The local library includes embedded Liberation Sans Regular at `sharedassets0.assets:769`.
Its 350,200 bytes retain SHA-256 `e5b0af421ea2bfbc1ac8d251d647268087ae82786234c57f757d1f0b90fa8b49`.
Chrome loaded this font in the isolated fixture.
The current shell uses the self-hosted Liberation Sans version specified above.
The TMP asset's null font pointer does not mean that the installed build lacks font bytes.

The library preserves sprite borders, pivots, atlas links, UI layouts, and native material properties.
The current UI uses selected source records. Complete library preservation does not establish Unity UI or shader parity.
Six-face cubemap previews use clipped 8-bit PNGs. Native DDS faces retain BC6H HDR data and mip levels.
Read [the asset audit](docs/research/2026-10-10-full-asset-audit.md) for measured coverage and support limits.

## World ownership

Use one `WorldStream` and one current source loader contract.
Keep the frozen source generation frame separate from runtime origin shifts.
Select bounded windows around the player and current vehicle. Deduplicate shared regions.
Prepare required render and collision owners before publication or recovery transport.
Keep the accepted region when admission fails.

Keep imported assets separate from world prototypes, active geometry, pending work, source history, and actual changes.
Count retained container capacities. Count shared GPU handles once.
Use the existing `SourceLighting` API for terrain illumination.
Source global reflection and SH-volume contributions remain outside this world layer.
Keep technical measurements in the development snapshot. Add no player memory or world-quality controls.
