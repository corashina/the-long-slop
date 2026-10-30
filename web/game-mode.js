const KEY='gameMode.biome',START_KEY='gameMode.startCar';
// Game options, as `gamemodesettingsscript`. Defaults are its `defS`. They are global, not per save.
const OPTIONS=[['peaceful','gameMode.peaceful',false],['survival','gameMode.survival',true],['homesick','gameMode.homesick',false]];
const INT_MIN=-2147483648,INT_MAX=2147483647;
export function readBiome(storage){try{const v=Number(storage.getItem(KEY));return Number.isInteger(v)&&v>=0&&v<=3?v:0;}catch{return 0;}}
export function writeBiome(storage,value){try{storage.setItem(KEY,String(value));}catch{}}
// -1 is Random. Other values index the manifest start car fields.
export function readStartCar(storage,count){try{const raw=storage.getItem(START_KEY);const v=/^-?\d+$/.test(raw)?Number(raw):-1;return v>=-1&&v<count?v:-1;}catch{return -1;}}
export function writeStartCar(storage,value){try{storage.setItem(START_KEY,String(value));}catch{}}
export function readOptions(storage){const options={};for(const [name,key,fallback] of OPTIONS){let raw=null;try{raw=storage.getItem(key);}catch{}options[name]=raw==='1'?true:raw==='0'?false:fallback;}return options;}
export function writeOption(storage,name,value){try{storage.setItem(OPTIONS.find(o=>o[0]===name)[1],value?'1':'0');}catch{}}
// Simulation bits: 1 peaceful, 2 survival. Homesick has no world effect in the original.
export const optionBits=options=>Number(options.peaceful)|Number(options.survival)<<1;
// `mainmenuscript.chSeed`: C# int.Parse, else the int limit of the sign, else 0 for an empty field.
export function parseSeed(text){
 if(/^\s*[+-]?\d+\s*$/.test(text)){const v=Number(text);if(v>=INT_MIN&&v<=INT_MAX)return v;}
 return text.length>0?(text[0]==='-'?INT_MIN:INT_MAX):0;
}
// `chnewSeed`: Random.Range(int.MinValue, int.MaxValue).
export const randomSeed=(random=Math.random)=>Math.floor(INT_MIN+random()*(INT_MAX-INT_MIN));
function localStorageOf(target){try{return target.localStorage;}catch{return null;}}
export function installGameMode(target,select){
 const storage=localStorageOf(target);
 select.value=String(readBiome(storage));
 select.addEventListener('change',()=>writeBiome(storage,Number(select.value)));
 target.gameBiome=()=>readBiome(storage);
}
// `toggles` maps option names to checkboxes. `changed` gets the options after each change.
export function installGameOptions(target,toggles,changed=()=>{}){
 const storage=localStorageOf(target),options=readOptions(storage);let pending=null;
 for(const [name,node] of Object.entries(toggles)){
  node.checked=options[name];
  node.addEventListener('change',()=>{options[name]=node.checked;writeOption(storage,name,node.checked);pending=optionBits(options);changed(options);});
 }
 target.gameOptions=()=>{const bits=pending;pending=null;return bits??undefined;};
 return {options:()=>({...options}),bits:()=>optionBits(options)};
}
// The menu rolls a new seed at each start, as `mainmenuscript.Start`.
export function installSeed(target,input,random=Math.random){
 let seed=randomSeed(random);input.value=String(seed);
 input.addEventListener('change',()=>{seed=parseSeed(input.value);input.value=String(seed);});
 target.gameSeed=()=>seed;
 return {reroll(){seed=randomSeed(random);input.value=String(seed);}};
}
// `names` are the manifest shell start car names, in start field order.
export function installStartCar(target,select,names){
 const storage=localStorageOf(target);
 names.forEach((name,i)=>select.add(new Option(name,String(i))));
 select.value=String(readStartCar(storage,names.length));
 select.addEventListener('change',()=>writeStartCar(storage,Number(select.value)));
 target.gameStartCar=()=>readStartCar(storage,names.length);
}
