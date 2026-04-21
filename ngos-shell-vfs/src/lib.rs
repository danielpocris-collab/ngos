//! Canonical subsystem role:
//! - subsystem: shell VFS operations and command dispatchers
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-vfs`
//! - truth path role: operator-facing VFS, path, and fd actions for the ngos native shell
//!
//! Canonical contract families exposed from this crate:
//! - VFS mutation and inspection command contracts (cat, write, copy, stat, list, …)
//! - path tree traversal command contracts (tree-path, find-path)
//! - file-descriptor control command contracts (dup, close, seek, fcntl, …)
//! - shared IO helpers (write_line, shell_emit_lines, shell_write_all, shell_read_file_text, shell_read_file_bytes)

#![no_std]

extern crate alloc;

mod fd_cmd;
mod io;
mod ops;
mod path_cmd;
mod vfs_cmd;

pub use fd_cmd::try_handle_fd_agent_command;
pub use io::{
    object_kind_name, shell_emit_lines, shell_read_file_bytes, shell_read_file_text,
    shell_write_all, write_line,
};
pub use ops::{
    shell_append_file, shell_assert_file_contains, shell_cat_file, shell_copy_file,
    shell_mkdir_path, shell_mkfile_path, shell_write_file,
};
pub use path_cmd::try_handle_path_agent_command;
pub use vfs_cmd::try_handle_vfs_agent_command;

#[cfg(test)]
mod tests;
