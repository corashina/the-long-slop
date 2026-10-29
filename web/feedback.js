import {applyLayout} from './ui-layout.js';
export function formatGear(gear){return gear<0?'R':gear===0?'N':String(gear);}
/** Item prompt bits (Rust `ItemPrompt`). */
export const PROMPT={ATTACH:1,COIL:2,DRAIN:4,RELOAD:8,UNLOAD:16,LOOK:32,FOCUS:64};
/** Computer prompt bits (Rust `ComputerPrompt`). TYPING routes the keyboard to the computer. */
export const COMPUTER={TURN_ON:1,TURN_OFF:2,USE:4,CONNECT:8,DISCONNECT:16,TYPING:32};
export const ACTION={USE:1,DOOR:2,PICKUP:4,DROP:8,MOUNT:16,DISMOUNT:32,FILL:64,TUNE:128,ENTER:256,EXIT:512,SWITCH:1024,PUSH:2048,GRIP:4096,THROW:8192,ROTATE:32768,RELEASE_GRIP:65536,PRIMARY_SWITCH:131072,SCROLL:262144,CONSUME:524288,DRINK:1048576,HELD_SWITCH:2097152,WASH:4194304,SCRUB:8388608,EQUIP:16777216,RECENTER:33554432,RELEASE_EQUIPMENT:67108864,RELEASE_EQUIPMENT_SEATED:134217728,TAKE_ONE:268435456,SMOKE:536870912,PUT_ON:1073741824,CRANK:2147483648};
const FLUIDS=['Gas','Oil','Alcohol','Water','Blood','Diesel'];
export function formatRadio(r){if(r.context===11&&Number.isFinite(r.radioVolume))return 'Volume: '+Math.round(r.radioVolume*10).toString().padStart(2,'0');if(r.context===12&&Number.isFinite(r.radioFrequency))return (r.radioAm?'AM '+r.radioFrequency.toFixed(0).padStart(3,'0'):'FM '+r.radioFrequency.toFixed(1).padStart(4,'0'));return '';}
function volume(value){return (Math.round(value*10)/10).toFixed(1).replace(/\.0$/,'');}
export function formatTank(r) {
 if(r.noText)return '';
 if(r.context===14)return Math.round(r.amount).toString();
 const prefix=r.hasName?r.fluidName+':\n':'';
 if(r.amount<=Math.fround(.05))return prefix+'empty';
 const amount=volume(r.amount),capacity=volume(r.capacity);
 let result=prefix+(amount===capacity?amount+' L (full)':amount+' / '+capacity+' L');
 // Source GetCapString follows the tank's fluid list order: type + 1 per 4 bits, 0 ends.
 for(let order=r.fluidOrder??0;order;order>>>=4){const i=(order&15)-1,percent=Math.fround(Math.fround(r.quantities[i]/r.amount)*100);if(percent>=1)result+='\n'+Math.round(percent)+'% '+FLUIDS[i];}
 return result;
}
/** Rust `Crosshair` layer bits. E, adjust and F hold a sprite choice. */
export const CROSSHAIR={E_SHIFT:0,ADJUST_SHIFT:2,F_SHIFT:4,MMB:128,LMB:256,DISMOUNT:512,CAN_PICKUP:1024,PICKUP:2048,TOO_HEAVY:4096,DEFAULT:8192,WEAPON:16384};
const E_ICONS=['','enter','exit','switch'],ADJUST_ICONS=['','door','slide','slide-vertical'],F_ICONS=['','push','mount','hose','attach'];
/** Source `mainscript.CrossHairStuff` layers in sibling (draw) order: the sprite role, or null when hidden, and the source size. */
export function crosshairLayers(bits){
 const e=E_ICONS[bits>>CROSSHAIR.E_SHIFT&3],adjust=ADJUST_ICONS[bits>>CROSSHAIR.ADJUST_SHIFT&3],f=F_ICONS[bits>>CROSSHAIR.F_SHIFT&7];
 const on=(bit,role)=>bits&bit?role:null;
 return [[on(CROSSHAIR.DISMOUNT,'dismount'),75,40],[on(CROSSHAIR.CAN_PICKUP,'pickup'),75,40],[on(CROSSHAIR.PICKUP,'carrying'),75,40],
  [on(CROSSHAIR.DEFAULT,'default'),35,35],[on(CROSSHAIR.TOO_HEAVY,'too-heavy'),75,40],[f||null,40,40],[adjust||null,40,40],
  [on(CROSSHAIR.WEAPON,'weapon'),50,50],[e||null,40,40],[on(CROSSHAIR.MMB,'cap'),40,40],[on(CROSSHAIR.LMB,'fill'),40,40]];
}
/** Source `GCPickups` alphas under the carrying layer: medium and heavy, by `weightM`. */
export function pickupWeightAlphas(weight){
 if(!(weight<0.9))return [0,0];
 const inverse=(a,b,v)=>Math.max(0,Math.min(1,(v-a)/(b-a)));
 return [inverse(0.9,0.5,weight),inverse(0.5,0,weight)];
}
export class FeedbackState {
 constructor(){this.visible=false;this.enabled=true;this.active=false;this.fluidText='';this.fluidName=undefined;this.capacity=undefined;this.amountKey=undefined;this.percentKeys=[-1,-1,-1,-1,-1,-1];this.hasName=undefined;this.noText=undefined;this.empty=undefined;this.lmbUse=false;this.actions=-1;this.context=-1;this.carrying=false;this.notice='';this.targetName='';this.condition=-1;this.throwCharge=0;this.throwActive=false;this.prompts=0;this.computer=0;this.crosshair=0;this.weight=1;}
 setVisible(active){this.active=active;this.visible=active&&this.enabled;}
 toggle(){this.enabled=!this.enabled;this.visible=this.active&&this.enabled;}
 update(r){
  const amountKey=Math.round(r.amount*(r.context===14?1:10)),empty=r.amount<=Math.fround(.05);
  let changed=r.context!==this.context||amountKey!==this.amountKey||r.capacity!==this.capacity||r.fluidName!==this.fluidName||r.hasName!==this.hasName||r.noText!==this.noText||empty!==this.empty||r.fluidOrder!==this.fluidOrder;
  for(let i=0;i<6;i++){const percent=r.amount>0?Math.fround(Math.fround(r.quantities[i]/r.amount)*100):0;const key=percent>=1?Math.round(percent):-1;if(this.percentKeys[i]!==key){changed=true;this.percentKeys[i]=key;}}
  const radioChanged=r.radioAm!==this.radioAm||r.radioFrequency!==this.radioFrequency||r.radioVolume!==this.radioVolume;
  if(changed||radioChanged){this.fluidText=r.context===11||r.context===12?formatRadio(r):formatTank(r);this.amountKey=amountKey;this.empty=empty;this.capacity=r.capacity;this.fluidName=r.fluidName;this.hasName=r.hasName;this.noText=r.noText;this.fluidOrder=r.fluidOrder;}
  this.radioAm=r.radioAm;this.radioFrequency=r.radioFrequency;this.radioVolume=r.radioVolume;
  const targetName=r.context===11?'Volume +/-':r.context===12?'Tune':r.name;
  const detailChanged=targetName!==this.targetName||r.condition!==this.condition||r.throwCharge!==this.throwCharge||r.throwActive!==this.throwActive;
  this.targetName=targetName;this.condition=r.condition;this.throwCharge=r.throwCharge;this.throwActive=r.throwActive;
  const actionsChanged=r.actions!==this.actions||r.context!==this.context||r.carrying!==this.carrying||r.notice!==this.notice||r.lmbUse!==this.lmbUse||r.prompts!==this.prompts||r.computer!==this.computer||r.crosshair!==this.crosshair||r.weight!==this.weight;
  this.lmbUse=r.lmbUse;this.prompts=r.prompts;this.computer=r.computer;this.crosshair=r.crosshair;this.weight=r.weight;this.actions=r.actions;this.context=r.context;this.carrying=r.carrying;this.notice=r.notice;
  return changed||radioChanged||actionsChanged||detailChanged;
 }
}
export function createFeedback(ui,manifest) {
 const state=new FeedbackState(),root=document.querySelector('#hud'),icon=document.querySelector('#crosshair'),fluid=document.querySelector('#fluid-text'),carry=document.querySelector('#carry-help'),notice=document.querySelector('#action-notice'),target=document.querySelector('#target-name'),condition=document.querySelector('#target-condition'),charge=document.querySelector('#throw-charge'),level=document.querySelector('#throw-level');
 const gear=document.querySelector('#driving-gear');let lastGear,wasSeated;
 const rows=[
  [ACTION.USE,'use','[E]: Open/Close'],
  [ACTION.DOOR,'door','[LMB]+[Mouse]: Open/Close'],
  [ACTION.PICKUP,'pickup','[F]: Pick Up'],
  [ACTION.DROP,'drop','[F]: Drop'],
  [ACTION.MOUNT,'mount','[F]: Mount'],
  [ACTION.DISMOUNT,'dismount','[F]: Dismount'],
  [ACTION.FILL,'fill','[LMB]: Fill'],
  [ACTION.CONSUME,'eat','[LMB]: Eat'],
  [ACTION.SMOKE,'smoke','[LMB]: Smoke'],
  [ACTION.TAKE_ONE,'takeOne','[E]: Take one'],
  [ACTION.PUT_ON,'putOn','Tap Tab: put on'],
  [ACTION.DRINK,'drink','Hold [LMB]: Drink'],
  [ACTION.TUNE,'tune','[E]+[Mouse] / [Scroll]: Open/Close'],
  [ACTION.ENTER,'enter','Release [E]: Sit'],
  [ACTION.EXIT,'exit','[E]: Get out/stand'],
  [ACTION.SWITCH,'switch','[E]: Switch'],
  [ACTION.PRIMARY_SWITCH,'primary','[E] / [LMB]: Switch'],
  [ACTION.PUSH,'push','[F]: Push'],
  [ACTION.GRIP,'grip','[F]: Grip'],
  [ACTION.RELEASE_GRIP,'release','[WASD]: Push / [F]: Release'],
  [ACTION.THROW,'throw','[F]: Drop / hold to throw'],
  [ACTION.SCROLL,'scroll','[Scroll]: Open/Close'],
  [ACTION.HELD_SWITCH,'heldSwitch','[LMB]: Switch flashlight'],
  [ACTION.WASH,'wash','[LMB]: Wash'],
  [ACTION.SCRUB,'scrub','[LMB]: Scrub'],
  [ACTION.EQUIP,'equip','Tap Tab: equip'],
  [ACTION.RECENTER,'recenter','Hold Tab: recenter'],
  [ACTION.RELEASE_EQUIPMENT,'releaseEquipment','Tab or MMB: release'],
  [ACTION.RELEASE_EQUIPMENT_SEATED,'releaseEquipmentSeated','MMB: release'],
  [ACTION.CRANK,'crank','[E]+[Mouse]: Crank'],
  [0,'hoseAttach','[F]: '+ui.hose_prompts.attach,PROMPT.ATTACH],
  [0,'coilUp','[R]: '+ui.hose_prompts.coilup,PROMPT.COIL],
  [0,'drain','[LMB]: '+ui.hose_prompts.drain,PROMPT.DRAIN],
  [0,'reload','[R]: '+ui.ammo_prompts.reload,PROMPT.RELOAD],
  [0,'unload','[E]: '+ui.ammo_prompts.unload,PROMPT.UNLOAD],
  [0,'binocularsUse','[LMB]: '+ui.binocular_prompts.use,PROMPT.LOOK],
  [0,'binocularsFocus','[MMB]: '+ui.binocular_prompts.focus,PROMPT.FOCUS],
  [0,'computerTurnOn','[R]: '+ui.computer_prompts.turnon,0,COMPUTER.TURN_ON],
  [0,'computerTurnOff','[R]: '+ui.computer_prompts.turnoff,0,COMPUTER.TURN_OFF],
  [0,'computerUse','[LMB]: '+ui.computer_prompts.use,0,COMPUTER.USE],
  [0,'computerConnect','[E]: '+ui.computer_prompts.connect,0,COMPUTER.CONNECT],
  [0,'computerDisconnect','[E]: '+ui.computer_prompts.disconnect,0,COMPUTER.DISCONNECT]
 ].map(([bit,role,text,prompt=0,computer=0])=>{const n=document.createElement('p');n.textContent=text;n.className='action-row';n.hidden=true;root.append(n);return {bit,prompt,computer,role,n,family:0,top:0,line:0,offset:-1};});
 const named=Object.fromEntries(rows.map(r=>[r.role,r.n]));
 // fpscontroller GCWeapon: the gun crosshair sprite tinted by its color. It is one crosshair layer.
 const weapon=document.createElement('div');weapon.id='weapon-crosshair';weapon.hidden=true;let weaponIcon=0;
 const layers=crosshairLayers(0).map((_,i)=>{if(i===7)return weapon;const n=document.createElement('img');n.alt='';n.hidden=true;return n;});
 icon.append(...layers);
 const pickupChildren=['carrying-medium','carrying-heavy'].map(role=>{const n=document.createElement('img');n.alt='';n.src=ui.urls[ui.icons[role]];n.style.opacity=0;n.hidden=true;return n;});
 layers[2].after(...pickupChildren);
 layers[3].style.opacity=ui.layouts.reticle.image_color[3];
 function showWeapon(){weapon.hidden=!(weaponIcon&&state.crosshair&CROSSHAIR.WEAPON);}
 let lastLayers=-1,lastWeight=-1,lastFluid='',lastNotice='',lastName='',lastCondition='',lastCharge=-1,lastCarry='',lastDoor='',lastUse='',lastTune='',lastScroll='',lastPrimary='',lastCrank='';
 const translationHelp='[RMB]+[Mouse]: Move\n[Scroll]: Distance\n[Q]+[Scroll]: Posture';
 const rotationHelp='[RMB]+[Mouse]: Move\n[MMB]+[Mouse]: Rotate\n[RMB]+[MMB]+[Mouse]: Roll\n[Scroll]: Distance\n[Q]+[Scroll]: Posture';
 const counts=new Uint8Array(4);
 const destinations={use:'use',door:'use',pickup:'pickup',drop:'pickup',mount:'mount',dismount:'mount',fill:'fill',eat:'use',smoke:'use',takeOne:'use',putOn:'pickup',drink:'use',tune:'use',enter:'use',exit:'use',switch:'use',primary:'use',push:'pickup',grip:'pickup',release:'pickup',throw:'pickup',scroll:'use',heldSwitch:'use',wash:'use',scrub:'use',equip:'pickup',recenter:'pickup',releaseEquipment:'pickup',releaseEquipmentSeated:'pickup',crank:'use',hoseAttach:'mount',coilUp:'use',drain:'use',reload:'use',unload:'use',computerTurnOn:'use',computerTurnOff:'use',computerUse:'use',computerConnect:'use',computerDisconnect:'use'};
 const families={use:0,pickup:1,mount:2,fill:3};
 function placeRows(){
  counts.fill(0);
  for(const row of rows){if(row.n.hidden)continue;const offset=counts[row.family]++;if(offset!==row.offset){row.n.style.top=(row.top+offset*row.line)+'px';row.offset=offset;}}
 }
 function resize(){
  for(const row of rows){const role=destinations[row.role],box=applyLayout(row.n,ui.layouts[role]);row.family=families[role];row.top=box.y;row.line=box.fontSize*1.4;row.offset=-1;}
  placeRows();
  const scale=applyLayout(icon,ui.layouts.reticle).scale;icon.style.left='50%';icon.style.top='50%';icon.style.width=icon.style.height='0';
  for(const [layer,[,width,height]] of layers.map((n,i)=>[n,crosshairLayers(0)[i]])){layer.style.width=width*scale+'px';layer.style.height=height*scale+'px';}
  for(const child of pickupChildren){child.style.width=layers[2].style.width;child.style.height=layers[2].style.height;}
  const fluidBox=applyLayout(fluid,ui.layouts.fluid);fluid.style.top=(fluidBox.y+fluidBox.height/2)+'px';fluid.style.height='auto';fluid.style.width='max-content';fluid.style.left='50%';
  const carryBox=applyLayout(carry,ui.layouts.carry);carry.style.left='auto';carry.style.right='8px';carry.style.width='max-content';carry.style.height='auto';carry.style.top='auto';carry.style.bottom=(innerHeight-carryBox.y-carryBox.height)+'px';
  notice.style.fontSize=ui.layouts.use.font_size*scale+'px';
  for(const n of [target,condition,charge]){n.style.fontSize=ui.layouts.use.font_size*scale+'px';n.style.left='50%';}
  target.style.top=(innerHeight/2-70*scale)+'px';condition.style.top=(innerHeight/2-40*scale)+'px';charge.style.top=(innerHeight/2+120*scale)+'px';
 }
 function paint(){
  root.hidden=!state.visible;
  for(const r of rows)r.n.hidden=r.computer?!(state.computer&r.computer):r.prompt?!(state.prompts&r.prompt):!(state.actions&r.bit)||(r.bit===ACTION.DROP&&Boolean(state.actions&ACTION.THROW))||(r.bit===ACTION.SWITCH&&Boolean(state.actions&ACTION.PRIMARY_SWITCH))||(r.bit===ACTION.MOUNT&&Boolean(state.prompts&PROMPT.ATTACH));
  const doorText=state.context===7?'[LMB]+[Mouse]: Move':'[LMB]+[Mouse]: Open/Close';
  if(doorText!==lastDoor){named.door.textContent=doorText;lastDoor=doorText;}
  const useText=state.context===10?(state.lmbUse&&!(state.actions&ACTION.HELD_SWITCH)?'[E] / [LMB]: Switch':'[E]: Switch'):'[E]: Open/Close';
  if(useText!==lastUse){named.use.textContent=useText;lastUse=useText;}
  const radio=state.context===11||state.context===12;
  const tuneText=radio?(state.lmbUse?'[E] / [LMB]+[Mouse]: Tune':'[E]+[Mouse]: Tune'):'[E]+[Mouse] / [Scroll]: Open/Close';
  if(tuneText!==lastTune){named.tune.textContent=tuneText;lastTune=tuneText;}
  const scrollText=radio?'[Scroll]: Tune':'[Scroll]: Open/Close';
  if(scrollText!==lastScroll){named.scroll.textContent=scrollText;lastScroll=scrollText;}
  const primaryText=state.actions&ACTION.HELD_SWITCH?'[E]: Switch':'[E] / [LMB]: Switch';
  if(primaryText!==lastPrimary){named.primary.textContent=primaryText;lastPrimary=primaryText;}
  const crankText=state.lmbUse?'[E] / [LMB]+[Mouse]: Crank':'[E]+[Mouse]: Crank';
  if(crankText!==lastCrank){named.crank.textContent=crankText;lastCrank=crankText;}
  placeRows();
  if(state.targetName!==lastName){target.textContent=state.targetName;lastName=state.targetName;}target.hidden=!state.targetName;
  const detail=state.condition>=0?'Condition '+Math.round(state.condition*100)+'%':'';
  if(detail!==lastCondition){condition.textContent=detail;lastCondition=detail;}condition.hidden=!detail;
  charge.hidden=!state.throwActive;const strength=Math.max(0,Math.min(1,state.throwCharge||0));
  if(strength!==lastCharge){level.value=strength;lastCharge=strength;}
  carry.hidden=!state.carrying;const help=state.actions&ACTION.ROTATE?rotationHelp:translationHelp;
  if(help!==lastCarry){carry.textContent=help;lastCarry=help;}
  if(state.crosshair!==lastLayers){lastLayers=state.crosshair;crosshairLayers(state.crosshair).forEach(([role],i)=>{const layer=layers[i];if(layer===weapon)return;layer.hidden=!role;if(role&&layer.dataset.role!==role){layer.src=ui.urls[ui.icons[role]];layer.dataset.role=role;}});for(const child of pickupChildren)child.hidden=layers[2].hidden;showWeapon();}
  if(state.weight!==lastWeight){lastWeight=state.weight;pickupWeightAlphas(state.weight).forEach((alpha,i)=>{pickupChildren[i].style.opacity=alpha;});}
  if(state.fluidText!==lastFluid){fluid.textContent=state.fluidText;lastFluid=state.fluidText;}fluid.hidden=!state.fluidText;
  const message=state.context===6?'Unlimited Gas':state.notice;
  if(message!==lastNotice){notice.textContent=message;lastNotice=message;}notice.hidden=!message;
 }
 function setWeapon(sprite,color){
  if(sprite!==weaponIcon){weaponIcon=sprite;const url=sprite?ui.urls[sprite-1]:'';weapon.style.maskImage=weapon.style.webkitMaskImage=url?`url("${url}")`:'none';showWeapon();}
  weapon.style.backgroundColor=`rgba(${color&255},${color>>>8&255},${color>>>16&255},${(color>>>24&255)/255})`;
 }
 const inventory=document.createElement('div');inventory.id='inventory';inventory.setAttribute('aria-label','Inventory');root.append(inventory);
 const slotNodes=Array.from({length:4},(_,i)=>{const tile=document.createElement('div'),number=document.createElement('span'),icon=document.createElement('img');number.textContent=String(i+1);icon.hidden=true;tile.append(number,icon);inventory.append(tile);return {tile,icon,id:0};});
 const itemIcons=new Map(manifest.items.filter(i=>i.pickup?.can_inventory).map(i=>[i.id,{name:i.name,url:ui.urls[ui.sprites.findIndex(s=>s.source.file===i.pickup.inventory_icon?.file&&s.source.path_id===i.pickup.inventory_icon?.path_id)]}]));
 const swap=document.createElement('progress');swap.max=1;swap.setAttribute('aria-label','Swap progress');inventory.append(swap);
 function updateInventory(r){
  for(let i=0;i<4;i++){const n=slotNodes[i],id=r.slots[i];n.tile.classList.toggle('selected',r.selected===i);if(n.id!==id){n.id=id;const item=itemIcons.get(id);n.icon.hidden=!item;if(item){n.icon.src=item.url;n.icon.alt=item.name;}}}
  swap.hidden=!(r.progress>0);swap.value=Math.min(1,r.progress);
 }
 const key=event=>{if(event.code==='Backspace'&&!event.repeat&&state.active){event.preventDefault();state.toggle();paint();}};
 window.addEventListener('keydown',key);window.addEventListener('resize',resize);resize();paint();
 return {state,inventory:updateInventory,weapon:setWeapon,drivingState(value,seated){if(value!==lastGear){gear.textContent='Gear: '+formatGear(value);lastGear=value;}if(seated!==wasSeated){gear.hidden=!seated;wasSeated=seated;}},update(record){if(state.update(record))paint();},setVisible(value){const before=state.visible;state.setVisible(value);if(before!==state.visible)paint();},dispose(){window.removeEventListener('keydown',key);window.removeEventListener('resize',resize);}};
}
