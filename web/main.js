const start = document.querySelector('#start');
const screen = document.querySelector('#start-screen');
const status = document.querySelector('#status');
const runtimeError = document.querySelector('#runtime-error');
const canvas = document.querySelector('#game');
const hud = document.querySelector('#hud');
const speed = document.querySelector('#speed');
const cameraMode = document.querySelector('#camera-mode');
let started = false;
let lastSpeed;
let lastCamera;
window.gameState = { speedKmh: 0, camera: 'Chase', x: 0, y: 0, z: 0, contacts: 0, frameSteps: 0, totalSteps: 0 };
window.gameStarted = () => started;
window.gameUpdate = (kmh, mode, x, y, z, contacts, steps) => {
  const state = window.gameState;
  state.speedKmh = kmh;
  state.camera = mode ? 'Cockpit' : 'Chase';
  state.x = x; state.y = y; state.z = z;
  state.contacts = contacts;
  state.frameSteps = steps;
  state.totalSteps += steps;
  const rounded = Math.round(Math.abs(kmh));
  if (rounded !== lastSpeed) { speed.textContent = rounded; lastSpeed = rounded; }
  if (mode !== lastCamera) { cameraMode.textContent = state.camera; lastCamera = mode; }
};
let failed = false;
let ready = false;
let timeout;

function fail(message) {
  failed = true;
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
  started = true;
  canvas.focus();
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
