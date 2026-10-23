"""Critical startup visibility for source storage helpers."""
import unittest
from types import SimpleNamespace as NS

from tools.reference.visibility import recover_startup_inactive_nodes


class StartupVisibilityTests(unittest.TestCase):
    def test_storage_visual_links_hide_trunk_and_engine_helpers(self):
        readers = {i: NS(path_id=i, assets_file=NS(source_container_path='level1'))
                   for i in (79301, 69971, 18272, 24896, 24534, 4321)}
        fields = {
            79301: dict(m_Enabled=1, visual=dict(m_PathID=18272)),
            69971: dict(m_Enabled=1, visual=dict(m_PathID=4321)),
        }
        transforms = {18272: readers[24896], 4321: readers[24534]}
        for i in fields:
            readers[i].type = NS(name='MonoBehaviour')
        source = NS(
            class_name=lambda _: 'mountStuff',
            read=lambda reader: fields[reader.path_id],
            resolve=lambda owner, pointer: readers[pointer['m_PathID']],
            transform_for=lambda reader: transforms[reader.path_id],
        )
        records = [dict(active=True, components=[readers[i]]) for i in fields]
        expected = {('level1', 24896), ('level1', 24534)}
        self.assertEqual(recover_startup_inactive_nodes(source, records), expected)
        for record in records:
            record['active'] = False
        self.assertEqual(recover_startup_inactive_nodes(source, records), set())
        for record in records:
            record['active'] = True
        for field in fields.values():
            field['m_Enabled'] = 0
        self.assertEqual(recover_startup_inactive_nodes(source, records), set())
        for field in fields.values():
            field.update(m_Enabled=1, visual=dict(m_PathID=0))
        self.assertEqual(recover_startup_inactive_nodes(source, records), set())


if __name__ == '__main__':
    unittest.main()
