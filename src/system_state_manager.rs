#[allow(dead_code)]
use crate::{
    errors::{Issue, SystemStateSnapshot, SystemSummary, ToolRunSummary},
    file_system_watcher::FileInfo,
};

pub struct SystemStateManager {
    pub tool_run_summaries: Vec<ToolRunSummary>,
    pub workspace_files: std::collections::HashMap<String, FileInfo>,
    pub system_summary: SystemSummary,
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
    pub syslog_lines: Vec<String>,
    pub found_issues: Vec<Issue>,
}

impl SystemStateManager {
    pub fn new() -> SystemStateManager {
        SystemStateManager {
            tool_run_summaries: todo!(),
            workspace_files: todo!(),
            system_summary: todo!(),
            stdout_lines: todo!(),
            stderr_lines: todo!(),
            syslog_lines: todo!(),
            found_issues: todo!(),
        }
    }

    pub fn get_current_state(&self) -> SystemStateSnapshot {
        SystemStateSnapshot {
            tool_run_summaries: Vec::new(),
            workspace_files: &self.workspace_files,
            system_summary: self.system_summary.clone(),
            stdout_lines: &self.stdout_lines,
            stderr_lines: &self.stderr_lines,
            syslog_lines: &self.syslog_lines,
            found_issues: &self.found_issues,
        }
    }
}
