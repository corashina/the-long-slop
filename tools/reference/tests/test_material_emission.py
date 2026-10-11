import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from tools.reference.gltf import GltfExporter
from tools.reference.tests import test_materials


class MaterialEmissionTests(unittest.TestCase):
    def test_source_keyword_controls_emission_factor_and_map(self):
        for enabled in (False, True):
            source, reader = test_materials.MaterialTests().material_source(texture_pointer=12)
            data = source.read(reader)
            data['m_ShaderKeywords'] = '_EMISSION' if enabled else ''
            data['m_SavedProperties']['m_Colors'] = [('_EmissionColor', dict(r=2.67,g=1.24,b=0.,a=1.))]
            data['m_SavedProperties']['m_TexEnvs'] = [('_EmissionMap', data['m_SavedProperties']['m_TexEnvs'][0][1])]
            with self.subTest(enabled=enabled), tempfile.TemporaryDirectory(dir='.cache') as directory, patch('tools.reference.gltf.material_source_evidence'):
                exporter = GltfExporter(source, Path(directory) / 'lamp.gltf', None)
                material = exporter.document.materials[exporter.material(reader)]
                record = exporter.material_records[0]
                strength = material.extensions.get('KHR_materials_emissive_strength', {}).get('emissiveStrength',1.)
                self.assertEqual([v*strength for v in material.emissiveFactor], [2.67,1.24,0.] if enabled else [0.,0.,0.])
                self.assertTrue(all(0 <= v <= 1 for v in material.emissiveFactor))
                self.assertEqual(any(t['slot'] == '_EmissionMap' for t in record['properties']['textures']), enabled)
