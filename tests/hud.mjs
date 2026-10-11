import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

test('HUD displays committed throw charge and hides it after release or cancellation', () => {
  const elements = new Map();
  const document = {
    querySelector(selector) {
      if (!elements.has(selector)) elements.set(selector, { hidden: true, value: 0, textContent: '', addEventListener() {} });
      return elements.get(selector);
    },
    addEventListener() {},
  };
  const window = { addEventListener() {} };
  const source = readFileSync(new URL('../web/main.js', import.meta.url), 'utf8');
  // Exercise the real HUD callback without starting WASM or graphics.
  runInNewContext(source.replace('load().catch((error) => fail(error.message));', ''), { document, window });
  const update = (held, charge, active) => window.gameDetail('', '', -1, -1, held, charge, active, 0, false, false, false, false, true, 0, 0, 0, 0, 0, 0);
  update('Rim', 0.45, true);
  assert.equal(window.gameState.throwCharge, 0.45);
  assert.equal(window.gameState.throwActive, true);
  assert.equal(elements.get('#throw-charge').hidden, false);
  assert.equal(elements.get('#throw-level').value, 0.45);
  update('Rim', 0, false);
  assert.equal(elements.get('#throw-charge').hidden, true);
  assert.equal(elements.get('#throw-level').value, 0);
  update('Rim', 1.02, true);
  assert.equal(elements.get('#throw-level').value, 1);
  update('', 0, false);
  assert.equal(window.gameState.throwActive, false);
  assert.equal(elements.get('#throw-charge').hidden, true);
});
