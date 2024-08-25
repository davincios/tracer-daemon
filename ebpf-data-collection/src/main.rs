#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_probe_read_user_str_bytes,
    macros::{map, tracepoint},
    maps::{HashMap, PerfEventArray},
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

#[map] //
static WATCHLIST: HashMap<u32, u32> = HashMap::<u32, u32>::with_max_entries(1024, 0);

#[tracepoint]
pub fn watch(ctx: TracePointContext) -> u32 {
    try_tracerd(ctx).unwrap_or_default()
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

    let mut found: i32 = -1;
    let mut index = len;
    while index > 0 {
        index -= 1;

        let val = filename[index];
        if val == b'/' || val == b'\\' {
            found = index as i32;
            break;
        }
    }

    if found == -1 {
        return Ok(0);
    }

    let found = found as usize;

    if found >= len - 2 {
        return Ok(0);
    }

    let start_ptr = unsafe { filename.as_ptr().add(found + 1) };
    let binary_slice = unsafe { core::slice::from_raw_parts(start_ptr, len - found - 1) };
    let binary_name = unsafe { core::str::from_utf8_unchecked(binary_slice) };

    info!(&ctx, "read kernel string");

    let data = ProcessData {
        comm: filename,
        len,
    };

    unsafe {
        EVENTS.output(&ctx, &data, 0);
    }

    // PROCESS_EVENTS.push(&data, 0).map_err(|_| 3u32)?;

    info!(&ctx, "execve called with filename: {}", binary_name);

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
