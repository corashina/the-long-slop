const start = document.querySelector('#start');
const screen = document.querySelector('#start-screen');
const status = document.querySelector('#status');
const runtimeError = document.querySelector('#runtime-error');
const canvas = document.querySelector('#game');
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
