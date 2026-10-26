"""Checked computer (computerscript) and TV (displayscript) inputs."""
import math
from .caps import word
from .source import key, source_id
from .survival import managed_evidence

# Catalog item index, catalog name, prefab script, and script class.
COMPUTER=(73,'gep01',133932,'computerscript')
DISPLAY=(152,'teve01',144977,'displayscript')
EVIDENCE=[
    'WriteLn("Ready.");',
    'line = new string(screenText, num, num2 - num);',
    'WriteLn2("?Syntax  Error");',
    'yield return new WaitForSeconds(0.05f);',
    'yield return new WaitForSeconds(0.01f);',
    'if (Time.time - lastCursorSwitched >= 0.35f && !musicMode)',
    'float num = Mathf.Pow(2f, (float)(note + (octave - 4) * 12) / 12f);',
    'sounds[num].pitch = num + ((UnityEngine.Random.Range(0f, 1f) < pitchDifferenceChance) ? UnityEngine.Random.Range((0f - pitchDifference) * 0.5f, pitchDifference * 0.5f) : 0f);',
    'musicSelectedOctave = Mathf.Clamp(--musicSelectedOctave, 0, 9);',
    'material.SetTexture("_EmissionMap", input);',
    'ON = !ON;']


def computer_prompts(source) -> dict:
    """fpscontroller texts: R turns a computer on or off, LMB uses it, E connects or disconnects a held one."""
    return {name: word(source, name) for name in ('turnon', 'turnoff', 'use', 'connect', 'disconnect')}


def owner(source, manifest, spec):
    """The catalog prototype of a prefab script, its reader and fields, and a link check."""
    index,name,script,cname=spec
    item=manifest['world']['catalog']['items'][index]
    if item['name']!=name or item['prototype'] is None:raise ValueError(f'Catalog item {index} {name} is not a world prototype')
    prototype=next(p for p in manifest['world']['prototypes'] if p['source']==item['prototype'])
    reader=source.get('sharedassets1.assets',script)
    if reader.type.name!='MonoBehaviour' or source.class_name(reader)!=cname:raise ValueError(f'{key(reader)} is not {cname}')
    nodes={(n['source']['file'],n['source']['path_id']) for n in prototype['nodes']}
    def node(target):
        if target.type.name not in ('Transform','RectTransform'):
            if target.type.name!='GameObject':target=source.resolve(target,source.read(target)['m_GameObject'])
            target=source.transform_for(target)
        if key(target) not in nodes:raise ValueError(f'{key(target)} lies outside catalog item {name}')
        return target
    node(reader)
    def link(fields, field, kind):
        target=source.resolve(reader,fields[field])
        if target is None or target.type.name!=kind:raise ValueError(f'{key(reader)} {field} is not {kind}')
        return target
    return {'item':index,'prototype':item['prototype'],'source':source_id(reader)},prototype,reader,source.read(reader),node,link


def display_switch(source, prototype, display, node):
    """The usablescript whose turnedEvent calls displayscript.Turned."""
    found=[]
    for n in prototype['nodes']:
        transform=source.get(n['source']['file'],n['source']['path_id'])
        for c in source.components(transform)[1]:
            if c.type.name!='MonoBehaviour' or source.class_name(c)!='usablescript':continue
            for call in source.read(c)['turnedEvent']['m_PersistentCalls']['m_Calls']:
                if call['m_CallState']==0:continue
                target=source.resolve(c,call['m_Target'])
                if target is not None and key(target)==key(display):
                    if call['m_MethodName']!='Turned' or call['m_Mode']!=1:raise ValueError(f'{key(c)} display call differs')
                    found.append(c)
    if len(found)!=1:raise ValueError('Display needs one Turned caller')
    usable=found[0];d=source.read(usable)
    if not d['turnable'] or d['slideTurnable'] or d['rotateAble'] or d['slideAble'] or d['maxTurnState']!=0 or len(d['turnClips'])!=1:
        raise ValueError(f'{key(usable)} display switch differs')
    clip=source.resolve(usable,d['turnClips'][0])
    if clip is None or clip.type.name!='AudioClip':raise ValueError(f'{key(usable)} turn clip is not AudioClip')
    source.item_clips.add(key(clip))
    return {'source':source_id(usable),'node':source_id(node(usable)),'clip':source_id(clip),'volume':d['turnVolume']}


def recover_computer(source, manifest) -> dict:
    managed_evidence(source, EVIDENCE)
    record,prototype,reader,d,node,link=owner(source,manifest,COMPUTER)
    pickup=[c for n in [source.transform_for(source.resolve(reader,d['m_GameObject']))] for c in source.components(n)[1]
        if c.type.name=='MonoBehaviour' and source.class_name(c)=='pickupable']
    if len(pickup)!=1 or key(source.resolve(pickup[0],source.read(pickup[0])['computer']))!=key(reader):
        raise ValueError(f'{key(reader)} pickupable computer link differs')
    link(d,'musicClip','AudioClip')
    if d['ON'] or d['connected'] or d['display']['m_PathID'] or d['musicMode']:raise ValueError(f'{key(reader)} start state differs')
    computer={**record,'x_size':d['xSize'],'y_size':d['ySize'],'cursor_step':[d['CursorPosM']['x'],d['CursorPosM']['y']],
        'volume':d['musicVolume'],'pitch_difference_chance':d['pitchDifferenceChance'],'pitch_difference':d['pitchDifference']}
    record,prototype,reader,d,node,link=owner(source,manifest,DISPLAY)
    renderer=link(d,'R','MeshRenderer');node(renderer)
    materials=[link(d,f,'Material') for f in ('defMaterial','OffMaterial')]
    display={**record,'screen':source_id(node(renderer)),'on_material':source_id(materials[0]),'off_material':source_id(materials[1]),
        'startup_on':bool(d['ON']),'switch':display_switch(source,prototype,reader,node)}
    return {'computer':computer,'display':display}


def _shape(value, fields):
    if type(value) is not dict or set(value)!=set(fields.split()):raise ValueError('Computer record fields differ')


def _number(*values):
    if any(type(v) not in (int,float) or type(v) is bool or not math.isfinite(v) for v in values):raise ValueError('Computer number differs')


def validate_computer(record, manifest) -> None:
    _shape(record,'computer display')
    c=record['computer']
    _shape(c,'item prototype source x_size y_size cursor_step volume pitch_difference_chance pitch_difference')
    _number(*c['cursor_step'],c['volume'],c['pitch_difference_chance'],c['pitch_difference'])
    if (c['item'],c['source']['path_id'])!=COMPUTER[::2] or not (0<c['x_size']<=256 and 0<c['y_size']<=256):raise ValueError('Computer values differ')
    if not (0<=c['volume']<=1 and 0<=c['pitch_difference_chance']<=1 and c['pitch_difference']>=0):raise ValueError('Computer music values differ')
    t=record['display']
    _shape(t,'item prototype source screen on_material off_material startup_on switch')
    _shape(t['switch'],'source node clip volume')
    _number(t['switch']['volume'])
    if (t['item'],t['source']['path_id'])!=DISPLAY[::2] or t['on_material']==t['off_material']:raise ValueError('Display values differ')
    clips={(x['source']['file'],x['source']['path_id']) for x in manifest['audio']['clips']}
    if (t['switch']['clip']['file'],t['switch']['clip']['path_id']) not in clips:raise ValueError('Display switch clip is not exported')
