import unittest
from types import SimpleNamespace as NS
from tools.reference.world import Selection, required_record

class WorldTests(unittest.TestCase):
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

    def test_generation_counts_are_required_and_checked(self):
        from tools.reference.world import generation_counts
        self.assertEqual(generation_counts({'genAtStart':100,'genInOneFrame':1}), {'genAtStart':100,'genInOneFrame':1})
        for road in ({}, {'genAtStart':99,'genInOneFrame':1}, {'genAtStart':100,'genInOneFrame':2}):
            with self.assertRaises((KeyError,ValueError)): generation_counts(road)
