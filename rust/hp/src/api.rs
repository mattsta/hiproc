use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;

/// The main client for making API calls.
pub struct ApiClient {
    base_url: String,
}

#[derive(Serialize)]
struct RecallRequest<'a> {
    name: &'a str,
    namespace: &'a str,
    user: &'a str,
    hostname: &'a str,
    cwd: &'a str,
}

impl ApiClient {
    /// Creates a new `ApiClient`.
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Saves a new command to the server.
    pub async fn save_command(&self, new_command: NewCommand) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/commands/", self.base_url))
            .json(&new_command)
            .send()
            .await?;
        Ok(res.error_for_status()?.json().await?)
    }

    /// Searches for commands on the server.
    pub async fn get_commands(&self, query: &str, namespace: Option<&str>, user: Option<&str>, scope: Option<&str>) -> Result<Vec<Command>> {
        let client = reqwest::Client::new();
        let mut query_params = vec![("q", query)];
        if let Some(ns) = namespace {
            query_params.push(("namespace", ns));
        }
        if let Some(u) = user {
            query_params.push(("user", u));
        }
        if let Some(s) = scope {
            query_params.push(("scope", s));
        }

        let res = client
            .get(format!("{}/commands/", self.base_url))
            .query(&query_params)
            .send()
            .await?;
        Ok(res.error_for_status()?.json().await?)
    }

    /// Recalls a command from the server.
    pub async fn recall_command(&self, namespace: &str, name: &str, user: &str, hostname: &str, cwd: &str) -> Result<Command> {
        let client = reqwest::Client::new();
        let recall_request = RecallRequest {
            name,
            namespace,
            user,
            hostname,
            cwd,
        };
        let res = client
            .post(format!("{}/commands/recall", self.base_url))
            .json(&recall_request)
            .send()
            .await?;

        if res.status() == 404 {
            bail!("Command '{}' in namespace '{}' not found for the current context.", name, namespace);
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Gets a list of all namespaces from the server.
    pub async fn get_namespaces(&self) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/namespaces/", self.base_url))
            .send()
            .await?;
        Ok(res.error_for_status()?.json().await?)
    }

    /// Deletes a command from the server.
    pub async fn delete_command(&self, command_id: i32, user: &str) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .delete(format!("{}/commands/{}", self.base_url, command_id))
            .query(&[("user", user)])
            .send()
            .await?;
        
        if res.status() == 404 {
            bail!("Command not found, or you don't have permission to delete it.");
        }

        Ok(res.error_for_status()?.json().await?)
    }
    /// Updates a command on the server.
    pub async fn update_command(&self, command_id: i32, user: &str, command_update: CommandUpdate) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .put(format!("{}/commands/{}", self.base_url, command_id))
            .query(&[("user", user)])
            .json(&command_update)
            .send()
            .await?;

        if res.status() == 404 {
            bail!("Command not found, or you don't have permission to edit it.");
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Gets all commands for a user.
    pub async fn get_all_user_commands(&self, user: &str) -> Result<Vec<Command>> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/commands/all", self.base_url))
            .query(&[("user", user)])
            .send()
            .await?;
        Ok(res.error_for_status()?.json().await?)
    }

/// Renames a command on the server.
    pub async fn rename_command(&self, command_id: i32, user: &str, command_rename: CommandRename) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .patch(format!("{}/commands/{}", self.base_url, command_id))
            .query(&[("user", user)])
            .json(&command_rename)
            .send()
            .await?;

        if res.status() == 404 {
            bail!("Command not found, or you don't have permission to rename it.");
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Gets a single command by ID.
    pub async fn get_command_by_id(&self, command_id: i32, user: &str) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/commands/by-id/{}", self.base_url, command_id))
            .query(&[("user", user)])
            .send()
            .await?;

        if res.status() == 404 {
            bail!("Command with ID {} not found or you don't have permission to access it.", command_id);
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Executes a command by ID, tracking the execution.
    pub async fn execute_command(&self, command_id: i32, user: &str) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/commands/{}/execute", self.base_url, command_id))
            .query(&[("user", user)])
            .send()
            .await?;

        if res.status() == 404 {
            bail!("Command with ID {} not found or you don't have permission to execute it.", command_id);
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Recalls a command by name with enhanced contextual matching.
    pub async fn recall_command_by_name(&self, request: RecallByNameRequest) -> Result<Command> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/commands/recall-by-name", self.base_url))
            .json(&request)
            .send()
            .await?;

        if res.status() == 404 {
            bail!("No command named '{}' found in the current context.", request.name);
        }

        Ok(res.error_for_status()?.json().await?)
    }

    /// Get contextual command suggestions.
    pub async fn get_suggestions(&self, request: SuggestionsRequest) -> Result<Vec<Command>> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/suggestions", self.base_url))
            .json(&request)
            .send()
            .await?;

        Ok(res.error_for_status()?.json().await?)
    }

    /// Detect project context and get namespace suggestions.
    pub async fn detect_project_context(&self, request: ProjectContextRequest) -> Result<ProjectContextResponse> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/project-context", self.base_url))
            .json(&request)
            .send()
            .await?;

        Ok(res.error_for_status()?.json().await?)
    }

    /// Get commands similar to the specified command.
    pub async fn get_similar_commands(&self, command_id: i32, limit: Option<i32>) -> Result<Vec<Command>> {
        let client = reqwest::Client::new();
        let mut url = format!("{}/commands/{}/similar", self.base_url, command_id);
        
        if let Some(limit) = limit {
            url = format!("{}?limit={}", url, limit);
        }
        
        let res = client.get(url).send().await?;
        Ok(res.error_for_status()?.json().await?)
    }

    /// Create an execution history record for analytics.
    pub async fn create_execution_record(&self, execution: ExecutionHistoryCreate) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/execution-history", self.base_url))
            .json(&execution)
            .send()
            .await?;

        Ok(res.error_for_status()?.json().await?)
    }

    /// Get execution analytics.
    pub async fn get_execution_analytics(&self, user: Option<&str>, days: Option<i32>) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let url = format!("{}/analytics/execution", self.base_url);
        let mut params = Vec::new();
        let days_str;
        
        if let Some(user) = user {
            params.push(("user", user));
        }
        if let Some(days) = days {
            days_str = days.to_string();
            params.push(("days", days_str.as_str()));
        }
        
        let res = client.get(url).query(&params).send().await?;
        Ok(res.error_for_status()?.json().await?)
    }
}

/// Represents a command returned from the server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    pub id: i32,
    pub command_string: String,
    pub name: String,
    pub namespace: String,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub hostname: Option<String>,
    pub scope: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub use_count: i32,
    #[serde(default)]
    pub is_new: bool,
}

/// Represents a new command to be sent to the server.
#[derive(Serialize, Debug)]
pub struct NewCommand {
    pub command_string: String,
    pub name: String,
    pub namespace: String,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub hostname: Option<String>,
    pub scope: String,
}

/// Represents a command update payload.
#[derive(Serialize, Debug)]
pub struct CommandUpdate {
    pub command_string: String,
}

/// Represents a command rename payload.
#[derive(Serialize, Debug)]
pub struct CommandRename {
    pub name: String,
    pub namespace: String,
}

/// Represents a request to recall a command by name with enhanced context.
#[derive(Serialize, Debug)]
pub struct RecallByNameRequest {
    pub name: String,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub cwd: Option<String>,
    pub namespace_hint: Option<String>,
    pub scope_hint: Option<String>,
}

/// Represents a quick-save request payload.
#[derive(Serialize, Debug)]
pub struct QuickSaveRequest {
    pub command_string: String,
    pub name: String,
    pub namespace: Option<String>,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub hostname: Option<String>,
    pub scope: String,
}

/// Represents a request for contextual suggestions.
#[derive(Serialize, Debug)]
pub struct SuggestionsRequest {
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub cwd: Option<String>,
    pub project_type: Option<String>,
    pub limit: i32,
}

/// Represents a project context detection request.
#[derive(Serialize, Debug)]
pub struct ProjectContextRequest {
    pub directory_path: String,
    pub user: Option<String>,
}

/// Represents a project context response.
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectContextResponse {
    pub detected_namespace: Option<String>,
    pub project_type: Option<String>,
    pub confidence_score: i32,
    pub similar_commands: Vec<String>,
}

/// Represents an execution history record for analytics.
#[derive(Serialize, Debug)]
pub struct ExecutionHistoryCreate {
    pub command_id: i32,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub cwd: Option<String>,
    pub arguments: Option<String>,
    pub execution_method: String,
    pub duration_ms: Option<i32>,
    pub exit_code: Option<i32>,
}
