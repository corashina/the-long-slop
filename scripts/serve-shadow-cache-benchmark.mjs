import {createServer} from 'node:http';
import {createReadStream} from 'node:fs';
import {stat,mkdir,writeFile} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
const root=process.cwd(),web=resolve(root,'web'),baseline=process.env.BENCH_VARIANT==='baseline';
const types={'.css':'text/css','.ttf':'font/ttf','.otf':'font/otf','.glb':'model/gltf-binary','.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm','.json':'application/json','.png':'image/png','.gltf':'model/gltf+json','.wav':'audio/wav'};
createServer(async(req,res)=>{try{
 const pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
 if(req.method==='POST'&&/^\/__benchmark-result\/[a-z0-9-]+$/.test(pathname)){const chunks=[];for await(const chunk of req)chunks.push(chunk);const dir=resolve(root,'.cache/benchmark/results');await mkdir(dir,{recursive:true});const label=pathname.split('/').at(-1);const body=Buffer.concat(chunks);JSON.parse(body.toString());await writeFile(resolve(dir,label+'.json'),body);res.writeHead(200).end('saved');return;}
 let path=resolve(web,'.'+(pathname==='/'?'/index.html':pathname));
 if(pathname==='/__measure.mjs')path=resolve(root,'scripts/measure-shadow-cache.mjs');
 else if(baseline&&pathname.startsWith('/pkg/'))path=resolve(root,'.cache/benchmark/playable-baseline'+pathname);
 else if(!path.startsWith(web+sep))throw Error('Invalid path');
 const file=await stat(path);if(!file.isFile())throw Error('Not a file');
 res.writeHead(200,{'Content-Type':types[extname(path)]??'application/octet-stream','Content-Length':file.size,'Cache-Control':pathname.startsWith('/assets/runtime/')?'public,max-age=31536000,immutable':'no-cache'});
 createReadStream(path).pipe(res);
}catch{res.writeHead(404).end();}}).listen(Number(process.env.PORT), '127.0.0.1',()=>console.log(`Benchmark ${baseline?'baseline':'shadow-cache'}: http://127.0.0.1:${process.env.PORT}/`));
