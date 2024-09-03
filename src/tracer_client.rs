use crate::config_manager::{INTERCEPTOR_STDERR_FILE, INTERCEPTOR_STDOUT_FILE};
use crate::errors::{ErrorRecognition, ERROR_TEMPLATES};
// src/tracer_client.rs
use crate::event_recorder::{EventRecorder, EventType};
use crate::events::{send_end_run_event, send_start_run_event};
use crate::file_content_watcher::stderr_patterns::STDERR_PATTERNS;
use crate::file_content_watcher::stdout_patterns::STDOUT_PATTERNS;
use crate::file_content_watcher::syslog_patterns::SYSLOG_PATTERNS;
use crate::file_content_watcher::FileContentWatcher;
use crate::file_system_watcher::FileSystemWatcher;
use crate::metrics::SystemMetricsCollector;
use crate::process_watcher::ProcessWatcher;
use crate::submit_batched_data::submit_batched_data;
use crate::system_state_manager::SystemStateManager;
use crate::{config_manager::Config, process_watcher::ShortLivedProcessLog};
use crate::{FILE_CACHE_DIR, SYSLOG_FILE};
use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};
use std::ops::Sub;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Pid, System};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RunMetadata {
    pub last_interaction: Instant,
    pub name: String,
    pub id: String,
    pub service_name: String,
    pub parent_pid: Option<Pid>,
    pub start_time: DateTime<Utc>,
}

const RUN_COMPLICATED_PROCESS_IDENTIFICATION: bool = false;
const WAIT_FOR_PROCESS_BEFORE_NEW_RUN: bool = false;

pub type LinesBufferArc = Arc<RwLock<Vec<String>>>;

pub struct TracerClient {
    system: System,
    last_sent: Option<Instant>,
    interval: Duration,
    last_interaction_new_run_duration: Duration,
    process_metrics_send_interval: Duration,
    last_file_size_change_time_delta: TimeDelta,
    pub logs: EventRecorder,
    process_watcher: ProcessWatcher,
    metrics_collector: SystemMetricsCollector,
    file_system_watcher: FileSystemWatcher,
    file_content_watcher: FileContentWatcher,
    error_recognizer: ErrorRecognition<'static>,
    system_state_manager: SystemStateManager,
    workflow_directory: String,
    api_key: String,
    service_url: String,
    current_run: Option<RunMetadata>,
    syslog_lines_buffer: LinesBufferArc,
    stdout_lines_buffer: LinesBufferArc,
    stderr_lines_buffer: LinesBufferArc,
}

impl TracerClient {
    pub async fn new(config: Config, workflow_directory: String) -> Result<TracerClient> {
        let service_url = config.service_url.clone();

        println!("Initializing TracerClient with API Key: {}", config.api_key);
        println!("Service URL: {}", service_url);

        let file_watcher = FileSystemWatcher::new();

        file_watcher.prepare_cache_directory(FILE_CACHE_DIR)?;

        Ok(TracerClient {
            // fixed values
            api_key: config.api_key,
            service_url,
            interval: Duration::from_millis(config.process_polling_interval_ms),
            last_interaction_new_run_duration: Duration::from_millis(config.new_run_pause_ms),
            process_metrics_send_interval: Duration::from_millis(
                config.process_metrics_send_interval_ms,
            ),
            last_file_size_change_time_delta: TimeDelta::milliseconds(
                config.file_size_not_changing_period_ms as i64,
            ),
            // updated values
            system: System::new_all(),
            last_sent: None,
            current_run: None,
            // Sub mannagers
            logs: EventRecorder::new(),
            file_system_watcher: file_watcher,
            workflow_directory,
            syslog_lines_buffer: Arc::new(RwLock::new(Vec::new())),
            stdout_lines_buffer: Arc::new(RwLock::new(Vec::new())),
            stderr_lines_buffer: Arc::new(RwLock::new(Vec::new())),
            process_watcher: ProcessWatcher::new(config.targets),
            metrics_collector: SystemMetricsCollector::new(),
            file_content_watcher: FileContentWatcher::new(),
            error_recognizer: ErrorRecognition::new(&ERROR_TEMPLATES),
            system_state_manager: SystemStateManager::new(),
        })
    }

    pub fn reload_config_file(&mut self, config: &Config) {
        self.api_key.clone_from(&config.api_key);
        self.service_url.clone_from(&config.service_url);
        self.interval = Duration::from_millis(config.process_polling_interval_ms);
        self.process_watcher.reload_targets(config.targets.clone());
    }

    pub fn setup_file_content_watcher(&mut self) -> tokio::task::JoinHandle<()> {
        self.file_content_watcher.add_entry(
            SYSLOG_FILE.into(),
            &SYSLOG_PATTERNS,
            self.syslog_lines_buffer.clone(),
        );

        self.file_content_watcher.add_entry(
            INTERCEPTOR_STDOUT_FILE.into(),
            &STDOUT_PATTERNS,
            self.stdout_lines_buffer.clone(),
        );

        self.file_content_watcher.add_entry(
            INTERCEPTOR_STDERR_FILE.into(),
            &STDERR_PATTERNS,
            self.stderr_lines_buffer.clone(),
        );

        self.file_content_watcher.setup_thread()
    }

    pub fn fill_logs_with_short_lived_process(
        &mut self,
        short_lived_process_log: ShortLivedProcessLog,
    ) -> Result<()> {
        self.process_watcher
            .fill_logs_with_short_lived_process(short_lived_process_log, &mut self.logs)?;
        Ok(())
    }

    pub fn get_syslog_lines_buffer(&self) -> LinesBufferArc {
        self.syslog_lines_buffer.clone()
    }

    pub fn get_stdout_stderr_lines_buffer(&self) -> (LinesBufferArc, LinesBufferArc) {
        (
            self.stdout_lines_buffer.clone(),
            self.stderr_lines_buffer.clone(),
        )
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

    pub fn get_run_metadata(&self) -> Option<RunMetadata> {
        self.current_run.clone()
    }

    pub async fn run_cleanup(&mut self) -> Result<()> {
        if let Some(run) = self.current_run.as_mut() {
            if !RUN_COMPLICATED_PROCESS_IDENTIFICATION {
                return Ok(());
            }
            if run.last_interaction.elapsed() > self.last_interaction_new_run_duration {
                self.logs.record_event(
                    EventType::FinishedRun,
                    "Run ended due to inactivity".to_string(),
                    None,
                    None,
                );
                self.current_run = None;
            } else if run.parent_pid.is_none() && !self.process_watcher.is_empty() {
                run.parent_pid = self.process_watcher.get_parent_pid(Some(run.start_time));
            } else if run.parent_pid.is_some() {
                let parent_pid = run.parent_pid.unwrap();
                if !self
                    .process_watcher
                    .is_process_alive(&self.system, parent_pid)
                {
                    self.logs.record_event(
                        EventType::FinishedRun,
                        "Run ended due to parent process termination".to_string(),
                        None,
                        None,
                    );
                    self.current_run = None;
                }
            }
        } else if !WAIT_FOR_PROCESS_BEFORE_NEW_RUN || !self.process_watcher.is_empty() {
            let earliest_process_time = self.process_watcher.get_earliest_process_time();
            self.start_new_run(Some(earliest_process_time.sub(Duration::from_millis(1))))
                .await?;
        }
        Ok(())
    }

    pub async fn start_new_run(&mut self, timestamp: Option<DateTime<Utc>>) -> Result<()> {
        if self.current_run.is_some() {
            self.stop_run().await?;
        }

        let result = send_start_run_event(&self.service_url, &self.api_key, &self.system).await?;

        self.current_run = Some(RunMetadata {
            last_interaction: Instant::now(),
            parent_pid: None,
            start_time: timestamp.unwrap_or_else(Utc::now),
            name: result.run_name,
            id: result.run_id,
            service_name: result.service_name,
        });

        Ok(())
    }

    pub async fn stop_run(&mut self) -> Result<()> {
        if self.current_run.is_some() {
            send_end_run_event(&self.service_url, &self.api_key).await?;
            self.current_run = None;
            self.system_state_manager.clear_all();
        }
        Ok(())
    }

    /// These functions require logs and the system
    pub fn poll_processes(&mut self) -> Result<()> {
        self.process_watcher.poll_processes(
            &mut self.system,
            &mut self.logs,
            &self.file_system_watcher,
        )?;

        if self.current_run.is_some() && !self.process_watcher.is_empty() {
            self.current_run.as_mut().unwrap().last_interaction = Instant::now();
        }
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

    pub async fn poll_files(&mut self) -> Result<()> {
        self.file_system_watcher
            .poll_files(
                &self.service_url,
                &self.api_key,
                &self.workflow_directory,
                FILE_CACHE_DIR,
                self.last_file_size_change_time_delta,
            )
            .await?;

        Ok(())
    }

    pub async fn poll_file_content_watcher_streams(&mut self) -> Result<()> {
        FileContentWatcher::send_lines_to_endpoint(
            &format!("{}/stdout-capture", self.service_url),
            &self.api_key,
            &self.stdout_lines_buffer,
            false,
        )
        .await?;

        FileContentWatcher::send_lines_to_endpoint(
            &format!("{}/stdout-capture", self.service_url),
            &self.api_key,
            &self.stderr_lines_buffer,
            true,
        )
        .await?;

        let timestamp: u64 = Utc::now().timestamp_millis() as u64;

        self.system_state_manager
            .add_syslog_lines(timestamp, self.syslog_lines_buffer.read().await.clone());
        self.system_state_manager
            .add_stdout_lines(timestamp, self.stdout_lines_buffer.read().await.clone());
        self.system_state_manager
            .add_stderr_lines(timestamp, self.stderr_lines_buffer.read().await.clone());

        self.file_content_watcher
            .poll_files_and_clear_buffers()
            .await?;

        Ok(())
    }

    pub async fn poll_errors(&mut self) -> Result<()> {
        self.error_recognizer.recognize_and_record_errors(
            &mut self.logs,
            &mut self.system_state_manager,
            &self.file_system_watcher,
        );
        Ok(())
    }

    pub fn refresh_sysinfo(&mut self) {
        self.system.refresh_all();
        let disks = Disks::new_with_refreshed_list()
            .into_iter()
            .map(|d| {
                let total_space = d.total_space();
                let available_space = d.available_space();
                let used_space = total_space - available_space;
                (used_space as f64 / total_space as f64) * 100.0
            })
            .collect();
        self.system_state_manager.refresh_system_summary(
            self.system.global_cpu_info().cpu_usage() as f64,
            self.system.available_memory() as f64 / self.system.total_memory() as f64,
            disks,
        );
    }

    pub fn reset_just_started_process_flag(&mut self) {
        self.process_watcher.reset_just_started_process_flag();
    }

    pub fn get_service_url(&self) -> &str {
        &self.service_url
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}
