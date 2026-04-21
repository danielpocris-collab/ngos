//! AMD User Mode Driver (UMD) for RDNA 3/4/5.
//! Translates high-level `DrawOp` commands into PM4 packet streams.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::render_command_agent::{DrawOp, RgbaColor};

pub const PM4_TYPE_3: u32 = 3;
pub fn pm4_header(opcode: u8, count: u16) -> u32 {
    (PM4_TYPE_3 << 30) | (((count as u32) & 0x3FFF) << 16) | (((opcode as u32) & 0xFF) << 8)
}

pub struct AmdUmdContext {
    pub pushbuffer: Vec<u32>,
    pub framebuffer_paddr: u64,
}

impl AmdUmdContext {
    pub fn new(framebuffer_paddr: u64) -> Self {
        Self {
            pushbuffer: Vec::new(),
            framebuffer_paddr,
        }
    }

    pub fn emit_packet(&mut self, opcode: u8, body: &[u32]) {
        self.pushbuffer.push(pm4_header(opcode, body.len() as u16));
        for &word in body {
            self.pushbuffer.push(word);
        }
    }

    pub fn translate_draw_op(&mut self, op: &DrawOp) {
        match op {
            DrawOp::Clear { color } => {
                // AMD SDMA Fill (Opcode 0x03 in SDMA 6.0)
                // Nota: In PM4 am folosi DRAW_RECT_FILL sau o comanda de Compute
                let rgba = ((color.a as u32) << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                
                // Exemplu pachet PM4: SET_CONFIG_REG (0x28) pentru Color Buffer
                self.emit_packet(0x28, &[0x0000A000, rgba]); 
            }
            _ => {}
        }
    }

    pub fn serialize_to_hex(&self) -> String {
        self.pushbuffer.iter().map(|word| format!("{:08x}", word)).collect::<Vec<_>>().join(" ")
    }
}
