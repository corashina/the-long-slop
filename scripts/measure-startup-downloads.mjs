import {createHash} from 'node:crypto';
import {request} from 'node:http';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {dirname, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createBrotliDecompress} from 'node:zlib';
import {createStaticServer} from './serve.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const compressionRoot = resolve(root, '.cache/http-compression');
const index = JSON.parse(await readFile(resolve(compressionRoot, 'index.json'), 'utf8'));
const resources = ['/pkg/my_summer_slop_bg.wasm', '/assets/reference/manifest.json'];
const plan = Object.keys(index.files).find(name => name.startsWith('/assets/runtime/') && name.endsWith('.plan.json'));
if (plan) resources.push(plan);

async function download(origin, path, encoding, etag) {
  const started = performance.now();
  const response = await new Promise((resolve, reject) => {
    const call = request(new URL(path, origin), {headers: {'Accept-Encoding': encoding,
      ...(etag ? {'If-None-Match': etag} : {})}}, resolve);
    call.on('error', reject); call.end();
  });
  let encodedBytes = 0, decodedBytes = 0;
  const hash = createHash('sha256');
  response.on('data', bytes => {encodedBytes += bytes.length;});
  const decoded = response.statusCode !== 304 && response.headers['content-encoding'] === 'br'
    ? response.pipe(createBrotliDecompress()) : response;
  response.on('error', error => decoded.destroy(error));
  for await (const bytes of decoded) {decodedBytes += bytes.length; hash.update(bytes);}
  const result = {path, status: response.statusCode, encoding: response.headers['content-encoding'] ?? 'identity',
    etag: response.headers.etag, encodedBytes, decodedBytes, milliseconds: performance.now() - started};
  if (etag) {
    if (result.status !== 304 || encodedBytes !== 0) throw Error('Conditional request did not reuse the response: ' + path);
  } else if (result.status !== 200 || decodedBytes !== index.files[path].source.size
    || hash.digest('hex') !== index.files[path].source.sha256) throw Error('Decoded response differs: ' + path);
  return result;
}

const report = {createdAt: new Date().toISOString(),
  scope: 'HTTP download and conditional revalidation only; not playable startup or an exclusive performance benchmark.',
  source: Object.fromEntries(resources.map(path => [path, index.files[path].source])), runs: []};
for (const [label, encoding] of [['A', 'identity'], ['B', 'br'], ['A2', 'identity']]) {
  const server = createStaticServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const origin = `http://127.0.0.1:${server.address().port}/`;
    const cold = [], cached = [];
    for (const path of resources) cold.push(await download(origin, path, encoding));
    for (const item of cold) cached.push(await download(origin, item.path, encoding, item.etag));
    report.runs.push({label, cold, cached});
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
}
const output = resolve(root, '.cache/startup-download-measurements.json');
await mkdir(dirname(output), {recursive: true});
await writeFile(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({output, runs: report.runs.map(run => ({label: run.label,
  encodedBytes: run.cold.reduce((sum, entry) => sum + entry.encodedBytes, 0),
  milliseconds: run.cold.reduce((sum, entry) => sum + entry.milliseconds, 0),
  cachedStatuses: run.cached.map(entry => entry.status)}))}));
