//! Handles loading configuration for the `hp` client.
//!
//! This module defines the `Settings` struct and the logic for loading it.
//! Configuration is loaded from three locations (in order of precedence):
//! 1. Global file in the user's config directory: ~/.config/hiproc/config.toml
//! 2. File in the same directory as the binary: <binary_dir>/hiproc.toml
//! 3. Local file in the current directory: ./hiproc.toml (highest precedence)
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::env;

/// Contains all configuration settings for the client.
#[derive(Debug, Deserialize)]
pub struct Settings {
    /// The URL of the `hiproc` server.
    pub server_url: String,
}

impl Settings {
    /// Creates a new `Settings` struct by loading configuration from files.
    /// 
    /// Configuration is loaded in order of precedence:
    /// 1. Global config: ~/.config/hiproc/config.toml
    /// 2. Binary-adjacent config: <binary_dir>/hiproc.toml
    /// 3. Local config: ./hiproc.toml (highest precedence)
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // 1. Add global config file from user's config directory
        if let Some(mut config_path) = home::home_dir() {
            config_path.push(".config");
            config_path.push("hiproc");
            config_path.push("config.toml");
            builder = builder.add_source(File::from(config_path).required(false));
        }

        // 2. Add config file from the same directory as the binary
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let mut binary_config_path = exe_dir.to_path_buf();
                binary_config_path.push("hiproc.toml");
                builder = builder.add_source(File::from(binary_config_path).required(false));
            }
        }

        // 3. Add local config file (highest precedence)
        builder = builder.add_source(File::with_name("hiproc.toml").required(false));

        let s = builder.build()?;
        s.try_deserialize()
    }
}
