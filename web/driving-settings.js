// Driving settings from browser storage, sent to Rust as `DrivingSettings` JSON.
// `storage` returns the Storage. The browser throws on access when storage is blocked.
const KEY='the-long-slop.mouse-driving';
// `inputs` and `outputs` are the mouse X and Y sliders. `mouse` is the exported `driving.mouse_drive`.
export function installDrivingSettings(storage,inputs,outputs,mouse){
 const ranges=[mouse.sensitivity_x,mouse.sensitivity_y];
 const valid=value=>Array.isArray(value)&&value.length===2&&value.every((v,i)=>Number.isFinite(v)&&v>=ranges[i].minimum&&v<=ranges[i].maximum);
 let stored;try{stored=JSON.parse(storage().getItem(KEY))?.mouse;}catch{}
 const settings={mouse:valid(stored)?stored:ranges.map(r=>r.default)};
 let pending=true;
 ranges.forEach((range,i)=>{
  const input=inputs[i],show=()=>{outputs[i].textContent=settings.mouse[i].toFixed(2);};
  Object.assign(input,{min:String(range.minimum),max:String(range.maximum),step:'any',value:String(settings.mouse[i])});show();
  input.addEventListener('input',()=>{
   settings.mouse[i]=Math.min(range.maximum,Math.max(range.minimum,Number(input.value)));show();pending=true;
   try{storage().setItem(KEY,JSON.stringify(settings));}catch{}
  });
 });
 // Rust takes the settings at start and after each change.
 return {take(){if(!pending)return undefined;pending=false;return JSON.stringify(settings);}};
}
