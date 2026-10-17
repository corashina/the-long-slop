import {createServer} from 'node:http';
import {createReadStream} from 'node:fs';
import {mkdir,stat,writeFile} from 'node:fs/promises';
import {dirname,extname,resolve,sep} from 'node:path';
import {fileURLToPath} from 'node:url';

// node serve.mjs <web-root> <package> <asset-web-root> <report-directory> <port>
const here=dirname(fileURLToPath(import.meta.url));
const [web,pkg,assets,out]=process.argv.slice(2,6).map(value=>resolve(value));
const port=Number(process.argv[6]);
const types={'.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm',
 '.json':'application/json','.css':'text/css','.png':'image/png','.gltf':'model/gltf+json','.bin':'application/octet-stream',
 '.glb':'model/gltf-binary','.wav':'audio/wav','.ttf':'font/ttf','.otf':'font/otf'};
await mkdir(out,{recursive:true});
createServer(async(req,res)=>{
 try{
  const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
  if(name==='/__report'&&req.method==='POST'){
   let body='';for await(const chunk of req){body+=chunk;if(body.length>16000000)throw Error('Large report');}
   const data=JSON.parse(body);if(!/^[a-z0-9-]+$/.test(data.name))throw Error('Invalid report name');
   await writeFile(resolve(out,data.name+'.json'),JSON.stringify(data,null,2));res.end('saved');return;
  }
  if(req.method!=='GET'&&req.method!=='HEAD'){res.writeHead(405).end();return;}
  let base=web,path;
  if(name==='/__probe.js'){base=here;path=resolve(here,'probe.js');}
  else if(name.startsWith('/__fixtures/')){base=out;path=resolve(out,'.'+name.slice(11));}
  else if(name.startsWith('/pkg/')){base=pkg;path=resolve(pkg,'.'+name.slice(4));}
  else if(name.startsWith('/assets/')){base=assets;path=resolve(assets,'.'+name);}
  else path=resolve(web,'.'+(name==='/'?'/index.html':name));
  if(!path.startsWith(base+sep)){res.writeHead(403).end();return;}
  const file=await stat(path);if(!file.isFile())throw Error('Not a file');
  res.writeHead(200,{'Content-Type':types[extname(path)]??'application/octet-stream','Content-Length':file.size,
   'Cache-Control':name.startsWith('/assets/runtime/')?'public, max-age=31536000, immutable':'no-cache'});
  if(req.method==='HEAD')res.end();else createReadStream(path).pipe(res);
 }catch(error){res.writeHead(404).end(String(error));}
}).listen(port,'127.0.0.1',()=>console.log(`Sampler benchmark: http://127.0.0.1:${port}`));
