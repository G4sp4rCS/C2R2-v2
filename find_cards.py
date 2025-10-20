#!/usr/bin/env python3
"""
Script para encontrar dónde están las tarjetas y direcciones realmente
"""
import sqlite3
import os
from pathlib import Path

def get_all_edge_profiles():
    """Encuentra todos los perfiles de Edge"""
    appdata = os.getenv('LOCALAPPDATA')
    if not appdata:
        return []
    
    edge_dir = Path(appdata) / 'Microsoft' / 'Edge' / 'User Data'
    profiles = []
    
    # Default
    default = edge_dir / 'Default' / 'Web Data'
    if default.exists():
        profiles.append(('Default', default))
    
    # Profile 1, 2, 3, etc.
    for i in range(1, 10):
        profile_path = edge_dir / f'Profile {i}' / 'Web Data'
        if profile_path.exists():
            profiles.append((f'Profile {i}', profile_path))
    
    return profiles

def check_table_data(conn, table_name):
    """Verifica si una tabla tiene datos"""
    try:
        cursor = conn.cursor()
        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        count = cursor.fetchone()[0]
        return count
    except:
        return 0

def main():
    profiles = get_all_edge_profiles()
    
    print(f"\n🔍 Perfiles encontrados: {len(profiles)}")
    
    for profile_name, db_path in profiles:
        print(f"\n{'='*80}")
        print(f"📂 Perfil: {profile_name}")
        print(f"   Path: {db_path}")
        
        import shutil
        temp_path = Path(os.getenv('TEMP')) / f'webdata_{profile_name.replace(" ", "_")}.db'
        
        try:
            shutil.copy2(db_path, temp_path)
            conn = sqlite3.connect(str(temp_path))
            
            # Verificar tablas de tarjetas
            card_tables = [
                'credit_cards',
                'unmasked_credit_cards', 
                'masked_credit_cards',
                'edge_tokenized_credit_cards',
                'server_credit_cards'
            ]
            
            print(f"\n💳 TARJETAS:")
            for table in card_tables:
                count = check_table_data(conn, table)
                if count > 0:
                    print(f"   ✅ {table}: {count} registros")
                    
                    # Mostrar datos
                    try:
                        cursor = conn.cursor()
                        cursor.execute(f"SELECT * FROM {table} LIMIT 1")
                        row = cursor.fetchone()
                        if row:
                            columns = [desc[0] for desc in cursor.description]
                            print(f"      Columnas: {', '.join(columns)}")
                    except:
                        pass
                else:
                    print(f"   ⚪ {table}: 0 registros")
            
            # Verificar tablas de direcciones
            address_tables = [
                'addresses',
                'autofill_profile_addresses',
                'autofill_profiles',
                'edge_server_addresses'
            ]
            
            print(f"\n📍 DIRECCIONES:")
            for table in address_tables:
                count = check_table_data(conn, table)
                if count > 0:
                    print(f"   ✅ {table}: {count} registros")
                    
                    # Mostrar datos
                    try:
                        cursor = conn.cursor()
                        cursor.execute(f"SELECT * FROM {table} LIMIT 1")
                        row = cursor.fetchone()
                        if row:
                            columns = [desc[0] for desc in cursor.description]
                            print(f"      Columnas: {', '.join(columns)}")
                    except:
                        pass
                else:
                    print(f"   ⚪ {table}: 0 registros")
            
            conn.close()
            temp_path.unlink()
            
        except Exception as e:
            print(f"   ❌ Error: {e}")

if __name__ == '__main__':
    main()
