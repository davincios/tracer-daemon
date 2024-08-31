use crate::file_content_watcher::IssueFindPattern;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STDOUT_PATTERNS: Vec<IssueFindPattern> = vec![];
}
