mod process;

extern crate daemonize;

use daemonize::Daemonize;
use process::*;
use std::fs::File;
use tokio::time::Duration;

const DEBOUNCE_SECS: u64 = 3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tr = TracerClient::new()?;

    loop {
        tr.remove_stale();
        tr.poll_processes();

        tr.send_global_stat().await?;
        // TODO: commented until backend would be able to handle it
        // tr.send_proc_stat().await?;

        tr.refresh();
        tokio::time::sleep(Duration::from_secs(DEBOUNCE_SECS)).await;
    }
}
