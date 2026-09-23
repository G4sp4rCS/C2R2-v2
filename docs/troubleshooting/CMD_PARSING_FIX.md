# Command Parsing Fix for C2R2

## Problem Statement

The C2R2 server had issues handling commands with quoted arguments and paths containing spaces. Examples of failing commands:

```
C2R2[1]> /cmd dir "C:\Program Files"
# Failed: Arguments split incorrectly

C2R2[1]> /cmd dir 'C:\Program Files (x86)'
# Failed: Single quotes not respected

C2R2[1]> /cmd dir "..\..\Program Files (x86)"
# Failed: Quoted path with spaces broken
```

## Root Cause

The issue occurred in two places:

1. **Server command parsing** (`c2r2-server/src/main.rs:660`): Used `split_whitespace()` which doesn't respect quotes
2. **Agent command obfuscation** (`agent/src/argfuscator.rs`): Also used `split_whitespace()` which broke quoted arguments

## Solution

### Server Changes (`c2r2-server/src/main.rs`)

Added two new functions:

#### 1. `parse_command_line(line: &str) -> Vec<String>`

Parses command line input respecting both single (`'`) and double (`"`) quotes:

```rust
/// Parses a command line string respecting quotes (both single and double)
/// Examples:
/// - `dir "C:\Program Files"` -> ["dir", "C:\Program Files"]
/// - `dir 'C:\Program Files'` -> ["dir", "C:\Program Files"]
/// - `dir C:\Windows` -> ["dir", "C:\Windows"]
```

**Key features:**
- Recognizes double quotes (`"`)
- Recognizes single quotes (`'`)
- Handles nested quotes correctly
- Removes quote characters from parsed arguments
- Preserves whitespace within quotes

#### 2. `reconstruct_command(args: &[String]) -> String`

Reconstructs a command from parsed arguments, adding quotes where necessary:

```rust
/// Reconstructs a command line from parsed arguments, adding quotes where needed
/// Arguments containing spaces or special characters will be quoted
```

**Key features:**
- Automatically quotes arguments containing spaces
- Preserves arguments without spaces as-is
- Ensures proper quote placement

### Agent Changes (`agent/src/argfuscator.rs`)

Added three new functions to handle quoted arguments during obfuscation:

#### 1. `parse_command_args(command: &str) -> Vec<String>`

Similar to server's parser, extracts arguments respecting quotes.

#### 2. `reconstruct_command_args(args: &[String]) -> String`

Reconstructs command from arguments, adding quotes where needed.

#### 3. Updated `obfuscate_command()`

Modified to use the new quote-aware parser instead of `split_whitespace()`:

**Before:**
```rust
let parts: Vec<&str> = result.split_whitespace().collect();
```

**After:**
```rust
let args = parse_command_args(&result);
```

This ensures that arguments are properly grouped before obfuscation is applied.

#### 4. Updated `add_quotes_to_args()`

Modified to handle pre-quoted arguments:

**Before:**
- Split by whitespace (broke quoted args)
- Only quoted paths with special chars

**After:**
- Uses `parse_command_args()` to respect existing quotes
- Quotes arguments with spaces or special characters

## Command Handler Updates

Updated multiple command handlers to use the new parsing:

### `/cmd` and `/cmd_all`
```rust
// Before:
let command = parts[1..].join(" ");

// After:
let command = reconstruct_command(&parts[1..]);
```

### `/download` and `/upload`
```rust
// Before:
let remote_path = parts[1..].join(" ").trim_matches('"').to_string();

// After:
let remote_path = parts[1..].join(" ");  // Quotes already handled
```

### `/encrypt` and `/decrypt`
Updated to work with `Vec<String>` instead of `Vec<&str>`.

## Testing

The parser was validated with various test cases:

```rust
Input:  dir "C:\Program Files"
Parsed: ["dir", "C:\\Program Files"]
Output: dir "C:\Program Files"

Input:  dir 'C:\Program Files (x86)'
Parsed: ["dir", "C:\\Program Files (x86)"]
Output: dir "C:\Program Files (x86)"

Input:  dir C:\Windows
Parsed: ["dir", "C:\\Windows"]
Output: dir C:\Windows

Input:  dir "..\..\Program Files (x86)"
Parsed: ["dir", "..\\..\\Program Files (x86)"]
Output: dir "..\..\Program Files (x86)"
```

## Usage Examples

After the fix, these commands now work correctly:

```bash
# Navigate to directories with spaces
C2R2[1]> /cmd cd "C:\Program Files"

# List directory contents
C2R2[1]> /cmd dir "C:\Program Files (x86)"

# Execute commands with paths containing spaces
C2R2[1]> /cmd type "C:\Users\User Name\Documents\file.txt"

# Use single quotes (converted to double quotes internally)
C2R2[1]> /cmd dir 'C:\Program Files'

# Mix of quoted and unquoted arguments
C2R2[1]> /cmd copy file.txt "C:\My Documents\"
```

## Backward Compatibility

The changes are fully backward compatible:

- Commands without quotes work as before
- Commands with quotes now work correctly
- Existing command handlers continue to function
- Obfuscation still applies to all commands

## Benefits

1. **User Experience**: Users can now use natural quoting for paths with spaces
2. **Reliability**: Commands execute correctly on Windows targets
3. **Consistency**: Both single and double quotes work
4. **Maintainability**: Clear parsing logic that's easy to understand and extend

## Files Modified

1. `c2r2-server/src/main.rs` - Server command parsing and reconstruction
2. `agent/src/argfuscator.rs` - Agent command obfuscation with quote awareness
