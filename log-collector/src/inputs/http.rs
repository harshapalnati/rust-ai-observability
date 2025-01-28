use axum::{
    routing::post,
    Router,
    http::StatusCode,
    response::IntoResponse,
    Json
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct LogEntry {
    pub source: String,
    pub level: String,
    pub message: String,
}

pub async fn ingest_log(Json(payload): Json<LogEntry>) -> impl IntoResponse {
    // Validate source
    if payload.source.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing or empty 'source' field").into_response();
    }

    // Validate level
    match payload.level.as_str() {
        "INFO" | "ERROR" | "DEBUG" => {}
        _ => return (StatusCode::BAD_REQUEST, "Invalid 'level' field").into_response(),
    }

    // Validate message
    if payload.message.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing or empty 'message' field").into_response();
    }

    // If all validations pass, proceed
    (StatusCode::OK, "Log ingested successfully").into_response()
}

pub fn create_http_router() -> Router {
    Router::new()
        .route("/logs", post(ingest_log))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_log_entry() {
        let valid_entry = LogEntry {
            source: "app-1".to_string(),
            level: "INFO".to_string(),
            message: "Application started".to_string(),
        };

        let response = ingest_log(Json(valid_entry)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_log_entry_level() {
        let invalid_entry = LogEntry {
            source: "app-1".to_string(),
            level: "INVALID".to_string(),
            message: "Application started".to_string(),
        };

        let response = ingest_log(Json(invalid_entry)).await.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::Router;
    use axum::http::StatusCode;
    use axum_test_helper::TestClient;

    #[tokio::test]
    async fn test_http_endpoint_valid_log() {
        // Create a test router
        let app = Router::new().route("/logs", post(ingest_log));

        // Build a test client
        let client = TestClient::new(app);

        // Send a valid log request
        let response = client.post("/logs")
            .json(&serde_json::json!({
                "source": "app-1",
                "level": "INFO",
                "message": "User logged in"
            }))
            .send()
            .await;

        // Check the response
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await;
        assert_eq!(body, "Log ingested successfully");
    }

    #[tokio::test]
    async fn test_http_endpoint_invalid_log() {
        let app = Router::new().route("/logs", post(ingest_log));
        let client = TestClient::new(app);

        let response = client.post("/logs")
            .json(&serde_json::json!({
                "source": "app-1",
                "message": "User logged in"
            }))
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response.text().await;
        println!("Response body: {}", body);
        assert!(body.contains("level"));
    }
}
