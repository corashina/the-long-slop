"""Recover original first-person camera layers without changing hierarchy."""
from .source import source_id


def normal_view_mask(masks):
    result=0
    for mask in masks:
        if not 0<=mask<=0xffffffff:raise ValueError('Invalid source camera mask')
        result|=mask
    return result


def visible_to_camera(layer, mask):
    if not 0<=layer<32:raise ValueError('Invalid source GameObject layer')
    return bool(mask&(1<<layer))


def recover_camera_visibility(source):
    player=source.get('level1',80125);fields=source.read(player);cameras=[]
    for role in ('Cam','mainCam','farCamera','skyCamera'):
        reader=source.resolve(player,fields[role]);data=source.read(reader)
        cameras.append({'role':role,'source':source_id(reader),'mask':data['m_CullingMask']['m_Bits']})
    return {'cameras':cameras,'normal_view_mask':normal_view_mask(camera['mask'] for camera in cameras)}
