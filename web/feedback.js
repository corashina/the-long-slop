import {applyLayout} from './ui-layout.js';
export const ACTION={USE:1,DOOR:2,PICKUP:4,DROP:8,MOUNT:16,DISMOUNT:32,FILL:64,TUNE:128,ENTER:256,EXIT:512,SWITCH:1024,PUSH:2048,GRIP:4096,THROW:8192,SPLIT:16384,ROTATE:32768,RELEASE_GRIP:65536,PRIMARY_SWITCH:131072,SCROLL:262144};
const FLUIDS=['Gas','Oil','Alcohol','Water','Blood','Diesel'];
function volume(value){return (Math.round(value*10)/10).toFixed(1).replace(/\.0$/,'');}
export function formatTank(r) {
 if(r.noText)return '';
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
 if(mask&ACTION.EXIT)return 'exit';
 if(mask&ACTION.ENTER)return 'enter';
 if(mask&ACTION.DOOR)return 'door';
 if(mask&(ACTION.SWITCH|ACTION.PRIMARY_SWITCH))return 'cap';
 if(mask&(ACTION.PUSH|ACTION.GRIP|ACTION.RELEASE_GRIP))return 'pickup';
 if(mask&(ACTION.USE|ACTION.TUNE))return 'cap';
 if(mask&ACTION.PICKUP)return 'pickup';
 return carrying?'carrying':'default';
}
export class FeedbackState {
 constructor(){this.visible=false;this.enabled=true;this.active=false;this.fluidText='';this.fluidName=undefined;this.capacity=undefined;this.amountKey=undefined;this.percentKeys=[-1,-1,-1,-1,-1,-1];this.hasName=undefined;this.noText=undefined;this.empty=undefined;this.lmbUse=false;this.actions=-1;this.context=-1;this.carrying=false;this.notice='';this.targetName='';this.condition=-1;this.throwCharge=0;this.throwActive=false;}
 setVisible(active){this.active=active;this.visible=active&&this.enabled;}
 toggle(){this.enabled=!this.enabled;this.visible=this.active&&this.enabled;}
 update(r){
  const amountKey=Math.round(r.amount*10),empty=r.amount<=Math.fround(.05);
  let changed=amountKey!==this.amountKey||r.capacity!==this.capacity||r.fluidName!==this.fluidName||r.hasName!==this.hasName||r.noText!==this.noText||empty!==this.empty;
  for(let i=0;i<6;i++){const percent=r.amount>0?Math.fround(Math.fround(r.quantities[i]/r.amount)*100):0;const key=percent>=1?Math.round(percent):-1;if(this.percentKeys[i]!==key){changed=true;this.percentKeys[i]=key;}}
  if(changed){this.fluidText=formatTank(r);this.amountKey=amountKey;this.empty=empty;this.capacity=r.capacity;this.fluidName=r.fluidName;this.hasName=r.hasName;this.noText=r.noText;}
  const detailChanged=r.name!==this.targetName||r.condition!==this.condition||r.throwCharge!==this.throwCharge||r.throwActive!==this.throwActive;
  this.targetName=r.name;this.condition=r.condition;this.throwCharge=r.throwCharge;this.throwActive=r.throwActive;
  const actionsChanged=r.actions!==this.actions||r.context!==this.context||r.carrying!==this.carrying||r.notice!==this.notice||r.lmbUse!==this.lmbUse;
  this.lmbUse=r.lmbUse;this.actions=r.actions;this.context=r.context;this.carrying=r.carrying;this.notice=r.notice;
  return changed||actionsChanged||detailChanged;
 }
}
export function createFeedback(ui,switchIconUrl) {
 const state=new FeedbackState(),root=document.querySelector('#hud'),icon=document.querySelector('#crosshair'),fluid=document.querySelector('#fluid-text'),carry=document.querySelector('#carry-help'),notice=document.querySelector('#action-notice'),target=document.querySelector('#target-name'),condition=document.querySelector('#target-condition'),charge=document.querySelector('#throw-charge'),level=document.querySelector('#throw-level');
 const rows=[
  [ACTION.USE,'use','[E]: Open/Close'],
  [ACTION.DOOR,'door','[E]+[Mouse]: Open/Close'],
  [ACTION.PICKUP,'pickup','[F]: Pick Up'],
  [ACTION.DROP,'drop','[F]: Drop'],
  [ACTION.MOUNT,'mount','[F]: Mount'],
  [ACTION.DISMOUNT,'dismount','[F]: Dismount'],
  [ACTION.FILL,'fill','[LMB]: Fill'],
  [ACTION.TUNE,'tune','[E]+[Mouse] / [Scroll]: Open/Close'],
  [ACTION.ENTER,'enter','[E]: Drive'],
  [ACTION.EXIT,'exit','[E]: Get out/stand'],
  [ACTION.SWITCH,'switch','[E]: Switch'],
  [ACTION.PRIMARY_SWITCH,'primary','[E] / [LMB]: Switch'],
  [ACTION.PUSH,'push','[F]: Push'],
  [ACTION.GRIP,'grip','[F]: Grip'],
  [ACTION.RELEASE_GRIP,'release','[WASD]: Push / [F]: Release'],
  [ACTION.THROW,'throw','[F]: Drop / hold to throw'],
  [ACTION.SPLIT,'split','[X]: Split / assemble'],
  [ACTION.SCROLL,'scroll','[Scroll]: Open/Close']
 ].map(([bit,role,text])=>{const n=document.createElement('p');n.textContent=text;n.className='action-row';n.hidden=true;root.append(n);return {bit,role,n,family:0,top:0,line:0,offset:-1};});
 let lastIcon='',lastFluid='',lastNotice='',lastName='',lastCondition='',lastCharge=-1,lastCarry='',lastDoor='',lastUse='';
 const translationHelp='[RMB]+[Mouse]: Move\n[Scroll]: Distance / [Tab]: Center';
 const rotationHelp='[RMB]+[Mouse]: Move\n[MMB]+[Mouse]: Rotate\n[RMB]+[MMB]+[Mouse]: Roll\n[Scroll]: Distance / [Tab]: Center';
 const counts=new Uint8Array(4);
 const destinations={use:'use',door:'use',pickup:'pickup',drop:'pickup',mount:'mount',dismount:'mount',fill:'fill',tune:'use',enter:'use',exit:'use',switch:'use',primary:'use',push:'pickup',grip:'pickup',release:'pickup',throw:'pickup',split:'mount',scroll:'use'};
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
  const doorText=state.context===7?'[E]+[Mouse]: Move':'[E]+[Mouse]: Open/Close';
  if(doorText!==lastDoor){rows[1].n.textContent=doorText;lastDoor=doorText;}
  const useText=state.context===10?(state.lmbUse?'[E] / [LMB]: Switch':'[E]: Switch'):'[E]: Open/Close';
  if(useText!==lastUse){rows[0].n.textContent=useText;lastUse=useText;}
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
 const key=event=>{if(event.code==='Backspace'&&!event.repeat&&state.active){event.preventDefault();state.toggle();paint();}};
 window.addEventListener('keydown',key);window.addEventListener('resize',resize);resize();paint();
 return {state,update(record){if(state.update(record))paint();},setVisible(value){const before=state.visible;state.setVisible(value);if(before!==state.visible)paint();},dispose(){window.removeEventListener('keydown',key);window.removeEventListener('resize',resize);}};
}
