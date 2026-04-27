#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootInfoHeader {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub arch: BootArch,
    pub flags: u64,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum BootArch {
    X86_64 = 1,
    AArch64 = 2,
    RiscV64 = 3,
}
