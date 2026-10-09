"""Recover cap controls and local visual marker links."""
from .source import key, vector


def opening_metadata(source, reader, node_ids, fluid_tune_speed):
    fields=source.read(reader)
    def node(pointer):
        if not pointer['m_PathID']:return None
        target=source.resolve(reader,pointer)
        if target.type.name=='GameObject':target=source.transform_for(target)
        if key(target) not in node_ids:raise ValueError(f'Cap visual outside selected hierarchy: {key(target)}')
        return node_ids[key(target)]
    def euler(value):
        x,y,z=vector(value)
        return [-x,-y,z]
    return {'valve':fields['valve'],'can_tune':bool(fields['canTune']),'tune_rate':fluid_tune_speed*fields['tuneSpeed'],
        'visuals':{'open_nodes':[node(pointer) for pointer in fields['GOpen']],
            'closed_nodes':[node(pointer) for pointer in fields['GClosed']],
            'cap_node':node(fields['TCap']),'closed_node':node(fields['TClosed']),'open_node':node(fields['TOpen']),
            'euler_rotation':bool(fields['frot']),'closed_euler_degrees':euler(fields['closedRot']),
            'open_euler_degrees':euler(fields['openRot'])}}
