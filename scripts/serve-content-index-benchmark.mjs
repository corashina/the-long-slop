import {createServer} from 'node:http';
import {createReadStream} from 'node:fs';
import {stat,mkdir,realpath,writeFile,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,dirname,extname,relative,isAbsolute,sep} from 'node:path';
import {fileURLToPath} from 'node:url';

const project=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const types={'.css':'text/css','.ttf':'font/ttf','.otf':'font/otf','.glb':'model/gltf-binary','.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm','.json':'application/json','.png':'image/png','.gltf':'model/gltf+json','.wav':'audio/wav'};
const inside=(root,path)=>{const local=relative(root,path);return local===''||(!isAbsolute(local)&&local!=='..'&&!local.startsWith('..'+sep));};
async function directory(path){const root=await realpath(path);if(!(await stat(root)).isDirectory())throw Error(`Not a directory: ${path}`);return root;}
async function mountedFile(root,name){
 const path=resolve(root,name);
 if(!inside(root,path))throw Error('Invalid path');
 const actual=await realpath(path);
 if(!inside(root,actual))throw Error('Invalid linked path');
 const file=await stat(actual);
 if(!file.isFile())throw Error('Not a file');
 return {path:actual,size:file.size};
}
async function sha256(root,name){const file=await mountedFile(root,name),hash=createHash('sha256');for await(const chunk of createReadStream(file.path))hash.update(chunk);return hash.digest('hex');}
async function scriptHashes(root,excluded=[]){
 const entries=await readdir(root,{recursive:true,withFileTypes:true}),names=entries.filter(entry=>entry.isFile()&&/\.(?:m?js|wasm)$/.test(entry.name)).map(entry=>relative(root,resolve(entry.parentPath,entry.name)).split(sep).join('/')).filter(name=>!excluded.includes(name.split('/')[0])).sort();
 return Object.fromEntries(await Promise.all(names.map(async name=>[name,await sha256(root,name)])));
}

export async function createContentIndexBenchmarkServer({webRoot,scriptsRoot,assetRoot,packageRoot,resultsRoot=resolve(project,'.cache/benchmark/content-index/results'),variant,commit,buildProfile='release'}={}) {
 if(!['baseline','candidate','baseline-recheck'].includes(variant))throw Error('BENCH_VARIANT must be baseline, candidate, or baseline-recheck');
 if(!webRoot||!assetRoot||!packageRoot||!scriptsRoot)throw Error('BENCH_WEB_ROOT, BENCH_ASSET_ROOT, BENCH_PACKAGE_ROOT, and BENCH_SCRIPTS_ROOT are required');
 if(!/^[a-f0-9]{40}$/.test(commit??''))throw Error('BENCH_COMMIT must identify the frozen build');
 const [web,assets,pkg,scripts]=await Promise.all([directory(webRoot),directory(assetRoot),directory(packageRoot),directory(scriptsRoot)]);
 const requestedResults=resolve(resultsRoot);
 if([web,assets,pkg,scripts].some(root=>inside(root,requestedResults)))throw Error('Receipts must be outside served directories');
 await mkdir(requestedResults,{recursive:true});
 const results=await directory(requestedResults);
 if([web,assets,pkg,scripts].some(root=>inside(root,results)))throw Error('Receipts must be outside served directories');
 const [manifestSha256,webHashes,runnerHashes,packages]=await Promise.all([sha256(assets,'reference/manifest.json'),scriptHashes(web,['assets','pkg']),scriptHashes(scripts),scriptHashes(pkg)]);
 const configuration={variant,commit,buildProfile,webRoot:web,scriptsRoot:scripts,assetRoot:assets,packageRoot:pkg,resultsRoot:results,manifestSha256,identity:{web:webHashes,scripts:runnerHashes,packages}};
 return createServer(async(request,response)=>{
  try {
   const pathname=decodeURIComponent(new URL(request.url,'http://localhost').pathname);
   if(request.method==='POST'&&/^\/__benchmark-result\/[a-z0-9][a-z0-9-]{0,95}$/.test(pathname)){
    const chunks=[];let size=0;
    for await(const chunk of request){size+=chunk.length;if(size>8*1024*1024){response.writeHead(413).end('Receipt too large');return;}chunks.push(chunk);}
    let receipt;
    try{receipt=JSON.parse(Buffer.concat(chunks).toString('utf8'));}catch{response.writeHead(400).end('Invalid JSON');return;}
    const label=pathname.split('/').at(-1);
    if(!receipt||typeof receipt!=='object'||Array.isArray(receipt)||receipt.schemaVersion!==2||receipt.label!==label||!['garage','driving','startup-cold','startup-warm','interaction','save-load'].includes(receipt.scenario)||typeof receipt.instrument!=='boolean'||typeof receipt.profileCpu!=='boolean'||typeof receipt.profileStreaming!=='boolean'||!['cold','warm','unknown'].includes(receipt.cacheState)||JSON.stringify(receipt.configuration)!==JSON.stringify(configuration)){
     response.writeHead(400).end('Invalid receipt');return;
    }
    const path=resolve(results,`${variant}-${label}.json`);
    await writeFile(path,JSON.stringify({...receipt,server:configuration},null,2)+'\n',{flag:'wx'});
    response.writeHead(201,{'Content-Type':'application/json','Cache-Control':'no-store'}).end(JSON.stringify({path}));return;
   }
   if(!['GET','HEAD'].includes(request.method)){response.writeHead(405,{Allow:'GET, HEAD'}).end();return;}
   if(pathname==='/__benchmark-config'){
    response.writeHead(200,{'Content-Type':'application/json','Cache-Control':'no-store'}).end(request.method==='HEAD'?undefined:JSON.stringify(configuration));return;
   }
   let mount=web,name=pathname==='/'?'index.html':pathname.slice(1);
   if(pathname==='/__measure.mjs'){mount=scripts;name='measure-content-index.mjs';}
   else if(pathname==='/__input.mjs'){mount=scripts;name='benchmark-input.mjs';}
   else if(pathname.startsWith('/pkg/')){mount=pkg;name=pathname.slice(5);}
   else if(pathname.startsWith('/assets/')){mount=assets;name=pathname.slice(8);}
   const file=await mountedFile(mount,name);
   response.writeHead(200,{'Content-Type':types[extname(file.path)]??'application/octet-stream','Content-Length':file.size,
    'Cache-Control':pathname.startsWith('/assets/runtime/')?'public,max-age=31536000,immutable':'no-cache'});
   if(request.method==='HEAD'){response.end();return;}
   createReadStream(file.path).on('error',()=>response.destroy()).pipe(response);
  } catch(error) {
   const status=error.code==='EEXIST'?409:error.code==='ENOENT'||error.code==='ENOTDIR'?404:400;
   if(!response.headersSent)response.writeHead(status).end(status===409?'Receipt already exists':status===404?'Not found':'Invalid request');
   else response.destroy();
  }
 });
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const port=Number(process.env.PORT??4173);
 if(!Number.isInteger(port)||port<1||port>65535)throw Error('PORT must be between 1 and 65535');
 const server=await createContentIndexBenchmarkServer({webRoot:process.env.BENCH_WEB_ROOT,assetRoot:process.env.BENCH_ASSET_ROOT,packageRoot:process.env.BENCH_PACKAGE_ROOT,
  scriptsRoot:process.env.BENCH_SCRIPTS_ROOT,commit:process.env.BENCH_COMMIT,resultsRoot:process.env.BENCH_RESULTS_ROOT,variant:process.env.BENCH_VARIANT});
 server.listen(port,'127.0.0.1',()=>console.log(`Content index ${process.env.BENCH_VARIANT}: http://127.0.0.1:${port}/`));
}
