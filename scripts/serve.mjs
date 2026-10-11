import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../web');
const types = new Map([
  ['.html', 'text/html; charset=utf-8'],
  ['.css', 'text/css; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.wasm', 'application/wasm'],
  ['.json', 'application/json'],
  ['.gltf', 'model/gltf+json'],
  ['.bin', 'application/octet-stream'],
  ['.png', 'image/png'],
  ['.wav', 'audio/wav'],
  ['.mp3', 'audio/mpeg'],
]);

createServer(async (request, response) => {
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    response.writeHead(405, { Allow: 'GET, HEAD' }).end();
    return;
  }
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const path = resolve(root, `.${pathname === '/' ? '/index.html' : pathname}`);
    if (!path.startsWith(root + sep)) {
      response.writeHead(403).end('Forbidden');
      return;
    }
    const file = await stat(path);
    if (!file.isFile()) {
      response.writeHead(404).end('Not found');
      return;
    }
    // Media elements seek with byte ranges.
    const range = /^bytes=(\d*)-(\d*)$/.exec(request.headers.range ?? '');
    const start = range && range[1] ? Number(range[1]) : range && range[2] ? Math.max(0, file.size - Number(range[2])) : 0;
    const end = range && range[1] && range[2] ? Math.min(Number(range[2]), file.size - 1) : file.size - 1;
    if (range && (start > end || start >= file.size)) {
      response.writeHead(416, { 'Content-Range': `bytes */${file.size}` }).end();
      return;
    }
    response.writeHead(range ? 206 : 200, {
      'Content-Type': types.get(extname(path)) ?? 'application/octet-stream',
      'Content-Length': end - start + 1,
      'Accept-Ranges': 'bytes',
      ...(range ? { 'Content-Range': `bytes ${start}-${end}/${file.size}` } : {}),
      'Cache-Control': 'no-store',
    });
    if (request.method === 'HEAD') {
      response.end();
      return;
    }
    createReadStream(path, { start, end }).on('error', () => response.destroy()).pipe(response);
  } catch (error) {
    const code = error.code === 'ENOENT' || error.code === 'ENOTDIR' ? 404 : 400;
    response.writeHead(code).end(code === 404 ? 'Not found' : 'Invalid request');
  }
}).listen(4173, '127.0.0.1', () => {
  console.log('My Summer Slop: http://127.0.0.1:4173');
});
