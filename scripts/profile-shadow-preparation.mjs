// Run each release variant separately. Keep CPU profiles separate from FPS samples.
// NODE_PATH must contain the installed playwright package.
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {parseArgs} from 'node:util';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createContentIndexBenchmarkServer} from './serve-content-index-benchmark.mjs';
import {poll} from './benchmark-poll.mjs';
const {chromium} = createRequire(import.meta.url)('playwright');
const {values} = parseArgs({options: {
  assets: {type:'string'}, package: {type:'string'}, variant: {type:'string'},
  out: {type:'string', default:'.cache/shadow-preparation/reports'},
  seconds: {type:'string', default:'10'}, timeout: {type:'string', default:'180000'},
  profile: {type:'boolean', default:false},
  scenario: {type:'string', default:'garage'}, save: {type:'string'},
}});
const seconds = Number(values.seconds), timeout = Number(values.timeout);
if (!Number.isFinite(seconds) || seconds < 1 || !Number.isFinite(timeout) || timeout < 1000) throw Error('Invalid measurement duration or timeout');
if (!['garage','driving'].includes(values.scenario)) throw Error('Invalid scenario');
if (values.scenario === 'driving' && !values.save) throw Error('Driving requires a frozen save with a seated player');
const saveText = values.save ? await readFile(values.save,'utf8') : null;
const saved = saveText ? JSON.parse(saveText) : null;
const out = resolve(values.out);
await mkdir(out, {recursive:true});
const server = await createContentIndexBenchmarkServer({assetRoot:values.assets, packageRoot:values.package,
  resultsRoot:out, variant:values.variant});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser, page, stage = 'launch';
const errors = [];
const result = {variant:values.variant, scenario:values.scenario, profile:values.profile, valid:false, recordedAt:new Date().toISOString()};
if (saveText) result.saveSha256 = createHash('sha256').update(saveText).digest('hex');
try {
  browser = await chromium.launch({channel:'chrome', headless:true});
  result.browserVersion = browser.version();
  result.headless = true;
  const context = await browser.newContext({viewport:{width:1280,height:720}, deviceScaleFactor:1});
  page = await context.newPage();
  page.on('pageerror', error => { if (errors.length < 30) errors.push(error.message); });
  page.on('console', message => { if (message.type() === 'error' && errors.length < 30) errors.push(message.text()); });
  stage = 'startup';
  await page.goto(`http://127.0.0.1:${server.address().port}/`, {waitUntil:'domcontentloaded', timeout});
  result.configuration = await page.evaluate(async () => (await fetch('/__benchmark-config')).json());
  if (values.profile) {
    await poll(() => page.evaluate(async () => {
      const wasm = await import('/pkg/the_long_slop.js');
      try {
        wasm.reflection_preparation_profile(true);
        wasm.shadow_preparation_profile?.(true);
        return true;
      } catch { return false; } // The main module initializes the WASM instance asynchronously.
    }), timeout);
  }
  await page.waitForFunction(() => {
    if (!window.gameStartupDone?.()) return false;
    const trace = window.gameStartupTrace?.();
    if (trace?.failure) throw Error(trace.failure.message);
    return trace?.completed && trace.inputReady && !document.querySelector('#new-game').disabled;
  }, null, {timeout});
  stage = 'new-game';
  await page.locator('#new-game').click();
  await page.waitForFunction(() => window.gameState?.started && !document.querySelector('#resume').disabled, null, {timeout});
  if (saved) {
    stage = 'load';
    await page.evaluate(async saved => {
      const {saveRuntime} = await import('/main.js');
      const token = await saveRuntime.prepare(saved);
      await saveRuntime.commit(token);
    }, saved);
  }
  await page.waitForFunction(() => window.gameState?.started && !document.querySelector('#resume').disabled, null, {timeout});
  stage = 'resume';
  await page.locator('#resume').click();
  await page.waitForFunction(() => window.gameState?.started && !window.gameState.paused, null, {timeout:30000});
  stage = 'settle';
  await poll(() => page.evaluate(async () => {
    const world = await window.measureWorld();
    return !world.world.pending_work && !world.world.generation_jobs;
  }), timeout);
  result.render = await page.evaluate(() => {
    const canvas = document.querySelector('#game'), gl = canvas.getContext('webgl2');
    if (!gl) throw Error('WebGL2 is unavailable');
    const info = gl.getExtension('WEBGL_debug_renderer_info');
    return {canvas:[canvas.width,canvas.height], dpr:devicePixelRatio,
      gpu:info?gl.getParameter(info.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER),
      pointerLock:document.pointerLockElement===canvas};
  });
  if (result.render.canvas[0]!==1280 || result.render.canvas[1]!==720 || result.render.dpr!==1) throw Error('Benchmark viewport mismatch');
  if (/SwiftShader|llvmpipe|Software Rasterizer/i.test(result.render.gpu)) throw Error('Hardware GPU is unavailable');
  // Trusted keyboard input reaches the public game input path before measurement.
  await page.keyboard.press('KeyW');
  if (values.scenario === 'driving') await page.keyboard.down('KeyW');
  stage = values.profile ? 'profile' : 'fps';
  result.sample = await page.evaluate(async ({seconds, profile, scenario}) => {
    const wasm = await import('/pkg/the_long_slop.js');
    const {measureContentIndex} = await import('/__measure.mjs');
    const shadowProfile = wasm.shadow_preparation_profile;
    if (profile && shadowProfile) shadowProfile(true);
    if (profile) wasm.reflection_preparation_profile(true);
    try {
      const sample = await measureContentIndex({label:`${scenario}-${profile?'cpu':'fps'}`, scenario, durationMs:seconds*1000});
      return { ...sample, performanceSample:!profile && sample.performanceSample,
        shadow:profile && shadowProfile ? JSON.parse(wasm.shadow_preparation_audit()) : null,
        preparation:profile ? JSON.parse(wasm.reflection_preparation_audit()) : null };
    } finally {
      if (shadowProfile) shadowProfile(false);
      wasm.reflection_preparation_profile(false);
    }
  }, {seconds, profile:values.profile, scenario:values.scenario});
  const receipt = JSON.parse(await readFile(result.sample.receipt.path,'utf8'));
  receipt.profilePreparation = values.profile;
  receipt.performanceSample = result.sample.performanceSample;
  receipt.shadowProfile = result.sample.shadow;
  receipt.preparationProfile = result.sample.preparation;
  await writeFile(result.sample.receipt.path, JSON.stringify(receipt,null,2)+'\n');
  result.valid = result.sample.valid;
} catch (error) {
  result.error = error.message;
} finally {
  result.stage = stage;
  result.errors = errors;
  if (page) {
    try {
      result.observed = await page.evaluate(async () => {
        const observed = {startup:window.gameStartupTrace?.(), state:window.gameState,
          status:document.querySelector('#status')?.textContent,
          captureError:document.querySelector('#capture-error')?.textContent};
        try {
          const wasm = await import('/pkg/the_long_slop.js');
          observed.audit = JSON.parse(wasm.source_render_audit());
          if (wasm.shadow_preparation_audit) observed.shadowProfile = JSON.parse(wasm.shadow_preparation_audit());
          observed.preparationProfile = JSON.parse(wasm.reflection_preparation_audit());
        } catch (error) { observed.auditError = error.message; }
        return observed;
      });
      await page.screenshot({path:resolve(out, `${values.variant}-${values.scenario}-${stage}.png`)});
    } catch (error) { result.observationError = error.message; }
  }
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
const receipt = resolve(out, `${values.variant}-${values.scenario}-${values.profile?'cpu':'fps'}-summary.json`);
await writeFile(receipt, JSON.stringify(result,null,2)+'\n', {flag:'wx'});
console.log(JSON.stringify({valid:result.valid, stage, error:result.error, fps:result.sample?.fps, receipt}));
if (!result.valid) process.exitCode = 1;
