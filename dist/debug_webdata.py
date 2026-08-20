#!/usr/bin/env python3
"""
Script para diagnosticar la estructura de Web Data de Edge/Chrome
"""
import sqlite3
import os
from pathlib import Path

def get_edge_web_data():
    """Encuentra la base de datos Web Data de Edge"""
    appdata = os.getenv('LOCALAPPDATA')
    if not appdata:
        return None

    # Buscar en Default y perfiles
    profiles = ['Default', 'Profile 1', 'Profile 2', 'Profile 3']

    for profile in profiles:
        web_data = Path(appdata) / 'Microsoft' / 'Edge' / 'User Data' / profile / 'Web Data'
        if web_data.exists():
            return web_data

    return None

def analyze_database(db_path):
    """Analiza la estructura de la base de datos"""
    print(f"\n Analizando: {db_path}")
    print("=" * 80)

    try:
        conn = sqlite3.connect(str(db_path))
        cursor = conn.cursor()

        # Listar todas las tablas
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
        tables = cursor.fetchall()

        print(f"\n Tablas encontradas ({len(tables)}):")
        for table in tables:
            print(f"  - {table[0]}")

        # Analizar tablas de interés
        interesting_tables = ['credit_cards', 'autofill_profile_addresses', 'autofill_profiles']

        for table_name in interesting_tables:
            try:
                cursor.execute(f"SELECT * FROM {table_name} LIMIT 0")
                columns = [desc[0] for desc in cursor.description]

                print(f"\n Tabla: {table_name}")
                print(f"   Columnas: {', '.join(columns)}")

                # Contar registros
                cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
                count = cursor.fetchone()[0]
                print(f"   Registros: {count}")

                # Mostrar schema
                cursor.execute(f"SELECT sql FROM sqlite_master WHERE name='{table_name}'")
                schema = cursor.fetchone()
                if schema:
                    print(f"   Schema:")
                    for line in schema[0].split('\n'):
                        print(f"     {line}")

            except sqlite3.OperationalError as e:
                print(f"\n Tabla {table_name} no existe o error: {e}")

        conn.close()

    except Exception as e:
        print(f" Error: {e}")

if __name__ == '__main__':
    web_data_path = get_edge_web_data()

    if web_data_path:
        # Copiar a temp (porque puede estar bloqueado)
        import shutil
        temp_path = Path(os.getenv('TEMP')) / 'webdata_debug.db'

        try:
            shutil.copy2(web_data_path, temp_path)
            analyze_database(temp_path)
            temp_path.unlink()
        except Exception as e:
            print(f" Error copiando base de datos: {e}")
    else:
        print(" No se encontró Web Data de Edge")
