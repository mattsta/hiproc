//! `hp` is a command-line tool to save, recall, and organize your frequently used commands.
use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use comfy_table::Table;
use exec;
use skim::prelude::*;
use std::env;
use std::io::{self, Cursor};

pub mod api;
#[cfg(test)]
mod api_integration_test;
mod config;
mod history;
mod secrets;
mod templating;
#[cfg(test)]
mod templating_test;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(subcommand_required = true, arg_required_else_help = true)]
#[clap(
    after_help = "QUICK WORKFLOWS:\n  hp save \"command\"        Save command with auto-detected name/namespace\n  hp save \"command\" name   Save command with custom name, auto-detect namespace\n  hp do \"command\"          Execute and save command in one step (alias: hp x)\n  hp quick-save name       Save last shell command with custom name\n\nDIRECT EXECUTION:\n  hp <id>                  Execute stored command by ID\n  hp <namespace> <name>    Execute stored command by namespace and name\n\nExamples:\n  hp save \"cargo build\"             # Saves as 'cargo' in current project namespace\n  hp save \"ls -la\" list             # Saves as 'list' with auto-detected namespace\n  hp do git status                  # Executes and saves 'git status' as 'git/status'\n  hp 123                            # Run stored command ID 123\n  hp rust build                     # Run 'build' command from 'rust' namespace"
)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactively find and execute a command
    Find {
        #[clap(long)]
        scope: Option<String>,
        #[clap(long)]
        user: Option<String>,
    },
    /// Save a new command with smart defaults
    Save {
        /// The command string to save
        command_string: String,
        /// Optional name for the command (auto-detected from command if not provided)
        name: Option<String>,
        /// Optional namespace for the command (auto-detected from context if not provided)
        #[clap(long)]
        namespace: Option<String>,
        #[clap(long, default_value = "personal")]
        scope: String,
    },
    /// Search for commands
    Search {
        query: String,
        #[clap(long)]
        namespace: Option<String>,
        #[clap(long)]
        scope: Option<String>,
        #[clap(long)]
        user: Option<String>,
    },
    /// List all namespaces
    Namespaces,
    /// List user's commands with IDs
    List {
        #[clap(long)]
        namespace: Option<String>,
        #[clap(long)]
        scope: Option<String>,
    },
    /// Show detailed info about a command by ID
    Info { command_id: i32 },
    /// Show commands relevant to current directory and context
    Here {
        #[clap(long)]
        similar: bool,
        #[clap(long)]
        project: bool,
    },
    /// Get intelligent command suggestions based on context
    Suggest {
        #[clap(long)]
        limit: Option<i32>,
        #[clap(long)]
        project_type: Option<String>,
    },
    /// Show commands similar to a given command ID
    Similar {
        command_id: i32,
        #[clap(long, default_value = "5")]
        limit: i32,
    },
    /// Show execution analytics and insights
    Analytics {
        #[clap(long)]
        days: Option<i32>,
    },
    /// Rename a command by ID
    Rename {
        command_id: i32,
        new_namespace: String,
        new_name: String,
    },
    /// Delete a command by ID
    Delete { command_id: i32 },
    /// Edit a command by ID
    Edit { command_id: i32 },
    /// Generate shell completion scripts
    GenerateCompletions { shell: Shell },
    /// Execute a command by ID with optional arguments (also: hp <id>)
    Exec {
        command_id: i32,
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Execute a command by name with smart contextual matching
    Run {
        name: String,
        #[clap(long)]
        namespace: Option<String>,
        #[clap(long)]
        scope: Option<String>,
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Quick-save the last executed shell command
    QuickSave {
        name: String,
        #[clap(long)]
        namespace: Option<String>,
    },
    /// Execute and save a command with smart defaults
    #[clap(alias = "x")]
    Do {
        /// The command to execute and save
        #[clap(required = true, num_args = 1..)]
        command_parts: Vec<String>,
        /// Optional name for the command (auto-detected if not provided)
        #[clap(long)]
        name: Option<String>,
        /// Optional namespace for the command (auto-detected if not provided)
        #[clap(long)]
        namespace: Option<String>,
    },
    /// Execute a shell command and save it (legacy, use run-save instead)
    #[clap(hide = true)]
    Shell {
        #[clap(required = true, num_args = 1..)]
        command_parts: Vec<String>,
    },

    /// Recall and execute a command by namespace and name, or execute by ID.
    #[clap(external_subcommand)]
    Recall(Vec<String>),
}

/// Auto-detect a command name from the command string.
///
/// This function extracts a reasonable name from a command string by taking the first word
/// and cleaning it up.
fn detect_name_from_command(command_string: &str) -> String {
    // Split the command and take the first word (the main command)
    let first_word = command_string
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("command");

    // Remove common path prefixes and get just the command name
    let command_name = if first_word.contains('/') {
        // Take the last component of a path
        first_word.split('/').last().unwrap_or(first_word)
    } else {
        first_word
    };

    // Clean up the name (remove extensions, special chars, etc.)
    let clean_name = command_name
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();

    if clean_name.is_empty() {
        "command".to_string()
    } else {
        clean_name
    }
}

/// Detect namespace from current directory context.
///
/// This function analyzes the current directory to suggest a namespace for the command.
fn detect_namespace_from_context() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;

    // Check for common project indicators and use directory name with prefix
    if cwd.join("package.json").exists() {
        return cwd
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string());
    }
    if cwd.join("Cargo.toml").exists() {
        return cwd
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string());
    }
    if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() {
        return cwd
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string());
    }
    if cwd.join(".git").exists() {
        return cwd
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string());
    }

    // Fallback to directory name
    cwd.file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// Execute a command with history tracking.
///
/// This function handles command execution and creates execution history records
/// for analytics and tracking purposes.
async fn execute_command_with_tracking(
    api_client: &api::ApiClient,
    command: &api::Command,
    user: &str,
    hostname: &str,
    cwd: &str,
    resolved_command: &str,
    execution_method: &str,
    args: &[String],
) -> Result<()> {
    use std::process::{Command, Stdio};
    use std::time::Instant;

    println!("Executing command {}: {}", command.id, resolved_command);

    let start_time = Instant::now();

    // Execute the command and wait for completion to get timing and exit code
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(resolved_command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn command")?;

    let exit_status = child.wait().context("Failed to wait for command")?;
    let duration = start_time.elapsed();

    let exit_code = exit_status.code().unwrap_or(-1);

    // Create execution history record with timing and exit code data
    let execution_record = api::ExecutionHistoryCreate {
        command_id: command.id,
        user: Some(user.to_string()),
        hostname: Some(hostname.to_string()),
        cwd: Some(cwd.to_string()),
        arguments: if args.is_empty() {
            None
        } else {
            Some(args.join(" "))
        },
        execution_method: execution_method.to_string(),
        duration_ms: Some(duration.as_millis() as i32),
        exit_code: Some(exit_code),
    };

    // Track execution in background - don't fail if this fails
    if let Err(e) = api_client.create_execution_record(execution_record).await {
        eprintln!("Warning: Failed to track execution: {}", e);
    }

    // Exit with the same code as the executed command
    if !exit_status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn format_find_output(commands: &[api::Command]) -> String {
    commands
        .iter()
        .map(|cmd| {
            format!(
                "ID:{:<4} {:<15} {:<15} {:<10} {:<10} {:<25} :: {}",
                cmd.id,
                cmd.namespace,
                cmd.name,
                cmd.user.as_deref().unwrap_or(""),
                cmd.scope,
                cmd.cwd.as_deref().unwrap_or(""),
                cmd.command_string
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // The completions command is a special case that doesn't need the API client.
    if let Commands::GenerateCompletions { shell } = cli.command {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }

    let settings = config::Settings::new().context("Failed to read configuration")?;
    let api_client = api::ApiClient::new(settings.server_url);
    let user = whoami::username();

    match cli.command {
        Commands::Find {
            scope,
            user: user_filter,
        } => {
            let commands = api_client
                .get_commands("", None, user_filter.as_deref(), scope.as_deref())
                .await?;
            if commands.is_empty() {
                println!("No commands found.");
                return Ok(());
            }

            let item_text = format_find_output(&commands);

            let options = SkimOptionsBuilder::default()
                .height(Some("50%"))
                .multi(false)
                .header(Some("ID    Namespace        Name             User       Scope      Directory                 :: Command"))
                .build()
                .unwrap();

            let item_reader = SkimItemReader::default();
            let items = item_reader.of_bufread(Cursor::new(item_text));

            let selected_items = Skim::run_with(&options, Some(items))
                .map(|out| out.selected_items)
                .unwrap_or_else(Vec::new);

            if let Some(item) = selected_items.first() {
                let output = item.output();
                let parts: Vec<&str> = output.split("::").collect();
                if let (Some(id_part), Some(command_string)) = (parts.get(0), parts.get(1)) {
                    // Extract command ID from the "ID:123" format
                    if let Some(id_str) = id_part.strip_prefix("ID:") {
                        if let Ok(command_id) = id_str.trim().parse::<i32>() {
                            // Fetch the command to get full details for tracking
                            if let Ok(command) = api_client.execute_command(command_id, &user).await
                            {
                                let resolved_command =
                                    secrets::resolve_secrets(command_string.trim())?;

                                let cwd = env::current_dir()?
                                    .to_str()
                                    .context("Invalid CWD")?
                                    .to_string();
                                let hostname = gethostname::gethostname()
                                    .to_str()
                                    .context("Invalid hostname")?
                                    .to_string();

                                execute_command_with_tracking(
                                    &api_client,
                                    &command,
                                    &user,
                                    &hostname,
                                    &cwd,
                                    &resolved_command,
                                    "find",
                                    &[],
                                )
                                .await?;
                                return Ok(());
                            }
                        }
                    }

                    // Fallback to simple execution if tracking fails
                    let resolved_command = secrets::resolve_secrets(command_string.trim())?;
                    let err = exec::Command::new("sh")
                        .arg("-c")
                        .arg(resolved_command)
                        .exec();
                    anyhow::bail!("Failed to execute command: {}", err);
                }
            }
        }
        Commands::Save {
            command_string,
            name,
            namespace,
            scope,
        } => {
            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            // Auto-detect name if not provided
            let final_name = name.unwrap_or_else(|| detect_name_from_command(&command_string));

            // Auto-detect namespace if not provided
            let final_namespace = namespace.unwrap_or_else(|| {
                detect_namespace_from_context().unwrap_or_else(|| "ad-hoc".to_string())
            });

            let new_command = api::NewCommand {
                command_string: command_string.clone(),
                name: final_name.clone(),
                namespace: final_namespace.clone(),
                user: Some(user),
                cwd: Some(cwd),
                hostname: Some(hostname),
                scope,
            };
            let cmd = api_client
                .save_command(new_command)
                .await
                .context("Failed to save command to server")?;

            if cmd.is_new {
                println!(
                    "✓ Saved '{}' as '{}/{}' (ID: {})",
                    command_string, final_namespace, final_name, cmd.id
                );
            } else {
                println!(
                    "Command already exists as '{}/{}' (ID: {})",
                    final_namespace, final_name, cmd.id
                );
            }
        }
        Commands::Search {
            query,
            namespace,
            scope,
            user,
        } => {
            let commands = api_client
                .get_commands(
                    &query,
                    namespace.as_deref(),
                    user.as_deref(),
                    scope.as_deref(),
                )
                .await
                .context("Failed to search for commands on server")?;
            let mut table = Table::new();
            table.set_header(vec![
                "ID",
                "Namespace",
                "Name",
                "User",
                "Hostname",
                "Scope",
                "Directory",
                "Created",
                "Last Used",
                "Uses",
                "Command",
            ]);
            for cmd in commands {
                table.add_row(vec![
                    cmd.id.to_string(),
                    cmd.namespace,
                    cmd.name,
                    cmd.user.unwrap_or_default(),
                    cmd.hostname.unwrap_or_default(),
                    cmd.scope,
                    cmd.cwd.unwrap_or_default(),
                    cmd.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    cmd.last_used_at
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                    cmd.use_count.to_string(),
                    cmd.command_string,
                ]);
            }
            println!("{table}");
        }
        Commands::Namespaces => {
            let namespaces = api_client
                .get_namespaces()
                .await
                .context("Failed to get namespaces from server")?;
            if namespaces.is_empty() {
                println!("No namespaces found.");
            } else {
                let mut table = Table::new();
                table.set_header(vec!["Namespace"]);
                for ns in namespaces {
                    table.add_row(vec![ns]);
                }
                println!("{table}");
            }
        }
        Commands::List { namespace, scope } => {
            let commands = api_client
                .get_all_user_commands(&user)
                .await
                .context("Failed to get commands from server")?;

            let filtered_commands: Vec<_> = commands
                .iter()
                .filter(|cmd| {
                    let namespace_match =
                        namespace.as_ref().map_or(true, |ns| &cmd.namespace == ns);
                    let scope_match = scope.as_ref().map_or(true, |sc| &cmd.scope == sc);
                    namespace_match && scope_match
                })
                .collect();

            if filtered_commands.is_empty() {
                println!("No commands found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.set_header(vec![
                "ID",
                "Namespace",
                "Name",
                "Hostname",
                "Scope",
                "Directory",
                "Created",
                "Last Used",
                "Uses",
                "Command",
            ]);
            for cmd in filtered_commands {
                table.add_row(vec![
                    cmd.id.to_string(),
                    cmd.namespace.clone(),
                    cmd.name.clone(),
                    cmd.hostname.as_deref().unwrap_or("").to_string(),
                    cmd.scope.clone(),
                    cmd.cwd.as_deref().unwrap_or("").to_string(),
                    cmd.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    cmd.last_used_at
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                    cmd.use_count.to_string(),
                    cmd.command_string.clone(),
                ]);
            }
            println!("{table}");
        }
        Commands::Info { command_id } => {
            let commands = api_client.get_all_user_commands(&user).await?;
            let command = commands
                .iter()
                .find(|cmd| cmd.id == command_id)
                .context("Command with specified ID not found or you don't have permission")?;

            let mut table = Table::new();
            table.set_header(vec!["Property", "Value"]);
            table.add_row(vec!["Command ID", &command.id.to_string()]);
            table.add_row(vec!["Name", &command.name]);
            table.add_row(vec!["Namespace", &command.namespace]);
            table.add_row(vec!["Scope", &command.scope]);
            table.add_row(vec!["User", command.user.as_deref().unwrap_or("N/A")]);
            table.add_row(vec![
                "Hostname",
                command.hostname.as_deref().unwrap_or("N/A"),
            ]);
            table.add_row(vec!["Directory", command.cwd.as_deref().unwrap_or("N/A")]);
            table.add_row(vec![
                "Created",
                &command.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]);
            table.add_row(vec![
                "Last Used",
                &command
                    .last_used_at
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or("Never".to_string()),
            ]);
            table.add_row(vec!["Use Count", &command.use_count.to_string()]);
            table.add_row(vec!["Command", &command.command_string]);
            println!("{table}");
        }
        Commands::Here { similar, project } => {
            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            if project {
                let project_request = api::ProjectContextRequest {
                    directory_path: cwd.clone(),
                    user: Some(user.clone()),
                };
                let project_context = api_client
                    .detect_project_context(project_request)
                    .await
                    .context("Failed to detect project context")?;

                println!("Project Context for {}", cwd);
                if let Some(namespace) = &project_context.detected_namespace {
                    println!("  Detected Namespace: {}", namespace);
                }
                if let Some(project_type) = &project_context.project_type {
                    println!("  Project Type: {}", project_type);
                }
                println!("  Confidence Score: {}", project_context.confidence_score);
                if !project_context.similar_commands.is_empty() {
                    println!(
                        "  Similar Commands: {}",
                        project_context.similar_commands.join(", ")
                    );
                }
            } else {
                let suggestions_request = api::SuggestionsRequest {
                    user: Some(user.clone()),
                    hostname: Some(hostname),
                    cwd: Some(cwd.clone()),
                    project_type: None,
                    limit: if similar { 10 } else { 5 },
                };
                let suggestions = api_client
                    .get_suggestions(suggestions_request)
                    .await
                    .context("Failed to get context suggestions")?;

                if suggestions.is_empty() {
                    println!(
                        "No contextual suggestions found for current directory: {}",
                        cwd
                    );
                } else {
                    println!("Commands relevant to current context ({}):", cwd);
                    let mut table = Table::new();
                    table.set_header(vec!["ID", "Name", "Uses", "Last Used", "Command"]);
                    for cmd in suggestions {
                        let last_used = cmd
                            .last_used_at
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or("Never".to_string());
                        table.add_row(vec![
                            cmd.id.to_string(),
                            cmd.name,
                            cmd.use_count.to_string(),
                            last_used,
                            cmd.command_string,
                        ]);
                    }
                    println!("{table}");
                }
            }
        }
        Commands::Suggest {
            limit,
            project_type,
        } => {
            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            let suggestions_request = api::SuggestionsRequest {
                user: Some(user.clone()),
                hostname: Some(hostname),
                cwd: Some(cwd),
                project_type,
                limit: limit.unwrap_or(5),
            };
            let suggestions = api_client
                .get_suggestions(suggestions_request)
                .await
                .context("Failed to get intelligent suggestions")?;

            if suggestions.is_empty() {
                println!("No intelligent suggestions available for your current context.");
                println!("Try running commands and saving them to build up your command history.");
            } else {
                println!("Intelligent command suggestions based on your context:");
                let mut table = Table::new();
                table.set_header(vec![
                    "ID",
                    "Namespace",
                    "Name",
                    "Uses",
                    "Last Used",
                    "Command",
                ]);
                for cmd in suggestions {
                    let last_used = cmd
                        .last_used_at
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or("Never".to_string());
                    table.add_row(vec![
                        cmd.id.to_string(),
                        cmd.namespace,
                        cmd.name,
                        cmd.use_count.to_string(),
                        last_used,
                        cmd.command_string,
                    ]);
                }
                println!("{table}");
            }
        }
        Commands::Similar { command_id, limit } => {
            let similar_commands = api_client
                .get_similar_commands(command_id, Some(limit))
                .await
                .context("Failed to get similar commands")?;

            if similar_commands.is_empty() {
                println!("No similar commands found for command ID {}.", command_id);
            } else {
                println!("Commands similar to ID {}:", command_id);
                let mut table = Table::new();
                table.set_header(vec![
                    "ID",
                    "Namespace",
                    "Name",
                    "Uses",
                    "Last Used",
                    "Command",
                ]);
                for cmd in similar_commands {
                    let last_used = cmd
                        .last_used_at
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or("Never".to_string());
                    table.add_row(vec![
                        cmd.id.to_string(),
                        cmd.namespace,
                        cmd.name,
                        cmd.use_count.to_string(),
                        last_used,
                        cmd.command_string,
                    ]);
                }
                println!("{table}");
            }
        }
        Commands::Analytics { days } => {
            let analytics = api_client
                .get_execution_analytics(Some(&user), days)
                .await
                .context("Failed to get execution analytics")?;

            println!("Execution Analytics for user: {}", user);
            if let Some(days_val) = days {
                println!("Period: Last {} days", days_val);
            } else {
                println!("Period: Last 30 days (default)");
            }
            println!();

            if let Some(total_executions) = analytics.get("total_executions") {
                println!("Total Executions: {}", total_executions);
            }
            if let Some(unique_commands) = analytics.get("unique_commands") {
                println!("Unique Commands: {}", unique_commands);
            }
            if let Some(avg_per_day) = analytics.get("average_executions_per_day") {
                println!(
                    "Average per Day: {:.1}",
                    avg_per_day.as_f64().unwrap_or(0.0)
                );
            }

            if let Some(top_commands) = analytics
                .get("most_used_commands")
                .and_then(|v| v.as_array())
            {
                println!("\nMost Used Commands:");
                let mut table = Table::new();
                table.set_header(vec!["Rank", "Namespace", "Name", "Executions"]);
                for (i, cmd_data) in top_commands.iter().enumerate() {
                    if let (Some(name), Some(namespace), Some(count)) = (
                        cmd_data.get("name").and_then(|v| v.as_str()),
                        cmd_data.get("namespace").and_then(|v| v.as_str()),
                        cmd_data.get("execution_count").and_then(|v| v.as_u64()),
                    ) {
                        table.add_row(vec![
                            (i + 1).to_string(),
                            namespace.to_string(),
                            name.to_string(),
                            count.to_string(),
                        ]);
                    }
                }
                println!("{table}");
            }

            if let Some(methods) = analytics
                .get("execution_methods")
                .and_then(|v| v.as_object())
            {
                println!("\nExecution Methods:");
                let mut table = Table::new();
                table.set_header(vec!["Method", "Count"]);
                for (method, count) in methods {
                    table.add_row(vec![
                        method.to_string(),
                        count.as_u64().unwrap_or(0).to_string(),
                    ]);
                }
                println!("{table}");
            }
        }
        Commands::Rename {
            command_id,
            new_namespace,
            new_name,
        } => {
            let rename_payload = api::CommandRename {
                name: new_name,
                namespace: new_namespace,
            };
            api_client
                .rename_command(command_id, &user, rename_payload)
                .await
                .context("Failed to rename command on server")?;
            println!("Renamed command {}", command_id);
        }
        Commands::Edit { command_id } => {
            // First, get the command to get its current content
            let commands = api_client.get_all_user_commands(&user).await?;
            let command_to_edit = commands
                .iter()
                .find(|cmd| cmd.id == command_id)
                .context("Command with specified ID not found or you don't have permission")?;

            // Open the command string in the user's editor
            let edited_command_string = edit::edit(command_to_edit.command_string.clone())?;

            // Now, update the command by ID
            let update_payload = api::CommandUpdate {
                command_string: edited_command_string.trim().to_string(),
            };
            api_client
                .update_command(command_id, &user, update_payload)
                .await
                .context("Failed to update command on server")?;

            println!("Updated command with ID {}", command_id);
        }
        Commands::Delete { command_id } => {
            // Delete the command by ID
            let deleted_command = api_client.delete_command(command_id, &user).await.context(
                "Failed to delete command - command not found or you don't have permission",
            )?;

            println!(
                "Deleted command with ID {} ('{}' from namespace '{}')",
                command_id, deleted_command.name, deleted_command.namespace
            );
        }
        Commands::Exec { command_id, args } => {
            // Execute command directly by ID
            let command = api_client
                .execute_command(command_id, &user)
                .await
                .context("Failed to execute command")?;

            let templated_command = templating::resolve_arguments(&command.command_string, &args)?;
            let resolved_command = secrets::resolve_secrets(&templated_command)?;

            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            execute_command_with_tracking(
                &api_client,
                &command,
                &user,
                &hostname,
                &cwd,
                &resolved_command,
                "id",
                &args,
            )
            .await?;
        }
        Commands::Run {
            name,
            namespace,
            scope,
            args,
        } => {
            // Execute command by name with smart contextual matching
            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            let request = api::RecallByNameRequest {
                name: name.clone(),
                user: Some(user.clone()),
                hostname: Some(hostname.clone()),
                cwd: Some(cwd.clone()),
                namespace_hint: namespace,
                scope_hint: scope,
            };

            let command = api_client
                .recall_command_by_name(request)
                .await
                .context("Failed to find command")?;

            let templated_command = templating::resolve_arguments(&command.command_string, &args)?;
            let resolved_command = secrets::resolve_secrets(&templated_command)?;

            execute_command_with_tracking(
                &api_client,
                &command,
                &user,
                &hostname,
                &cwd,
                &resolved_command,
                "name",
                &args,
            )
            .await?;
        }
        Commands::QuickSave { name, namespace } => {
            // Get the last command from shell history
            let history_manager = history::HistoryManager::new();

            if !history_manager.is_available() {
                eprintln!(
                    "Shell history integration not available: {}",
                    history_manager.status_description()
                );
                println!(
                    "To save a command manually, use: hp save \"<command>\" --name {} --namespace {}",
                    name,
                    namespace.as_deref().unwrap_or("<namespace>")
                );
                return Ok(());
            }

            match history_manager.get_last_command()? {
                Some(command_string) => {
                    // Auto-detect namespace if not provided
                    let detected_namespace = namespace.unwrap_or_else(|| {
                        // Try to detect namespace from current directory
                        detect_namespace_from_context().unwrap_or_else(|| "quick-save".to_string())
                    });

                    let cwd = env::current_dir()?
                        .to_str()
                        .context("Invalid CWD")?
                        .to_string();
                    let hostname = gethostname::gethostname()
                        .to_str()
                        .context("Invalid hostname")?
                        .to_string();

                    let new_command = api::NewCommand {
                        command_string: command_string.clone(),
                        name: name.clone(),
                        namespace: detected_namespace.clone(),
                        user: Some(user),
                        cwd: Some(cwd),
                        hostname: Some(hostname),
                        scope: "personal".to_string(),
                    };

                    let cmd = api_client
                        .save_command(new_command)
                        .await
                        .context("Failed to save command to server")?;

                    if cmd.is_new {
                        println!(
                            "✓ Quick-saved '{}' as '{}' in namespace '{}'",
                            command_string, name, detected_namespace
                        );
                        println!("  Command ID: {}", cmd.id);
                    } else {
                        println!("Command already exists with ID {}", cmd.id);
                    }
                }
                None => {
                    println!("No recent command found in shell history.");
                    println!(
                        "To save a command manually, use: hp save \"<command>\" --name {} --namespace {}",
                        name,
                        namespace.as_deref().unwrap_or("<namespace>")
                    );
                }
            }
        }
        Commands::Do {
            command_parts,
            name,
            namespace,
        } => {
            let command_string = shlex::try_join(command_parts.iter().map(|s| s.as_str()))
                .context("Failed to join command parts")?;

            // Use provided name or auto-detect
            let final_name = name.unwrap_or_else(|| detect_name_from_command(&command_string));

            // Use provided namespace or auto-detect
            let final_namespace = namespace.unwrap_or_else(|| {
                detect_namespace_from_context().unwrap_or_else(|| "ad-hoc".to_string())
            });

            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();
            let new_command = api::NewCommand {
                command_string: command_string.clone(),
                name: final_name.clone(),
                namespace: final_namespace.clone(),
                user: Some(user.clone()),
                cwd: Some(cwd.clone()),
                hostname: Some(hostname.clone()),
                scope: "personal".to_string(),
            };
            let saved_command = api_client
                .save_command(new_command)
                .await
                .context("Failed to save command to server")?;

            println!(
                "✓ Saved and executing '{}' as '{}/{}' (ID: {})",
                command_string, final_namespace, final_name, saved_command.id
            );

            // Execute using our tracking function
            execute_command_with_tracking(
                &api_client,
                &saved_command,
                &user,
                &hostname,
                &cwd,
                &command_string,
                "do",
                &[],
            )
            .await?;
        }
        Commands::Shell { command_parts } => {
            let command_string = shlex::try_join(command_parts.iter().map(|s| s.as_str()))
                .context("Failed to join command parts")?;

            // Use smart name detection instead of just first word
            let name = detect_name_from_command(&command_string);

            // Use smart namespace detection instead of hardcoded "ad-hoc"
            let namespace = detect_namespace_from_context().unwrap_or_else(|| "ad-hoc".to_string());

            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();
            let new_command = api::NewCommand {
                command_string: command_string.clone(),
                name: name.clone(),
                namespace: namespace.clone(),
                user: Some(user.clone()),
                cwd: Some(cwd.clone()),
                hostname: Some(hostname.clone()),
                scope: "personal".to_string(),
            };
            let saved_command = api_client
                .save_command(new_command)
                .await
                .context("Failed to save command to server")?;

            println!(
                "✓ Saved and executing '{}' as '{}/{}' (ID: {})",
                command_string, namespace, name, saved_command.id
            );

            // Execute using our tracking function
            execute_command_with_tracking(
                &api_client,
                &saved_command,
                &user,
                &hostname,
                &cwd,
                &command_string,
                "shell",
                &[],
            )
            .await?;
        }
        Commands::Recall(args) => {
            if args.is_empty() {
                anyhow::bail!("Error: requires at least one argument: <id> or <namespace> <name>");
            }

            // Check if first argument is a numeric ID
            if args.len() == 1 || args[0].parse::<i32>().is_ok() {
                if let Ok(command_id) = args[0].parse::<i32>() {
                    // Direct execution by ID
                    let extra_args = &args[1..].to_vec();
                    let command = api_client
                        .execute_command(command_id, &user)
                        .await
                        .context("Failed to execute command by ID")?;

                    let templated_command =
                        templating::resolve_arguments(&command.command_string, extra_args)?;
                    let resolved_command = secrets::resolve_secrets(&templated_command)?;

                    let cwd = env::current_dir()?
                        .to_str()
                        .context("Invalid CWD")?
                        .to_string();
                    let hostname = gethostname::gethostname()
                        .to_str()
                        .context("Invalid hostname")?
                        .to_string();

                    execute_command_with_tracking(
                        &api_client,
                        &command,
                        &user,
                        &hostname,
                        &cwd,
                        &resolved_command,
                        "id",
                        extra_args,
                    )
                    .await?;

                    return Ok(()); // Exit after successful ID execution
                }
            }

            // Traditional namespace/name execution
            if args.len() < 2 {
                anyhow::bail!("Error: recall requires at least two arguments: <namespace> <name>");
            }
            let namespace = &args[0];
            let name = &args[1];
            let extra_args = &args[2..].to_vec();
            let cwd = env::current_dir()?
                .to_str()
                .context("Invalid CWD")?
                .to_string();
            let hostname = gethostname::gethostname()
                .to_str()
                .context("Invalid hostname")?
                .to_string();

            let command = api_client
                .recall_command(namespace, name, &user, &hostname, &cwd)
                .await
                .context("Failed to recall command from server")?;

            let templated_command =
                templating::resolve_arguments(&command.command_string, extra_args)?;
            let resolved_command = secrets::resolve_secrets(&templated_command)?;

            execute_command_with_tracking(
                &api_client,
                &command,
                &user,
                &hostname,
                &cwd,
                &resolved_command,
                "namespace_name",
                extra_args,
            )
            .await?;
        }
        // This case is now handled before the API client is initialized.
        Commands::GenerateCompletions { .. } => unreachable!(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_detect_name_from_command() {
        // Test basic command
        assert_eq!(detect_name_from_command("ls -la"), "ls");

        // Test path-based command
        assert_eq!(detect_name_from_command("/usr/bin/git status"), "git");
        assert_eq!(detect_name_from_command("./scripts/build.sh"), "build");

        // Test complex command
        assert_eq!(detect_name_from_command("cargo build --release"), "cargo");
        assert_eq!(detect_name_from_command("npm run dev"), "npm");

        // Test command with special characters
        assert_eq!(
            detect_name_from_command("python3.11 -m pip install uv"),
            "python3"
        );

        // Test empty/edge cases
        assert_eq!(detect_name_from_command(""), "command");
        assert_eq!(detect_name_from_command("   "), "command");
        assert_eq!(detect_name_from_command("@#$%"), "command");

        // Test commands with extensions
        assert_eq!(detect_name_from_command("script.py arg1 arg2"), "script");
        assert_eq!(detect_name_from_command("./test.sh"), "test");
    }

    #[test]
    fn test_format_find_output() {
        let commands = vec![api::Command {
            id: 1,
            command_string: "echo hi".to_string(),
            name: "hello".to_string(),
            namespace: "there".to_string(),
            user: Some("matt".to_string()),
            cwd: Some("/Users/matt/repos/hiproc".to_string()),
            hostname: Some("optionality.local".to_string()),
            scope: "personal".to_string(),
            created_at: Utc::now(),
            last_used_at: None,
            use_count: 0,
            is_new: false,
        }];
        let output = format_find_output(&commands);
        assert!(output.contains("ID:1"));
        assert!(output.contains("there"));
        assert!(output.contains("hello"));
        assert!(output.contains("matt"));
        assert!(output.contains("personal"));
        assert!(output.contains("/Users/matt/repos/hiproc"));
        assert!(output.contains("echo hi"));
    }

    #[test]
    fn test_namespace_detection_with_package_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package_json_path = temp_dir.path().join("package.json");
        std::fs::write(&package_json_path, "{}").unwrap();

        // Change to the temp directory for testing
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_namespace_from_context();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should return the temp directory name
        assert!(result.is_some());
        let dir_name = temp_dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(result, Some(dir_name.to_string()));
    }

    #[test]
    fn test_namespace_detection_priority() {
        // Test that namespace detection follows the correct priority order
        let temp_dir = tempfile::tempdir().unwrap();

        // Create both package.json and Cargo.toml to test priority
        let package_json_path = temp_dir.path().join("package.json");
        std::fs::write(&package_json_path, "{}").unwrap();

        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml_path, "[package]\nname = \"rust-project\"\n").unwrap();

        // Change to the temp directory for testing
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_namespace_from_context();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should pick directory name (package.json has higher priority)
        let dir_name = temp_dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(result, Some(dir_name.to_string()));
    }
}
