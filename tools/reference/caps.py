"""Recover cap controls and local visual marker links."""
import ast
import re
from .source import key, vector


def parse_default_words(code: str) -> list[str]:
    """Return ls.FillDefWW: the default English text for each ls.w value."""
    body=code.split('private void FillDefWW()',1)[1].split('\n\t}',1)[0]
    words=[None]*int(re.search(r'defww = new string\[(\d+)\];',body)[1])
    for index,text in re.findall(r'defww\[(\d+)\] = ("(?:[^"\\]|\\.)*");',body):
        words[int(index)]=ast.literal_eval(text)
    if None in words:raise ValueError('Default language table is incomplete')
    return words


def default_words(source) -> list[str]:
    if not hasattr(source,'default_words'):
        from .library.reader import checked
        records={r['file']:r for r in source.library.manifest['managed_analysis']['files']}
        code=checked(source.library.records_root,records['managed/Assembly-CSharp.cs']).decode('utf-8-sig')
        source.default_words=parse_default_words(code)
    return source.default_words


def cap_presentation(fields, words):
    """GetCapString shows ls.l.g(nameText). The serialized nameString is not read."""
    return {'has_name':bool(fields['hasName']),'name':words[fields['nameText']] if fields['hasName'] else '','no_text':bool(fields['noText'])}


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
        'presentation':cap_presentation(fields,default_words(source)),
        'visuals':{'open_nodes':[node(pointer) for pointer in fields['GOpen']],
            'closed_nodes':[node(pointer) for pointer in fields['GClosed']],
            'cap_node':node(fields['TCap']),'closed_node':node(fields['TClosed']),'open_node':node(fields['TOpen']),
            'euler_rotation':bool(fields['frot']),'closed_euler_degrees':euler(fields['closedRot']),
            'open_euler_degrees':euler(fields['openRot'])}}
