import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { mkdir, realpath, stat, writeFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../web');
const testRoot = resolve(root, '../tests/browser');
const traceRoot = resolve(root, '../.cache/startup');
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
  if (request.method === 'POST' && request.url === '/__tests__/startup-trace') {
    let body = '';
    for await (const chunk of request) {
      body += chunk;
      if (body.length > 2 * 1024 * 1024) {
        response.writeHead(413).end('Trace too large');
        return;
      }
    }
    try {
      const trace = JSON.parse(body);
      if (!trace || typeof trace !== 'object') throw Error('Invalid trace');
      await mkdir(traceRoot, { recursive: true });
      const name = `startup-${Date.now()}.json`;
      await writeFile(resolve(traceRoot, name), JSON.stringify(trace, null, 2));
      response.writeHead(201, { 'Content-Type': 'text/plain; charset=utf-8' }).end(`.cache/startup/${name}`);
    } catch {
      response.writeHead(400).end('Invalid trace');
    }
    return;
  }
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    response.writeHead(405, { Allow: 'GET, HEAD' }).end();
    return;
  }
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const testRoute = pathname.startsWith('/__tests__/');
    const routeRoot = testRoute ? testRoot : root;
    const routePath = testRoute ? pathname.slice('/__tests__'.length) : pathname;
    const path = resolve(routeRoot, `.${routePath === '/' ? '/index.html' : routePath}`);
    if (!path.startsWith(routeRoot + sep)) {
      response.writeHead(403).end('Forbidden');
      return;
    }
    const file = await stat(path);
    if (testRoute && !(await realpath(path)).startsWith(testRoot + sep)) {
      response.writeHead(403).end('Forbidden');
      return;
    }
    if (!file.isFile()) {
      response.writeHead(404).end('Not found');
      return;
    }
    response.writeHead(200, {
      'Content-Type': types.get(extname(path)) ?? 'application/octet-stream',
      'Content-Length': file.size,
      'Last-Modified': file.mtime.toUTCString(),
      'Cache-Control': 'no-store',
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
