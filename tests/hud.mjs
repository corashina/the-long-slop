import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import test from 'node:test';

function fixture(){
 const elements=new Map();
 const element=()=>({hidden:true,value:0,textContent:'',style:{},children:[],addEventListener(){},append(node){this.children.push(node);}});
 const document={querySelector(selector){if(!elements.has(selector))elements.set(selector,element());return elements.get(selector);},createElement:element,addEventListener(){}};
 const window={addEventListener(){}};
 const layout={canvas:{resolution:[1920,1080],mode:0},ancestry:[],font_size:20,font:'Arial',color:[1,1,1,1],image_color:[1,1,1,1],sprite:null};
 const ui={layouts:Object.fromEntries(['reticle','use','pickup','mount','fill','carry','fluid'].map(role=>[role,layout])),urls:['icon'],icons:Object.fromEntries(['default','pickup','dismount','carrying','cap','mount','fill','enter','exit','door'].map(role=>[role,0]))};
 const source=path=>readFileSync(new URL(path,import.meta.url),'utf8').replace(/^import .*;\r?\n/gm,'').replace(/export /g,'');
 const main=source('../web/main.js').replace('load().catch(error=>fail(error.message));','');
 runInNewContext(source('../web/save-storage.js')+'\n'+source('../web/save-session.js')+'\n'+source('../web/save-bridge.js')+'\n'+source('../web/ui-layout.js')+'\n'+source('../web/input.js')+'\n'+source('../web/feedback.js')+'\n'+main+'\nfeedback=createFeedback(ui);feedback.setVisible(true);',{document,window,TextEncoder,innerWidth:1280,innerHeight:720,ui});
 const update=(held,charge,active,mask=8192)=>window.gameFeedback(mask,8,'Rim','',Boolean(held),0,0,false,true,'',.7,held,charge,active,800,true,true,false,false,true,.5,0,0,0,0,0);
 return {elements,window,update};
}

test('HUD displays committed throw charge and clears it after release or cancellation',()=>{
 const {elements,window,update}=fixture();
 update('Rim',.45,true);
 assert.equal(window.gameState.throwCharge,.45);
 assert.equal(window.gameState.throwActive,true);
 assert.equal(window.gameState.heldItem,'Rim');
 assert.equal(window.gameState.rpm,800);
 assert.equal(elements.get('#throw-charge').hidden,false);
 assert.equal(elements.get('#throw-level').value,.45);
 update('Rim',0,false);
 assert.equal(elements.get('#throw-charge').hidden,true);
 assert.equal(elements.get('#throw-level').value,0);
 update('Rim',1.02,true);
 assert.equal(elements.get('#throw-level').value,1);
 update('',0,false);
 assert.equal(window.gameState.throwActive,false);
 assert.equal(elements.get('#throw-charge').hidden,true);
});

test('merged feedback ABI preserves source fluids and independent action rows',()=>{
 const {elements,window}=fixture();
 window.gameFeedback(2|32|262144,7,'Left window','',false,.5,39,true,false,'Fuel Tank',.7,'',0,false,800,true,true,false,true,true,.5,0,0,0,0,0);
 assert.equal(elements.get('#fluid-text').textContent,'Fuel Tank:\n0.5 / 39 L\n100% Gas');
 assert.equal(elements.get('#target-name').textContent,'Left window');
 assert.equal(elements.get('#target-condition').textContent,'Condition 70%');
 const rows=elements.get('#hud').children.filter(row=>!row.hidden);
 assert.equal(rows.length,3);
 const use=rows.filter(row=>row.textContent.includes('[Mouse]')||row.textContent.startsWith('[Scroll]'));
 assert.equal(new Set(use.map(row=>row.style.top)).size,2);
 assert.equal(window.gameState.seated,true);
 assert.equal(window.gameState.fluidVolumes[0],.5);
});

test('source pickup restriction hides rotate help and primary switch avoids duplicate prompts',()=>{
 const {elements,window,update}=fixture();
 update('Fuel can',0,false);
 assert.doesNotMatch(elements.get('#carry-help').textContent,/Rotate|Roll/);
 update('Rim',0,false,8192|32768);
 assert.match(elements.get('#carry-help').textContent,/Rotate/);
 window.gameFeedback(1024|131072,7,'Interior light','',false,0,0,false,true,'',-1,'',0,false,0,false,false,false,false,true,0,0,0,0,0,0);
 const rows=elements.get('#hud').children.filter(row=>!row.hidden);
 assert.equal(rows.length,1);
 assert.equal(rows[0].textContent,'[E] / [LMB]: Switch');
});

test('unnamed source tank label cannot replace the resolved action owner',()=>{
 const {elements,window}=fixture();
 window.gameFeedback(1|4,2,'Fuel can','',false,.5,5,false,false,'0',-1,'',0,false,0,false,false,false,false,true,.5,0,0,0,0,0);
 assert.equal(window.gameState.target,'Fuel can');
 assert.equal(elements.get('#target-name').textContent,'Fuel can');
 assert.equal(elements.get('#fluid-text').textContent,'0.5 / 5 L\n100% Gas');
});
