use crate::pipeline::{ClickHousePipeline, LogPipeline, LogEntry};
use axum::{routing::post, Router, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

/// HTTP log entry format
#[derive(Deserialize, Debug)]
pub struct HttpLogEntry {
    pub source: String,
    pub level: String,
    pub message: String,
    pub request_id: Option<String>,
    pub environment: Option<String>,
    pub hostname: Option<String>,
    pub application_version: Option<String>,
    pub user_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Ingestion endpoint: Validates, processes, and enriches logs
pub async fn ingest_log(Json(payload): Json<HttpLogEntry>) -> impl IntoResponse {
    // Validate the basic structure of the log
    if payload.source.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing or empty 'source' field").into_response();
    }
    if payload.level.trim().is_empty() || !["INFO", "ERROR", "DEBUG"].contains(&payload.level.to_uppercase().as_str()) {
        return (StatusCode::BAD_REQUEST, "Invalid 'level' field").into_response();
    }
    if payload.message.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing or empty 'message' field").into_response();
    }

    // Convert the payload into a standard LogEntry and pass it through the pipeline
    let pipeline = ClickHousePipeline::new();
    let log_entry = HttpLogEntry {
        source: payload.source,
        level: payload.level,
        message: payload.message,
        request_id: payload.request_id,
        environment: payload.environment,
        hostname: payload.hostname,
        application_version: payload.application_version,
        user_id: payload.user_id,
        tags: payload.tags,
    };
    let log_entry = LogEntry {
        source: log_entry.source,
        level: log_entry.level,
        message: log_entry.message,
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        request_id: log_entry.request_id.unwrap_or_default(),
        environment: log_entry.environment.unwrap_or_default(),
        hostname: log_entry.hostname.unwrap_or_default(),
        application_version: log_entry.application_version.unwrap_or_default(),
        user_id: log_entry.user_id.unwrap_or_default(),
        tags: log_entry.tags.unwrap_or_default(),
    };

    println!("LogEntry: {:?}", log_entry);
    let enriched_log = pipeline.process(log_entry);

    // Print the enriched log for debugging purposes
    tracing::info!("Enriched log: {:?}", enriched_log);

    // Return success response
    (StatusCode::OK, "Log processed successfully").into_response()
}

/// HTTP router with the ingestion route
pub fn create_http_router() -> Router {
    Router::new()
        .route("/logs", post(ingest_log))
}

/// Tests for the HTTP ingestion and pipeline
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test_helper::TestClient;

    #[tokio::test]
    async fn test_valid_log_entry() {
        let valid_entry = HttpLogEntry {
            source: "app-1".to_string(),
            level: "INFO".to_string(),
            message: "Application started".to_string(),
            request_id: Some("123".to_string()),
            environment: Some("test".to_string()),
            hostname: Some("localhost".to_string()),
            application_version: Some("1.0.0".to_string()),
            user_id: Some("user1".to_string()),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        };

        let response = ingest_log(Json(valid_entry)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_log_entry_level() {
        let invalid_entry = HttpLogEntry {
            source: "app-1".to_string(),
            level: "INVALID".to_string(),
            message: "Application started".to_string(),
            request_id: Some("123".to_string()),
            environment: Some("test".to_string()),
            hostname: Some("localhost".to_string()),
            application_version: Some("1.0.0".to_string()),
            user_id: Some("user1".to_string()),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        };

        let response = ingest_log(Json(invalid_entry)).await.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_endpoint_valid_log() {
        let app = create_http_router();
        let client = TestClient::new(app);

        let response = client.post("/logs")
            .json(&serde_json::json!({
                "source": "app-1",
                "level": "INFO",
                "message": "User logged in"
            }))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "Log processed successfully");
    }

    #[tokio::test]
    async fn test_http_endpoint_invalid_log() {
        let app = create_http_router();
        let client = TestClient::new(app);

        let response = client.post("/logs")
            .json(&serde_json::json!({
                "source": "app-1",
                "message": "User logged in"
            }))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.text().await;
        assert!(body.contains("Invalid 'level' field"));
    }

    #[tokio::test]
    async fn test_multiple_logs_in_sequence() {
        let app = create_http_router();
        let client = TestClient::new(app);

        for i in 1..=10 {
            let log_entry = serde_json::json!({
                "source": format!("app-{}", i),
                "level": "INFO",
                "message": format!("User logged in #{}", i)
            });

            let response = client.post("/logs")
                .json(&log_entry)
                .send()
                .await;

            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.text().await, "Log processed successfully");
        }
    }
}
