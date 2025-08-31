//! Handles the parsing and substitution of runtime arguments into command templates.
use anyhow::{bail, Result};
use regex::Regex;

/// A structure to hold the result of parsing user-provided arguments.
pub struct ParsedArgs {
    named_args: std::collections::HashMap<String, String>,
    passthrough_args: Vec<String>,
}

/// Parses a vector of raw string arguments into named and passthrough arguments.
///
/// - Named arguments are in the format `KEY:VALUE`.
/// - All other arguments are considered passthrough.
fn parse_arguments(raw_args: &[String]) -> ParsedArgs {
    let mut named_args = std::collections::HashMap::new();
    let mut passthrough_args = Vec::new();

    for arg in raw_args {
        if let Some((key, value)) = arg.split_once(':') {
            named_args.insert(key.to_string(), value.to_string());
        } else {
            passthrough_args.push(arg.clone());
        }
    }

    ParsedArgs { named_args, passthrough_args }
}

/// Resolves a command string template with user-provided arguments.
///
/// 1. Substitutes `{{PLACEHOLDER}}` with named arguments (`KEY:VALUE`).
/// 2. Appends any remaining passthrough arguments to the end.
/// 3. Fails if any placeholders are left unresolved.
pub fn resolve_arguments(command_template: &str, raw_args: &[String]) -> Result<String> {
    let args = parse_arguments(raw_args);
    let mut resolved_command = command_template.to_string();

    // Substitute named arguments
    for (key, value) in args.named_args {
        resolved_command = resolved_command.replace(&format!("{{{{{}}}}}", key), &value);
    }

    // Check for any remaining, unresolved placeholders
    let re = Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}")?;
    if let Some(unresolved) = re.find(&resolved_command) {
        bail!("The following placeholder was not provided: {}", unresolved.as_str());
    }

    // Append passthrough arguments
    if !args.passthrough_args.is_empty() {
        resolved_command.push(' ');
        resolved_command.push_str(&args.passthrough_args.join(" "));
    }

    Ok(resolved_command)
}
