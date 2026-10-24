import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createFeedback,formatTank} from '../../web/feedback.js';

function setup(t){
 const originals=new Map(['window','document','innerWidth','innerHeight'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
 class Element {style={};classList={toggle(){}};children=[];hidden=false;textContent='';append(...nodes){this.children.push(...nodes);}setAttribute(){}}
 const nodes=new Map(),document={createElement:()=>new Element(),querySelector:selector=>{if(!nodes.has(selector))nodes.set(selector,new Element());return nodes.get(selector);}};
 Object.assign(globalThis,{window:new EventTarget(),document,innerWidth:1280,innerHeight:720});
 const layout={canvas:{resolution:[1280,720],mode:0},ancestry:[],font_size:16,font:'sans-serif',color:[1,1,1,1],image_color:[1,1,1,1],sprite:null};
 const roles=['use','pickup','mount','fill','reticle','fluid','carry'];
 const ui={layouts:Object.fromEntries(roles.map(role=>[role,layout])),urls:[],icons:{},sprites:[]};
 const feedback=createFeedback(ui,'switch',{items:[]});feedback.setVisible(true);
 const record={actions:0,context:0,carrying:false,amount:0,capacity:0,hasName:false,noText:true,fluidName:'',quantities:[0,0,0,0,0,0],name:'',notice:'',condition:-1,throwCharge:0,throwActive:false};
 t.after(()=>{feedback.dispose();for(const [key,descriptor] of originals){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}});
 return {nodes,update(actions,carrying=false){feedback.update({...record,actions,carrying});return nodes.get('#hud').children.filter(n=>n.className==='action-row'&&!n.hidden).map(n=>n.textContent);}};
}

test('G08 HUD rows follow equipment availability and remove obsolete carry wording',t=>{
 const app=setup(t),equip=1<<24,recenter=1<<25,release=1<<26,seatedRelease=1<<27;
 assert.deepEqual(app.update(equip|recenter,true),['Tap Tab: equip','Hold Tab: recenter']);
 assert.doesNotMatch(app.nodes.get('#carry-help').textContent,/Tab|Center/);
 assert.match(app.nodes.get('#carry-help').textContent,/Move/);
 assert.match(app.nodes.get('#carry-help').textContent,/Distance/);
 assert.deepEqual(app.update(release),['Tab or MMB: release']);
 assert.deepEqual(app.update(seatedRelease),['MMB: release']);
 assert.deepEqual(app.update(recenter,true),['Hold Tab: recenter']);
 assert.deepEqual(app.update(0),[]);
});

test('G08 HUD rows preserve tool and carry controls',t=>{
 const app=setup(t);
 const rows=app.update((1<<25)|32768|8388608|2097152,true);
 assert.deepEqual(rows,['[LMB]: Switch flashlight','[LMB]: Scrub','Hold Tab: recenter']);
 assert.match(app.nodes.get('#carry-help').textContent,/Rotate/);
 assert.match(app.nodes.get('#carry-help').textContent,/Roll/);
 assert.doesNotMatch(app.nodes.get('#carry-help').textContent,/Tab|Center/);
});
test('F15 cap text lists fluids in the tank list order',()=>{
 // Water (3) entered first, then gas (0): order nibbles are type + 1.
 const r={noText:false,context:0,hasName:true,fluidName:'Coolant',amount:2,capacity:5,quantities:[0.5,0,0,1.5,0,0],fluidOrder:4|(1<<4)};
 assert.equal(formatTank(r),'Coolant:\n2 / 5 L\n75% Water\n25% Gas');
});
