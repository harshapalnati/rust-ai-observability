// src/log_pipeline.rs

use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use clickhouse::{Client, Row};
use std::{sync::Arc};
use tracing::{info, error};
use dotenv::dotenv;
use serde_json;

/// Standardized log entry structure with enriched fields
#[derive(Debug, Serialize, Deserialize, Row, Clone,)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub source: String,
    pub level: String,
    pub message: String,
    pub timestamp: String,
    pub request_id: String,
    pub environment: String,
    pub hostname: String,
    pub application_version: String,
    pub user_id: String,
    pub tags: Vec<String>,
}

/// Log processing pipeline trait
pub trait LogPipeline {
    fn process(&self, raw_log: LogEntry) -> LogEntry;
}

/// ClickHousePipeline enriches logs and stores them in ClickHouse
pub struct ClickHousePipeline {
    client: Arc<Client>,
}

impl ClickHousePipeline {
    /// Initialize the pipeline with a ClickHouse client using environment variables
    pub fn new() -> Self {
        dotenv().ok();
        
        let url = std::env::var("url").expect("ClickHouse URL not set");
        let username = "default"; // Assuming default user
        let password = std::env::var("Password").expect("ClickHouse Password not set");
        
        info!("Connecting to ClickHouse with:");
        info!("URL: {}", url);

        let client = Client::default()
            .with_url(url)
            .with_user(username)
            .with_password(password);
        
        Self {
            client: Arc::new(client)
        }
    }

    /// Check if the table exists, if not create it
    pub async fn check_and_create_table(&self) -> clickhouse::error::Result<()> {
        let sql = "SHOW TABLES";
        match self.client.query(sql).fetch_all::<String>().await {
            Ok(rows) if rows.contains(&"logs".to_string()) => {
                info!("Table 'logs' already exists.");
            }
            _ => {
                info!("Creating table 'test_logs'...");
                let create_sql = r#"
                    CREATE TABLE IF NOT EXISTS logs (
                        source String,
                        level String,
                        message String,
                        timestamp String,
                        request_id String,
                        environment String,
                        hostname String,
                        application_version String,
                        user_id String,
                        tags Array(String)

                    ) ENGINE = MergeTree()
                    ORDER BY ( timestamp, source)
                "#;
                self.client.query(create_sql).execute().await?;
            }
        }
        Ok(())
    }

    /// Insert a log entry into ClickHouse
    pub async fn save_to_clickhouse(&self, log: &LogEntry) -> clickhouse::error::Result<()> {
        let mut insert = self.client.insert("logs")?;
        insert.write(log).await?;
        insert.end().await
    }
}

impl LogPipeline for ClickHousePipeline {
    fn process(&self, mut raw_log: LogEntry) -> LogEntry {
        raw_log.level = match raw_log.level.to_uppercase().as_str() {
            "INFO" | "WARNING" | "ERROR" | "DEBUG" | "TRACE" => raw_log.level.to_uppercase(),
            _ => "INFO".to_string(),
        };

      
        raw_log.hostname = hostname::get()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());

          // Only overwrite the environment if it is empty
    if raw_log.environment.trim().is_empty() {
        raw_log.environment = "unknown".to_string();
    }

    // Only overwrite the hostname if it is empty
    if raw_log.hostname.trim().is_empty() {
        raw_log.hostname = hostname::get()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());
    }

    // Only overwrite the application version if it is empty
    if raw_log.application_version.trim().is_empty() {
        raw_log.application_version = "1.0.0".to_string();
    }
       

        let client = self.client.clone();
        let log_copy = raw_log.clone();
        tokio::spawn(async move {
            if let Err(e) = async {
                let mut insert = client.insert("logs")?;
                insert.write(&log_copy).await?;
                insert.end().await
            }.await {
                eprintln!("Failed to save log to ClickHouse: {:?}", e);
            }
        });

        raw_log
    }
}