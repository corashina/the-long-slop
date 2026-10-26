// Driving settings from browser storage, sent to Rust as `DrivingSettings` JSON.
// `storage` returns the Storage. The browser throws on access when storage is blocked.
const KEY='the-long-slop.mouse-driving';
// `inputs` and `outputs` are the mouse X and Y sliders. `driving` is the exported `driving` record.
export function installDrivingSettings(storage,inputs,outputs,driving){
 const ranges=[driving.mouse_drive.sensitivity_x,driving.mouse_drive.sensitivity_y],wheel=driving.steering_wheel;
 const valid=value=>Array.isArray(value)&&value.length===2&&value.every((v,i)=>Number.isFinite(v)&&v>=ranges[i].minimum&&v<=ranges[i].maximum);
 let stored;try{stored=JSON.parse(storage().getItem(KEY))?.mouse;}catch{}
 // Steering wheel fields are the export defaults: no axis or button is bound.
 const settings={mouse:valid(stored)?stored:ranges.map(r=>r.default),h_shifter:wheel.h_shifter,
  axes:wheel.axes.map(a=>({bound:null,enabled:a.enabled,invert:a.invert,middle:a.middle?.default??0,dead:a.dead.default,sens:a.sens.default,max:a.max.default})),
  buttons:Array(11).fill(null)};
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
