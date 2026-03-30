#[cfg(target_os = "none")]
use core::arch::asm;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

#[cfg(target_os = "none")]
const I8042_DATA_PORT: u16 = 0x60;
#[cfg(target_os = "none")]
const I8042_STATUS_PORT: u16 = 0x64;
#[cfg(target_os = "none")]
const I8042_COMMAND_PORT: u16 = 0x64;
#[cfg(target_os = "none")]
const I8042_STATUS_OUTPUT_FULL: u8 = 1 << 0;
#[cfg(target_os = "none")]
const I8042_STATUS_INPUT_FULL: u8 = 1 << 1;
#[cfg(target_os = "none")]
const I8042_COMMAND_READ_CONFIG: u8 = 0x20;
#[cfg(target_os = "none")]
const I8042_COMMAND_WRITE_CONFIG: u8 = 0x60;
#[cfg(target_os = "none")]
const I8042_COMMAND_ENABLE_FIRST_PORT: u8 = 0xae;

const INPUT_BUFFER_CAPACITY: usize = 512;
const LINE_BUFFER_CAPACITY: usize = 256;
const EVENT_BUFFER_CAPACITY: usize = 128;

static INPUT_BUFFER: [AtomicU8; INPUT_BUFFER_CAPACITY] =
    [const { AtomicU8::new(0) }; INPUT_BUFFER_CAPACITY];
static INPUT_HEAD: AtomicUsize = AtomicUsize::new(0);
static INPUT_TAIL: AtomicUsize = AtomicUsize::new(0);

static LINE_BUFFER: [AtomicU8; LINE_BUFFER_CAPACITY] =
    [const { AtomicU8::new(0) }; LINE_BUFFER_CAPACITY];
static LINE_LEN: AtomicUsize = AtomicUsize::new(0);
static LINE_CURSOR: AtomicUsize = AtomicUsize::new(0);

static EVENT_BUFFER: [AtomicU8; EVENT_BUFFER_CAPACITY] =
    [const { AtomicU8::new(0) }; EVENT_BUFFER_CAPACITY];
static EVENT_HEAD: AtomicUsize = AtomicUsize::new(0);
static EVENT_TAIL: AtomicUsize = AtomicUsize::new(0);

static LEFT_SHIFT: AtomicBool = AtomicBool::new(false);
static RIGHT_SHIFT: AtomicBool = AtomicBool::new(false);
static LEFT_CTRL: AtomicBool = AtomicBool::new(false);
static RIGHT_CTRL: AtomicBool = AtomicBool::new(false);
static LEFT_ALT: AtomicBool = AtomicBool::new(false);
static RIGHT_ALT: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static EXTENDED_PREFIX: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Character(u8),
    Enter,
    Tab,
    Backspace,
    Escape,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Function(u8),
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    CapsLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyStateSnapshot {
    pub left_shift: bool,
    pub right_shift: bool,
    pub left_ctrl: bool,
    pub right_ctrl: bool,
    pub left_alt: bool,
    pub right_alt: bool,
    pub caps_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub action: KeyAction,
    pub state: KeyStateSnapshot,
}

pub fn init() {
    reset_state();
    #[cfg(target_os = "none")]
    unsafe {
        drain_output();
        wait_input_clear();
        outb(I8042_COMMAND_PORT, I8042_COMMAND_READ_CONFIG);
        wait_output_full();
        let mut config = inb(I8042_DATA_PORT);
        config |= 1;
        config &= !(1 << 4);
        wait_input_clear();
        outb(I8042_COMMAND_PORT, I8042_COMMAND_WRITE_CONFIG);
        wait_input_clear();
        outb(I8042_DATA_PORT, config);
        wait_input_clear();
        outb(I8042_COMMAND_PORT, I8042_COMMAND_ENABLE_FIRST_PORT);
    }
}

pub fn reset_state() {
    INPUT_HEAD.store(0, Ordering::Relaxed);
    INPUT_TAIL.store(0, Ordering::Relaxed);
    LINE_LEN.store(0, Ordering::Relaxed);
    LINE_CURSOR.store(0, Ordering::Relaxed);
    EVENT_HEAD.store(0, Ordering::Relaxed);
    EVENT_TAIL.store(0, Ordering::Relaxed);
    LEFT_SHIFT.store(false, Ordering::Relaxed);
    RIGHT_SHIFT.store(false, Ordering::Relaxed);
    LEFT_CTRL.store(false, Ordering::Relaxed);
    RIGHT_CTRL.store(false, Ordering::Relaxed);
    LEFT_ALT.store(false, Ordering::Relaxed);
    RIGHT_ALT.store(false, Ordering::Relaxed);
    CAPS_LOCK.store(false, Ordering::Relaxed);
    EXTENDED_PREFIX.store(false, Ordering::Relaxed);
}

pub fn input_ready() -> bool {
    INPUT_HEAD.load(Ordering::Acquire) != INPUT_TAIL.load(Ordering::Acquire)
}

pub fn read_byte_nonblocking() -> Option<u8> {
    ring_pop(
        &INPUT_BUFFER,
        &INPUT_HEAD,
        &INPUT_TAIL,
        INPUT_BUFFER_CAPACITY,
    )
}

pub fn state_snapshot() -> KeyStateSnapshot {
    KeyStateSnapshot {
        left_shift: LEFT_SHIFT.load(Ordering::Relaxed),
        right_shift: RIGHT_SHIFT.load(Ordering::Relaxed),
        left_ctrl: LEFT_CTRL.load(Ordering::Relaxed),
        right_ctrl: RIGHT_CTRL.load(Ordering::Relaxed),
        left_alt: LEFT_ALT.load(Ordering::Relaxed),
        right_alt: RIGHT_ALT.load(Ordering::Relaxed),
        caps_lock: CAPS_LOCK.load(Ordering::Relaxed),
    }
}

#[allow(dead_code)]
pub fn read_event_nonblocking() -> Option<KeyEvent> {
    let raw = ring_pop(
        &EVENT_BUFFER,
        &EVENT_HEAD,
        &EVENT_TAIL,
        EVENT_BUFFER_CAPACITY,
    )?;
    decode_event_byte(raw)
}

pub fn handle_irq() {
    #[cfg(target_os = "none")]
    while status_output_full() {
        let scancode = unsafe { inb(I8042_DATA_PORT) };
        handle_scancode(scancode);
    }
}

pub fn handle_scancode(scancode: u8) {
    if scancode == 0xe0 {
        EXTENDED_PREFIX.store(true, Ordering::Relaxed);
        return;
    }
    let extended = EXTENDED_PREFIX.swap(false, Ordering::Relaxed);
    let released = (scancode & 0x80) != 0;
    let code = scancode & 0x7f;
    let Some(key_code) = decode_key_code(code, extended) else {
        return;
    };
    let action = if released {
        KeyAction::Release
    } else {
        KeyAction::Press
    };
    update_modifier_state(key_code, action);
    let state = state_snapshot();
    let _ = push_event(KeyEvent {
        code: key_code,
        action,
        state,
    });
    if action == KeyAction::Press {
        apply_key_press(key_code, state);
    }
}

fn apply_key_press(code: KeyCode, state: KeyStateSnapshot) {
    match code {
        KeyCode::Character(byte) => {
            let translated = translate_character(byte, state);
            if state.left_ctrl || state.right_ctrl {
                if let Some(control) = control_byte(translated) {
                    insert_line_byte(control);
                    echo_bytes(&[caret_prefix(translated), caret_suffix(translated)]);
                    return;
                }
            }
            if insert_line_byte(translated) {
                redraw_line_from_cursor();
            }
        }
        KeyCode::Space => {
            if insert_line_byte(b' ') {
                redraw_line_from_cursor();
            }
        }
        KeyCode::Tab => {
            if insert_line_byte(b'\t') {
                redraw_line_from_cursor();
            }
        }
        KeyCode::Backspace => {
            if backspace_line_byte().is_some() {
                move_cursor_left(1);
                redraw_line_from_cursor();
            }
        }
        KeyCode::Enter => {
            commit_line();
            echo_bytes(b"\n");
        }
        KeyCode::Escape => enqueue_escape_sequence(b"\x1b"),
        KeyCode::ArrowLeft => {
            if move_line_cursor_left() {
                move_cursor_left(1);
            } else {
                enqueue_escape_sequence(b"\x1b[D");
            }
        }
        KeyCode::ArrowRight => {
            if move_line_cursor_right() {
                move_cursor_right(1);
            } else {
                enqueue_escape_sequence(b"\x1b[C");
            }
        }
        KeyCode::Home => {
            let moved = move_line_cursor_home();
            if moved != 0 {
                move_cursor_left(moved);
            } else {
                enqueue_escape_sequence(b"\x1b[H");
            }
        }
        KeyCode::End => {
            let moved = move_line_cursor_end();
            if moved != 0 {
                move_cursor_right(moved);
            } else {
                enqueue_escape_sequence(b"\x1b[F");
            }
        }
        KeyCode::Delete => {
            if delete_line_byte().is_some() {
                redraw_line_from_cursor();
            } else {
                enqueue_escape_sequence(b"\x1b[3~");
            }
        }
        KeyCode::ArrowUp => enqueue_escape_sequence(b"\x1b[A"),
        KeyCode::ArrowDown => enqueue_escape_sequence(b"\x1b[B"),
        KeyCode::Insert => enqueue_escape_sequence(b"\x1b[2~"),
        KeyCode::PageUp => enqueue_escape_sequence(b"\x1b[5~"),
        KeyCode::PageDown => enqueue_escape_sequence(b"\x1b[6~"),
        KeyCode::Function(number) => enqueue_function_key(number),
        KeyCode::LeftShift
        | KeyCode::RightShift
        | KeyCode::LeftControl
        | KeyCode::RightControl
        | KeyCode::LeftAlt
        | KeyCode::RightAlt
        | KeyCode::CapsLock => {}
    }
}

fn enqueue_function_key(number: u8) {
    let sequence = match number {
        1 => Some(b"\x1bOP".as_slice()),
        2 => Some(b"\x1bOQ".as_slice()),
        3 => Some(b"\x1bOR".as_slice()),
        4 => Some(b"\x1bOS".as_slice()),
        5 => Some(b"\x1b[15~".as_slice()),
        6 => Some(b"\x1b[17~".as_slice()),
        7 => Some(b"\x1b[18~".as_slice()),
        8 => Some(b"\x1b[19~".as_slice()),
        9 => Some(b"\x1b[20~".as_slice()),
        10 => Some(b"\x1b[21~".as_slice()),
        11 => Some(b"\x1b[23~".as_slice()),
        12 => Some(b"\x1b[24~".as_slice()),
        _ => None,
    };
    if let Some(sequence) = sequence {
        enqueue_escape_sequence(sequence);
    }
}

fn enqueue_escape_sequence(bytes: &[u8]) {
    for &byte in bytes {
        enqueue_byte(byte);
    }
}

fn insert_line_byte(byte: u8) -> bool {
    let len = LINE_LEN.load(Ordering::Acquire);
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    if len >= LINE_BUFFER_CAPACITY || cursor > len {
        return false;
    }
    for index in (cursor..len).rev() {
        let moved = LINE_BUFFER[index].load(Ordering::Relaxed);
        LINE_BUFFER[index + 1].store(moved, Ordering::Relaxed);
    }
    LINE_BUFFER[cursor].store(byte, Ordering::Relaxed);
    LINE_LEN.store(len + 1, Ordering::Release);
    LINE_CURSOR.store(cursor + 1, Ordering::Release);
    true
}

fn backspace_line_byte() -> Option<u8> {
    let len = LINE_LEN.load(Ordering::Acquire);
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    if len == 0 || cursor == 0 || cursor > len {
        return None;
    }
    let index = cursor - 1;
    let byte = LINE_BUFFER[index].load(Ordering::Relaxed);
    for shift in index + 1..len {
        let moved = LINE_BUFFER[shift].load(Ordering::Relaxed);
        LINE_BUFFER[shift - 1].store(moved, Ordering::Relaxed);
    }
    LINE_LEN.store(len - 1, Ordering::Release);
    LINE_CURSOR.store(cursor - 1, Ordering::Release);
    Some(byte)
}

fn delete_line_byte() -> Option<u8> {
    let len = LINE_LEN.load(Ordering::Acquire);
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    if cursor >= len {
        return None;
    }
    let byte = LINE_BUFFER[cursor].load(Ordering::Relaxed);
    for shift in cursor + 1..len {
        let moved = LINE_BUFFER[shift].load(Ordering::Relaxed);
        LINE_BUFFER[shift - 1].store(moved, Ordering::Relaxed);
    }
    LINE_LEN.store(len - 1, Ordering::Release);
    Some(byte)
}

fn commit_line() {
    let len = LINE_LEN.swap(0, Ordering::AcqRel);
    for index in 0..len {
        enqueue_byte(LINE_BUFFER[index].load(Ordering::Relaxed));
    }
    LINE_CURSOR.store(0, Ordering::Release);
    enqueue_byte(b'\n');
}

fn move_line_cursor_left() -> bool {
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    if cursor == 0 {
        return false;
    }
    LINE_CURSOR.store(cursor - 1, Ordering::Release);
    true
}

fn move_line_cursor_right() -> bool {
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    let len = LINE_LEN.load(Ordering::Acquire);
    if cursor >= len {
        return false;
    }
    LINE_CURSOR.store(cursor + 1, Ordering::Release);
    true
}

fn move_line_cursor_home() -> usize {
    let cursor = LINE_CURSOR.swap(0, Ordering::AcqRel);
    cursor
}

fn move_line_cursor_end() -> usize {
    let len = LINE_LEN.load(Ordering::Acquire);
    let cursor = LINE_CURSOR.swap(len, Ordering::AcqRel);
    len.saturating_sub(cursor)
}

fn redraw_line_from_cursor() {
    let cursor = LINE_CURSOR.load(Ordering::Acquire);
    let len = LINE_LEN.load(Ordering::Acquire);
    for index in cursor.saturating_sub(1)..len {
        echo_bytes(&[LINE_BUFFER[index].load(Ordering::Relaxed)]);
    }
    echo_bytes(b" ");
    let tail = len.saturating_sub(cursor);
    if tail != 0 {
        move_cursor_left(tail + 1);
    } else {
        move_cursor_left(1);
    }
}

fn move_cursor_left(count: usize) {
    for _ in 0..count {
        echo_bytes(b"\x08");
    }
}

fn move_cursor_right(count: usize) {
    for _ in 0..count {
        echo_bytes(b"\x1b[C");
    }
}

fn enqueue_byte(byte: u8) {
    let _ = ring_push(
        &INPUT_BUFFER,
        &INPUT_HEAD,
        &INPUT_TAIL,
        INPUT_BUFFER_CAPACITY,
        byte,
    );
}

fn push_event(event: KeyEvent) -> bool {
    let Some(raw) = encode_event_byte(event.code, event.action) else {
        return false;
    };
    ring_push(
        &EVENT_BUFFER,
        &EVENT_HEAD,
        &EVENT_TAIL,
        EVENT_BUFFER_CAPACITY,
        raw,
    )
}

fn ring_push(
    storage: &[AtomicU8],
    head: &AtomicUsize,
    tail: &AtomicUsize,
    capacity: usize,
    byte: u8,
) -> bool {
    let current_head = head.load(Ordering::Acquire);
    let next = (current_head + 1) % capacity;
    if next == tail.load(Ordering::Acquire) {
        return false;
    }
    storage[current_head].store(byte, Ordering::Relaxed);
    head.store(next, Ordering::Release);
    true
}

fn ring_pop(
    storage: &[AtomicU8],
    head: &AtomicUsize,
    tail: &AtomicUsize,
    capacity: usize,
) -> Option<u8> {
    let current_tail = tail.load(Ordering::Acquire);
    if current_tail == head.load(Ordering::Acquire) {
        return None;
    }
    let byte = storage[current_tail].load(Ordering::Relaxed);
    tail.store((current_tail + 1) % capacity, Ordering::Release);
    Some(byte)
}

fn update_modifier_state(code: KeyCode, action: KeyAction) {
    let pressed = action == KeyAction::Press;
    match code {
        KeyCode::LeftShift => LEFT_SHIFT.store(pressed, Ordering::Relaxed),
        KeyCode::RightShift => RIGHT_SHIFT.store(pressed, Ordering::Relaxed),
        KeyCode::LeftControl => LEFT_CTRL.store(pressed, Ordering::Relaxed),
        KeyCode::RightControl => RIGHT_CTRL.store(pressed, Ordering::Relaxed),
        KeyCode::LeftAlt => LEFT_ALT.store(pressed, Ordering::Relaxed),
        KeyCode::RightAlt => RIGHT_ALT.store(pressed, Ordering::Relaxed),
        KeyCode::CapsLock if action == KeyAction::Press => {
            CAPS_LOCK.fetch_xor(true, Ordering::Relaxed);
        }
        _ => {}
    }
}

fn decode_key_code(code: u8, extended: bool) -> Option<KeyCode> {
    if extended {
        return match code {
            0x1c => Some(KeyCode::Enter),
            0x1d => Some(KeyCode::RightControl),
            0x35 => Some(KeyCode::Character(b'/')),
            0x38 => Some(KeyCode::RightAlt),
            0x47 => Some(KeyCode::Home),
            0x48 => Some(KeyCode::ArrowUp),
            0x49 => Some(KeyCode::PageUp),
            0x4b => Some(KeyCode::ArrowLeft),
            0x4d => Some(KeyCode::ArrowRight),
            0x4f => Some(KeyCode::End),
            0x50 => Some(KeyCode::ArrowDown),
            0x51 => Some(KeyCode::PageDown),
            0x52 => Some(KeyCode::Insert),
            0x53 => Some(KeyCode::Delete),
            _ => None,
        };
    }

    match code {
        0x01 => Some(KeyCode::Escape),
        0x02 => Some(KeyCode::Character(b'1')),
        0x03 => Some(KeyCode::Character(b'2')),
        0x04 => Some(KeyCode::Character(b'3')),
        0x05 => Some(KeyCode::Character(b'4')),
        0x06 => Some(KeyCode::Character(b'5')),
        0x07 => Some(KeyCode::Character(b'6')),
        0x08 => Some(KeyCode::Character(b'7')),
        0x09 => Some(KeyCode::Character(b'8')),
        0x0a => Some(KeyCode::Character(b'9')),
        0x0b => Some(KeyCode::Character(b'0')),
        0x0c => Some(KeyCode::Character(b'-')),
        0x0d => Some(KeyCode::Character(b'=')),
        0x0e => Some(KeyCode::Backspace),
        0x0f => Some(KeyCode::Tab),
        0x10 => Some(KeyCode::Character(b'q')),
        0x11 => Some(KeyCode::Character(b'w')),
        0x12 => Some(KeyCode::Character(b'e')),
        0x13 => Some(KeyCode::Character(b'r')),
        0x14 => Some(KeyCode::Character(b't')),
        0x15 => Some(KeyCode::Character(b'y')),
        0x16 => Some(KeyCode::Character(b'u')),
        0x17 => Some(KeyCode::Character(b'i')),
        0x18 => Some(KeyCode::Character(b'o')),
        0x19 => Some(KeyCode::Character(b'p')),
        0x1a => Some(KeyCode::Character(b'[')),
        0x1b => Some(KeyCode::Character(b']')),
        0x1c => Some(KeyCode::Enter),
        0x1d => Some(KeyCode::LeftControl),
        0x1e => Some(KeyCode::Character(b'a')),
        0x1f => Some(KeyCode::Character(b's')),
        0x20 => Some(KeyCode::Character(b'd')),
        0x21 => Some(KeyCode::Character(b'f')),
        0x22 => Some(KeyCode::Character(b'g')),
        0x23 => Some(KeyCode::Character(b'h')),
        0x24 => Some(KeyCode::Character(b'j')),
        0x25 => Some(KeyCode::Character(b'k')),
        0x26 => Some(KeyCode::Character(b'l')),
        0x27 => Some(KeyCode::Character(b';')),
        0x28 => Some(KeyCode::Character(b'\'')),
        0x29 => Some(KeyCode::Character(b'`')),
        0x2a => Some(KeyCode::LeftShift),
        0x2b => Some(KeyCode::Character(b'\\')),
        0x2c => Some(KeyCode::Character(b'z')),
        0x2d => Some(KeyCode::Character(b'x')),
        0x2e => Some(KeyCode::Character(b'c')),
        0x2f => Some(KeyCode::Character(b'v')),
        0x30 => Some(KeyCode::Character(b'b')),
        0x31 => Some(KeyCode::Character(b'n')),
        0x32 => Some(KeyCode::Character(b'm')),
        0x33 => Some(KeyCode::Character(b',')),
        0x34 => Some(KeyCode::Character(b'.')),
        0x35 => Some(KeyCode::Character(b'/')),
        0x36 => Some(KeyCode::RightShift),
        0x38 => Some(KeyCode::LeftAlt),
        0x39 => Some(KeyCode::Space),
        0x3a => Some(KeyCode::CapsLock),
        0x3b => Some(KeyCode::Function(1)),
        0x3c => Some(KeyCode::Function(2)),
        0x3d => Some(KeyCode::Function(3)),
        0x3e => Some(KeyCode::Function(4)),
        0x3f => Some(KeyCode::Function(5)),
        0x40 => Some(KeyCode::Function(6)),
        0x41 => Some(KeyCode::Function(7)),
        0x42 => Some(KeyCode::Function(8)),
        0x43 => Some(KeyCode::Function(9)),
        0x44 => Some(KeyCode::Function(10)),
        0x57 => Some(KeyCode::Function(11)),
        0x58 => Some(KeyCode::Function(12)),
        _ => None,
    }
}

fn control_byte(byte: u8) -> Option<u8> {
    let lower = byte.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        Some(lower - b'a' + 1)
    } else {
        match byte {
            b'[' => Some(0x1b),
            b'\\' => Some(0x1c),
            b']' => Some(0x1d),
            b'6' => Some(0x1e),
            b'-' => Some(0x1f),
            _ => None,
        }
    }
}

fn translate_character(byte: u8, state: KeyStateSnapshot) -> u8 {
    let shifted = state.left_shift || state.right_shift;
    if byte.is_ascii_alphabetic() {
        let uppercase = shifted ^ state.caps_lock;
        if uppercase {
            byte.to_ascii_uppercase()
        } else {
            byte.to_ascii_lowercase()
        }
    } else if shifted {
        match byte {
            b'1' => b'!',
            b'2' => b'@',
            b'3' => b'#',
            b'4' => b'$',
            b'5' => b'%',
            b'6' => b'^',
            b'7' => b'&',
            b'8' => b'*',
            b'9' => b'(',
            b'0' => b')',
            b'-' => b'_',
            b'=' => b'+',
            b'[' => b'{',
            b']' => b'}',
            b'\\' => b'|',
            b';' => b':',
            b'\'' => b'"',
            b'`' => b'~',
            b',' => b'<',
            b'.' => b'>',
            b'/' => b'?',
            _ => byte,
        }
    } else {
        byte
    }
}

fn caret_prefix(byte: u8) -> u8 {
    if byte.is_ascii_lowercase() {
        b'^'
    } else {
        b'^'
    }
}

fn caret_suffix(byte: u8) -> u8 {
    byte.to_ascii_uppercase()
}

fn encode_event_byte(code: KeyCode, action: KeyAction) -> Option<u8> {
    let code = match code {
        KeyCode::Character(_) => return None,
        KeyCode::Enter => 1,
        KeyCode::Tab => 2,
        KeyCode::Backspace => 3,
        KeyCode::Escape => 4,
        KeyCode::Space => 5,
        KeyCode::ArrowUp => 6,
        KeyCode::ArrowDown => 7,
        KeyCode::ArrowLeft => 8,
        KeyCode::ArrowRight => 9,
        KeyCode::Home => 10,
        KeyCode::End => 11,
        KeyCode::Insert => 12,
        KeyCode::Delete => 13,
        KeyCode::PageUp => 14,
        KeyCode::PageDown => 15,
        KeyCode::Function(number) => 16 + number,
        KeyCode::LeftShift => 40,
        KeyCode::RightShift => 41,
        KeyCode::LeftControl => 42,
        KeyCode::RightControl => 43,
        KeyCode::LeftAlt => 44,
        KeyCode::RightAlt => 45,
        KeyCode::CapsLock => 46,
    };
    Some((code << 1) | u8::from(action == KeyAction::Release))
}

#[allow(dead_code)]
fn decode_event_byte(raw: u8) -> Option<KeyEvent> {
    let action = if (raw & 1) != 0 {
        KeyAction::Release
    } else {
        KeyAction::Press
    };
    let code = match raw >> 1 {
        1 => KeyCode::Enter,
        2 => KeyCode::Tab,
        3 => KeyCode::Backspace,
        4 => KeyCode::Escape,
        5 => KeyCode::Space,
        6 => KeyCode::ArrowUp,
        7 => KeyCode::ArrowDown,
        8 => KeyCode::ArrowLeft,
        9 => KeyCode::ArrowRight,
        10 => KeyCode::Home,
        11 => KeyCode::End,
        12 => KeyCode::Insert,
        13 => KeyCode::Delete,
        14 => KeyCode::PageUp,
        15 => KeyCode::PageDown,
        17..=28 => KeyCode::Function((raw >> 1) - 16),
        40 => KeyCode::LeftShift,
        41 => KeyCode::RightShift,
        42 => KeyCode::LeftControl,
        43 => KeyCode::RightControl,
        44 => KeyCode::LeftAlt,
        45 => KeyCode::RightAlt,
        46 => KeyCode::CapsLock,
        _ => return None,
    };
    Some(KeyEvent {
        code,
        action,
        state: state_snapshot(),
    })
}

fn echo_bytes(bytes: &[u8]) {
    crate::serial::write_bytes(bytes);
}

#[cfg(target_os = "none")]
fn status_output_full() -> bool {
    unsafe { (inb(I8042_STATUS_PORT) & I8042_STATUS_OUTPUT_FULL) != 0 }
}

#[cfg(target_os = "none")]
unsafe fn drain_output() {
    while status_output_full() {
        let _ = unsafe { inb(I8042_DATA_PORT) };
    }
}

#[cfg(target_os = "none")]
unsafe fn wait_input_clear() {
    while (unsafe { inb(I8042_STATUS_PORT) } & I8042_STATUS_INPUT_FULL) != 0 {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
unsafe fn wait_output_full() {
    while !status_output_full() {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[cfg(target_os = "none")]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

#[cfg(test)]
pub fn inject_scancodes(scancodes: &[u8]) {
    for &scancode in scancodes {
        handle_scancode(scancode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn keyboard_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn drain_input() -> Vec<u8> {
        let mut output = Vec::new();
        while let Some(byte) = read_byte_nonblocking() {
            output.push(byte);
        }
        output
    }

    #[test]
    fn modifier_state_tracks_shift_control_alt_and_caps_lock() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[0x2a, 0x1d, 0x38, 0x3a]);
        let state = state_snapshot();
        assert!(state.left_shift);
        assert!(state.left_ctrl);
        assert!(state.left_alt);
        assert!(state.caps_lock);

        inject_scancodes(&[0xaa, 0x9d, 0xb8]);
        let state = state_snapshot();
        assert!(!state.left_shift);
        assert!(!state.left_ctrl);
        assert!(!state.left_alt);
        assert!(state.caps_lock);
    }

    #[test]
    fn printable_keys_backspace_and_enter_feed_canonical_stdin_buffer() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[0x1e, 0x30, 0x0e, 0x2e, 0x1c]);
        assert_eq!(drain_input(), b"ac\n");
    }

    #[test]
    fn line_cursor_supports_mid_line_insert_delete_and_commit() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[
            0x23, 0x12, 0x26, 0x26, 0x18, // hello
            0xe0, 0x4b, 0xe0, 0x4b, // left left
            0x14, // t => hel t lo
            0xe0, 0x53, // delete => remove first l after cursor
            0x1c,
        ]);
        assert_eq!(drain_input(), b"helto\n");
    }

    #[test]
    fn home_end_and_right_arrow_move_cursor_within_line() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[
            0x1e, 0x30, 0x2e, // abc
            0xe0, 0x47, // home
            0x2d, // xabc
            0xe0, 0x4d, 0xe0, 0x4d, // right right
            0x20, // xadbc
            0xe0, 0x4f, // end
            0x12, // e
            0x1c,
        ]);
        assert_eq!(drain_input(), b"xabdce\n");
    }

    #[test]
    fn caps_lock_shift_and_control_translate_letters_coherently() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[0x3a, 0x1e, 0x2a, 0x1e, 0xaa, 0x1d, 0x1e, 0x9d, 0x1c]);
        assert_eq!(drain_input(), vec![b'A', b'a', 0x01, b'\n']);
    }

    #[test]
    fn e0_special_keys_translate_to_escape_sequences() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[
            0xe0, 0x48, 0xe0, 0x50, 0xe0, 0x4b, 0xe0, 0x4d, 0xe0, 0x47, 0xe0, 0x4f, 0xe0, 0x52,
            0xe0, 0x53,
        ]);
        assert_eq!(
            drain_input(),
            b"\x1b[A\x1b[B\x1b[D\x1b[C\x1b[H\x1b[F\x1b[2~\x1b[3~"
        );
    }

    #[test]
    fn function_keys_and_tab_are_translated() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[0x3b, 0x3c, 0x0f, 0x1c]);
        assert_eq!(drain_input(), b"\x1bOP\x1bOQ\t\n");
    }

    #[test]
    fn event_queue_tracks_press_and_release_for_special_keys() {
        let _guard = keyboard_test_lock().lock().unwrap();
        reset_state();
        inject_scancodes(&[0x2a, 0xaa, 0xe0, 0x48, 0xe0, 0xc8]);

        let shift_press = read_event_nonblocking().expect("expected shift press");
        assert_eq!(shift_press.code, KeyCode::LeftShift);
        assert_eq!(shift_press.action, KeyAction::Press);

        let shift_release = read_event_nonblocking().expect("expected shift release");
        assert_eq!(shift_release.code, KeyCode::LeftShift);
        assert_eq!(shift_release.action, KeyAction::Release);

        let up_press = read_event_nonblocking().expect("expected up press");
        assert_eq!(up_press.code, KeyCode::ArrowUp);
        assert_eq!(up_press.action, KeyAction::Press);

        let up_release = read_event_nonblocking().expect("expected up release");
        assert_eq!(up_release.code, KeyCode::ArrowUp);
        assert_eq!(up_release.action, KeyAction::Release);
    }
}
