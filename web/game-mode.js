const KEY='gameMode.biome';
export function readBiome(storage){try{const v=Number(storage.getItem(KEY));return Number.isInteger(v)&&v>=0&&v<=3?v:0;}catch{return 0;}}
export function writeBiome(storage,value){try{storage.setItem(KEY,String(value));}catch{}}
export function installGameMode(target,select){
 let storage=null;try{storage=target.localStorage;}catch{}
 select.value=String(readBiome(storage));
 select.addEventListener('change',()=>writeBiome(storage,Number(select.value)));
 target.gameBiome=()=>readBiome(storage);
}
