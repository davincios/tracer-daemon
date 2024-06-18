mod process;

extern crate daemonize;

use anyhow::Result;
use daemonize::Daemonize;
use process::*;
use std::fs::File;
use tokio::time::Duration;

const DEFAULT_CONFIG_PATH: &str = ".config/tracer/tracer.toml";

fn main() -> Result<()> {
    let daemonize = Daemonize::new()
        .pid_file("/tmp/tracerd.pid")
        .working_directory("/tmp")
        .stdout(File::create("/tmp/tracerd.out")?)
        .stderr(File::create("/tmp/tracerd.err")?);

    match daemonize.start() {
        Ok(_) => println!("tracer-daemon started"),
        Err(e) => eprintln!("Error, {}", e),
    }

    async_main()
}

#[tokio::main]
async fn async_main() -> Result<()> {
    let default_conf_path = format!("{}/{}", std::env::var("HOME")?, DEFAULT_CONFIG_PATH);

    let config: ConfigFile = toml::from_str(&std::fs::read_to_string(
        std::env::var("TRACER_CONFIG").unwrap_or(default_conf_path),
    )?)?;

    let interval = config.polling_interval_ms;

    let mut tr = TracerClient::from_config(config)?;

    loop {
        tr.remove_completed_processes().await?;
        tr.poll_processes().await?;

        tr.send_global_stat().await?;
        // TODO: commented until backend would be able to handle it
        // tr.send_proc_stat().await?;

        tr.refresh();
        tokio::time::sleep(Duration::from_millis(interval)).await;
    }
}
