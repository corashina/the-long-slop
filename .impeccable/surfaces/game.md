# Game surface

Mode: Experience.

## Direction

Use the source UI for My Summer Slop's supported garage-to-road features.
Keep the title, fixed Lada, and garage spawn.
Use source menu art, fonts, action icons, fluid text, and vehicle instruments.
Keep the imported scene visible before Start.
Use source values instead of serif headings and a persistent HUD card.
The approved source UI spec and extracted records define visual authority.

## Quality bar

- Show supported actions and actual fluid state.
- Keep menus and controls readable at desktop and narrow window sizes.
- Clear input on pause, capture loss, and focus loss.
- Keep Start disabled when required assets or instrument bindings fail.
- Bind render entities and materials once.
- Preserve recovery state, restart behavior, and the route-test waiver.

## Evidence

Read docs/verification.md for current test results and browser limits.
Native tests cover source input gestures, fluid inputs, instruments, recovery, and rebasing.
Node tests cover menu capture state, input edges, icon precedence, and fluid display.
The in-app browser rejects pointer lock. Do not claim a verified live driving route.
