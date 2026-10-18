"""Source draw-distance settings for the selected profile."""
import struct

# The source minimum. All distance-dependent exports use this one value.
DRAW_DISTANCE = 500.


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def fraction(settings, draw=DRAW_DISTANCE):
    """settingsscript.DrawDistanceChanged: float32 InverseLerp, clamped."""
    lo, hi = settings['DDSmin']['drawDistance'], settings['DDSmax']['drawDistance']
    return max(0., min(1., f32(f32(draw - lo) / f32(hi - lo))))


def setting(settings, name, draw=DRAW_DISTANCE):
    """settingsscript.DrawDistanceChanged: float32 Lerp between DDSmin and DDSmax."""
    lo, hi = settings['DDSmin'][name], settings['DDSmax'][name]
    return f32(lo + f32(f32(hi - lo) * fraction(settings, draw)))


def distances(settings, captured):
    """Recompute the captured source distances, then return them for DRAW_DISTANCE."""
    draw = captured['drawDistance']
    for name, value in captured.items():
        if setting(settings, name, draw) != value:
            raise ValueError(f'Source draw-distance interpolation differs from captured distances.{name}')
    return {name: setting(settings, name) for name in captured}
