# Test sandbox scripts

Claude runs the ROADMAP test pass with these in Chrome. They are not automatic tests.

1. Open the game with `?sandbox`. Add `&cars=Bus01,Bus02` for cars in front of the garage car, `&date=11-01` for a fixed date.
2. Paste `helpers.js` into the game tab. Start a New game. Then paste one area file, for example `buses.js`.
3. Call the steps. `window.sandbox` has `teleport(x, z)`, `garage()`, `building(name)`, `spawn(name)`, `hand(name)`, `fill(fluid, liters)`, `cycle(seconds)` and `state(radius)`.
4. `__run` holds each step with the sandbox state and F3. Write it to the run report with one screenshot per step.
