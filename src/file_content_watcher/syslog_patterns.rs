use crate::file_content_watcher::IssueFindPattern;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SYSLOG_PATTERNS: Vec<IssueFindPattern> = vec![IssueFindPattern::new(
        "OUT_OF_MEMORY".to_string(),
        "Out of memory".to_string(),
        "(?i)Out of memory".to_string()
    )];
}
