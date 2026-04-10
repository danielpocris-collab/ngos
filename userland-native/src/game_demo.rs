#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", feature(alloc_error_handler))]

#[cfg(target_os = "none")]
extern crate alloc;

#[cfg(target_os = "none")]
use alloc::string::String;
#[cfg(target_os = "none")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(target_os = "none")]
use core::ptr;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(target_os = "none")]
use ngos_gfx_translate::{DrawOp, FrameScript, RgbaColor};
#[cfg(target_os = "none")]
use ngos_user_abi::{BootstrapArgs, ExitCode};
#[cfg(target_os = "none")]
use ngos_user_runtime::{Amd64SyscallBackend, Runtime};

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
struct NullAllocator;

#[cfg(target_os = "none")]
const USER_HEAP_SIZE: usize = 1024 * 1024;

#[cfg(target_os = "none")]
#[repr(align(16))]
struct UserHeap([u8; USER_HEAP_SIZE]);

#[cfg(target_os = "none")]
static mut USER_HEAP: UserHeap = UserHeap([0; USER_HEAP_SIZE]);

#[cfg(target_os = "none")]
static USER_HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "none")]
unsafe impl GlobalAlloc for NullAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(1);
        let size = layout.size().max(1);
        let heap_base = ptr::addr_of_mut!(USER_HEAP.0).cast::<u8>() as usize;
        let heap_end = heap_base + USER_HEAP_SIZE;
        let mut current = USER_HEAP_NEXT.load(Ordering::Acquire);
        loop {
            let aligned = (heap_base + current + align - 1) & !(align - 1);
            let Some(next) = aligned.checked_add(size) else {
                return ptr::null_mut();
            };
            if next > heap_end {
                return ptr::null_mut();
            }
            let next_offset = next - heap_base;
            match USER_HEAP_NEXT.compare_exchange(
                current,
                next_offset,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(observed) => current = observed,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(target_os = "none")]
#[global_allocator]
static GLOBAL_ALLOCATOR: NullAllocator = NullAllocator;

#[cfg(target_os = "none")]
#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
fn run_game_demo<B: ngos_user_abi::SyscallBackend>(
    runtime: &Runtime<B>,
    _bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    let _ = runtime.write(1, b"GameDemo: starting render loop\n");

    for i in 0..5 {
        let script = FrameScript {
            width: 1280,
            height: 720,
            frame_tag: String::from("demo-frame"),
            queue: String::from("graphics"),
            present_mode: String::from("mailbox"),
            completion: String::from("fire-and-forget"),
            ops: alloc::vec![DrawOp::Clear {
                color: RgbaColor {
                    r: (i * 40) as u8,
                    g: 100,
                    b: 200,
                    a: 255,
                },
            }],
        };

        let encoded = script.encode("perf-profile");
        let _ = runtime.present_gpu_frame("/dev/gpu0", encoded.payload.as_bytes());
        let _ = runtime.write(1, b"GameDemo: frame submitted\n");
    }

    let _ = runtime.write(1, b"GameDemo: finished successfully\n");
    0
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    _argc: usize,
    _argv: *const *const u8,
    _envp: *const *const u8,
    _sysret_reserved: usize,
    _stack_alignment: usize,
    _auxv: *const ngos_user_abi::AuxvEntry,
) -> ! {
    let runtime = Runtime::new(Amd64SyscallBackend);
    let argv = ["game-demo"];
    let envp: [&str; 0] = [];
    let auxv = [];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    runtime.start(&bootstrap, run_game_demo)
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
