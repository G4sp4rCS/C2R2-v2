#!/usr/bin/env python3
"""
Automatic String Obfuscation Tool for Rust
Replaces ALL string literals with obfstr!() macro calls
"""

import re
import os
import glob
from pathlib import Path

def obfuscate_rust_strings(content):
    """
    Replace all string literals with obfstr!() calls
    Handles:
    - Regular strings: "text" → obfstr!("text")
    - Raw strings: r"text" → obfstr!(r"text")
    - Byte strings: b"text" → obfstr!("text").as_bytes()
    - Already obfuscated strings (skip them)
    
    SKIP:
    - #[cfg(...)] attributes
    - Format placeholders {}
    - Escape sequences in certain contexts
    """
    
    # Step 1: Handle byte strings b"..." → obfstr!("...").as_bytes()
    def replace_byte_string(match):
        before_context = content[max(0, match.start()-30):match.start()]
        
        # Skip if already obfuscated
        if 'obfstr!' in before_context:
            return match.group(0)
        
        # Skip if inside #[cfg(...)]
        if '#[cfg' in before_context:
            return match.group(0)
        
        string_content = match.group(2)
        return f'obfstr!("{string_content}").as_bytes()'
    
    content = re.sub(r'\b(b)"([^"]*)"', replace_byte_string, content)
    
    # Step 2: Handle raw strings r"..." → obfstr!(r"...")
    def replace_raw_string(match):
        before_context = content[max(0, match.start()-30):match.start()]
        
        # Skip if already obfuscated
        if 'obfstr!' in before_context:
            return match.group(0)
        
        # Skip if inside #[cfg(...)]
        if '#[cfg' in before_context:
            return match.group(0)
        
        string_content = match.group(2)
        return f'obfstr!(r"{string_content}")'
    
    content = re.sub(r'\b(r)"([^"]*)"', replace_raw_string, content)
    
    # Step 3: Handle regular strings "..." → obfstr!("...")
    def replace_regular_string(match):
        string_content = match.group(1)
        
        # Get context before the string (50 chars)
        before_context = content[max(0, match.start()-50):match.start()]
        
        # Skip if already obfuscated
        if 'obfstr!' in before_context:
            return match.group(0)
        
        # Skip if inside #[cfg(...)] or #[target_os = ...]
        if re.search(r'#\[(cfg|target_os)', before_context):
            return match.group(0)
        
        # Skip if inside format!(), println!(), writeln!(), write!(), panic!(), etc.
        if re.search(r'(format|println|writeln|write|panic|assert|debug|info|warn|error)!\s*\($', before_context):
            return match.group(0)
        
        # Skip format string placeholders (contains {} or {:)
        if '{' in string_content:
            return match.group(0)
        
        # Skip very short strings (likely single chars or empty)
        if len(string_content) <= 1:
            return match.group(0)
        
        # Skip if it's part of an escape sequence pattern (e.g., "\" followed by letter)
        if string_content.startswith('\\') and len(string_content) == 2:
            return match.group(0)
        
        return f'obfstr!("{string_content}")'
    
    # Match strings that are NOT already inside obfstr!()
    content = re.sub(r'"([^"]*)"', replace_regular_string, content)
    
    return content

def add_obfstr_import(content):
    """
    Add 'use obfstr::obfstr;' if not already present
    """
    if 'use obfstr::obfstr' in content:
        return content  # Already imported
    
    # Find the first 'use' statement and add after it
    lines = content.split('\n')
    insert_index = -1
    
    for i, line in enumerate(lines):
        if line.strip().startswith('use '):
            insert_index = i + 1
    
    if insert_index == -1:
        # No use statements, add at top after comments
        for i, line in enumerate(lines):
            if not line.strip().startswith('//') and line.strip() != '':
                insert_index = i
                break
    
    if insert_index != -1:
        lines.insert(insert_index, 'use obfstr::obfstr;')
        return '\n'.join(lines)
    
    return content

def process_rust_file(file_path):
    """
    Process a single Rust file
    """
    print(f"Processing: {file_path}")
    
    with open(file_path, 'r', encoding='utf-8') as f:
        original_content = f.read()
    
    # Add obfstr import
    content = add_obfstr_import(original_content)
    
    # Obfuscate strings
    content = obfuscate_rust_strings(content)
    
    # Count changes
    original_count = original_content.count('"')
    new_count = content.count('obfstr!')
    changes = new_count - original_content.count('obfstr!')
    
    if changes > 0:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  ✅ Obfuscated {changes} new strings")
    else:
        print(f"  ⏭️  No new strings to obfuscate")
    
    return changes

def main():
    """
    Main function: process all Rust files in stealer-dll/src/stealer/
    """
    script_dir = Path(__file__).parent.parent  # C2R2 root
    stealer_dir = script_dir / "stealer-dll" / "src" / "stealer"
    
    if not stealer_dir.exists():
        print(f"❌ Directory not found: {stealer_dir}")
        return
    
    print(f"🔍 Scanning: {stealer_dir}")
    print("=" * 60)
    
    rust_files = glob.glob(str(stealer_dir / "**" / "*.rs"), recursive=True)
    
    if not rust_files:
        print("❌ No Rust files found")
        return
    
    print(f"Found {len(rust_files)} Rust files\n")
    
    total_changes = 0
    for file_path in sorted(rust_files):
        changes = process_rust_file(file_path)
        total_changes += changes
        print()
    
    print("=" * 60)
    print(f"✅ DONE! Obfuscated {total_changes} new strings across {len(rust_files)} files")
    print("\n Next steps:")
    print("   1. cargo build --release --package stealer-dll")
    print("   2. strings target/release/stealer.dll | grep -E 'Chrome|Edge|SELECT'")
    print("   3. Verify strings are NOT visible")

if __name__ == "__main__":
    main()
