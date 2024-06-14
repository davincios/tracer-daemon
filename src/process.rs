use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use sysinfo::{Pid, System};

const CONFIG_PATH: &str = "~/.config/tracer-daemon/tracer.toml";

#[derive(Deserialize)]
struct ConfigFile {
    api_key: String,
    targets: Vec<String>,
}

pub struct TracerClient {
    api_key: String,
    targets: Vec<String>,
    seen: HashMap<Pid, String>,
    system: System,
    service_url: String,
}

impl TracerClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config: ConfigFile = toml::from_str(&std::fs::read_to_string(CONFIG_PATH)?)?;

        Ok(Self {
            api_key: config.api_key,
            targets: config.targets,
            seen: HashMap::new(),
            system: System::new_all(),
            service_url: "".to_owned(),
        })
    }

    // pub fn remove_stale(sys: &mut System, seen: &mut HashMap<Pid, String>) {
    pub fn remove_stale(&mut self) {
        let mut to_remove = vec![];
        // find processes that exited
        for (pid, p_name) in self.seen.iter() {
            if !self.system.processes().contains_key(&pid) {
                println!("[{}] {} exited", Utc::now(), p_name);
                to_remove.push(pid.clone());
            }
        }
        // cleanup exited processes
        for i in to_remove.iter() {
            self.seen.remove(i);
        }
    }

    pub fn poll_processes(&mut self) {
        for (pid, proc) in self.system.processes().iter() {
            if !self.seen.contains_key(pid) && self.targets.contains(&proc.name().to_string()) {
                self.seen.insert(*pid, proc.name().to_string());
                println!("[{}] {} is running", Utc::now(), proc.name());
            }
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    /// Sends current load of a system to the server
    pub async fn send_global_stat(&self) -> Result<(), Box<dyn std::error::Error>> {
        // let mut data = json!({
        //     "logs": [{
        //         "message": message,
        //         "event_type": "process_status",
        //         "process_type": "pipeline",
        //         "process_status": process_status,
        //         "api_key": self.api_key,
        //         "attributes": attributes // Add attributes if provided
        //     }]
        // });

        Client::new()
            .post(&self.service_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json("TODO")
            .send()
            .await?;

        Ok(())
    }

    // Sends current resource consumption of target processes to the server
    // pub async fn send_proc_stat(&self) -> Result<(), Box<dyn std::error::Error>> {}
}
