use std::time::Duration;

// src/events/mod.rs
use crate::{
    debug_log::Logger,
    http_client::{send_http_event, send_http_get},
    metrics::SystemMetricsCollector,
};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sysinfo::System;
use tracing::info;

#[derive(Debug)]
pub enum EventStatus {
    #[allow(dead_code)]
    NewRun,
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EventStatus::NewRun => "new_run".to_string(),
            }
        )
    }
}

pub async fn send_log_event(service_url: &str, api_key: &str, message: String) -> Result<String> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "run_status_message",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &log_entry)
        .await
        .context("Failed to send HTTP event")
}

pub async fn send_alert_event(service_url: &str, api_key: &str, message: String) -> Result<String> {
    let alert_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "alert",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &alert_entry)
        .await
        .context("Failed to send HTTP event")
}

pub struct RunEventOut {
    pub run_name: String,
    pub run_id: String,
    pub service_name: String,
}

const AWS_METADATA_URL: &str = "http://169.254.169.254/latest/meta-data/";

async fn get_aws_instance_metadata() -> Result<Value> {
    let (status, response_text) =
        send_http_get(AWS_METADATA_URL, None, Some(Duration::from_secs(2))).await?;

    serde_json::from_str(&response_text).context(format!(
        "Failed to get AWS instance metadata. Status: {}, Response: {}",
        status, response_text
    ))
}

async fn gather_system_properties(system: &System) -> Value {
    let aws_metadata = get_aws_instance_metadata().await.unwrap_or(json!(null));

    let disk_metadata = SystemMetricsCollector::gather_disk_data();

    json!(
        {
            "os": System::name(),
            "os_version": System::os_version(),
            "kernel_version": System::kernel_version(),
            "arch": System::cpu_arch(),
            "num_cpus": system.cpus().len(),
            "hostname": System::host_name(),
            "total_memory": system.total_memory(),
            "total_swap": system.total_swap(),
            "uptime": System::uptime(),
            "aws_metadata": aws_metadata,
            "is_aws_instance": !aws_metadata.is_null(),
            "system_disk_io": disk_metadata,
        }
    )
}

pub async fn send_start_run_event(
    service_url: &str,
    api_key: &str,
    system: &System,
) -> Result<RunEventOut> {
    info!("Starting new pipeline...");

    let logger = Logger::new();

    #[derive(Deserialize)]
    struct RunLogOutProperties {
        run_name: String,
        run_id: String,
        service_name: String,
    }

    #[derive(Deserialize)]
    struct RunLogOut {
        properties: RunLogOutProperties,
    }

    #[derive(Deserialize)]
    struct RunLogResult {
        result: Vec<RunLogOut>,
    }

    let system_properties = gather_system_properties(system).await;

    let init_entry = json!({
        "message": "[CLI] Starting new pipeline run",
        "process_type": "pipeline",
        "process_status": "new_run",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
        "attributes": system_properties,
    });

    let result = send_http_event(service_url, api_key, &init_entry).await?;

    let value: RunLogResult = serde_json::from_str(&result).unwrap();

    logger
        .log(
            format!("New pipeline run result: {}", result).as_str(),
            None,
        )
        .await;

    if value.result.len() != 1 {
        return Err(anyhow::anyhow!("Invalid response from server"));
    }

    let run_name = &value.result[0].properties.run_name;

    let run_id = &value.result[0].properties.run_id;

    let service_name = &value.result[0].properties.service_name;

    logger
        .log(
            format!(
                "Run name: {}, run id: {}, service name: {}",
                run_name, run_id, service_name
            )
            .as_str(),
            None,
        )
        .await;

    info!("Started pipeline run successfully...");

    Ok(RunEventOut {
        run_name: run_name.clone(),
        run_id: run_id.clone(),
        service_name: service_name.clone(),
    })
}

pub async fn send_end_run_event(service_url: &str, api_key: &str) -> Result<String> {
    info!("Finishing pipeline run...");

    let end_entry = json!({
        "message": "[CLI] Finishing pipeline run",
        "process_type": "pipeline",
        "process_status": "finished_run",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    let result = send_http_event(service_url, api_key, &end_entry).await;

    info!("Ended pipeline run successfully...");
    result
}

pub async fn send_daemon_start_event(service_url: &str, api_key: &str) -> Result<String> {
    let daemon_start_entry: serde_json::Value = json!({
        "message": "[CLI] Starting daemon",
        "process_type": "pipeline",
        "process_status": "daemon_start",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &daemon_start_entry).await
}

pub async fn send_update_tags_event(
    service_url: &str,
    api_key: &str,
    tags: Vec<String>,
) -> Result<String> {
    let tags_entry = json!({
        "tags": tags,
        "message": "[CLI] Updating tags",
        "process_type": "pipeline",
        "process_status": "tag_update",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &tags_entry).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Error;

    #[tokio::test]
    async fn test_event_log() -> Result<(), Error> {
        let config = ConfigManager::load_default_config();
        send_log_event(
            &config.service_url.clone(),
            &config.api_key.clone(),
            "Test".to_string(),
        )
        .await?;

        Ok(())
    }
}
