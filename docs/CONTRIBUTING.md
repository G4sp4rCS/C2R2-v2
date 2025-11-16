# Contributing to C2R2-v2

Thank you for your interest in contributing to C2R2-v2! This document provides guidelines for contributing to the project.

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all. We pledge to:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

### Our Standards

**Acceptable behavior**:
- Using welcoming and inclusive language
- Being respectful of differing viewpoints
- Gracefully accepting constructive feedback
- Focusing on what is best for the community
- Showing empathy toward others

**Unacceptable behavior**:
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

### Ethical Use

As this is an offensive security tool:

✅ **Acceptable contributions**:
- Security improvements and bug fixes
- Feature enhancements for legitimate testing
- Documentation improvements
- Performance optimizations
- Code quality improvements

❌ **Unacceptable contributions**:
- Features designed solely for malicious use
- Exploits targeting specific organizations without disclosure
- Features that violate laws or regulations
- Attempts to weaponize the tool for illegal activities

## Getting Started

### Prerequisites

1. **Required Knowledge**:
   - Rust programming language
   - Windows internals (for agent/stealer development)
   - Network programming
   - Security concepts

2. **Development Environment**:
   - Rust 1.70+ with rustup
   - MinGW-w64 for cross-compilation
   - Git for version control
   - Code editor with Rust support (VS Code, RustRover, etc.)

3. **Read the Documentation**:
   - [Architecture](ARCHITECTURE.md) - System design
   - [Development Guide](DEVELOPMENT.md) - Technical details
   - [Security](SECURITY.md) - Security considerations

### Setting Up Development Environment

```bash
# Clone repository
git clone https://github.com/G4sp4rCS/C2R2-v2.git
cd C2R2-v2

# Install dependencies
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64  # On Linux

# Build project
cargo build --workspace

# Run tests
cargo test --workspace

# Check code
cargo clippy --workspace
cargo fmt --check --all
```

## How to Contribute

### Reporting Bugs

**Before submitting a bug report**:
1. Check existing issues to avoid duplicates
2. Verify the bug in the latest version
3. Collect relevant information

**Bug report should include**:
- Clear, descriptive title
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version, etc.)
- Relevant logs or error messages
- Screenshots if applicable

**Example**:
```markdown
## Bug: Agent fails to connect with IPv6 addresses

**Description**: Agent cannot connect to C2 server when IPv6 address is specified.

**Steps to Reproduce**:
1. Build agent with IPv6 server address
2. Start server on IPv6 address
3. Run agent
4. Observe connection failure

**Expected**: Agent connects successfully
**Actual**: Connection times out

**Environment**:
- OS: Windows 11 22H2
- Rust: 1.75.0
- Target: x86_64-pc-windows-gnu

**Logs**:
```
DEBUG: Connecting to [2001:db8::1]:4444
ERROR: Connection failed: Network unreachable
```
```

### Suggesting Enhancements

**Before suggesting an enhancement**:
1. Check if it already exists or is planned
2. Consider if it aligns with project goals
3. Think about implementation feasibility

**Enhancement proposal should include**:
- Clear, descriptive title
- Motivation and use case
- Proposed solution or implementation
- Alternatives considered
- Potential drawbacks or concerns

**Example**:
```markdown
## Feature Request: DNS Beacon Support

**Motivation**: TCP beacons can be blocked by firewalls. DNS is rarely blocked.

**Proposed Solution**:
- Implement DNS tunneling for C2 communication
- Use TXT records for command/data transfer
- Add domain generation algorithm (DGA) support

**Alternatives**:
- ICMP tunneling
- HTTP/HTTPS beacons

**Concerns**:
- More complex implementation
- Higher latency
- May require custom DNS server
```

### Pull Requests

#### Process

1. **Fork and clone**:
   ```bash
   git clone https://github.com/your-username/C2R2-v2.git
   cd C2R2-v2
   git remote add upstream https://github.com/G4sp4rCS/C2R2-v2.git
   ```

2. **Create a branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make changes**:
   - Write code following style guidelines
   - Add tests for new functionality
   - Update documentation
   - Run linters and formatters

4. **Commit changes**:
   ```bash
   git add .
   git commit -m "feat: add DNS beacon support"
   ```

5. **Push and create PR**:
   ```bash
   git push origin feature/your-feature-name
   ```
   Then create pull request on GitHub.

#### Commit Message Guidelines

Follow conventional commits format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples**:
```
feat(agent): add DNS beacon support

Implement DNS tunneling for C2 communication using TXT records.
Includes domain generation algorithm for evasion.

Closes #123
```

```
fix(stealer): handle locked Firefox database

Use shadow copy to access Firefox database when browser is running.
Prevents database locked errors.

Fixes #456
```

#### Code Review Process

1. **Automated checks** must pass:
   - Build succeeds
   - Tests pass
   - Linters pass (clippy)
   - Formatting is correct (rustfmt)

2. **Manual review** by maintainers:
   - Code quality and style
   - Security implications
   - Documentation completeness
   - Test coverage

3. **Feedback and iteration**:
   - Address review comments
   - Update based on feedback
   - Push additional commits

4. **Merge**:
   - Once approved, PR will be merged
   - Your contribution will be credited

## Coding Standards

### Rust Style Guide

Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

```rust
// Good: Clear, idiomatic Rust
pub fn connect_to_server(address: &str) -> Result<TcpStream, io::Error> {
    TcpStream::connect(address)
}

// Bad: Unclear, non-idiomatic
pub fn conn(a: &str) -> Result<TcpStream, io::Error> {
    let s = TcpStream::connect(a);
    s
}
```

### Documentation

**All public items must be documented**:

```rust
/// Connects to the C2 server and initiates beacon loop.
///
/// # Arguments
///
/// * `address` - Server address in format "host:port"
///
/// # Returns
///
/// * `Ok(())` - Successfully connected and operating
/// * `Err(BeaconError)` - Connection or operation failed
///
/// # Examples
///
/// ```no_run
/// beacon::connect("192.168.1.10:4444")?;
/// ```
pub fn connect(address: &str) -> Result<(), BeaconError> {
    // Implementation
}
```

**Use doc comments for modules**:

```rust
//! Beacon module for C2 communication.
//!
//! This module implements configurable beacon timing with jitter
//! and exponential backoff for failed connections.

/// Beacon configuration
pub struct BeaconConfig {
    /// Check-in interval in seconds
    pub interval: u64,
    /// Jitter percentage (0-100)
    pub jitter_percent: u64,
}
```

### Error Handling

**Use Result types**:

```rust
// Good: Explicit error handling
pub fn steal_passwords() -> Result<Vec<Credential>, StealerError> {
    let db = open_database()?;
    let creds = query_passwords(&db)?;
    Ok(creds)
}

// Bad: Panics or unwraps
pub fn steal_passwords() -> Vec<Credential> {
    let db = open_database().unwrap();  // DON'T DO THIS
    query_passwords(&db).unwrap()
}
```

**Define custom error types**:

```rust
#[derive(Debug)]
pub enum BeaconError {
    ConnectionFailed(io::Error),
    InvalidConfig(String),
    Timeout,
}

impl fmt::Display for BeaconError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BeaconError::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            BeaconError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            BeaconError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl Error for BeaconError {}
```

### Testing

**Write tests for new functionality**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_jitter_calculation() {
        let config = BeaconConfig {
            interval: 60,
            jitter_percent: 30,
        };
        
        for _ in 0..100 {
            let sleep = calculate_sleep_duration(&config);
            let secs = sleep.as_secs();
            assert!(secs >= 42 && secs <= 78);
        }
    }

    #[test]
    fn test_command_obfuscation() {
        let cmd = "whoami";
        let obfuscated = obfuscate_command(cmd);
        
        assert_ne!(cmd, obfuscated);
        assert!(obfuscated.to_lowercase().contains("whoami"));
    }
}
```

**Run tests before submitting**:

```bash
cargo test --workspace
cargo test --package agent -- --nocapture  # With output
```

### Security Guidelines

1. **No secrets in code**:
   ```rust
   // Bad: Hardcoded secrets
   const API_KEY: &str = "sk-1234567890abcdef";
   
   // Good: Load from environment or config
   let api_key = env::var("API_KEY")?;
   ```

2. **Input validation**:
   ```rust
   pub fn set_beacon_interval(interval: u64) -> Result<(), ConfigError> {
       if interval < 10 || interval > 3600 {
           return Err(ConfigError::InvalidInterval);
       }
       Ok(())
   }
   ```

3. **Secure by default**:
   ```rust
   // Good: Fail safely
   if let Err(e) = dangerous_operation() {
       log::error!("Operation failed: {}", e);
       return Err(e);
   }
   
   // Bad: Continue on error
   let _ = dangerous_operation();  // Ignoring error
   ```

## Development Workflow

### Branch Strategy

- `main` - Stable releases
- `develop` - Development branch
- `feature/*` - New features
- `fix/*` - Bug fixes
- `docs/*` - Documentation updates

### Release Process

1. **Version Bump**:
   ```toml
   # Update Cargo.toml
   [package]
   version = "2.1.0"
   ```

2. **Changelog Update**:
   ```markdown
   ## [2.1.0] - 2024-01-15
   
   ### Added
   - DNS beacon support
   - Process injection module
   
   ### Fixed
   - Agent connection stability
   - Stealer Firefox handling
   ```

3. **Testing**:
   ```bash
   cargo test --workspace
   cargo build --release --workspace
   # Manual testing
   ```

4. **Tag and Release**:
   ```bash
   git tag -a v2.1.0 -m "Release version 2.1.0"
   git push origin v2.1.0
   ```

## Documentation Contributions

### Types of Documentation

1. **Code Documentation**:
   - Inline comments for complex logic
   - Doc comments for public APIs
   - Module-level documentation

2. **User Documentation**:
   - Usage guides
   - Tutorials
   - Command reference

3. **Developer Documentation**:
   - Architecture documentation
   - API reference
   - Contributing guidelines

### Documentation Standards

**Be Clear and Concise**:
```markdown
# Good
## Beacon Configuration

Configure beacon timing with `/beacon <interval>:<jitter>`.

Example: `/beacon 60:30` sets 60 second interval with ±30% jitter.

# Bad
## Beacon Configuration

You can configure the beacon timing by using the /beacon command.
The format is interval:jitter where interval is the number of seconds...
```

**Include Examples**:
```markdown
# Good
### Downloading Files

```bash
C2R2 [1]> /download C:\Users\john\Desktop\document.pdf
[+] File downloaded: downloads/client1_document.pdf
```

# Bad
### Downloading Files

Use the /download command to download files from the agent.
```

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Pull Requests**: Code contributions
- **Security Issues**: Use GitHub Security Advisory for vulnerabilities

### Getting Help

**Before asking for help**:
1. Read the documentation thoroughly
2. Search existing issues and discussions
3. Try debugging yourself
4. Prepare a minimal reproduction case

**When asking for help**:
1. Be specific and provide details
2. Include code snippets and error messages
3. Describe what you've tried
4. Be patient and respectful

## Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Credited in release notes
- Mentioned in commit messages (Co-authored-by)
- Acknowledged in documentation

## License

By contributing to C2R2-v2, you agree that your contributions will be licensed under the project's license.

## Questions?

If you have questions about contributing:
1. Check this guide and other documentation
2. Search existing issues
3. Open a discussion on GitHub
4. Contact maintainers

---

**Thank you for contributing to C2R2-v2! Your efforts help make this project better for everyone.**
