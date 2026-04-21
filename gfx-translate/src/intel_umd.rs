//! Intel User Mode Driver (UMD) for Xe/Xe2.
//! Translates high-level `DrawOp` commands into Intel Batch Buffers.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::render_command_agent::{DrawOp, RgbaColor};

// Intel Xe Command Opcodes
pub const MI_NOOP: u32 = 0x00000000;
pub const MI_BATCH_BUFFER_END: u32 = 0x05000000;
pub const XY_COLOR_BLT: u32 = 0x50400000; // 2D Fill/Clear

pub struct IntelUmdContext {
    pub batch_buffer: Vec<u32>,
    pub framebuffer_paddr: u64,
}

impl IntelUmdContext {
    pub fn new(framebuffer_paddr: u64) -> Self {
        Self {
            batch_buffer: Vec::new(),
            framebuffer_paddr,
        }
    }

    pub fn emit(&mut self, word: u32) {
        self.batch_buffer.push(word);
    }

    pub fn translate_draw_op(&mut self, op: &DrawOp) {
        match op {
            DrawOp::Clear { color } => {
                let rgba = ((color.a as u32) << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                
                // Intel XY_COLOR_BLT (6 words)
                self.emit(XY_COLOR_BLT | (6 - 2)); 
                self.emit(0x03 << 24 | 0xF0 << 16 | (1920 * 4) as u32); // ROP, Pitch
                self.emit(0); // Top-Left (0,0)
                self.emit(1080 << 16 | 1920); // Bottom-Right
                self.emit((self.framebuffer_paddr & 0xFFFFFFFF) as u32);
                self.emit(rgba);
            }
            _ => {}
        }
    }

    pub fn serialize_to_hex(&self) -> String {
        self.batch_buffer.iter().map(|word| format!("{:08x}", word)).collect::<Vec<_>>().join(" ")
    }
}
