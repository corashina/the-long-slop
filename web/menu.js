import {applyLayout,layoutBox,rgba} from './ui-layout.js';
export const ACTIONS={START:1,PAUSE:2,CONTINUE:4,RESTART:8,RECOVER:16};
export class MenuState {
 constructor(){this.state='loading';this.readyFlag=false;this.started=false;this.actions=0;this.error='';this.generation=0;this.pending=false;this.origin='start-menu';}
 ready(){if(this.state==='error')return;this.readyFlag=true;this.state='start-menu';}
 fail(message){this.generation++;this.pending=false;this.readyFlag=false;this.state='error';this.error=message;this.actions=this.started?ACTIONS.PAUSE:0;}
 pause(){this.generation++;this.pending=false;this.actions=this.started?ACTIONS.PAUSE:0;if(this.state==='error')return;if(this.state==='controls'){this.origin=this.started?'pause-menu':'start-menu';return;}this.state=this.readyFlag?(this.started?'pause-menu':'start-menu'):'loading';}
 controls(){if(this.pending)return;if(this.state!=='start-menu' && this.state!=='pause-menu')return;this.origin=this.state;this.state='controls';}
 back(){if(this.state==='controls')this.state=this.origin;}
 recover(){if(this.state==='pause-menu')this.actions=ACTIONS.RECOVER;}
 takeActions(){const a=this.actions;this.actions=0;return a;}
 async capture(action,request,unlock){
  if(!this.readyFlag || this.pending || !['start-menu','pause-menu'].includes(this.state))return false;
  const token=++this.generation;this.pending=true;this.error='';
  try {try{unlock?.();}catch{}await request();if(token!==this.generation)return false;this.started=true;this.actions=action;this.state='playing';return true;}
  catch(error){if(token===this.generation){this.error='Mouse capture failed. '+error.message+' Select '+(this.started?'Continue':'Start')+' to try again.';this.state=this.started?'pause-menu':'start-menu';}return false;}
  finally {if(token===this.generation)this.pending=false;}
 }
}
export function createMenu({canvas,ui,clearInput,queueAction,audio}) {
 const model=new MenuState(),menu=document.querySelector('#menu'),controls=document.querySelector('#controls-screen');
 const initial=document.querySelector('#start-actions'),paused=document.querySelector('#pause-actions'),error=document.querySelector('#capture-error'),status=document.querySelector('#status');
 const art=document.querySelector('#menu-art'),frame=document.querySelector('#menu-frame'),hud=document.querySelector('#hud');
 const nodes=[['#start','start'],['#start-controls','start-controls'],['#resume','continue'],['#controls','controls'],['#restart','restart'],['#recover','recover'],['#back','back']];
 let lastState='',lastError='',lastTravel=-1;
 const columnWrapper=document.querySelector('.control-columns'),columns=columnWrapper.querySelectorAll(':scope > section');
 function resize(){
  const narrow=innerWidth<800;menu.classList.toggle('narrow',narrow);
  for(const [selector,role] of nodes){const n=document.querySelector(selector);applyLayout(n,ui.layouts[role],ui.urls);if(ui.layoutUrls[role]){n.style.backgroundImage='url("'+ui.layoutUrls[role]+'")';n.style.backgroundColor='transparent';}if(narrow&&selector!=='#back'){Object.assign(n.style,{left:'auto',top:'auto',width:'100%',height:'44px',fontSize:'18px'});}}
  for(let i=0;i<columns.length;i++){
   const role=i===0?'foot':'driving',column=columns[i],layout=ui.layouts[role],box=layoutBox(layout,innerWidth,innerHeight);
   if(narrow){Object.assign(column.style,{position:'relative',left:'auto',top:'auto',width:'100%',height:'auto',overflow:'visible'});}
   else {Object.assign(column.style,{position:'absolute',left:box.x+'px',top:box.y+'px',width:box.width+'px',height:Math.max(100,innerHeight-box.y-110)+'px',overflow:'auto'});}
   column.querySelector('dl').style.fontSize=(narrow?16:box.fontSize)+'px';
   column.querySelector('dl').style.color=rgba(layout.color);
  }
 }
 function paint(){
  const state=model.state;menu.hidden=state==='playing';controls.hidden=state!=='controls';initial.hidden=!['loading','start-menu','error'].includes(state);paused.hidden=state!=='pause-menu';
  if(hud&&state!=='playing')hud.hidden=true;
  for(const id of ['#start','#resume','#restart'])document.querySelector(id).disabled=!model.readyFlag||model.pending;
  document.querySelector('#start-controls').disabled=!model.readyFlag||model.pending;document.querySelector('#recover').disabled=!model.readyFlag;
  error.hidden=!model.error;if(lastError!==model.error){error.textContent=model.error;lastError=model.error;}
  if(state!==lastState){
   if(state==='start-menu'||state==='pause-menu'){
    const pair=ui.pairUrls[Math.floor(Math.random()*ui.pairUrls.length)];art.style.backgroundImage='url("'+pair.background+'")';frame.style.backgroundImage='url("'+pair.frame+'")';
    queueMicrotask(()=>document.querySelector(state==='pause-menu'?'#resume':'#start').focus({preventScroll:true}));
   } else if(state==='controls')queueMicrotask(()=>document.querySelector('#back').focus({preventScroll:true}));
   lastState=state;
  }
 }
 function flush(){const a=model.takeActions();if(a)queueAction(a);}
 function pause(){audio?.pause();clearInput();model.pause();if(document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();}
 async function capture(action){if(action===ACTIONS.RESTART)audio?.restart(audio.session+1);clearInput();const pending=model.capture(action,async()=>{canvas.focus();await canvas.requestPointerLock();if(document.pointerLockElement!==canvas)throw Error('The browser did not capture the mouse.');},()=>audio?.unlock());paint();const success=await pending;if(!success)audio?.pause();if(!success && document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();}
 document.querySelector('#start').addEventListener('click',()=>capture(ACTIONS.START));
 document.querySelector('#resume').addEventListener('click',()=>capture(ACTIONS.CONTINUE));
 document.querySelector('#restart').addEventListener('click',()=>capture(ACTIONS.RESTART));
 for(const id of ['#controls','#start-controls'])document.querySelector(id).addEventListener('click',()=>{model.controls();paint();});
 document.querySelector('#back').addEventListener('click',()=>{model.back();paint();});
 document.querySelector('#recover').addEventListener('click',()=>{clearInput();model.recover();flush();paint();});
 const fullscreen=document.querySelector('#fullscreen');fullscreen.disabled=!document.fullscreenEnabled;
 fullscreen.addEventListener('click',async()=>{try{if(document.fullscreenElement)await document.exitFullscreen();else await document.querySelector('main').requestFullscreen();}catch{model.error='Fullscreen is unavailable.';paint();}});
 document.addEventListener('fullscreenchange',()=>{fullscreen.textContent=document.fullscreenElement?'Exit fullscreen':'Fullscreen';resize();});
 window.addEventListener('keydown',event=>{if(event.code!=='Escape'||event.repeat)return;event.preventDefault();if(model.state==='controls'){model.back();paint();}else if(model.state==='playing'||model.pending)pause();});
 document.addEventListener('pointerlockchange',()=>{if(document.pointerLockElement!==canvas && model.state==='playing')pause();});
 window.addEventListener('blur',pause);document.addEventListener('visibilitychange',()=>{if(document.hidden)pause();});
 window.addEventListener('resize',resize);resize();paint();
 return {model,ready(){model.ready();status.textContent='The garage is ready.';paint();},sync(started,stopped,travelM){model.started=started;if(stopped && started && model.state==='playing'){model.pause();paint();}const km=Math.round(travelM/10)/100;if(lastTravel!==km){document.querySelector('#session-distance').textContent='Session distance: '+km.toFixed(2)+' km';lastTravel=km;}},fail(message){clearInput();model.fail(message);if(document.pointerLockElement===canvas)document.exitPointerLock();flush();paint();},dispose(){window.removeEventListener('resize',resize);}};
}
