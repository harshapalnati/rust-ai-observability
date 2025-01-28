mod inputs;
mod pipeline;
use axum::Server;
use inputs::http::create_http_router;
use std::net::SocketAddr;
use tracing::{info, error};
use tracing_subscriber;
use crate::pipeline::{LogEntry, ClickHousePipeline};
use chrono::Utc;
use dotenv;
use clickhouse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up tracing for logging
    tracing_subscriber::fmt().init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Start HTTP server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("HTTP log collector running at http://{}", addr);
    let app = create_http_router();
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}