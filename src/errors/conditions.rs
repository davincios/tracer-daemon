use predicates::{prelude::predicate, str::RegexPredicate, Predicate};

use super::{Issue, SystemStateSnapshot};

pub trait ErrorBaseCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool;
}

pub struct FileExistsCondition {
    pub file_path: RegexPredicate,
}

impl ErrorBaseCondition for FileExistsCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        for file in system_state.workspace_files.keys() {
            if self.file_path.eval(file) {
                return true;
            }
        }
        false
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
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state
            .tool_run_summaries
            .iter()
            .any(|t| t.tool_name == self.tool_name && t.run_duration > self.run_time)
    }
}

pub struct ToolCPUUsageGreaterThanCondition {
    pub tool_name: String,
    pub threshold: f64,
}

impl ErrorBaseCondition for ToolCPUUsageGreaterThanCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state
            .tool_run_summaries
            .iter()
            .any(|t| t.tool_name == self.tool_name && t.max_cpu_usage > self.threshold)
    }
}

pub struct ToolMemoryUsageGreaterThanCondition {
    pub tool_name: String,
    pub threshold: f64,
}

impl ErrorBaseCondition for ToolMemoryUsageGreaterThanCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state
            .tool_run_summaries
            .iter()
            .any(|t| t.tool_name == self.tool_name && t.max_memory_utilization > self.threshold)
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

    pub fn trigger(&self, logs: &[String]) -> bool {
        logs.iter().any(|l| self.regex.eval(l))
    }
}

pub enum LogContainsCondition {
    Stdout(LogContainsInner),
    Stderr(LogContainsInner),
    Syslog(LogContainsInner),
}

impl ErrorBaseCondition for LogContainsCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        match self {
            LogContainsCondition::Stdout(inner) => inner.trigger(system_state.stdout_lines),
            LogContainsCondition::Stderr(inner) => inner.trigger(system_state.stderr_lines),
            LogContainsCondition::Syslog(inner) => inner.trigger(system_state.syslog_lines),
        }
    }
}

pub struct SystemCPUCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemCPUCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state.system_summary.cpu_utilization > self.threshold
    }
}

pub struct SystemMemoryCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemMemoryCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state.system_summary.memory_utilization > self.threshold
    }
}

pub struct SystemDiskUtilizationCondition {
    pub threshold: f64,
}

impl ErrorBaseCondition for SystemDiskUtilizationCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state
            .system_summary
            .disk_utilizations
            .iter()
            .any(|d| *d > self.threshold)
    }
}

pub struct IssueCondition {
    pub issue: Issue,
}

impl ErrorBaseCondition for IssueCondition {
    fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        system_state.found_issues.iter().any(|i| i == &self.issue)
    }
}

pub enum ErrorCondition {
    ExternalTrigger(Box<dyn ErrorBaseCondition + Sync>),
    Or(Vec<ErrorCondition>),
    And(Vec<ErrorCondition>),
    Not(Box<ErrorCondition>),
}

impl ErrorCondition {
    pub fn trigger(&self, system_state: &SystemStateSnapshot) -> bool {
        match self {
            ErrorCondition::ExternalTrigger(condition) => condition.trigger(system_state),
            ErrorCondition::Or(conditions) => conditions.iter().any(|c| c.trigger(system_state)),
            ErrorCondition::And(conditions) => conditions.iter().all(|c| c.trigger(system_state)),
            ErrorCondition::Not(condition) => !condition.trigger(system_state),
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
