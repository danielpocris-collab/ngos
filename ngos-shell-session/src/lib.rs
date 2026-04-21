//! Shell session orchestration, pipeline execution, and session rendering.

#![no_std]
extern crate alloc;

mod label;
mod record;
mod render;
mod sources;

pub mod pipeline;
pub mod session_cmd;

pub use record::{
    shell_contract_record, shell_domain_record, shell_mount_record, shell_resource_record,
};
pub use render::{
    shell_render_aliases, shell_render_env, shell_render_variables, shell_session_record,
};
pub use session_cmd::try_handle_session_agent_command;
