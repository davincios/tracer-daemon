use anyhow::{Context, Result};
use aya::programs::TracePoint;
use aya::{include_bytes_aligned, Bpf};
use aya_log::BpfLogger;
use log::warn;

pub fn initialize() -> Result<()> {
    #[cfg(debug_assertions)]
    let mut bpf = Bpf::load(include_bytes_aligned!(
        "./bpfel-unknown-none/debug/ebpf-data-collection"
    ))?;
    #[cfg(not(debug_assertions))]
    let mut bpf = Bpf::load(include_bytes_aligned!(
        "./bpfel-unknown-none/release/ebpf-data-collection"
    ))?;

    if let Err(e) = BpfLogger::init(&mut bpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {}", e);
    }

    let program: &mut TracePoint = bpf
        .program_mut("watch")
        .unwrap()
        .try_into()
        .context("failed to find the ebpf program")?;

    program.load().context("failed to load ebpf program")?;

    program
        .attach("syscalls", "sys_enter_execve")
        .context("failed to attach the ebpf program")?;

    Ok(())
}
