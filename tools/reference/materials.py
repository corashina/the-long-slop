"""Decode source normal channels before writing glTF textures."""
import math

from PIL import Image, ImageMath

from .source import source_id


def decode_normal_rgba(pixel: tuple[int, int, int, int], encoding: str) -> tuple[float, float, float]:
    if encoding not in {'ag', 'rg', 'rgb'}:
        raise ValueError(f'Unknown normal encoding: {encoding}')
    x = pixel[3 if encoding == 'ag' else 0] * 2 / 255 - 1
    y = pixel[1] * 2 / 255 - 1
    z = pixel[2] * 2 / 255 - 1 if encoding == 'rgb' else math.sqrt(max(0., 1 - x*x - y*y))
    return x, y, z


def decode_normal_image(image: Image.Image, encoding: str) -> Image.Image:
    if encoding not in {'ag', 'rg', 'rgb'}:
        raise ValueError(f'Unknown normal encoding: {encoding}')
    red, green, blue, alpha = image.convert('RGBA').split()
    if encoding == 'rgb':
        return Image.merge('RGB', (red, green, blue))
    red = alpha if encoding == 'ag' else red
    x = red.point(lambda value: value * 2 / 255 - 1, mode='F')
    y = green.point(lambda value: value * 2 / 255 - 1, mode='F')
    z = ImageMath.lambda_eval(lambda p: p['max'](1 - p['x']**2 - p['y']**2, 0)**0.5, x=x, y=y)
    blue = z.point(lambda value: (value + 1) * 127.5 + 0.5).convert('L')
    return Image.merge('RGB', (red, green, blue))


def normal_encoding(texture, shader, shader_name: str) -> str:
    # Unity 2021.3.18f1 uses UnpackNormal/UnpackScaleNormal on D3D11.
    # DXT5nm stores X in alpha; BC5 stores X in red. Both store Y in green.
    platforms = shader.platforms if shader else []
    if shader_name in {'Standard', 'FX/Glass/Stained BumpDistort'} and set(platforms) == {4}:
        from UnityPy.enums import TextureFormat
        formats = {TextureFormat.DXT5: 'ag', TextureFormat.DXT5Crunched: 'ag', TextureFormat.BC5: 'rg'}
        if texture.m_TextureFormat in formats:
            return formats[texture.m_TextureFormat]
    raise ValueError(f'Unresolved normal encoding: {source_id(texture.object_reader)} _BumpMap; '
                     f'shader={shader_name}, platforms={platforms}, format={texture.m_TextureFormat}')
