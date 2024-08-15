use anyhow::Result;
use aya::programs::TracePoint;
use aya::{include_bytes_aligned, Bpf};
use aya_log::BpfLogger;
use log::{debug, info, warn};

pub fn initialize() -> Result<Bpf> {
    info!("starting...");

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {}", ret);
    }

    #[cfg(debug_assertions)]
    let mut bpf = Bpf::load(include_bytes_aligned!(
        "../../ebpf-build/bpfel-unknown-none/debug/ebpf-data-collection"
    ))?;

    #[cfg(not(debug_assertions))]
    let mut bpf = Bpf::load(include_bytes_aligned!(concat!(
        "../../ebpf-build/bpfel-unknown-none/release/ebpf-data-collection"
    )))?;
    info!("found bpf...");
    if let Err(e) = BpfLogger::init(&mut bpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {}", e);
    }

    info!("initialized...");
    let program: &mut TracePoint = bpf.program_mut("watch").unwrap().try_into()?;
    info!("found program...");
    program.load()?;
    info!("loaded program...");
    program.attach("syscalls", "sys_enter_execve")?;
    info!("attached program...");

    Ok(bpf)
}
