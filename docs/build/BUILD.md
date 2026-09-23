# Building C2R2-v2 Agent

The C2R2-v2 agent supports two build modes: **Development** and **Production**.

## Build Modes

### Development Mode (Default)
- **Console window visible** for debugging
- **Debug prints enabled** (shows all internal operations)
- **Use for**: Testing, development, debugging

### Production Mode
- **No console window** (100% stealthy)
- **No debug prints** (silent operation)
- **Use for**: Real deployments, red team operations

## Quick Build Commands

### Development Build (with console and debug output)
```bash
cd agent
cargo build --release --features dev --target x86_64-pc-windows-gnu
```

Or simply (since `dev` is the default):
```bash
cd agent
cargo build --release --target x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/agent.exe`

### Production Build (no console, no debug output, stealthy)
```bash
cd agent
cargo build --release --no-default-features --features production --target x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/agent.exe`

## Using the Builder

The builder tool also supports these modes:

### Development Agent
```bash
cd builder
cargo run --release -- build-agent --name agent-dev --server 192.168.1.10:4444
```

### Production Agent (Stealthy)
```bash
cd builder
cargo run --release -- build-agent --name agent-prod --server 192.168.1.10:4444 --production
```

## Differences Between Modes

| Feature | Development | Production |
|---------|------------|------------|
| Console Window |  Visible |  Hidden |
| Debug Prints |  Enabled |  Disabled |
| Stealth |  Low |  High |
| Use Case | Testing/Debug | Real Operations |

## Technical Details

### Windows Subsystem
- **Dev mode**: `windows_subsystem = "console"` - Opens a console window
- **Production mode**: `windows_subsystem = "windows"` - No console window

### Debug Macro
The `debug_print!` macro is used throughout the code:
- In dev mode: Expands to `println!` statements
- In production mode: Expands to nothing (zero runtime cost)

### Code Example
```rust
// This line only prints in dev mode
debug_print!("DEBUG: Connecting to C2 server...");

// In production, this compiles to nothing
```

## Important Notes

 **Always use production mode for real deployments**
- Development builds are easily detected by security products
- Console windows are visible and suspicious
- Debug output can leak sensitive information

 **Use development mode for**
- Testing connectivity
- Debugging command execution
- Verifying module loading
- Troubleshooting issues

## Builder Integration

The builder tool (`builder/`) will be updated to support the `--production` flag:
- Without flag: Builds development agent
- With `--production`: Builds production agent (stealthy)

Example:
```bash
# Development agent
cargo run --release -- build-agent --name test-agent --server 10.0.0.5:4444

# Production agent (stealthy)
cargo run --release -- build-agent --name prod-agent --server 10.0.0.5:4444 --production
```

## Verification

To verify which mode an agent was built with:

### Development Agent
- When run, you'll see a console window
- Debug messages will be printed
- Easier to troubleshoot

### Production Agent
- No console window appears
- Runs silently in background
- No debug output
- Fully stealthy

## Troubleshooting

### Issue: Need to press Enter to execute commands
**Solution**: This is a known issue in the original code. The stdin blocking has been addressed, but if you experience this:
1. Rebuild with production mode (recommended)
2. Use the latest code which has fixed the read_line blocking issue

### Issue: Agent is detected by AV
**Solution**:
1. Always use production mode for real operations
2. Ensure you've built with `--features production`
3. Development builds are intentionally more visible for debugging

### Issue: Can't debug issues in production builds
**Solution**:
1. Reproduce the issue with a development build
2. Debug with console window and debug prints
3. Once fixed, rebuild in production mode
