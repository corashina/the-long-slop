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

Use imported home, car, and source pole geometry. Keep procedural road, terrain, and roadside stops.
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

### Scene lighting

Use the installed build's material and lighting records for imported objects.
Keep the existing procedural terrain and road materials.
Source sky, clouds, stars, ambient light, and fog follow the active cycle.
Source lamp choices retain authored material links, emission, cookies, and shadow flags.
Use one `SourceLighting` owner and one shared material cache.
Keep cycle values in persistent uniform buffers. Select prepared lamp handles during updates.
Retain imported scene nodes and shared assets through recovery and restart.
Replace light proxies and road pole instances on restart.
Keep loading markers on warmup instances only.

The renderer uses WebGL2 and a shared 4000-square shadow atlas.
Each shadow cell has a 398-square usable viewport.
Read `docs/verification.md` before claiming source pixel parity.

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
Use source directional illumination and cycle-driven fog for scene depth.
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
Show L as off, low beam, and high beam.

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
