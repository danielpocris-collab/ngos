#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", feature(alloc_error_handler))]

#[cfg(any(target_os = "none", test))]
extern crate alloc;

#[cfg(target_os = "none")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(any(target_os = "none", test))]
use core::cell::UnsafeCell;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use core::ptr;
#[cfg(any(target_os = "none", test))]
use core::slice;
#[cfg(any(target_os = "none", test))]
use core::str;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(target_os = "none")]
use ngos_user_abi::bootstrap::parse_boot_context;
#[cfg(any(target_os = "none", test))]
use ngos_user_abi::{AT_NULL, AuxvEntry, BootstrapArgs, ExitCode};
#[cfg(target_os = "none")]
use ngos_user_abi::{USER_DEBUG_MARKER_EXIT, USER_DEBUG_MARKER_START};
#[cfg(target_os = "none")]
use ngos_user_runtime::{Amd64SyscallBackend, Runtime};

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
        let heap_base = unsafe { ptr::addr_of_mut!(USER_HEAP.0).cast::<u8>() as usize };
        let heap_end = heap_base + USER_HEAP_SIZE;
        let mut current = USER_HEAP_NEXT.load(Ordering::Acquire);
        loop {
            let aligned = align_up_usize(heap_base + current, align);
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
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
    auxv: *const AuxvEntry,
    _stack_alignment: usize,
) -> ! {
    debug_break(USER_DEBUG_MARKER_START, argc as u64);
    debug_break(0x4e47_4f53_5553_5041, argv as u64);
    debug_break(0x4e47_4f53_5553_5042, envp as u64);
    debug_break(0x4e47_4f53_5553_5043, auxv as u64);

    let runtime = Runtime::new(Amd64SyscallBackend);
    debug_break(0x4e47_4f53_5553_5031, 0);
    let bootstrap = parse_bootstrap(argc, argv, envp, auxv).unwrap_or_else(|code| {
        debug_break(USER_DEBUG_MARKER_EXIT, code as u64);
        runtime.exit(code);
    });
    debug_break(0x4e47_4f53_5553_5032, bootstrap.argc as u64);
    let boot_context = if bootstrap.is_boot_mode() {
        parse_boot_context(&bootstrap).ok()
    } else {
        None
    };
    debug_break(0x4e47_4f53_5553_5033, boot_context.is_some() as u64);
    if let Some(context) = &boot_context {
        let _ = runtime.report_boot_session(
            ngos_user_abi::BootSessionStatus::Success,
            ngos_user_abi::BootSessionStage::Bootstrap,
            0,
            context
                .module_phys_end
                .saturating_sub(context.module_phys_start),
        );
        let _ = runtime.report_boot_session(
            ngos_user_abi::BootSessionStatus::Success,
            ngos_user_abi::BootSessionStage::NativeRuntime,
            0,
            context.entry,
        );
    }
    debug_break(0x4e47_4f53_5553_5034, 0);
    let code = ngos_userland_native::main(&runtime, &bootstrap);
    if let Some(context) = &boot_context {
        let status = if code == 0 {
            ngos_user_abi::BootSessionStatus::Success
        } else {
            ngos_user_abi::BootSessionStatus::Failure
        };
        let _ = runtime.report_boot_session(
            status,
            ngos_user_abi::BootSessionStage::Complete,
            code,
            context.module_len,
        );
    }

    debug_break(USER_DEBUG_MARKER_EXIT, code as u64);
    runtime.exit(code)
}

#[cfg(any(target_os = "none", test))]
const MAX_BOOTSTRAP_ARGS: usize = 16;
#[cfg(any(target_os = "none", test))]
const MAX_BOOTSTRAP_ENVP: usize = 32;
#[cfg(any(target_os = "none", test))]
const MAX_BOOTSTRAP_AUXV: usize = 32;

#[cfg(any(target_os = "none", test))]
struct BootstrapStorage {
    argv: UnsafeCell<[&'static str; MAX_BOOTSTRAP_ARGS]>,
    envp: UnsafeCell<[&'static str; MAX_BOOTSTRAP_ENVP]>,
    auxv: UnsafeCell<[AuxvEntry; MAX_BOOTSTRAP_AUXV]>,
}

#[cfg(any(target_os = "none", test))]
unsafe impl Sync for BootstrapStorage {}

#[cfg(any(target_os = "none", test))]
static BOOTSTRAP_STORAGE: BootstrapStorage = BootstrapStorage {
    argv: UnsafeCell::new([""; MAX_BOOTSTRAP_ARGS]),
    envp: UnsafeCell::new([""; MAX_BOOTSTRAP_ENVP]),
    auxv: UnsafeCell::new([AuxvEntry { key: 0, value: 0 }; MAX_BOOTSTRAP_AUXV]),
};

#[cfg(any(target_os = "none", test))]
fn parse_bootstrap<'a>(
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
    auxv: *const AuxvEntry,
) -> Result<BootstrapArgs<'a>, ExitCode> {
    #[cfg(target_os = "none")]
    debug_break(0x4e47_4f53_5553_5141, argc as u64);

    if argc > MAX_BOOTSTRAP_ARGS {
        return Err(120);
    }
    if argc != 0 && argv.is_null() {
        return Err(125);
    }
    if envp.is_null() {
        return Err(126);
    }
    if auxv.is_null() {
        return Err(127);
    }

    let argv_storage = BOOTSTRAP_STORAGE.argv.get();
    for index in 0..argc {
        #[cfg(target_os = "none")]
        debug_break(0x4e47_4f53_5553_5142, index as u64);
        let ptr = unsafe { argv.add(index).read() };
        #[cfg(target_os = "none")]
        debug_break(0x4e47_4f53_5553_5148, ptr as u64);
        if ptr.is_null() {
            return Err(121);
        }
        let value = unsafe { parse_c_string(ptr)? };
        #[cfg(target_os = "none")]
        debug_break(0x4e47_4f53_5553_5149, value.len() as u64);
        unsafe {
            (*argv_storage)[index] = value;
        }
    }
    #[cfg(target_os = "none")]
    debug_break(0x4e47_4f53_5553_5143, argc as u64);

    let env_storage = BOOTSTRAP_STORAGE.envp.get();
    let mut envc = 0usize;
    loop {
        if envc >= MAX_BOOTSTRAP_ENVP {
            return Err(122);
        }
        let ptr = unsafe { envp.add(envc).read() };
        if ptr.is_null() {
            break;
        }
        let value = unsafe { parse_c_string(ptr)? };
        unsafe {
            (*env_storage)[envc] = value;
        }
        envc += 1;
    }
    #[cfg(target_os = "none")]
    debug_break(0x4e47_4f53_5553_5145, envc as u64);

    let aux_storage = BOOTSTRAP_STORAGE.auxv.get();
    let mut auxc = 0usize;
    loop {
        if auxc >= MAX_BOOTSTRAP_AUXV {
            return Err(123);
        }
        #[cfg(target_os = "none")]
        debug_break(0x4e47_4f53_5553_5146, auxc as u64);
        let entry = unsafe { auxv.add(auxc).read() };
        if entry.key == AT_NULL && entry.value == 0 {
            break;
        }
        unsafe {
            (*aux_storage)[auxc] = entry;
        }
        auxc += 1;
    }
    #[cfg(target_os = "none")]
    debug_break(0x4e47_4f53_5553_5147, auxc as u64);

    let argv_values = unsafe { &(&*argv_storage)[..argc] };
    let env_values = unsafe { &(&*env_storage)[..envc] };
    let aux_values = unsafe { &(&*aux_storage)[..auxc] };

    Ok(BootstrapArgs::new(argv_values, env_values, aux_values))
}

#[cfg(any(target_os = "none", test))]
unsafe fn parse_c_string<'a>(ptr: *const u8) -> Result<&'a str, ExitCode> {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    str::from_utf8(bytes).map_err(|_| 124)
}

#[cfg(target_os = "none")]
const fn align_up_usize(value: usize, align: usize) -> usize {
    if align == 0 {
        value
    } else {
        let rem = value % align;
        if rem == 0 {
            value
        } else {
            value + (align - rem)
        }
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
fn debug_break(marker: u64, value: u64) {
    unsafe {
        core::arch::asm!(
            "mov rax, {marker}",
            "mov rdi, {value}",
            "int3",
            marker = in(reg) marker,
            value = in(reg) value,
            options(nostack)
        );
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    unsafe {
        core::arch::asm!("ud2", options(noreturn));
    }
}

#[cfg(target_os = "none")]
#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    debug_break(0x4e47_4f53_5553_414c, 0);
    unsafe {
        core::arch::asm!("ud2", options(noreturn));
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    let mut index = 0;
    while index < len {
        unsafe {
            *dst.add(index) = *src.add(index);
        }
        index += 1;
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(dst: *mut u8, value: i32, len: usize) -> *mut u8 {
    let byte = value as u8;
    let mut index = 0;
    while index < len {
        unsafe {
            *dst.add(index) = byte;
        }
        index += 1;
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    if core::ptr::eq(dst.cast_const(), src) || len == 0 {
        return dst;
    }
    if (dst as usize) < (src as usize) {
        let mut index = 0usize;
        while index < len {
            unsafe {
                *dst.add(index) = *src.add(index);
            }
            index += 1;
        }
    } else {
        let mut index = len;
        while index != 0 {
            index -= 1;
            unsafe {
                *dst.add(index) = *src.add(index);
            }
        }
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(lhs: *const u8, rhs: *const u8, len: usize) -> i32 {
    let mut index = 0;
    while index < len {
        let left = unsafe { *lhs.add(index) };
        let right = unsafe { *rhs.add(index) };
        if left != right {
            return left as i32 - right as i32;
        }
        index += 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ngos_user_abi::{AT_NULL, AT_PAGESZ, AuxvEntry, BOOT_ARG_FLAG, BOOT_ENV_MARKER};

    #[test]
    fn native_entry_parses_raw_bootstrap_inputs() {
        let argv0 = b"ngos-userland-native\0";
        let argv1 = b"--boot\0";
        let env0 = b"NGOS_BOOT=1\0";
        let argv = [argv0.as_ptr(), argv1.as_ptr(), core::ptr::null()];
        let envp = [env0.as_ptr(), core::ptr::null()];
        let auxv = [
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            AuxvEntry { key: 25, value: 1 },
            AuxvEntry {
                key: AT_NULL,
                value: 0,
            },
        ];

        let bootstrap = parse_bootstrap(2, argv.as_ptr(), envp.as_ptr(), auxv.as_ptr()).unwrap();
        assert_eq!(bootstrap.argc, 2);
        assert_eq!(bootstrap.argv, ["ngos-userland-native", BOOT_ARG_FLAG]);
        assert_eq!(bootstrap.envp, [BOOT_ENV_MARKER]);
        assert_eq!(
            bootstrap.auxv[0],
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096
            }
        );
        assert_eq!(bootstrap.auxv[1], AuxvEntry { key: 25, value: 1 });
    }
}
