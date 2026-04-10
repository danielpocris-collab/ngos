//! Session-level rendering: aliases, variables, env, session record.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;

use ngos_shell_types::{
    ShellAlias, ShellRecordField, ShellSemanticValue, ShellVariable, shell_render_record_value,
    shell_variable_type_name,
};
use ngos_user_abi::bootstrap::{BootOutcomePolicy, SessionContext};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub(crate) fn write_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    text: &str,
) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn shell_render_aliases<B: SyscallBackend>(
    runtime: &Runtime<B>,
    aliases: &[ShellAlias],
) -> Result<(), ExitCode> {
    if aliases.is_empty() {
        return write_line(runtime, "aliases=0");
    }
    for alias in aliases {
        write_line(runtime, &format!("alias {}='{}'", alias.name, alias.value))?;
    }
    Ok(())
}

pub fn shell_render_variables<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &[ngos_shell_types::ShellVariable],
) -> Result<(), ExitCode> {
    if variables.is_empty() {
        return write_line(runtime, "vars=0");
    }
    for variable in variables {
        write_line(
            runtime,
            &format!(
                "var {}={} type={}",
                variable.name,
                variable.value,
                shell_variable_type_name(variable)
            ),
        )?;
    }
    Ok(())
}

pub fn shell_render_env<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
) -> Result<(), ExitCode> {
    write_line(runtime, &format!("protocol={}", context.protocol))?;
    write_line(runtime, &format!("process={}", context.process_name))?;
    write_line(runtime, &format!("image={}", context.image_path))?;
    write_line(runtime, &format!("cwd={cwd}"))?;
    write_line(
        runtime,
        &format!("root_mount_path={}", context.root_mount_path),
    )?;
    write_line(
        runtime,
        &format!("root_mount_name={}", context.root_mount_name),
    )?;
    write_line(runtime, &format!("page_size={}", context.page_size))?;
    write_line(runtime, &format!("entry=0x{:x}", context.entry))?;
    write_line(runtime, &format!("image_base=0x{:x}", context.image_base))?;
    write_line(runtime, &format!("stack_top=0x{:x}", context.stack_top))?;
    write_line(runtime, &format!("phdr=0x{:x}", context.phdr))?;
    write_line(runtime, &format!("phent={}", context.phent))?;
    write_line(runtime, &format!("phnum={}", context.phnum))?;
    write_line(
        runtime,
        &format!(
            "outcome_policy={}",
            match context.outcome_policy {
                BootOutcomePolicy::RequireZeroExit => "require-zero-exit",
                BootOutcomePolicy::AllowAnyExit => "allow-any-exit",
            }
        ),
    )
}

pub fn shell_session_record(context: &SessionContext, cwd: &str) -> ShellVariable {
    let fields = vec![
        ShellRecordField {
            key: String::from("protocol"),
            value: context.protocol.clone(),
        },
        ShellRecordField {
            key: String::from("cwd"),
            value: cwd.to_string(),
        },
        ShellRecordField {
            key: String::from("root"),
            value: context.root_mount_path.clone(),
        },
        ShellRecordField {
            key: String::from("image"),
            value: context.image_path.clone(),
        },
        ShellRecordField {
            key: String::from("process"),
            value: context.process_name.clone(),
        },
        ShellRecordField {
            key: String::from("entry"),
            value: format!("{:#x}", context.entry),
        },
    ];
    ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    }
}
