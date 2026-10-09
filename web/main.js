const canvas = document.querySelector('#game');
const start = document.querySelector('#start');
const screen = document.querySelector('#start-screen');
const status = document.querySelector('#status');
const runtimeError = document.querySelector('#runtime-error');
const hud = document.querySelector('#hud');
const pauseScreen = document.querySelector('#pause-screen');
const captureError = document.querySelector('#capture-error');
const prompt = document.querySelector('#action-prompt');
const target = document.querySelector('#target');
const detail = document.querySelector('#target-detail');
const heldLabel = document.querySelector('#held');
const engineLabel = document.querySelector('#engine');
const speed = document.querySelector('#speed');
const cameraMode = document.querySelector('#camera-mode');
const fuel = document.querySelector('#fuel');
const fuelLevel = document.querySelector('#fuel-level');
const trip = document.querySelector('#trip');
const crosshair = document.querySelector('#crosshair');
const heldCodes = new Set();
const keyBits = { KeyW: 1, ArrowUp: 1, KeyS: 2, ArrowDown: 2, KeyA: 4, ArrowLeft: 4,
  KeyD: 8, ArrowRight: 8, Space: 16, KeyE: 32, ShiftLeft: 64, ShiftRight: 64,
  ControlLeft: 128, ControlRight: 128, KeyF: 256, Mouse0: 512, KeyI: 1024, KeyC: 2048, Mouse2: 4096, KeyX:8192 };
let keys = 0;
let pressed = 0;
let released = 0;
let mouseX = 0;
let mouseY = 0;
let wheel = 0;
let actions = 0;
let failed = false;
let ready = false;
let timeout;
let lastPaused;
let lastFuelNumber;
let lastTripNumber;
let lastSpeedNumber;
let lastAmount;
let lastCondition;
let lastHeld;
let lastRpm;
let lastIgnition;
let lastRunning;
let lastStarter;
let lastHandbrake;
const lastVolumes=[-2,-2,-2,-2,-2,-2];
const fluidNames=['gasoline','oil','alcohol','water','blood','diesel'];
const cached = new Map();
function write(element, value) {
  if (cached.get(element) === value) return;
  element.textContent = value;
  cached.set(element, value);
}
function clearInput() {
  heldCodes.clear(); keys = 0; pressed = 0; released = 0; mouseX = 0; mouseY = 0; wheel = 0;
}
function setCode(code, down) {
  const old = keys;
  if (down) heldCodes.add(code); else heldCodes.delete(code);
  keys = 0;
  for (const heldCode of heldCodes) keys |= keyBits[heldCode];
  pressed |= keys & ~old;
  released |= old & ~keys;
}
function captured() { return document.pointerLockElement === canvas; }
function pause() {
  clearInput();
  if (window.gameState.started || actions & 13) actions = (actions & ~13) | 2;
  if (captured()) document.exitPointerLock();
}
async function capture(action) {
  if (!ready || failed) return;
  clearInput();
  captureError.textContent = '';
  try {
    canvas.focus();
    await canvas.requestPointerLock();
    if (!captured()) throw new Error('The browser did not capture the mouse.');
    actions = (actions & ~2) | action;
    screen.hidden = true;
    hud.hidden = false;
  } catch (error) {
    pause();
    const message = `Mouse capture failed. ${error.message} Select Resume to try again.`;
    captureError.textContent = message;
    if (!window.gameState.started) status.textContent = message.replace('Resume', 'Start');
    else pauseScreen.hidden = false;
  }
}
window.gameKeys = () => keys;
window.gamePressed = () => { const value = pressed; pressed = 0; return value; };
window.gameReleased = () => { const value = released; released = 0; return value; };
window.gameMouseX = () => { const value = mouseX; mouseX = 0; return value; };
window.gameMouseY = () => { const value = mouseY; mouseY = 0; return value; };
window.gameWheel = () => { const value = wheel; wheel = 0; return value; };
window.gameActions = () => { const value = actions; actions = 0; return value; };
window.gameState = { speedKmh: 0, camera: 'First person', x: 0, y: 0, z: 0, contacts: 0,
  frameSteps: 0, totalSteps: 0, activeChunks: 0, colliderCount: 0, absoluteX: 0, absoluteY: 0, absoluteZ: 0,
  originX: 0, originY: 0, originZ: 0, rebases: 0, travelM: 0, lateralBand: 0, fuelLiters: 0,
  paused: false, started: false, target: '', targetAction: '', heldItem: '', rpm: 0, ignition: false,
  running: false, starter: false, seated: false, handbrake: true, fluidVolumes:[0,0,0,0,0,0] };
window.gameUpdate = (kmh, mode, x, y, z, contacts, steps, chunks, colliders, absoluteX, absoluteY, absoluteZ,
  originX, originY, originZ, rebases, travelM, lateralBand, fuelLiters, paused, started) => {
  const state = window.gameState;
  state.speedKmh = kmh; state.camera = mode ? 'First person' : 'Chase';
  state.x = x; state.y = y; state.z = z; state.contacts = contacts;
  state.frameSteps = steps; state.totalSteps += steps; state.activeChunks = chunks; state.colliderCount = colliders;
  state.absoluteX = absoluteX; state.absoluteY = absoluteY; state.absoluteZ = absoluteZ;
  state.originX = originX; state.originY = originY; state.originZ = originZ;
  state.rebases = rebases; state.travelM = travelM; state.lateralBand = lateralBand;
  state.fuelLiters = fuelLiters; state.paused = paused; state.started = started;
  const roundedSpeed=Math.round(Math.abs(kmh));
  if (roundedSpeed!==lastSpeedNumber) {write(speed,String(roundedSpeed));lastSpeedNumber=roundedSpeed;}
  write(cameraMode, state.camera);
  const liters = Math.round(fuelLiters * 10) / 10;
  if (liters!==lastFuelNumber) {write(fuel,liters.toFixed(1));fuelLevel.value=liters;lastFuelNumber=liters;}
  const distance=Math.round(travelM / 10) / 100;
  if (distance!==lastTripNumber) {write(trip,distance.toFixed(2));lastTripNumber=distance;}
  if (lastPaused !== paused) { pauseScreen.hidden = !paused || !started; lastPaused = paused; }
  crosshair.hidden = paused || !started || !mode;
};
window.gameDetail = (name, action, amount, condition, held, rpm, ignition, running, starter, seated, handbrake, gasoline,oil,alcohol,water,blood,diesel) => {
  const state = window.gameState;
  state.target = name; state.targetAction = action; state.heldItem = held;
  state.rpm = rpm; state.ignition = ignition; state.running = running;
  state.starter = starter; state.seated = seated; state.handbrake = handbrake;
  const volumes=state.fluidVolumes;
  volumes[0]=gasoline;volumes[1]=oil;volumes[2]=alcohol;volumes[3]=water;volumes[4]=blood;volumes[5]=diesel;
  write(target, name);
  write(prompt, action);
  const roundedAmount=Math.round(amount*10)/10;
  const roundedCondition=Math.round(condition*100);
  let changed=roundedAmount!==lastAmount || roundedCondition!==lastCondition;
  for (let i=0;i<6;i++) {const rounded=Math.round(volumes[i]*10)/10;if (rounded!==lastVolumes[i]) {lastVolumes[i]=rounded;changed=true;}}
  if (changed) {
    if (name==='Gasoline pump') write(detail,'Unlimited gasoline');
    else if (amount>=0) {
      const parts=[];
      for (let i=0;i<6;i++) {if (volumes[i]>0.0001) parts.push(`${lastVolumes[i].toFixed(1)} L ${fluidNames[i]}`);}
      write(detail,parts.length ? parts.join(' · ') : 'Empty');
    } else write(detail,condition>=0 ? `Condition ${roundedCondition}%` : '');
    lastAmount=roundedAmount;lastCondition=roundedCondition;
  }
  if (held!==lastHeld) {write(heldLabel,held ? `Carrying ${held}` : '');lastHeld=held;}
  const roundedRpm=Math.round(rpm);
  if (roundedRpm!==lastRpm || ignition!==lastIgnition || running!==lastRunning || starter!==lastStarter || handbrake!==lastHandbrake) {
    write(engineLabel,`${starter ? 'Starter' : running ? 'Engine running' : ignition ? 'Ignition on' : 'Ignition off'} · ${roundedRpm} rpm${handbrake ? ' · Handbrake on' : ''}`);
    lastRpm=roundedRpm;lastIgnition=ignition;lastRunning=running;lastStarter=starter;lastHandbrake=handbrake;
  }
};
function fail(message) {
  failed = true; pause(); clearTimeout(timeout); start.disabled = true;
  const text = `Cannot start the game. ${message} Reload to try again.`;
  status.textContent = text;
  if (screen.hidden) { runtimeError.textContent = text; runtimeError.hidden = false; }
}
window.gameFailure = fail;
window.gameReady = () => {
  if (failed) return;
  clearTimeout(timeout); ready = true; status.textContent = 'The garage is ready.'; start.disabled = false;
};
window.addEventListener('error', (event) => fail(event.message || 'The runtime stopped.'));
window.addEventListener('unhandledrejection', (event) => fail(event.reason?.message || String(event.reason)));
canvas.addEventListener('webglcontextlost', (event) => { event.preventDefault(); fail('The graphics context was lost.'); });
start.addEventListener('click', () => capture(1));
window.addEventListener('keydown', (event) => {
  if (!ready || failed || !window.gameState.started) return;
  if (event.code === 'KeyP' || event.code === 'Escape') {
    event.preventDefault(); if (!event.repeat) { if (window.gameState.paused) capture(4); else pause(); }
    return;
  }
  if (!captured() || window.gameState.paused || actions & 2) return;
  if (event.code in keyBits) { event.preventDefault(); if (!event.repeat) setCode(event.code, true); }
  else if (event.code === 'KeyR' && !event.repeat) { clearInput(); actions |= 16; }
});
window.addEventListener('keyup', (event) => {
  if (event.code in keyBits) { event.preventDefault(); setCode(event.code, false); }
});
canvas.addEventListener('mousedown', (event) => {
  if (!captured() || window.gameState.paused) return;
  if (event.button === 0 || event.button === 2) { event.preventDefault(); setCode(`Mouse${event.button}`, true); }
});
window.addEventListener('mouseup', (event) => {
  if (event.button === 0 || event.button === 2) setCode(`Mouse${event.button}`, false);
});
canvas.addEventListener('contextmenu', (event) => event.preventDefault());
canvas.addEventListener('mousemove', (event) => {
  if (captured() && !window.gameState.paused) { mouseX += event.movementX * 0.12; mouseY += event.movementY * 0.12; }
});
canvas.addEventListener('wheel', (event) => {
  if (!captured() || window.gameState.paused) return;
  event.preventDefault(); wheel += -event.deltaY * 0.001;
}, { passive: false });
document.addEventListener('pointerlockchange', () => { if (!captured()) pause(); });
window.addEventListener('blur', pause);
document.addEventListener('visibilitychange', () => { if (document.hidden) pause(); });
document.querySelector('#pause').addEventListener('click', pause);
document.querySelector('#resume').addEventListener('click', () => capture(4));
document.querySelector('#restart').addEventListener('click', () => capture(8));
const fullscreen = document.querySelector('#fullscreen');
fullscreen.disabled = !document.fullscreenEnabled;
fullscreen.addEventListener('click', async () => {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.querySelector('main').requestFullscreen();
  } catch { captureError.textContent = 'Fullscreen is unavailable.'; }
});
document.addEventListener('fullscreenchange', () => { fullscreen.textContent = document.fullscreenElement ? 'Exit fullscreen' : 'Fullscreen'; });
async function load() {
  const probe = document.createElement('canvas');
  const graphics = probe.getContext('webgl2');
  if (!graphics) throw new Error('This browser cannot use WebGL2.');
  graphics.getExtension('WEBGL_lose_context')?.loseContext();
  status.textContent = 'Loading the game code…';
  const { default: init, run } = await import('./pkg/desert_drive.js');
  const wasm = await fetch('./pkg/desert_drive_bg.wasm');
  if (!wasm.ok) throw new Error(`Cannot load game code (HTTP ${wasm.status}).`);
  await init({ module_or_path: wasm });
  status.textContent = 'Loading the garage assets…';
  const response = await fetch('./assets/reference/manifest.json');
  if (!response.ok) throw new Error(`Cannot load reference/manifest.json (HTTP ${response.status}).`);
  const text = await response.text();
  const manifest=JSON.parse(text);
  const tank=manifest.tanks.find((tank)=>tank.item===manifest.engine.fuel_tank);
  fuelLevel.max=tank.capacity_liters;fuelLevel.optimum=tank.capacity_liters;
  timeout = setTimeout(() => fail('The garage did not load within 120 seconds.'), 120_000);
  run(text);
}
load().catch((error) => fail(error.message));
