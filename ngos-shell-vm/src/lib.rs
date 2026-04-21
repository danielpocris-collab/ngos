//! Canonical subsystem role:
//! - subsystem: native VM control surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing VM actions over canonical memory and
//!   process contracts
//!
//! Canonical contract families handled here:
//! - VM command contracts
//! - process memory management command contracts
//! - VM inspection and probe contracts
//!
//! This module may issue VM-related operations, but it must not redefine VM
//! truth, address-space ownership, or kernel memory semantics.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::parse_u64_arg;
use ngos_user_abi::{Errno, ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

enum VmAgentCommand {
    MapAnonymous {
        pid: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: String,
    },
    ProbeMapAnonymous {
        pid: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: String,
    },
    SetBreak {
        pid: u64,
        new_end: u64,
    },
    ProbeSetBreak {
        pid: u64,
        new_end: u64,
    },
    Quarantine {
        pid: u64,
        vm_object_id: u64,
        reason: u64,
    },
    Release {
        pid: u64,
        vm_object_id: u64,
    },
    LoadWord {
        pid: u64,
        addr: u64,
    },
    StoreWord {
        pid: u64,
        addr: u64,
        value: u32,
    },
    ProbeStoreWord {
        pid: u64,
        addr: u64,
        value: u32,
    },
    SyncRange {
        pid: u64,
        addr: u64,
        length: u64,
    },
    ProtectRange {
        pid: u64,
        addr: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: &'static str,
    },
    UnmapRange {
        pid: u64,
        addr: u64,
        length: u64,
    },
    AdviseRange {
        pid: u64,
        addr: u64,
        length: u64,
        advice: u64,
        label: &'static str,
    },
    Pressure {
        pid: u64,
        target_pages: u64,
    },
    PressureGlobal {
        target_pages: u64,
    },
}

impl VmAgentCommand {
    fn parse_perms(token: &str) -> Option<(bool, bool, bool, &'static str)> {
        match token {
            "r--" => Some((true, false, false, "r--")),
            "rw-" => Some((true, true, false, "rw-")),
            "r-x" => Some((true, false, true, "r-x")),
            "rwx" => Some((true, true, true, "rwx")),
            "---" => Some((false, false, false, "---")),
            _ => None,
        }
    }

    fn parse(line: &str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("vm-map-anon ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parts.next().and_then(Self::parse_perms),
                ) {
                    (Some(pid), Some(length), Some((readable, writable, executable, _))) => {
                        Ok(Self::MapAnonymous {
                            pid,
                            length,
                            readable,
                            writable,
                            executable,
                            label: parts.collect::<Vec<_>>().join(" "),
                        })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-probe-map-anon ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parts.next().and_then(Self::parse_perms),
                ) {
                    (Some(pid), Some(length), Some((readable, writable, executable, _))) => {
                        Ok(Self::ProbeMapAnonymous {
                            pid,
                            length,
                            readable,
                            writable,
                            executable,
                            label: parts.collect::<Vec<_>>().join(" "),
                        })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-brk ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (parse_u64_arg(parts.next()), parse_u64_arg(parts.next())) {
                    (Some(pid), Some(new_end)) => Ok(Self::SetBreak { pid, new_end }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-probe-brk ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (parse_u64_arg(parts.next()), parse_u64_arg(parts.next())) {
                    (Some(pid), Some(new_end)) => Ok(Self::ProbeSetBreak { pid, new_end }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-quarantine ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                ) {
                    (Some(pid), Some(vm_object_id), reason) => Ok(Self::Quarantine {
                        pid,
                        vm_object_id,
                        reason: reason.unwrap_or(1),
                    }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-release ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (parse_u64_arg(parts.next()), parse_u64_arg(parts.next())) {
                    (Some(pid), Some(vm_object_id)) => Ok(Self::Release { pid, vm_object_id }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-load-word ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (parse_u64_arg(parts.next()), parse_u64_arg(parts.next())) {
                    (Some(pid), Some(addr)) => Ok(Self::LoadWord { pid, addr }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-store-word ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parts.next().and_then(|value| value.parse::<u32>().ok()),
                ) {
                    (Some(pid), Some(addr), Some(value)) => {
                        Ok(Self::StoreWord { pid, addr, value })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-probe-store-word ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parts.next().and_then(|value| value.parse::<u32>().ok()),
                ) {
                    (Some(pid), Some(addr), Some(value)) => {
                        Ok(Self::ProbeStoreWord { pid, addr, value })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-sync-range ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                ) {
                    (Some(pid), Some(addr), Some(length)) => {
                        Ok(Self::SyncRange { pid, addr, length })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-protect ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parts.next().and_then(Self::parse_perms),
                ) {
                    (
                        Some(pid),
                        Some(addr),
                        Some(length),
                        Some((readable, writable, executable, label)),
                    ) => Ok(Self::ProtectRange {
                        pid,
                        addr,
                        length,
                        readable,
                        writable,
                        executable,
                        label,
                    }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-unmap ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                    parse_u64_arg(parts.next()),
                ) {
                    (Some(pid), Some(addr), Some(length)) => {
                        Ok(Self::UnmapRange { pid, addr, length })
                    }
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-advise ") {
            let mut parts = rest.split_whitespace();
            let parsed = match (
                parse_u64_arg(parts.next()),
                parse_u64_arg(parts.next()),
                parse_u64_arg(parts.next()),
                parts.next(),
            ) {
                (Some(pid), Some(addr), Some(length), Some("normal")) => Ok(Self::AdviseRange {
                    pid,
                    addr,
                    length,
                    advice: 0,
                    label: "normal",
                }),
                (Some(pid), Some(addr), Some(length), Some("sequential")) => {
                    Ok(Self::AdviseRange {
                        pid,
                        addr,
                        length,
                        advice: 1,
                        label: "sequential",
                    })
                }
                (Some(pid), Some(addr), Some(length), Some("random")) => Ok(Self::AdviseRange {
                    pid,
                    addr,
                    length,
                    advice: 2,
                    label: "random",
                }),
                (Some(pid), Some(addr), Some(length), Some("willneed")) => Ok(Self::AdviseRange {
                    pid,
                    addr,
                    length,
                    advice: 3,
                    label: "willneed",
                }),
                (Some(pid), Some(addr), Some(length), Some("dontneed")) => Ok(Self::AdviseRange {
                    pid,
                    addr,
                    length,
                    advice: 4,
                    label: "dontneed",
                }),
                _ => Err(2),
            };
            return Some(parsed);
        }
        if let Some(rest) = line.strip_prefix("vm-pressure ") {
            let mut parts = rest.split_whitespace();
            return Some(
                match (parse_u64_arg(parts.next()), parse_u64_arg(parts.next())) {
                    (Some(pid), Some(target_pages)) => Ok(Self::Pressure { pid, target_pages }),
                    _ => Err(2),
                },
            );
        }
        if let Some(rest) = line.strip_prefix("vm-pressure-global ") {
            let mut parts = rest.split_whitespace();
            return Some(match parse_u64_arg(parts.next()) {
                Some(target_pages) => Ok(Self::PressureGlobal { target_pages }),
                None => Err(2),
            });
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>) -> Result<(), ExitCode> {
        match *self {
            Self::MapAnonymous {
                pid,
                length,
                readable,
                writable,
                executable,
                ref label,
            } => {
                let start = runtime
                    .map_anonymous_memory(pid, length, readable, writable, executable, label)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-map-anon pid={} start={} len={} perms={}{}{} label={}",
                        pid,
                        start,
                        length,
                        if readable { "r" } else { "-" },
                        if writable { "w" } else { "-" },
                        if executable { "x" } else { "-" },
                        label
                    ),
                )
            }
            Self::ProbeMapAnonymous {
                pid,
                length,
                readable,
                writable,
                executable,
                ref label,
            } => {
                let outcome = match runtime
                    .map_anonymous_memory(pid, length, readable, writable, executable, label)
                {
                    Ok(start) => format!(
                        "vm-probe-map-anon pid={} start={} len={} outcome=mapped label={}",
                        pid, start, length, label
                    ),
                    Err(errno) => format!(
                        "vm-probe-map-anon pid={} len={} outcome=error errno={:?} label={}",
                        pid, length, errno, label
                    ),
                };
                write_line(runtime, &outcome)
            }
            Self::SetBreak { pid, new_end } => {
                let end = runtime.set_process_break(pid, new_end).map_err(|_| 235)?;
                write_line(runtime, &format!("vm-brk pid={} end={}", pid, end))
            }
            Self::ProbeSetBreak { pid, new_end } => {
                let outcome = match runtime.set_process_break(pid, new_end) {
                    Ok(end) => format!("vm-probe-brk pid={} end={} outcome=updated", pid, end),
                    Err(errno) => format!(
                        "vm-probe-brk pid={} requested={} outcome=error errno={:?}",
                        pid, new_end, errno
                    ),
                };
                write_line(runtime, &outcome)
            }
            Self::Quarantine {
                pid,
                vm_object_id,
                reason,
            } => {
                runtime
                    .quarantine_vm_object(pid, vm_object_id, reason)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-quarantine pid={} vm-object={} reason={}",
                        pid, vm_object_id, reason
                    ),
                )
            }
            Self::Release { pid, vm_object_id } => {
                runtime
                    .release_vm_object(pid, vm_object_id)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!("vm-release pid={} vm-object={}", pid, vm_object_id),
                )
            }
            Self::LoadWord { pid, addr } => {
                let value = runtime.load_memory_word(pid, addr).map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!("vm-load-word pid={} addr={} value={}", pid, addr, value),
                )
            }
            Self::StoreWord { pid, addr, value } => {
                runtime
                    .store_memory_word(pid, addr, value)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!("vm-store-word pid={} addr={} value={}", pid, addr, value),
                )
            }
            Self::ProbeStoreWord { pid, addr, value } => {
                let outcome = match runtime.store_memory_word(pid, addr, value) {
                    Ok(()) => format!(
                        "vm-probe-store-word pid={} addr={} value={} outcome=stored",
                        pid, addr, value
                    ),
                    Err(Errno::Busy) => format!(
                        "vm-probe-store-word pid={} addr={} value={} outcome=blocked errno=busy",
                        pid, addr, value
                    ),
                    Err(errno) => format!(
                        "vm-probe-store-word pid={} addr={} value={} outcome=error errno={:?}",
                        pid, addr, value, errno
                    ),
                };
                write_line(runtime, &outcome)
            }
            Self::SyncRange { pid, addr, length } => {
                runtime
                    .sync_memory_range(pid, addr, length)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!("vm-sync-range pid={} addr={} len={}", pid, addr, length),
                )
            }
            Self::ProtectRange {
                pid,
                addr,
                length,
                readable,
                writable,
                executable,
                label,
            } => {
                runtime
                    .protect_memory_range(pid, addr, length, readable, writable, executable)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-protect pid={} addr={} len={} perms={}",
                        pid, addr, length, label
                    ),
                )
            }
            Self::UnmapRange { pid, addr, length } => {
                runtime
                    .unmap_memory_range(pid, addr, length)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!("vm-unmap pid={} addr={} len={}", pid, addr, length),
                )
            }
            Self::AdviseRange {
                pid,
                addr,
                length,
                advice,
                label,
            } => {
                runtime
                    .advise_memory_range(pid, addr, length, advice)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-advise pid={} addr={} len={} advice={}",
                        pid, addr, length, label
                    ),
                )
            }
            Self::Pressure { pid, target_pages } => {
                let reclaimed = runtime
                    .reclaim_memory_pressure(pid, target_pages)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-pressure pid={} target-pages={} reclaimed-pages={}",
                        pid, target_pages, reclaimed
                    ),
                )
            }
            Self::PressureGlobal { target_pages } => {
                let reclaimed = runtime
                    .reclaim_memory_pressure_global(target_pages)
                    .map_err(|_| 235)?;
                write_line(
                    runtime,
                    &format!(
                        "vm-pressure-global target-pages={} reclaimed-pages={}",
                        target_pages, reclaimed
                    ),
                )
            }
        }
    }
}

pub fn try_handle_vm_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match VmAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("vm-map-anon ") {
                "usage: vm-map-anon <pid> <len> <r--|rw-|r-x|rwx|---> <label...>"
            } else if line.starts_with("vm-probe-map-anon ") {
                "usage: vm-probe-map-anon <pid> <len> <r--|rw-|r-x|rwx|---> <label...>"
            } else if line.starts_with("vm-brk ") {
                "usage: vm-brk <pid> <new-end>"
            } else if line.starts_with("vm-probe-brk ") {
                "usage: vm-probe-brk <pid> <new-end>"
            } else if line.starts_with("vm-quarantine ") {
                "usage: vm-quarantine <pid> <vm-object-id> [reason]"
            } else if line.starts_with("vm-release ") {
                "usage: vm-release <pid> <vm-object-id>"
            } else if line.starts_with("vm-load-word ") {
                "usage: vm-load-word <pid> <addr>"
            } else if line.starts_with("vm-store-word ") {
                "usage: vm-store-word <pid> <addr> <value>"
            } else if line.starts_with("vm-sync-range ") {
                "usage: vm-sync-range <pid> <addr> <len>"
            } else if line.starts_with("vm-protect ") {
                "usage: vm-protect <pid> <addr> <len> <r--|rw-|r-x|rwx|--->"
            } else if line.starts_with("vm-unmap ") {
                "usage: vm-unmap <pid> <addr> <len>"
            } else if line.starts_with("vm-advise ") {
                "usage: vm-advise <pid> <addr> <len> <normal|sequential|random|willneed|dontneed>"
            } else if line.starts_with("vm-pressure ") {
                "usage: vm-pressure <pid> <target-pages>"
            } else if line.starts_with("vm-pressure-global ") {
                "usage: vm-pressure-global <target-pages>"
            } else {
                "usage: vm-probe-store-word <pid> <addr> <value>"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime))
}
