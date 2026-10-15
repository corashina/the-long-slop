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
  ['.ttf', 'font/ttf'],
  ['.otf', 'font/otf'],
  ['.glb', 'model/gltf-binary'],
]);

const port = Number(process.env.PORT ?? 4173);

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
    const etag = `W/"${file.size.toString(16)}-${file.mtimeMs.toString(16)}"`;
    const cache = pathname.startsWith('/assets/runtime/') ? 'public, max-age=31536000, immutable' : 'no-cache';
    const headers = {
      ETag: etag,
      'Cache-Control': cache,
    };
    if (request.headers['if-none-match']?.split(',').map(value => value.trim()).some(value => value === etag || value === '*')) {
      response.writeHead(304, headers).end();
      return;
    }
    response.writeHead(200, {
      ...headers,
      'Content-Type': types.get(extname(path)) ?? 'application/octet-stream',
      'Content-Length': file.size,
      'Last-Modified': file.mtime.toUTCString(),
    });
    if (request.method === 'HEAD') {
      response.end();
      return;
    }
    createReadStream(path).on('error', () => response.destroy()).pipe(response);
  } catch (error) {
    const code = error.code === 'ENOENT' || error.code === 'ENOTDIR' ? 404 : 400;
    response.writeHead(code).end(code === 404 ? 'Not found' : 'Invalid request');
  }
}).listen(port, '127.0.0.1', function () {
  console.log(`My Summer Slop: http://127.0.0.1:${this.address().port}`);
});
