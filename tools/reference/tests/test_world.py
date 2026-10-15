import unittest
from types import SimpleNamespace as NS
from tools.reference.world import Selection, required_record

class WorldTests(unittest.TestCase):
    def test_prototype_material_and_texture_keep_checked_source_receipts(self):
        from contextlib import redirect_stdout
        from io import StringIO
        from pathlib import Path
        from tempfile import TemporaryDirectory
        from unittest.mock import patch
        from tools.reference.world import export_world_prototypes
        from tools.reference.world_spawn import project_runtime_receipts
        file = NS(name='fixture', source_container_path='fixture')
        readers = {i: NS(assets_file=file, path_id=i, type=NS(name=kind))
                   for i, kind in ((1, 'Transform'), (2, 'Material'), (3, 'Texture2D'))}
        root = readers[1]
        data = {1: dict(m_LocalPosition=dict(x=0., y=0., z=0.),
                        m_LocalRotation=dict(x=0., y=0., z=0., w=1.),
                        m_LocalScale=dict(x=1., y=1., z=1.)),
                2: {'m_Name': 'material'}, 3: {'m_Name': 'texture'}}
        source = NS(read=lambda r: data[r.path_id], get=lambda f, i: readers[i],
                    hierarchy=lambda *args, **kwargs: [], chain=lambda r: [root])
        library = NS(record_files={f'fixture/{i}.json': {'sha256': str(i) * 64} for i in readers})
        selected = Selection(source, library)
        selected.receipt(root)
        material = dict(source=dict(file='fixture', path_id=2),
                        properties=dict(textures=[dict(source=dict(file='fixture', path_id=3))]))
        report = dict(deferred_materials=[], nodes=[], materials=[material],
                      material_count=1, sha256='a' * 64, dependencies=[])
        with TemporaryDirectory() as directory, redirect_stdout(StringIO()):
            with patch('tools.reference.world.GltfExporter', return_value=NS(export=lambda roots: report)):
                prototypes = export_world_prototypes(source, library, selected, Path(directory),
                    {('fixture', 1): root}, {('fixture', 1): 'prop'}, {}, 1)
            world = dict(library={}, source_hashes=[], prototypes=prototypes,
                         receipts=list(selected.receipts.values()))
            project_runtime_receipts(world, directory)
        self.assertEqual({r['source']['path_id']: r['sha256'] for r in world['receipts']},
                         {i: str(i) * 64 for i in readers})
        self.assertEqual(selected.records, {})

    def test_renderer_export_keeps_two_sided_source_shadows(self):
        from tools.reference.render_lighting import recover_renderer
        file = NS(name='fixture', source_container_path='fixture')
        renderer = NS(assets_file=file, path_id=1, type=NS(name='MeshRenderer'))
        mesh = object()
        data = dict(m_ProbeAnchor={'m_PathID': 0}, m_CastShadows=2,
                    m_ReceiveShadows=1, m_ReflectionProbeUsage=0, m_MotionVectors=1,
                    m_SortingLayerID=0, m_SortingLayer=0, m_SortingOrder=0)
        bounds = {'m_Center': dict(x=0., y=0., z=0.), 'm_Extent': dict(x=1., y=1., z=1.)}
        source = NS(read=lambda r: data if r is renderer else {'m_LocalAABB': bounds})
        result = recover_renderer(source, renderer, mesh)
        self.assertEqual(result['cast_shadows'], 2)

    def test_startup_transform_chain_retains_excluded_root_and_parent(self):
        file = NS(name='level1', source_container_path='level1')
        readers = {i: NS(assets_file=file, path_id=i, type=NS(name='Transform')) for i in (1, 2)}
        data = {1: {'m_Father': {'m_FileID': 0, 'm_PathID': 2}},
                2: {'m_Father': {'m_FileID': 0, 'm_PathID': 0}}}
        source = NS(read=lambda r: data[r.path_id],
                    resolve=lambda r, p: readers.get(p['m_PathID']))
        library = NS(record_files={f'level1/{i}.json': {'sha256': 'a' * 64} for i in readers})
        selection = Selection(source, library)
        selection.transform_chain(readers[1])
        self.assertEqual(set(selection.records), {('level1', 1), ('level1', 2)})

    def test_road_lamps_retain_light_binding_and_parameters(self):
        file = NS(name='fixture', source_container_path='fixture')
        readers = {i: NS(assets_file=file, path_id=i, type=NS(name=kind))
                   for i, kind in ((1, 'MonoBehaviour'), (2, 'GameObject'), (3, 'Light'))}
        pointer = lambda i: {'m_FileID': 0, 'm_PathID': i}
        data = {1: {'m_GameObject': pointer(2), 'LLight': pointer(3)},
                2: {'m_Name': 'lamp'}, 3: {'m_GameObject': pointer(2), 'm_Intensity': 1.5}}
        source = NS(read=lambda reader: data[reader.path_id],
                    resolve=lambda reader, p: readers[p['m_PathID']],
                    class_name=lambda reader: 'kozlampa')
        library = NS(record_files={f'fixture/{i}.json': {'sha256': 'a' * 64} for i in readers})
        selection = Selection(source, library)
        controller = selection.component_record(readers[1])
        light = selection.component_record(readers[3])
        self.assertEqual(controller['fields']['LLight']['value'], {'file': 'fixture', 'path_id': 3})
        self.assertEqual(light['fields']['m_Intensity'], {'kind': 'number', 'value': 1.5})

    def test_lod_components_retain_bounds_and_levels(self):
        file = NS(name='fixture', source_container_path='fixture')
        lod = NS(assets_file=file, path_id=1, type=NS(name='LODGroup'))
        owner = NS(assets_file=file, path_id=2, type=NS(name='GameObject'))
        data = {'m_GameObject': {'m_FileID': 0, 'm_PathID': 2},
                'm_LocalReferencePoint': {'x': 1.0, 'y': 2.0, 'z': 3.0},
                'm_Size': 20.0, 'm_LODs': []}
        source = NS(read=lambda reader: data if reader is lod else {'m_Name': 'house'},
                    resolve=lambda reader, pointer: owner)
        library = NS(record_files={f'fixture/{i}.json': {'sha256': 'a' * 64} for i in (1, 2)})
        result = Selection(source, library).component_record(lod)
        self.assertEqual(result['fields']['m_Size'], {'kind': 'number', 'value': 20.0})
        self.assertEqual(result['fields']['m_LocalReferencePoint']['value']['y']['value'], 2.0)
        self.assertEqual(result['fields']['m_LODs'], {'kind': 'array', 'value': []})

    def test_world_selection_requires_resolved_links(self):
        owner = NS(assets_file=NS(source_container_path='level1'),path_id=64898)
        class Source:
            def read(self, obj):
                if obj is not owner:
                    raise AssertionError('Read a dependent file before checking its link')
                return {'roadGens': [{'m_FileID': 0, 'm_PathID': 64771}]}
            def resolve(self, obj, pointer):
                raise KeyError('missing source record')
        with self.assertRaisesRegex(ValueError, 'level1:64898.*roadGens'):
            required_record(Source(), owner, ('roadGens',))
