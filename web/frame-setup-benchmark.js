// Run on an active game tab served by serve-content-index-benchmark.mjs.
// Keep instrumented setup counts separate from release frame-time samples.
export async function measureFrameSetup({instrument = false, label = 'frame-setup', ...options} = {}) {
  const {measureContentIndex} = await import('/__measure.mjs');
  const wasm = await import('/pkg/my_summer_slop.js');
  const gl = document.querySelector('#game').getContext('webgl2');
  const saved = new Map();
  const uploads = {bufferData: 0, bufferSubData: 0, bytes: 0};
  if (instrument) {
    for (const name of ['bufferData', 'bufferSubData']) {
      const original = gl[name];
      saved.set(name, original);
      gl[name] = function(...args) {
        uploads[name]++;
        const data = args[name === 'bufferData' ? 1 : 2];
        const offset = args[3] ?? 0;
        const length = args[4];
        uploads.bytes += typeof data === 'number' ? data :
          data == null ? 0 : length ? length * (data.BYTES_PER_ELEMENT ?? 1) :
          data.byteLength - offset * (data.BYTES_PER_ELEMENT ?? 1);
        return original.apply(this, args);
      };
    }
  }
  let frames, setup;
  wasm.reflection_preparation_profile(instrument);
  try {
    frames = await measureContentIndex({...options, instrument, label: `${label}-frames`});
  } finally {
    setup = JSON.parse(wasm.reflection_preparation_audit());
    wasm.reflection_preparation_profile(false);
    for (const [name, original] of saved) gl[name] = original;
  }
  const stages = Object.fromEntries(setup.stages.map((name, index) => {
    const sum = {calls: 0, total_ms: 0, items: 0, unchanged: 0, writes: 0, capacity_growths: 0, upload_bytes: 0};
    for (const job of setup.measurements) for (const key of Object.keys(sum)) sum[key] += job[index][key];
    return [name, sum];
  }));
  const result = {...frames, label, scenario: options.scenario ?? 'garage', instrument, uploads, stages};
  const response = await fetch(`/__benchmark-result/${label}`, {
    method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(result),
  });
  if (!response.ok) throw Error(`Cannot save frame setup receipt: HTTP ${response.status}`);
  return {...result, receipt: await response.json()};
}