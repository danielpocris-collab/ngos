//! Shell UX agents — thin re-export shim to ngos-shell-ux.
pub(super) use ngos_shell_ux::{
    shell_render_history, shell_render_history_find,
    shell_render_history_tail, shell_render_recent_work,
    shell_render_help_ux, shell_render_help_topic,
    shell_render_command_card, shell_render_command_explain,
    shell_render_examples, shell_render_suggest,
    shell_render_suggest_next, shell_render_apropos,
    shell_render_whereami, shell_render_unknown_command_feedback,
    shell_is_meta_history_command,
};
