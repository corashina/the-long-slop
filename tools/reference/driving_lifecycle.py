"""Strict original setup audio lifetime and player dependency evidence."""
import json
from .driving_driver import validate_body_state


def _mark(value, callback=None):
    if (not isinstance(value, dict) or not {'callback', 'order', 'frame', 'time', 'update_sequence', 'input_revision'} <= value.keys() or
            not value['callback'] or any(type(value[k]) is not int or value[k] < 0 for k in ('order', 'frame', 'update_sequence', 'input_revision')) or
            type(value['time']) not in (int, float) or (callback and value['callback'] != callback)):
        raise ValueError('Missing or invalid original lifecycle callback mark')
    return value['order']


def _vector(value, axes='xyz'):
    return isinstance(value, dict) and all(type(value.get(k)) in (int, float) for k in axes)


def validate_lifecycle(run):
    report = run.get('lifecycle', {})
    if (any(report.get(k) is not True for k in ('begun', 'owners_complete', 'cohort_closed', 'ready', 'armed')) or
            not report.get('owner_completion_evidence') or report.get('new_after_close') != []):
        raise ValueError('Incomplete source lifecycle readiness or setup cohort closure')
    opened = _mark(report.get('opened'), 'world.entry')
    owners_completed = _mark(report.get('owners_completed'), 'observer.Initialize.explicit-setup-exit')
    closed = _mark(report.get('closed'), 'observer.prepared-held.close-cohort')
    prepared = _mark(report.get('prepared'), 'observer.prepared-held')
    if not opened < owners_completed < closed < prepared:
        raise ValueError('Invalid setup cohort closure order')
    owners = report.get('owner_callbacks')
    if not isinstance(owners, list) or not owners:
        raise ValueError('Missing original setup owner completion observations')
    for owner in owners:
        if not opened < _mark(owner):
            raise ValueError('Original source owner callback precedes cohort observation')
    audio = report.get('audio')
    if not isinstance(audio, list) or not audio:
        raise ValueError('Missing retained source audio cohort')
    identities = set()
    for value in audio:
        if (value.get('prefab') != 'sharedassets1.assets:21250' or not {'creation_parent', 'parent'} <= value.keys() or
                any(not value.get(k) for k in ('prefab', 'clip', 'source_owner', 'source_stack', 'component', 'collider')) or
                any(type(value.get(k)) is not int or value[k] == 0 for k in
                    ('object_id', 'component_id', 'collider_id', 'prefab_id', 'clip_id')) or
                type(value.get('clip_length')) not in (int, float) or value['clip_length'] < 0 or
                value.get('destroy_delay') != value['clip_length'] or value.get('original_destroy_requested') is not True):
            raise ValueError('Incomplete original source audio creation or destruction provenance')
        if value['object_id'] in identities:
            raise ValueError('Repeated actual source audio object reference')
        identities.add(value['object_id'])
        created = _mark(value.get('created'))
        requested = _mark(value.get('destruction_requested'), 'mainscript.PlayClipAtPoint.Destroy(Object,clip.length)')
        parent_observed = _mark(value.get('parent_observed'))
        if (value['parent_observed']['callback'] not in ('driver.update.exit.parent', 'observer.LateUpdate.parent') or
                not created < requested or not created < parent_observed or any(type(value.get(k)) is not bool for k in
                ('required', 'scope_member', 'created_after_close', 'object_null', 'component_null', 'collider_null')) or
                value['created_after_close'] != (created > closed)):
            raise ValueError('Missing actual source audio parent or scope observation')
        membership = value.get('membership')
        if not isinstance(membership, list) or not membership:
            raise ValueError('Missing exact captured-geometry audio membership observations')
        previous_scope = created
        required = False
        for scope in membership:
            observed = _mark(scope.get('observed'))
            if (observed <= previous_scope or type(scope.get('member')) is not bool or
                    type(scope.get('owner_is_null')) is not bool or type(scope.get('owner_id')) is not int or
                    not {'parent', 'owner'} <= scope.keys() or
                    (scope['owner_is_null'] and (scope['owner_id'] != 0 or scope['owner'])) or
                    (not scope['owner_is_null'] and (scope['owner_id'] == 0 or not scope['owner']))):
                raise ValueError('Invalid actual audio parent or body scope transition')
            if scope['member']:
                required = True
                if observed >= closed:
                    raise ValueError('Captured-geometry audio joined after readiness closure')
            previous_scope = observed
        if value['required'] is not required or value['scope_member'] is not membership[-1]['member']:
            raise ValueError('Audio required cohort differs from actual geometry membership')
        all_null = all(value[k] for k in ('object_null', 'component_null', 'collider_null'))
        if 'removed' not in value:
            raise ValueError('Missing actual source audio removal observation')
        if all_null:
            removed = _mark(value['removed'])
            if (value['removed']['callback'] not in ('driver.update.exit.Unity-null', 'observer.LateUpdate.Unity-null') or
                    not requested < removed or not parent_observed < removed):
                raise ValueError('Invalid actual source audio native removal observation')
        else:
            removed = None
            if value['removed'] and value['removed'].get('order', 0):
                raise ValueError('Live actual source audio conflicts with removal evidence')
        if required and (not all_null or not opened < created < requested < removed < closed):
            raise ValueError('Actual captured-geometry source audio removal is incomplete')
    required_audio = [a for a in audio if a['required']]
    if not required_audio:
        raise ValueError('Missing actual captured-geometry source audio cohort')
    stamps = {s['phase']: s for s in run['driver']['stamps']}
    setup_revision = stamps['stationary-input-applied']['update_sequence']
    prepared_sequence = stamps['stationary-update-exit']['update_sequence']
    held_sequence = stamps['held-update-exit']['update_sequence']
    updates = report.get('updates')
    if not isinstance(updates, list) or len(updates) < 3:
        raise ValueError('Missing completed natural source dependency updates')
    previous = None
    by_sequence = {}
    phases = (('entry', 'driver.update.entry'), ('forward', 'driver.forward.before'),
              ('look_entry', 'driver.look-point.entry'), ('look_exit', 'driver.look-point.exit'),
              ('look_output', 'driver.look.exit'), ('midpoint', 'driver.midpoint.after'), ('exit', 'driver.update.exit'))
    for update in updates:
        sequence = update.get('sequence')
        if (type(sequence) is not int or update.get('setup_revision') != setup_revision or sequence <= setup_revision or
                (previous and sequence != previous['sequence'] + 1)):
            raise ValueError('Invalid original player dependency revision')
        marks = [_mark(update.get(field), callback) for field, callback in phases]
        if marks != sorted(set(marks)) or (previous and marks[0] <= previous['exit']['order']):
            raise ValueError('Original player writer callback order differs')
        for field, _ in phases:
            if update[field]['update_sequence'] != sequence:
                raise ValueError('Original player writer callback revision differs')
        if any(not _vector(update.get(k)) for k in ('entry_look', 'entry_piss', 'entry_piss_forward', 'entry_head', 'entry_head_forward',
                'consumed_look', 'consumed_piss', 'produced_look', 'midpoint_position')) or any(
                not _vector(update.get(k), 'xyzw') for k in ('forward_rotation', 'look_rotation')):
            raise ValueError('Missing exact original player writer values')
        for boundary in ('entry', 'exit'):
            validate_body_state(update.get(boundary+'_body'))
            if (update[boundary+'_body']['callback'] != update[boundary]['callback'] or
                    update[boundary+'_body']['update_sequence'] != sequence or
                    any(not _vector(update.get(boundary+'_'+name)) for name in
                        ('parent_velocity', 'car_velocity_history', 'head_velocity_history', 'head_bob_history')) or
                    type(update.get(boundary+'_stored_dt')) not in (int, float)):
                raise ValueError('Missing exact source body or bob history callback observation')
        if type(update.get('delta_time')) not in (int, float):
            raise ValueError('Missing observed source delta time')
        rays = update.get('rays')
        if not isinstance(rays, list) or len(rays) != 2:
            raise ValueError('Missing original LookPointCalc ray observations')
        for ray in rays:
            if (any(not _vector(ray.get(k)) for k in ('origin', 'direction', 'point')) or
                    type(ray.get('mask')) is not int or type(ray.get('distance')) not in (int, float) or
                    type(ray.get('hit')) is not bool or type(ray.get('collider_id')) is not int or
                    (ray['hit'] and (not ray.get('collider') or ray['collider_id'] == 0)) or
                    (not ray['hit'] and (ray.get('collider') != 'explicit miss' or ray['collider_id'] != 0))):
                raise ValueError('Incomplete original source ray hit or explicit miss')
        fields = update.get('rest_fields')
        if (not isinstance(fields, list) or not fields or len({v.get('key') for v in fields}) != len(fields) or
                any(not v.get('key') or not isinstance(v.get('value'), str) or not v['value'] for v in fields)):
            raise ValueError('Missing exact retained driver geometry fields')
        for value in fields:
            if not isinstance(json.loads(value['value']), dict):
                raise ValueError('Invalid retained driver geometry field')
        dependency = bool(previous and update.get('look_producer') == previous['sequence'] and
                          update.get('midpoint_producer') == previous['sequence'] and
                          update['consumed_look'] == previous['produced_look'] and
                          update['consumed_piss'] == previous['midpoint_position'])
        if update.get('dependency_complete') is not dependency:
            raise ValueError('Invalid original player producer and consumer dependency evidence')
        equal = bool(previous and fields == previous['rest_fields'])
        if update.get('rest_equal') is not equal:
            raise ValueError('Invalid exact stationary driver geometry readiness evidence')
        by_sequence[sequence] = update
        previous = update
    for sequence in (prepared_sequence, held_sequence):
        update = by_sequence.get(sequence, {})
        if update.get('dependency_complete') is not True or update.get('rest_equal') is not True:
            raise ValueError('Held readiness requires consumed post-setup producers and exact natural driver rest')
    if (report['closed']['update_sequence'] != prepared_sequence or closed <= by_sequence[prepared_sequence]['exit']['order'] or
            report['prepared']['update_sequence'] != prepared_sequence):
        raise ValueError('Prepared boundary precedes natural source readiness')
    if by_sequence[held_sequence]['entry']['order'] <= prepared:
        raise ValueError('Held boundary is not an independent next original Update')
    return dict(audio_objects=len(audio), required_audio_objects=len(required_audio), prepared_update_sequence=prepared_sequence, held_update_sequence=held_sequence)
