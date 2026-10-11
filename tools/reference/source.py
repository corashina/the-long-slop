"""Resolve Unity identities once. Decode only selected custom components."""
from pathlib import Path


def source_id(reader):
    return {'file': reader.assets_file.name, 'path_id': reader.path_id}


def key(reader):
    return reader.assets_file.name, reader.path_id


def vector(value, width=3):
    return tuple(value[name] for name in ('x', 'y', 'z', 'w')[:width])


def normalize_arrays(node):
    """Restore array container types without changing scalar string layout."""
    if node.m_Children and node.m_Children[0].m_Type == 'Array':
        array = node.m_Children[0]
        scalar_string = node.m_Type == 'string' and len(array.m_Children) > 1 and array.m_Children[1].m_Type == 'char'
        if not scalar_string:
            node.m_Type = 'vector'
    for child in node.m_Children:
        normalize_arrays(child)


class Source:
    def __init__(self, steam_root: Path):
        import UnityPy
        from UnityPy.helpers import TypeTreeHelper
        from UnityPy.helpers.TypeTreeGenerator import TypeTreeGenerator
        directory = steam_root / 'TheLongDrive_Data'
        self.directory = directory
        self.env = UnityPy.load(*(str(directory / name) for name in (
            'level0', 'level1', 'sharedassets1.assets', 'sharedassets0.assets',
            'globalgamemanagers', 'globalgamemanagers.assets', 'resources.assets')))
        self.generator = TypeTreeGenerator('2021.3.18f1')
        self.generator.load_local_game(str(steam_root))
        self.env.typetree_generator = self.generator
        # The native reader caches type dispatch before generated-tree repairs.
        # Use UnityPy's Python reader so repaired array types control dispatch.
        TypeTreeHelper.read_typetree_boost = None
        self.objects = {key(obj): obj for obj in self.env.objects}
        self.data_cache = {}
        self.object_cache = {}
        self.tree_cache = {}
        self.evidence = {}
        self.class_cache = {}

    def get(self, file, path_id):
        return self.objects[file, path_id]

    def transform_for(self, game_object):
        for part in self.read(game_object)['m_Component']:
            reader = self.resolve(game_object,part['component'])
            if reader.type.name == 'Transform':
                return reader
        raise ValueError(f'GameObject has no Transform: {key(game_object)}')

    def chain(self, transform):
        result = []
        seen = set()
        while transform:
            if key(transform) in seen:
                raise ValueError('Transform parent cycle')
            seen.add(key(transform))
            result.append(self.read(transform))
            transform = self.resolve(transform,result[-1]['m_Father'])
        return result

    def resolve(self, owner, pointer):
        from UnityPy.classes.PPtr import PPtr
        if not pointer['m_PathID']:
            return None
        return PPtr(m_FileID=pointer['m_FileID'], m_PathID=pointer['m_PathID'], assetsfile=owner.assets_file).deref()

    def object(self, reader):
        identity = key(reader)
        if identity not in self.object_cache:
            self.object_cache[identity] = reader.parse_as_object(check_read=True)
        return self.object_cache[identity]

    def class_name(self, reader):
        identity = key(reader)
        if identity not in self.class_cache:
            head = reader.parse_monobehaviour_head()
            self.class_cache[identity] = head.m_Script.deref_parse_as_object().m_ClassName
        return self.class_cache[identity]

    def read(self, reader):
        identity = key(reader)
        if identity in self.data_cache:
            return self.data_cache[identity]
        if reader.type.name == 'MonoBehaviour':
            from UnityPy.helpers.Tpk import get_typetree_node
            standard = get_typetree_node(reader.class_id, reader.version)
            name = self.class_name(reader)
            head = reader.parse_monobehaviour_head()
            tree_key = key(head.m_Script.deref()), reader.version
            if tree_key not in self.tree_cache:
                node = reader.generate_monobehaviour_node()
                header_names = {child.m_Name for child in standard.m_Children}
                node.m_Children = [*standard.m_Children, *[child for child in node.m_Children if child.m_Name not in header_names]]
                normalize_arrays(node)
                self.tree_cache[tree_key] = node
            data = reader.parse_as_dict(node=self.tree_cache[tree_key], check_read=True)
            head = reader.parse_monobehaviour_head()
            for field in ('m_Script', 'm_GameObject'):
                pointer = getattr(head, field)
                if data[field] != {'m_FileID': pointer.file_id, 'm_PathID': pointer.path_id}:
                    raise ValueError(f'Header mismatch: {identity} {field}')
            self.evidence[f'{identity[0]}:{identity[1]}'] = {
                'class': name, 'bytes': reader.byte_size, 'complete_byte_consumption': True,
            }
        else:
            data = reader.parse_as_dict(check_read=True)
        self.data_cache[identity] = data
        return data

    def components(self, transform):
        data = self.read(transform)
        game_object = self.resolve(transform, data['m_GameObject'])
        return game_object, [self.resolve(game_object, part['component']) for part in self.read(game_object)['m_Component']]

    def hierarchy(self, root, activate_root=False):
        result = []
        seen = set()
        def visit(reader, parent, path, active):
            if key(reader) in seen:
                raise ValueError(f'Duplicate hierarchy reference: {key(reader)}')
            seen.add(key(reader))
            game_object, components = self.components(reader)
            go = self.read(game_object)
            path = f'{path}/{go["m_Name"]}' if path else go['m_Name']
            active = active and (bool(go['m_IsActive']) or (parent is None and activate_root))
            result.append({'reader': reader, 'game_object': game_object, 'components': components, 'parent': parent, 'path': path, 'active': active})
            for child in self.read(reader)['m_Children']:
                visit(self.resolve(reader, child), reader, path, active)
        visit(root, None, '', True)
        return result
