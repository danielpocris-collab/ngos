#![no_std]

extern crate alloc;

use core::fmt;

pub mod bootstrap;

pub const AMD64_USER_CODE_SELECTOR: u16 = 0x33;
pub const AMD64_USER_STACK_SELECTOR: u16 = 0x2b;
pub const AMD64_USER_RFLAGS: usize = 0x202;
pub const USER_DEBUG_MARKER_START: u64 = 0x4e47_4f53_5553_5254;
pub const USER_DEBUG_MARKER_MAIN: u64 = 0x4e47_4f53_5553_524d;
pub const USER_DEBUG_MARKER_EXIT: u64 = 0x4e47_4f53_5553_5258;

pub type ExitCode = i32;
pub type SyscallNumber = u64;
pub type PollEvents = u32;

pub const ABI_VERSION: u32 = 1;
pub const START_SYMBOL: &str = "_start";
pub const STACK_ALIGNMENT: usize = 16;

pub const POLLIN: PollEvents = 0x0001;
pub const POLLPRI: PollEvents = 0x0002;
pub const POLLOUT: PollEvents = 0x0004;

pub const AT_NULL: usize = 0;
pub const AT_PHDR: usize = 3;
pub const AT_PHENT: usize = 4;
pub const AT_PHNUM: usize = 5;
pub const AT_PAGESZ: usize = 6;
pub const AT_ENTRY: usize = 9;
pub const AT_PLATFORM: usize = 15;

pub const BOOT_ARG_FLAG: &str = "--boot";
pub const BOOT_ENV_MARKER: &str = "NGOS_BOOT=1";
pub const SESSION_ENV_MARKER: &str = "NGOS_SESSION=1";
pub const SESSION_ENV_PROTOCOL_PREFIX: &str = "NGOS_SESSION_PROTOCOL=";
pub const SESSION_ENV_OUTCOME_POLICY_PREFIX: &str = "NGOS_SESSION_OUTCOME_POLICY=";
pub const BOOT_ENV_PROTOCOL_PREFIX: &str = "NGOS_BOOT_PROTOCOL=";
pub const BOOT_ENV_MODULE_PREFIX: &str = "NGOS_BOOT_MODULE=";
pub const BOOT_ENV_MODULE_LEN_PREFIX: &str = "NGOS_BOOT_MODULE_LEN=";
pub const BOOT_ENV_MODULE_PHYS_START_PREFIX: &str = "NGOS_BOOT_MODULE_PHYS_START=";
pub const BOOT_ENV_MODULE_PHYS_END_PREFIX: &str = "NGOS_BOOT_MODULE_PHYS_END=";
pub const BOOT_ENV_CMDLINE_PREFIX: &str = "NGOS_BOOT_CMDLINE=";
pub const BOOT_ENV_PROOF_PREFIX: &str = "NGOS_BOOT_PROOF=";
pub const BOOT_ENV_OUTCOME_POLICY_PREFIX: &str = "NGOS_BOOT_OUTCOME_POLICY=";
pub const PROCESS_NAME_ENV_PREFIX: &str = "NGOS_PROCESS_NAME=";
pub const IMAGE_PATH_ENV_PREFIX: &str = "NGOS_IMAGE_PATH=";
pub const CWD_ENV_PREFIX: &str = "NGOS_CWD=";
pub const ROOT_MOUNT_PATH_ENV_PREFIX: &str = "NGOS_ROOT_MOUNT_PATH=";
pub const ROOT_MOUNT_NAME_ENV_PREFIX: &str = "NGOS_ROOT_MOUNT_NAME=";
pub const IMAGE_BASE_ENV_PREFIX: &str = "NGOS_IMAGE_BASE=";
pub const STACK_TOP_ENV_PREFIX: &str = "NGOS_STACK_TOP=";
pub const PHDR_ENV_PREFIX: &str = "NGOS_PHDR=";
pub const PHENT_ENV_PREFIX: &str = "NGOS_PHENT=";
pub const PHNUM_ENV_PREFIX: &str = "NGOS_PHNUM=";
pub const FRAMEBUFFER_PRESENT_ENV_PREFIX: &str = "NGOS_FRAMEBUFFER_PRESENT=";
pub const FRAMEBUFFER_WIDTH_ENV_PREFIX: &str = "NGOS_FRAMEBUFFER_WIDTH=";
pub const FRAMEBUFFER_HEIGHT_ENV_PREFIX: &str = "NGOS_FRAMEBUFFER_HEIGHT=";
pub const FRAMEBUFFER_PITCH_ENV_PREFIX: &str = "NGOS_FRAMEBUFFER_PITCH=";
pub const FRAMEBUFFER_BPP_ENV_PREFIX: &str = "NGOS_FRAMEBUFFER_BPP=";
pub const MEMORY_REGION_COUNT_ENV_PREFIX: &str = "NGOS_MEMORY_REGION_COUNT=";
pub const USABLE_MEMORY_BYTES_ENV_PREFIX: &str = "NGOS_USABLE_MEMORY_BYTES=";
pub const PHYSICAL_MEMORY_OFFSET_ENV_PREFIX: &str = "NGOS_PHYSICAL_MEMORY_OFFSET=";
pub const RSDP_ENV_PREFIX: &str = "NGOS_RSDP=";
pub const KERNEL_PHYS_START_ENV_PREFIX: &str = "NGOS_KERNEL_PHYS_START=";
pub const KERNEL_PHYS_END_ENV_PREFIX: &str = "NGOS_KERNEL_PHYS_END=";

pub const SYS_READ: SyscallNumber = 0;
pub const SYS_WRITE: SyscallNumber = 1;
pub const SYS_EXIT: SyscallNumber = 2;
pub const SYS_CLOSE: SyscallNumber = 3;
pub const SYS_DUP: SyscallNumber = 4;
pub const SYS_FCNTL: SyscallNumber = 5;
pub const SYS_POLL: SyscallNumber = 6;
pub const SYS_MMAP: SyscallNumber = 7;
pub const SYS_MUNMAP: SyscallNumber = 8;
pub const SYS_READV: SyscallNumber = 9;
pub const SYS_WRITEV: SyscallNumber = 10;
pub const SYS_CREATE_DOMAIN: SyscallNumber = 11;
pub const SYS_CREATE_RESOURCE: SyscallNumber = 12;
pub const SYS_CREATE_CONTRACT: SyscallNumber = 13;
pub const SYS_LIST_DOMAINS: SyscallNumber = 14;
pub const SYS_INSPECT_DOMAIN: SyscallNumber = 15;
pub const SYS_LIST_RESOURCES: SyscallNumber = 16;
pub const SYS_INSPECT_RESOURCE: SyscallNumber = 17;
pub const SYS_LIST_CONTRACTS: SyscallNumber = 18;
pub const SYS_INSPECT_CONTRACT: SyscallNumber = 19;
pub const SYS_GET_DOMAIN_NAME: SyscallNumber = 20;
pub const SYS_GET_RESOURCE_NAME: SyscallNumber = 21;
pub const SYS_GET_CONTRACT_LABEL: SyscallNumber = 22;
pub const SYS_SET_CONTRACT_STATE: SyscallNumber = 23;
pub const SYS_INVOKE_CONTRACT: SyscallNumber = 24;
pub const SYS_ACQUIRE_RESOURCE: SyscallNumber = 25;
pub const SYS_RELEASE_RESOURCE: SyscallNumber = 26;
pub const SYS_TRANSFER_RESOURCE: SyscallNumber = 27;
pub const SYS_SET_RESOURCE_POLICY: SyscallNumber = 28;
pub const SYS_CLAIM_RESOURCE: SyscallNumber = 29;
pub const SYS_RELEASE_CLAIMED_RESOURCE: SyscallNumber = 30;
pub const SYS_LIST_RESOURCE_WAITERS: SyscallNumber = 31;
pub const SYS_CANCEL_RESOURCE_CLAIM: SyscallNumber = 32;
pub const SYS_SET_RESOURCE_GOVERNANCE: SyscallNumber = 33;
pub const SYS_SET_RESOURCE_CONTRACT_POLICY: SyscallNumber = 34;
pub const SYS_SET_RESOURCE_ISSUER_POLICY: SyscallNumber = 35;
pub const SYS_SET_RESOURCE_STATE: SyscallNumber = 36;
pub const SYS_BOOT_REPORT: SyscallNumber = 37;
pub const SYS_LIST_PROCESSES: SyscallNumber = 38;
pub const SYS_READ_PROCFS: SyscallNumber = 39;
pub const SYS_STAT_PATH: SyscallNumber = 40;
pub const SYS_LSTAT_PATH: SyscallNumber = 41;
pub const SYS_STATFS_PATH: SyscallNumber = 42;
pub const SYS_OPEN_PATH: SyscallNumber = 43;
pub const SYS_READLINK_PATH: SyscallNumber = 44;
pub const SYS_MKDIR_PATH: SyscallNumber = 45;
pub const SYS_MKFILE_PATH: SyscallNumber = 46;
pub const SYS_SYMLINK_PATH: SyscallNumber = 47;
pub const SYS_RENAME_PATH: SyscallNumber = 48;
pub const SYS_UNLINK_PATH: SyscallNumber = 49;
pub const SYS_LIST_PATH: SyscallNumber = 50;
pub const SYS_SEND_SIGNAL: SyscallNumber = 51;
pub const SYS_PENDING_SIGNALS: SyscallNumber = 52;
pub const SYS_BLOCKED_PENDING_SIGNALS: SyscallNumber = 53;
pub const SYS_SPAWN_PATH_PROCESS: SyscallNumber = 54;
pub const SYS_REAP_PROCESS: SyscallNumber = 55;
pub const SYS_INSPECT_PROCESS: SyscallNumber = 56;
pub const SYS_GET_PROCESS_NAME: SyscallNumber = 57;
pub const SYS_GET_PROCESS_IMAGE_PATH: SyscallNumber = 58;
pub const SYS_GET_PROCESS_CWD: SyscallNumber = 59;
pub const SYS_CHDIR_PATH: SyscallNumber = 60;
pub const SYS_MKSOCK_PATH: SyscallNumber = 61;
pub const SYS_CONFIGURE_NETIF_IPV4: SyscallNumber = 62;
pub const SYS_BIND_UDP_SOCKET: SyscallNumber = 63;
pub const SYS_INSPECT_NETIF: SyscallNumber = 64;
pub const SYS_INSPECT_NETSOCK: SyscallNumber = 65;
pub const SYS_SET_NETIF_LINK_STATE: SyscallNumber = 66;
pub const SYS_CREATE_EVENT_QUEUE: SyscallNumber = 67;
pub const SYS_WAIT_EVENT_QUEUE: SyscallNumber = 68;
pub const SYS_WATCH_NET_EVENTS: SyscallNumber = 69;
pub const SYS_REMOVE_NET_EVENTS: SyscallNumber = 70;
pub const SYS_CONFIGURE_NETIF_ADMIN: SyscallNumber = 71;
pub const SYS_CONNECT_UDP_SOCKET: SyscallNumber = 72;
pub const SYS_SENDTO_UDP_SOCKET: SyscallNumber = 73;
pub const SYS_RECVFROM_UDP_SOCKET: SyscallNumber = 74;
pub const SYS_COMPLETE_NET_TX: SyscallNumber = 75;
pub const SYS_WATCH_PROCESS_EVENTS: SyscallNumber = 76;
pub const SYS_REMOVE_PROCESS_EVENTS: SyscallNumber = 77;
pub const SYS_WATCH_RESOURCE_EVENTS: SyscallNumber = 78;
pub const SYS_REMOVE_RESOURCE_EVENTS: SyscallNumber = 79;
pub const SYS_PAUSE_PROCESS: SyscallNumber = 80;
pub const SYS_RESUME_PROCESS: SyscallNumber = 81;
pub const SYS_RENICE_PROCESS: SyscallNumber = 82;
pub const SYS_INSPECT_SYSTEM_SNAPSHOT: SyscallNumber = 83;
pub const SYS_INSPECT_DEVICE: SyscallNumber = 84;
pub const SYS_INSPECT_DRIVER: SyscallNumber = 85;
pub const SYS_LOAD_MEMORY_WORD: SyscallNumber = 86;
pub const SYS_STORE_MEMORY_WORD: SyscallNumber = 87;
pub const SYS_QUARANTINE_VM_OBJECT: SyscallNumber = 88;
pub const SYS_RELEASE_VM_OBJECT: SyscallNumber = 89;
pub const SYS_SYNC_MEMORY_RANGE: SyscallNumber = 90;
pub const SYS_ADVISE_MEMORY_RANGE: SyscallNumber = 91;
pub const SYS_PROTECT_MEMORY_RANGE: SyscallNumber = 92;
pub const SYS_UNMAP_MEMORY_RANGE: SyscallNumber = 93;
pub const SYS_MAP_ANONYMOUS_MEMORY: SyscallNumber = 94;
pub const SYS_SET_PROCESS_BREAK: SyscallNumber = 95;
pub const SYS_RECLAIM_MEMORY_PRESSURE: SyscallNumber = 96;
pub const SYS_CONTROL_DESCRIPTOR: SyscallNumber = 97;
pub const SYS_REGISTER_READINESS: SyscallNumber = 98;
pub const SYS_COLLECT_READINESS: SyscallNumber = 99;
pub const SYS_CONFIGURE_DEVICE_QUEUE: SyscallNumber = 100;
pub const SYS_WATCH_GRAPHICS_EVENTS: SyscallNumber = 101;
pub const SYS_REMOVE_GRAPHICS_EVENTS: SyscallNumber = 102;
pub const SYS_INSPECT_DEVICE_REQUEST: SyscallNumber = 103;
pub const SYS_CREATE_GPU_BUFFER: SyscallNumber = 110;
pub const SYS_WRITE_GPU_BUFFER: SyscallNumber = 111;
pub const SYS_INSPECT_GPU_BUFFER: SyscallNumber = 112;
pub const SYS_SUBMIT_GPU_BUFFER: SyscallNumber = 113;
pub const SYS_INSPECT_GPU_SCANOUT: SyscallNumber = 114;
pub const SYS_READ_GPU_SCANOUT_FRAME: SyscallNumber = 115;
pub const SYS_PRESENT_GPU_FRAME: SyscallNumber = 116;
pub const SYS_INSPECT_GPU_BINDING: SyscallNumber = 117;
pub const SYS_INSPECT_GPU_VBIOS: SyscallNumber = 118;
pub const SYS_INSPECT_GPU_GSP: SyscallNumber = 119;
pub const SYS_INSPECT_GPU_INTERRUPT: SyscallNumber = 120;
pub const SYS_INSPECT_GPU_DISPLAY: SyscallNumber = 121;
pub const SYS_SPAWN_CONFIGURED_PROCESS: SyscallNumber = 122;
pub const SYS_INSPECT_GPU_POWER: SyscallNumber = 123;
pub const SYS_SET_GPU_POWER_STATE: SyscallNumber = 124;
pub const SYS_INSPECT_GPU_MEDIA: SyscallNumber = 125;
pub const SYS_START_GPU_MEDIA_SESSION: SyscallNumber = 126;
pub const SYS_INSPECT_GPU_NEURAL: SyscallNumber = 127;
pub const SYS_INJECT_GPU_NEURAL_SEMANTIC: SyscallNumber = 128;
pub const SYS_COMMIT_GPU_NEURAL_FRAME: SyscallNumber = 129;
pub const SYS_INSPECT_GPU_TENSOR: SyscallNumber = 130;
pub const SYS_DISPATCH_GPU_TENSOR_KERNEL: SyscallNumber = 131;
pub const SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL: SyscallNumber = 132;
pub const SYS_MAP_FILE_MEMORY: SyscallNumber = 133;
pub const SYS_SPAWN_PROCESS_COPY_VM: SyscallNumber = 134;
pub const SYS_BIND_PROCESS_CONTRACT: SyscallNumber = 135;
pub const SYS_BIND_DEVICE_DRIVER: SyscallNumber = 108;
pub const SYS_UNBIND_DEVICE_DRIVER: SyscallNumber = 109;
pub const SYS_SET_PROCESS_ARGS: SyscallNumber = 104;
pub const SYS_SET_PROCESS_ENV: SyscallNumber = 105;
pub const SYS_SET_PROCESS_CWD: SyscallNumber = 106;
pub const SYS_MKCHAN_PATH: SyscallNumber = 107;

pub const NATIVE_BLOCK_IO_MAGIC: u32 = 0x4e42_4c4b;
pub const NATIVE_BLOCK_IO_VERSION: u16 = 1;
pub const NATIVE_BLOCK_IO_OP_READ: u16 = 1;
pub const NATIVE_BLOCK_IO_OP_WRITE: u16 = 2;
pub const NATIVE_BLOCK_IO_SECURITY_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Errno {
    Perm = 1,
    NoEnt = 2,
    Srch = 3,
    Intr = 4,
    Io = 5,
    Nxio = 6,
    TooBig = 7,
    NoExec = 8,
    Badf = 9,
    Child = 10,
    Again = 11,
    NoMem = 12,
    Access = 13,
    Fault = 14,
    Busy = 16,
    Exist = 17,
    NotDir = 20,
    IsDir = 21,
    Inval = 22,
    Pipe = 32,
    Range = 34,
    NotSup = 95,
    TimedOut = 110,
}

impl Errno {
    pub const fn code(self) -> u16 {
        self as u16
    }

    pub const fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::Perm),
            2 => Some(Self::NoEnt),
            3 => Some(Self::Srch),
            4 => Some(Self::Intr),
            5 => Some(Self::Io),
            6 => Some(Self::Nxio),
            7 => Some(Self::TooBig),
            8 => Some(Self::NoExec),
            9 => Some(Self::Badf),
            10 => Some(Self::Child),
            11 => Some(Self::Again),
            12 => Some(Self::NoMem),
            13 => Some(Self::Access),
            14 => Some(Self::Fault),
            16 => Some(Self::Busy),
            17 => Some(Self::Exist),
            20 => Some(Self::NotDir),
            21 => Some(Self::IsDir),
            22 => Some(Self::Inval),
            32 => Some(Self::Pipe),
            34 => Some(Self::Range),
            95 => Some(Self::NotSup),
            110 => Some(Self::TimedOut),
            _ => None,
        }
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Errno::Perm => "EPERM",
            Errno::NoEnt => "ENOENT",
            Errno::Srch => "ESRCH",
            Errno::Intr => "EINTR",
            Errno::Io => "EIO",
            Errno::Nxio => "ENXIO",
            Errno::TooBig => "E2BIG",
            Errno::NoExec => "ENOEXEC",
            Errno::Badf => "EBADF",
            Errno::Child => "ECHILD",
            Errno::Again => "EAGAIN",
            Errno::NoMem => "ENOMEM",
            Errno::Access => "EACCES",
            Errno::Fault => "EFAULT",
            Errno::Busy => "EBUSY",
            Errno::Exist => "EEXIST",
            Errno::NotDir => "ENOTDIR",
            Errno::IsDir => "EISDIR",
            Errno::Inval => "EINVAL",
            Errno::Pipe => "EPIPE",
            Errno::Range => "ERANGE",
            Errno::NotSup => "ENOTSUP",
            Errno::TimedOut => "ETIMEDOUT",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SecurityErrorCode {
    CapabilityMissing = 1,
    CapabilityExpired = 2,
    CapabilityObjectMismatch = 3,
    CapabilityIssuerMismatch = 4,
    CapabilityGenerationMismatch = 5,
    RightsDenied = 6,
    LabelReadDenied = 7,
    LabelWriteDenied = 8,
    IntegrityMismatch = 9,
    ProvenanceMismatch = 10,
    InvalidSecurityState = 11,
    CapabilityRevoked = 12,
    DelegationDenied = 13,
}

impl SecurityErrorCode {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            1 => Some(Self::CapabilityMissing),
            2 => Some(Self::CapabilityExpired),
            3 => Some(Self::CapabilityObjectMismatch),
            4 => Some(Self::CapabilityIssuerMismatch),
            5 => Some(Self::CapabilityGenerationMismatch),
            6 => Some(Self::RightsDenied),
            7 => Some(Self::LabelReadDenied),
            8 => Some(Self::LabelWriteDenied),
            9 => Some(Self::IntegrityMismatch),
            10 => Some(Self::ProvenanceMismatch),
            11 => Some(Self::InvalidSecurityState),
            12 => Some(Self::CapabilityRevoked),
            13 => Some(Self::DelegationDenied),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SecurityError {
    pub code: SecurityErrorCode,
    pub detail: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BlockRightsMask(pub u64);

impl BlockRightsMask {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const SUBMIT: Self = Self(1 << 2);
    pub const COMPLETE: Self = Self(1 << 3);
    pub const INSPECT_DEVICE: Self = Self(1 << 4);
    pub const INSPECT_DRIVER: Self = Self(1 << 5);
    pub const ATTEST: Self = Self(1 << 6);
    pub const DELEGATE: Self = Self(1 << 7);
    pub const ALL: Self = Self(
        Self::READ.0
            | Self::WRITE.0
            | Self::SUBMIT.0
            | Self::COMPLETE.0
            | Self::INSPECT_DEVICE.0
            | Self::INSPECT_DRIVER.0
            | Self::ATTEST.0
            | Self::DELEGATE.0,
    );

    pub const fn contains(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ConfidentialityLevel {
    Public = 0,
    Internal = 1,
    Sensitive = 2,
    Secret = 3,
    Kernel = 4,
}

impl ConfidentialityLevel {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Public),
            1 => Some(Self::Internal),
            2 => Some(Self::Sensitive),
            3 => Some(Self::Secret),
            4 => Some(Self::Kernel),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum IntegrityLevel {
    Untrusted = 0,
    Verified = 1,
    System = 2,
    Kernel = 3,
}

impl IntegrityLevel {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Untrusted),
            1 => Some(Self::Verified),
            2 => Some(Self::System),
            3 => Some(Self::Kernel),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SecurityLabel {
    pub confidentiality: ConfidentialityLevel,
    pub integrity: IntegrityLevel,
    pub reserved: [u8; 6],
}

impl SecurityLabel {
    pub const fn new(confidentiality: ConfidentialityLevel, integrity: IntegrityLevel) -> Self {
        Self {
            confidentiality,
            integrity,
            reserved: [0; 6],
        }
    }

    pub const fn public_untrusted() -> Self {
        Self::new(ConfidentialityLevel::Public, IntegrityLevel::Untrusted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IntegrityTagKind {
    None = 0,
    Blake3 = 1,
    Sha256 = 2,
    Ed25519 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct IntegrityTag {
    pub kind: IntegrityTagKind,
    pub reserved: u32,
    pub bytes: [u8; 32],
}

impl IntegrityTag {
    pub const fn zeroed(kind: IntegrityTagKind) -> Self {
        Self {
            kind,
            reserved: 0,
            bytes: [0; 32],
        }
    }
}

pub type Authenticator = IntegrityTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct CapabilityToken {
    pub object_id: u64,
    pub rights: BlockRightsMask,
    pub issuer_id: u64,
    pub subject_id: u64,
    pub generation: u64,
    pub revocation_epoch: u64,
    pub delegation_depth: u32,
    pub delegated: u32,
    pub nonce: u64,
    pub expiry_epoch: u64,
    pub authenticator: Authenticator,
}

impl CapabilityToken {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        object_id: u64,
        rights: BlockRightsMask,
        issuer_id: u64,
        subject_id: u64,
        generation: u64,
        revocation_epoch: u64,
        delegation_depth: u32,
        delegated: bool,
        nonce: u64,
        expiry_epoch: u64,
        authenticator: Authenticator,
    ) -> Self {
        Self {
            object_id,
            rights,
            issuer_id,
            subject_id,
            generation,
            revocation_epoch,
            delegation_depth,
            delegated: delegated as u32,
            nonce,
            expiry_epoch,
            authenticator,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SubjectSecurityContext {
    pub subject_id: u64,
    pub active_issuer_id: u64,
    pub rights_ceiling: BlockRightsMask,
    pub label: SecurityLabel,
    pub session_nonce: u64,
    pub current_epoch: u64,
    pub minimum_revocation_epoch: u64,
    pub max_delegation_depth: u32,
}

impl SubjectSecurityContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject_id: u64,
        active_issuer_id: u64,
        rights_ceiling: BlockRightsMask,
        label: SecurityLabel,
        session_nonce: u64,
        current_epoch: u64,
        minimum_revocation_epoch: u64,
        max_delegation_depth: u32,
    ) -> Self {
        Self {
            subject_id,
            active_issuer_id,
            rights_ceiling,
            label,
            session_nonce,
            current_epoch,
            minimum_revocation_epoch,
            max_delegation_depth,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ObjectSecurityContext {
    pub object_id: u64,
    pub required_rights: BlockRightsMask,
    pub minimum_label: SecurityLabel,
    pub current_label: SecurityLabel,
    pub lineage: ProvenanceTag,
    pub integrity: IntegrityTag,
    pub revocation_epoch: u64,
    pub max_delegation_depth: u32,
}

impl ObjectSecurityContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        object_id: u64,
        required_rights: BlockRightsMask,
        minimum_label: SecurityLabel,
        current_label: SecurityLabel,
        lineage: ProvenanceTag,
        integrity: IntegrityTag,
        revocation_epoch: u64,
        max_delegation_depth: u32,
    ) -> Self {
        Self {
            object_id,
            required_rights,
            minimum_label,
            current_label,
            lineage,
            integrity,
            revocation_epoch,
            max_delegation_depth,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProvenanceOriginKind {
    Unknown = 0,
    Subject = 1,
    Device = 2,
    Driver = 3,
    Request = 4,
    Completion = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ProvenanceTag {
    pub origin_kind: ProvenanceOriginKind,
    pub reserved0: u32,
    pub origin_id: u64,
    pub parent_origin_id: u64,
    pub parent_measurement: [u8; 32],
    pub edge_id: u64,
    pub measurement: IntegrityTag,
}

impl ProvenanceTag {
    pub fn root(
        origin_kind: ProvenanceOriginKind,
        origin_id: u64,
        edge_id: u64,
        measurement: IntegrityTag,
    ) -> Self {
        Self {
            origin_kind,
            reserved0: 0,
            origin_id,
            parent_origin_id: 0,
            parent_measurement: [0; 32],
            edge_id,
            measurement,
        }
    }

    pub fn child(
        origin_kind: ProvenanceOriginKind,
        origin_id: u64,
        parent: &ProvenanceTag,
        edge_id: u64,
        measurement: IntegrityTag,
    ) -> Self {
        Self {
            origin_kind,
            reserved0: 0,
            origin_id,
            parent_origin_id: parent.origin_id,
            parent_measurement: parent.measurement.bytes,
            edge_id,
            measurement,
        }
    }
}

pub type CryptographicDna = ProvenanceTag;

pub fn required_block_rights_for_op(op: u16) -> Option<BlockRightsMask> {
    match op {
        NATIVE_BLOCK_IO_OP_READ => Some(BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)),
        NATIVE_BLOCK_IO_OP_WRITE => Some(BlockRightsMask::WRITE.union(BlockRightsMask::SUBMIT)),
        _ => None,
    }
}

pub fn validate_rights(
    available: BlockRightsMask,
    required: BlockRightsMask,
) -> Result<(), SecurityError> {
    if available.contains(required) {
        Ok(())
    } else {
        Err(SecurityError {
            code: SecurityErrorCode::RightsDenied,
            detail: required.0 as u32,
        })
    }
}

pub const fn join_labels(left: SecurityLabel, right: SecurityLabel) -> SecurityLabel {
    let confidentiality = if (left.confidentiality as u8) >= (right.confidentiality as u8) {
        left.confidentiality
    } else {
        right.confidentiality
    };
    let integrity = if (left.integrity as u8) <= (right.integrity as u8) {
        left.integrity
    } else {
        right.integrity
    };
    SecurityLabel::new(confidentiality, integrity)
}

pub fn check_ifc_read(subject: SecurityLabel, object: SecurityLabel) -> Result<(), SecurityError> {
    if (subject.confidentiality as u8) < (object.confidentiality as u8) {
        return Err(SecurityError {
            code: SecurityErrorCode::LabelReadDenied,
            detail: object.confidentiality as u32,
        });
    }
    if (subject.integrity as u8) > (object.integrity as u8) {
        return Err(SecurityError {
            code: SecurityErrorCode::LabelReadDenied,
            detail: object.integrity as u32,
        });
    }
    Ok(())
}

pub fn check_ifc_write(subject: SecurityLabel, object: SecurityLabel) -> Result<(), SecurityError> {
    if (subject.confidentiality as u8) > (object.confidentiality as u8) {
        return Err(SecurityError {
            code: SecurityErrorCode::LabelWriteDenied,
            detail: object.confidentiality as u32,
        });
    }
    if (subject.integrity as u8) < (object.integrity as u8) {
        return Err(SecurityError {
            code: SecurityErrorCode::LabelWriteDenied,
            detail: object.integrity as u32,
        });
    }
    Ok(())
}

pub fn verify_integrity_tag(
    expected: &IntegrityTag,
    candidate: &IntegrityTag,
) -> Result<(), SecurityError> {
    if expected.kind == candidate.kind && expected.bytes == candidate.bytes {
        Ok(())
    } else {
        Err(SecurityError {
            code: SecurityErrorCode::IntegrityMismatch,
            detail: candidate.kind as u32,
        })
    }
}

pub fn validate_subject_context(subject: &SubjectSecurityContext) -> Result<(), SecurityError> {
    if subject.subject_id == 0 || subject.active_issuer_id == 0 {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 0,
        });
    }
    Ok(())
}

pub fn validate_object_context(object: &ObjectSecurityContext) -> Result<(), SecurityError> {
    if object.object_id == 0 {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 1,
        });
    }
    if (object.current_label.confidentiality as u8) < (object.minimum_label.confidentiality as u8)
        || (object.current_label.integrity as u8) > (object.minimum_label.integrity as u8)
    {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 2,
        });
    }
    if object.max_delegation_depth == 0
        && object.required_rights.intersects(BlockRightsMask::DELEGATE)
    {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 8,
        });
    }
    Ok(())
}

pub fn validate_label_transition(
    from: SecurityLabel,
    to: SecurityLabel,
) -> Result<(), SecurityError> {
    if (to.confidentiality as u8) < (from.confidentiality as u8)
        || (to.integrity as u8) > (from.integrity as u8)
    {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 3,
        });
    }
    Ok(())
}

pub fn validate_provenance_tag(tag: &ProvenanceTag) -> Result<(), SecurityError> {
    if tag.origin_id == 0 {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 4,
        });
    }
    if matches!(
        tag.origin_kind,
        ProvenanceOriginKind::Request | ProvenanceOriginKind::Completion
    ) && tag.parent_origin_id == 0
    {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 5,
        });
    }
    Ok(())
}

pub fn validate_integrity_tag(tag: &IntegrityTag) -> Result<(), SecurityError> {
    match tag.kind {
        IntegrityTagKind::None => Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 6,
        }),
        _ => Ok(()),
    }
}

pub fn validate_capability_token(
    token: &CapabilityToken,
    current_epoch: u64,
) -> Result<(), SecurityError> {
    if token.object_id == 0 || token.issuer_id == 0 || token.subject_id == 0 {
        return Err(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: 7,
        });
    }
    if token.rights == BlockRightsMask::NONE {
        return Err(SecurityError {
            code: SecurityErrorCode::RightsDenied,
            detail: 0,
        });
    }
    if token.is_expired(current_epoch) {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityExpired,
            detail: 0,
        });
    }
    if token.delegated != 0 && token.delegation_depth == 0 {
        return Err(SecurityError {
            code: SecurityErrorCode::DelegationDenied,
            detail: 0,
        });
    }
    validate_integrity_tag(&token.authenticator)
}

pub fn validate_revocation(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
) -> Result<(), SecurityError> {
    if token.revocation_epoch < subject.minimum_revocation_epoch
        || token.revocation_epoch < object.revocation_epoch
    {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityRevoked,
            detail: token.revocation_epoch as u32,
        });
    }
    Ok(())
}

pub fn validate_delegation(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    required_rights: BlockRightsMask,
) -> Result<(), SecurityError> {
    if token.delegated == 0 {
        return Ok(());
    }
    if token.delegation_depth > subject.max_delegation_depth
        || token.delegation_depth > object.max_delegation_depth
    {
        return Err(SecurityError {
            code: SecurityErrorCode::DelegationDenied,
            detail: token.delegation_depth,
        });
    }
    if required_rights.intersects(BlockRightsMask::DELEGATE)
        && !token.rights.contains(BlockRightsMask::DELEGATE)
    {
        return Err(SecurityError {
            code: SecurityErrorCode::DelegationDenied,
            detail: 2,
        });
    }
    Ok(())
}

pub fn delegate_capability(
    parent: &CapabilityToken,
    delegated_subject_id: u64,
    delegated_rights: BlockRightsMask,
    delegated_nonce: u64,
    expiry_epoch: u64,
    authenticator: Authenticator,
) -> Result<CapabilityToken, SecurityError> {
    if !parent.rights.contains(BlockRightsMask::DELEGATE) {
        return Err(SecurityError {
            code: SecurityErrorCode::DelegationDenied,
            detail: 3,
        });
    }
    validate_rights(parent.rights, delegated_rights)?;
    Ok(CapabilityToken::new(
        parent.object_id,
        delegated_rights,
        parent.issuer_id,
        delegated_subject_id,
        parent.generation,
        parent.revocation_epoch,
        parent.delegation_depth.saturating_add(1),
        true,
        delegated_nonce,
        expiry_epoch,
        authenticator,
    ))
}

pub const fn derive_effective_request_label(
    subject: SecurityLabel,
    object: SecurityLabel,
) -> SecurityLabel {
    join_labels(subject, object)
}

pub fn derive_effective_completion_label(
    request: SecurityLabel,
    completion: SecurityLabel,
) -> Result<SecurityLabel, SecurityError> {
    validate_label_transition(request, completion)?;
    Ok(join_labels(request, completion))
}

pub const fn security_error_to_errno(error: SecurityErrorCode) -> Errno {
    match error {
        SecurityErrorCode::CapabilityMissing
        | SecurityErrorCode::CapabilityExpired
        | SecurityErrorCode::CapabilityObjectMismatch
        | SecurityErrorCode::CapabilityIssuerMismatch
        | SecurityErrorCode::CapabilityGenerationMismatch
        | SecurityErrorCode::CapabilityRevoked
        | SecurityErrorCode::RightsDenied
        | SecurityErrorCode::LabelReadDenied
        | SecurityErrorCode::LabelWriteDenied => Errno::Access,
        SecurityErrorCode::DelegationDenied => Errno::Perm,
        SecurityErrorCode::IntegrityMismatch | SecurityErrorCode::ProvenanceMismatch => Errno::Io,
        SecurityErrorCode::InvalidSecurityState => Errno::Inval,
    }
}

pub fn check_capability(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    required_rights: BlockRightsMask,
    request_integrity: &IntegrityTag,
) -> Result<(), SecurityError> {
    validate_subject_context(subject)?;
    validate_object_context(object)?;
    validate_provenance_tag(&object.lineage)?;
    validate_integrity_tag(&object.integrity)?;
    validate_capability_token(token, subject.current_epoch)?;
    validate_revocation(subject, object, token)?;
    validate_delegation(subject, object, token, required_rights)?;
    if token.object_id != object.object_id {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityObjectMismatch,
            detail: 0,
        });
    }
    if token.subject_id != subject.subject_id {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityIssuerMismatch,
            detail: 0,
        });
    }
    if token.issuer_id != subject.active_issuer_id {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityIssuerMismatch,
            detail: 1,
        });
    }
    if token.expiry_epoch < subject.current_epoch {
        return Err(SecurityError {
            code: SecurityErrorCode::CapabilityExpired,
            detail: 0,
        });
    }
    validate_rights(token.rights, required_rights)?;
    validate_rights(subject.rights_ceiling, required_rights)?;
    validate_rights(token.rights, object.required_rights)?;
    verify_integrity_tag(&token.authenticator, request_integrity)
}

impl CapabilityToken {
    pub const fn is_expired(&self, current_epoch: u64) -> bool {
        self.expiry_epoch < current_epoch
    }

    pub const fn covers(&self, required: BlockRightsMask) -> bool {
        self.rights.contains(required)
    }
}

pub fn derive_request_provenance(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    request_integrity: IntegrityTag,
    edge_id: u64,
) -> ProvenanceTag {
    ProvenanceTag {
        origin_kind: ProvenanceOriginKind::Request,
        reserved0: 0,
        origin_id: subject.subject_id,
        parent_origin_id: object.lineage.origin_id,
        parent_measurement: object.lineage.measurement.bytes,
        edge_id: edge_id ^ token.nonce ^ token.generation,
        measurement: request_integrity,
    }
}

pub fn derive_completion_provenance(
    request: &ProvenanceTag,
    device_origin_id: u64,
    completion_integrity: IntegrityTag,
    edge_id: u64,
) -> ProvenanceTag {
    ProvenanceTag {
        origin_kind: ProvenanceOriginKind::Completion,
        reserved0: 0,
        origin_id: device_origin_id,
        parent_origin_id: request.origin_id,
        parent_measurement: request.measurement.bytes,
        edge_id: edge_id ^ request.edge_id,
        measurement: completion_integrity,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compose_block_request(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    capability: CapabilityToken,
    op: u16,
    sector: u64,
    sector_count: u32,
    block_size: u32,
    request_label: SecurityLabel,
    request_integrity: IntegrityTag,
    edge_id: u64,
) -> Result<NativeBlockIoRequest, SecurityError> {
    let rights = required_block_rights_for_op(op).ok_or(SecurityError {
        code: SecurityErrorCode::InvalidSecurityState,
        detail: op as u32,
    })?;
    let provenance =
        derive_request_provenance(subject, object, &capability, request_integrity, edge_id);
    let request = NativeBlockIoRequest::new(
        op,
        sector,
        sector_count,
        block_size,
        rights,
        capability,
        request_label,
        provenance,
        request_integrity,
    );
    request.validate_security(subject, object)?;
    Ok(request)
}

pub fn compose_block_completion(
    request: &NativeBlockIoRequest,
    device_origin_id: u64,
    status: u32,
    bytes_transferred: u32,
    completion_label: SecurityLabel,
    completion_integrity: IntegrityTag,
    edge_id: u64,
) -> Result<NativeBlockIoCompletion, SecurityError> {
    let provenance = derive_completion_provenance(
        &request.provenance,
        device_origin_id,
        completion_integrity,
        edge_id,
    );
    let completion = NativeBlockIoCompletion::new(
        request.op,
        status,
        bytes_transferred,
        request.block_size,
        request.rights,
        completion_label,
        provenance,
        completion_integrity,
    );
    completion.preserves_security(request)?;
    Ok(completion)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SyscallFrame {
    pub number: SyscallNumber,
    pub arg0: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
}

impl SyscallFrame {
    pub const fn new(number: SyscallNumber, args: [usize; 6]) -> Self {
        Self {
            number,
            arg0: args[0],
            arg1: args[1],
            arg2: args[2],
            arg3: args[3],
            arg4: args[4],
            arg5: args[5],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SyscallReturn {
    raw: isize,
}

impl SyscallReturn {
    pub const fn ok(value: usize) -> Self {
        Self {
            raw: value as isize,
        }
    }

    pub const fn err(errno: Errno) -> Self {
        Self {
            raw: -(errno.code() as isize),
        }
    }

    pub const fn from_raw(raw: isize) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> isize {
        self.raw
    }

    pub fn into_result(self) -> Result<usize, Errno> {
        if self.raw < 0 {
            let code = self.raw.unsigned_abs() as u16;
            Err(Errno::from_code(code).unwrap_or(Errno::Io))
        } else {
            Ok(self.raw as usize)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallCallingConvention {
    pub name: &'static str,
    pub number_register: &'static str,
    pub argument_registers: [&'static str; 6],
    pub result_register: &'static str,
    pub error_encoding: &'static str,
    pub stack_alignment: usize,
}

pub const AMD64_SYSCALL_CALLING_CONVENTION: SyscallCallingConvention = SyscallCallingConvention {
    name: "amd64-syscall",
    number_register: "rax",
    argument_registers: ["rdi", "rsi", "rdx", "r10", "r8", "r9"],
    result_register: "rax",
    error_encoding: "negative-rax errno",
    stack_alignment: STACK_ALIGNMENT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxvEntry {
    pub key: usize,
    pub value: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct UserIoVec {
    pub base: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapArgs<'a> {
    pub argc: usize,
    pub argv: &'a [&'a str],
    pub envp: &'a [&'a str],
    pub auxv: &'a [AuxvEntry],
}

impl<'a> BootstrapArgs<'a> {
    pub fn new(argv: &'a [&'a str], envp: &'a [&'a str], auxv: &'a [AuxvEntry]) -> Self {
        Self {
            argc: argv.len(),
            argv,
            envp,
            auxv,
        }
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.argv.contains(&flag)
    }

    pub fn has_env_value(&self, value: &str) -> bool {
        self.envp.contains(&value)
    }

    pub fn env_value(&self, prefix: &str) -> Option<&'a str> {
        self.envp
            .iter()
            .find_map(|entry| entry.strip_prefix(prefix))
    }

    pub fn aux_value(&self, key: usize) -> Option<usize> {
        self.auxv
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value)
    }

    pub fn is_boot_mode(&self) -> bool {
        self.has_env_value(BOOT_ENV_MARKER)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartFrame {
    pub argc: usize,
    pub argv: *const *const u8,
    pub envp: *const *const u8,
    pub auxv: *const AuxvEntry,
    pub stack_alignment: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amd64UserEntryRegisters {
    pub rip: usize,
    pub rsp: usize,
    pub rflags: usize,
    pub cs: u16,
    pub ss: u16,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub r8: usize,
    pub r9: usize,
}

impl Amd64UserEntryRegisters {
    pub fn from_start_frame(entry_point: usize, stack_pointer: usize, frame: StartFrame) -> Self {
        Self {
            rip: entry_point,
            rsp: stack_pointer,
            rflags: AMD64_USER_RFLAGS,
            cs: AMD64_USER_CODE_SELECTOR,
            ss: AMD64_USER_STACK_SELECTOR,
            rdi: frame.argc,
            rsi: frame.argv as usize,
            rdx: frame.envp as usize,
            rcx: frame.auxv as usize,
            r8: frame.stack_alignment,
            r9: 0,
        }
    }
}

pub trait SyscallBackend {
    /// # Safety
    ///
    /// The caller must provide a syscall frame whose pointers, buffer lengths,
    /// and ownership assumptions match the active ABI contract expected by the
    /// backend implementation.
    unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlCmd {
    GetFl,
    GetFd,
    SetFl { nonblock: bool },
    SetFd { cloexec: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceKind {
    Memory = 0,
    Storage = 1,
    Channel = 2,
    Device = 3,
    Namespace = 4,
    Surface = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeObjectKind {
    File = 0,
    Directory = 1,
    Symlink = 2,
    Socket = 3,
    Device = 4,
    Driver = 5,
    Process = 6,
    Memory = 7,
    Channel = 8,
    EventQueue = 9,
    SleepQueue = 10,
}

impl NativeObjectKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::File),
            1 => Some(Self::Directory),
            2 => Some(Self::Symlink),
            3 => Some(Self::Socket),
            4 => Some(Self::Device),
            5 => Some(Self::Driver),
            6 => Some(Self::Process),
            7 => Some(Self::Memory),
            8 => Some(Self::Channel),
            9 => Some(Self::EventQueue),
            10 => Some(Self::SleepQueue),
            _ => None,
        }
    }
}

impl NativeResourceKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Memory),
            1 => Some(Self::Storage),
            2 => Some(Self::Channel),
            3 => Some(Self::Device),
            4 => Some(Self::Namespace),
            5 => Some(Self::Surface),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeContractKind {
    Execution = 0,
    Memory = 1,
    Io = 2,
    Device = 3,
    Display = 4,
    Observe = 5,
}

impl NativeContractKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Execution),
            1 => Some(Self::Memory),
            2 => Some(Self::Io),
            3 => Some(Self::Device),
            4 => Some(Self::Display),
            5 => Some(Self::Observe),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeContractState {
    Active = 0,
    Suspended = 1,
    Revoked = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceArbitrationPolicy {
    Fifo = 0,
    Lifo = 1,
}

impl NativeResourceArbitrationPolicy {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Fifo),
            1 => Some(Self::Lifo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceGovernanceMode {
    Queueing = 0,
    ExclusiveLease = 1,
}

impl NativeResourceGovernanceMode {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Queueing),
            1 => Some(Self::ExclusiveLease),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceState {
    Active = 0,
    Suspended = 1,
    Retired = 2,
}

impl NativeResourceState {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Active),
            1 => Some(Self::Suspended),
            2 => Some(Self::Retired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceContractPolicy {
    Any = 0,
    Execution = 1,
    Memory = 2,
    Io = 3,
    Device = 4,
    Display = 5,
    Observe = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeResourceIssuerPolicy {
    AnyIssuer = 0,
    CreatorOnly = 1,
    DomainOwnerOnly = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootSessionStatus {
    Success = 0,
    Failure = 1,
}

impl BootSessionStatus {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Success),
            1 => Some(Self::Failure),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootSessionStage {
    Bootstrap = 0,
    NativeRuntime = 1,
    Complete = 2,
}

impl BootSessionStage {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Bootstrap),
            1 => Some(Self::NativeRuntime),
            2 => Some(Self::Complete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BootSessionReport {
    pub status: u32,
    pub stage: u32,
    pub code: i32,
    pub reserved: u32,
    pub detail: u64,
}

impl NativeResourceIssuerPolicy {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::AnyIssuer),
            1 => Some(Self::CreatorOnly),
            2 => Some(Self::DomainOwnerOnly),
            _ => None,
        }
    }
}

impl NativeResourceContractPolicy {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Any),
            1 => Some(Self::Execution),
            2 => Some(Self::Memory),
            3 => Some(Self::Io),
            4 => Some(Self::Device),
            5 => Some(Self::Display),
            6 => Some(Self::Observe),
            _ => None,
        }
    }
}

impl NativeContractState {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Active),
            1 => Some(Self::Suspended),
            2 => Some(Self::Revoked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeDomainRecord {
    pub id: u64,
    pub owner: u64,
    pub parent: u64,
    pub resource_count: u64,
    pub contract_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeResourceRecord {
    pub id: u64,
    pub domain: u64,
    pub creator: u64,
    pub holder_contract: u64,
    pub kind: u32,
    pub state: u32,
    pub arbitration: u32,
    pub governance: u32,
    pub contract_policy: u32,
    pub issuer_policy: u32,
    pub waiting_count: u64,
    pub acquire_count: u64,
    pub handoff_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeContractRecord {
    pub id: u64,
    pub domain: u64,
    pub resource: u64,
    pub issuer: u64,
    pub kind: u32,
    pub state: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeResourceClaimRecord {
    pub resource: u64,
    pub holder_contract: u64,
    pub acquire_count: u64,
    pub position: u64,
    pub queued: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeResourceReleaseRecord {
    pub resource: u64,
    pub handoff_contract: u64,
    pub acquire_count: u64,
    pub handoff_count: u64,
    pub handed_off: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeResourceCancelRecord {
    pub resource: u64,
    pub waiting_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeFileStatusRecord {
    pub inode: u64,
    pub size: u64,
    pub kind: u32,
    pub cloexec: u32,
    pub nonblock: u32,
    pub readable: u32,
    pub writable: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeFileSystemStatusRecord {
    pub mount_count: u64,
    pub node_count: u64,
    pub read_only: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkInterfaceConfig {
    pub addr: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeUdpBindConfig {
    pub remote_ipv4: [u8; 4],
    pub local_port: u16,
    pub remote_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkAdminConfig {
    pub mtu: u64,
    pub tx_capacity: u64,
    pub rx_capacity: u64,
    pub tx_inflight_limit: u64,
    pub admin_up: u32,
    pub promiscuous: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeUdpConnectConfig {
    pub remote_ipv4: [u8; 4],
    pub remote_port: u16,
    pub reserved: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeUdpSendToConfig {
    pub remote_ipv4: [u8; 4],
    pub remote_port: u16,
    pub reserved: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeUdpRecvMeta {
    pub remote_ipv4: [u8; 4],
    pub remote_port: u16,
    pub reserved: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeEventQueueMode {
    Kqueue = 0,
    Epoll = 1,
}

impl NativeEventQueueMode {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Kqueue),
            1 => Some(Self::Epoll),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeEventSourceKind {
    Descriptor = 0,
    Timer = 1,
    Process = 2,
    Signal = 3,
    MemoryWait = 4,
    Resource = 5,
    Network = 6,
    Graphics = 7,
}

impl NativeEventSourceKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Descriptor),
            1 => Some(Self::Timer),
            2 => Some(Self::Process),
            3 => Some(Self::Signal),
            4 => Some(Self::MemoryWait),
            5 => Some(Self::Resource),
            6 => Some(Self::Network),
            7 => Some(Self::Graphics),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeSchedulerClass {
    LatencyCritical = 0,
    Interactive = 1,
    BestEffort = 2,
    Background = 3,
}

impl NativeSchedulerClass {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::LatencyCritical),
            1 => Some(Self::Interactive),
            2 => Some(Self::BestEffort),
            3 => Some(Self::Background),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeProcessEventWatchConfig {
    pub token: u64,
    pub poll_events: u32,
    pub exited: u32,
    pub reaped: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeResourceEventWatchConfig {
    pub token: u64,
    pub poll_events: u32,
    pub claimed: u32,
    pub queued: u32,
    pub canceled: u32,
    pub released: u32,
    pub handed_off: u32,
    pub revoked: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeNetworkEventKind {
    LinkChanged = 0,
    RxReady = 1,
    TxDrained = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeGraphicsEventKind {
    Submitted = 0,
    Completed = 1,
    Failed = 2,
    Drained = 3,
    Canceled = 4,
    Faulted = 5,
    Recovered = 6,
    Retired = 7,
    LeaseReleased = 8,
    LeaseAcquired = 9,
}

impl NativeGraphicsEventKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Submitted),
            1 => Some(Self::Completed),
            2 => Some(Self::Failed),
            3 => Some(Self::Drained),
            4 => Some(Self::Canceled),
            5 => Some(Self::Faulted),
            6 => Some(Self::Recovered),
            7 => Some(Self::Retired),
            8 => Some(Self::LeaseReleased),
            9 => Some(Self::LeaseAcquired),
            _ => None,
        }
    }
}

impl NativeNetworkEventKind {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::LinkChanged),
            1 => Some(Self::RxReady),
            2 => Some(Self::TxDrained),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkLinkStateConfig {
    pub link_up: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkEventWatchConfig {
    pub token: u64,
    pub poll_events: u32,
    pub link_changed: u32,
    pub rx_ready: u32,
    pub tx_drained: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGraphicsEventWatchConfig {
    pub token: u64,
    pub poll_events: u32,
    pub submitted: u32,
    pub completed: u32,
    pub failed: u32,
    pub drained: u32,
    pub canceled: u32,
    pub faulted: u32,
    pub recovered: u32,
    pub retired: u32,
    pub lease_released: u32,
    pub lease_acquired: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeEventRecord {
    pub token: u64,
    pub events: u32,
    pub source_kind: u32,
    pub source_arg0: u64,
    pub source_arg1: u64,
    pub source_arg2: u64,
    pub detail0: u32,
    pub detail1: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeReadinessRecord {
    pub owner: u64,
    pub fd: u64,
    pub readable: u32,
    pub writable: u32,
    pub priority: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkInterfaceRecord {
    pub admin_up: u32,
    pub link_up: u32,
    pub promiscuous: u32,
    pub reserved: u32,
    pub mtu: u64,
    pub tx_capacity: u64,
    pub rx_capacity: u64,
    pub tx_inflight_limit: u64,
    pub tx_inflight_depth: u64,
    pub free_buffer_count: u64,
    pub mac: [u8; 6],
    pub mac_reserved: [u8; 2],
    pub ipv4_addr: [u8; 4],
    pub ipv4_netmask: [u8; 4],
    pub ipv4_gateway: [u8; 4],
    pub ipv4_reserved: [u8; 4],
    pub rx_ring_depth: u64,
    pub tx_ring_depth: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_completions: u64,
    pub tx_dropped: u64,
    pub rx_dropped: u64,
    pub attached_socket_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeNetworkSocketRecord {
    pub local_ipv4: [u8; 4],
    pub remote_ipv4: [u8; 4],
    pub local_port: u16,
    pub remote_port: u16,
    pub connected: u32,
    pub reserved: u32,
    pub rx_depth: u64,
    pub rx_queue_limit: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub dropped_packets: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeDeviceRecord {
    pub class: u32,
    pub state: u32,
    pub reserved0: u64,
    pub queue_depth: u64,
    pub queue_capacity: u64,
    pub submitted_requests: u64,
    pub completed_requests: u64,
    pub total_latency_ticks: u64,
    pub max_latency_ticks: u64,
    pub total_queue_wait_ticks: u64,
    pub max_queue_wait_ticks: u64,
    pub link_up: u32,
    pub reserved1: u32,
    pub block_size: u32,
    pub reserved2: u32,
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeDriverRecord {
    pub state: u32,
    pub reserved: u32,
    pub bound_device_count: u64,
    pub queued_requests: u64,
    pub in_flight_requests: u64,
    pub completed_requests: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeDeviceRequestRecord {
    pub issuer: u64,
    pub kind: u32,
    pub state: u32,
    pub opcode: u64,
    pub buffer_id: u64,
    pub payload_len: u64,
    pub response_len: u64,
    pub submitted_tick: u64,
    pub started_tick: u64,
    pub completed_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuBufferRecord {
    pub owner: u64,
    pub length: u64,
    pub used_len: u64,
    pub reserved: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuScanoutRecord {
    pub presented_frames: u64,
    pub last_frame_len: u64,
    pub reserved0: u64,
    pub reserved1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuBindingRecord {
    pub present: u32,
    pub msi_supported: u32,
    pub msi_message_limit: u32,
    pub resizable_bar_enabled: u32,
    pub subsystem_id: u32,
    pub bar1_total_mib: u32,
    pub framebuffer_total_mib: u32,
    pub display_engine_confirmed: u32,
    pub architecture_name: [u8; 32],
    pub product_name: [u8; 64],
    pub die_name: [u8; 16],
    pub bus_interface: [u8; 32],
    pub inf_section: [u8; 32],
    pub kernel_service: [u8; 32],
    pub vbios_version: [u8; 32],
    pub part_number: [u8; 32],
    pub msi_source_name: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuVbiosRecord {
    pub present: u32,
    pub enabled: u32,
    pub vendor_id: u32,
    pub rom_bar_raw: u32,
    pub device_id: u32,
    pub physical_base: u64,
    pub image_len: u64,
    pub header_len: u32,
    pub pcir_offset: u32,
    pub bit_offset: u32,
    pub nvfw_offset: u32,
    pub header: [u8; 16],
    pub board_name: [u8; 64],
    pub board_code: [u8; 32],
    pub version: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuGspRecord {
    pub present: u32,
    pub loopback_ready: u32,
    pub firmware_known: u32,
    pub blackwell_blob_present: u32,
    pub hardware_ready: u32,
    pub driver_model_wddm: u32,
    pub loopback_completions: u64,
    pub loopback_failures: u64,
    pub firmware_version: [u8; 16],
    pub blob_summary: [u8; 48],
    pub refusal_reason: [u8; 48],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuInterruptRecord {
    pub present: u32,
    pub vector: u32,
    pub delivered_count: u64,
    pub msi_supported: u32,
    pub message_limit: u32,
    pub windows_interrupt_message_maximum: u32,
    pub hardware_servicing_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuDisplayRecord {
    pub present: u32,
    pub active_pipes: u32,
    pub planned_frames: u64,
    pub last_present_offset: u64,
    pub last_present_len: u64,
    pub hardware_programming_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeGpuPowerRecord {
    pub present: u32,
    pub pstate: u32,
    pub graphics_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub boost_clock_mhz: u32,
    pub hardware_power_management_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct NativeGpuMediaRecord {
    pub present: u32,
    pub sessions: u32,
    pub codec: u32,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub hardware_media_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct NativeGpuNeuralRecord {
    pub present: u32,
    pub model_loaded: u32,
    pub active_semantics: u32,
    pub last_commit_completed: u32,
    pub hardware_neural_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct NativeGpuTensorRecord {
    pub present: u32,
    pub active_jobs: u32,
    pub last_kernel_id: u32,
    pub hardware_tensor_confirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct NativeSpawnProcessConfig {
    pub name_ptr: usize,
    pub name_len: usize,
    pub path_ptr: usize,
    pub path_len: usize,
    pub cwd_ptr: usize,
    pub cwd_len: usize,
    pub argv_ptr: usize,
    pub argv_len: usize,
    pub argv_count: usize,
    pub envp_ptr: usize,
    pub envp_len: usize,
    pub envp_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeBlockIoRequest {
    pub magic: u32,
    pub version: u16,
    pub op: u16,
    pub sector: u64,
    pub sector_count: u32,
    pub block_size: u32,
    pub rights: BlockRightsMask,
    pub capability: CapabilityToken,
    pub label: SecurityLabel,
    pub provenance: ProvenanceTag,
    pub integrity: IntegrityTag,
}

impl NativeBlockIoRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        op: u16,
        sector: u64,
        sector_count: u32,
        block_size: u32,
        rights: BlockRightsMask,
        capability: CapabilityToken,
        label: SecurityLabel,
        provenance: ProvenanceTag,
        integrity: IntegrityTag,
    ) -> Self {
        Self {
            magic: NATIVE_BLOCK_IO_MAGIC,
            version: NATIVE_BLOCK_IO_SECURITY_VERSION,
            op,
            sector,
            sector_count,
            block_size,
            rights,
            capability,
            label,
            provenance,
            integrity,
        }
    }

    pub fn validate_security(
        &self,
        subject: &SubjectSecurityContext,
        object: &ObjectSecurityContext,
    ) -> Result<(), SecurityError> {
        if self.magic != NATIVE_BLOCK_IO_MAGIC || self.version != NATIVE_BLOCK_IO_SECURITY_VERSION {
            return Err(SecurityError {
                code: SecurityErrorCode::InvalidSecurityState,
                detail: self.version as u32,
            });
        }
        if self.sector_count == 0 || self.block_size == 0 {
            return Err(SecurityError {
                code: SecurityErrorCode::InvalidSecurityState,
                detail: self.sector_count,
            });
        }
        let required_rights = required_block_rights_for_op(self.op).ok_or(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: self.op as u32,
        })?;
        validate_rights(self.rights, required_rights)?;
        validate_provenance_tag(&self.provenance)?;
        validate_integrity_tag(&self.integrity)?;
        check_capability(
            subject,
            object,
            &self.capability,
            self.rights,
            &self.integrity,
        )?;
        match self.op {
            NATIVE_BLOCK_IO_OP_READ => check_ifc_read(subject.label, self.label),
            NATIVE_BLOCK_IO_OP_WRITE => check_ifc_write(subject.label, self.label),
            _ => Err(SecurityError {
                code: SecurityErrorCode::InvalidSecurityState,
                detail: self.op as u32,
            }),
        }
    }

    pub fn required_rights(&self) -> Result<BlockRightsMask, SecurityError> {
        required_block_rights_for_op(self.op).ok_or(SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: self.op as u32,
        })
    }

    pub const fn is_read(&self) -> bool {
        self.op == NATIVE_BLOCK_IO_OP_READ
    }

    pub const fn is_write(&self) -> bool {
        self.op == NATIVE_BLOCK_IO_OP_WRITE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeBlockIoCompletion {
    pub magic: u32,
    pub version: u16,
    pub op: u16,
    pub status: u32,
    pub bytes_transferred: u32,
    pub block_size: u32,
    pub rights: BlockRightsMask,
    pub label: SecurityLabel,
    pub provenance: ProvenanceTag,
    pub integrity: IntegrityTag,
}

impl NativeBlockIoCompletion {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        op: u16,
        status: u32,
        bytes_transferred: u32,
        block_size: u32,
        rights: BlockRightsMask,
        label: SecurityLabel,
        provenance: ProvenanceTag,
        integrity: IntegrityTag,
    ) -> Self {
        Self {
            magic: NATIVE_BLOCK_IO_MAGIC,
            version: NATIVE_BLOCK_IO_SECURITY_VERSION,
            op,
            status,
            bytes_transferred,
            block_size,
            rights,
            label,
            provenance,
            integrity,
        }
    }

    pub fn preserves_security(&self, request: &NativeBlockIoRequest) -> Result<(), SecurityError> {
        if self.magic != NATIVE_BLOCK_IO_MAGIC || self.version != NATIVE_BLOCK_IO_SECURITY_VERSION {
            return Err(SecurityError {
                code: SecurityErrorCode::InvalidSecurityState,
                detail: self.version as u32,
            });
        }
        if self.op != request.op {
            return Err(SecurityError {
                code: SecurityErrorCode::InvalidSecurityState,
                detail: self.op as u32,
            });
        }
        validate_provenance_tag(&self.provenance)?;
        validate_integrity_tag(&self.integrity)?;
        validate_rights(request.rights, self.rights)?;
        validate_label_transition(request.label, self.label).map_err(|_| SecurityError {
            code: SecurityErrorCode::InvalidSecurityState,
            detail: self.op as u32,
        })?;
        if self.provenance.parent_origin_id != request.provenance.origin_id
            || self.provenance.parent_measurement != request.provenance.measurement.bytes
        {
            return Err(SecurityError {
                code: SecurityErrorCode::ProvenanceMismatch,
                detail: self.op as u32,
            });
        }
        Ok(())
    }

    pub const fn is_success(&self) -> bool {
        self.status == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeProcessRecord {
    pub pid: u64,
    pub parent: u64,
    pub address_space: u64,
    pub main_thread: u64,
    pub state: u32,
    pub exit_code: i32,
    pub descriptor_count: u64,
    pub capability_count: u64,
    pub environment_count: u64,
    pub memory_region_count: u64,
    pub thread_count: u64,
    pub pending_signal_count: u64,
    pub session_reported: u32,
    pub session_status: u32,
    pub session_stage: u32,
    pub scheduler_class: u32,
    pub scheduler_budget: u32,
    pub cpu_runtime_ticks: u64,
    pub execution_contract: u64,
    pub memory_contract: u64,
    pub io_contract: u64,
    pub observe_contract: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSystemSnapshotRecord {
    pub current_tick: u64,
    pub busy_ticks: u64,
    pub process_count: u64,
    pub active_process_count: u64,
    pub blocked_process_count: u64,
    pub queued_processes: u64,
    pub queued_latency_critical: u64,
    pub queued_interactive: u64,
    pub queued_normal: u64,
    pub queued_background: u64,
    pub deferred_task_count: u64,
    pub sleeping_processes: u64,
    pub total_event_queue_count: u64,
    pub total_event_queue_pending: u64,
    pub total_event_queue_waiters: u64,
    pub total_socket_count: u64,
    pub saturated_socket_count: u64,
    pub total_socket_rx_depth: u64,
    pub total_socket_rx_limit: u64,
    pub max_socket_rx_depth: u64,
    pub total_network_tx_dropped: u64,
    pub total_network_rx_dropped: u64,
    pub running_pid: u64,
    pub reserved0: u64,
    pub reserved1: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LABEL_LOW: SecurityLabel =
        SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
    const TEST_LABEL_HIGH: SecurityLabel =
        SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::System);

    const TEST_INTEGRITY_A: IntegrityTag = IntegrityTag {
        kind: IntegrityTagKind::Blake3,
        reserved: 0,
        bytes: [0x11; 32],
    };

    const TEST_INTEGRITY_B: IntegrityTag = IntegrityTag {
        kind: IntegrityTagKind::Blake3,
        reserved: 0,
        bytes: [0x22; 32],
    };

    #[test]
    fn syscall_numbers_are_stable_for_first_round_trip() {
        assert_eq!(SYS_READ, 0);
        assert_eq!(SYS_WRITE, 1);
        assert_eq!(SYS_EXIT, 2);
        assert_eq!(SYS_CLOSE, 3);
        assert_eq!(SYS_CREATE_DOMAIN, 11);
        assert_eq!(SYS_CREATE_RESOURCE, 12);
        assert_eq!(SYS_CREATE_CONTRACT, 13);
        assert_eq!(SYS_LIST_DOMAINS, 14);
        assert_eq!(SYS_INSPECT_DOMAIN, 15);
        assert_eq!(SYS_LIST_RESOURCES, 16);
        assert_eq!(SYS_INSPECT_RESOURCE, 17);
        assert_eq!(SYS_LIST_CONTRACTS, 18);
        assert_eq!(SYS_INSPECT_CONTRACT, 19);
        assert_eq!(SYS_GET_DOMAIN_NAME, 20);
        assert_eq!(SYS_GET_RESOURCE_NAME, 21);
        assert_eq!(SYS_GET_CONTRACT_LABEL, 22);
        assert_eq!(SYS_SET_CONTRACT_STATE, 23);
        assert_eq!(SYS_INVOKE_CONTRACT, 24);
        assert_eq!(SYS_ACQUIRE_RESOURCE, 25);
        assert_eq!(SYS_RELEASE_RESOURCE, 26);
        assert_eq!(SYS_TRANSFER_RESOURCE, 27);
        assert_eq!(SYS_SET_RESOURCE_POLICY, 28);
        assert_eq!(SYS_CLAIM_RESOURCE, 29);
        assert_eq!(SYS_RELEASE_CLAIMED_RESOURCE, 30);
        assert_eq!(SYS_LIST_RESOURCE_WAITERS, 31);
        assert_eq!(SYS_CANCEL_RESOURCE_CLAIM, 32);
        assert_eq!(SYS_SET_RESOURCE_GOVERNANCE, 33);
        assert_eq!(SYS_SET_RESOURCE_CONTRACT_POLICY, 34);
        assert_eq!(SYS_SET_RESOURCE_ISSUER_POLICY, 35);
        assert_eq!(SYS_SET_RESOURCE_STATE, 36);
        assert_eq!(SYS_BOOT_REPORT, 37);
        assert_eq!(SYS_LIST_PROCESSES, 38);
        assert_eq!(SYS_READ_PROCFS, 39);
        assert_eq!(SYS_STAT_PATH, 40);
        assert_eq!(SYS_LSTAT_PATH, 41);
        assert_eq!(SYS_STATFS_PATH, 42);
        assert_eq!(SYS_OPEN_PATH, 43);
        assert_eq!(SYS_READLINK_PATH, 44);
        assert_eq!(SYS_MKDIR_PATH, 45);
        assert_eq!(SYS_MKFILE_PATH, 46);
        assert_eq!(SYS_SYMLINK_PATH, 47);
        assert_eq!(SYS_RENAME_PATH, 48);
        assert_eq!(SYS_UNLINK_PATH, 49);
        assert_eq!(SYS_LIST_PATH, 50);
        assert_eq!(SYS_SEND_SIGNAL, 51);
        assert_eq!(SYS_PENDING_SIGNALS, 52);
        assert_eq!(SYS_BLOCKED_PENDING_SIGNALS, 53);
        assert_eq!(SYS_SPAWN_PATH_PROCESS, 54);
        assert_eq!(SYS_REAP_PROCESS, 55);
        assert_eq!(SYS_INSPECT_PROCESS, 56);
        assert_eq!(SYS_GET_PROCESS_NAME, 57);
        assert_eq!(SYS_GET_PROCESS_IMAGE_PATH, 58);
        assert_eq!(SYS_GET_PROCESS_CWD, 59);
        assert_eq!(SYS_CHDIR_PATH, 60);
        assert_eq!(SYS_MKSOCK_PATH, 61);
        assert_eq!(SYS_CONFIGURE_NETIF_IPV4, 62);
        assert_eq!(SYS_BIND_UDP_SOCKET, 63);
        assert_eq!(SYS_INSPECT_NETIF, 64);
        assert_eq!(SYS_INSPECT_NETSOCK, 65);
        assert_eq!(SYS_SET_NETIF_LINK_STATE, 66);
        assert_eq!(SYS_CREATE_EVENT_QUEUE, 67);
        assert_eq!(SYS_WAIT_EVENT_QUEUE, 68);
        assert_eq!(SYS_WATCH_NET_EVENTS, 69);
        assert_eq!(SYS_REMOVE_NET_EVENTS, 70);
        assert_eq!(SYS_CONFIGURE_NETIF_ADMIN, 71);
        assert_eq!(SYS_CONNECT_UDP_SOCKET, 72);
        assert_eq!(SYS_SENDTO_UDP_SOCKET, 73);
        assert_eq!(SYS_RECVFROM_UDP_SOCKET, 74);
        assert_eq!(SYS_COMPLETE_NET_TX, 75);
        assert_eq!(SYS_WATCH_PROCESS_EVENTS, 76);
        assert_eq!(SYS_REMOVE_PROCESS_EVENTS, 77);
        assert_eq!(SYS_WATCH_RESOURCE_EVENTS, 78);
        assert_eq!(SYS_REMOVE_RESOURCE_EVENTS, 79);
        assert_eq!(SYS_PAUSE_PROCESS, 80);
        assert_eq!(SYS_RESUME_PROCESS, 81);
        assert_eq!(SYS_RENICE_PROCESS, 82);
        assert_eq!(SYS_INSPECT_SYSTEM_SNAPSHOT, 83);
        assert_eq!(SYS_INSPECT_DEVICE, 84);
        assert_eq!(SYS_INSPECT_DRIVER, 85);
        assert_eq!(SYS_LOAD_MEMORY_WORD, 86);
        assert_eq!(SYS_STORE_MEMORY_WORD, 87);
        assert_eq!(SYS_QUARANTINE_VM_OBJECT, 88);
        assert_eq!(SYS_RELEASE_VM_OBJECT, 89);
        assert_eq!(SYS_SYNC_MEMORY_RANGE, 90);
        assert_eq!(SYS_ADVISE_MEMORY_RANGE, 91);
        assert_eq!(SYS_PROTECT_MEMORY_RANGE, 92);
        assert_eq!(SYS_UNMAP_MEMORY_RANGE, 93);
        assert_eq!(SYS_MAP_ANONYMOUS_MEMORY, 94);
        assert_eq!(SYS_SET_PROCESS_BREAK, 95);
        assert_eq!(SYS_RECLAIM_MEMORY_PRESSURE, 96);
        assert_eq!(SYS_CONTROL_DESCRIPTOR, 97);
        assert_eq!(SYS_REGISTER_READINESS, 98);
        assert_eq!(SYS_COLLECT_READINESS, 99);
        assert_eq!(SYS_CONFIGURE_DEVICE_QUEUE, 100);
        assert_eq!(SYS_WATCH_GRAPHICS_EVENTS, 101);
        assert_eq!(SYS_REMOVE_GRAPHICS_EVENTS, 102);
        assert_eq!(SYS_INSPECT_DEVICE_REQUEST, 103);
        assert_eq!(SYS_SET_PROCESS_ARGS, 104);
        assert_eq!(SYS_SET_PROCESS_ENV, 105);
        assert_eq!(SYS_SET_PROCESS_CWD, 106);
        assert_eq!(SYS_MKCHAN_PATH, 107);
        assert_eq!(SYS_BIND_DEVICE_DRIVER, 108);
        assert_eq!(SYS_UNBIND_DEVICE_DRIVER, 109);
        assert_eq!(SYS_CREATE_GPU_BUFFER, 110);
        assert_eq!(SYS_WRITE_GPU_BUFFER, 111);
        assert_eq!(SYS_INSPECT_GPU_BUFFER, 112);
        assert_eq!(SYS_SUBMIT_GPU_BUFFER, 113);
        assert_eq!(SYS_INSPECT_GPU_SCANOUT, 114);
        assert_eq!(SYS_READ_GPU_SCANOUT_FRAME, 115);
        assert_eq!(SYS_PRESENT_GPU_FRAME, 116);
        assert_eq!(SYS_INSPECT_GPU_BINDING, 117);
        assert_eq!(SYS_INSPECT_GPU_VBIOS, 118);
        assert_eq!(SYS_INSPECT_GPU_GSP, 119);
        assert_eq!(SYS_INSPECT_GPU_INTERRUPT, 120);
        assert_eq!(SYS_INSPECT_GPU_DISPLAY, 121);
        assert_eq!(SYS_SPAWN_CONFIGURED_PROCESS, 122);
        assert_eq!(SYS_INSPECT_GPU_POWER, 123);
        assert_eq!(SYS_SET_GPU_POWER_STATE, 124);
        assert_eq!(SYS_INSPECT_GPU_MEDIA, 125);
        assert_eq!(SYS_START_GPU_MEDIA_SESSION, 126);
        assert_eq!(SYS_INSPECT_GPU_NEURAL, 127);
        assert_eq!(SYS_INJECT_GPU_NEURAL_SEMANTIC, 128);
        assert_eq!(SYS_COMMIT_GPU_NEURAL_FRAME, 129);
        assert_eq!(SYS_INSPECT_GPU_TENSOR, 130);
        assert_eq!(SYS_DISPATCH_GPU_TENSOR_KERNEL, 131);
        assert_eq!(SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, 132);
        assert_eq!(SYS_MAP_FILE_MEMORY, 133);
        assert_eq!(SYS_SPAWN_PROCESS_COPY_VM, 134);
        assert_eq!(SYS_BIND_PROCESS_CONTRACT, 135);
    }

    #[test]
    fn amd64_calling_convention_matches_runtime_contract() {
        let cc = AMD64_SYSCALL_CALLING_CONVENTION;
        assert_eq!(cc.number_register, "rax");
        assert_eq!(
            cc.argument_registers,
            ["rdi", "rsi", "rdx", "r10", "r8", "r9"]
        );
        assert_eq!(cc.result_register, "rax");
        assert_eq!(cc.stack_alignment, STACK_ALIGNMENT);
    }

    #[test]
    fn syscall_return_roundtrips_success_and_errno() {
        let ok = SyscallReturn::ok(1234);
        assert_eq!(ok.raw(), 1234);
        assert_eq!(ok.into_result(), Ok(1234));

        let err = SyscallReturn::err(Errno::Badf);
        assert_eq!(err.raw(), -(Errno::Badf.code() as isize));
        assert_eq!(err.into_result(), Err(Errno::Badf));
    }

    #[test]
    fn unknown_negative_errno_maps_to_eio() {
        let raw = SyscallReturn::from_raw(-65_000);
        assert_eq!(raw.into_result(), Err(Errno::Io));
    }

    #[test]
    fn bootstrap_args_expose_boot_contract_helpers() {
        let argv = ["ngos-userland-native", BOOT_ARG_FLAG];
        let envp = [
            BOOT_ENV_MARKER,
            "TERM=dumb",
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_BOOT_MODULE_PHYS_START=0x200000",
            "NGOS_BOOT_MODULE_PHYS_END=0x203000",
            "NGOS_IMAGE_PATH=ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x0",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
            "NGOS_MEMORY_REGION_COUNT=4",
            "NGOS_USABLE_MEMORY_BYTES=16777216",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0xffff800000000000",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x180000",
            "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        ];
        let auxv = [
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert!(bootstrap.is_boot_mode());
        assert!(bootstrap.has_flag(BOOT_ARG_FLAG));
        assert_eq!(
            bootstrap.env_value(BOOT_ENV_PROTOCOL_PREFIX),
            Some("limine")
        );
        assert_eq!(
            bootstrap.env_value(BOOT_ENV_MODULE_PREFIX),
            Some("ngos-userland-native")
        );
        assert_eq!(
            bootstrap.env_value(BOOT_ENV_MODULE_PHYS_START_PREFIX),
            Some("0x200000")
        );
        assert_eq!(
            bootstrap.env_value(BOOT_ENV_MODULE_PHYS_END_PREFIX),
            Some("0x203000")
        );
        assert_eq!(
            bootstrap.env_value(PROCESS_NAME_ENV_PREFIX),
            Some("ngos-userland-native")
        );
        assert_eq!(
            bootstrap.env_value(IMAGE_PATH_ENV_PREFIX),
            Some("ngos-userland-native")
        );
        assert_eq!(bootstrap.env_value(CWD_ENV_PREFIX), Some("/"));
        assert_eq!(bootstrap.env_value(ROOT_MOUNT_PATH_ENV_PREFIX), Some("/"));
        assert_eq!(
            bootstrap.env_value(ROOT_MOUNT_NAME_ENV_PREFIX),
            Some("rootfs")
        );
        assert_eq!(bootstrap.env_value(IMAGE_BASE_ENV_PREFIX), Some("0x0"));
        assert_eq!(
            bootstrap.env_value(STACK_TOP_ENV_PREFIX),
            Some("0x7fffffff0000")
        );
        assert_eq!(bootstrap.env_value(PHDR_ENV_PREFIX), Some("0x40"));
        assert_eq!(bootstrap.env_value(PHENT_ENV_PREFIX), Some("56"));
        assert_eq!(bootstrap.env_value(PHNUM_ENV_PREFIX), Some("2"));
        assert_eq!(
            bootstrap.env_value(FRAMEBUFFER_PRESENT_ENV_PREFIX),
            Some("1")
        );
        assert_eq!(
            bootstrap.env_value(FRAMEBUFFER_WIDTH_ENV_PREFIX),
            Some("1920")
        );
        assert_eq!(
            bootstrap.env_value(FRAMEBUFFER_HEIGHT_ENV_PREFIX),
            Some("1080")
        );
        assert_eq!(
            bootstrap.env_value(FRAMEBUFFER_PITCH_ENV_PREFIX),
            Some("7680")
        );
        assert_eq!(bootstrap.env_value(FRAMEBUFFER_BPP_ENV_PREFIX), Some("32"));
        assert_eq!(
            bootstrap.env_value(MEMORY_REGION_COUNT_ENV_PREFIX),
            Some("4")
        );
        assert_eq!(
            bootstrap.env_value(USABLE_MEMORY_BYTES_ENV_PREFIX),
            Some("16777216")
        );
        assert_eq!(
            bootstrap.env_value(PHYSICAL_MEMORY_OFFSET_ENV_PREFIX),
            Some("0xffff800000000000")
        );
        assert_eq!(bootstrap.env_value(RSDP_ENV_PREFIX), Some("0xdeadbeef"));
        assert_eq!(
            bootstrap.env_value(KERNEL_PHYS_START_ENV_PREFIX),
            Some("0x100000")
        );
        assert_eq!(
            bootstrap.env_value(KERNEL_PHYS_END_ENV_PREFIX),
            Some("0x180000")
        );
        assert_eq!(
            bootstrap.env_value(BOOT_ENV_OUTCOME_POLICY_PREFIX),
            Some("require-zero-exit")
        );
        assert_eq!(bootstrap.aux_value(AT_PAGESZ), Some(4096));
        assert_eq!(bootstrap.aux_value(AT_ENTRY), Some(0x401000));
    }

    #[test]
    fn native_kind_enums_roundtrip_from_raw_values() {
        assert_eq!(
            NativeResourceKind::from_raw(NativeResourceKind::Device as u32),
            Some(NativeResourceKind::Device)
        );
        assert_eq!(
            NativeContractKind::from_raw(NativeContractKind::Display as u32),
            Some(NativeContractKind::Display)
        );
        assert_eq!(NativeResourceKind::from_raw(99), None);
        assert_eq!(NativeContractKind::from_raw(99), None);
        assert_eq!(
            NativeContractState::from_raw(NativeContractState::Active as u32),
            Some(NativeContractState::Active)
        );
        assert_eq!(
            NativeResourceArbitrationPolicy::from_raw(NativeResourceArbitrationPolicy::Fifo as u32),
            Some(NativeResourceArbitrationPolicy::Fifo)
        );
        assert_eq!(
            NativeResourceGovernanceMode::from_raw(
                NativeResourceGovernanceMode::ExclusiveLease as u32
            ),
            Some(NativeResourceGovernanceMode::ExclusiveLease)
        );
        assert_eq!(
            NativeResourceContractPolicy::from_raw(NativeResourceContractPolicy::Display as u32),
            Some(NativeResourceContractPolicy::Display)
        );
        assert_eq!(
            NativeResourceIssuerPolicy::from_raw(NativeResourceIssuerPolicy::CreatorOnly as u32),
            Some(NativeResourceIssuerPolicy::CreatorOnly)
        );
        assert_eq!(
            NativeResourceState::from_raw(NativeResourceState::Suspended as u32),
            Some(NativeResourceState::Suspended)
        );
        assert_eq!(NativeContractState::from_raw(99), None);
        assert_eq!(NativeResourceArbitrationPolicy::from_raw(99), None);
        assert_eq!(NativeResourceGovernanceMode::from_raw(99), None);
        assert_eq!(NativeResourceContractPolicy::from_raw(99), None);
        assert_eq!(NativeResourceIssuerPolicy::from_raw(99), None);
        assert_eq!(NativeResourceState::from_raw(99), None);
        assert_eq!(
            BootSessionStatus::from_raw(BootSessionStatus::Success as u32),
            Some(BootSessionStatus::Success)
        );
        assert_eq!(
            BootSessionStage::from_raw(BootSessionStage::Complete as u32),
            Some(BootSessionStage::Complete)
        );
        assert_eq!(BootSessionStatus::from_raw(99), None);
        assert_eq!(BootSessionStage::from_raw(99), None);
    }

    #[test]
    fn validate_rights_allows_and_denies_expected_masks() {
        let available = BlockRightsMask::READ.union(BlockRightsMask::WRITE);
        assert!(validate_rights(available, BlockRightsMask::READ).is_ok());
        let err = validate_rights(available, BlockRightsMask::SUBMIT).unwrap_err();
        assert_eq!(err.code, SecurityErrorCode::RightsDenied);
        assert_eq!(
            required_block_rights_for_op(NATIVE_BLOCK_IO_OP_READ),
            Some(BlockRightsMask::READ.union(BlockRightsMask::SUBMIT))
        );
        assert_eq!(
            required_block_rights_for_op(NATIVE_BLOCK_IO_OP_WRITE),
            Some(BlockRightsMask::WRITE.union(BlockRightsMask::SUBMIT))
        );
        assert_eq!(required_block_rights_for_op(0xffff), None);
    }

    #[test]
    fn ifc_read_and_write_enforce_lattice_rules() {
        let readable_object =
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        assert!(check_ifc_read(TEST_LABEL_HIGH, readable_object).is_ok());
        assert_eq!(
            check_ifc_read(TEST_LABEL_LOW, TEST_LABEL_HIGH)
                .unwrap_err()
                .code,
            SecurityErrorCode::LabelReadDenied
        );

        assert!(check_ifc_write(TEST_LABEL_HIGH, TEST_LABEL_HIGH).is_ok());
        assert_eq!(
            check_ifc_write(TEST_LABEL_HIGH, TEST_LABEL_LOW)
                .unwrap_err()
                .code,
            SecurityErrorCode::LabelWriteDenied
        );
    }

    #[test]
    fn join_labels_preserves_confidentiality_and_integrity_invariants() {
        let joined = join_labels(TEST_LABEL_LOW, TEST_LABEL_HIGH);
        assert_eq!(joined.confidentiality, ConfidentialityLevel::Secret);
        assert_eq!(joined.integrity, IntegrityLevel::Verified);

        let joined_reverse = join_labels(TEST_LABEL_HIGH, TEST_LABEL_LOW);
        assert_eq!(joined, joined_reverse);
    }

    #[test]
    fn provenance_derivation_is_stable() {
        let subject = SubjectSecurityContext {
            subject_id: 7,
            active_issuer_id: 11,
            rights_ceiling: BlockRightsMask::ALL,
            label: TEST_LABEL_HIGH,
            session_nonce: 0xabc,
            current_epoch: 100,
            minimum_revocation_epoch: 3,
            max_delegation_depth: 4,
        };
        let object = ObjectSecurityContext {
            object_id: 41,
            required_rights: BlockRightsMask::READ,
            minimum_label: TEST_LABEL_LOW,
            current_label: TEST_LABEL_HIGH,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 41,
                parent_origin_id: 0,
                parent_measurement: [0x44; 32],
                edge_id: 1,
                measurement: TEST_INTEGRITY_A,
            },
            integrity: TEST_INTEGRITY_A,
            revocation_epoch: 2,
            max_delegation_depth: 4,
        };
        let token = CapabilityToken {
            object_id: 41,
            rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            issuer_id: 11,
            subject_id: 7,
            generation: 3,
            revocation_epoch: 3,
            delegation_depth: 0,
            delegated: 0,
            nonce: 9,
            expiry_epoch: 101,
            authenticator: TEST_INTEGRITY_A,
        };

        let req_a = derive_request_provenance(&subject, &object, &token, TEST_INTEGRITY_A, 99);
        let req_b = derive_request_provenance(&subject, &object, &token, TEST_INTEGRITY_A, 99);
        assert_eq!(req_a, req_b);

        let completion_a = derive_completion_provenance(&req_a, 200, TEST_INTEGRITY_B, 77);
        let completion_b = derive_completion_provenance(&req_b, 200, TEST_INTEGRITY_B, 77);
        assert_eq!(completion_a, completion_b);
        assert_eq!(completion_a.parent_measurement, TEST_INTEGRITY_A.bytes);
    }

    #[test]
    fn integrity_tag_verification_detects_mismatch() {
        assert!(verify_integrity_tag(&TEST_INTEGRITY_A, &TEST_INTEGRITY_A).is_ok());
        let err = verify_integrity_tag(&TEST_INTEGRITY_A, &TEST_INTEGRITY_B).unwrap_err();
        assert_eq!(err.code, SecurityErrorCode::IntegrityMismatch);
    }

    #[test]
    fn capability_check_validates_identity_rights_and_integrity() {
        let subject = SubjectSecurityContext {
            subject_id: 5,
            active_issuer_id: 9,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label: TEST_LABEL_HIGH,
            session_nonce: 1,
            current_epoch: 50,
            minimum_revocation_epoch: 2,
            max_delegation_depth: 2,
        };
        let object = ObjectSecurityContext {
            object_id: 12,
            required_rights: BlockRightsMask::READ,
            minimum_label: TEST_LABEL_LOW,
            current_label: TEST_LABEL_LOW,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 12,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 0,
                measurement: TEST_INTEGRITY_A,
            },
            integrity: TEST_INTEGRITY_A,
            revocation_epoch: 2,
            max_delegation_depth: 2,
        };
        let token = CapabilityToken {
            object_id: 12,
            rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            issuer_id: 9,
            subject_id: 5,
            generation: 1,
            revocation_epoch: 2,
            delegation_depth: 0,
            delegated: 0,
            nonce: 2,
            expiry_epoch: 51,
            authenticator: TEST_INTEGRITY_A,
        };

        assert!(
            check_capability(
                &subject,
                &object,
                &token,
                BlockRightsMask::READ,
                &TEST_INTEGRITY_A
            )
            .is_ok()
        );

        let err = check_capability(
            &subject,
            &object,
            &token,
            BlockRightsMask::WRITE,
            &TEST_INTEGRITY_A,
        )
        .unwrap_err();
        assert_eq!(err.code, SecurityErrorCode::RightsDenied);

        let err = check_capability(
            &subject,
            &object,
            &token,
            BlockRightsMask::READ,
            &TEST_INTEGRITY_B,
        )
        .unwrap_err();
        assert_eq!(err.code, SecurityErrorCode::IntegrityMismatch);
        assert!(!token.is_expired(subject.current_epoch));
        assert!(token.covers(BlockRightsMask::READ));
    }

    #[test]
    fn block_request_validation_enforces_security_model() {
        let subject = SubjectSecurityContext {
            subject_id: 21,
            active_issuer_id: 22,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label: SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified),
            session_nonce: 1,
            current_epoch: 9,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 2,
        };
        let object = ObjectSecurityContext {
            object_id: 33,
            required_rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            minimum_label: TEST_LABEL_LOW,
            current_label: TEST_LABEL_LOW,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 33,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 7,
                measurement: TEST_INTEGRITY_A,
            },
            integrity: TEST_INTEGRITY_A,
            revocation_epoch: 1,
            max_delegation_depth: 2,
        };
        let capability = CapabilityToken {
            object_id: 33,
            rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            issuer_id: 22,
            subject_id: 21,
            generation: 1,
            revocation_epoch: 1,
            delegation_depth: 0,
            delegated: 0,
            nonce: 2,
            expiry_epoch: 10,
            authenticator: TEST_INTEGRITY_A,
        };
        let request = NativeBlockIoRequest::new(
            NATIVE_BLOCK_IO_OP_READ,
            128,
            4,
            512,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            capability,
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel),
            derive_request_provenance(&subject, &object, &capability, TEST_INTEGRITY_A, 99),
            TEST_INTEGRITY_A,
        );

        assert!(request.validate_security(&subject, &object).is_ok());

        let invalid = NativeBlockIoRequest {
            sector_count: 0,
            ..request
        };
        assert_eq!(
            invalid
                .validate_security(&subject, &object)
                .unwrap_err()
                .code,
            SecurityErrorCode::InvalidSecurityState
        );
    }

    #[test]
    fn block_completion_validation_detects_downgrade_and_lineage_break() {
        let request = NativeBlockIoRequest::new(
            NATIVE_BLOCK_IO_OP_READ,
            1,
            1,
            512,
            BlockRightsMask::READ,
            CapabilityToken {
                object_id: 1,
                rights: BlockRightsMask::READ,
                issuer_id: 2,
                subject_id: 3,
                generation: 4,
                revocation_epoch: 1,
                delegation_depth: 0,
                delegated: 0,
                nonce: 5,
                expiry_epoch: 6,
                authenticator: TEST_INTEGRITY_A,
            },
            SecurityLabel::new(ConfidentialityLevel::Sensitive, IntegrityLevel::Verified),
            ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Request,
                reserved0: 0,
                origin_id: 77,
                parent_origin_id: 1,
                parent_measurement: [0; 32],
                edge_id: 10,
                measurement: TEST_INTEGRITY_A,
            },
            TEST_INTEGRITY_A,
        );
        let valid = NativeBlockIoCompletion::new(
            NATIVE_BLOCK_IO_OP_READ,
            0,
            512,
            512,
            BlockRightsMask::READ,
            SecurityLabel::new(ConfidentialityLevel::Sensitive, IntegrityLevel::Verified),
            ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Completion,
                reserved0: 0,
                origin_id: 88,
                parent_origin_id: 77,
                parent_measurement: TEST_INTEGRITY_A.bytes,
                edge_id: 11,
                measurement: TEST_INTEGRITY_B,
            },
            TEST_INTEGRITY_B,
        );
        assert!(valid.preserves_security(&request).is_ok());

        let downgraded = NativeBlockIoCompletion {
            label: SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel),
            ..valid
        };
        assert_eq!(
            downgraded.preserves_security(&request).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );

        let broken_lineage = NativeBlockIoCompletion {
            provenance: ProvenanceTag {
                parent_origin_id: 999,
                ..valid.provenance
            },
            ..valid
        };
        assert_eq!(
            broken_lineage
                .preserves_security(&request)
                .unwrap_err()
                .code,
            SecurityErrorCode::ProvenanceMismatch
        );

        let wrong_rights = NativeBlockIoCompletion {
            rights: BlockRightsMask::READ.union(BlockRightsMask::WRITE),
            ..valid
        };
        assert_eq!(
            wrong_rights.preserves_security(&request).unwrap_err().code,
            SecurityErrorCode::RightsDenied
        );
    }

    #[test]
    fn compose_helpers_build_valid_request_and_completion() {
        let subject = SubjectSecurityContext {
            subject_id: 90,
            active_issuer_id: 91,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label: SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified),
            session_nonce: 7,
            current_epoch: 80,
            minimum_revocation_epoch: 4,
            max_delegation_depth: 4,
        };
        let object = ObjectSecurityContext {
            object_id: 92,
            required_rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            minimum_label: SecurityLabel::new(
                ConfidentialityLevel::Internal,
                IntegrityLevel::Kernel,
            ),
            current_label: SecurityLabel::new(
                ConfidentialityLevel::Internal,
                IntegrityLevel::Kernel,
            ),
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 92,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 12,
                measurement: TEST_INTEGRITY_A,
            },
            integrity: TEST_INTEGRITY_A,
            revocation_epoch: 4,
            max_delegation_depth: 4,
        };
        let capability = CapabilityToken::new(
            92,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            91,
            90,
            2,
            4,
            0,
            false,
            3,
            81,
            TEST_INTEGRITY_A,
        );
        let request = compose_block_request(
            &subject,
            &object,
            capability,
            NATIVE_BLOCK_IO_OP_READ,
            64,
            8,
            512,
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel),
            TEST_INTEGRITY_A,
            55,
        )
        .unwrap();
        assert_eq!(
            request.rights,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
        );

        let completion = compose_block_completion(
            &request,
            object.object_id,
            0,
            4096,
            request.label,
            TEST_INTEGRITY_B,
            56,
        )
        .unwrap();
        assert_eq!(
            completion.provenance.parent_origin_id,
            request.provenance.origin_id
        );
        assert!(request.is_read());
        assert!(!request.is_write());
        assert_eq!(
            request.required_rights().unwrap(),
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
        );
        assert!(completion.is_success());
    }

    #[test]
    fn subject_and_object_context_validation_rejects_invalid_state() {
        let subject = SubjectSecurityContext {
            subject_id: 0,
            active_issuer_id: 1,
            rights_ceiling: BlockRightsMask::READ,
            label: TEST_LABEL_LOW,
            session_nonce: 0,
            current_epoch: 1,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        assert_eq!(
            validate_subject_context(&subject).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );

        let object = ObjectSecurityContext {
            object_id: 1,
            required_rights: BlockRightsMask::READ,
            minimum_label: SecurityLabel::new(
                ConfidentialityLevel::Sensitive,
                IntegrityLevel::Verified,
            ),
            current_label: SecurityLabel::new(
                ConfidentialityLevel::Internal,
                IntegrityLevel::Kernel,
            ),
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 1,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 0,
                measurement: TEST_INTEGRITY_A,
            },
            integrity: TEST_INTEGRITY_A,
            revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        assert_eq!(
            validate_object_context(&object).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );
    }

    #[test]
    fn label_transition_and_errno_mapping_are_explicit() {
        let from = SecurityLabel::new(ConfidentialityLevel::Sensitive, IntegrityLevel::Verified);
        let to_ok = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        let to_bad = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);

        assert!(validate_label_transition(from, to_ok).is_ok());
        assert_eq!(
            validate_label_transition(from, to_bad).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );
        assert_eq!(
            security_error_to_errno(SecurityErrorCode::RightsDenied),
            Errno::Access
        );
        assert_eq!(
            security_error_to_errno(SecurityErrorCode::IntegrityMismatch),
            Errno::Io
        );
        assert_eq!(
            security_error_to_errno(SecurityErrorCode::InvalidSecurityState),
            Errno::Inval
        );
    }

    #[test]
    fn provenance_and_integrity_validation_reject_invalid_tags() {
        let invalid_provenance = ProvenanceTag {
            origin_kind: ProvenanceOriginKind::Request,
            reserved0: 0,
            origin_id: 0,
            parent_origin_id: 0,
            parent_measurement: [0; 32],
            edge_id: 1,
            measurement: TEST_INTEGRITY_A,
        };
        assert_eq!(
            validate_provenance_tag(&invalid_provenance)
                .unwrap_err()
                .code,
            SecurityErrorCode::InvalidSecurityState
        );

        let invalid_integrity = IntegrityTag::zeroed(IntegrityTagKind::None);
        assert_eq!(
            validate_integrity_tag(&invalid_integrity).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );
    }

    #[test]
    fn effective_labels_preserve_conservative_security_state() {
        let request = derive_effective_request_label(TEST_LABEL_LOW, TEST_LABEL_HIGH);
        assert_eq!(request.confidentiality, ConfidentialityLevel::Secret);
        assert_eq!(request.integrity, IntegrityLevel::Verified);

        let completion = derive_effective_completion_label(
            request,
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified),
        )
        .unwrap();
        assert_eq!(completion, request);

        assert_eq!(
            derive_effective_completion_label(
                request,
                SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel),
            )
            .unwrap_err()
            .code,
            SecurityErrorCode::InvalidSecurityState
        );
    }

    #[test]
    fn security_constructors_build_consistent_contexts_and_lineage() {
        let tag = TEST_INTEGRITY_A;
        let root = ProvenanceTag::root(ProvenanceOriginKind::Device, 500, 10, tag);
        let child = ProvenanceTag::child(ProvenanceOriginKind::Request, 501, &root, 11, tag);
        assert_eq!(child.parent_origin_id, 500);
        assert_eq!(child.parent_measurement, tag.bytes);

        let subject = SubjectSecurityContext::new(
            100,
            101,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            TEST_LABEL_HIGH,
            1,
            2,
            1,
            2,
        );
        let capability = CapabilityToken::new(
            102,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            101,
            100,
            3,
            1,
            0,
            false,
            4,
            5,
            tag,
        );
        let object = ObjectSecurityContext::new(
            102,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            TEST_LABEL_LOW,
            TEST_LABEL_LOW,
            root,
            tag,
            1,
            2,
        );
        assert_eq!(subject.subject_id, 100);
        assert_eq!(capability.subject_id, 100);
        assert_eq!(object.object_id, 102);
    }

    #[test]
    fn capability_token_validation_rejects_empty_or_expired_tokens() {
        let valid = CapabilityToken::new(
            1,
            BlockRightsMask::READ,
            2,
            3,
            4,
            1,
            0,
            false,
            5,
            10,
            TEST_INTEGRITY_A,
        );
        assert!(validate_capability_token(&valid, 9).is_ok());

        let empty = CapabilityToken::new(
            1,
            BlockRightsMask::NONE,
            2,
            3,
            4,
            1,
            0,
            false,
            5,
            10,
            TEST_INTEGRITY_A,
        );
        assert_eq!(
            validate_capability_token(&empty, 9).unwrap_err().code,
            SecurityErrorCode::RightsDenied
        );

        let expired = CapabilityToken::new(
            1,
            BlockRightsMask::READ,
            2,
            3,
            4,
            1,
            0,
            false,
            5,
            8,
            TEST_INTEGRITY_A,
        );
        assert_eq!(
            validate_capability_token(&expired, 9).unwrap_err().code,
            SecurityErrorCode::CapabilityExpired
        );
    }

    #[test]
    fn revocation_and_delegation_are_enforced() {
        let subject = SubjectSecurityContext::new(
            200,
            201,
            BlockRightsMask::READ
                .union(BlockRightsMask::SUBMIT)
                .union(BlockRightsMask::DELEGATE),
            TEST_LABEL_HIGH,
            1,
            50,
            5,
            1,
        );
        let object = ObjectSecurityContext::new(
            202,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            TEST_LABEL_LOW,
            TEST_LABEL_LOW,
            ProvenanceTag::root(ProvenanceOriginKind::Device, 202, 1, TEST_INTEGRITY_A),
            TEST_INTEGRITY_A,
            5,
            1,
        );
        let parent = CapabilityToken::new(
            202,
            BlockRightsMask::READ
                .union(BlockRightsMask::SUBMIT)
                .union(BlockRightsMask::DELEGATE),
            201,
            200,
            1,
            5,
            0,
            false,
            2,
            51,
            TEST_INTEGRITY_A,
        );
        assert!(validate_revocation(&subject, &object, &parent).is_ok());

        let delegated = delegate_capability(
            &parent,
            300,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            9,
            51,
            TEST_INTEGRITY_B,
        )
        .unwrap();
        assert_eq!(delegated.delegation_depth, 1);
        assert_eq!(delegated.delegated, 1);
        assert!(
            validate_delegation(
                &subject,
                &object,
                &delegated,
                BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
            )
            .is_ok()
        );

        let revoked = CapabilityToken::new(
            202,
            BlockRightsMask::READ,
            201,
            200,
            1,
            4,
            0,
            false,
            2,
            51,
            TEST_INTEGRITY_A,
        );
        assert_eq!(
            validate_revocation(&subject, &object, &revoked)
                .unwrap_err()
                .code,
            SecurityErrorCode::CapabilityRevoked
        );
    }
}
