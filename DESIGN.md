---
name: My Summer Slop
description: Imported garage preparation with dark readable controls.
colors:
  accent: "#d9ad6f"
  accent-hover: "#eac48e"
  accent-active: "#c79959"
  text: "#f3e5cd"
  muted: "#d6c6ac"
  background: "#26221c"
  panel: "#282218f5"
  button-text: "#211a12"
  secondary: "#514536"
  secondary-hover: "#695a45"
  secondary-active: "#403628"
  fuel-low: "#e98f63"
  error-panel: "#4b211df5"
typography:
  display:
    fontFamily: '"Libre Baskerville", serif'
    fontSize: "38px"
    fontWeight: 400
    lineHeight: 1.1
  title:
    fontFamily: '"Libre Baskerville", serif'
    fontSize: "32px"
    fontWeight: 400
  body:
    fontFamily: "system-ui, sans-serif"
    fontSize: "16px"
    lineHeight: 1.5
  status:
    fontFamily: "system-ui, sans-serif"
    fontSize: "14px"
    lineHeight: 1.5
  label:
    fontFamily: "system-ui, sans-serif"
    fontSize: "12px"
  speed:
    fontFamily: "system-ui, sans-serif"
    fontSize: "32px"
    fontWeight: 500
    lineHeight: 1.1
spacing:
  action-gap: "8px"
  readout-gap: "12px"
  inset: "16px"
  compact-panel-inline: "20px"
  panel-inset: "24px"
  panel-padding: "28px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.button-text}"
    padding: "10px 24px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-primary-active:
    backgroundColor: "{colors.accent-active}"
  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.text}"
    padding: "10px 24px"
  button-secondary-hover:
    backgroundColor: "{colors.secondary-hover}"
  button-secondary-active:
    backgroundColor: "{colors.secondary-active}"
  button-compact:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.text}"
    padding: "8px 12px"
    typography: "{typography.label}"
  controls-panel:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    padding: "28px"
  driving-hud:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    padding: "16px 20px"
  error-panel:
    backgroundColor: "{colors.error-panel}"
    textColor: "{colors.text}"
    padding: "20px"
---

# Design System: My Summer Slop

## Overview

**Creative North Star: "Warm desert driving"**

The player sees a rust-painted car, sand, and a hazy sky through the live game view.
Dark panels keep controls and fuel state readable over the scene.

Use original procedural geometry for the car, road, terrain, and roadside stops.
Use the existing warm palette when adding approved scene details.

**Key Characteristics:**

- Warm sand and worn rust paint.
- Dark rectangular control panels.
- Serif headings and compact numeric readouts.

## Colors

Use sand tones for actions and pale text over dark brown panels.

### Primary

- **Sand accent:** Start and Resume buttons, canvas focus, and the fuel meter.
- **Sand hover and active:** Enabled primary button states.

### Secondary

- **Low fuel orange:** The Chromium meter uses this color below the low fuel threshold.
- **Error brown:** The runtime error panel uses a dark red background.

### Neutral

- **Pale text:** Headings, main readouts, and secondary button text.
- **Muted sand:** Instructions, units, and camera state.
- **Panel brown:** Start, pause, and driving panels.
- **Secondary brown:** HUD buttons, Restart, and the fuel meter track.
- **Dark button text:** Primary button labels and selected text.

**The Scene Palette Rule.** Use warm material colors with the existing gray-blue sky and fog.

Scene materials use sRGB values in `src/presentation/car.rs` and `src/presentation/world.rs`.
The car uses rust paint (0.55, 0.25, 0.13) and faded paint (0.65, 0.41, 0.25).
The terrain uses sand (0.59, 0.44, 0.28) with small spatial color variation.
The road uses brown-gray (0.22, 0.21, 0.18) and sand-colored stripes (0.86, 0.75, 0.48).
The sky and fog share sRGB (0.64, 0.72, 0.73).
These are material inputs. Lighting and Reinhard tonemapping change their rendered appearance.

## Typography

**Display Font:** Libre Baskerville, with a serif fallback.

**Body Font:** The browser system UI font, with a sans-serif fallback.

The local font file is `web/fonts/LibreBaskerville.ttf`.
Its SIL Open Font License is `web/fonts/OFL.txt`.
The stylesheet declares normal weights from 400 through 700 and uses `font-display: swap`.

### Hierarchy

- **Display:** The start heading uses the display token.
- **Title:** The pause heading uses the title token.
- **Body:** Instructions use the browser's normal text size and the body line height.
- **Status:** Loading state uses the status token.
- **Label:** Controls, units, camera state, and fuel readouts use the label token.
- **Speed:** The speed readout uses the speed token.

**The Heading Rule.** Use Libre Baskerville for headings and the system UI font for controls.

Use tabular numbers in the driving HUD. Buttons use weight 600.
The refuel prompt uses 13px text with a 1.5 line height.

## Layout

Use a full viewport canvas (100vw by 100dvh). Overlay the control panels.
The start panel sits at the lower left and has a 370px width limit.
Its horizontal inset uses `clamp(16px, 4vw, 64px)`.
Its bottom inset uses `clamp(16px, 5vh, 48px)`.
Keep its width and height inside the viewport with a 32px total margin.
The start panel scrolls when its content exceeds that height.

The HUD sits 24px from the lower left, with a 330px width limit.
The pause panel sits at the viewport center, with a 360px width limit.
Both width limits reserve 32px for viewport margins.
Action rows wrap with an 8px gap.
Control rows use a two-column grid with a 114px key column and a 12px gap.

At widths of 600px or less, use 24px start panel padding and 16px HUD insets.
Use a 106px key column and an 8px control row gap at that width.

## Elevation & Depth

Use dark translucent backgrounds to separate controls from the game.
The current control panels have no box shadows, blur, or animated state transitions.
Use the existing directional light and linear distance fog for scene depth.
Fog starts at 110 world units and reaches its full value at 430 world units.
Car materials vary roughness and metal content. Glass uses alpha blending.

## Shapes

Use square panel corners and borderless buttons.
The fuel track has square corners and a 6px height.
Use imported home, car, and supply geometry. Keep procedural road and station geometry.
Keep the cockpit road view clear through the front glass.

## Components

### Buttons

Primary buttons use sand fills and dark labels.
Secondary buttons use brown fills and pale labels.
Both use a 44px minimum height and the recorded padding.
HUD buttons use a 38px minimum height and compact padding.
Enabled hover and active states change the fill.
Keyboard focus uses a 2px pale outline with a 3px offset.
Disabled buttons use 0.5 opacity and the default cursor.

### Control Panels

Start and pause panels use the shared dark background and 28px padding.
The start button fills the panel width.
Control descriptions use muted text beside pale key labels.

### Driving HUD

Place speed and camera state on the first row.
Place fuel and trip on the next row, above the fuel meter.
Show the aimed item, fluid amount, condition, and action above the buttons.
Use the shared panel background and compact panel padding.

### Runtime Error

Place the alert 16px from the top and sides.
Use the error background, 20px padding, and a 1.5 line height.
Allow long error text to wrap within the panel.

## Do's and Don'ts

### Do:

- **Do** keep the live scene visible behind controls.
- **Do** use the recorded button focus states.
- **Do** keep HUD numbers tabular.
- **Do** use the local Libre Baskerville font for headings.

### Don't:

- **Don't** substitute procedural assets when the required export is missing.
- **Don't** add interface features outside the approved product scope.


## Reference library availability

The local library includes embedded Liberation Sans Regular at `sharedassets0.assets:769`.
Its 350,200 bytes retain SHA-256 `e5b0af421ea2bfbc1ac8d251d647268087ae82786234c57f757d1f0b90fa8b49`.
Chrome loaded this font in the isolated fixture.
The current shell retains the typography specified above. Pending source UI work uses the recovered font.
The TMP asset's null font pointer does not mean that the installed build lacks font bytes.

The library preserves sprite borders, pivots, atlas links, UI layouts, and native material properties.
These records support later adapters. They do not establish current Unity UI or shader parity.
Six-face cubemap previews use clipped 8-bit PNGs. Native DDS faces retain BC6H HDR data and mip levels.
Read [the asset audit](docs/research/2026-10-10-full-asset-audit.md) for measured coverage and support limits.
