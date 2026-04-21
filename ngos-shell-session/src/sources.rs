//! Pipeline value sources: procfs, fd, jobs, storage, network, signals, caps, waiters.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ngos_shell_proc::{
    fixed_text_field, native_process_state_label, read_process_text, read_procfs_all,
};
use ngos_shell_types::{
    ShellJob, ShellRecordField, ShellSemanticValue, ShellVariable, shell_render_list_value,
    shell_render_record_value,
};
use ngos_user_abi::{
    ExitCode, NATIVE_STORAGE_LINEAGE_DEPTH, NativeNetworkInterfaceRecord,
    NativeNetworkSocketRecord, NativeProcessIdentityRecord, NativeProcessRecord, SyscallBackend,
};
use ngos_user_runtime::Runtime;

pub(crate) fn render_ipv4(addr: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
}

pub(crate) fn shell_collect_waiters<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<String, ExitCode> {
    let mut ids = vec![0u64; 8];
    loop {
        let count = runtime
            .list_resource_waiters(resource, &mut ids)
            .map_err(|_| 229)?;
        if count <= ids.len() {
            ids.truncate(count);
            let rendered = if ids.is_empty() {
                String::from("-")
            } else {
                ids.into_iter()
                    .map(|id| format!("{id}"))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            return Ok(rendered);
        }
        ids.resize(count, 0);
    }
}

pub(crate) fn shell_process_identity_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<ShellVariable, ExitCode> {
    let identity: NativeProcessIdentityRecord =
        runtime.get_process_identity(pid).map_err(|_| 251)?;
    let root = read_process_text(runtime, pid, Runtime::get_process_root).map_err(|_| 251)?;
    let groups = if identity.supplemental_count == 0 {
        String::from("-")
    } else {
        identity.supplemental_gids[..identity.supplemental_count as usize]
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut fields = Vec::with_capacity(5);
    fields.push(ShellRecordField {
        key: String::from("uid"),
        value: identity.uid.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("gid"),
        value: identity.gid.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("umask"),
        value: format!("{:03o}", identity.umask & 0o777),
    });
    fields.push(ShellRecordField {
        key: String::from("groups"),
        value: groups,
    });
    fields.push(ShellRecordField {
        key: String::from("root"),
        value: root,
    });
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_job_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    jobs: &[ShellJob],
) -> ShellVariable {
    let items = jobs
        .iter()
        .map(|job| {
            if let Some(exit) = job.reaped_exit {
                format!("pid={}:reaped:{exit}", job.pid)
            } else {
                let state = runtime
                    .inspect_process(job.pid)
                    .ok()
                    .map(|record| native_process_state_label(record.state).to_string())
                    .unwrap_or_else(|| String::from("unknown"));
                format!("pid={}:live:{state}", job.pid)
            }
        })
        .collect::<Vec<_>>();
    ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    }
}

pub(crate) fn shell_waiter_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<ShellVariable, ExitCode> {
    let rendered = shell_collect_waiters(runtime, resource)?;
    let items = if rendered == "-" {
        Vec::new()
    } else {
        rendered
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_queue_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ShellVariable, ExitCode> {
    let bytes = read_procfs_all(runtime, "/proc/1/queues")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let items = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_fd_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ShellVariable, ExitCode> {
    let bytes = read_procfs_all(runtime, "/proc/1/fd")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let items = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_fdinfo_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
) -> Result<ShellVariable, ExitCode> {
    let path = format!("/proc/1/fdinfo/{fd}");
    let bytes = read_procfs_all(runtime, &path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let mut fields = Vec::<ShellRecordField>::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(":\t") else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        fields.push(ShellRecordField {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    if fields.is_empty() {
        return Err(203);
    }
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_maps_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<ShellVariable, ExitCode> {
    let path = format!("/proc/{pid}/maps");
    let bytes = read_procfs_all(runtime, &path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let items = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_procfs_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<ShellVariable, ExitCode> {
    let bytes = read_procfs_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let items = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_mount_inventory_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ShellVariable, ExitCode> {
    shell_procfs_list_value(runtime, "/proc/1/mounts")
}

pub(crate) fn shell_procfs_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<ShellVariable, ExitCode> {
    let bytes = read_procfs_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let mut fields = Vec::<ShellRecordField>::with_capacity(text.lines().count());
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let split = line
            .split_once(":\t")
            .or_else(|| line.split_once('='))
            .or_else(|| line.split_once('\t'));
        let Some((key, value)) = split else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        fields.push(ShellRecordField {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    if fields.is_empty() {
        return Err(203);
    }
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_procfs_token_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<ShellVariable, ExitCode> {
    let bytes = read_procfs_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    let mut fields = Vec::<ShellRecordField>::with_capacity(text.split_whitespace().count());
    for token in text.split_whitespace() {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        let key = key.trim_end_matches(':').trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        fields.push(ShellRecordField {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    if fields.is_empty() {
        return Err(203);
    }
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

fn shell_capability_items_for_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<Vec<String>, ExitCode> {
    let path = format!("/proc/{pid}/caps");
    if let Ok(bytes) = read_procfs_all(runtime, &path)
        && let Ok(text) = core::str::from_utf8(&bytes)
    {
        let items = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !items.is_empty() {
            return Ok(items);
        }
    }
    let record: NativeProcessRecord = runtime.inspect_process(pid).map_err(|_| 251)?;
    Ok((0..record.capability_count)
        .map(|index| format!("capability:{index}"))
        .collect::<Vec<_>>())
}

pub(crate) fn shell_capability_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<ShellVariable, ExitCode> {
    let items = shell_capability_items_for_pid(runtime, pid)?;
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_pending_signal_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    blocked_only: bool,
) -> Result<ShellVariable, ExitCode> {
    let mut buffer = [0u8; 64];
    let count = if blocked_only {
        runtime
            .blocked_pending_signals(pid, &mut buffer)
            .map_err(|_| 249)?
    } else {
        runtime.pending_signals(pid, &mut buffer).map_err(|_| 250)?
    };
    let items = if count == 0 {
        Vec::new()
    } else {
        buffer[..count]
            .iter()
            .map(|signal| signal.to_string())
            .collect::<Vec<_>>()
    };
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_network_socket_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<ShellVariable, ExitCode> {
    let record: NativeNetworkSocketRecord = runtime
        .inspect_network_socket(socket_path)
        .map_err(|_| 246)?;
    let fields = vec![
        ShellRecordField {
            key: String::from("path"),
            value: socket_path.to_string(),
        },
        ShellRecordField {
            key: String::from("local_addr"),
            value: render_ipv4(record.local_ipv4),
        },
        ShellRecordField {
            key: String::from("local_port"),
            value: record.local_port.to_string(),
        },
        ShellRecordField {
            key: String::from("remote_addr"),
            value: render_ipv4(record.remote_ipv4),
        },
        ShellRecordField {
            key: String::from("remote_port"),
            value: record.remote_port.to_string(),
        },
        ShellRecordField {
            key: String::from("connected"),
            value: if record.connected != 0 {
                String::from("yes")
            } else {
                String::from("no")
            },
        },
        ShellRecordField {
            key: String::from("rx_depth"),
            value: record.rx_depth.to_string(),
        },
        ShellRecordField {
            key: String::from("rx_limit"),
            value: record.rx_queue_limit.to_string(),
        },
        ShellRecordField {
            key: String::from("rx_packets"),
            value: record.rx_packets.to_string(),
        },
        ShellRecordField {
            key: String::from("tx_packets"),
            value: record.tx_packets.to_string(),
        },
        ShellRecordField {
            key: String::from("dropped"),
            value: record.dropped_packets.to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_network_interface_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<ShellVariable, ExitCode> {
    let record: NativeNetworkInterfaceRecord = runtime
        .inspect_network_interface(device_path)
        .map_err(|_| 246)?;
    let fields = vec![
        ShellRecordField {
            key: String::from("path"),
            value: device_path.to_string(),
        },
        ShellRecordField {
            key: String::from("admin"),
            value: if record.admin_up != 0 {
                String::from("up")
            } else {
                String::from("down")
            },
        },
        ShellRecordField {
            key: String::from("link"),
            value: if record.link_up != 0 {
                String::from("up")
            } else {
                String::from("down")
            },
        },
        ShellRecordField {
            key: String::from("promisc"),
            value: if record.promiscuous != 0 {
                String::from("on")
            } else {
                String::from("off")
            },
        },
        ShellRecordField {
            key: String::from("mtu"),
            value: record.mtu.to_string(),
        },
        ShellRecordField {
            key: String::from("tx_capacity"),
            value: record.tx_capacity.to_string(),
        },
        ShellRecordField {
            key: String::from("rx_capacity"),
            value: record.rx_capacity.to_string(),
        },
        ShellRecordField {
            key: String::from("inflight_limit"),
            value: record.tx_inflight_limit.to_string(),
        },
        ShellRecordField {
            key: String::from("addr"),
            value: render_ipv4(record.ipv4_addr),
        },
        ShellRecordField {
            key: String::from("netmask"),
            value: render_ipv4(record.ipv4_netmask),
        },
        ShellRecordField {
            key: String::from("gateway"),
            value: render_ipv4(record.ipv4_gateway),
        },
        ShellRecordField {
            key: String::from("rx_packets"),
            value: record.rx_packets.to_string(),
        },
        ShellRecordField {
            key: String::from("tx_packets"),
            value: record.tx_packets.to_string(),
        },
        ShellRecordField {
            key: String::from("sockets"),
            value: record.attached_socket_count.to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_storage_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime
        .inspect_storage_volume(device_path)
        .map_err(|_| 246)?;
    let lineage = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let mut fields = Vec::with_capacity(26);
    fields.push(ShellRecordField {
        key: String::from("path"),
        value: device_path.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("valid"),
        value: record.valid.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("dirty"),
        value: record.dirty.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("generation"),
        value: record.generation.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("parent_generation"),
        value: record.parent_generation.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("replay_generation"),
        value: record.replay_generation.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("prepared_count"),
        value: record.prepared_commit_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("recovered_count"),
        value: record.recovered_commit_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("repaired_count"),
        value: record.repaired_snapshot_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("lineage_depth"),
        value: NATIVE_STORAGE_LINEAGE_DEPTH.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("history_count"),
        value: lineage.count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("history_newest"),
        value: lineage.newest_generation.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("history_oldest"),
        value: lineage.oldest_generation.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("history_contiguous"),
        value: if lineage.lineage_contiguous != 0 {
            String::from("yes")
        } else {
            String::from("no")
        },
    });
    fields.push(ShellRecordField {
        key: String::from("payload_len"),
        value: record.payload_len.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("checksum"),
        value: record.payload_checksum.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("alloc_total"),
        value: record.allocation_total_blocks.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("alloc_used"),
        value: record.allocation_used_blocks.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("files"),
        value: record.mapped_file_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("dirs"),
        value: record.mapped_directory_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("symlinks"),
        value: record.mapped_symlink_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("extents"),
        value: record.mapped_extent_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("volume"),
        value: fixed_text_field(&record.volume_id).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("state"),
        value: fixed_text_field(&record.state_label).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("tag"),
        value: fixed_text_field(&record.last_commit_tag).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("preview"),
        value: fixed_text_field(&record.payload_preview).to_string(),
    });
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_storage_history_entry_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    index: usize,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let Some(entry) = record.entries.iter().take(record.count as usize).nth(index) else {
        return Err(205);
    };
    let fields = vec![
        ShellRecordField {
            key: String::from("index"),
            value: index.to_string(),
        },
        ShellRecordField {
            key: String::from("generation"),
            value: entry.generation.to_string(),
        },
        ShellRecordField {
            key: String::from("parent_generation"),
            value: entry.parent_generation.to_string(),
        },
        ShellRecordField {
            key: String::from("payload_checksum"),
            value: entry.payload_checksum.to_string(),
        },
        ShellRecordField {
            key: String::from("kind"),
            value: fixed_text_field(&entry.kind_label).to_string(),
        },
        ShellRecordField {
            key: String::from("state"),
            value: fixed_text_field(&entry.state_label).to_string(),
        },
        ShellRecordField {
            key: String::from("tag"),
            value: fixed_text_field(&entry.tag_label).to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub(crate) fn shell_storage_history_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let mut items = Vec::new();
    for (index, entry) in record
        .entries
        .iter()
        .take(record.count as usize)
        .enumerate()
    {
        items.push(format!(
            "index={} generation={} parent={} checksum={} kind={} state={} tag={}",
            index,
            entry.generation,
            entry.parent_generation,
            entry.payload_checksum,
            fixed_text_field(&entry.kind_label),
            fixed_text_field(&entry.state_label),
            fixed_text_field(&entry.tag_label)
        ));
    }
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_storage_history_range_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    start: usize,
    count: usize,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let mut items = Vec::new();
    if record.valid != 0 && count != 0 && start < record.count as usize {
        for (offset, entry) in record
            .entries
            .iter()
            .take(record.count as usize)
            .skip(start)
            .take(count)
            .enumerate()
        {
            items.push(format!(
                "index={} generation={} parent={} checksum={} kind={} state={} tag={}",
                start + offset,
                entry.generation,
                entry.parent_generation,
                entry.payload_checksum,
                fixed_text_field(&entry.kind_label),
                fixed_text_field(&entry.state_label),
                fixed_text_field(&entry.tag_label)
            ));
        }
    }
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_list_value(&items),
        semantic: Some(ShellSemanticValue::List(items)),
    })
}

pub(crate) fn shell_storage_history_tail_list_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    count: usize,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let total = record.count as usize;
    let start = total.saturating_sub(count);
    shell_storage_history_range_list_value(runtime, device_path, start, count)
}
