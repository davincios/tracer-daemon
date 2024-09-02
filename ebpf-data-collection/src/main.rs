#![no_std]
#![no_main]

use core::hash::{Hash, Hasher};

use aya_ebpf::{
    helpers::{bpf_probe_read_user, bpf_probe_read_user_str_bytes},
    macros::{map, tracepoint},
    maps::{HashMap, PerfEventArray},
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use fnv::FnvHasher;

#[repr(C)]
struct ExecveArgs {
    unused: i32, // common_type(offset: 0, size: 2, signed: 0) + common_flags(offset: 2, size: 1, signed: 0) + common_preempt_count
    pid: i32,    // common_pid;	offset:4;	size:4;	signed:1;
    unused2: u64, // int __syscall_nr;	offset:8;	size:4;	signed:1; + extra size to get next field to offet 16
    filename_ptr: u64, // const char * filename;	offset:16;	size:8;	signed:0;
    argv_ptr: u64,
    envp_ptr: u64,
}

#[repr(C)]
pub struct ProcessData {
    pub comm: [u8; 64],
    pub args: [u8; 128],
    pub len: usize,
}

#[map(name = "EVENTS")]
static mut EVENTS: PerfEventArray<ProcessData> = PerfEventArray::with_max_entries(1024, 0);

#[map] //
static WATCHLIST: HashMap<u64, u8> = HashMap::<u64, u8>::with_max_entries(1024, 0);

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
    let mut filename = [0u8; 64];
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

    let mut hasher = FnvHasher::default();
    let mut found: i32 = -1;
    let mut index = len;
    while index > 0 {
        index -= 1;

        let val = filename[index];

        if val == b'/' || val == b'\\' {
            found = index as i32;
            break;
        }

        val.hash(&mut hasher);
    }

    if found == -1 {
        return Ok(0);
    }

    let found = found as usize;

    if found >= len - 2 {
        return Ok(0);
    }

    let hashed = hasher.finish();

    unsafe {
        if WATCHLIST.get(&hashed).is_none() {
            // not in the watch list, exit early
            return Ok(0);
        }
    }

    let argv_ptr = args.argv_ptr as *const *const u8;
    let mut arg_index: usize = 0;
    let mut arg_list = [0u8; 128];
    for i in 0..8 {
        let len = unsafe {
            let arg_ptr = bpf_probe_read_user(argv_ptr.add(i as usize)).unwrap();
            if arg_ptr.is_null() {
                break;
            }

            bpf_probe_read_user_str_bytes(arg_ptr, &mut arg_list[arg_index..])
                .map_err(|er| {
                    info!(&ctx, "failed to read kernel arg string: {}", er);

                    3u32
                })?
                .len()
        };

        arg_index += len;

        if arg_index >= arg_list.len() - 1 {
            break;
        }

        if i < 7 {
            arg_list[arg_index] = b' ';
            arg_index += 1;
        }
    }

    let data = ProcessData {
        comm: filename,
        args: arg_list,
        len,
    };

    unsafe {
        EVENTS.output(&ctx, &data, 0);
    }

    // everything after is extra, can be removed

    let start_ptr = unsafe { filename.as_ptr().add(found + 1) };
    let binary_slice_len = len - found - 1;
    let binary_slice = unsafe { core::slice::from_raw_parts(start_ptr, binary_slice_len) };
    let binary_name = unsafe { core::str::from_utf8_unchecked(binary_slice) };

    info!(&ctx, "read kernel string");

    info!(&ctx, "execve called with filename: {}", binary_name);

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
