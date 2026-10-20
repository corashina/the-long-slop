import unittest
from types import SimpleNamespace as NS

from tools.reference import mechanisms


class MechanismTests(unittest.TestCase):
    def fields(self):
        return dict(Xforce=True, Yforce=False, Zforce=True, invForce=True,
                    force=10000., startForce=False, endForce=False,
                    startPosEnd={'m_PathID': 11}, endPosEnd={'m_PathID': 12},
                    startPosStart={'m_PathID': 13}, endPosStart={'m_PathID': 14})

    def test_endpoint_fields_and_clip_gates(self):
        source = NS(resolve=lambda owner, pointer: object() if pointer['m_PathID'] else None)
        fields = self.fields()
        result = mechanisms.recover_endpoint_reaction(source, object(), fields, 7)
        self.assertEqual(result, dict(kind='rotation', node=7, axes=[True, False, True],
                                     inverted=True, force=10000., events=[True] * 4))
        for index, field in enumerate(('startPosEnd', 'endPosEnd', 'startPosStart', 'endPosStart')):
            fields = self.fields()
            fields[field] = {'m_PathID': 0}
            events = [True] * 4
            events[index] = False
            self.assertEqual(mechanisms.recover_endpoint_reaction(source, object(), fields, 7)['events'], events)
        for force in (-1., float('nan'), float('inf')):
            fields = self.fields()
            fields['force'] = force
            with self.assertRaises(ValueError):
                mechanisms.recover_endpoint_reaction(source, object(), fields, 7)

    def test_no_reaction_is_explicit(self):
        fields = self.fields()
        fields.update(Xforce=False, Yforce=False, Zforce=False)
        source = NS(resolve=lambda owner, pointer: None)
        self.assertEqual(mechanisms.recover_endpoint_reaction(source, object(), fields, 7), {'kind': 'none'})
        with self.assertRaises(ValueError):
            mechanisms.recover_endpoint_reaction(source, object(), self.fields(), 7)
        fields = self.fields()
        del fields['startPosEnd']
        with self.assertRaises(KeyError):
            mechanisms.recover_endpoint_reaction(source, object(), fields, 7)
