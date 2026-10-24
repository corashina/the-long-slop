import {applyLayout} from './ui-layout.js';
export function formatGear(gear){return gear<0?'R':gear===0?'N':String(gear);}
export const ACTION={USE:1,DOOR:2,PICKUP:4,DROP:8,MOUNT:16,DISMOUNT:32,FILL:64,TUNE:128,ENTER:256,EXIT:512,SWITCH:1024,PUSH:2048,GRIP:4096,THROW:8192,ROTATE:32768,RELEASE_GRIP:65536,PRIMARY_SWITCH:131072,SCROLL:262144,CONSUME:524288,DRINK:1048576,HELD_SWITCH:2097152,WASH:4194304,SCRUB:8388608,EQUIP:16777216,RECENTER:33554432,RELEASE_EQUIPMENT:67108864,RELEASE_EQUIPMENT_SEATED:134217728,TAKE_ONE:268435456,SMOKE:536870912,PUT_ON:1073741824};
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
 for(let i=0;i<6;i++){const percent=Math.fround(Math.fround(r.quantities[i]/r.amount)*100);if(percent>=1)result+='\n'+Math.round(percent)+'% '+FLUIDS[i];}
 return result;
}
export function selectIcon(mask,context,carrying) {
 if(context===10&&(mask&ACTION.USE))return 'light_switch';
 if(mask&ACTION.MOUNT)return 'mount';
 if(mask&ACTION.DISMOUNT)return 'dismount';
 if(mask&ACTION.FILL)return 'fill';
 if(mask&(ACTION.CONSUME|ACTION.DRINK|ACTION.SMOKE|ACTION.TAKE_ONE))return 'carrying';
 if(mask&ACTION.EXIT)return 'exit';
 if(mask&ACTION.ENTER)return 'enter';
 if(mask&ACTION.DOOR)return 'door';
 if(mask&(ACTION.SWITCH|ACTION.PRIMARY_SWITCH))return 'cap';
 if(mask&(ACTION.PUSH|ACTION.GRIP|ACTION.RELEASE_GRIP))return 'pickup';
 if(mask&(ACTION.USE|ACTION.TUNE))return 'cap';
 if(mask&ACTION.PICKUP)return 'pickup';
 if(mask&ACTION.HELD_SWITCH)return 'light_switch';
 return carrying?'carrying':'default';
}
export class FeedbackState {
 constructor(){this.visible=false;this.enabled=true;this.active=false;this.fluidText='';this.fluidName=undefined;this.capacity=undefined;this.amountKey=undefined;this.percentKeys=[-1,-1,-1,-1,-1,-1];this.hasName=undefined;this.noText=undefined;this.empty=undefined;this.lmbUse=false;this.actions=-1;this.context=-1;this.carrying=false;this.notice='';this.targetName='';this.condition=-1;this.throwCharge=0;this.throwActive=false;}
 setVisible(active){this.active=active;this.visible=active&&this.enabled;}
 toggle(){this.enabled=!this.enabled;this.visible=this.active&&this.enabled;}
 update(r){
  const amountKey=Math.round(r.amount*(r.context===14?1:10)),empty=r.amount<=Math.fround(.05);
  let changed=r.context!==this.context||amountKey!==this.amountKey||r.capacity!==this.capacity||r.fluidName!==this.fluidName||r.hasName!==this.hasName||r.noText!==this.noText||empty!==this.empty;
  for(let i=0;i<6;i++){const percent=r.amount>0?Math.fround(Math.fround(r.quantities[i]/r.amount)*100):0;const key=percent>=1?Math.round(percent):-1;if(this.percentKeys[i]!==key){changed=true;this.percentKeys[i]=key;}}
  const radioChanged=r.radioAm!==this.radioAm||r.radioFrequency!==this.radioFrequency||r.radioVolume!==this.radioVolume;
  if(changed||radioChanged){this.fluidText=r.context===11||r.context===12?formatRadio(r):formatTank(r);this.amountKey=amountKey;this.empty=empty;this.capacity=r.capacity;this.fluidName=r.fluidName;this.hasName=r.hasName;this.noText=r.noText;}
  this.radioAm=r.radioAm;this.radioFrequency=r.radioFrequency;this.radioVolume=r.radioVolume;
  const targetName=r.context===11?'Volume +/-':r.context===12?'Tune':r.name;
  const detailChanged=targetName!==this.targetName||r.condition!==this.condition||r.throwCharge!==this.throwCharge||r.throwActive!==this.throwActive;
  this.targetName=targetName;this.condition=r.condition;this.throwCharge=r.throwCharge;this.throwActive=r.throwActive;
  const actionsChanged=r.actions!==this.actions||r.context!==this.context||r.carrying!==this.carrying||r.notice!==this.notice||r.lmbUse!==this.lmbUse;
  this.lmbUse=r.lmbUse;this.actions=r.actions;this.context=r.context;this.carrying=r.carrying;this.notice=r.notice;
  return changed||radioChanged||actionsChanged||detailChanged;
 }
}
export function createFeedback(ui,switchIconUrl,manifest) {
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
  [ACTION.RELEASE_EQUIPMENT_SEATED,'releaseEquipmentSeated','MMB: release']
 ].map(([bit,role,text])=>{const n=document.createElement('p');n.textContent=text;n.className='action-row';n.hidden=true;root.append(n);return {bit,role,n,family:0,top:0,line:0,offset:-1};});
 let lastIcon='',lastFluid='',lastNotice='',lastName='',lastCondition='',lastCharge=-1,lastCarry='',lastDoor='',lastUse='',lastTune='',lastScroll='',lastPrimary='';
 const translationHelp='[RMB]+[Mouse]: Move\n[Scroll]: Distance\n[Q]+[Scroll]: Posture';
 const rotationHelp='[RMB]+[Mouse]: Move\n[MMB]+[Mouse]: Rotate\n[RMB]+[MMB]+[Mouse]: Roll\n[Scroll]: Distance\n[Q]+[Scroll]: Posture';
 const counts=new Uint8Array(4);
 const destinations={use:'use',door:'use',pickup:'pickup',drop:'pickup',mount:'mount',dismount:'mount',fill:'fill',eat:'use',smoke:'use',takeOne:'use',putOn:'pickup',drink:'use',tune:'use',enter:'use',exit:'use',switch:'use',primary:'use',push:'pickup',grip:'pickup',release:'pickup',throw:'pickup',scroll:'use',heldSwitch:'use',wash:'use',scrub:'use',equip:'pickup',recenter:'pickup',releaseEquipment:'pickup',releaseEquipmentSeated:'pickup'};
 const families={use:0,pickup:1,mount:2,fill:3};
 function placeRows(){
  counts.fill(0);
  for(const row of rows){if(row.n.hidden)continue;const offset=counts[row.family]++;if(offset!==row.offset){row.n.style.top=(row.top+offset*row.line)+'px';row.offset=offset;}}
 }
 function resize(){
  for(const row of rows){const role=destinations[row.role],box=applyLayout(row.n,ui.layouts[role]);row.family=families[role];row.top=box.y;row.line=box.fontSize*1.4;row.offset=-1;}
  placeRows();
  const reticle=applyLayout(icon,ui.layouts.reticle),scale=reticle.scale;
  const active=lastIcon||'default';icon.style.width=((active==='pickup'||active==='dismount'||active==='carrying')?75:active==='default'?35:40)*scale+'px';
  icon.style.height=(active==='default'?35:40)*scale+'px';icon.style.left='50%';icon.style.top='50%';
  icon.style.opacity=active==='default'?ui.layouts.reticle.image_color[3]:1;
  const fluidBox=applyLayout(fluid,ui.layouts.fluid);fluid.style.top=(fluidBox.y+fluidBox.height/2)+'px';fluid.style.height='auto';fluid.style.width='max-content';fluid.style.left='50%';
  const carryBox=applyLayout(carry,ui.layouts.carry);carry.style.left='auto';carry.style.right='8px';carry.style.width='max-content';carry.style.height='auto';carry.style.top='auto';carry.style.bottom=(innerHeight-carryBox.y-carryBox.height)+'px';
  notice.style.fontSize=ui.layouts.use.font_size*scale+'px';
  for(const n of [target,condition,charge]){n.style.fontSize=ui.layouts.use.font_size*scale+'px';n.style.left='50%';}
  target.style.top=(innerHeight/2-70*scale)+'px';condition.style.top=(innerHeight/2-40*scale)+'px';charge.style.top=(innerHeight/2+120*scale)+'px';
 }
 function paint(){
  root.hidden=!state.visible;
  for(const r of rows)r.n.hidden=!(state.actions&r.bit)||(r.bit===ACTION.DROP&&Boolean(state.actions&ACTION.THROW))||(r.bit===ACTION.SWITCH&&Boolean(state.actions&ACTION.PRIMARY_SWITCH));
  const doorText=state.context===7?'[LMB]+[Mouse]: Move':'[LMB]+[Mouse]: Open/Close';
  if(doorText!==lastDoor){rows[1].n.textContent=doorText;lastDoor=doorText;}
  const useText=state.context===10?(state.lmbUse&&!(state.actions&ACTION.HELD_SWITCH)?'[E] / [LMB]: Switch':'[E]: Switch'):'[E]: Open/Close';
  if(useText!==lastUse){rows[0].n.textContent=useText;lastUse=useText;}
  const radio=state.context===11||state.context===12;
  const tuneText=radio?(state.lmbUse?'[E] / [LMB]+[Mouse]: Tune':'[E]+[Mouse]: Tune'):'[E]+[Mouse] / [Scroll]: Open/Close';
  if(tuneText!==lastTune){rows[9].n.textContent=tuneText;lastTune=tuneText;}
  const scrollText=radio?'[Scroll]: Tune':'[Scroll]: Open/Close';
  if(scrollText!==lastScroll){rows[18].n.textContent=scrollText;lastScroll=scrollText;}
  const primaryText=state.actions&ACTION.HELD_SWITCH?'[E]: Switch':'[E] / [LMB]: Switch';
  if(primaryText!==lastPrimary){rows[13].n.textContent=primaryText;lastPrimary=primaryText;}
  placeRows();
  if(state.targetName!==lastName){target.textContent=state.targetName;lastName=state.targetName;}target.hidden=!state.targetName;
  const detail=state.condition>=0?'Condition '+Math.round(state.condition*100)+'%':'';
  if(detail!==lastCondition){condition.textContent=detail;lastCondition=detail;}condition.hidden=!detail;
  charge.hidden=!state.throwActive;const strength=Math.max(0,Math.min(1,state.throwCharge||0));
  if(strength!==lastCharge){level.value=strength;lastCharge=strength;}
  carry.hidden=!state.carrying;const help=state.actions&ACTION.ROTATE?rotationHelp:translationHelp;
  if(help!==lastCarry){carry.textContent=help;lastCarry=help;}
  const selected=selectIcon(state.actions,state.context,state.carrying);
  if(selected!==lastIcon){icon.src=selected==='light_switch'?switchIconUrl:ui.urls[ui.icons[selected]];lastIcon=selected;resize();}
  if(state.fluidText!==lastFluid){fluid.textContent=state.fluidText;lastFluid=state.fluidText;}fluid.hidden=!state.fluidText;
  const message=state.context===6?'Unlimited Gas':state.notice;
  if(message!==lastNotice){notice.textContent=message;lastNotice=message;}notice.hidden=!message;
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
 return {state,inventory:updateInventory,drivingState(value,seated){if(value!==lastGear){gear.textContent='Gear: '+formatGear(value);lastGear=value;}if(seated!==wasSeated){gear.hidden=!seated;wasSeated=seated;}},update(record){if(state.update(record))paint();},setVisible(value){const before=state.visible;state.setVisible(value);if(before!==state.visible)paint();},dispose(){window.removeEventListener('keydown',key);window.removeEventListener('resize',resize);}};
}
