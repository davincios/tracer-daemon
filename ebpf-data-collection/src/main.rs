#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_probe_read_user_str_bytes,
    macros::{map, tracepoint},
    maps::{HashMap, PerfEventArray, Queue},
    programs::TracePointContext,
};
use aya_log_ebpf::info;

#[repr(C)]
struct ExecveArgs {
    unused: u64,
    unused2: u64,
    filename_ptr: u64,
    argv_ptr: u64,
    envp_ptr: u64,
}

#[repr(C)]
pub struct ProcessData {
    pub comm: [u8; 128],
    pub len: usize,
}

#[map(name = "EVENTS")]
static mut EVENTS: PerfEventArray<ProcessData> = PerfEventArray::with_max_entries(1024, 0);

#[map]
static PROCESS_EVENTS: Queue<ProcessData> = Queue::<ProcessData>::with_max_entries(1024, 0);

#[map] //
static WATCHLIST: HashMap<u32, u32> = HashMap::<u32, u32>::with_max_entries(1024, 0);

#[tracepoint]
pub fn watch(ctx: TracePointContext) -> u32 {
    match try_tracerd(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_tracerd(ctx: TracePointContext) -> Result<u32, u32> {
    info!(
        &ctx,
        "tracepoint sys_enter_execve called, getting file name"
    );

    let args: ExecveArgs = unsafe { ctx.read_at(0).map_err(|_| 2u32)? };
    let mut filename = [0u8; 128];
    let filename_ptr = args.filename_ptr as *const u8;

    info!(&ctx, "mapped exec args");

    let len = unsafe {
        bpf_probe_read_user_str_bytes(filename_ptr, &mut filename)
            .map_err(|er| {
                info!(&ctx, "failed to read kernel string: {}", er);

                3u32
            })?
            .len()
    };

    info!(&ctx, "read kernel string");

    let data = ProcessData {
        comm: filename,
        len,
    };

    unsafe {
        EVENTS.output(&ctx, &data, 0);
    }

    // PROCESS_EVENTS.push(&data, 0).map_err(|_| 3u32)?;

    unsafe {
        info!(
            &ctx,
            "execve called with filename: {}",
            core::str::from_utf8_unchecked(&filename)
        );
    }

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
