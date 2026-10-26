// One reused 320x180 target. The source thumbnail is a 320x180 JPG.
export function createThumbnail(canvas){
 const offscreen=typeof OffscreenCanvas==='function',target=offscreen?new OffscreenCanvas(320,180):Object.assign(document.createElement('canvas'),{width:320,height:180}),context=target.getContext('2d');
 const encode=()=>offscreen?target.convertToBlob({type:'image/jpeg',quality:0.9}):new Promise(resolve=>target.toBlob(resolve,'image/jpeg',0.9));
 return {async capture(){try{context.drawImage(canvas,0,0,320,180);return await encode();}catch{return null;}}};
}
