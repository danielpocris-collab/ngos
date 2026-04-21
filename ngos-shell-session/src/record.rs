//! Shell record builders for domain, resource, contract, mount.

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ngos_shell_types::{
    ShellRecordField, ShellSemanticValue, ShellVariable, shell_render_record_value,
};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::label::{
    contract_kind_name, contract_state_name, mount_propagation_name, resource_arbitration_name,
    resource_governance_name, resource_kind_name, resource_state_name,
};
use crate::sources::shell_collect_waiters;

pub fn shell_domain_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<ShellVariable, ExitCode> {
    let info = runtime.inspect_domain(id).map_err(|_| 220)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_domain_name(id, &mut name).map_err(|_| 221)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 222)?;
    let fields = vec![
        ShellRecordField {
            key: String::from("id"),
            value: info.id.to_string(),
        },
        ShellRecordField {
            key: String::from("owner"),
            value: info.owner.to_string(),
        },
        ShellRecordField {
            key: String::from("parent"),
            value: info.parent.to_string(),
        },
        ShellRecordField {
            key: String::from("resources"),
            value: info.resource_count.to_string(),
        },
        ShellRecordField {
            key: String::from("contracts"),
            value: info.contract_count.to_string(),
        },
        ShellRecordField {
            key: String::from("name"),
            value: label.to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub fn shell_resource_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<ShellVariable, ExitCode> {
    let info = runtime.inspect_resource(id).map_err(|_| 207)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_resource_name(id, &mut name).map_err(|_| 224)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 225)?;
    let waiters = shell_collect_waiters(runtime, id)?;
    let fields = vec![
        ShellRecordField {
            key: String::from("id"),
            value: info.id.to_string(),
        },
        ShellRecordField {
            key: String::from("domain"),
            value: info.domain.to_string(),
        },
        ShellRecordField {
            key: String::from("creator"),
            value: info.creator.to_string(),
        },
        ShellRecordField {
            key: String::from("kind"),
            value: resource_kind_name(info.kind).to_string(),
        },
        ShellRecordField {
            key: String::from("state"),
            value: resource_state_name(info.state).to_string(),
        },
        ShellRecordField {
            key: String::from("arbitration"),
            value: resource_arbitration_name(info.arbitration).to_string(),
        },
        ShellRecordField {
            key: String::from("governance"),
            value: resource_governance_name(info.governance).to_string(),
        },
        ShellRecordField {
            key: String::from("holder"),
            value: info.holder_contract.to_string(),
        },
        ShellRecordField {
            key: String::from("waiters"),
            value: waiters,
        },
        ShellRecordField {
            key: String::from("name"),
            value: label.to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub fn shell_contract_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<ShellVariable, ExitCode> {
    let info = runtime.inspect_contract(id).map_err(|_| 226)?;
    let mut label = [0u8; 128];
    let copied = runtime
        .get_contract_label(id, &mut label)
        .map_err(|_| 227)?;
    let text = core::str::from_utf8(&label[..copied]).map_err(|_| 228)?;
    let fields = vec![
        ShellRecordField {
            key: String::from("id"),
            value: info.id.to_string(),
        },
        ShellRecordField {
            key: String::from("domain"),
            value: info.domain.to_string(),
        },
        ShellRecordField {
            key: String::from("resource"),
            value: info.resource.to_string(),
        },
        ShellRecordField {
            key: String::from("issuer"),
            value: info.issuer.to_string(),
        },
        ShellRecordField {
            key: String::from("kind"),
            value: contract_kind_name(info.kind).to_string(),
        },
        ShellRecordField {
            key: String::from("state"),
            value: contract_state_name(info.state).to_string(),
        },
        ShellRecordField {
            key: String::from("label"),
            value: text.to_string(),
        },
    ];
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub fn shell_mount_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mount_path: &str,
) -> Result<ShellVariable, ExitCode> {
    let record = runtime.inspect_mount(mount_path).map_err(|_| 246)?;
    let mut fields = Vec::with_capacity(9);
    fields.push(ShellRecordField {
        key: String::from("path"),
        value: mount_path.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("id"),
        value: record.id.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("parent"),
        value: record.parent_mount_id.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("peer_group"),
        value: record.peer_group.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("master_group"),
        value: record.master_group.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("layer"),
        value: record.layer.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("entries"),
        value: record.entry_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("mode"),
        value: mount_propagation_name(record.propagation_mode).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("created_root"),
        value: if record.created_mount_root != 0 {
            String::from("yes")
        } else {
            String::from("no")
        },
    });
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}
