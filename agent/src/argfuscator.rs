//! Command-line obfuscation module (ArgFuscator).
//!
//! This module implements command-line obfuscation techniques similar to Invoke-ArgFuscator
//! to evade detection by security products that monitor command execution.
//!
//! # Techniques
//!
//! - **Random Case**: `whoami` → `wHoAmI`
//! - **Caret Insertion**: `whoami` → `who^am^i`
//! - **Quote Wrapping**: `whoami` → `"w"h"o"ami`
//! - **Environment Variables**: `cmd` → `%COMSPEC%`
//!
//! # References
//!
//! Based on: <https://github.com/wietze/Invoke-ArgFuscator>
//!
//! # Examples
//!
//! ```no_run
//! use agent::argfuscator::obfuscate;
//!
//! let obfuscated = obfuscate("whoami");
//! // Possible output: "wHo^Am^I"
//! ```

use rand::Rng;

/// Obfuscation configuration
pub struct ObfuscatorConfig {
    /// Probability of applying random case changes (0.0 - 1.0)
    pub random_case_prob: f32,
    /// Probability of inserting obfuscation characters (0.0 - 1.0)
    pub char_insertion_prob: f32,
    /// Enable quote insertion around arguments
    pub quote_insertion: bool,
    /// Enable environment variable substitution
    pub env_var_substitution: bool,
}

impl Default for ObfuscatorConfig {
    fn default() -> Self {
        Self {
            random_case_prob: 0.5,
            char_insertion_prob: 0.3,
            quote_insertion: true,
            env_var_substitution: true,
        }
    }
}

impl ObfuscatorConfig {
    /// Creates a high obfuscation config
    pub fn high() -> Self {
        Self {
            random_case_prob: 0.7,
            char_insertion_prob: 0.5,
            quote_insertion: true,
            env_var_substitution: true,
        }
    }

    /// Creates a low obfuscation config
    pub fn low() -> Self {
        Self {
            random_case_prob: 0.3,
            char_insertion_prob: 0.2,
            quote_insertion: false,
            env_var_substitution: false,
        }
    }

    /// Creates a disabled obfuscation config (for testing/debugging)
    pub fn disabled() -> Self {
        Self {
            random_case_prob: 0.0,
            char_insertion_prob: 0.0,
            quote_insertion: false,
            env_var_substitution: false,
        }
    }
}

/// Applies random case changes to a string
/// Example: "whoami" -> "wHoAmI"
fn apply_random_case(input: &str, probability: f32) -> String {
    let mut rng = rand::thread_rng();
    input
        .chars()
        .map(|c| {
            if c.is_alphabetic() && rng.gen::<f32>() < probability {
                if rng.gen_bool(0.5) {
                    c.to_uppercase().next().unwrap()
                } else {
                    c.to_lowercase().next().unwrap()
                }
            } else {
                c
            }
        })
        .collect()
}

/// Inserts obfuscation characters (carets) in Windows commands
/// Example: "whoami" -> "who^am^i"
/// Note: Carets are command separators in cmd.exe that can be used for obfuscation
fn insert_obfuscation_chars(input: &str, probability: f32) -> String {
    let mut rng = rand::thread_rng();
    let mut result = String::new();
    
    for (i, c) in input.chars().enumerate() {
        result.push(c);
        // Don't insert after last character or after special chars
        if i < input.len() - 1 && c.is_alphanumeric() && rng.gen::<f32>() < probability {
            result.push('^');
        }
    }
    
    result
}

/// Parses a command line string respecting quotes (both single and double)
/// Returns a Vec of arguments with quotes removed
fn parse_command_args(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;
    let mut chars = command.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
                // Don't include the quote character itself
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
                // Don't include the quote character itself
            }
            ' ' | '\t' if !in_double_quotes && !in_single_quotes => {
                // Whitespace outside quotes: end current argument
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => {
                // Regular character or whitespace inside quotes
                current_arg.push(ch);
            }
        }
    }
    
    // Add final argument if any
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    
    args
}

/// Reconstructs a command line from parsed arguments, adding quotes where needed
fn reconstruct_command_args(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }
    
    let mut result = String::new();
    
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        
        // Quote if argument contains spaces
        if arg.contains(' ') {
            result.push('"');
            result.push_str(arg);
            result.push('"');
        } else {
            result.push_str(arg);
        }
    }
    
    result
}

/// Adds quotes around arguments
/// Example: "curl http://example.com" -> "curl \"http://example.com\""
fn add_quotes_to_args(command: &str) -> String {
    let args = parse_command_args(command);
    if args.is_empty() {
        return command.to_string();
    }
    
    let mut result = args[0].to_string();
    for arg in args.iter().skip(1) {
        // Add quotes if not already quoted and contains special chars
        if arg.contains('/') || arg.contains(':') || arg.contains('\\') || arg.contains(' ') {
            result.push_str(&format!(" \"{}\"", arg));
        } else {
            result.push_str(&format!(" {}", arg));
        }
    }
    
    result
}

/// Substitutes common Windows paths with environment variables
/// Example: "C:\Windows\System32" -> "%windir%\System32"
fn substitute_env_vars(command: &str) -> String {
    let mut result = command.to_string();
    
    // Common Windows path substitutions
    let substitutions = [
        ("C:\\Windows", "%windir%"),
        ("C:\\windows", "%windir%"),
        ("c:\\Windows", "%windir%"),
        ("c:\\windows", "%windir%"),
        ("C:\\Program Files", "%ProgramFiles%"),
        ("C:\\Program Files (x86)", "%ProgramFiles(x86)%"),
        ("C:\\Users", "%SystemDrive%\\Users"),
    ];
    
    for (path, env_var) in &substitutions {
        result = result.replace(path, env_var);
    }
    
    result
}

/// Main obfuscation function
/// Applies various obfuscation techniques to a Windows command
/// 
/// # Examples
/// 
/// ```
/// let obfuscated = obfuscate_command("whoami", &ObfuscatorConfig::default());
/// // Result might be: "wH^o^A^mi" (varies due to randomization)
/// ```
/// 
/// ```
/// let obfuscated = obfuscate_command("curl http://example.com", &ObfuscatorConfig::high());
/// // Result might be: "cU^r^L \"http://example.com\"" (varies due to randomization)
/// ```
pub fn obfuscate_command(command: &str, config: &ObfuscatorConfig) -> String {
    let mut result = command.to_string();
    
    // Apply environment variable substitution first
    if config.env_var_substitution {
        result = substitute_env_vars(&result);
    }
    
    // Parse command respecting quotes to get proper arguments
    let args = parse_command_args(&result);
    if args.is_empty() {
        return result;
    }
    
    let mut obfuscated_args = Vec::new();
    
    for (i, arg) in args.iter().enumerate() {
        let mut obfuscated_arg = arg.to_string();
        
        // Apply random case (except for paths and quoted strings)
        if !arg.contains('\\') && !arg.contains('/') && !arg.starts_with('%') {
            obfuscated_arg = apply_random_case(&obfuscated_arg, config.random_case_prob);
        }
        
        // Apply character insertion to the command name and some arguments
        if i == 0 || (i > 0 && !arg.contains(':') && !arg.contains('\\') && !arg.contains('/')) {
            obfuscated_arg = insert_obfuscation_chars(&obfuscated_arg, config.char_insertion_prob);
        }
        
        obfuscated_args.push(obfuscated_arg);
    }
    
    // Reconstruct command with proper quoting
    result = reconstruct_command_args(&obfuscated_args);
    
    // Apply quote insertion for paths if enabled
    if config.quote_insertion {
        result = add_quotes_to_args(&result);
    }
    
    result
}

/// Obfuscates a command with default configuration
pub fn obfuscate(command: &str) -> String {
    obfuscate_command(command, &ObfuscatorConfig::default())
}

/// Obfuscates a command with high obfuscation level
pub fn obfuscate_high(command: &str) -> String {
    obfuscate_command(command, &ObfuscatorConfig::high())
}

/// Obfuscates a command with low obfuscation level
pub fn obfuscate_low(command: &str) -> String {
    obfuscate_command(command, &ObfuscatorConfig::low())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_case() {
        let input = "whoami";
        let output = apply_random_case(input, 1.0); // 100% probability
        assert_eq!(input.len(), output.len());
        // Check that at least some characters changed
        assert_ne!(input, output);
    }

    #[test]
    fn test_char_insertion() {
        let input = "whoami";
        let output = insert_obfuscation_chars(input, 1.0);
        assert!(output.len() > input.len());
        assert!(output.contains('^'));
    }

    #[test]
    fn test_env_var_substitution() {
        let input = "C:\\Windows\\System32\\cmd.exe";
        let output = substitute_env_vars(input);
        assert!(output.contains("%windir%"));
        assert!(!output.contains("C:\\Windows"));
    }

    #[test]
    fn test_obfuscate_basic() {
        let input = "whoami";
        let output = obfuscate(input);
        // Should be different but still a valid command
        assert!(!output.is_empty());
    }

    #[test]
    fn test_parse_command_args_with_double_quotes() {
        let input = r#"dir "C:\Program Files""#;
        let args = parse_command_args(input);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "dir");
        assert_eq!(args[1], "C:\\Program Files");
    }

    #[test]
    fn test_parse_command_args_with_single_quotes() {
        let input = r"dir 'C:\Program Files (x86)'";
        let args = parse_command_args(input);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "dir");
        assert_eq!(args[1], "C:\\Program Files (x86)");
    }

    #[test]
    fn test_parse_command_args_no_quotes() {
        let input = "dir C:\\Windows";
        let args = parse_command_args(input);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "dir");
        assert_eq!(args[1], "C:\\Windows");
    }

    #[test]
    fn test_reconstruct_command_with_spaces() {
        let args = vec![
            "dir".to_string(),
            "C:\\Program Files".to_string(),
        ];
        let output = reconstruct_command_args(&args);
        assert_eq!(output, r#"dir "C:\Program Files""#);
    }

    #[test]
    fn test_reconstruct_command_without_spaces() {
        let args = vec![
            "dir".to_string(),
            "C:\\Windows".to_string(),
        ];
        let output = reconstruct_command_args(&args);
        assert_eq!(output, "dir C:\\Windows");
    }

    #[test]
    fn test_obfuscate_with_quoted_path() {
        let input = r#"dir "C:\Program Files""#;
        let output = obfuscate_command(input, &ObfuscatorConfig::disabled());
        // With disabled config, should preserve the path with quotes
        assert!(output.contains("C:\\Program Files") || output.contains(r#""C:\Program Files""#));
    }
}
