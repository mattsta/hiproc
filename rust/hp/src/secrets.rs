//! Handles the detection and substitution of secrets in command strings.
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::env;

/// Finds all placeholders (e.g., `{{SECRET_NAME}}`) in a command string,
/// resolves them, and returns the substituted command.
pub fn resolve_secrets(command_string: &str) -> Result<String> {
    let re = Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}")?;
    let mut resolved_secrets: HashMap<String, String> = HashMap::new();
    let mut final_command = command_string.to_string();

    for cap in re.captures_iter(command_string) {
        let placeholder = cap.get(0).unwrap().as_str();
        let secret_name = cap.get(1).unwrap().as_str();

        if let Some(secret_value) = resolved_secrets.get(secret_name) {
            final_command = final_command.replace(placeholder, secret_value);
            continue;
        }

        // 1. Try to get the secret from an environment variable
        if let Ok(secret_value) = env::var(secret_name) {
            resolved_secrets.insert(secret_name.to_string(), secret_value.clone());
            final_command = final_command.replace(placeholder, &secret_value);
            continue;
        }

        // 2. If not in env, prompt the user securely
        let secret_value =
            rpassword::prompt_password(format!("Enter value for secret '{}': ", secret_name))
                .context("Failed to read secret from prompt")?;
        resolved_secrets.insert(secret_name.to_string(), secret_value.clone());
        final_command = final_command.replace(placeholder, &secret_value);
    }

    Ok(final_command)
}
