"""Reject missing hold ownership, motion, release, and solve evidence."""


def stationary(body):
    return (body.get('present') and body.get('kinematic') and
            all(body.get(field) == dict(x=0., y=0., z=0.) for field in ('velocity', 'angular_velocity')))


def validate_body_hold(run):
    report = run.get('body_trace', {})
    events = report.get('events', [])
    isolated = run.get('isolated_wheel_probe', False)
    if not events or not report.get('ended') or report.get('hold_failure') or report.get('isolated') != isolated:
        raise ValueError('Body hold evidence is missing or failed')
    original = report.get('retained_instance_id')
    copied = report.get('copied_instance_id')
    if not original or not report.get('retained_body') or (isolated and (not copied or copied == original or not report.get('copied_body'))):
        raise ValueError('Body hold roles are missing or conflated')
    original_hold = copy_hold = created = released = False
    prepared = backlog = ready = ended = False
    start_step = None
    solves = []
    for index, event in enumerate(events):
        kind, callback = event.get('kind'), event.get('callback')
        target, retained, copy = (event.get(name, {}) for name in ('target', 'retained', 'copied'))
        if (event.get('order') != index + 1 or retained.get('instance_id') != original or
                retained.get('key') != report['retained_body'] or not event.get('current_is_retained') or
                event.get('current', {}).get('instance_id') != original):
            raise ValueError('Body hold order or retained ownership changed')
        if kind == 'hold.started':
            if callback == 'observer.original.hold' and not original_hold and start_step is None:
                original_hold = True
                start_step = event['physics_step']
            elif callback == 'observer.copy.hold' and created and not copy_hold and not released:
                copy_hold = True
                previous = events[index - 1]
                if previous['kind'] != 'copy.created' or any(event[k] != previous[k] for k in ('physics_step', 'frame', 'time')):
                    raise ValueError('Copied body hold did not precede setup solves')
            else:
                raise ValueError('Body hold registration differs')
        if kind == 'copy.created':
            if not isolated or created or not original_hold or released or target.get('instance_id') != copied:
                raise ValueError('Copied body creation role or order differs')
            created = True
        if created and (copy.get('instance_id') != copied or copy.get('key') != report['copied_body']):
            raise ValueError('Copied body ownership changed')
        if kind == 'hold.boundary':
            if callback == 'prepared-held': prepared = True
            if callback == 'backlog-drained-held':
                if not prepared: raise ValueError('Body hold preparation is missing')
                backlog = True
            if callback == 'before-dynamic':
                if not backlog: raise ValueError('Body hold release is premature')
                ready = True
        if kind in ('hold.release.before', 'hold.release.after'):
            if not ready or released or target.get('instance_id') != (copied if isolated else original):
                raise ValueError('Body hold release is premature or targets the wrong role')
            if kind == 'hold.release.after':
                if events[index - 1]['kind'] != 'hold.release.before' or target.get('kinematic'):
                    raise ValueError('Body hold release evidence is incomplete')
                released = True
                if isolated: copy_hold = False
                else: original_hold = False
        if event.get('original_hold') != original_hold or event.get('copy_hold') != copy_hold:
            raise ValueError('Body hold ownership window differs')
        transient = kind in ('setter.after', 'hold.restore.before') and not event.get('requested', False)
        # A restoration requests true, but preserves the just-observed dynamic source result.
        transient = transient or kind == 'hold.restore.before'
        if original_hold and not transient and not stationary(retained):
            raise ValueError('Original body hold was violated')
        if copy_hold and not stationary(copy):
            raise ValueError('Copied body hold was violated')
        if kind == 'setter.after' and original_hold and not event.get('requested'):
            following = events[index + 1:index + 3]
            if ([e['kind'] for e in following] != ['hold.restore.before', 'hold.restore.after'] or
                    any(e.get('call_id') != event.get('call_id') or not e.get('requested') for e in following) or
                    not stationary(following[-1].get('retained', {}))):
                raise ValueError('Original source release has no distinct observer restoration')
        if kind.startswith('hold.restore.'):
            offset = 1 if kind.endswith('before') else 2
            source = events[index - offset] if index >= offset else {}
            if source.get('kind') != 'setter.after' or source.get('requested') or not original_hold:
                raise ValueError('Observer restoration is outside an original hold')
        if kind == 'hold.solve': solves.append(event['physics_step'])
        if kind == 'hold.end':
            if not released or index != len(events) - 1:
                raise ValueError('Body hold end state differs')
            ended = True
    if (start_step is None or not released or not ended or isolated != created or
            solves != list(range(start_step + 1, events[-1]['physics_step'] + 1)) or not solves):
        raise ValueError('Body hold setup solve or lifecycle evidence is incomplete')
    return {'solves': len(solves), 'isolated': isolated, 'original_instance_id': original, 'copied_instance_id': copied}
