use std::{path::PathBuf, sync::Arc};

use linemux::MuxedLines;
use predicates::{prelude::predicate, str::RegexPredicate, Predicate};
use serde::Serialize;
use serde_json::json;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use anyhow::Result;

use crate::{debug_log::Logger, http_client::send_http_body};

pub mod stderr_patterns;
pub mod stdout_patterns;
pub mod syslog_patterns;

pub struct IssueFindPattern {
    pub id: String,
    pub display_name: String,
    pub regex: RegexPredicate,
}

impl IssueFindPattern {
    pub fn new(id: String, display_name: String, regex: String) -> IssueFindPattern {
        IssueFindPattern {
            id,
            display_name,
            regex: predicate::str::is_match(regex).unwrap(),
        }
    }
}

const LINES_BEFORE: usize = 2;

#[derive(Serialize, Debug)]
pub struct IssueOutput {
    pub id: String,
    pub display_name: String,
    pub line_number: usize,
    pub lines_before: Vec<String>,
    pub line: String,
}

pub struct FileWatcherEntry {
    pub last_lines: Vec<String>,
    pub pending_lines: Arc<RwLock<Vec<String>>>,
    pub patterns: &'static Vec<IssueFindPattern>,
    pub file_path: PathBuf,
}

struct FileTailEntry {
    pub pending_lines: Arc<RwLock<Vec<String>>>,
    pub file_path: PathBuf,
}

impl FileWatcherEntry {
    pub fn new(
        file_path: PathBuf,
        patterns: &'static Vec<IssueFindPattern>,
        pending_lines: Arc<RwLock<Vec<String>>>,
    ) -> FileWatcherEntry {
        FileWatcherEntry {
            last_lines: Vec::new(),
            patterns,
            pending_lines,
            file_path,
        }
    }
}

pub struct FileContentWatcher {
    pub entries: Vec<FileWatcherEntry>,
}

impl FileContentWatcher {
    pub fn new() -> FileContentWatcher {
        FileContentWatcher { entries: vec![] }
    }

    pub fn add_entry(
        &mut self,
        file_path: PathBuf,
        patterns: &'static Vec<IssueFindPattern>,
        pending_lines: Arc<RwLock<Vec<String>>>,
    ) {
        self.entries
            .push(FileWatcherEntry::new(file_path, patterns, pending_lines));
    }

    async fn run_file_lines_read_thread(entries: Vec<FileTailEntry>) {
        let line_reader = MuxedLines::new();

        if line_reader.is_err() {
            return;
        }

        let logger = Logger::new();

        let mut line_reader = line_reader.unwrap();

        for entry in entries.iter() {
            let result = line_reader.add_file(entry.file_path.clone()).await;

            if result.is_err() {
                logger
                    .log(
                        &format!("Failed to add file to line reader: {:?}", entry.file_path),
                        None,
                    )
                    .await;
                continue;
            }
        }

        while let Ok(Some(line)) = line_reader.try_next().await {
            let source = line.source();
            for entry in entries.iter() {
                if source == entry.file_path {
                    let mut vec = entry.pending_lines.write().await;
                    let line = line.line();

                    vec.push(line.to_string());
                }
            }
        }
    }

    pub fn setup_thread(&self) -> tokio::task::JoinHandle<()> {
        let mut entries = Vec::new();
        for entry in self.entries.iter() {
            entries.push(FileTailEntry {
                pending_lines: entry.pending_lines.clone(),
                file_path: entry.file_path.clone(),
            });
        }

        tokio::spawn(Self::run_file_lines_read_thread(entries))
    }

    pub async fn poll_files_and_clear_buffers(&mut self) -> Result<Vec<IssueOutput>> {
        let mut issues: Vec<IssueOutput> = vec![];

        let logger = Logger::new();

        for entry in self.entries.iter_mut() {
            let mut lines = vec![];

            {
                let mut vec = entry.pending_lines.write().await;
                std::mem::swap(&mut lines, &mut vec);
            }

            for line in lines.iter() {
                for pattern in entry.patterns.iter() {
                    if pattern.regex.eval(line) {
                        let error_def = IssueOutput {
                            id: pattern.id.clone(),
                            display_name: pattern.display_name.clone(),
                            line_number: 0,
                            lines_before: Vec::new(),
                            line: line.clone(),
                        };

                        logger
                            .log(
                                &format!("Found error at {:?}: {:?}", entry.file_path, &error_def),
                                None,
                            )
                            .await;

                        issues.push(error_def);
                    }
                }

                entry.last_lines.push(line.clone());

                if entry.last_lines.len() > LINES_BEFORE {
                    entry.last_lines.remove(0);
                }
            }
        }

        Ok(issues)
    }

    pub async fn send_lines_to_endpoint(
        endpoint_url: &str,
        api_key: &str,
        pending_lines: &Arc<RwLock<Vec<String>>>,
        is_error: bool,
    ) -> Result<()> {
        let logger = Logger::new();

        if pending_lines.read().await.is_empty() {
            logger.log("No lines from stdout to send", None).await;
            return Ok(());
        }

        let body = json!({
            "lines": *pending_lines.as_ref().read().await,
            "isError": is_error
        });

        logger
            .log(&format!("Sending stdout lines: {:?}", body), None)
            .await;

        pending_lines.write().await.clear();

        send_http_body(endpoint_url, api_key, &body).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use super::*;
    use crate::debug_log::Logger;

    #[tokio::test]
    async fn test_grep_errors() {
        let test_file_path = "test-files/var/log/syslog";

        let file = File::open(test_file_path).unwrap();

        let file_lines = BufReader::new(file).lines();

        let lines = file_lines.map(|x| x.unwrap()).collect::<Vec<String>>();

        let mut file_content_watcher = FileContentWatcher::new();

        let patterns = &syslog_patterns::SYSLOG_PATTERNS;

        let pending_lines = Arc::new(RwLock::new(lines));

        file_content_watcher.add_entry(
            PathBuf::from(test_file_path),
            patterns,
            pending_lines.clone(),
        );

        match file_content_watcher.poll_files_and_clear_buffers().await {
            Ok(errors) => {
                let logger = Logger::new();

                let _ = logger
                    .log(
                        "grep_out_of_memory_errors",
                        Some(&serde_json::json!({
                            "errors": errors,
                        })),
                    )
                    .await;
            }
            Err(e) => eprintln!("Error occurred: {}", e),
        }
    }
}
