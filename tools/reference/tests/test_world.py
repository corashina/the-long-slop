import unittest
from types import SimpleNamespace as NS
from tools.reference.world import required_record

class WorldTests(unittest.TestCase):
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
