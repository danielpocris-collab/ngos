use super::*;
use ngos_user_abi::Errno;

const USER_ADDRESS_LIMIT_EXCLUSIVE: u64 = 0x0001_0000_0000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserMemoryAccessError {
    InvalidPid,
    Fault,
}

impl UserMemoryAccessError {
    pub const fn errno(self) -> Errno {
        match self {
            Self::InvalidPid => Errno::Srch,
            Self::Fault => Errno::Fault,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserRange {
    start: u64,
    len: usize,
    end: u64,
}

impl KernelRuntime {
    pub fn validate_user_pointer(
        &self,
        pid: ProcessId,
        ptr: usize,
        len: usize,
        write: bool,
    ) -> Result<(), UserMemoryAccessError> {
        let range = user_range(ptr, len)?;
        if len == 0 {
            self.processes
                .get(pid)
                .map_err(|_| UserMemoryAccessError::InvalidPid)?;
            return Ok(());
        }

        let space = self
            .processes
            .get_process_address_space(pid)
            .map_err(map_process_error_to_user_memory_error)?;
        let mut cursor = range.start;
        for region in space.memory_map() {
            if region.end <= cursor {
                continue;
            }
            if region.start > cursor {
                return Err(UserMemoryAccessError::Fault);
            }
            if write && !region.writable {
                return Err(UserMemoryAccessError::Fault);
            }
            if !write && !region.readable {
                return Err(UserMemoryAccessError::Fault);
            }
            cursor = core::cmp::min(range.end, region.end);
            if cursor == range.end {
                return Ok(());
            }
        }
        Err(UserMemoryAccessError::Fault)
    }

    pub fn copy_from_user(
        &mut self,
        pid: ProcessId,
        ptr: usize,
        len: usize,
    ) -> Result<Vec<u8>, UserMemoryAccessError> {
        self.validate_user_pointer(pid, ptr, len, false)?;
        if len == 0 {
            return Ok(Vec::new());
        }
        let range = user_range(ptr, len)?;
        let (touch_start, touch_len) = touch_window(range.start, range.end);
        self.touch_memory(pid, touch_start, touch_len, false)
            .map_err(map_runtime_error_to_user_memory_error)?;

        let mut out = Vec::with_capacity(len);
        for offset in 0..len {
            let addr = range.start + offset as u64;
            let aligned = addr & !0x3;
            let shift = ((addr & 0x3) * 8) as u32;
            let word = self
                .load_memory_word(pid, aligned)
                .map_err(map_runtime_error_to_user_memory_error)?;
            out.push(((word >> shift) & 0xff) as u8);
        }
        Ok(out)
    }

    pub fn copy_to_user(
        &mut self,
        pid: ProcessId,
        ptr: usize,
        bytes: &[u8],
    ) -> Result<(), UserMemoryAccessError> {
        self.validate_user_pointer(pid, ptr, bytes.len(), true)?;
        if bytes.is_empty() {
            return Ok(());
        }
        let range = user_range(ptr, bytes.len())?;
        let (touch_start, touch_len) = touch_window(range.start, range.end);
        self.touch_memory(pid, touch_start, touch_len, true)
            .map_err(map_runtime_error_to_user_memory_error)?;

        for (offset, byte) in bytes.iter().copied().enumerate() {
            let addr = range.start + offset as u64;
            let aligned = addr & !0x3;
            let shift = ((addr & 0x3) * 8) as u32;
            let mut word = self
                .load_memory_word(pid, aligned)
                .map_err(map_runtime_error_to_user_memory_error)?;
            word &= !(0xff << shift);
            word |= (byte as u32) << shift;
            self.store_memory_word(pid, aligned, word)
                .map_err(map_runtime_error_to_user_memory_error)?;
        }
        Ok(())
    }
}

fn user_range(ptr: usize, len: usize) -> Result<UserRange, UserMemoryAccessError> {
    if len == 0 {
        let start = ptr as u64;
        if start >= USER_ADDRESS_LIMIT_EXCLUSIVE {
            return Err(UserMemoryAccessError::Fault);
        }
        return Ok(UserRange {
            start,
            len,
            end: start,
        });
    }
    if ptr == 0 {
        return Err(UserMemoryAccessError::Fault);
    }
    let start = ptr as u64;
    if start >= USER_ADDRESS_LIMIT_EXCLUSIVE {
        return Err(UserMemoryAccessError::Fault);
    }
    let end = start
        .checked_add(len as u64)
        .ok_or(UserMemoryAccessError::Fault)?;
    if end > USER_ADDRESS_LIMIT_EXCLUSIVE {
        return Err(UserMemoryAccessError::Fault);
    }
    Ok(UserRange { start, len, end })
}

fn map_process_error_to_user_memory_error(error: ProcessError) -> UserMemoryAccessError {
    match error {
        ProcessError::InvalidPid | ProcessError::StalePid => UserMemoryAccessError::InvalidPid,
        ProcessError::InvalidTid
        | ProcessError::StaleTid
        | ProcessError::InvalidMemoryLayout
        | ProcessError::MemoryQuarantined { .. }
        | ProcessError::InvalidSignal
        | ProcessError::InvalidSessionReport
        | ProcessError::InvalidTransition { .. }
        | ProcessError::NotExited
        | ProcessError::Exhausted => UserMemoryAccessError::Fault,
    }
}

fn map_runtime_error_to_user_memory_error(error: RuntimeError) -> UserMemoryAccessError {
    match error {
        RuntimeError::Process(process) => map_process_error_to_user_memory_error(process),
        _ => UserMemoryAccessError::Fault,
    }
}

fn touch_window(start: u64, end: u64) -> (u64, u64) {
    let first_page = start & !0xfff;
    let last_touched = end.saturating_sub(1);
    let last_page_end = (last_touched & !0xfff).saturating_add(0x1000);
    (first_page, last_page_end.saturating_sub(first_page))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_process_with_rw_mapping() -> (KernelRuntime, ProcessId, u64) {
        let mut runtime = KernelRuntime::host_runtime_default();
        let pid = runtime
            .spawn_process("user", None, SchedulerClass::LatencyCritical)
            .unwrap();
        let start = runtime
            .map_anonymous_memory(pid, 0x1000, true, true, false, "user-buf")
            .unwrap();
        (runtime, pid, start)
    }

    #[test]
    fn validate_user_pointer_rejects_null_and_unmapped() {
        let (runtime, pid, start) = setup_process_with_rw_mapping();
        assert_eq!(
            runtime.validate_user_pointer(pid, 0, 1, false),
            Err(UserMemoryAccessError::Fault)
        );
        assert_eq!(
            runtime.validate_user_pointer(pid, (start as usize) + 0x2000, 4, false),
            Err(UserMemoryAccessError::Fault)
        );
    }

    #[test]
    fn copy_from_and_copy_to_user_roundtrip_bytes() {
        let (mut runtime, pid, start) = setup_process_with_rw_mapping();
        let addr = start as usize;
        runtime.copy_to_user(pid, addr, b"ngos").unwrap();
        let bytes = runtime.copy_from_user(pid, addr, 4).unwrap();
        assert_eq!(bytes, b"ngos");
    }

    #[test]
    fn copy_to_user_fails_on_unmapped_range() {
        let (mut runtime, pid, start) = setup_process_with_rw_mapping();
        let err = runtime
            .copy_to_user(pid, (start as usize) + 0x1000 - 1, b"ab")
            .unwrap_err();
        assert_eq!(err, UserMemoryAccessError::Fault);
    }
}
