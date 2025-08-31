/*!
Shell history integration for hiproc.

This module provides functionality to interact with various shell history systems
to enable quick-save workflows and last-command detection.
*/

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Represents different shell types that we can integrate with.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Unknown,
}

impl ShellType {
    /// Detect the current shell from environment variables.
    pub fn detect() -> Self {
        if let Ok(shell) = env::var("SHELL") {
            if shell.contains("bash") {
                ShellType::Bash
            } else if shell.contains("zsh") {
                ShellType::Zsh
            } else if shell.contains("fish") {
                ShellType::Fish
            } else {
                ShellType::Unknown
            }
        } else {
            ShellType::Unknown
        }
    }
    
    /// Get the typical history file path for this shell type.
    pub fn history_file_path(&self) -> Option<PathBuf> {
        let home = env::var("HOME").ok()?;
        let home_path = PathBuf::from(home);
        
        match self {
            ShellType::Bash => Some(home_path.join(".bash_history")),
            ShellType::Zsh => {
                // Try HISTFILE first, then fallback to default
                if let Ok(histfile) = env::var("HISTFILE") {
                    Some(PathBuf::from(histfile))
                } else {
                    Some(home_path.join(".zsh_history"))
                }
            },
            ShellType::Fish => Some(home_path.join(".local/share/fish/fish_history")),
            ShellType::Unknown => None,
        }
    }
}

/// Shell history manager for reading command history.
pub struct HistoryManager {
    shell_type: ShellType,
    history_path: Option<PathBuf>,
}

impl HistoryManager {
    /// Create a new history manager for the current shell.
    pub fn new() -> Self {
        let shell_type = ShellType::detect();
        let history_path = shell_type.history_file_path();
        
        Self {
            shell_type,
            history_path,
        }
    }
    
    /// Create a history manager for a specific shell type.
    #[allow(dead_code)]  // Will be used in Phase 3 for advanced features
    pub fn for_shell(shell_type: ShellType) -> Self {
        let history_path = shell_type.history_file_path();
        Self {
            shell_type,
            history_path,
        }
    }
    
    /// Get the last executed command from shell history.
    pub fn get_last_command(&self) -> Result<Option<String>> {
        let path = self.history_path.as_ref()
            .context("No history file path available for this shell")?;
            
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(path)
            .context("Failed to read history file")?;
            
        match self.shell_type {
            ShellType::Bash => self.parse_bash_history(&content),
            ShellType::Zsh => self.parse_zsh_history(&content),
            ShellType::Fish => self.parse_fish_history(&content),
            ShellType::Unknown => Ok(None),
        }
    }
    
    /// Get the last N commands from shell history.
    #[allow(dead_code)]  // Will be used in Phase 3 for command recommendations
    pub fn get_recent_commands(&self, count: usize) -> Result<Vec<String>> {
        let path = self.history_path.as_ref()
            .context("No history file path available for this shell")?;
            
        if !path.exists() {
            return Ok(vec![]);
        }
        
        let content = fs::read_to_string(path)
            .context("Failed to read history file")?;
            
        match self.shell_type {
            ShellType::Bash => self.parse_bash_recent(&content, count),
            ShellType::Zsh => self.parse_zsh_recent(&content, count),
            ShellType::Fish => self.parse_fish_recent(&content, count),
            ShellType::Unknown => Ok(vec![]),
        }
    }
    
    /// Parse bash history format (simple line-based).
    fn parse_bash_history(&self, content: &str) -> Result<Option<String>> {
        Ok(content.lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !line.starts_with('#'))  // Skip comments
            .last()
            .map(|s| s.to_string()))
    }
    
    /// Parse recent bash commands.
    #[allow(dead_code)]
    fn parse_bash_recent(&self, content: &str, count: usize) -> Result<Vec<String>> {
        let commands: Vec<String> = content.lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !line.starts_with('#'))
            .map(|s| s.to_string())
            .collect();
            
        Ok(commands.into_iter().rev().take(count).rev().collect())
    }
    
    /// Parse zsh history format (includes timestamps).
    fn parse_zsh_history(&self, content: &str) -> Result<Option<String>> {
        // Zsh history format: : 1234567890:0;command
        let last_command = content.lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                if line.starts_with(':') && line.contains(';') {
                    // Extract command after the semicolon
                    line.splitn(2, ';').nth(1).map(|s| s.to_string())
                } else {
                    // Fallback to treating as simple command
                    Some(line.to_string())
                }
            })
            .last();
            
        Ok(last_command)
    }
    
    /// Parse recent zsh commands.
    #[allow(dead_code)]
    fn parse_zsh_recent(&self, content: &str, count: usize) -> Result<Vec<String>> {
        let commands: Vec<String> = content.lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                if line.starts_with(':') && line.contains(';') {
                    line.splitn(2, ';').nth(1).map(|s| s.to_string())
                } else {
                    Some(line.to_string())
                }
            })
            .collect();
            
        Ok(commands.into_iter().rev().take(count).rev().collect())
    }
    
    /// Parse fish history format (YAML-like).
    fn parse_fish_history(&self, content: &str) -> Result<Option<String>> {
        // Fish history format:
        // - cmd: command here
        //   when: 1234567890
        let mut last_command = None;
        
        for line in content.lines() {
            if let Some(cmd) = line.strip_prefix("- cmd: ") {
                last_command = Some(cmd.to_string());
            }
        }
        
        Ok(last_command)
    }
    
    /// Parse recent fish commands.
    #[allow(dead_code)]
    fn parse_fish_recent(&self, content: &str, count: usize) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        
        for line in content.lines() {
            if let Some(cmd) = line.strip_prefix("- cmd: ") {
                commands.push(cmd.to_string());
            }
        }
        
        Ok(commands.into_iter().rev().take(count).rev().collect())
    }
    
    /// Check if history integration is available for the current shell.
    pub fn is_available(&self) -> bool {
        self.history_path.as_ref()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
    
    /// Get a user-friendly description of the shell integration status.
    pub fn status_description(&self) -> String {
        match (&self.shell_type, &self.history_path) {
            (ShellType::Unknown, _) => "Shell type not detected or unsupported".to_string(),
            (shell_type, None) => format!("History file path not available for {:?}", shell_type),
            (shell_type, Some(path)) => {
                if path.exists() {
                    format!("Connected to {:?} history at {}", shell_type, path.display())
                } else {
                    format!("History file not found at {} ({:?})", path.display(), shell_type)
                }
            }
        }
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shell_type_detection() {
        // This test depends on the current environment, so we mainly test the parsing logic
        let shell_type = ShellType::detect();
        println!("Detected shell type: {:?}", shell_type);
    }
    
    #[test]
    fn test_bash_history_parsing() {
        let manager = HistoryManager::for_shell(ShellType::Bash);
        let content = "command1\ncommand2\n# comment\ncommand3\n";
        
        let last = manager.parse_bash_history(content).unwrap();
        assert_eq!(last, Some("command3".to_string()));
        
        let recent = manager.parse_bash_recent(content, 2).unwrap();
        assert_eq!(recent, vec!["command2", "command3"]);
    }
    
    #[test]
    fn test_zsh_history_parsing() {
        let manager = HistoryManager::for_shell(ShellType::Zsh);
        let content = ": 1234567890:0;first command\n: 1234567891:0;second command\n";
        
        let last = manager.parse_zsh_history(content).unwrap();
        assert_eq!(last, Some("second command".to_string()));
        
        let recent = manager.parse_zsh_recent(content, 2).unwrap();
        assert_eq!(recent, vec!["first command", "second command"]);
    }
    
    #[test]
    fn test_fish_history_parsing() {
        let manager = HistoryManager::for_shell(ShellType::Fish);
        let content = "- cmd: first command\n  when: 1234567890\n- cmd: second command\n  when: 1234567891\n";
        
        let last = manager.parse_fish_history(content).unwrap();
        assert_eq!(last, Some("second command".to_string()));
        
        let recent = manager.parse_fish_recent(content, 2).unwrap();
        assert_eq!(recent, vec!["first command", "second command"]);
    }
    
    #[test]
    fn test_empty_history() {
        let manager = HistoryManager::for_shell(ShellType::Bash);
        let last = manager.parse_bash_history("").unwrap();
        assert_eq!(last, None);
        
        let recent = manager.parse_bash_recent("", 5).unwrap();
        assert!(recent.is_empty());
    }
}