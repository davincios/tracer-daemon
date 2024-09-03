use std::collections::HashMap;

use chrono::Utc;
use serde::Serialize;

use crate::errors::TriggerMetadata;
#[allow(dead_code)]
use crate::{
    errors::{Issue, SystemSummary, ToolRunSummary},
    file_system_watcher::FileInfo,
};

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct IssueEntry {
    pub timestamp: u64,
    pub issue: Issue,
}

const STATE_VALIDITY_DURATION: u64 = 1000 * 30; // 30 seconds
const CLEANUP_INTERVAL: u64 = 1000 * 2; // 2 seconds

#[derive(Clone, Serialize)]
pub struct SystemStateSnapshot<'a> {
    pub system_summary: SystemSummary,
    pub tool_run_summaries: Vec<ToolRunSummary>,
    pub workspace_files: &'a HashMap<String, FileInfo>,
    pub stdout_lines: &'a Vec<LogEntry>,
    pub stderr_lines: &'a Vec<LogEntry>,
    pub syslog_lines: &'a Vec<LogEntry>,
    pub found_issues: &'a Vec<IssueEntry>,
}

pub struct SystemStateManager {
    pub tool_run_summaries: Vec<ToolRunSummary>,
    pub system_summary: Option<SystemSummary>,
    pub stdout_lines: Vec<LogEntry>,
    pub stderr_lines: Vec<LogEntry>,
    pub syslog_lines: Vec<LogEntry>,
    pub found_issues: Vec<IssueEntry>,
    pub last_cleanup_time: Option<u64>,
}

impl SystemStateManager {
    pub fn new() -> SystemStateManager {
        SystemStateManager {
            tool_run_summaries: Vec::new(),
            system_summary: None,
            stdout_lines: Vec::new(),
            stderr_lines: Vec::new(),
            syslog_lines: Vec::new(),
            found_issues: Vec::new(),
            last_cleanup_time: None,
        }
    }

    pub fn cleanup_invalid(&mut self) {
        if self.last_cleanup_time.is_none()
            || Utc::now().timestamp_millis() as u64 - self.last_cleanup_time.unwrap()
                > CLEANUP_INTERVAL
        {
            return;
        }
        let now = Utc::now().timestamp_millis() as u64;
        self.tool_run_summaries
            .retain(|summary| now - summary.timestamp < STATE_VALIDITY_DURATION);
        self.stdout_lines
            .retain(|entry| now - entry.timestamp < STATE_VALIDITY_DURATION);
        self.stderr_lines
            .retain(|entry| now - entry.timestamp < STATE_VALIDITY_DURATION);
        self.syslog_lines
            .retain(|entry| now - entry.timestamp < STATE_VALIDITY_DURATION);
        self.found_issues
            .retain(|entry| now - entry.timestamp < STATE_VALIDITY_DURATION);
        self.last_cleanup_time = Some(now);
    }

    pub fn add_tool_run_summary(&mut self, summary: ToolRunSummary) {
        self.tool_run_summaries.push(summary);
    }

    pub fn add_stdout_lines(&mut self, timestamp: u64, messages: Vec<String>) {
        self.stdout_lines.append(
            &mut messages
                .iter()
                .map(|message| LogEntry {
                    timestamp,
                    message: message.clone(),
                })
                .collect(),
        );
    }

    pub fn add_stderr_lines(&mut self, timestamp: u64, messages: Vec<String>) {
        self.stderr_lines.append(
            &mut messages
                .iter()
                .map(|message| LogEntry {
                    timestamp,
                    message: message.clone(),
                })
                .collect(),
        );
    }

    pub fn add_syslog_lines(&mut self, timestamp: u64, messages: Vec<String>) {
        self.syslog_lines.append(
            &mut messages
                .iter()
                .map(|message| LogEntry {
                    timestamp,
                    message: message.clone(),
                })
                .collect(),
        );
    }

    pub fn add_found_issues(&mut self, timestamp: u64, issues: Vec<Issue>) {
        self.found_issues.append(
            &mut issues
                .iter()
                .map(|issue| IssueEntry {
                    timestamp,
                    issue: *issue,
                })
                .collect(),
        );
    }

    pub fn refresh_system_summary(
        &mut self,
        cpu_utilization: f64,
        memory_utilization: f64,
        disk_utilizations: Vec<f64>,
    ) {
        self.system_summary = Some(SystemSummary {
            cpu_utilization,
            memory_utilization,
            disk_utilizations,
        })
    }

    pub fn clear_all(&mut self) {
        self.tool_run_summaries.clear();
        self.stdout_lines.clear();
        self.stderr_lines.clear();
        self.syslog_lines.clear();
        self.found_issues.clear();
    }

    pub fn clear_by_trigger_metadata(&mut self, trigger_metadata: &TriggerMetadata) {
        if !trigger_metadata.stdout_lines.is_empty() {
            self.stdout_lines.retain(|entry| {
                !trigger_metadata.stdout_lines.iter().any(|metadata| {
                    metadata.timestamp == entry.timestamp && metadata.message == entry.message
                })
            });
        }

        if !trigger_metadata.stderr_lines.is_empty() {
            self.stderr_lines.retain(|entry| {
                !trigger_metadata.stderr_lines.iter().any(|metadata| {
                    metadata.timestamp == entry.timestamp && metadata.message == entry.message
                })
            });
        }

        if !trigger_metadata.syslog_lines.is_empty() {
            self.syslog_lines.retain(|entry| {
                !trigger_metadata.syslog_lines.iter().any(|metadata| {
                    metadata.timestamp == entry.timestamp && metadata.message == entry.message
                })
            });
        }
    }

    pub fn get_current_state<'a>(
        &'a self,
        workspace_files: &'a HashMap<String, FileInfo>,
    ) -> Option<SystemStateSnapshot<'a>> {
        self.system_summary.as_ref()?;
        Some(SystemStateSnapshot {
            tool_run_summaries: Vec::new(),
            workspace_files,
            system_summary: self.system_summary.as_ref().unwrap().clone(),
            stdout_lines: &self.stdout_lines,
            stderr_lines: &self.stderr_lines,
            syslog_lines: &self.syslog_lines,
            found_issues: &self.found_issues,
        })
    }
}
