mod bashrc_intercept;
mod config;
pub mod target_process;
pub use bashrc_intercept::INTERCEPTOR_STDOUT_FILE;
pub use config::{Config, ConfigManager};
