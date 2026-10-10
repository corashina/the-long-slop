"""Recover action owners from component links and source startup rules."""
from .source import key


def recover_masks(player):
    fields = {'use_mask': 'useLayer', 'pickup_mask': 'pickupLayer',
        'attach_mask': 'attachLayer', 'push_mask': 'pushLayer'}
    result = {}
    for name, field in fields.items():
        mask = player[field]['m_Bits']
        if not isinstance(mask, int) or not 0 < mask <= 0xffffffff:
            raise ValueError(f'fpscontroller.{field}: invalid action mask')
        result[name] = mask
    return result


def recover_bindings(source, nodes, components, item_nodes, colliders, hinges, openings, seats):
    component_items = {}
    for node, group in components.items():
        for readers in group.values():
            for reader in readers:
                if node in item_nodes:
                    component_items[key(reader)] = item_nodes[node]
    uses = {(r['source']['file'], r['source']['path_id']): r['item'] for r in hinges}
    uses.update({(r['source']['file'], r['source']['path_id']): r['cap'] for r in openings})
    for seat in seats:
        for node in seat['hit_nodes']:
            for reader in components[node].get('seatscript', []):
                uses[key(reader)] = seat['item']
    layers = {}
    # pickupable.LayerStart assigns cols[].p after layerScript.FStart.
    for node, group in components.items():
        if not nodes[node - 1]['active']:
            continue
        for reader in group.get('pickupable', []):
            value = source.read(reader)
            if key(reader) not in component_items:
                raise ValueError(f'pickupable owner missing: {key(reader)}')
            for pointer in value['cols']:
                layer = source.resolve(reader, pointer)
                if layer is None or source.class_name(layer) != 'layerScript':
                    raise ValueError(f'pickupable.cols: invalid layerScript link: {key(reader)}')
                identity = key(layer)
                if identity in layers and layers[identity] != key(reader):
                    raise ValueError(f'pickupable.cols: duplicate owner: {identity}')
                layers[identity] = key(reader)
    def ancestor(node, classes):
        while node is not None:
            group = components[node]
            for name in classes:
                readers = group.get(name, [])
                if readers:
                    if len(readers) != 1:
                        raise ValueError(f'{name}: multiple action owners on node {node}')
                    return readers[0]
            node = nodes[node - 1]['parent']
        return None
    def item(reader, field):
        if reader is None:
            return None
        identity = key(reader)
        if identity not in component_items:
            raise ValueError(f'{field}: owner outside selected items: {identity}')
        return component_items[identity]
    result = []
    for collider in colliders:
        node = collider['node']
        use = ancestor(node, ('tankcapscript', 'usablescript', 'seatscript'))
        use_owner = uses.get(key(use), item(use, 'use owner')) if use else None
        pickup = ancestor(node, ('pickupable',))
        pickup_owner = None
        if pickup is not None and not source.read(pickup)['cantPickup']:
            pickup_owner = item(pickup, 'pickupable')
        removal_owner = None
        for slot in components[node].get('partslotscript', []):
            part = source.resolve(slot, source.read(slot)['part'])
            if part is not None:
                if source.class_name(part) != 'partscript':
                    raise ValueError(f'partslotscript.part: invalid partscript: {key(slot)}')
                pick = source.resolve(part, source.read(part)['p'])
                if pick is None or source.class_name(pick) != 'pickupable':
                    raise ValueError(f'partscript.p: invalid pickupable: {key(part)}')
                removal_owner = item(pick, 'partslotscript.part.p')
        layer_readers = components[node].get('layerScript', [])
        for layer in layer_readers:
            value = source.read(layer)
            if value['col']['m_PathID']:
                linked = source.resolve(layer, value['col'])
                if linked is None or not linked.type.name.endswith('Collider'):
                    raise ValueError(f'layerScript.col: invalid collider: {key(layer)}')
            # The player queries layerScript on the hit GameObject. Its p owner
            # applies to each Collider on that GameObject, regardless of col.
            owner_id = layers.get(key(layer))
            owner = source.get(*owner_id) if owner_id else source.resolve(layer, value['p'])
            if owner is not None:
                if source.class_name(owner) != 'pickupable':
                    raise ValueError(f'layerScript.p: invalid pickupable: {key(layer)}')
                part = source.resolve(owner, source.read(owner)['part'])
                if part is not None:
                    if source.class_name(part) != 'partscript':
                        raise ValueError(f'pickupable.part: invalid partscript: {key(owner)}')
                    if removal_owner is None:
                        removal_owner = item(owner, 'layerScript.p.part')
        if removal_owner is None and pickup is not None:
            part = source.resolve(pickup, source.read(pickup)['part'])
            if part is not None:
                if source.class_name(part) != 'partscript':
                    raise ValueError(f'pickupable.part: invalid partscript: {key(pickup)}')
                removal_owner = item(pickup, 'pickupable.part')
        push = ancestor(node, ('pushablescript',))
        result.append({'collider': collider['id'], 'use_owner': use_owner,
            'pickup_owner': pickup_owner, 'removal_owner': removal_owner,
            'push_owner': item(push, 'pushablescript')})
    return result


def recover_pickup(source, reader):
    import math
    from .source import source_id, vector
    from .gltf import convert_point, convert_rotation
    from .collision import quaternion_product
    fields = source.read(reader)
    angles = [math.radians(v) * .5 for v in vector(fields['recenterRot'])]
    x, y, z = angles
    rotation = quaternion_product((0., math.sin(y), 0., math.cos(y)),
        quaternion_product((math.sin(x), 0., 0., math.cos(x)), (0., 0., math.sin(z), math.cos(z))))
    return {'source': source_id(reader), 'cant_pickup': bool(fields['cantPickup']),
        'cant_pickup_while_mounted': bool(fields['cantPickupWhileMount']),
        'cant_pickup_while_seated': bool(fields['cantPickupWhileSitting']),
        'no_rotation': bool(fields['noRot']), 'move_with_rotation': bool(fields['moveWithRot']),
        'use_recenter_position': bool(fields['useRecenter']),
        'recenter_position': convert_point(vector(fields['recenterPos'])),
        'use_recenter_rotation': bool(fields['useRecenterRot']),
        'recenter_rotation': convert_rotation(rotation), 'recenter_distance': fields['recenterDist']}
