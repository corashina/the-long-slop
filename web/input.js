const BITS={KeyW:1,KeyS:2,KeyA:4,KeyD:8,Space:16,KeyE:32,ShiftLeft:64,ShiftRight:64,ControlLeft:128,ControlRight:128,KeyF:256,Mouse0:512,KeyI:1024,KeyC:2048,Mouse1:4096,KeyX:8192,Mouse2:16384,Tab:32768,KeyL:65536,KeyH:131072,Numpad8:131072,KeyV:262144,KeyP:524288,KeyB:1048576};
const DRIVING_BITS={clutch:2**21,shift_up:2**22,shift_down:2**23,reverse:2**24,neutral:2**25,gear1:2**26,gear2:2**27,gear3:2**28,gear4:2**29,gear5:2**30,gear6:2**31};
export class InputState {
 constructor(bindings){this.bits={...BITS};for(const {action,codes} of bindings){if(!(action in DRIVING_BITS))throw Error('Unknown driving action: '+action);for(const code of codes)this.bits[code]=((this.bits[code]??0)|DRIVING_BITS[action])>>>0;}this.codes=new Set();this.frame={held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0};this.clear();}
 code(code,down){if(!(code in this.bits))return false;const old=this.held;if(down)this.codes.add(code);else this.codes.delete(code);this.held=0;for(const k of this.codes)this.held=(this.held|this.bits[k])>>>0;this.pressed=(this.pressed|(this.held&~old))>>>0;this.released=(this.released|(old&~this.held))>>>0;return true;}
 mouse(x,y){if(Number.isFinite(x))this.mouseX+=x;if(Number.isFinite(y))this.mouseY+=y;}
 scroll(value){if(Number.isFinite(value))this.wheel+=value;}
 clear(){this.codes.clear();this.held=this.pressed=this.released=this.mouseX=this.mouseY=this.wheel=0;}
 takeFrame(){const f=this.frame;f.held=this.held;f.pressed=this.pressed;f.released=this.released;f.mouseX=this.mouseX;f.mouseY=this.mouseY;f.wheel=this.wheel;this.pressed=this.released=this.mouseX=this.mouseY=this.wheel=0;return f;}
}
export function createInput(canvas,acceptInput,bindings) {
 const state=new InputState(bindings);const listeners=[];
 const listen=(node,event,handler,options)=>{node.addEventListener(event,handler,options);listeners.push([node,event,handler,options]);};
 listen(window,'keydown',e=>{if(acceptInput() && e.code in state.bits){e.preventDefault();if(!e.repeat)state.code(e.code,true);}});
 listen(window,'keyup',e=>{if(e.code in state.bits){if(acceptInput())e.preventDefault();state.code(e.code,false);}});
 listen(canvas,'mousedown',e=>{if(acceptInput() && e.button<=2){e.preventDefault();state.code('Mouse'+e.button,true);}});
 listen(window,'mouseup',e=>{if(e.button<=2)state.code('Mouse'+e.button,false);});
 listen(canvas,'mousemove',e=>{if(acceptInput())state.mouse(e.movementX*0.12,e.movementY*0.12);});
 listen(canvas,'wheel',e=>{if(acceptInput()){e.preventDefault();state.scroll(-e.deltaY*0.001);}},{passive:false});
 listen(canvas,'contextmenu',e=>e.preventDefault());
 listen(window,'blur',()=>state.clear());
 listen(document,'visibilitychange',()=>{if(document.hidden)state.clear();});
 return {state,clear:()=>state.clear(),takeFrame:()=>state.takeFrame(),dispose:()=>{for(const [n,e,h,o] of listeners)n.removeEventListener(e,h,o);}};
}
