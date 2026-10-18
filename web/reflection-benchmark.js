// Run in the browser console after readiness:
// const {sampleReflection} = await import('/reflection-benchmark.js');
// await sampleReflection({seconds: 10, preparation: false});
import {ACTIONS} from './menu.js';

// Test input uses the same public host calls as keyboard and mouse input.
// Reload after tests, or call restore(), to remove the overrides.
export function createReflectionTestInput() {
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

export async function sampleReflection({seconds = 10, preparation = false} = {}) {
  const wasm = await import('./pkg/my_summer_slop.js');
  const audit = () => JSON.parse(wasm.source_render_audit());
  const startup = window.gameStartupTrace();
  if (!startup.completed || !startup.inputReady) throw new Error('Wait for complete startup and input readiness.');
  const beforeWorld = await window.measureWorld();
  if (beforeWorld.world.generation_jobs || beforeWorld.world.pending_work) throw new Error('Wait for world work to settle.');
  if (audit().state !== 'Ready') throw new Error('Wait for renderer readiness.');
  if (document.hidden) throw new Error('Keep the game tab visible.');
  const times = new Float64Array(16384);
  const original = window.gamePerformanceFrame;
  let count = 0, hidden = false;
  const visibility = () => { hidden ||= document.hidden; };
  const update = (physicsMs, worldMs) => {
    if (count < times.length) times[count++] = performance.now();
    return original(physicsMs, worldMs);
  };
  document.addEventListener('visibilitychange', visibility);
  wasm.reflection_preparation_profile(preparation);
  const before = audit();
  const sampleStart = performance.now();
  window.gamePerformanceFrame = update;
  const startState = {...window.gameState};
  let measurements, after, sampleEnd;
  try {
    await new Promise(resolve => setTimeout(resolve, seconds * 1000));
    sampleEnd = performance.now();
    after = audit();
    measurements = JSON.parse(wasm.reflection_preparation_audit());
  } finally {
    if (window.gamePerformanceFrame === update) window.gamePerformanceFrame = original;
    document.removeEventListener('visibilitychange', visibility);
    wasm.reflection_preparation_profile(false);
  }
  if (hidden) throw new Error('The game tab was hidden during measurement.');
  if (count < 2 || count === times.length) throw new Error('Invalid frame sample count.');
  const endState = {...window.gameState};
  const afterWorld = await window.measureWorld();
  if (afterWorld.world.generation_jobs || afterWorld.world.pending_work || beforeWorld.world.generated_objects !== afterWorld.world.generated_objects) {
    throw new Error('World work changed during measurement.');
  }
  if (after.state !== 'Ready') throw new Error('Renderer readiness changed during measurement.');
  const submittedFrames = after.graph.token - before.graph.token;
  if (!before.graph.presented || !after.graph.presented || Math.abs(submittedFrames - count) > 2) {
    throw new Error('Frame callbacks did not match submitted frames.');
  }
  for (let index = 0; index < before.graph.probes.length; index++) {
    const probe = before.graph.probes[index], current = after.graph.probes[index];
    if (probe.enabled && (!current.published || current.generation <= probe.generation)) {
      throw new Error('An enabled probe did not complete a capture cycle.');
    }
  }
  if (startState.started && (startState.paused || endState.paused || endState.totalSteps <= startState.totalSteps)) {
    throw new Error('Physics must stay active during gameplay measurement.');
  }
  const intervals = Array.from(times.subarray(1, count), (time, index) => time - times[index]);
  const elapsed = times[count - 1] - times[0];
  intervals.sort((a, b) => a - b);
  const percentile = fraction => intervals[Math.ceil(intervals.length * fraction) - 1];
  const canvas = document.querySelector('#game');
  return {
    seconds, preparation, frames: intervals.length, submittedFrames, source: startup.buildIdentity,
    fps: 1000 * intervals.length / elapsed,
    submittedFps: 1000 * submittedFrames / (sampleEnd - sampleStart),
    frameMs: {mean: elapsed / intervals.length, p50: percentile(.5), p95: percentile(.95), p99: percentile(.99), worst: intervals.at(-1)},
    canvas: [canvas.width, canvas.height], dpr: devicePixelRatio,
    startState, endState, before, after, measurements,
    world: {before: beforeWorld.world, after: afterWorld.world},
  };
}
