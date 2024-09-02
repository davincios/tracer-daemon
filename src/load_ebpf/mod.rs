use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyhow::Result;
use aya::maps::{AsyncPerfEventArray, HashMap};
use aya::programs::TracePoint;
use aya::util::online_cpus;
use aya::{include_bytes_aligned, Bpf, Pod};
use aya_log::BpfLogger;
use fnv::FnvHasher;
use log::{debug, info, warn};
use tokio::sync::RwLock;
use tokio_util::bytes::BytesMut;
use tokio_util::sync::CancellationToken;

use crate::config_manager;
use crate::config_manager::target_process::target_matching::TargetMatch;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProcessData {
    pub comm: [u8; 64],
    pub args: [u8; 128],
    pub len: usize,
}

unsafe impl Pod for ProcessData {}

pub async fn initialize(
    cancellation: CancellationToken,
    config: Arc<RwLock<config_manager::Config>>,
) -> Result<Bpf> {
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

    let mut perf_array = AsyncPerfEventArray::try_from(bpf.take_map("EVENTS").unwrap())?;

    let allowed = {
        let config = config.read().await;

        config
            .targets
            .iter()
            .map(|t| match &t.match_type {
                TargetMatch::ProcessName(name) => name.clone(),
                TargetMatch::ShortLivedProcessExecutable(name) => name.clone(),
                TargetMatch::CommandContains(c) => c.process_name.clone().unwrap(),
                TargetMatch::BinPathStartsWith(name) => name.clone(),
            })
            .collect::<Vec<String>>()
    };

    allowed.iter().for_each(|n| {
        info!("tracking: {}", n);
    });

    let mut allowlist: HashMap<_, u64, u8> = HashMap::try_from(bpf.take_map("WATCHLIST").unwrap())?;

    for val in allowed {
        let mut hasher = FnvHasher::default();
        for v in val.as_bytes().iter().rev() {
            v.hash(&mut hasher);
        }

        let hashed = hasher.finish();
        allowlist.insert(hashed, 1, 0)?;
    }

    let cpu_len = online_cpus()?.len();
    for cpu_id in online_cpus()? {
        let mut perf_fd = perf_array.open(cpu_id, Some(256))?;

        let cancel = cancellation.clone();
        tokio::spawn(async move {
            let mut buffers = (0..cpu_len)
                .map(|_| BytesMut::with_capacity(10240))
                .collect::<Vec<_>>();

            while !cancel.is_cancelled() {
                let events = perf_fd.read_events(&mut buffers).await.unwrap();
                for i in 0..events.read {
                    let buf = &mut buffers[i];
                    let ptr = buf.as_ptr() as *const ProcessData;
                    let data = unsafe { ptr.read_unaligned() };
                    let filename =
                        std::str::from_utf8(&data.comm[..data.len]).unwrap_or("Invalid UTF-8");
                    let args = std::str::from_utf8(&data.args).unwrap_or("Invalid UTF-8 in args");
                    info!("running: {} with args: {}", filename, args);
                }
            }
        });
    }

    Ok(bpf)
}
