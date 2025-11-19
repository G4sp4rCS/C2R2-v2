# Changelog

All notable changes to C2R2-v2 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Critical: Persistence mechanism failures after reboot** ([#persistence-fix])
  - Fixed issue where persistence entries pointed to non-existent executable paths
  - Agent now intelligently detects if running from temporary location (Downloads, Desktop, USB)
  - Automatically copies to persistent AppData location only when needed
  - Uses anti-AV techniques for file copying (chunk-based, hidden attributes)
  - Prevents "Windows cannot find executable" errors after system reboot
  - See [PERSISTENCE_FIX.md](PERSISTENCE_FIX.md) for detailed documentation

### Added
- Smart executable location detection in persistence module
- Functions: `is_persistent_location()` and `is_temporary_location()`
- Intelligent file relocation with `ensure_persistent_location()`
- Anti-AV file copy technique using variable-sized chunks
- Comprehensive documentation in PERSISTENCE_FIX.md

### Changed
- `get_current_exe_path()` now ensures executable is in persistent location before establishing persistence
- Persistence mechanism only copies executable when actually needed (reduces AV detection)

## [2.0.0] - 2024-01-15

### Added
- Complete professional documentation in `/docs` directory
  - Architecture documentation
  - Installation guide
  - Usage reference
  - Module documentation
  - API reference
  - Security best practices
  - Contributing guidelines
  - Development guide
- Rust idiomatic documentation comments (///, //!) throughout codebase
- MIT License with educational use disclaimer
- Changelog file

### Changed
- Improved code documentation with comprehensive examples
- Enhanced module-level documentation with safety notes

### Documentation
- Added comprehensive `/docs` folder with 9 documentation files
- Added inline Rust documentation to all major modules
- Added LICENSE file with educational use disclaimer
- Created this CHANGELOG.md

## [1.0.0] - Previous Releases

### Version 2.0 - Direct Connection
- Direct TCP connection (no shellcode)
- Multi-client support with async server
- System information auto-collection
- Remote command execution via cmd
- Command obfuscation (ArgFuscator)
- File transfer (download/upload) with Base64
- Beacon communication with jitter
- Multiple persistence mechanisms:
  - Registry Run keys
  - Scheduled Tasks
  - WMI Event Subscriptions
  - Startup folder
- Colored CLI with tables
- Cross-compilation support (Linux → Windows)
- Lightweight agent (~60KB)

### Credential Harvesting Module
- Chrome/Chromium-based browsers
- Firefox-based browsers
- Discord token extraction
- Telegram session hijacking
- Cryptocurrency wallet discovery
- Gaming platform credentials (Steam, Epic)
- Autofill data and credit cards

### Evasion Techniques
- Direct syscalls to bypass hooks
- String obfuscation with obfstr
- Command-line obfuscation
- Anti-debugging checks
- Anti-VM detection
- Sandbox evasion

### Server Features
- Async multi-client handling with Tokio
- Interactive CLI with rustyline
- Pretty-printed tables
- Structured logging with tracing
- Command history
- Client session management

### Builder Tool
- Agent generation with embedded config
- Module encryption (AES-256-GCM)
- Key management
- Cross-compilation automation

## Roadmap

### Planned for 2.1.0
- [ ] HTTP/HTTPS C2 protocol
- [ ] DNS tunneling support
- [ ] Domain fronting
- [ ] Certificate pinning
- [ ] Additional persistence methods
- [ ] Process injection capabilities
- [ ] Memory-only execution improvements

### Planned for 2.2.0
- [ ] Lateral movement module
- [ ] Privilege escalation techniques
- [ ] Network scanner module
- [ ] Keylogger module
- [ ] Screenshot capture

### Planned for 3.0.0
- [ ] Web-based C2 interface
- [ ] Telegram bot interface
- [ ] Multi-user support
- [ ] Agent relay/pivot capabilities
- [ ] Plugin system
- [ ] RESTful API

## Security Fixes

All security issues should be reported privately via GitHub Security Advisories.

---

## Version History

### Legend
- `Added` for new features
- `Changed` for changes in existing functionality
- `Deprecated` for soon-to-be removed features
- `Removed` for now removed features
- `Fixed` for any bug fixes
- `Security` for vulnerability fixes
- `Documentation` for documentation changes

---

**Note**: This project is for educational and authorized testing only. See LICENSE for full disclaimer.
