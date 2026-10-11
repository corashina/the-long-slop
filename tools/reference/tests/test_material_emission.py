import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from tools.reference.gltf import GltfExporter


class MaterialEmissionTests(unittest.TestCase):
    def test_source_keyword_controls_emission_factor_and_map(self):
        reader = SimpleNamespace(assets_file=SimpleNamespace(name="source.assets",source_container_path="source.assets"), path_id=1)
        for enabled in (False, True):
            with self.subTest(enabled=enabled), tempfile.TemporaryDirectory(dir=".cache") as directory:
                data = {
                    "m_Name": "warning lamp", "m_Shader": {},
                    "m_ValidKeywords": ["_EMISSION"] if enabled else [],
                    "m_SavedProperties": {
                        "m_Colors": [("_EmissionColor", {"r": 2.67, "g": 1.24, "b": 0.0, "a": 1.0})],
                        "m_TexEnvs": [("_EmissionMap", {})],
                    },
                }
                shader = SimpleNamespace(assets_file=reader.assets_file,path_id=2)
                source = SimpleNamespace(read=lambda r: data if r is reader else {"m_Name":"Standard"},
                    resolve=lambda *_: shader)
                exporter = GltfExporter(source, Path(directory) / "lamp.gltf", None)
                texture_calls = []
                def texture(*args):
                    texture_calls.append(args)
                    return {"index": 0}
                exporter.texture = texture
                index = exporter.material(reader)
                material = exporter.document.materials[index]
                strength=material.extensions.get('KHR_materials_emissive_strength',{}).get('emissiveStrength',1.)
                self.assertEqual([value*strength for value in material.emissiveFactor], [2.67, 1.24, 0.0] if enabled else [0.0, 0.0, 0.0])
                self.assertTrue(all(0 <= value <= 1 for value in material.emissiveFactor))
                self.assertEqual(material.emissiveTexture is not None, enabled)
                self.assertEqual(len(texture_calls), int(enabled))


if __name__ == "__main__":
    unittest.main()
