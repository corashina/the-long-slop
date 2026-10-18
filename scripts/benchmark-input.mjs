// Browser-only input control for matched benchmark sessions. Call restore() after use.
import {ACTIONS} from "/menu.js";
export function createBenchmarkInput() {
  const fields = {gameActions: 'actions', gameKeys: 'keys', gamePressed: 'pressed', gameReleased: 'released', gameMouseX: 'mouseX', gameMouseY: 'mouseY'};
  const original = Object.fromEntries(Object.keys(fields).map(name => [name, window[name]]));
  const input = {actions: 0, keys: 0, pressed: 0, released: 0, mouseX: 0, mouseY: 0, autoResume: false};
  window.gameKeys = () => { original.gameKeys(); return input.keys; };
  window.gameActions = () => {
    const explicit = input.actions, queued = original.gameActions();
    input.actions = 0;
    if (explicit) return explicit;
    return input.autoResume ? (window.gameState.paused ? ACTIONS.CONTINUE : queued & ~ACTIONS.PAUSE) : queued;
  };
  for (const host of ['gamePressed', 'gameReleased', 'gameMouseX', 'gameMouseY']) {
    const field = fields[host];
    window[host] = () => { const value = input[field]; input[field] = 0; return value; };
  }
  const overlays = ['#menu', '#performance'].map(selector => document.querySelector(selector));
  const display = overlays.map(node => node.style.display);
  for (const node of overlays) node.style.display = 'none';
  input.restore = () => {
    for (const host of Object.keys(fields)) window[host] = original[host];
    overlays.forEach((node, index) => { node.style.display = display[index]; });
  };
  return input;
}

