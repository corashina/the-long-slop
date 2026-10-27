import copy
import unittest

from tools.reference import guns


def sid(path_id, file='sharedassets1.assets'):
    return {'file': file, 'path_id': path_id}


def curve():
    return {'keys': [{'time': 0., 'value': 0., 'in_tangent': 0., 'out_tangent': 0., 'in_weight': 0., 'out_weight': 0.,
                      'weighted_mode': 0}], 'pre_wrap_mode': 2, 'post_wrap_mode': 2}


def rule():
    return {'value': 0., 'equal': True, 'bigger': False, 'smaller': False, 'near': False, 'near_value': 0.}


def weapon(prototype, source, kind):
    node = sid(prototype + 1)
    return {'prototype': sid(prototype), 'source': sid(source), 'pickup': sid(source + 1),
            'ammo': {'source': sid(source + 2), 'count': 6, 'float_count': 0., 'maximum': 6., 'float_or_int': False,
                     'type': 1, 'random_range': [0., 6.]},
            'kind': kind, 'condition': None, 'max_condition_weakness': .5, 'infinite': False, 'shot_count': 1,
            'shot_count_spread': 0., 'automatic': False, 'automatic_false_shot': False, 'min_shoot_time': .1,
            'mask': 1, 'mask2': 0, 'muzzle': node,
            'clips': {k: [sid(5)] for k in guns.CLIP_FIELDS}, 'audio_m': 1.,
            'flash': {'light': sid(prototype + 9), 'node': node, 'time': .1, 'intensity': 1., 'curve': curve()},
            'recoil': None, 'reload_hand': None,
            'barrels': [{'node': node, 'max_angle': 30., 'inv': False, 'axis': 0, 'curve': None}],
            'moving_parts': [], 'triggers': [], 'trigger_triggers': [], 'use_conditional_trigger_triggers': False,
            'conditional_trigger_triggers': [],
            'fr': {'value': 0., 'fr_speed': 1., 'use_x': False, 'use_y': True, 'breakable': True,
                   'play_sound_even_if_cant_shoot': False, 'shoot': rule(), 'reload': rule()},
            'sounds': {'source': None, 'open': {'clip': sid(5), 'volume': 1.}, 'open_end': None, 'close': None,
                       'move': None, 'reload': None},
            'crosshair': {'icon': sid(7), 'color': [1.] * 4, 'empty_color': [1.] * 4},
            'explode_stuffs': False, 'explode_chance': 0., 'hithole': False}


class GunsTests(unittest.TestCase):
    def setUp(self):
        particle = {'materials': [1], 'curves': {'InitialModule.startLifetime': {}}}
        decal = {'source': sid(50), 'node': sid(50), 'local_pose': {},
                 'mesh': {'source': sid(51), 'bounds_center': [0.] * 3, 'bounds_extents': [.5, .5, 0.]},
                 'material': {'textures': [{'texture': 1}]}, 'lifetime': None}
        revolver = {'type': 'revolver', 'chambers': [sid(101)], 'cylinder': sid(101), 'axis': [0., 0., 1.], 'angle': 60.,
                    'duration': .2, 'shoot_bullet_pos': sid(101), 'cylinder_start_rotation': [0.] * 3,
                    'turn_clip': sid(5), 'turn_volume': 1.}
        smg = {'type': 'magazine', 'mag': sid(201), 'mag_pos': sid(201), 'mag_prefab': sid(300), 'has_mag': True}
        self.record = {'source': sid(64919, 'level1'), 'ammo_mass': list(guns.AMMO_MASS),
                       'ammo_velocity': list(guns.AMMO_VELOCITY), 'tracer': {'start': particle, 'end': particle},
                       'decals': [decal] * 6, 'weapons': [weapon(100, 1000, revolver), weapon(200, 2000, smg)]}
        self.manifest = {
            'audio': {'clips': [{'source': sid(5)}]}, 'ui': {'sprites': [{'source': sid(7)}]},
            'lighting': {'textures': [{'id': 1}]},
            'world': {'prototypes': [{'source': sid(p), 'nodes': [{'source': sid(p + 1)}]} for p in (100, 200, 300)]}}

    def test_validate_guns(self):
        guns.validate_guns(self.record, self.manifest)
        for change in [{'ammo_mass': [0.] * 7}, {'decals': self.record['decals'][:5]}, {'extra': 1},
                       {'weapons': self.record['weapons'][::-1]}, {'weapons': []}]:
            with self.subTest(change=list(change)), self.assertRaises(ValueError):
                guns.validate_guns({**self.record, **change}, self.manifest)

    def test_weapon_errors_name_the_weapon(self):
        for index, path, value, text in [(0, ('muzzle',), sid(999), '1000'),
                                         (0, ('kind', 'chambers'), [sid(999)], '1000'),
                                         (1, ('clips', 'one_shot'), [sid(6)], '2000'),
                                         (1, ('ammo', 'type'), 7, '2000'),
                                         (1, ('kind', 'mag_prefab'), sid(400), '2000'),
                                         (1, ('kind', 'type'), 'laser', '2000'),
                                         (1, ('crosshair', 'icon'), sid(8), '2000')]:
            record = copy.deepcopy(self.record)
            target = record['weapons'][index]
            for field in path[:-1]:
                target = target[field]
            target[path[-1]] = value
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, text):
                guns.validate_guns(record, self.manifest)
