"""Source draw-distance settings. The game resolves them at runtime."""
import struct

FIELDS = ('drawDistance', 'fogModifier', 'Cam1FarPlane', 'Cam2NearPlane', 'Cam2FarPlane',
          'TerrainGenDist', 'TerrainRemoveDist', 'DistantTerrainGenDist', 'DistantTerrainRemoveDist',
          'DistantTerrain2GenDist', 'DistantTerrain2RemoveDist',
          'DistantTerrainShader1Near', 'DistantTerrainShader1Far', 'DistantTerrainShader1Amount',
          'DistantTerrainShader2Near', 'DistantTerrainShader2Far', 'DistantTerrainShader2Amount',
          'ObjGenDist', 'ObjRemoveDist', 'MountainGenDist', 'MountainRemoveDist',
          'BuildingGenDist', 'BuildingRemoveDist', 'RoadGenDist',
          'ItemRemoveDist', 'ItemSpawnDist', 'ItemFreezeDist', 'ItemUnFreezeDist')


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def fraction(settings, draw):
    """settingsscript.DrawDistanceChanged: float32 InverseLerp, clamped."""
    lo, hi = settings['DDSmin']['drawDistance'], settings['DDSmax']['drawDistance']
    return max(0., min(1., f32(f32(draw - lo) / f32(hi - lo))))


def setting(settings, name, draw):
    """settingsscript.DrawDistanceChanged: float32 Lerp between DDSmin and DDSmax."""
    lo, hi = settings['DDSmin'][name], settings['DDSmax'][name]
    return f32(lo + f32(hi - lo) * fraction(settings, draw))


def record(source, settings, captured):
    """Check the captured scene distances against the source lerp, then return both source sets."""
    draw = captured['drawDistance']
    for name, value in captured.items():
        if setting(settings, name, draw) != value:
            raise ValueError(f'Source draw-distance interpolation differs from captured distances.{name}')
    return {'source': source,
            'minimum': {name: settings['DDSmin'][name] for name in FIELDS},
            'maximum': {name: settings['DDSmax'][name] for name in FIELDS},
            'default': settings['S']['FDrawDistance']}
