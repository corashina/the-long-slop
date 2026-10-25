"""Checked melee hit effects: mainscript hit particles and hit volumes."""
from .source import key, source_id
from .survival import managed_evidence
from .particles import particle

MAIN=('level1',64919)
# mainscript.particle order. Tyre smoke and tyre sand are not melee hit effects.
HIT_PARTICLES=('spark','blood','plant','wood','sand')
# mainscript.soundHitType order: metal, blood, plant, wood, sand, glass.
HIT_VOLUMES=[0.3,0.6,0.4,0.4,0.5,0.4]
EVIDENCE=[
    'public static float[] hitVolume = new float[6] { 0.3f, 0.6f, 0.4f, 0.4f, 0.5f, 0.4f };',
    'Particles[(int)tipus].Emit(num);',
    'mainscript.M.Particle(mainscript.particle.sand, vector, T.position - vector, (int)((float)sparknum * sand));',
    'tosaveitemscript2.UpdBuried(shovelSpeed);']


def recover_melee(source, output) -> dict:
    managed_evidence(source, EVIDENCE)
    main=source.get(*MAIN);d=source.read(main)
    if source.class_name(main)!='mainscript':raise ValueError(f'{key(main)} is not mainscript')
    particles=[]
    for name,pointer in zip(HIT_PARTICLES,d['Particles']):
        system=source.resolve(main,pointer)
        if system is None or system.type.name!='ParticleSystem':raise ValueError(f'mainscript particle {name} is not ParticleSystem')
        # Particle() moves the shared system and calls Emit(num); emission does not run.
        particles.append(particle(source,system,f'melee_{name}',0.,output))
    return {'source':source_id(main),'hit_volumes':HIT_VOLUMES,'particles':particles}


def validate_melee(record, manifest) -> None:
    if set(record)!={'source','hit_volumes','particles'}:raise ValueError('Melee record fields differ')
    if record['hit_volumes']!=HIT_VOLUMES or len(record['particles'])!=len(HIT_PARTICLES):raise ValueError('Melee hit values differ')
    for p in record['particles']:
        if not p['materials'] or 'InitialModule.startLifetime' not in p['curves']:raise ValueError('Melee particle data missing')
