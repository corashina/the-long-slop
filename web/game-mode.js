const KEY='gameMode.biome',START_KEY='gameMode.startCar';
export function readBiome(storage){try{const v=Number(storage.getItem(KEY));return Number.isInteger(v)&&v>=0&&v<=3?v:0;}catch{return 0;}}
export function writeBiome(storage,value){try{storage.setItem(KEY,String(value));}catch{}}
// -1 is Random. Other values index the manifest start car fields.
export function readStartCar(storage,count){try{const raw=storage.getItem(START_KEY);const v=/^-?\d+$/.test(raw)?Number(raw):-1;return v>=-1&&v<count?v:-1;}catch{return -1;}}
export function writeStartCar(storage,value){try{storage.setItem(START_KEY,String(value));}catch{}}
function localStorageOf(target){try{return target.localStorage;}catch{return null;}}
export function installGameMode(target,select){
 const storage=localStorageOf(target);
 select.value=String(readBiome(storage));
 select.addEventListener('change',()=>writeBiome(storage,Number(select.value)));
 target.gameBiome=()=>readBiome(storage);
}
// `names` are the manifest shell start car names, in start field order.
export function installStartCar(target,select,names){
 const storage=localStorageOf(target);
 names.forEach((name,i)=>select.add(new Option(name,String(i))));
 select.value=String(readStartCar(storage,names.length));
 select.addEventListener('change',()=>writeStartCar(storage,Number(select.value)));
 target.gameStartCar=()=>readStartCar(storage,names.length);
}
