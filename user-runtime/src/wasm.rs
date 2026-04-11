use alloc::{string::String, vec, vec::Vec};
use core::str;

use crate::Runtime;
use ngos_user_abi::{Errno, SyscallBackend};

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const WASM_VERSION: &[u8; 4] = &[0x01, 0x00, 0x00, 0x00];
const SECTION_CUSTOM: u8 = 0;
const SECTION_TYPE: u8 = 1;
const SECTION_IMPORT: u8 = 2;
const SECTION_FUNCTION: u8 = 3;
const SECTION_MEMORY: u8 = 5;
const SECTION_EXPORT: u8 = 7;
const SECTION_CODE: u8 = 10;
const KIND_FUNCTION: u8 = 0x00;
const KIND_MEMORY: u8 = 0x02;
const VALTYPE_I32: u8 = 0x7f;
const VALTYPE_I64: u8 = 0x7e;
const BLOCKTYPE_EMPTY: u8 = 0x40;
const OPCODE_UNREACHABLE: u8 = 0x00;
const OPCODE_NOP: u8 = 0x01;
const OPCODE_LOOP: u8 = 0x03;
const OPCODE_IF: u8 = 0x04;
const OPCODE_ELSE: u8 = 0x05;
const OPCODE_END: u8 = 0x0b;
const OPCODE_BR: u8 = 0x0c;
const OPCODE_BR_IF: u8 = 0x0d;
const OPCODE_RETURN: u8 = 0x0f;
const OPCODE_CALL: u8 = 0x10;
const OPCODE_DROP: u8 = 0x1a;
const OPCODE_SELECT: u8 = 0x1b;
const OPCODE_LOCAL_GET: u8 = 0x20;
const OPCODE_LOCAL_SET: u8 = 0x21;
const OPCODE_LOCAL_TEE: u8 = 0x22;
const OPCODE_I32_LOAD: u8 = 0x28;
const OPCODE_I64_LOAD: u8 = 0x29;
const OPCODE_I32_STORE: u8 = 0x36;
const OPCODE_I64_STORE: u8 = 0x37;
const OPCODE_I32_CONST: u8 = 0x41;
const OPCODE_I64_CONST: u8 = 0x42;
const OPCODE_I32_EQZ: u8 = 0x45;
const OPCODE_I64_EQZ: u8 = 0x50;
const OPCODE_I32_EQ: u8 = 0x46;
const OPCODE_I32_NE: u8 = 0x47;
const OPCODE_I32_LT_S: u8 = 0x48;
const OPCODE_I32_ADD: u8 = 0x6a;
const OPCODE_I32_SUB: u8 = 0x6b;
const OPCODE_I32_MUL: u8 = 0x6c;
const OPCODE_MEMORY_SIZE: u8 = 0x3f;
const OPCODE_MEMORY_GROW: u8 = 0x40;
const EXPORT_RUN: &str = "run";
const IMPORT_MODULE: &str = "ngos";
const IMPORT_OBSERVE_PROCESS_CAPABILITY_COUNT: &str = "observe-process-capability-count";
const IMPORT_OBSERVE_SYSTEM_PROCESS_COUNT: &str = "observe-system-process-count";
const IMPORT_OBSERVE_PROCESS_STATUS_BYTES: &str = "observe-process-status-bytes";
const IMPORT_OBSERVE_PROCESS_CWD_ROOT: &str = "observe-process-cwd-root";

pub const WASM_BOOT_PROOF_COMPONENT: &[u8] = &[
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x09, 0x02, 0x60, 0x00, 0x01, 0x7E, 0x60,
    0x00, 0x01, 0x7F, 0x02, 0x4D, 0x02, 0x04, 0x6E, 0x67, 0x6F, 0x73, 0x20, 0x6F, 0x62, 0x73, 0x65,
    0x72, 0x76, 0x65, 0x2D, 0x70, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x2D, 0x63, 0x61, 0x70, 0x61,
    0x62, 0x69, 0x6C, 0x69, 0x74, 0x79, 0x2D, 0x63, 0x6F, 0x75, 0x6E, 0x74, 0x00, 0x00, 0x04, 0x6E,
    0x67, 0x6F, 0x73, 0x1C, 0x6F, 0x62, 0x73, 0x65, 0x72, 0x76, 0x65, 0x2D, 0x73, 0x79, 0x73, 0x74,
    0x65, 0x6D, 0x2D, 0x70, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x2D, 0x63, 0x6F, 0x75, 0x6E, 0x74,
    0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6E, 0x00, 0x02, 0x0A,
    0x18, 0x01, 0x16, 0x00, 0x10, 0x00, 0x50, 0x04, 0x7F, 0x41, 0x02, 0x05, 0x10, 0x01, 0x50, 0x04,
    0x7F, 0x41, 0x01, 0x05, 0x41, 0x00, 0x0B, 0x0B, 0x0B,
];

pub const WASM_PROCESS_IDENTITY_COMPONENT: &[u8] = &[
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x09, 0x02, 0x60, 0x00, 0x01, 0x7E, 0x60,
    0x00, 0x01, 0x7F, 0x02, 0x45, 0x02, 0x04, 0x6E, 0x67, 0x6F, 0x73, 0x1C, 0x6F, 0x62, 0x73, 0x65,
    0x72, 0x76, 0x65, 0x2D, 0x70, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x2D, 0x73, 0x74, 0x61, 0x74,
    0x75, 0x73, 0x2D, 0x62, 0x79, 0x74, 0x65, 0x73, 0x00, 0x00, 0x04, 0x6E, 0x67, 0x6F, 0x73, 0x18,
    0x6F, 0x62, 0x73, 0x65, 0x72, 0x76, 0x65, 0x2D, 0x70, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x2D,
    0x63, 0x77, 0x64, 0x2D, 0x72, 0x6F, 0x6F, 0x74, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x07,
    0x01, 0x03, 0x72, 0x75, 0x6E, 0x00, 0x02, 0x0A, 0x18, 0x01, 0x16, 0x00, 0x10, 0x00, 0x50, 0x04,
    0x7F, 0x41, 0x02, 0x05, 0x10, 0x01, 0x50, 0x04, 0x7F, 0x41, 0x01, 0x05, 0x41, 0x00, 0x0B, 0x0B,
    0x0B,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmCapability {
    ObserveProcessCapabilityCount,
    ObserveSystemProcessCount,
    ObserveProcessStatusBytes,
    ObserveProcessCwdRoot,
}

impl WasmCapability {
    pub const fn import_name(self) -> &'static str {
        match self {
            Self::ObserveProcessCapabilityCount => IMPORT_OBSERVE_PROCESS_CAPABILITY_COUNT,
            Self::ObserveSystemProcessCount => IMPORT_OBSERVE_SYSTEM_PROCESS_COUNT,
            Self::ObserveProcessStatusBytes => IMPORT_OBSERVE_PROCESS_STATUS_BYTES,
            Self::ObserveProcessCwdRoot => IMPORT_OBSERVE_PROCESS_CWD_ROOT,
        }
    }

    pub const fn marker_name(self) -> &'static str {
        match self {
            Self::ObserveProcessCapabilityCount => "observe-process-capability-count",
            Self::ObserveSystemProcessCount => "observe-system-process-count",
            Self::ObserveProcessStatusBytes => "observe-process-status-bytes",
            Self::ObserveProcessCwdRoot => "observe-process-cwd-root",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmVerdict {
    Ready,
    IdleSystem,
    UnboundProcessCapability,
}

impl WasmVerdict {
    pub const fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Ready),
            1 => Some(Self::IdleSystem),
            2 => Some(Self::UnboundProcessCapability),
            _ => None,
        }
    }

    pub const fn marker_name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::IdleSystem => "idle-system",
            Self::UnboundProcessCapability => "unbound-process-capability",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmObservation {
    pub pid: u64,
    pub process_capability_count: u64,
    pub process_count: u64,
    pub process_status_bytes: u64,
    pub process_cwd_root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExecutionReport {
    pub observation: WasmObservation,
    pub verdict: WasmVerdict,
    pub granted_capabilities: Vec<WasmCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmExecutionError {
    MissingCapability {
        capability: WasmCapability,
        import: String,
    },
    MissingExport {
        export: String,
    },
    UnsupportedImport {
        module: String,
        name: String,
    },
    UnsupportedSection {
        id: u8,
    },
    InvalidModule {
        reason: &'static str,
    },
    Trap {
        reason: &'static str,
    },
    Syscall(Errno),
}

impl From<Errno> for WasmExecutionError {
    fn from(value: Errno) -> Self {
        Self::Syscall(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueType {
    I32,
    I64,
}

#[derive(Clone, Copy)]
struct FunctionType {
    params: u32,
    result: Option<ValueType>,
}

#[derive(Clone)]
struct ImportFunction {
    module: String,
    name: String,
    type_index: u32,
}

#[derive(Clone)]
struct WasmFunction<'a> {
    type_index: u32,
    body: &'a [u8],
}

#[derive(Clone)]
struct ParsedModule<'a> {
    types: Vec<FunctionType>,
    imports: Vec<ImportFunction>,
    functions: Vec<WasmFunction<'a>>,
    run_function_index: u32,
}

#[derive(Clone, Copy)]
enum Value {
    I32(i32),
    I64(i64),
}

const WASM_PAGE_SIZE: usize = 65536;
const WASM_MAX_PAGES: usize = 65536;

pub fn execute_wasm_component<B: SyscallBackend>(
    runtime: &Runtime<B>,
    artifact: &[u8],
    pid: u64,
    granted_capabilities: &[WasmCapability],
) -> Result<WasmExecutionReport, WasmExecutionError> {
    let module = parse_module(artifact)?;
    for import in &module.imports {
        let capability = capability_for_import(&import.module, &import.name)?;
        if !granted_capabilities.contains(&capability) {
            return Err(WasmExecutionError::MissingCapability {
                capability,
                import: import.name.clone(),
            });
        }
    }

    let process = runtime.inspect_process(pid)?;
    let mut pids = [0u64; 32];
    let process_count = runtime.list_processes(&mut pids)? as u64;
    let mut status = [0u8; 256];
    let process_status_bytes = runtime.read_procfs("/proc/1/status", &mut status)? as u64;
    let mut cwd = [0u8; 64];
    let cwd_len = runtime.get_process_cwd(pid, &mut cwd)?;
    let process_cwd_root = &cwd[..cwd_len] == b"/";
    let observation = WasmObservation {
        pid,
        process_capability_count: process.capability_count,
        process_count,
        process_status_bytes,
        process_cwd_root,
    };
    let mut stack = Vec::with_capacity(16);
    let mut memory = vec![0u8; WASM_PAGE_SIZE];
    let raw_verdict = execute_function(
        &module,
        module.run_function_index,
        &observation,
        &mut stack,
        &mut memory,
    )?;
    let verdict = WasmVerdict::from_code(raw_verdict).ok_or(WasmExecutionError::Trap {
        reason: "unknown verdict code",
    })?;
    Ok(WasmExecutionReport {
        observation,
        verdict,
        granted_capabilities: granted_capabilities.to_vec(),
    })
}

/// Load and execute a WASM module from the filesystem.
pub fn execute_wasm_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    pid: u64,
    granted_capabilities: &[WasmCapability],
) -> Result<WasmExecutionReport, WasmExecutionError> {
    let fd = runtime.open_path(path).map_err(|e| match e {
        Errno::NoEnt => WasmExecutionError::InvalidModule {
            reason: "wasm file not found",
        },
        Errno::Access => WasmExecutionError::InvalidModule {
            reason: "permission denied",
        },
        _ => WasmExecutionError::Syscall(e),
    })?;

    let mut buffer = [0u8; 65536];
    let len = runtime.read(fd, &mut buffer).map_err(WasmExecutionError::Syscall)?;
    runtime.close(fd).ok();

    execute_wasm_component(runtime, &buffer[..len], pid, granted_capabilities)
}

fn capability_for_import(module: &str, name: &str) -> Result<WasmCapability, WasmExecutionError> {
    if module != IMPORT_MODULE {
        return Err(WasmExecutionError::UnsupportedImport {
            module: module.into(),
            name: name.into(),
        });
    }
    match name {
        IMPORT_OBSERVE_PROCESS_CAPABILITY_COUNT => {
            Ok(WasmCapability::ObserveProcessCapabilityCount)
        }
        IMPORT_OBSERVE_SYSTEM_PROCESS_COUNT => Ok(WasmCapability::ObserveSystemProcessCount),
        IMPORT_OBSERVE_PROCESS_STATUS_BYTES => Ok(WasmCapability::ObserveProcessStatusBytes),
        IMPORT_OBSERVE_PROCESS_CWD_ROOT => Ok(WasmCapability::ObserveProcessCwdRoot),
        _ => Err(WasmExecutionError::UnsupportedImport {
            module: module.into(),
            name: name.into(),
        }),
    }
}

fn parse_module(bytes: &[u8]) -> Result<ParsedModule<'_>, WasmExecutionError> {
    if bytes.len() < 8 || &bytes[..4] != WASM_MAGIC || &bytes[4..8] != WASM_VERSION {
        return Err(WasmExecutionError::InvalidModule {
            reason: "invalid wasm header",
        });
    }

    let mut offset = 8usize;
    let mut types = Vec::new();
    let mut imports = Vec::new();
    let mut function_type_indices = Vec::new();
    let mut export_run_index = None;
    let mut function_bodies = Vec::new();

    while offset < bytes.len() {
        let section_id = read_u8(bytes, &mut offset)?;
        let section_len = read_uleb(bytes, &mut offset)? as usize;
        let end = offset
            .checked_add(section_len)
            .ok_or(WasmExecutionError::InvalidModule {
                reason: "section overflow",
            })?;
        if end > bytes.len() {
            return Err(WasmExecutionError::InvalidModule {
                reason: "truncated section",
            });
        }
        let section = &bytes[offset..end];
        offset = end;
        match section_id {
            SECTION_CUSTOM => {}
            SECTION_TYPE => types = parse_type_section(section)?,
            SECTION_IMPORT => imports = parse_import_section(section)?,
            SECTION_FUNCTION => function_type_indices = parse_function_section(section)?,
            SECTION_EXPORT => export_run_index = parse_export_section(section)?,
            SECTION_CODE => function_bodies = parse_code_section(section)?,
            id => {
                return Err(WasmExecutionError::UnsupportedSection { id });
            }
        }
    }

    if function_type_indices.len() != function_bodies.len() {
        return Err(WasmExecutionError::InvalidModule {
            reason: "function/code length mismatch",
        });
    }
    let run_function_index = export_run_index.ok_or_else(|| WasmExecutionError::MissingExport {
        export: EXPORT_RUN.into(),
    })?;
    let mut functions = Vec::with_capacity(function_bodies.len());
    for (type_index, body) in function_type_indices
        .into_iter()
        .zip(function_bodies.into_iter())
    {
        functions.push(WasmFunction { type_index, body });
    }
    Ok(ParsedModule {
        types,
        imports,
        functions,
        run_function_index,
    })
}

fn parse_type_section(section: &[u8]) -> Result<Vec<FunctionType>, WasmExecutionError> {
    let mut offset = 0usize;
    let count = read_uleb(section, &mut offset)? as usize;
    let mut types = Vec::with_capacity(count);
    for _ in 0..count {
        if read_u8(section, &mut offset)? != 0x60 {
            return Err(WasmExecutionError::InvalidModule {
                reason: "unsupported type form",
            });
        }
        let param_count = read_uleb(section, &mut offset)?;
        for _ in 0..param_count {
            let _ = read_val_type(section, &mut offset)?;
        }
        let result_count = read_uleb(section, &mut offset)?;
        let result = match result_count {
            0 => None,
            1 => Some(read_val_type(section, &mut offset)?),
            _ => {
                return Err(WasmExecutionError::InvalidModule {
                    reason: "multi-value results are unsupported",
                });
            }
        };
        types.push(FunctionType {
            params: param_count,
            result,
        });
    }
    ensure_consumed(section, offset)?;
    Ok(types)
}

fn parse_import_section(section: &[u8]) -> Result<Vec<ImportFunction>, WasmExecutionError> {
    let mut offset = 0usize;
    let count = read_uleb(section, &mut offset)? as usize;
    let mut imports = Vec::with_capacity(count);
    for _ in 0..count {
        let module = read_name(section, &mut offset)?;
        let name = read_name(section, &mut offset)?;
        let kind = read_u8(section, &mut offset)?;
        if kind != KIND_FUNCTION {
            return Err(WasmExecutionError::InvalidModule {
                reason: "non-function imports are unsupported",
            });
        }
        let type_index = read_uleb(section, &mut offset)?;
        imports.push(ImportFunction {
            module,
            name,
            type_index,
        });
    }
    ensure_consumed(section, offset)?;
    Ok(imports)
}

fn parse_function_section(section: &[u8]) -> Result<Vec<u32>, WasmExecutionError> {
    let mut offset = 0usize;
    let count = read_uleb(section, &mut offset)? as usize;
    let mut functions = Vec::with_capacity(count);
    for _ in 0..count {
        functions.push(read_uleb(section, &mut offset)?);
    }
    ensure_consumed(section, offset)?;
    Ok(functions)
}

fn parse_export_section(section: &[u8]) -> Result<Option<u32>, WasmExecutionError> {
    let mut offset = 0usize;
    let count = read_uleb(section, &mut offset)? as usize;
    let mut run = None;
    for _ in 0..count {
        let name = read_name(section, &mut offset)?;
        let kind = read_u8(section, &mut offset)?;
        let index = read_uleb(section, &mut offset)?;
        if kind == KIND_FUNCTION && name == EXPORT_RUN {
            run = Some(index);
        }
    }
    ensure_consumed(section, offset)?;
    Ok(run)
}

fn parse_code_section(section: &[u8]) -> Result<Vec<&[u8]>, WasmExecutionError> {
    let mut offset = 0usize;
    let count = read_uleb(section, &mut offset)? as usize;
    let mut bodies = Vec::with_capacity(count);
    for _ in 0..count {
        let body_len = read_uleb(section, &mut offset)? as usize;
        let body_end = offset
            .checked_add(body_len)
            .ok_or(WasmExecutionError::InvalidModule {
                reason: "code body overflow",
            })?;
        if body_end > section.len() {
            return Err(WasmExecutionError::InvalidModule {
                reason: "truncated code body",
            });
        }
        bodies.push(&section[offset..body_end]);
        offset = body_end;
    }
    ensure_consumed(section, offset)?;
    Ok(bodies)
}

fn execute_function(
    module: &ParsedModule<'_>,
    function_index: u32,
    observation: &WasmObservation,
    stack: &mut Vec<Value>,
    memory: &mut Vec<u8>,
) -> Result<i32, WasmExecutionError> {
    if function_index < module.imports.len() as u32 {
        return Err(WasmExecutionError::Trap {
            reason: "run export cannot target import",
        });
    }
    let local_index = (function_index - module.imports.len() as u32) as usize;
    let function = module
        .functions
        .get(local_index)
        .ok_or(WasmExecutionError::Trap {
            reason: "function index out of range",
        })?;
    let signature =
        *module
            .types
            .get(function.type_index as usize)
            .ok_or(WasmExecutionError::Trap {
                reason: "type index out of range",
            })?;
    if signature.params != 0 || signature.result != Some(ValueType::I32) {
        return Err(WasmExecutionError::Trap {
            reason: "run export must be () -> i32",
        });
    }

    let mut offset = 0usize;
    let local_group_count = read_uleb(function.body, &mut offset)? as usize;
    let mut locals = Vec::<Value>::new();
    for _ in 0..local_group_count {
        let count = read_uleb(function.body, &mut offset)? as usize;
        let val_type = read_val_type(function.body, &mut offset)?;
        for _ in 0..count {
            locals.push(match val_type {
                ValueType::I32 => Value::I32(0),
                ValueType::I64 => Value::I64(0),
            });
        }
    }
    let stack_base = stack.len();
    let terminator = execute_block(
        &function.body[offset..],
        &module.imports,
        &module.functions,
        &module.types,
        observation,
        stack,
        &mut locals,
        memory,
    )?;
    if terminator != BlockTerminator::End {
        return Err(WasmExecutionError::Trap {
            reason: "unexpected block termination",
        });
    }
    if stack.len() != stack_base + 1 {
        return Err(WasmExecutionError::Trap {
            reason: "run did not leave exactly one result",
        });
    }
    match stack.pop() {
        Some(Value::I32(value)) => Ok(value),
        _ => Err(WasmExecutionError::Trap {
            reason: "run result type mismatch",
        }),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockTerminator {
    Else,
    End,
}

fn execute_block(
    code: &[u8],
    imports: &[ImportFunction],
    functions: &[WasmFunction<'_>],
    types: &[FunctionType],
    observation: &WasmObservation,
    stack: &mut Vec<Value>,
    locals: &mut Vec<Value>,
    memory: &mut Vec<u8>,
) -> Result<BlockTerminator, WasmExecutionError> {
    let mut offset = 0usize;
    loop {
        if offset >= code.len() {
            return Err(WasmExecutionError::Trap {
                reason: "unterminated block",
            });
        }
        match read_u8(code, &mut offset)? {
            OPCODE_UNREACHABLE => {
                return Err(WasmExecutionError::Trap {
                    reason: "unreachable executed",
                });
            }
            OPCODE_NOP => {}
            OPCODE_DROP => {
                stack.pop().ok_or(WasmExecutionError::Trap {
                    reason: "stack underflow on drop",
                })?;
            }
            OPCODE_SELECT => {
                let cond = pop_i32(stack)?;
                let val2 = stack.pop().ok_or(WasmExecutionError::Trap {
                    reason: "stack underflow on select",
                })?;
                let val1 = stack.pop().ok_or(WasmExecutionError::Trap {
                    reason: "stack underflow on select",
                })?;
                if cond != 0 {
                    stack.push(val1);
                } else {
                    stack.push(val2);
                }
            }
            OPCODE_LOCAL_GET => {
                let index = read_uleb(code, &mut offset)? as usize;
                let value = *locals.get(index).ok_or(WasmExecutionError::Trap {
                    reason: "local index out of range",
                })?;
                stack.push(value);
            }
            OPCODE_LOCAL_SET => {
                let index = read_uleb(code, &mut offset)? as usize;
                let value = stack.pop().ok_or(WasmExecutionError::Trap {
                    reason: "stack underflow on local.set",
                })?;
                if index >= locals.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "local index out of range",
                    });
                }
                locals[index] = value;
            }
            OPCODE_LOCAL_TEE => {
                let index = read_uleb(code, &mut offset)? as usize;
                let value = *stack.last().ok_or(WasmExecutionError::Trap {
                    reason: "stack underflow on local.tee",
                })?;
                if index >= locals.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "local index out of range",
                    });
                }
                locals[index] = value;
            }
            OPCODE_I32_LOAD => {
                let _align = read_uleb(code, &mut offset)?;
                let offset_mem = read_uleb(code, &mut offset)? as usize;
                let addr = (pop_i32(stack)? as u32) as usize + offset_mem;
                if addr + 4 > memory.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "out of bounds memory access",
                    });
                }
                let value = i32::from_le_bytes([
                    memory[addr],
                    memory[addr + 1],
                    memory[addr + 2],
                    memory[addr + 3],
                ]);
                stack.push(Value::I32(value));
            }
            OPCODE_I64_LOAD => {
                let _align = read_uleb(code, &mut offset)?;
                let offset_mem = read_uleb(code, &mut offset)? as usize;
                let addr = (pop_i32(stack)? as u32) as usize + offset_mem;
                if addr + 8 > memory.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "out of bounds memory access",
                    });
                }
                let value = i64::from_le_bytes([
                    memory[addr],
                    memory[addr + 1],
                    memory[addr + 2],
                    memory[addr + 3],
                    memory[addr + 4],
                    memory[addr + 5],
                    memory[addr + 6],
                    memory[addr + 7],
                ]);
                stack.push(Value::I64(value));
            }
            OPCODE_I32_STORE => {
                let _align = read_uleb(code, &mut offset)?;
                let offset_mem = read_uleb(code, &mut offset)? as usize;
                let value = pop_i32(stack)?;
                let addr = (pop_i32(stack)? as u32) as usize + offset_mem;
                if addr + 4 > memory.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "out of bounds memory access",
                    });
                }
                memory[addr..addr + 4].copy_from_slice(&value.to_le_bytes());
            }
            OPCODE_I64_STORE => {
                let _align = read_uleb(code, &mut offset)?;
                let offset_mem = read_uleb(code, &mut offset)? as usize;
                let value = pop_i64(stack)?;
                let addr = (pop_i32(stack)? as u32) as usize + offset_mem;
                if addr + 8 > memory.len() {
                    return Err(WasmExecutionError::Trap {
                        reason: "out of bounds memory access",
                    });
                }
                memory[addr..addr + 8].copy_from_slice(&value.to_le_bytes());
            }
            OPCODE_MEMORY_SIZE => {
                stack.push(Value::I32((memory.len() / WASM_PAGE_SIZE) as i32));
            }
            OPCODE_MEMORY_GROW => {
                let delta = pop_i32(stack)? as usize;
                let current_pages = memory.len() / WASM_PAGE_SIZE;
                let new_pages = current_pages + delta;
                if new_pages > WASM_MAX_PAGES {
                    stack.push(Value::I32(-1));
                } else {
                    let new_len = new_pages * WASM_PAGE_SIZE;
                    memory.resize(new_len, 0);
                    stack.push(Value::I32(current_pages as i32));
                }
            }
            OPCODE_CALL => {
                let index = read_uleb(code, &mut offset)?;
                invoke_function(index, imports, functions, types, observation, stack, locals, memory)?;
            }
            OPCODE_I32_CONST => {
                stack.push(Value::I32(read_sleb_i32(code, &mut offset)?));
            }
            OPCODE_I64_CONST => {
                stack.push(Value::I64(read_sleb_i64(code, &mut offset)?));
            }
            OPCODE_I32_EQZ => {
                let value = pop_i32(stack)?;
                stack.push(Value::I32(i32::from(value == 0)));
            }
            OPCODE_I64_EQZ => {
                let value = pop_i64(stack)?;
                stack.push(Value::I32(i32::from(value == 0)));
            }
            OPCODE_I32_EQ => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(i32::from(a == b)));
            }
            OPCODE_I32_NE => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(i32::from(a != b)));
            }
            OPCODE_I32_LT_S => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(i32::from(a < b)));
            }
            OPCODE_I32_ADD => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(a.wrapping_add(b)));
            }
            OPCODE_I32_SUB => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(a.wrapping_sub(b)));
            }
            OPCODE_I32_MUL => {
                let b = pop_i32(stack)?;
                let a = pop_i32(stack)?;
                stack.push(Value::I32(a.wrapping_mul(b)));
            }
            OPCODE_IF => {
                let block_type = read_u8(code, &mut offset)?;
                execute_if(
                    code,
                    &mut offset,
                    block_type,
                    imports,
                    functions,
                    types,
                    observation,
                    stack,
                    locals,
                    memory,
                )?;
            }
            OPCODE_LOOP => {
                let _block_type = read_u8(code, &mut offset)?;
                let loop_start = offset;
                loop {
                    let terminator = execute_block(
                        &code[offset..],
                        imports,
                        functions,
                        types,
                        observation,
                        stack,
                        locals,
                        memory,
                    )?;
                    offset = advance_block_offset(code, offset)?;
                    if terminator == BlockTerminator::End {
                        break;
                    }
                    offset = loop_start;
                }
            }
            OPCODE_BR => {
                let _label = read_uleb(code, &mut offset)?;
                return Ok(BlockTerminator::End);
            }
            OPCODE_BR_IF => {
                let label = read_uleb(code, &mut offset)?;
                let cond = pop_i32(stack)?;
                if cond != 0 {
                    for _ in 0..label {
                        let _ = skip_to_else_or_end(code, offset)?;
                    }
                    return Ok(BlockTerminator::End);
                }
            }
            OPCODE_RETURN => {
                return Ok(BlockTerminator::End);
            }
            OPCODE_ELSE => return Ok(BlockTerminator::Else),
            OPCODE_END => return Ok(BlockTerminator::End),
            _ => {
                return Err(WasmExecutionError::Trap {
                    reason: "unsupported opcode",
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_if(
    code: &[u8],
    offset: &mut usize,
    block_type: u8,
    imports: &[ImportFunction],
    functions: &[WasmFunction<'_>],
    types: &[FunctionType],
    observation: &WasmObservation,
    stack: &mut Vec<Value>,
    locals: &mut Vec<Value>,
    memory: &mut Vec<u8>,
) -> Result<(), WasmExecutionError> {
    if block_type != BLOCKTYPE_EMPTY && block_type != VALTYPE_I32 {
        return Err(WasmExecutionError::Trap {
            reason: "unsupported if block type",
        });
    }
    let wants_result = block_type == VALTYPE_I32;
    let stack_base = stack.len().saturating_sub(1);
    let condition = pop_i32(stack)? != 0;
    if condition {
        let branch = execute_block(
            &code[*offset..],
            imports,
            functions,
            types,
            observation,
            stack,
            locals,
            memory,
        )?;
        *offset = advance_block_offset(code, *offset)?;
        if branch == BlockTerminator::Else {
            *offset = skip_branch(code, *offset)?;
        }
    } else {
        match skip_to_else_or_end(code, *offset)? {
            (BlockTerminator::Else, new_offset) => {
                *offset = new_offset;
                let branch = execute_block(
                    &code[*offset..],
                    imports,
                    functions,
                    types,
                    observation,
                    stack,
                    locals,
                    memory,
                )?;
                if branch != BlockTerminator::End {
                    return Err(WasmExecutionError::Trap {
                        reason: "else branch did not terminate with end",
                    });
                }
                *offset = advance_block_offset(code, *offset)?;
            }
            (BlockTerminator::End, new_offset) => {
                *offset = new_offset;
            }
        }
    }
    let expected = stack_base + usize::from(wants_result);
    if stack.len() != expected {
        return Err(WasmExecutionError::Trap {
            reason: "if stack effect mismatch",
        });
    }
    Ok(())
}

fn invoke_function(
    function_index: u32,
    imports: &[ImportFunction],
    functions: &[WasmFunction<'_>],
    types: &[FunctionType],
    observation: &WasmObservation,
    stack: &mut Vec<Value>,
    locals: &mut Vec<Value>,
    memory: &mut Vec<u8>,
) -> Result<(), WasmExecutionError> {
    if function_index < imports.len() as u32 {
        let import = &imports[function_index as usize];
        let signature = *types
            .get(import.type_index as usize)
            .ok_or(WasmExecutionError::Trap {
                reason: "import type index out of range",
            })?;
        if signature.params != 0 || signature.result != Some(ValueType::I64) {
            return Err(WasmExecutionError::Trap {
                reason: "unsupported import signature",
            });
        }
        let value = match capability_for_import(&import.module, &import.name)? {
            WasmCapability::ObserveProcessCapabilityCount => {
                observation.process_capability_count as i64
            }
            WasmCapability::ObserveSystemProcessCount => observation.process_count as i64,
            WasmCapability::ObserveProcessStatusBytes => observation.process_status_bytes as i64,
            WasmCapability::ObserveProcessCwdRoot => i64::from(observation.process_cwd_root),
        };
        stack.push(Value::I64(value));
        return Ok(());
    }
    let local_index = (function_index - imports.len() as u32) as usize;
    let function = functions.get(local_index).ok_or(WasmExecutionError::Trap {
        reason: "callee out of range",
    })?;
    let signature = *types
        .get(function.type_index as usize)
        .ok_or(WasmExecutionError::Trap {
            reason: "callee type out of range",
        })?;
    if signature.params != 0 {
        return Err(WasmExecutionError::Trap {
            reason: "functions with parameters are unsupported",
        });
    }
    let mut body_offset = 0usize;
    let local_group_count = read_uleb(function.body, &mut body_offset)? as usize;
    let mut callee_locals = Vec::<Value>::new();
    for _ in 0..local_group_count {
        let count = read_uleb(function.body, &mut body_offset)? as usize;
        let val_type = read_val_type(function.body, &mut body_offset)?;
        for _ in 0..count {
            callee_locals.push(match val_type {
                ValueType::I32 => Value::I32(0),
                ValueType::I64 => Value::I64(0),
            });
        }
    }
    let stack_base = stack.len();
    let terminator = execute_block(
        &function.body[body_offset..],
        imports,
        functions,
        types,
        observation,
        stack,
        &mut callee_locals,
        memory,
    )?;
    if terminator != BlockTerminator::End {
        return Err(WasmExecutionError::Trap {
            reason: "callee block did not end cleanly",
        });
    }
    match signature.result {
        None => {
            if stack.len() != stack_base {
                return Err(WasmExecutionError::Trap {
                    reason: "void callee changed stack depth",
                });
            }
        }
        Some(ValueType::I32) | Some(ValueType::I64) => {
            if stack.len() != stack_base + 1 {
                return Err(WasmExecutionError::Trap {
                    reason: "callee result depth mismatch",
                });
            }
        }
    }
    Ok(())
}

fn skip_to_else_or_end(
    code: &[u8],
    mut offset: usize,
) -> Result<(BlockTerminator, usize), WasmExecutionError> {
    let mut depth = 0usize;
    while offset < code.len() {
        match read_u8(code, &mut offset)? {
            OPCODE_IF => {
                let _ = read_u8(code, &mut offset)?;
                depth += 1;
            }
            OPCODE_ELSE if depth == 0 => return Ok((BlockTerminator::Else, offset)),
            OPCODE_ELSE => {}
            OPCODE_END if depth == 0 => return Ok((BlockTerminator::End, offset)),
            OPCODE_END => depth -= 1,
            OPCODE_CALL => {
                let _ = read_uleb(code, &mut offset)?;
            }
            OPCODE_I32_CONST => {
                let _ = read_sleb_i32(code, &mut offset)?;
            }
            OPCODE_I64_EQZ => {}
            _ => {
                return Err(WasmExecutionError::Trap {
                    reason: "unsupported opcode while skipping branch",
                });
            }
        }
    }
    Err(WasmExecutionError::Trap {
        reason: "unterminated skipped branch",
    })
}

fn advance_block_offset(code: &[u8], start: usize) -> Result<usize, WasmExecutionError> {
    match skip_to_else_or_end(code, start)? {
        (BlockTerminator::Else, offset) | (BlockTerminator::End, offset) => Ok(offset),
    }
}

fn skip_branch(code: &[u8], start: usize) -> Result<usize, WasmExecutionError> {
    match skip_to_else_or_end(code, start)? {
        (BlockTerminator::End, offset) => Ok(offset),
        (BlockTerminator::Else, _) => Err(WasmExecutionError::Trap {
            reason: "unexpected nested else while skipping branch",
        }),
    }
}

fn pop_i32(stack: &mut Vec<Value>) -> Result<i32, WasmExecutionError> {
    match stack.pop() {
        Some(Value::I32(value)) => Ok(value),
        _ => Err(WasmExecutionError::Trap {
            reason: "expected i32 on stack",
        }),
    }
}

fn pop_i64(stack: &mut Vec<Value>) -> Result<i64, WasmExecutionError> {
    match stack.pop() {
        Some(Value::I64(value)) => Ok(value),
        _ => Err(WasmExecutionError::Trap {
            reason: "expected i64 on stack",
        }),
    }
}

fn read_val_type(bytes: &[u8], offset: &mut usize) -> Result<ValueType, WasmExecutionError> {
    match read_u8(bytes, offset)? {
        VALTYPE_I32 => Ok(ValueType::I32),
        VALTYPE_I64 => Ok(ValueType::I64),
        _ => Err(WasmExecutionError::InvalidModule {
            reason: "unsupported value type",
        }),
    }
}

fn read_name(bytes: &[u8], offset: &mut usize) -> Result<String, WasmExecutionError> {
    let len = read_uleb(bytes, offset)? as usize;
    let end = offset
        .checked_add(len)
        .ok_or(WasmExecutionError::InvalidModule {
            reason: "name overflow",
        })?;
    if end > bytes.len() {
        return Err(WasmExecutionError::InvalidModule {
            reason: "truncated name",
        });
    }
    let text =
        str::from_utf8(&bytes[*offset..end]).map_err(|_| WasmExecutionError::InvalidModule {
            reason: "invalid utf-8 name",
        })?;
    *offset = end;
    Ok(text.into())
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> Result<u8, WasmExecutionError> {
    if *offset >= bytes.len() {
        return Err(WasmExecutionError::InvalidModule {
            reason: "unexpected end of input",
        });
    }
    let value = bytes[*offset];
    *offset += 1;
    Ok(value)
}

fn read_uleb(bytes: &[u8], offset: &mut usize) -> Result<u32, WasmExecutionError> {
    let mut shift = 0u32;
    let mut value = 0u32;
    loop {
        let byte = read_u8(bytes, offset)?;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift > 28 {
            return Err(WasmExecutionError::InvalidModule {
                reason: "uleb too large",
            });
        }
    }
}

fn read_sleb_i32(bytes: &[u8], offset: &mut usize) -> Result<i32, WasmExecutionError> {
    let mut shift = 0u32;
    let mut value = 0i32;
    let mut byte;
    loop {
        byte = read_u8(bytes, offset)?;
        value |= i32::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift > 35 {
            return Err(WasmExecutionError::InvalidModule {
                reason: "sleb too large",
            });
        }
    }
    if shift < 32 && (byte & 0x40) != 0 {
        value |= !0 << shift;
    }
    Ok(value)
}

fn read_sleb_i64(bytes: &[u8], offset: &mut usize) -> Result<i64, WasmExecutionError> {
    let mut shift = 0u32;
    let mut value = 0i64;
    let mut byte;
    loop {
        byte = read_u8(bytes, offset)?;
        value |= i64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift > 63 {
            return Err(WasmExecutionError::InvalidModule {
                reason: "sleb too large",
            });
        }
    }
    if shift < 64 && (byte & 0x40) != 0 {
        value |= !0 << shift;
    }
    Ok(value)
}

fn ensure_consumed(section: &[u8], offset: usize) -> Result<(), WasmExecutionError> {
    if offset == section.len() {
        Ok(())
    } else {
        Err(WasmExecutionError::InvalidModule {
            reason: "section payload not fully consumed",
        })
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;
    use crate::Runtime;
    use ngos_user_abi::{
        NativeProcessRecord, SYS_GET_PROCESS_CWD, SYS_INSPECT_PROCESS, SYS_LIST_PROCESSES,
        SYS_READ_PROCFS, SyscallBackend, SyscallFrame, SyscallReturn,
    };

    struct WasmBackend {
        frames: RefCell<Vec<SyscallFrame>>,
    }

    impl WasmBackend {
        fn new() -> Self {
            Self {
                frames: RefCell::new(Vec::new()),
            }
        }
    }

    impl SyscallBackend for WasmBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            self.frames.borrow_mut().push(frame);
            match frame.number {
                SYS_INSPECT_PROCESS => {
                    let ptr = frame.arg1 as *mut NativeProcessRecord;
                    unsafe {
                        ptr.write(NativeProcessRecord {
                            pid: frame.arg0 as u64,
                            parent: 0,
                            address_space: 4,
                            main_thread: 1,
                            state: 1,
                            exit_code: 0,
                            descriptor_count: 3,
                            capability_count: 2,
                            environment_count: 1,
                            memory_region_count: 4,
                            thread_count: 1,
                            pending_signal_count: 0,
                            session_reported: 0,
                            session_status: 0,
                            session_stage: 0,
                            scheduler_class: 1,
                            scheduler_budget: 2,
                            cpu_runtime_ticks: 11,
                            execution_contract: 0,
                            memory_contract: 0,
                            io_contract: 0,
                            observe_contract: 0,
                            reserved: 0,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_LIST_PROCESSES => {
                    let ptr = frame.arg0 as *mut u64;
                    unsafe {
                        *ptr = 1;
                        *ptr.add(1) = 2;
                    }
                    SyscallReturn::ok(2)
                }
                SYS_READ_PROCFS => {
                    let payload = b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\n";
                    let ptr = frame.arg2 as *mut u8;
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, payload.len());
                    }
                    SyscallReturn::ok(payload.len())
                }
                SYS_GET_PROCESS_CWD => {
                    let ptr = frame.arg1 as *mut u8;
                    unsafe {
                        *ptr = b'/';
                    }
                    SyscallReturn::ok(1)
                }
                _ => SyscallReturn::err(Errno::Inval),
            }
        }
    }

    #[test]
    fn wasm_component_executes_with_explicit_capability_bindings() {
        let runtime = Runtime::new(WasmBackend::new());
        let report = execute_wasm_component(
            &runtime,
            WASM_BOOT_PROOF_COMPONENT,
            1,
            &[
                WasmCapability::ObserveProcessCapabilityCount,
                WasmCapability::ObserveSystemProcessCount,
            ],
        )
        .unwrap();

        assert_eq!(report.verdict, WasmVerdict::Ready);
        assert_eq!(report.observation.pid, 1);
        assert_eq!(report.observation.process_capability_count, 2);
        assert_eq!(report.observation.process_count, 2);
        assert!(report.observation.process_status_bytes > 0);
        assert!(report.observation.process_cwd_root);
    }

    #[test]
    fn wasm_component_refuses_missing_capability_binding() {
        let runtime = Runtime::new(WasmBackend::new());
        let error = execute_wasm_component(
            &runtime,
            WASM_BOOT_PROOF_COMPONENT,
            1,
            &[WasmCapability::ObserveProcessCapabilityCount],
        )
        .unwrap_err();

        assert_eq!(
            error,
            WasmExecutionError::MissingCapability {
                capability: WasmCapability::ObserveSystemProcessCount,
                import: IMPORT_OBSERVE_SYSTEM_PROCESS_COUNT.into(),
            }
        );
    }

    #[test]
    fn wasm_second_component_executes_with_process_identity_bindings() {
        let runtime = Runtime::new(WasmBackend::new());
        let report = execute_wasm_component(
            &runtime,
            WASM_PROCESS_IDENTITY_COMPONENT,
            1,
            &[
                WasmCapability::ObserveProcessStatusBytes,
                WasmCapability::ObserveProcessCwdRoot,
            ],
        )
        .unwrap();

        assert_eq!(report.verdict, WasmVerdict::Ready);
        assert!(report.observation.process_status_bytes > 0);
        assert!(report.observation.process_cwd_root);
    }
}
