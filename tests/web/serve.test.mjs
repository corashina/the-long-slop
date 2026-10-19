import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { request } from 'node:http';
import { mkdtemp, mkdir, rm, stat, utimes, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { brotliCompressSync, brotliDecompressSync } from 'node:zlib';
import { createStaticServer } from '../../scripts/serve.mjs';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

async function fixture(t, changeIndex) {
  const directory = await mkdtemp(join(tmpdir(), 'static-brotli-'));
  const root = join(directory, 'web');
  const compressionRoot = join(directory, 'compression');
  const pathname = '/pkg/game.wasm';
  const sourcePath = join(root, 'pkg/game.wasm');
  const source = Buffer.from('wasm fixture '.repeat(1000));
  const encoded = brotliCompressSync(source);
  const encodedPath = join(compressionRoot, `${sha256(encoded)}.br`);
  await mkdir(dirname(sourcePath), { recursive: true });
  await mkdir(compressionRoot);
  await writeFile(sourcePath, source);
  await writeFile(encodedPath, encoded);
  await writeFile(join(root, 'index.html'), '<html>fixture</html>');
  await writeFile(join(root, 'texture.bin'), source);
  await writeFile(join(directory, 'secret.json'), 'secret');
  const file = await stat(sourcePath);
  const entry = {
    source: { size: file.size, mtimeMs: file.mtimeMs, ctimeMs: file.ctimeMs, sha256: sha256(source) },
    brotli: { file: `${sha256(encoded)}.br`, size: encoded.length, sha256: sha256(encoded) },
  };
  const index = { version: 1, files: { [pathname]: entry } };
  const publish = await changeIndex?.(index, { root, compressionRoot, sourcePath, encodedPath });
  if (publish !== false) await writeFile(join(compressionRoot, 'index.json'), JSON.stringify(index));
  const server = createStaticServer({ root, compressionRoot });
  t.after(async () => {
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    await rm(directory, { recursive: true, force: true });
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const get = (headers = {}, method = 'GET', path = pathname) => new Promise((resolve, reject) => {
    const req = request({ hostname: '127.0.0.1', port: server.address().port, path, headers, method, agent: false }, response => {
      const chunks = [];
      response.on('data', bytes => chunks.push(bytes));
      response.on('end', () => resolve({ status: response.statusCode, headers: response.headers, body: Buffer.concat(chunks) }));
      response.on('error', reject);
    });
    req.on('error', reject);
    req.end();
  });
  return { get, source, encoded, root, compressionRoot, sourcePath, encodedPath, index };
}

test('serves verified Brotli with original MIME and encoded length', async t => {
  const fixtureData = await fixture(t, async (index, { root }) => {
    const path = '/assets/runtime/build/reference/garage.plan.json';
    const fullPath = join(root, path);
    await mkdir(dirname(fullPath), { recursive: true });
    await writeFile(fullPath, 'wasm fixture '.repeat(1000));
    const file = await stat(fullPath);
    index.files[path] = { ...index.files['/pkg/game.wasm'], source: { ...index.files['/pkg/game.wasm'].source, mtimeMs: file.mtimeMs, ctimeMs: file.ctimeMs } };
  });
  const { get, source, encoded } = fixtureData;
  const response = await get({ 'Accept-Encoding': 'br' });
  assert.equal(response.status, 200);
  assert.equal(response.headers['content-encoding'], 'br');
  assert.equal(response.headers['content-type'], 'application/wasm');
  assert.equal(Number(response.headers['content-length']), encoded.length);
  assert.equal(response.headers.vary, 'Accept-Encoding');
  assert.equal(response.headers['cache-control'], 'no-cache');
  assert.deepEqual(response.body, encoded);
  assert.deepEqual(brotliDecompressSync(response.body), source);
  const json = await get({ 'Accept-Encoding': 'br' }, 'GET', '/assets/runtime/build/reference/garage.plan.json');
  assert.equal(json.headers['content-encoding'], 'br');
  assert.equal(json.headers['content-type'], 'application/json');
  assert.equal(json.headers['cache-control'], 'public, max-age=31536000, immutable');
  const binary = await get({ 'Accept-Encoding': 'br' }, 'GET', '/texture.bin');
  assert.equal(binary.headers['content-encoding'], undefined);
  assert.deepEqual(binary.body, source);
});

test('negotiates quality, explicit rejection, wildcard, and identity', async t => {
  const { get } = await fixture(t);
  const cases = [
    [undefined, undefined, 200], ['', undefined, 200], ['gzip', undefined, 200],
    ['br', 'br', 200], ['gzip, BR ; Q=1.000', 'br', 200], ['br;q=0', undefined, 200],
    ['br;q=0.3, identity;q=0.2', 'br', 200], ['br;q=0.2, identity;q=0.8', undefined, 200],
    ['br;q=0.2', undefined, 200], ['*', 'br', 200], ['*;q=0', undefined, 406],
    ['br;q=0, *;q=1', undefined, 200], ['br;q=1, *;q=0', 'br', 200],
    ['identity;q=0', undefined, 406], ['gzip, identity;q=0', undefined, 406],
    ['br;q=0, identity;q=0', undefined, 406], ['*;q=0, identity;q=1', undefined, 200],
    ['br;q=0.001, identity;q=0', 'br', 200],
  ];
  for (const [encoding, expectedEncoding, status] of cases) {
    const response = await get(encoding === undefined ? {} : { 'Accept-Encoding': encoding });
    assert.equal(response.status, status, encoding);
    assert.equal(response.headers['content-encoding'], expectedEncoding, encoding);
    assert.equal(response.headers.vary, 'Accept-Encoding', encoding);
  }
});

test('uses distinct ETags and weak If-None-Match comparison', async t => {
  const { get } = await fixture(t);
  const identity = await get();
  const compressed = await get({ 'Accept-Encoding': 'br' });
  assert.notEqual(identity.headers.etag, compressed.headers.etag);
  for (const [encoding, etag] of [['identity', identity.headers.etag], ['br', compressed.headers.etag]]) {
    for (const condition of [etag, etag.replace(/^W\//, ''), `W/${etag.replace(/^W\//, '')}`, '"other", ' + etag, '*']) {
      const response = await get({ 'Accept-Encoding': encoding, 'If-None-Match': condition });
      assert.equal(response.status, 304);
      assert.equal(response.headers.etag, etag);
      assert.equal(response.headers.vary, 'Accept-Encoding');
      assert.equal(response.body.length, 0);
    }
  }
  const response = await get({ 'Accept-Encoding': 'br', 'If-None-Match': identity.headers.etag });
  assert.equal(response.status, 200);
});

test('HEAD has GET headers and no body for each representation', async t => {
  const { get } = await fixture(t);
  for (const encoding of ['identity', 'br']) {
    const headers = { 'Accept-Encoding': encoding };
    const getResponse = await get(headers);
    const headResponse = await get(headers, 'HEAD');
    assert.equal(headResponse.status, 200);
    for (const name of ['content-length', 'content-type', 'content-encoding', 'etag', 'vary', 'cache-control', 'last-modified']) {
      assert.equal(headResponse.headers[name], getResponse.headers[name], name);
    }
    assert.equal(headResponse.body.length, 0);
  }
});

test('missing cache uses identity and rejects requests that require Brotli', async t => {
  const { get } = await fixture(t, async (_index, { compressionRoot }) => {
    await rm(compressionRoot, { recursive: true });
    return false;
  });
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], undefined);
  assert.equal((await get({ 'Accept-Encoding': 'br, identity;q=0' })).status, 406);
});

test('source mutation invalidates an already verified variant', async t => {
  const { get, source, sourcePath } = await fixture(t);
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], 'br');
  const changed = Buffer.from(source);
  changed[0] ^= 1;
  await writeFile(sourcePath, changed);
  const future = new Date(Date.now() + 10000);
  await utimes(sourcePath, future, future);
  const response = await get({ 'Accept-Encoding': 'br' });
  assert.equal(response.headers['content-encoding'], undefined);
  assert.deepEqual(response.body, changed);
  assert.equal((await get({ 'Accept-Encoding': 'br, identity;q=0' })).status, 406);
});

test('compressed mutation invalidates its cached hash and can recover', async t => {
  const { get, encoded, encodedPath } = await fixture(t);
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], 'br');
  const changed = Buffer.from(encoded);
  changed[0] ^= 1;
  await writeFile(encodedPath, changed);
  const future = new Date(Date.now() + 10000);
  await utimes(encodedPath, future, future);
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], undefined);
  assert.equal((await get({ 'Accept-Encoding': 'br, identity;q=0' })).status, 406);
  await writeFile(encodedPath, encoded);
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], 'br');
});

test('rejects wrong source hashes, encoded hashes, and unsafe cache paths', async t => {
  for (const kind of ['source hash', 'encoded hash', 'cache path']) {
    const { get } = await fixture(t, index => {
      const entry = index.files['/pkg/game.wasm'];
      if (kind === 'source hash') entry.source.sha256 = '0'.repeat(64);
      if (kind === 'encoded hash') entry.brotli.sha256 = '0'.repeat(64);
      if (kind === 'cache path') entry.brotli.file = '../secret.json';
    });
    assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], undefined, kind);
    assert.equal((await get({ 'Accept-Encoding': 'br, identity;q=0' })).status, 406, kind);
  }
});

test('keeps one index snapshot and rejects HTTP path traversal', async t => {
  const { get, compressionRoot } = await fixture(t);
  await writeFile(join(compressionRoot, 'index.json'), '{invalid');
  assert.equal((await get({ 'Accept-Encoding': 'br' })).headers['content-encoding'], 'br');
  assert.equal((await get({}, 'GET', '/%2e%2e%2fsecret.json')).status, 403);
  assert.equal((await get({}, 'GET', '/%2e%2e%5csecret.json')).status, 403);
  assert.equal((await get({}, 'GET', '/%zz')).status, 400);
  assert.equal((await get({}, 'POST')).status, 405);
  assert.equal((await get({}, 'GET', '/')).headers['content-type'], 'text/html; charset=utf-8');
  assert.equal((await get({}, 'GET', '/missing.json')).status, 404);
});
