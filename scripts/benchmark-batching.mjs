// Serve a frozen release package and source assets for matched batching checks.
// node scripts/benchmark-batching.mjs --pkg PATH --reference PATH --runtime PATH --library PATH --port 4188
import {createServer} from 'node:http';
import {createReadStream} from 'node:fs';
import {mkdir,stat,writeFile} from 'node:fs/promises';
import {dirname,extname,resolve,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
const {values}=parseArgs({options:Object.fromEntries(['pkg','reference','runtime','library','port','reports'].map(name=>[name,{type:'string'}]))});
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
for(const key of ['pkg','reference','runtime','library'])if(!values[key])throw Error(`Missing --${key}`);
const reports=resolve(values.reports??resolve(root,'.cache/batching/reports'));
const mounts=[['/pkg/',values.pkg],['/assets/reference/',values.reference],['/assets/runtime/',values.runtime],['/assets/library/',values.library],['/',resolve(root,'web')]];
const types={'.html':'text/html','.css':'text/css','.wav':'audio/wav','.ttf':'font/ttf','.otf':'font/otf','.glb':'model/gltf-binary','.js':'text/javascript','.mjs':'text/javascript','.json':'application/json','.wasm':'application/wasm','.png':'image/png','.gltf':'model/gltf+json','.bin':'application/octet-stream'};
createServer(async(request,response)=>{
 try{
  const pathname=decodeURIComponent(new URL(request.url,'http://localhost').pathname);
  if(request.method==='POST'&&pathname.startsWith('/__batching/report/')){
   const name=pathname.slice('/__batching/report/'.length);
   if(!/^[a-zA-Z0-9_-]+$/.test(name)){response.writeHead(400).end();return;}
   const parts=[];let length=0;
   for await(const part of request){length+=part.length;if(length>8*1024*1024)throw Error('Report too large');parts.push(part);}
   const report=JSON.parse(Buffer.concat(parts).toString());
   await mkdir(reports,{recursive:true});await writeFile(resolve(reports,`${name}.json`),JSON.stringify(report,null,2),{flag:'wx'});response.end('Saved');return;
  }
  if(request.method!=='GET'&&request.method!=='HEAD'){response.writeHead(405).end();return;}
  let file;
  if(pathname==='/__batching/probe.js')file=resolve(root,'scripts/batching-probe.js');
  else{
   const [prefix,directory]=mounts.find(([prefix])=>pathname.startsWith(prefix));
   const base=resolve(directory);file=resolve(base,pathname==='/'?'index.html':pathname.slice(prefix.length));
   if(!file.startsWith(base+sep)){response.writeHead(403).end();return;}
  }
  const info=await stat(file);if(!info.isFile()){response.writeHead(404).end();return;}
  response.writeHead(200,{'Content-Type':types[extname(file)]??'application/octet-stream','Content-Length':info.size,'Cache-Control':pathname.startsWith('/assets/runtime/')?'public, max-age=31536000, immutable':'no-cache'});
  if(request.method==='HEAD'){response.end();return;}createReadStream(file).on('error',()=>response.destroy()).pipe(response);
 }catch(error){response.writeHead(error.code==='ENOENT'?404:400).end(String(error.message));}
}).listen(Number(values.port??4188),'127.0.0.1',function(){console.log(`Batching benchmark: http://127.0.0.1:${this.address().port}`);});
