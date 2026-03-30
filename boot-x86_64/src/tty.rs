use ngos_user_abi::{Errno, POLLIN, POLLOUT};

pub fn stdin_ready() -> bool {
    crate::keyboard::input_ready() || crate::serial::input_ready()
}

pub fn poll_mask_for_stdin(interest: u32) -> usize {
    if stdin_ready() {
        (interest & POLLIN) as usize
    } else {
        0
    }
}

pub const fn poll_mask_for_output(interest: u32) -> usize {
    (interest & POLLOUT) as usize
}

pub fn read_stdin_byte_nonblocking() -> Option<u8> {
    crate::keyboard::read_byte_nonblocking().or_else(crate::serial::read_byte_nonblocking)
}

pub fn read_stdin(buffer: *mut u8, len: usize, nonblock: bool) -> Result<usize, Errno> {
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }

    let mut read = 0usize;
    let mut pending = None::<u8>;
    while read < len {
        let next = pending.take().or_else(read_stdin_byte_nonblocking);
        match next {
            Some(byte) => {
                let byte = if byte == b'\r' {
                    match read_stdin_byte_nonblocking() {
                        Some(b'\n') => b'\n',
                        Some(other) => {
                            pending = Some(other);
                            b'\n'
                        }
                        None => b'\n',
                    }
                } else {
                    byte
                };
                unsafe {
                    buffer.add(read).write(byte);
                }
                read += 1;
            }
            None if read != 0 => break,
            None if nonblock => return Err(Errno::Again),
            None => core::hint::spin_loop(),
        }
    }
    Ok(read)
}

pub fn write_stdout(bytes: &[u8]) -> usize {
    crate::serial::write_bytes(bytes);
    bytes.len()
}

pub fn write_stderr(bytes: &[u8]) -> usize {
    crate::serial::write_stderr_bytes(bytes);
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn tty_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn stdin_prefers_keyboard_then_falls_back_to_serial() {
        let _guard = tty_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _io = crate::serial::lock_test_io();
        crate::keyboard::reset_state();
        crate::serial::clear_input();
        crate::serial::clear_output();
        crate::keyboard::inject_scancodes(&[0x1e, 0x1c]);
        crate::serial::inject_input(b"serial\n");

        let mut output = Vec::new();
        while let Some(byte) = read_stdin_byte_nonblocking() {
            output.push(byte);
        }

        assert_eq!(output, b"a\nserial\n");
    }

    #[test]
    fn blocking_read_stdin_normalizes_carriage_return() {
        let _guard = tty_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _io = crate::serial::lock_test_io();
        crate::keyboard::reset_state();
        crate::serial::clear_input();
        crate::serial::clear_output();
        crate::serial::inject_input(b"ab\r");

        let mut buffer = [0u8; 8];
        let read = read_stdin(buffer.as_mut_ptr(), 3, false).expect("read");
        assert_eq!(read, 3);
        assert_eq!(&buffer[..read], b"ab\n");
    }

    #[test]
    fn output_paths_preserve_stdout_stderr_split() {
        let _guard = tty_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _io = crate::serial::lock_test_io();
        crate::keyboard::reset_state();
        crate::serial::clear_input();
        crate::serial::clear_output();
        assert_eq!(write_stdout(b"ok\n"), 3);
        assert_eq!(write_stderr(b"err\n"), 4);
        assert_eq!(crate::serial::take_output(), b"ok\nerr\n");
        assert_eq!(crate::serial::take_error_output(), b"err\n");
    }
}
