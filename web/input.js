const BITS={KeyW:1,KeyS:2,KeyA:4,KeyD:8,Space:16,KeyE:32,ShiftLeft:64,ShiftRight:64,ControlLeft:128,ControlRight:128,KeyF:256,Mouse0:512,KeyI:1024,KeyC:2048,Mouse1:4096,KeyR:8192,Mouse2:16384,Tab:32768,KeyL:65536,KeyH:131072,Numpad8:131072,KeyV:262144,KeyP:524288,KeyB:1048576};
const PLAYER_BITS={KeyQ:1,KeyZ:2,KeyY:2,CapsLock:4,KeyX:8};
// Typing keys (Rust TypingInput): piano bits 0-16 (computerscript order), arrows 17-20, music keys A S X 21-23, any key 24.
const TYPING_HELD={KeyQ:1,Digit2:2,KeyW:4,Digit3:8,KeyE:16,KeyR:32,Digit5:64,KeyT:128,Digit6:256,KeyY:512,KeyZ:512,Digit7:1024,KeyU:2048,KeyI:4096,Digit9:8192,KeyO:16384,Digit0:32768,KeyP:65536,ArrowUp:1<<17,ArrowDown:1<<18,ArrowLeft:1<<19,ArrowRight:1<<20};
const TYPING_PRESSED={KeyA:1<<21,KeyS:1<<22,KeyX:1<<23},TYPING_ANY=1<<24,TEXT_CAPACITY=64;
// Driving actions that are player bits (crates/simulation/src/simulation.rs player_keys).
const PLAYER_ACTIONS={mouse_drive:16,point:32};
const DRIVING_BITS={clutch:2**21,shift_up:2**22,shift_down:2**23,reverse:2**24,neutral:2**25,gear1:2**26,gear2:2**27,gear3:2**28,gear4:2**29,gear5:2**30,gear6:2**31};
export class InputState {
 constructor(bindings){this.typing=false;this.typedCodes=new Set();this.text=new Uint8Array(TEXT_CAPACITY);this.textLength=0;this.typingPressed=0;this.inventoryBits={Digit1:1,Digit2:2,Digit3:4,Digit4:8};this.bits={...BITS};this.playerBits={...PLAYER_BITS};for(const {action,codes} of bindings){if(action in PLAYER_ACTIONS){for(const code of codes)this.playerBits[code]=(this.playerBits[code]??0)|PLAYER_ACTIONS[action];continue;}if(!(action in DRIVING_BITS))throw Error('Unknown driving action: '+action);for(const code of codes)this.bits[code]=((this.bits[code]??0)|DRIVING_BITS[action])>>>0;}this.codes=new Set();this.frame={held:0,pressed:0,released:0,mouseX:0,mouseY:0,wheel:0,inventoryHeld:0,inventoryReleased:0,playerHeld:0,playerPressed:0,playerReleased:0};this.clear();}
 code(code,down){if(!(code in this.bits)&&!(code in this.inventoryBits)&&!(code in this.playerBits))return false;const old=this.held,oldInventory=this.inventoryHeld,oldPlayer=this.playerHeld;if(down)this.codes.add(code);else this.codes.delete(code);this.held=this.inventoryHeld=this.playerHeld=0;for(const k of this.codes){this.held=(this.held|(this.bits[k]??0))>>>0;this.inventoryHeld|=this.inventoryBits[k]??0;this.playerHeld|=this.playerBits[k]??0;}this.playerPressed|=this.playerHeld&~oldPlayer;this.playerReleased|=oldPlayer&~this.playerHeld;this.inventoryReleased|=oldInventory&~this.inventoryHeld;this.pressed=(this.pressed|(this.held&~old))>>>0;this.released=(this.released|(old&~this.held))>>>0;return true;}
 mouse(x,y){if(Number.isFinite(x))this.mouseX+=x;if(Number.isFinite(y))this.mouseY+=y;}
 scroll(value){if(Number.isFinite(value))this.wheel+=value;}
 /** Route the keyboard to a computer (Rust ComputerPrompt.TYPING). Game keys release. */
 setTyping(on){if(on===this.typing)return;this.typing=on;this.clear();}
 /** Input.inputString and the held typing keys. Repeats only add space and Backspace. */
 type(code,key,repeat){if(!repeat){this.typedCodes.add(code);this.typingPressed|=TYPING_PRESSED[code]??0;}const c=key==='Enter'?13:key==='Backspace'?8:key.length===1?Math.min(key.charCodeAt(0),127):0;if(c&&(!repeat||c===32||c===8)&&this.textLength<TEXT_CAPACITY)this.text[this.textLength++]=c;}
 untype(code){this.typedCodes.delete(code);}
 typingKeys(){let keys=this.typingPressed|(this.typedCodes.size?TYPING_ANY:0);for(const c of this.typedCodes)keys|=TYPING_HELD[c]??0;this.typingPressed=0;return keys;}
 takeText(buffer){const n=Math.min(this.textLength,buffer.length);buffer.set(this.text.subarray(0,n));this.textLength=0;return n;}
 clear(){this.typedCodes.clear();this.textLength=0;this.typingPressed=0;this.codes.clear();this.inventoryHeld=this.inventoryReleased=0;this.playerHeld=this.playerPressed=this.playerReleased=0;this.held=this.pressed=this.released=this.mouseX=this.mouseY=this.wheel=0;}
 takeFrame(){const f=this.frame;f.held=this.held;f.pressed=this.pressed;f.released=this.released;f.mouseX=this.mouseX;f.mouseY=this.mouseY;f.wheel=this.wheel;f.inventoryHeld=this.inventoryHeld;f.inventoryReleased=this.inventoryReleased;f.playerHeld=this.playerHeld;f.playerPressed=this.playerPressed;f.playerReleased=this.playerReleased;this.playerPressed=this.playerReleased=0;this.inventoryReleased=0;this.pressed=this.released=this.mouseX=this.mouseY=this.wheel=0;return f;}
}
export function createInput(canvas,acceptInput,bindings,profile) {
 const state=new InputState(bindings);const listeners=[];
 let sequence=0,frameSequence=0;
 const mark=e=>{if(profile?.enabled)profile.input(++sequence,e.timeStamp,e.isTrusted);};
 const clear=()=>{state.clear();frameSequence=sequence;profile?.clearInput();};
 const listen=(node,event,handler,options)=>{node.addEventListener(event,handler,options);listeners.push([node,event,handler,options]);};
  listen(window,'keydown',e=>{if(acceptInput()&&state.typing&&e.code!=='Escape'){e.preventDefault();e.stopImmediatePropagation();state.type(e.code,e.key,e.repeat);mark(e);return;}if(acceptInput() && (e.code in state.bits||e.code in state.inventoryBits||e.code in state.playerBits)){e.preventDefault();if(!e.repeat){state.code(e.code,true);mark(e);}}});
  listen(window,'keyup',e=>{state.untype(e.code);if((e.code in state.bits||e.code in state.inventoryBits||e.code in state.playerBits)){if(acceptInput())e.preventDefault();state.code(e.code,false);mark(e);}});
 listen(canvas,'mousedown',e=>{if(acceptInput() && e.button<=2){e.preventDefault();state.code('Mouse'+e.button,true);mark(e);}});
 listen(window,'mouseup',e=>{if(e.button<=2){state.code('Mouse'+e.button,false);mark(e);}});
  listen(canvas,'mousemove',e=>{if(acceptInput()){state.mouse(e.movementX*0.12,e.movementY*0.12);if(e.movementX||e.movementY)mark(e);}});
 listen(canvas,'wheel',e=>{if(acceptInput()){e.preventDefault();state.scroll(-e.deltaY*0.001);mark(e);}},{passive:false});
 listen(canvas,'contextmenu',e=>e.preventDefault());
 listen(window,'blur',clear);
 listen(document,'visibilitychange',()=>{if(document.hidden)clear();});
 return {state,clear,takeFrame:()=>{frameSequence=sequence;return state.takeFrame();},consume:step=>profile?.consume(frameSequence,step),dispose:()=>{for(const [n,e,h,o] of listeners)n.removeEventListener(e,h,o);}};
}
