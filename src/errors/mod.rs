#![allow(dead_code)]

pub mod conditions;
mod templates;
use conditions::ErrorCondition;
use serde::Serialize;
pub use templates::ERROR_TEMPLATES;

use crate::{
    event_recorder::{EventRecorder, EventType},
    file_system_watcher::FileSystemWatcher,
    system_state_manager::{LogEntry, SystemStateManager, SystemStateSnapshot},
};

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
pub enum Issue {
    OutOfMemory,
    Other,
}

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Medium,
    High,
    Critical,
}

#[derive(Serialize, Clone)]
pub struct ToolRunSummary {
    pub tool_name: String,
    pub tool_path: String,
    pub run_duration: u64,
    pub max_memory_utilization: f64,
    pub max_cpu_usage: f64,
    pub timestamp: u64,
}

#[derive(Serialize, Clone)]
pub struct SystemSummary {
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub disk_utilizations: Vec<f64>,
}

pub struct ErrorTemplate {
    pub id: String,
    pub display_name: String,
    pub severity: ErrorSeverity,
    pub causes: Vec<String>,
    pub advices: Vec<String>,
    pub condition: ErrorCondition,
}

#[derive(Serialize, Default)]
pub struct TriggerMetadata {
    pub stdout_lines: Vec<LogEntry>,
    pub stderr_lines: Vec<LogEntry>,
    pub syslog_lines: Vec<LogEntry>,
    pub files: Vec<String>,
    pub tool_run_summaries: Vec<ToolRunSummary>,
    pub issues: Vec<Issue>, // Isn't really used at the moment
}

impl TriggerMetadata {
    pub fn new_stdout(stdout_line: LogEntry) -> TriggerMetadata {
        TriggerMetadata {
            stdout_lines: vec![stdout_line],
            ..Default::default()
        }
    }

    pub fn new_stderr(stderr_line: LogEntry) -> TriggerMetadata {
        TriggerMetadata {
            stderr_lines: vec![stderr_line],
            ..Default::default()
        }
    }

    pub fn new_file(file: String) -> TriggerMetadata {
        TriggerMetadata {
            files: vec![file],
            ..Default::default()
        }
    }

    pub fn new_syslog(syslog_line: LogEntry) -> TriggerMetadata {
        TriggerMetadata {
            syslog_lines: vec![syslog_line],
            ..Default::default()
        }
    }

    pub fn new_tool_run_summaries(tool_run_summary: ToolRunSummary) -> TriggerMetadata {
        TriggerMetadata {
            tool_run_summaries: vec![tool_run_summary],
            ..Default::default()
        }
    }

    pub fn new_issue(issue: Issue) -> TriggerMetadata {
        TriggerMetadata {
            issues: vec![issue],
            ..Default::default()
        }
    }

    pub fn merge(&mut self, other: TriggerMetadata) {
        self.stdout_lines.extend(other.stdout_lines);
        self.stderr_lines.extend(other.stderr_lines);
        self.syslog_lines.extend(other.syslog_lines);
        self.tool_run_summaries.extend(other.tool_run_summaries);
        self.issues.extend(other.issues);
        self.files.extend(other.files);
    }
}

#[derive(Serialize)]
pub struct ErrorOutput<'a> {
    pub id: String,
    pub display_name: String,
    pub severity: ErrorSeverity,
    pub causes: Vec<String>,
    pub advices: Vec<String>,
    pub trigger_metadata: TriggerMetadata,
    pub system_state: SystemStateSnapshot<'a>,
}

pub struct ErrorRecognition<'a> {
    pub templates: &'a Vec<ErrorTemplate>,
}

impl ErrorRecognition<'_> {
    pub fn new(templates: &Vec<ErrorTemplate>) -> ErrorRecognition {
        ErrorRecognition { templates }
    }

    pub fn recognize_errors<'a>(
        &self,
        system_state: SystemStateSnapshot<'a>,
    ) -> Vec<ErrorOutput<'a>> {
        let mut errors = Vec::new();
        for template in self.templates {
            if let Some(trigger_metadata) = template.condition.trigger(&system_state) {
                let error = ErrorOutput {
                    id: template.id.clone(),
                    display_name: template.display_name.clone(),
                    severity: template.severity,
                    causes: template.causes.clone(),
                    advices: template.advices.clone(),
                    trigger_metadata,
                    system_state: system_state.clone(),
                };
                errors.push(error);
            }
        }
        errors
    }

    pub fn recognize_and_record_errors(
        &self,
        event_recorder: &mut EventRecorder,
        system_state_manager: &mut SystemStateManager,
        file_system_watcher: &FileSystemWatcher,
    ) {
        let system_state =
            system_state_manager.get_current_state(file_system_watcher.get_current_all_files());
        if system_state.is_none() {
            return;
        }
        let errors = self.recognize_errors(system_state.unwrap());
        let mut triggers_to_clear = vec![];
        for error in errors {
            event_recorder.record_event(
                EventType::ErrorEvent,
                error.display_name.clone(),
                Some(serde_json::to_value(&error).unwrap()),
                None,
            );

            triggers_to_clear.push(error.trigger_metadata);
        }

        for trigger in triggers_to_clear {
            system_state_manager.clear_by_trigger_metadata(&trigger);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        errors::{ErrorRecognition, SystemStateSnapshot, SystemSummary},
        file_system_watcher::FileInfo,
        system_state_manager::IssueEntry,
    };

    use super::{
        conditions::{
            ErrorCondition, FileExistsCondition, IssueCondition, SystemCPUCondition,
            SystemMemoryCondition,
        },
        ErrorSeverity, ErrorTemplate, Issue,
    };

    fn create_test_template(id: String, condition: ErrorCondition) -> ErrorTemplate {
        ErrorTemplate {
            id,
            display_name: "".to_string(),
            severity: ErrorSeverity::Warning,
            causes: vec![],
            advices: vec![],
            condition,
        }
    }

    #[test]
    fn test_recognize_basic_issue_error() {
        let templates = vec![
            ErrorTemplate {
                id: "basic_issue".to_string(),
                display_name: "Basic issue".to_string(),
                severity: ErrorSeverity::Warning,
                causes: vec!["Basic issue cause".to_string()],
                advices: vec!["Basic issue advice".to_string()],
                condition: ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                    issue: Issue::Other,
                })),
            },
            ErrorTemplate {
                id: "other_issue".to_string(),
                display_name: "Other issue".to_string(),
                severity: ErrorSeverity::Warning,
                causes: vec!["Other issue cause".to_string()],
                advices: vec!["Other issue advice".to_string()],
                condition: ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                    issue: Issue::OutOfMemory,
                })),
            },
        ];

        let system_state = SystemStateSnapshot {
            system_summary: SystemSummary {
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                disk_utilizations: vec![],
            },
            tool_run_summaries: vec![],
            workspace_files: &HashMap::new(),
            stdout_lines: &vec![],
            stderr_lines: &vec![],
            syslog_lines: &vec![],
            found_issues: &vec![IssueEntry {
                timestamp: 0,
                issue: Issue::Other,
            }],
        };

        let error_recognition = ErrorRecognition::new(&templates);

        let errors = error_recognition.recognize_errors(system_state);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].id, "basic_issue");
        assert_eq!(errors[0].display_name, "Basic issue");
        assert_eq!(errors[0].severity, ErrorSeverity::Warning);
        assert_eq!(errors[0].causes, vec!["Basic issue cause".to_string()]);
        assert_eq!(errors[0].advices, vec!["Basic issue advice".to_string()]);
    }

    #[test]
    fn test_recognize_and_or_not() {
        let templates: Vec<ErrorTemplate> = vec![
            create_test_template(
                "and_issue".to_string(),
                ErrorCondition::And(vec![
                    ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                        issue: Issue::Other,
                    })),
                    ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                        issue: Issue::OutOfMemory,
                    })),
                ]),
            ),
            create_test_template(
                "or_issue".to_string(),
                ErrorCondition::Or(vec![
                    ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                        issue: Issue::Other,
                    })),
                    ErrorCondition::ExternalTrigger(Box::new(IssueCondition {
                        issue: Issue::OutOfMemory,
                    })),
                ]),
            ),
            create_test_template(
                "not_issue".to_string(),
                ErrorCondition::Not(Box::new(ErrorCondition::ExternalTrigger(Box::new(
                    IssueCondition {
                        issue: Issue::Other,
                    },
                )))),
            ),
        ];

        let system_state = SystemStateSnapshot {
            system_summary: SystemSummary {
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                disk_utilizations: vec![],
            },
            tool_run_summaries: vec![],
            workspace_files: &HashMap::new(),
            stdout_lines: &vec![],
            stderr_lines: &vec![],
            syslog_lines: &vec![],
            found_issues: &vec![IssueEntry {
                timestamp: 0,
                issue: Issue::OutOfMemory,
            }],
        };

        let error_recognition = ErrorRecognition::new(&templates);

        let errors = error_recognition.recognize_errors(system_state);

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].id, "or_issue");
        assert_eq!(errors[1].id, "not_issue");
    }

    #[test]
    fn test_recognize_system_cpu_memory() {
        let templates = vec![
            create_test_template(
                "high_cpu".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(SystemCPUCondition { threshold: 0.8 })),
            ),
            create_test_template(
                "high_memory".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(SystemMemoryCondition { threshold: 0.3 })),
            ),
            create_test_template(
                "even_higher_cpu".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(SystemCPUCondition { threshold: 0.9 })),
            ),
            create_test_template(
                "even_higher_memory".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(SystemMemoryCondition { threshold: 0.4 })),
            ),
        ];

        let system_state = SystemStateSnapshot {
            system_summary: SystemSummary {
                cpu_utilization: 0.85,
                memory_utilization: 0.35,
                disk_utilizations: vec![],
            },
            tool_run_summaries: vec![],
            workspace_files: &HashMap::new(),
            stdout_lines: &vec![],
            stderr_lines: &vec![],
            syslog_lines: &vec![],
            found_issues: &vec![],
        };

        let error_recognition = ErrorRecognition::new(&templates);

        let errors = error_recognition.recognize_errors(system_state);

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].id, "high_cpu");
        assert_eq!(errors[1].id, "high_memory");
    }

    #[test]
    fn test_recognize_file_exists() {
        let templates = vec![
            create_test_template(
                "file_exists".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(FileExistsCondition::new(
                    "test_file.txt",
                ))),
            ),
            create_test_template(
                "file_does_not_exist".to_string(),
                ErrorCondition::ExternalTrigger(Box::new(FileExistsCondition::new(
                    "non_existent_file.txt",
                ))),
            ),
        ];

        let mut workspace_files = HashMap::new();
        workspace_files.insert(
            "test_file.txt".to_string(),
            FileInfo {
                name: "test_file.txt".to_string(),
                directory: "test_directory".to_string(),
                size: 0,
                last_update: chrono::offset::Utc::now(),
            },
        );

        let system_state = SystemStateSnapshot {
            system_summary: SystemSummary {
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                disk_utilizations: vec![],
            },
            tool_run_summaries: vec![],
            workspace_files: &workspace_files,
            stdout_lines: &vec![],
            stderr_lines: &vec![],
            syslog_lines: &vec![],
            found_issues: &vec![],
        };

        let error_recognition = ErrorRecognition::new(&templates);

        let errors = error_recognition.recognize_errors(system_state);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].id, "file_exists");
    }
}
