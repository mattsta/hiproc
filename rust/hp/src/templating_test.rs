//! Unit tests for the templating engine.
#![cfg(test)]
use super::templating;

#[test]
fn test_resolve_no_args() {
    let template = "ls -la";
    let args = vec![];
    let result = templating::resolve_arguments(template, &args).unwrap();
    assert_eq!(result, "ls -la");
}

#[test]
fn test_resolve_named_args() {
    let template = "ssh -i key.pem user@{{HOSTNAME}}";
    let args = vec!["HOSTNAME:mysite.com".to_string()];
    let result = templating::resolve_arguments(template, &args).unwrap();
    assert_eq!(result, "ssh -i key.pem user@mysite.com");
}

#[test]
fn test_resolve_passthrough_args() {
    let template = "git log";
    let args = vec!["--oneline".to_string(), "--graph".to_string()];
    let result = templating::resolve_arguments(template, &args).unwrap();
    assert_eq!(result, "git log --oneline --graph");
}

#[test]
fn test_resolve_mixed_args() {
    let template = "docker run -p {{PORT}}:80 my-image";
    let args = vec![
        "PORT:8080".to_string(),
        "-d".to_string(),
        "--name=my-container".to_string(),
    ];
    let result = templating::resolve_arguments(template, &args).unwrap();
    assert_eq!(
        result,
        "docker run -p 8080:80 my-image -d --name=my-container"
    );
}

#[test]
fn test_resolve_missing_placeholder_fails() {
    let template = "echo {{MESSAGE}}";
    let args = vec![];
    let result = templating::resolve_arguments(template, &args);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "The following placeholder was not provided: {{MESSAGE}}"
    );
}
