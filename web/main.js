const start = document.querySelector('#start');
const screen = document.querySelector('#start-screen');
const status = document.querySelector('#status');
const runtimeError = document.querySelector('#runtime-error');
const canvas = document.querySelector('#game');
const hud = document.querySelector('#hud');
const speed = document.querySelector('#speed');
const cameraMode = document.querySelector('#camera-mode');
const fuel = document.querySelector('#fuel');
const fuelLevel = document.querySelector('#fuel-level');
const trip = document.querySelector('#trip');
const prompt = document.querySelector('#refuel-prompt');
const pauseScreen = document.querySelector('#pause-screen');
const held = new Set();
const keyBits = { KeyW: 1, ArrowUp: 1, KeyS: 2, ArrowDown: 2, KeyA: 4, ArrowLeft: 4,
  KeyD: 8, ArrowRight: 8, Space: 16, KeyE: 32 };
let keys = 0;
let actions = 0;
let lastPaused;
let lastFuel;
let lastTrip;
let lastPrompt;
function clearKeys() { held.clear(); keys = 0; }
function action(value) {
  clearKeys();
  if (value & 13) actions &= ~2;
  actions |= value;
}
function pause() {
  clearKeys();
  if (window.gameState.started || actions & 9) actions = (actions & ~4) | 2;
}
window.gameKeys = () => keys;
window.gameActions = () => { const value = actions; actions = 0; return value; };
let lastSpeed;
let lastCamera;
window.gameState = { speedKmh: 0, camera: 'Chase', x: 0, y: 0, z: 0, contacts: 0, frameSteps: 0, totalSteps: 0,
  activeChunks: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0, originX: 0, originY: 0, originZ: 0,
  rebases: 0, travelM: 0, lateralBand: 0, fuelLiters: 12, paused: false, started: false,
  stationX: 0, stationY: 0, stationZ: 0, stationDistanceM: 0, refuelEligible: false };
window.gameUpdate = (kmh, mode, x, y, z, contacts, steps, chunks, colliders, absoluteX, absoluteY, absoluteZ,
  originX, originY, originZ, rebases, travelM, lateralBand, fuelLiters, paused, started,
  stationX, stationY, stationZ, stationDistanceM, refuelEligible) => {
  const state = window.gameState;
  state.speedKmh = kmh;
  state.camera = mode ? 'Cockpit' : 'Chase';
  state.x = x; state.y = y; state.z = z;
  state.contacts = contacts;
  state.frameSteps = steps;
  state.totalSteps += steps;
  state.activeChunks = chunks;
  state.colliderCount = colliders;
  state.absoluteX = absoluteX; state.absoluteY = absoluteY; state.absoluteZ = absoluteZ;
  state.originX = originX; state.originY = originY; state.originZ = originZ;
  state.rebases = rebases; state.travelM = travelM;
  state.lateralBand = lateralBand;
  state.fuelLiters = fuelLiters; state.paused = paused; state.started = started;
  state.stationX = stationX; state.stationY = stationY; state.stationZ = stationZ;
  state.stationDistanceM = stationDistanceM; state.refuelEligible = refuelEligible;
  const liters = Math.round(fuelLiters * 10);
  const distance = Math.round(travelM / 10);
  if (liters !== lastFuel) {
    fuel.textContent = (liters / 10).toFixed(1);
    fuelLevel.value = liters / 10;
    lastFuel = liters;
  }
  if (distance !== lastTrip) { trip.textContent = (distance / 100).toFixed(2); lastTrip = distance; }
  const message = fuelLiters >= 39.99 ? 'Tank full.' : refuelEligible ? 'Hold E to refuel.' :
    fuelLiters <= 0 ? 'Tank empty. Press R to reach a station.' :
    stationDistanceM <= 20 ? 'Stop near the pump to refuel.' : '';
  if (message !== lastPrompt) { prompt.textContent = message; lastPrompt = message; }
  if (paused !== lastPaused) { pauseScreen.hidden = !paused || !started; lastPaused = paused; }
  const rounded = Math.round(Math.abs(kmh));
  if (rounded !== lastSpeed) { speed.textContent = rounded; lastSpeed = rounded; }
  if (mode !== lastCamera) { cameraMode.textContent = state.camera; lastCamera = mode; }
};
let failed = false;
let ready = false;
let timeout;

function fail(message) {
  failed = true;
  pause();
  clearTimeout(timeout);
  start.disabled = true;
  const text = `Cannot start the game. ${message} Reload to try again.`;
  status.textContent = text;
  if (screen.hidden) {
    runtimeError.textContent = text;
    runtimeError.hidden = false;
  }
}

window.gameReady = () => {
  if (failed) return;
  clearTimeout(timeout);
  ready = true;
  status.textContent = 'The scene is ready.';
  start.disabled = false;
};

window.addEventListener('error', (event) => fail(event.message || 'The runtime stopped.'));
window.addEventListener('unhandledrejection', (event) => {
  fail(event.reason?.message || String(event.reason));
});
canvas.addEventListener('webglcontextlost', (event) => {
  event.preventDefault();
  fail('The graphics context was lost.');
});
start.addEventListener('click', () => {
  if (!ready || failed) return;
  screen.hidden = true;
  hud.hidden = false;
  action(1);
  canvas.focus();
});

window.addEventListener('keydown', (event) => {
  if (!ready || failed || !window.gameState.started) return;
  if (event.code === 'Space' && event.target instanceof HTMLButtonElement) return;
  if (event.code in keyBits) {
    event.preventDefault();
    if (window.gameState.paused || actions & 2) return;
    if (event.repeat && !held.has(event.code)) return;
    held.add(event.code);
    keys |= keyBits[event.code];
  } else if (!event.repeat) {
    if (event.code === 'KeyP' || event.code === 'Escape') {
      event.preventDefault();
      action(window.gameState.paused ? 4 : 2);
      if (window.gameState.paused) canvas.focus();
    } else if (!window.gameState.paused && event.code === 'KeyR') action(16);
    else if (!window.gameState.paused && event.code === 'KeyC') actions |= 32;
  }
});
window.addEventListener('keyup', (event) => {
  if (!(event.code in keyBits)) return;
  event.preventDefault();
  held.delete(event.code);
  keys = 0;
  for (const code of held) keys |= keyBits[code];
});
window.addEventListener('blur', pause);
document.addEventListener('visibilitychange', () => { if (document.hidden) pause(); });
document.querySelector('#pause').addEventListener('click', () => action(2));
document.querySelector('#resume').addEventListener('click', () => {
  if (failed) return;
  action(4); canvas.focus();
});
document.querySelector('#restart').addEventListener('click', () => {
  if (failed) return;
  action(8); canvas.focus();
});
const fullscreen = document.querySelector('#fullscreen');
fullscreen.disabled = !document.fullscreenEnabled;
fullscreen.addEventListener('click', async () => {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.querySelector('main').requestFullscreen();
  } catch { prompt.textContent = 'Fullscreen is unavailable.'; }
});
document.addEventListener('fullscreenchange', () => {
  fullscreen.textContent = document.fullscreenElement ? 'Exit fullscreen' : 'Fullscreen';
});
async function load() {
  const probe = document.createElement('canvas');
  const graphics = probe.getContext('webgl2');
  if (!graphics) throw new Error('This browser cannot use WebGL2.');
  graphics.getExtension('WEBGL_lose_context')?.loseContext();
  const { default: init, run } = await import('./pkg/desert_drive.js');
  status.textContent = 'Loading the game code…';
  const wasm = await fetch('./pkg/desert_drive_bg.wasm');
  if (!wasm.ok) throw new Error(`The game code could not load (HTTP ${wasm.status}).`);
  await init({ module_or_path: wasm });
  status.textContent = 'Preparing the scene…';
  timeout = setTimeout(() => fail('The scene did not start within 60 seconds.'), 60_000);
  run();
}

load().catch((error) => fail(error.message));
