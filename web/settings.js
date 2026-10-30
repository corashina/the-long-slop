// Settings store: one browser storage record of source `settingsscript.S` fields.
// `reference` is the exported `settings` record. A stored slider value is the setting: slider value * multiplier.
// `fields` adds values outside the Settings screen: name to {default, valid(value)}. Tab resets skip them.
// `storage` returns the Storage. The browser throws on access when storage is blocked.
const KEY='the-long-slop.settings';
const round=(value,digits)=>{const scale=10**digits;return Math.round(value*scale)/scale;};
export function defaultSettings(reference,fields={}){
 const values={};
 for(const [name,f] of Object.entries(fields))values[name]=f.default;
 for(const [name,s] of Object.entries(reference.sliders))values[name]=s.default*s.multiplier;
 for(const [name,t] of Object.entries(reference.toggles))values[name]=t.default;
 return values;
}
function valid(reference,fields,name,value){
 if(fields[name])return fields[name].valid(value);
 const s=reference.sliders[name];
 return s?Number.isFinite(value)&&value>=s.minimum*s.multiplier&&value<=s.maximum*s.multiplier:typeof value==='boolean';
}
// Source `slidersettingscript`: a slider move rounds when `round` is set. Every change clamps, then rounds with `round_typed`.
// `value` is in slider units. Returns the setting.
export function sliderSetting(slider,value,moved){
 if(moved&&slider.round)value=round(value,slider.round_digit);
 value=Math.min(slider.maximum,Math.max(slider.minimum,value));
 if(slider.round_typed)value=round(value,slider.round_digit);
 return value*slider.multiplier;
}
export function createSettings(storage,reference,fields={}){
 const values=defaultSettings(reference,fields),listeners=new Set();
 let stored;try{stored=JSON.parse(storage().getItem(KEY));}catch{}
 if(stored!==null&&typeof stored==='object')for(const name of Object.keys(values))if(valid(reference,fields,name,stored[name]))values[name]=stored[name];
 // Source `togglesetting`: a toggle that is off turns its children off.
 const settle=()=>{for(const [name,t] of Object.entries(reference.toggles))if(!values[name])for(const child of t.children)values[child]=false;};
 settle();
 const changed=()=>{settle();try{storage().setItem(KEY,JSON.stringify(values));}catch{}for(const listener of listeners)listener(values);};
 return {
  values,reference,
  set(name,value){if(!valid(reference,fields,name,value)||values[name]===value)return;values[name]=value;changed();},
  // Source `tab.PressedReset`: sliders reset when they are interactable, toggles always.
  reset(tab,inMenu){
   for(const [name,s] of Object.entries(reference.sliders))if(s.tab===tab&&(inMenu||!s.menu_only))values[name]=s.default*s.multiplier;
   for(const [name,t] of Object.entries(reference.toggles))if(t.tab===tab)values[name]=t.default;
   changed();
  },
  listen(listener){listeners.add(listener);},
 };
}
// Rust `PlayerSettings` JSON.
export function playerSettings(v){
 return JSON.stringify({mouse:[v.FMouseSensXY,v.FMouseSensX,v.FMouseSensY],scroll:v.FMouseSensZ,object_move:v.FMouseMove,object_rotation:v.FMouseRot,
  key_steer:v.FKeySteerSens,fov:[v.FFieldOfViewFoot,v.FFieldOfViewCar],
  invert:[v.BInvertMouseSensX,v.BInvertMouseSensY,v.BInvertMouseSensZ,v.BInvertMouseMove,v.BInvertMouseRot],head_bob:[v.BWalkHeadBob,v.BDriveHeadBob]});
}
// Source `Application.targetFrameRate`: the limit while the mouse is captured, else 30. 0 is no limit.
export function frameLimit(v,playing){return playing?(v.BEnableFpsLimit?Math.trunc(v.FFPSLimit):0):30;}
// Rust takes the settings and the frame limit with `window.gamePlayerSettings()` and `window.gameFrameLimit()` at start and after each change.
export function installSettingsBridge(target,settings,playing){
 let pending=true,limit=-1;
 settings.listen(()=>{pending=true;});
 target.gamePlayerSettings=()=>{if(!pending)return undefined;pending=false;return playerSettings(settings.values);};
 target.gameFrameLimit=()=>{const next=frameLimit(settings.values,playing());if(next===limit)return undefined;limit=next;return next;};
}
