import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readAsset} from '../../web/world-assets.js';

test('native blocks pass checked IO unchanged and reject short, trailing, or corrupt bytes', async () => {
  const blocks = Uint8Array.from({length: 32}, (_, i) => i);
  const receipt = {file: 'reference/texture-fixture.assets-12-srgb.bc', bytes: blocks.length,
    sha256: createHash('sha256').update(blocks).digest('hex'), image_size: null};
  const previous = globalThis.fetch;
  try {
    globalThis.fetch = async () => new Response(blocks);
    assert.deepEqual(await readAsset('assets/runtime/test/', receipt), blocks);
    for (const bytes of [blocks.slice(0, -1), Uint8Array.from([...blocks, 0])]) {
      globalThis.fetch = async () => new Response(bytes);
      await assert.rejects(readAsset('assets/runtime/test/', receipt), /Asset length/);
    }
    globalThis.fetch = async () => new Response(new Uint8Array(blocks.length));
    await assert.rejects(readAsset('assets/runtime/test/', receipt), /SHA-256 mismatch/);
  } finally {
    globalThis.fetch = previous;
  }
});
