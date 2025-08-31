#![cfg(test)]
use super::api::{
    ApiClient, Command, ExecutionHistoryCreate, NewCommand, ProjectContextRequest,
    ProjectContextResponse, RecallByNameRequest, SuggestionsRequest,
};
use chrono::Utc;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_save_command() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let new_cmd = NewCommand {
        command_string: "test".to_string(),
        name: "test".to_string(),
        namespace: "test".to_string(),
        user: Some("testuser".to_string()),
        cwd: None,
        hostname: None,
        scope: "personal".to_string(),
    };
    let response_body = Command {
        id: 1,
        command_string: "test".to_string(),
        name: "test".to_string(),
        namespace: "test".to_string(),
        user: Some("testuser".to_string()),
        cwd: None,
        hostname: None,
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: None,
        use_count: 0,
        is_new: false,
    };

    Mock::given(method("POST"))
        .and(path("/commands/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.save_command(new_cmd).await;

    // Assert
    assert!(result.is_ok());
    let cmd = result.unwrap();
    assert_eq!(cmd.name, "test");
}

#[tokio::test]
async fn test_recall_command() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let response_body = Command {
        id: 1,
        command_string: "recalled command".to_string(),
        name: "recall_test".to_string(),
        namespace: "ns".to_string(),
        user: Some("testuser".to_string()),
        cwd: Some("/tmp".to_string()),
        hostname: Some("host".to_string()),
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: None,
        use_count: 0,
        is_new: false,
    };

    Mock::given(method("POST"))
        .and(path("/commands/recall"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client
        .recall_command("ns", "recall_test", "testuser", "host", "/tmp")
        .await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap().command_string, "recalled command");
}

#[tokio::test]
async fn test_get_suggestions() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let suggestions_request = SuggestionsRequest {
        user: Some("testuser".to_string()),
        hostname: Some("testhost".to_string()),
        cwd: Some("/test/dir".to_string()),
        project_type: Some("rust".to_string()),
        limit: 5,
    };

    let response_body = vec![Command {
        id: 1,
        command_string: "cargo build".to_string(),
        name: "build".to_string(),
        namespace: "rust".to_string(),
        user: Some("testuser".to_string()),
        cwd: Some("/test/dir".to_string()),
        hostname: Some("testhost".to_string()),
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        use_count: 5,
        is_new: false,
    }];

    Mock::given(method("POST"))
        .and(path("/suggestions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.get_suggestions(suggestions_request).await;

    // Assert
    assert!(result.is_ok());
    let suggestions = result.unwrap();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].command_string, "cargo build");
    assert_eq!(suggestions[0].use_count, 5);
}

#[tokio::test]
async fn test_detect_project_context() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let context_request = ProjectContextRequest {
        directory_path: "/test/rust/project".to_string(),
        user: Some("testuser".to_string()),
    };

    let response_body = ProjectContextResponse {
        detected_namespace: Some("rust-project".to_string()),
        project_type: Some("rust".to_string()),
        confidence_score: 95,
        similar_commands: vec!["cargo build".to_string(), "cargo test".to_string()],
    };

    Mock::given(method("POST"))
        .and(path("/project-context"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.detect_project_context(context_request).await;

    // Assert
    assert!(result.is_ok());
    let context = result.unwrap();
    assert_eq!(context.detected_namespace, Some("rust-project".to_string()));
    assert_eq!(context.project_type, Some("rust".to_string()));
    assert_eq!(context.confidence_score, 95);
    assert_eq!(context.similar_commands.len(), 2);
}

#[tokio::test]
async fn test_get_similar_commands() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let command_id = 123;
    let limit = 3;

    let response_body = vec![Command {
        id: 124,
        command_string: "cargo test".to_string(),
        name: "test".to_string(),
        namespace: "rust".to_string(),
        user: Some("testuser".to_string()),
        cwd: Some("/test/dir".to_string()),
        hostname: Some("testhost".to_string()),
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        use_count: 3,
        is_new: false,
    }];

    Mock::given(method("GET"))
        .and(path(format!("/commands/{}/similar", command_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.get_similar_commands(command_id, Some(limit)).await;

    // Assert
    assert!(result.is_ok());
    let similar_commands = result.unwrap();
    assert_eq!(similar_commands.len(), 1);
    assert_eq!(similar_commands[0].command_string, "cargo test");
}

#[tokio::test]
async fn test_recall_command_by_name() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let recall_request = RecallByNameRequest {
        name: "build".to_string(),
        user: Some("testuser".to_string()),
        hostname: Some("testhost".to_string()),
        cwd: Some("/test/dir".to_string()),
        namespace_hint: Some("rust".to_string()),
        scope_hint: Some("personal".to_string()),
    };

    let response_body = Command {
        id: 1,
        command_string: "cargo build --release".to_string(),
        name: "build".to_string(),
        namespace: "rust".to_string(),
        user: Some("testuser".to_string()),
        cwd: Some("/test/dir".to_string()),
        hostname: Some("testhost".to_string()),
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        use_count: 10,
        is_new: false,
    };

    Mock::given(method("POST"))
        .and(path("/commands/recall-by-name"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.recall_command_by_name(recall_request).await;

    // Assert
    assert!(result.is_ok());
    let command = result.unwrap();
    assert_eq!(command.command_string, "cargo build --release");
    assert_eq!(command.use_count, 10);
}

#[tokio::test]
async fn test_create_execution_record() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let execution_record = ExecutionHistoryCreate {
        command_id: 123,
        user: Some("testuser".to_string()),
        hostname: Some("testhost".to_string()),
        cwd: Some("/test/dir".to_string()),
        arguments: Some("--verbose".to_string()),
        execution_method: "id".to_string(),
        duration_ms: Some(1500),
        exit_code: Some(0),
    };

    let response_body = json!({"id": 456, "created": true});

    Mock::given(method("POST"))
        .and(path("/execution-history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.create_execution_record(execution_record).await;

    // Assert
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["id"], 456);
    assert_eq!(response["created"], true);
}

#[tokio::test]
async fn test_get_execution_analytics() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());

    let response_body = json!({
        "total_executions": 150,
        "unique_commands": 25,
        "average_executions_per_day": 10.5,
        "most_used_commands": [
            {"name": "build", "namespace": "rust", "execution_count": 45},
            {"name": "test", "namespace": "rust", "execution_count": 30}
        ],
        "execution_methods": {
            "id": 80,
            "name": 45,
            "namespace_name": 25
        }
    });

    Mock::given(method("GET"))
        .and(path("/analytics/execution"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client
        .get_execution_analytics(Some("testuser"), Some(30))
        .await;

    // Assert
    assert!(result.is_ok());
    let analytics = result.unwrap();
    assert_eq!(analytics["total_executions"], 150);
    assert_eq!(analytics["unique_commands"], 25);
    assert_eq!(analytics["average_executions_per_day"], 10.5);
}

#[tokio::test]
async fn test_execute_command_tracking() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let command_id = 123;

    let response_body = Command {
        id: command_id,
        command_string: "echo 'test execution'".to_string(),
        name: "test".to_string(),
        namespace: "testing".to_string(),
        user: Some("testuser".to_string()),
        cwd: Some("/test/dir".to_string()),
        hostname: Some("testhost".to_string()),
        scope: "personal".to_string(),
        created_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        use_count: 1,
        is_new: false,
    };

    Mock::given(method("POST"))
        .and(path(format!("/commands/{}/execute", command_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.execute_command(command_id, "testuser").await;

    // Assert
    assert!(result.is_ok());
    let command = result.unwrap();
    assert_eq!(command.id, command_id);
    assert_eq!(command.command_string, "echo 'test execution'");
    assert_eq!(command.use_count, 1);
}

#[tokio::test]
async fn test_error_handling_404() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());

    Mock::given(method("POST"))
        .and(path("/commands/recall"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    // Act
    let result = client
        .recall_command("nonexistent", "command", "user", "host", "/tmp")
        .await;

    // Assert
    assert!(result.is_err());
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("not found"));
}

#[tokio::test]
async fn test_similar_commands_empty_result() {
    // Arrange
    let server = MockServer::start().await;
    let client = ApiClient::new(server.uri());
    let command_id = 999;

    let response_body: Vec<Command> = vec![];

    Mock::given(method("GET"))
        .and(path(format!("/commands/{}/similar", command_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&server)
        .await;

    // Act
    let result = client.get_similar_commands(command_id, Some(5)).await;

    // Assert
    assert!(result.is_ok());
    let similar_commands = result.unwrap();
    assert_eq!(similar_commands.len(), 0);
}
