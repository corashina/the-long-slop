import {applyLayout,rgba} from './ui-layout.js';
export function createSurvival(ui) {
 const root=document.querySelector('#survival'),death=document.querySelector('#death-shade'),bars=[];
 for(const role of ['hp','food','water','bowel','bladder']){
  const fill=ui.layouts['need-'+role+'-fill'];
  // Unity Slider expands its zero-value fill into the authored fill area.
  const nodes={role,direction:ui.need_sliders[role].direction,fillLayout:{...fill,ancestry:fill.ancestry.slice(0,-1)}};
  for(const part of ['background','fill','icon']){
   const node=document.createElement('div');node.className='need-part';node.setAttribute('aria-hidden','true');root.append(node);nodes[part]=node;
  }
  const meter=document.createElement('meter');meter.className='need-value';meter.min=ui.need_sliders[role].minimum;meter.max=ui.need_sliders[role].maximum;meter.setAttribute('aria-label',role);root.append(meter);nodes.meter=meter;bars.push(nodes);
 }
 function resize(){for(const b of bars)for(const part of ['background','fill','icon']){const role='need-'+b.role+'-'+part;const layout=part==='fill'?b.fillLayout:ui.layouts[role];applyLayout(b[part],layout,ui.urls);b[part].style.backgroundColor=rgba(layout.image_color);if(ui.layoutUrls[role]){b[part].style.backgroundImage='url("'+ui.layoutUrls[role]+'")';b[part].style.backgroundColor='transparent';}}}
 window.addEventListener('resize',resize);resize();
 return {update(output){root.style.opacity=output.opacity;death.style.opacity=output.dead?1-output.lookScale/.8:0;for(const b of bars){const value=output[b.role];b.meter.value=value;const fill=Math.max(0,Math.min(1,(value-b.meter.min)/(b.meter.max-b.meter.min)))*100;b.fill.style.clipPath=b.direction===0?'inset(0 '+(100-fill)+'% 0 0)':b.direction===1?'inset(0 0 0 '+(100-fill)+'%)':b.direction===2?'inset('+(100-fill)+'% 0 0 0)':'inset(0 0 '+(100-fill)+'% 0)';}},setVisible(visible){root.hidden=!visible;death.hidden=!visible;}};
}
