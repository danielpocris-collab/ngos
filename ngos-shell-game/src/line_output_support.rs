use alloc::string::ToString;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

pub struct StackLineBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> StackLineBuffer<N> {
    pub fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub fn push_byte(&mut self, byte: u8) -> Result<(), ExitCode> {
        if self.len == N {
            return Err(190);
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    pub fn push_str(&mut self, text: &str) -> Result<(), ExitCode> {
        for byte in text.as_bytes() {
            self.push_byte(*byte)?;
        }
        Ok(())
    }

    pub fn push_bool(&mut self, value: bool) -> Result<(), ExitCode> {
        if value {
            self.push_str("true")
        } else {
            self.push_str("false")
        }
    }

    pub fn push_u64(&mut self, value: u64) -> Result<(), ExitCode> {
        self.push_str(&value.to_string())
    }

    pub fn push_i32(&mut self, value: i32) -> Result<(), ExitCode> {
        self.push_str(&value.to_string())
    }

    pub fn push_usize(&mut self, value: usize) -> Result<(), ExitCode> {
        self.push_str(&value.to_string())
    }
}
