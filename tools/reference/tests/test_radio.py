"""Critical source radio display links."""
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace as NS
from unittest.mock import patch

from tools.reference.radio import recover_radio_visuals
from tools.reference.tests.test_instruments import dashboard_fixture


def radio_fixture():
    source, manifest, reports, owner = dashboard_fixture()
    original_resolve = source.resolve
    extra = {i: NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=i)
             for i in (23, 24)}
    source.resolve = lambda owner, pointer: extra.get(pointer['m_PathID']) or original_resolve(owner, pointer)
    reports['car']['material_indices'].update({'level1:23': 5, 'level1:24': 6})
    fields = source.read(owner)
    fields.update(RS={'m_PathID': 500}, SOn={'m_PathID': 21}, SOff={'m_PathID': 22},
                  RMutato={'m_PathID': 200}, MutatoOn={'m_PathID': 23}, MutatoOff={'m_PathID': 24},
                  TMutato={'m_PathID': 100}, ST1={'m_PathID': 101}, ST2={'m_PathID': 102})
    for node in manifest['nodes']:
        node['asset'] = 7
    return source, manifest, reports, owner


class RadioTests(unittest.TestCase):
    def test_radio_visual_source_links(self):
        source, manifest, reports, owner = radio_fixture()
        result = recover_radio_visuals(source, owner, manifest, reports)
        self.assertEqual([result[k] for k in ('needle', 'start', 'end')], [100, 101, 102])
        for name, node, on, off in [('screen', 502, 21, 22), ('indicator', 400, 23, 24)]:
            switch = result[name]
            self.assertEqual(switch['kind'], 'renderer')
            self.assertEqual(switch['node'], node)
            for field, identity in [('on', on), ('off', off)]:
                self.assertEqual(switch[field], dict(source=dict(file='level1', path_id=identity),
                    asset=7, gltf_material=reports['car']['material_indices'][f'level1:{identity}']))
        for field in ('RS', 'SOn', 'SOff', 'RMutato', 'MutatoOn', 'MutatoOff', 'TMutato', 'ST1', 'ST2'):
            original = source.read(owner)[field]
            source.read(owner)[field] = {'m_PathID': 0}
            with self.assertRaises(ValueError, msg=field):
                recover_radio_visuals(source, owner, manifest, reports)
            source.read(owner)[field] = original
        reports['car']['material_indices'].clear()
        with self.assertRaises(ValueError):
            recover_radio_visuals(source, owner, manifest, reports)

    def test_radio_material_variants_are_exported(self):
        from tools.reference.gltf import GltfExporter
        import pygltflib as gl
        source, manifest, reports, owner = radio_fixture()
        data = source.read(owner)
        data['m_Script'] = {'m_PathID': 1}
        source.class_name = lambda _: 'radioscript'
        root = NS(assets_file=NS(name='test', source_container_path='test'), path_id=1)
        game_object = object()
        original_read = source.read
        source.read = lambda reader: ({'m_LocalPosition': dict(x=0., y=0., z=0.),
            'm_LocalRotation': dict(x=0., y=0., z=0., w=1.),
            'm_LocalScale': dict(x=1., y=1., z=1.)} if reader is root else
            {'m_Name': 'radio', 'm_IsActive': 1, 'm_Layer': 0} if reader is game_object else original_read(reader))
        source.hierarchy = lambda *args, **kwargs: [dict(reader=root, game_object=game_object,
            components=[owner], parent=None, path='radio')]
        source.lighting_prototype_roots = {('test', 1)}
        with tempfile.TemporaryDirectory() as directory:
            builder = GltfExporter(source, Path(directory) / 'radio.gltf', None)
            builder.binary.extend(b'\0' * 4)
            def material(reader):
                identity = f'{reader.assets_file.name}:{reader.path_id}'
                if identity not in builder.material_indices:
                    builder.material_indices[identity] = len(builder.document.materials)
                    builder.document.materials.append(gl.Material())
            builder.material = material
            with patch('tools.reference.gltf.recover_startup_inactive_nodes', return_value=set()), \
                 patch('tools.reference.conditions.export_condition_variants'):
                report = builder.export([root])
            self.assertEqual(set(report['material_indices']), {f'level1:{i}' for i in (21, 22, 23, 24)})
