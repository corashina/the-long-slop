import {applyLayout,layoutBox,rgba} from './ui-layout.js';
export const ACTIONS={PAUSE:2,CONTINUE:4,RECOVER:16};
const pendingText={save:'Saving...',load:'Loading...',newGame:'Starting a new game...'};
const resultText={save:'Save complete.',load:'Game loaded.',newGame:'New game ready.'};
function operationError(error){
 const message=error?.message??String(error);
 if(message==='No saved game.')return 'No saved game is available.';
 const fit=message.match(/Draw distance [\d.]+ m does not fit\. Largest: [\d.]+ m/);if(fit)return fit[0]+'.';
 if(message.startsWith('Incompatible save:'))return 'This save uses a different schema or source export.';
 return message;
}
export class MenuState {
 constructor(){this.state='loading';this.readyFlag=false;this.started=false;this.actions=0;this.error='';this.status='';this.generation=0;this.pending=false;this.operation='';this.origin='start-menu';this.dead=false;this.deathComplete=false;}
 ready(){if(this.state==='error')return;this.readyFlag=true;this.state=this.started?'pause-menu':'start-menu';if(!this.status||this.status==='Loading the garage...')this.status='The garage is ready.';}
 loading(){if(this.state==='error')return;if(!this.operation)this.generation++;this.pending=Boolean(this.operation);this.readyFlag=false;this.actions=0;this.state='loading';this.error='';this.status=this.operation?pendingText[this.operation]:'Loading the garage...';}
 fail(message){this.generation++;this.pending=false;this.operation='';this.readyFlag=false;this.state='error';this.error=message;this.status='';this.actions=this.started?ACTIONS.PAUSE:0;}
 pause(saveSession){if(this.operation&&this.operation!=='save'&&saveSession?.cancel()){this.generation++;this.pending=false;this.operation='';this.status='';this.error='';}if(!this.operation)this.generation++;this.pending=Boolean(this.operation);this.actions=this.started?ACTIONS.PAUSE:0;if(this.state==='error')return;if(this.state==='controls'||this.state==='game-mode'){this.origin=this.started?'pause-menu':'start-menu';return;}this.state=this.readyFlag?(this.started?'pause-menu':'start-menu'):'loading';}
 controls(){if(this.pending)return;if(this.state!=='start-menu' && this.state!=='pause-menu')return;this.origin=this.state;this.state='controls';}
 gameMode(){if(this.pending||this.state!=='start-menu')return;this.origin=this.state;this.state='game-mode';}
 back(){if(this.state==='controls'||this.state==='game-mode')this.state=this.origin;}
 recover(){if(this.state==='pause-menu'&&!this.pending&&!this.dead)this.actions=ACTIONS.RECOVER;}
 takeActions(){const a=this.actions;this.actions=0;return a;}
 async run(operation,saveSession){
  if(!this.readyFlag||this.pending||!['start-menu','pause-menu'].includes(this.state)||(operation==='save'&&!this.started))return;
  const token=++this.generation;this.pending=true;this.operation=operation;this.error='';this.status=pendingText[operation];
  try {
   await saveSession[operation]();if(token!==this.generation)return;
   if(operation!=='save'){this.started=true;this.state=this.readyFlag?'pause-menu':'loading';}
   if(operation!=='save'){this.dead=false;this.deathComplete=false;}
   this.status=resultText[operation];
  } catch(error){if(token===this.generation){this.error=operationError(error);this.status='';}}
  finally {if(token===this.generation){this.pending=false;this.operation='';}}
 }
 async capture(action,request,unlock){
  if(this.deathComplete || !this.readyFlag || !this.started || this.pending || this.state!=='pause-menu')return false;
  const token=++this.generation;this.pending=true;this.error='';
  try {try{unlock?.();}catch{}await request();if(token!==this.generation)return false;this.actions=action;this.state='playing';return true;}
  catch(error){if(token===this.generation){this.error='Mouse capture failed. '+error.message+' Select Continue to try again.';this.state='pause-menu';}return false;}
  finally {if(token===this.generation)this.pending=false;}
 }
}
export async function captureWithAudio(model,action,request,audio){const success=await model.capture(action,request,()=>audio?.unlock());if(!success)audio?.pause();return success;}
export function createMenu({canvas,ui,clearInput,queueAction,saveSession,slider,audio,drivingBindings}) {
 const model=new MenuState(),menu=document.querySelector('#menu'),controls=document.querySelector('#controls-screen'),gameMode=document.querySelector('#game-mode-screen');
 const initial=document.querySelector('#start-actions'),paused=document.querySelector('#pause-actions'),error=document.querySelector('#capture-error'),status=document.querySelector('#status');
 const art=document.querySelector('#menu-art'),frame=document.querySelector('#menu-frame'),hud=document.querySelector('#hud');
 const initialNodes=[['#new-game','start'],['#start-load','start'],['#start-controls','start-controls'],['#start-game-mode','start-controls']];
 const pauseNodes=[['#resume','continue'],['#save','continue'],['#load','continue'],['#pause-new-game','restart'],['#recover','recover'],['#controls','controls']];
 const actionNodes=[...initialNodes,...pauseNodes],backNodes=['#back','#game-mode-back'],nodes=[...actionNodes,...backNodes.map(selector=>[selector,'back'])];
 let lastState='',lastError='',lastStatus='',lastTravel=-1,pendingDriving=0;
 const clutch=document.querySelector('#automatic-clutch'),gears=document.querySelector('#automatic-gears');
 function drivingState(assists){clutch.checked=Boolean(assists&1);gears.checked=Boolean(assists&2)&&clutch.checked;gears.disabled=!clutch.checked||model.pending;clutch.disabled=model.pending;}
 function requestDriving(){
  if(model.pending||!['controls','pause-menu','start-menu'].includes(model.state))return;
  if(!clutch.checked)gears.checked=false;
  gears.disabled=!clutch.checked;
  pendingDriving=4|Number(clutch.checked)|(Number(gears.checked)<<1);
 }
 clutch.addEventListener('change',requestDriving);gears.addEventListener('change',requestDriving);
 {
  const names={ControlLeft:'Left Ctrl',ControlRight:'Right Ctrl',Numpad0:'Numpad 0',ArrowUp:'Up arrow',ArrowDown:'Down arrow',KeyR:'R',KeyN:'N',NumpadAdd:'Numpad +',NumpadSubtract:'Numpad -'};
  for(const {action,codes} of drivingBindings){const node=document.querySelector('[data-driving-binding="'+action+'"]');node.textContent=codes.map(code=>names[code]??code.replace('Digit','').replace('Numpad','Numpad ')).join(' / ');}
 }
 const columnWrapper=document.querySelector('.control-columns'),columns=columnWrapper.querySelectorAll(':scope > section');
 function resize(){
  const narrow=innerWidth<800;menu.classList.toggle('narrow',narrow);
  for(const [selector,role] of nodes){const n=document.querySelector(selector);applyLayout(n,ui.layouts[role],ui.urls);if(ui.layoutUrls[role]){n.style.backgroundImage='url("'+ui.layoutUrls[role]+'")';n.style.backgroundColor='transparent';}if(narrow&&!backNodes.includes(selector)){Object.assign(n.style,{left:'auto',top:'auto',width:'100%',height:'44px',fontSize:'18px'});}}
  if(!narrow){
   const row=layoutBox(ui.layouts.continue,innerWidth,innerHeight),gap=Math.min(16,row.height*.25),top=innerHeight-16-pauseNodes.length*row.height-(pauseNodes.length-1)*gap;
   pauseNodes.forEach(([selector],index)=>Object.assign(document.querySelector(selector).style,{left:row.x+'px',top:(top+index*(row.height+gap))+'px',width:row.width+'px',height:row.height+'px'}));
   const load=document.querySelector('#start-load'),start=layoutBox(ui.layouts.start,innerWidth,innerHeight);load.style.left=((innerWidth-start.width)/2)+'px';
   const mode=document.querySelector('#start-game-mode'),modeBox=layoutBox(ui.layouts['start-controls'],innerWidth,innerHeight);Object.assign(mode.style,{left:((innerWidth-modeBox.width)/2)+'px',top:(start.y-modeBox.height-Math.min(16,modeBox.height*.25))+'px'});
  }
  for(let i=0;i<columns.length;i++){
   const role=i===0?'foot':'driving',column=columns[i],layout=ui.layouts[role],box=layoutBox(layout,innerWidth,innerHeight);
   if(narrow){Object.assign(column.style,{position:'relative',left:'auto',top:'auto',width:'100%',height:'auto',overflow:'visible'});}
   else {Object.assign(column.style,{position:'absolute',left:box.x+'px',top:box.y+'px',width:box.width+'px',height:Math.max(100,innerHeight-box.y-110)+'px',overflow:'auto'});}
   column.querySelector('dl').style.fontSize=(narrow?16:box.fontSize)+'px';column.querySelector('dl').style.color=rgba(layout.color);
  }
 }
 function paint(){
  const state=model.state;menu.hidden=state==='playing';controls.hidden=state!=='controls';gameMode.hidden=state!=='game-mode';initial.hidden=!['loading','start-menu','error'].includes(state);paused.hidden=state!=='pause-menu';
  if(hud&&state!=='playing')hud.hidden=true;
  clutch.disabled=model.pending;gears.disabled=model.pending||!clutch.checked;
  slider.lock(model.pending||model.started||!(state==='start-menu'||state==='controls'&&model.origin==='start-menu'));
  for(const [selector] of actionNodes)document.querySelector(selector).disabled=!model.readyFlag||model.pending;
  document.querySelector('#recover').disabled ||= model.dead;
  document.querySelector('#resume').disabled ||= model.deathComplete;
  error.hidden=!model.error;if(lastError!==model.error){error.textContent=model.error;lastError=model.error;}
  if(lastStatus!==model.status){status.textContent=model.status;lastStatus=model.status;}
  if(state!==lastState){
   if(state==='start-menu'||state==='pause-menu'){
    const pair=ui.pairUrls[Math.floor(Math.random()*ui.pairUrls.length)];art.style.backgroundImage='url("'+pair.background+'")';frame.style.backgroundImage='url("'+pair.frame+'")';
    queueMicrotask(()=>document.querySelector(state==='pause-menu'?'#resume':'#new-game').focus({preventScroll:true}));
   } else if(state==='controls'||state==='game-mode')queueMicrotask(()=>document.querySelector(state==='controls'?'#back':'#game-mode-back').focus({preventScroll:true}));
   lastState=state;
  }
 }
 function flush(){const a=model.takeActions();if(a)queueAction(a);}
 function pause(cancelPending=false){audio?.pause();clearInput();model.pause(cancelPending===true?saveSession:undefined);if(document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();}
 async function operate(operation){audio?.pause();clearInput();const pending=model.run(operation,saveSession);paint();await pending;flush();paint();}
 async function capture(){clearInput();const pending=captureWithAudio(model,ACTIONS.CONTINUE,async()=>{canvas.focus();await canvas.requestPointerLock();if(document.pointerLockElement!==canvas)throw Error('The browser did not capture the mouse.');},audio);paint();const success=await pending;if(!success && document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();}
 document.querySelector('#new-game').addEventListener('click',()=>operate('newGame'));
 document.querySelector('#pause-new-game').addEventListener('click',()=>operate('newGame'));
 document.querySelector('#save').addEventListener('click',()=>operate('save'));
 for(const id of ['#start-load','#load'])document.querySelector(id).addEventListener('click',()=>operate('load'));
 document.querySelector('#resume').addEventListener('click',capture);
 for(const id of ['#controls','#start-controls'])document.querySelector(id).addEventListener('click',()=>{model.controls();paint();});
 document.querySelector('#start-game-mode').addEventListener('click',()=>{model.gameMode();paint();});
 for(const id of backNodes)document.querySelector(id).addEventListener('click',()=>{model.back();paint();});
 document.querySelector('#recover').addEventListener('click',()=>{clearInput();model.recover();flush();paint();});
 const fullscreen=document.querySelector('#fullscreen');fullscreen.disabled=!document.fullscreenEnabled;
 fullscreen.addEventListener('click',async()=>{try{if(document.fullscreenElement)await document.exitFullscreen();else await document.querySelector('main').requestFullscreen();}catch{model.error='Fullscreen is unavailable.';paint();}});
 document.addEventListener('fullscreenchange',()=>{fullscreen.textContent=document.fullscreenElement?'Exit fullscreen':'Fullscreen';resize();});
 window.addEventListener('keydown',event=>{if(event.code!=='Escape'||event.repeat)return;event.preventDefault();if(model.state==='controls'||model.state==='game-mode'){model.back();paint();}else if(model.state==='playing'||model.pending)pause(true);else if(model.state==='pause-menu')capture();});
 document.addEventListener('pointerlockchange',()=>{if(document.pointerLockElement!==canvas && model.state==='playing')pause();});
 window.addEventListener('blur',pause);document.addEventListener('visibilitychange',()=>{if(document.hidden)pause();});
 window.addEventListener('resize',resize);resize();paint();
 return {model,drivingState,takeDrivingAssists(){const request=pendingDriving;pendingDriving=0;return request;},survival(dead,due){if(dead===model.dead&&due===model.deathComplete)return;model.dead=dead;model.deathComplete=due;if(due){pause();model.status='You died. Select New game or Load.';queueMicrotask(()=>document.querySelector('#pause-new-game').focus({preventScroll:true}));}paint();},loading(){pendingDriving=0;audio?.pause();clearInput();model.loading();if(document.pointerLockElement===canvas)document.exitPointerLock();paint();},ready(){model.ready();paint();},sync(started,stopped,travelM){model.started=started;if(stopped && started && model.state==='playing'){model.pause();paint();}const km=Math.round(travelM/10)/100;if(lastTravel!==km){document.querySelector('#session-distance').textContent='Session distance: '+km.toFixed(2)+' km';lastTravel=km;}},fail(message){saveSession.cancel();clearInput();model.fail(message);if(document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();},dispose(){window.removeEventListener('resize',resize);}};
}
