import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtemp, mkdir, readFile, readdir, rm, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {brotliDecompressSync} from 'node:zlib';
import {prepareStartupCompression} from '../../scripts/prepare-http-compression.mjs';

test('preparation compresses current WASM and JSON outside checked packages and can rerun', async t => {
  const root = await mkdtemp(join(tmpdir(), 'tld-http-compression-'));
  t.after(() => rm(root, {recursive: true, force: true}));
  const webRoot = join(root, 'web'), compressionRoot = join(root, 'cache');
  const runtime = 'assets/runtime/current/';
  const manifest = {content_base: runtime, file_catalog: {files: [
    {file: 'reference/car.plan.json'}, {file: 'reference/image.png'},
  ]}};
  const originals = new Map([
    ['assets/reference/manifest.json', Buffer.from(JSON.stringify(manifest))],
    ['pkg/my_summer_slop_bg.wasm', Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])],
    ['pkg/my_summer_slop_webgpu_bg.wasm', Buffer.from([0, 97, 115, 109, 1, 0, 0, 1])],
    ['pkg/terrain_worker_bg.wasm', Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])],
    ['pkg/road_worker_bg.wasm', Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])],
    ['pkg/grass_worker_bg.wasm', Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])],
    [runtime + 'reference/car.plan.json', Buffer.from('{"nodes":[]}')],
    [runtime + 'reference/image.png', Buffer.from('already compressed')],
    ['assets/runtime/old/unused.json', Buffer.from('{}')],
  ]);
  for (const [name, bytes] of originals) {
    await mkdir(join(webRoot, name, '..'), {recursive: true});
    await writeFile(join(webRoot, name), bytes);
  }
  const first = await prepareStartupCompression({webRoot, compressionRoot});
  assert.equal(Object.keys(first.files).length, 7);
  for (const [name, entry] of Object.entries(first.files)) {
    const encoded = await readFile(join(compressionRoot, entry.brotli.file));
    const original = originals.get(name.slice(1));
    assert.deepEqual(brotliDecompressSync(encoded), original);
    assert.equal(entry.source.size, original.length);
    assert.equal(entry.brotli.size, encoded.length);
    assert.equal(entry.source.sha256, createHash('sha256').update(original).digest('hex'));
    assert.equal(entry.brotli.sha256, createHash('sha256').update(encoded).digest('hex'));
  }
  assert.deepEqual(await readdir(join(webRoot, runtime, 'reference')), ['car.plan.json', 'image.png']);
  assert.deepEqual(await prepareStartupCompression({webRoot, compressionRoot}), first);
  assert.deepEqual(JSON.parse(await readFile(join(compressionRoot, 'index.json'))), first);
  await writeFile(join(webRoot, runtime, 'reference/car.plan.json'), '{"nodes":[1]}');
  const next = await prepareStartupCompression({webRoot, compressionRoot});
  assert.notEqual(next.files['/' + runtime + 'reference/car.plan.json'].source.sha256,
    first.files['/' + runtime + 'reference/car.plan.json'].source.sha256);
  await writeFile(join(webRoot, 'assets/reference/manifest.json'), JSON.stringify({
    ...manifest, file_catalog: {files: [{file: '../outside.json'}]},
  }));
  await assert.rejects(prepareStartupCompression({webRoot, compressionRoot}), /receipt path/);
  assert.deepEqual(JSON.parse(await readFile(join(compressionRoot, 'index.json'))), next);
});
