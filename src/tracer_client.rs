// src/tracer_client.rs
use crate::event_recorder::{EventRecorder, EventType};
use crate::metrics::SystemMetricsCollector;
use crate::process_watcher::ProcessWatcher;
use crate::submit_batched_data::submit_batched_data;
use crate::{config_manager::Config, process_watcher::ShortLivedProcessLog};
use anyhow::Result;
use std::time::{Duration, Instant};
use sysinfo::System;

pub struct RunMetadata {
    pub last_interaction: Instant,
}

pub struct TracerClient {
    system: System,
    last_sent: Option<Instant>,
    interval: Duration,
    last_interaction_new_run_duration: Duration,
    process_metrics_send_interval: Duration,
    pub logs: EventRecorder,
    process_watcher: ProcessWatcher,
    metrics_collector: SystemMetricsCollector,
    api_key: String,
    service_url: String,
    current_run: Option<RunMetadata>
}

impl TracerClient {
    pub fn new(config: Config) -> Result<TracerClient> {
        let service_url = config.service_url.clone();

        println!("Initializing TracerClient with API Key: {}", config.api_key);
        println!("Service URL: {}", service_url);

        Ok(TracerClient {
            // fixed values
            api_key: config.api_key,
            service_url,
            interval: Duration::from_millis(config.process_polling_interval_ms),
            last_interaction_new_run_duration: Duration::from_millis(config.new_run_pause_ms),
            process_metrics_send_interval: Duration::from_millis(
                config.process_metrics_send_interval_ms,
            ),
            // updated values
            system: System::new_all(),
            last_sent: None,
            current_run: None,
            // Sub mannagers
            logs: EventRecorder::new(),
            process_watcher: ProcessWatcher::new(config.targets),
            metrics_collector: SystemMetricsCollector::new(),
        })
    }

    pub fn reload_config_file(&mut self, config: &Config) {
        self.api_key.clone_from(&config.api_key);
        self.service_url.clone_from(&config.service_url);
        self.interval = Duration::from_millis(config.process_polling_interval_ms);
        self.process_watcher.reload_targets(config.targets.clone());
    }

    pub fn fill_logs_with_short_lived_process(
        &mut self,
        short_lived_process_log: ShortLivedProcessLog,
    ) -> Result<()> {
        self.process_watcher
            .fill_logs_with_short_lived_process(short_lived_process_log, &mut self.logs)?;
        Ok(())
    }

    pub async fn submit_batched_data(&mut self) -> Result<()> {
        submit_batched_data(
            &self.api_key,
            &self.service_url,
            &mut self.system,
            &mut self.logs,
            &mut self.metrics_collector,
            &mut self.last_sent,
            self.interval,
        )
        .await
    }

    pub async fn run_cleanup(&mut self) -> Result<()> {
        if let Some(run) = self.current_run.as_mut() {
            if run.last_interaction.elapsed() > self.last_interaction_new_run_duration {
                self.logs.record_event(EventType::FinishedRun, "Run ended due to inactivity".to_string(), None);
                self.current_run = None;
            }
        }
        Ok(())
    }

    pub async fn start_new_run(&mut self) -> Result<()> {
        if self.current_run.is_some() {
            self.logs.record_event(EventType::FinishedRun, "Run ended due to new run".to_string(), None);
        }

        self.logs.record_event(EventType::NewRun, "[CLI] Starting new pipeline run".to_string(), None);
        self.current_run = Some(RunMetadata {
            last_interaction: Instant::now(),
        });
        Ok(())
    }

    pub async fn stop_run(&mut self) -> Result<()> {
        if let Some(_) = self.current_run.as_mut() {
            self.logs.record_event(EventType::FinishedRun, "Run ended due to user request".to_string(), None);
            self.current_run = None;
        }
        Ok(())
    }

    /// These functions require logs and the system
    pub async fn poll_processes(&mut self) -> Result<()> {
        self.process_watcher
            .poll_processes(&mut self.system, &mut self.logs)?;
        Ok(())
    }

    pub async fn poll_process_metrics(&mut self) -> Result<()> {
        self.process_watcher.poll_process_metrics(
            &self.system,
            &mut self.logs,
            self.process_metrics_send_interval,
        )?;
        Ok(())
    }

    pub async fn remove_completed_processes(&mut self) -> Result<()> {
        self.process_watcher
            .remove_completed_processes(&mut self.system, &mut self.logs)?;
        Ok(())
    }

    pub fn refresh_sysinfo(&mut self) {
        self.system.refresh_all();
    }

    pub fn get_service_url(&self) -> &str {
        &self.service_url
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}
