@echo off
REM Script para compilar los diagnósticos de Python a EXE
echo ========================================
echo Compilando diagnosticos a EXE
echo ========================================

REM Verificar que pyinstaller esté instalado
python -m pip show pyinstaller >nul 2>&1
if errorlevel 1 (
    echo [+] Instalando PyInstaller...
    python -m pip install pyinstaller
)

REM Compilar debug_webdata.py
echo.
echo [+] Compilando debug_webdata.py...
pyinstaller --onefile --console --name debug_webdata debug_webdata.py

REM Compilar find_cards.py
echo.
echo [+] Compilando find_cards.py...
pyinstaller --onefile --console --name find_cards find_cards.py

REM Limpiar archivos temporales
echo.
echo [+] Limpiando archivos temporales...
rmdir /s /q build
del /q *.spec

echo.
echo ========================================
echo Ejecutables generados en dist\:
echo   - dist\debug_webdata.exe
echo   - dist\find_cards.exe
echo ========================================
echo.
echo Ahora puedes copiar estos archivos a la VM
pause
