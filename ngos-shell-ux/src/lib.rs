//! Shell UX nano-semantic crate: history, help, suggest, catalog.

#![no_std]
extern crate alloc;

mod catalog;
mod help;
mod history;
mod suggest;

pub use catalog::{
    proof_command_summary, shell_guess_ux_topic, shell_is_meta_history_command, shell_ux_catalog,
    SHELL_UX_COMMANDS, UX_PROOF_COMMANDS,
};
pub use help::{
    shell_render_command_card, shell_render_command_explain, shell_render_examples,
    shell_render_help_topic, shell_render_help_ux,
};
pub use history::{
    shell_render_history, shell_render_history_find, shell_render_history_tail,
    shell_render_recent_work,
};
pub use suggest::{
    shell_render_apropos, shell_render_suggest, shell_render_suggest_next,
    shell_render_unknown_command_feedback, shell_render_whereami,
};
