use std::ops::ControlFlow;

use predicates::{prelude::predicate, str::RegexPredicate, Predicate};

use crate::system_state_manager::LogEntry;

use super::{Issue, SystemStateSnapshot, TriggerMetadata};

pub trait ErrorBaseCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata>;
}

pub struct FileExistsCondition {
    pub file_path: RegexPredicate,
}

impl ErrorBaseCondition for FileExistsCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        for file in system_state.workspace_files.keys() {
            if self.file_path.eval(file) {
                return Some(TriggerMetadata::new_file(file.clone()));
            }
        }
        None
    }
}

impl FileExistsCondition {
    pub fn new(file_path: &str) -> FileExistsCondition {
        FileExistsCondition {
            file_path: predicate::str::is_match(file_path).unwrap(),
        }
    }
}

pub struct ToolRunTimeGreaterThanCondition {
    pub tool_name: String,
    pub run_time: u64,
}

impl ErrorBaseCondition for ToolRunTimeGreaterThanCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        system_state
            .tool_run_summaries
            .iter()
            .find(|t| t.tool_name == self.tool_name && t.run_duration > self.run_time)
            .map(|tool_summary| TriggerMetadata::new_tool_run_summaries(tool_summary.clone()))
    }
}

pub struct ToolCPUUsageGreaterThanCondition {
    pub tool_name: String,
    pub threshold: f64,
}

impl ErrorBaseCondition for ToolCPUUsageGreaterThanCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        system_state
            .tool_run_summaries
            .iter()
            .find(|t| t.tool_name == self.tool_name && t.max_cpu_usage > self.threshold)
            .map(|tool_summary| TriggerMetadata::new_tool_run_summaries(tool_summary.clone()))
    }
}

pub struct ToolMemoryUsageGreaterThanCondition {
    pub tool_name: String,
    pub threshold: f64,
}

impl ErrorBaseCondition for ToolMemoryUsageGreaterThanCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        system_state
            .tool_run_summaries
            .iter()
            .find(|t| t.tool_name == self.tool_name && t.max_memory_utilization > self.threshold)
            .map(|tool_summary| TriggerMetadata::new_tool_run_summaries(tool_summary.clone()))
    }
}

pub struct LogContainsInner {
    pub regex: RegexPredicate,
}

impl LogContainsInner {
    pub fn new(regex: &str) -> LogContainsInner {
        LogContainsInner {
            regex: predicate::str::is_match(regex).unwrap(),
        }
    }

    pub fn trigger<'a>(&self, logs: &'a [LogEntry]) -> Option<&'a LogEntry> {
        logs.iter().find(|l| self.regex.eval(&l.message))
    }
}

pub enum LogContainsCondition {
    Stdout(LogContainsInner),
    Stderr(LogContainsInner),
    Syslog(LogContainsInner),
}

impl ErrorBaseCondition for LogContainsCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        match self {
            LogContainsCondition::Stdout(inner) => inner
                .trigger(system_state.stdout_lines)
                .map(|log| TriggerMetadata::new_stdout(log.clone())),
            LogContainsCondition::Stderr(inner) => inner
                .trigger(system_state.stderr_lines)
                .map(|log| TriggerMetadata::new_stderr(log.clone())),
            LogContainsCondition::Syslog(inner) => inner
                .trigger(system_state.syslog_lines)
                .map(|log| TriggerMetadata::new_syslog(log.clone())),
        }
    }
}

pub struct SystemCPUCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemCPUCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        if system_state.system_summary.cpu_utilization > self.threshold {
            Some(TriggerMetadata::default())
        } else {
            None
        }
    }
}

pub struct SystemMemoryCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemMemoryCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        if system_state.system_summary.memory_utilization > self.threshold {
            Some(TriggerMetadata::default())
        } else {
            None
        }
    }
}

pub struct SystemDiskUtilizationCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemDiskUtilizationCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        if system_state
            .system_summary
            .disk_utilizations
            .iter()
            .any(|d| *d > self.threshold)
        {
            Some(TriggerMetadata::default())
        } else {
            None
        }
    }
}

pub struct IssueCondition {
    pub issue: Issue,
}

impl ErrorBaseCondition for IssueCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        if system_state
            .found_issues
            .iter()
            .any(|i| i.issue == self.issue)
        {
            Some(TriggerMetadata::new_issue(self.issue))
        } else {
            None
        }
    }
}

pub enum ErrorCondition {
    ExternalTrigger(Box<dyn ErrorBaseCondition + Sync>),
    Or(Vec<ErrorCondition>),
    And(Vec<ErrorCondition>),
    Not(Box<ErrorCondition>),
}

impl ErrorCondition {
    pub fn trigger(&self, system_state: &SystemStateSnapshot) -> Option<TriggerMetadata> {
        match self {
            ErrorCondition::ExternalTrigger(condition) => condition.trigger(system_state),
            ErrorCondition::Or(conditions) => conditions.iter().fold(None, |acc, c| {
                if acc.is_some() {
                    acc
                } else if let Some(metadata) = c.trigger(system_state) {
                    Some(metadata)
                } else {
                    acc
                }
            }),
            ErrorCondition::And(conditions) => {
                if let ControlFlow::Continue(value) =
                    conditions
                        .iter()
                        .try_fold(None, |acc: Option<TriggerMetadata>, c| {
                            if let Some(metadata) = c.trigger(system_state) {
                                if let Some(mut acc_metadata) = acc {
                                    acc_metadata.merge(metadata);
                                    ControlFlow::Continue(Some(acc_metadata))
                                } else {
                                    ControlFlow::Continue(Some(metadata))
                                }
                            } else {
                                ControlFlow::Break(None::<TriggerMetadata>)
                            }
                        })
                {
                    value
                } else {
                    None
                }
            }
            ErrorCondition::Not(condition) => {
                if condition.trigger(system_state).is_some() {
                    None
                } else {
                    Some(TriggerMetadata::default())
                }
            }
        }
    }
}

#[macro_export]
macro_rules! condition {
    ($condition:expr) => {
        ErrorCondition::ExternalTrigger(Box::new($condition) as Box<dyn ErrorBaseCondition + Sync>)
    };
}

#[macro_export]
macro_rules! boxed_condition {
    ($condition:expr) => {
        Box::new(ErrorCondition::ExternalTrigger(
            Box::new($condition) as Box<dyn ErrorBaseCondition + Sync>
        ))
    };
}

#[macro_export]
macro_rules! stdout_condition {
    ($stdout_text:literal) => {
        ErrorCondition::ExternalTrigger(Box::new(LogContainsCondition::Stdout(
            LogContainsInner::new($stdout_text),
        )) as Box<dyn ErrorBaseCondition + Sync>)
    };
}

#[macro_export]
macro_rules! stderr_condition {
    ($stderr_text:literal) => {
        ErrorCondition::ExternalTrigger(Box::new(LogContainsCondition::Stderr(
            LogContainsInner::new($stderr_text),
        )) as Box<dyn ErrorBaseCondition + Sync>)
    };
}

#[macro_export]
macro_rules! syslog_condition {
    ($syslog_test:literal) => {
        ErrorCondition::ExternalTrigger(Box::new(LogContainsCondition::Syslog(
            LogContainsInner::new($syslog_test),
        )) as Box<dyn ErrorBaseCondition + Sync>)
    };
}
