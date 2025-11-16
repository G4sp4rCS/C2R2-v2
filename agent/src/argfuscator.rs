// ArgFuscator - Command-line obfuscation module
// Implements command-line obfuscation techniques similar to Invoke-ArgFuscator
// https://github.com/wietze/Invoke-ArgFuscator

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

/// Adds quotes around arguments
/// Example: "curl http://example.com" -> "curl \"http://example.com\""
fn add_quotes_to_args(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }
    
    let mut result = parts[0].to_string();
    for part in parts.iter().skip(1) {
        // Add quotes if not already quoted and contains special chars
        if !part.starts_with('"') && !part.starts_with('\'') && 
           (part.contains('/') || part.contains(':') || part.contains('\\')) {
            result.push_str(&format!(" \"{}\"", part));
        } else {
            result.push_str(&format!(" {}", part));
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
pub fn obfuscate_command(command: &str, config: &ObfuscatorConfig) -> String {
    let mut result = command.to_string();
    
    // Apply environment variable substitution first
    if config.env_var_substitution {
        result = substitute_env_vars(&result);
    }
    
    // Split command to apply different obfuscation to different parts
    let parts: Vec<&str> = result.split_whitespace().collect();
    if parts.is_empty() {
        return result;
    }
    
    let mut obfuscated_parts = Vec::new();
    
    for (i, part) in parts.iter().enumerate() {
        let mut obfuscated_part = part.to_string();
        
        // Apply random case (except for paths and quoted strings)
        if !part.contains('\\') && !part.contains('/') && 
           !part.starts_with('"') && !part.starts_with('\'') &&
           !part.starts_with('%') {
            obfuscated_part = apply_random_case(&obfuscated_part, config.random_case_prob);
        }
        
        // Apply character insertion to the command name and some arguments
        if i == 0 || (i > 0 && !part.contains(':') && !part.contains('\\') && !part.contains('/')) {
            obfuscated_part = insert_obfuscation_chars(&obfuscated_part, config.char_insertion_prob);
        }
        
        obfuscated_parts.push(obfuscated_part);
    }
    
    result = obfuscated_parts.join(" ");
    
    // Apply quote insertion
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
}
