import {applyLayout,rgba} from './ui-layout.js';
export function createSurvival(ui,cooldownSeconds) {
 const root=document.querySelector('#survival'),death=document.querySelector('#death-shade'),bars=[];
 const sleepRoot=document.createElement('div');sleepRoot.id='sleep-feedback';sleepRoot.hidden=true;root.parentNode.append(sleepRoot);
 const hint=document.createElement('div');hint.id='sleep-hint';hint.className='sleep-label';hint.textContent=ui.layouts['sleep-hint'].text;hint.hidden=true;root.parentNode.append(hint);
 const sleepNodes={};let sleepVisible=false,hintVisible=false,visible=false;
 for(const role of ['can-sleep','cannot-sleep','need-sleep-background','need-sleep-fill']){
  const node=document.createElement('div');node.className=role.endsWith('sleep')?'sleep-label':'need-part';node.textContent=ui.layouts[role].text;sleepRoot.append(node);sleepNodes[role]=node;
 }
 const sourceFill=ui.layouts['need-sleep-fill'],fillAncestry=sourceFill.ancestry.slice();
 fillAncestry[fillAncestry.length-1]={...fillAncestry.at(-1),anchor_min:[0,0],anchor_max:[1,1]};
 const sleepFill={...sourceFill,ancestry:fillAncestry};
 function resizeSleep(){applyLayout(hint,ui.layouts['sleep-hint'],ui.urls);for(const [role,node] of Object.entries(sleepNodes)){const layout=role==='need-sleep-fill'?sleepFill:ui.layouts[role];applyLayout(node,layout,ui.urls);if(node.className==='need-part')node.style.backgroundColor=rgba(layout.image_color);}}
 function updateSleep(output,hudEnabled){
  sleepVisible=hudEnabled&&output.held&&!output.dead&&output.phase!==2;sleepRoot.hidden=!visible||!sleepVisible;
  hintVisible=hudEnabled&&output.showHint;hint.hidden=!visible||!hintVisible;
  sleepNodes['can-sleep'].hidden=!output.eligible;sleepNodes['cannot-sleep'].hidden=output.eligible;
  const value=output.eligible?output.charge:Math.max(0,Math.min(1,output.cooldownRemaining/cooldownSeconds));
  const slider=ui.need_sliders.sleep,fill=Math.max(0,Math.min(1,(value-slider.minimum)/(slider.maximum-slider.minimum)))*100;
  sleepNodes['need-sleep-fill'].style.clipPath=slider.direction===0?'inset(0 '+(100-fill)+'% 0 0)':slider.direction===1?'inset(0 0 0 '+(100-fill)+'%)':slider.direction===2?'inset('+(100-fill)+'% 0 0 0)':'inset(0 0 '+(100-fill)+'% 0)';
 }

 for(const role of ['hp','food','water','bowel','bladder']){
  const fill=ui.layouts['need-'+role+'-fill'];
  // Unity Slider expands its zero-value fill into the authored fill area.
  const nodes={role,direction:ui.need_sliders[role].direction,fillLayout:{...fill,ancestry:fill.ancestry.slice(0,-1)}};
  for(const part of ['background','fill','icon']){
   const node=document.createElement('div');node.className='need-part';node.setAttribute('aria-hidden','true');root.append(node);nodes[part]=node;
  }
  const meter=document.createElement('meter');meter.className='need-value';meter.min=ui.need_sliders[role].minimum;meter.max=ui.need_sliders[role].maximum;meter.setAttribute('aria-label',role);root.append(meter);nodes.meter=meter;bars.push(nodes);
 }
 function resize(){resizeSleep();for(const b of bars)for(const part of ['background','fill','icon']){const role='need-'+b.role+'-'+part;const layout=part==='fill'?b.fillLayout:ui.layouts[role];applyLayout(b[part],layout,ui.urls);b[part].style.backgroundColor=rgba(layout.image_color);if(ui.layoutUrls[role]){b[part].style.backgroundImage='url("'+ui.layoutUrls[role]+'")';b[part].style.backgroundColor='transparent';}}}
 window.addEventListener('resize',resize);resize();
 return {updateSleep,update(output){root.style.opacity=output.opacity;death.style.opacity=output.dead?1-output.lookScale/.8:0;for(const b of bars){const value=output[b.role];b.meter.value=value;const fill=Math.max(0,Math.min(1,(value-b.meter.min)/(b.meter.max-b.meter.min)))*100;b.fill.style.clipPath=b.direction===0?'inset(0 '+(100-fill)+'% 0 0)':b.direction===1?'inset(0 0 0 '+(100-fill)+'%)':b.direction===2?'inset('+(100-fill)+'% 0 0 0)':'inset(0 0 '+(100-fill)+'% 0)';}},setVisible(value){visible=value;root.hidden=!value;death.hidden=!value;sleepRoot.hidden=!value||!sleepVisible;hint.hidden=!value||!hintVisible;}};
}
