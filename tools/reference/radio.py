"""Resolve the selected source radio display."""
from .instruments import recover_material_switch
from .source import key


def recover_radio_visuals(source, radio, manifest, reports):
    fields = source.read(radio)
    nodes = {(n['source']['file'], n['source']['path_id']): n for n in manifest['nodes']}

    def node(owner, pointer, renderer=False):
        reader = source.resolve(owner, pointer)
        if reader is None:
            raise ValueError('Missing radio visual pointer')
        if renderer:
            reader = source.transform_for(source.resolve(reader, source.read(reader)['m_GameObject']))
        if key(reader) not in nodes:
            raise ValueError(f'Missing radio visual node: {key(reader)}')
        return nodes[key(reader)]['id']

    screen = node(radio, fields['RS'], True)
    asset_id = next(n['asset'] for n in manifest['nodes'] if n['id'] == screen)
    asset = next(a for a in manifest['assets'] if a['id'] == asset_id)
    report = reports[asset['path'].removeprefix('reference/').removesuffix('.gltf')]
    result = {}
    for name, renderer, on, off in [('screen', 'RS', 'SOn', 'SOff'),
                                    ('indicator', 'RMutato', 'MutatoOn', 'MutatoOff')]:
        if not fields[renderer]['m_PathID']:
            raise ValueError('Missing radio renderer')
        result[name] = recover_material_switch(source, radio,
            dict(R=fields[renderer], OnM=fields[on], OffM=fields[off]),
            node, asset_id, report['material_indices'])
    for name, field in [('needle', 'TMutato'), ('start', 'ST1'), ('end', 'ST2')]:
        result[name] = node(radio, fields[field])
    selected = [result['screen']['node'], result['indicator']['node'],
                result['needle'], result['start'], result['end']]
    if any(n['asset'] != asset_id for n in manifest['nodes'] if n['id'] in selected):
        raise ValueError('Radio visual nodes must share the selected asset')
    return result
