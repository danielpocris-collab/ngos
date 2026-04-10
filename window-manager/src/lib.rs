#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: window management support
//! - owner layer: presentation support layer
//! - semantic owner: `window-manager`
//! - truth path role: window and interaction support for user-facing
//!   presentation flows
//!
//! Canonical contract families defined here:
//! - window state contracts
//! - window interaction support contracts
//! - compositor-facing window support contracts
//!
//! This crate may define window-management support behavior, but it must not
//! redefine kernel, runtime, or subsystem truth.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use ngos_compositor::{SurfaceId, SurfaceRect};
use ngos_gfx_translate::{DrawOp, RgbaColor};
use ngos_input_translate::{KeyEvent, MouseButton, MouseEvent, MouseEventType};

// ═══════════════════════════════════════════════════════════════════════════
// WINDOW AGENT
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub u32);

impl WindowId {
    pub fn new(id: u32) -> Self {
        WindowId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFlags {
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub closable: bool,
    pub focused: bool,
}

impl WindowFlags {
    pub const DEFAULT: Self = WindowFlags {
        resizable: true,
        minimizable: true,
        maximizable: true,
        closable: true,
        focused: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Move(i32, i32),
    Resize(u32, u32),
    Minimize,
    Maximize,
    Restore,
    Close,
    Focus,
    Blur,
    Mouse(MouseEvent),
    Key(KeyEvent),
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub state: WindowState,
    pub flags: WindowFlags,
    pub surface_id: Option<SurfaceId>,
    pub content_color: RgbaColor,
}

impl Window {
    pub fn new(id: WindowId, title: &str, x: i32, y: i32, width: u32, height: u32) -> Self {
        Window {
            id,
            title: String::from(title),
            x,
            y,
            width,
            height,
            state: WindowState::Normal,
            flags: WindowFlags::DEFAULT,
            surface_id: None,
            content_color: RgbaColor {
                r: 0x1f,
                g: 0x2a,
                b: 0x3d,
                a: 0xff,
            },
        }
    }

    pub fn with_flags(mut self, flags: WindowFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_color(mut self, color: RgbaColor) -> Self {
        self.content_color = color;
        self
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    pub fn title_bar_rect(&self) -> SurfaceRect {
        let title_height = 32.min(self.height / 8).max(1);
        SurfaceRect {
            x: self.x as u32,
            y: self.y as u32,
            width: self.width,
            height: title_height,
        }
    }

    pub fn content_rect(&self) -> SurfaceRect {
        let title_height = 32.min(self.height / 8).max(1);
        SurfaceRect {
            x: self.x as u32,
            y: (self.y + title_height as i32) as u32,
            width: self.width,
            height: self.height.saturating_sub(title_height),
        }
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn resize_to(&mut self, width: u32, height: u32) {
        if self.flags.resizable {
            self.width = width.max(100);
            self.height = height.max(80);
        }
    }

    pub fn minimize(&mut self) {
        if self.flags.minimizable {
            self.state = WindowState::Minimized;
        }
    }

    pub fn maximize(&mut self) {
        if self.flags.maximizable {
            self.state = if self.state == WindowState::Maximized {
                WindowState::Normal
            } else {
                WindowState::Maximized
            };
        }
    }

    pub fn close(&mut self) {
        if self.flags.closable {
            self.state = WindowState::Minimized;
        }
    }

    pub fn render_chrome(&self) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        let title_bar = self.title_bar_rect();

        // Title bar background
        let bar_color = if self.flags.focused {
            RgbaColor {
                r: 0x2a,
                g: 0x34,
                b: 0x48,
                a: 0xff,
            }
        } else {
            RgbaColor {
                r: 0x1a,
                g: 0x23,
                b: 0x33,
                a: 0xff,
            }
        };

        ops.push(DrawOp::Rect {
            x: title_bar.x,
            y: title_bar.y,
            width: title_bar.width,
            height: title_bar.height,
            color: bar_color,
        });

        // Title bar accent line
        if self.flags.focused {
            ops.push(DrawOp::Rect {
                x: title_bar.x,
                y: title_bar.y,
                width: 6,
                height: title_bar.height,
                color: RgbaColor {
                    r: 0x4b,
                    g: 0x92,
                    b: 0xe8,
                    a: 0xff,
                },
            });
        }

        // Window border
        ops.push(DrawOp::RoundedRect {
            x: self.x as u32,
            y: self.y as u32,
            width: self.width,
            height: self.height,
            radius: 8,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x40,
            },
        });

        // Shadow
        ops.push(DrawOp::ShadowRect {
            x: (self.x + 4) as u32,
            y: (self.y + 4) as u32,
            width: self.width,
            height: self.height,
            blur: 16,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x60,
            },
        });

        // Window buttons (close, minimize, maximize)
        let button_size = 20.min(title_bar.height);
        let button_y = title_bar.y + title_bar.height.saturating_sub(button_size) / 2;
        let mut button_x = title_bar.x + title_bar.width - 16;

        if self.flags.closable {
            button_x -= 28;
            ops.push(self.render_button(button_x, button_y, 20, 20, 0xe8, 0x4b, 0x4b));
        }

        if self.flags.maximizable {
            button_x -= 28;
            let color = if self.state == WindowState::Maximized {
                0x4b
            } else {
                0x4b
            };
            ops.push(self.render_button(button_x, button_y, 20, 20, 0x4b, 0xe8, 0x4b));
        }

        if self.flags.minimizable {
            button_x -= 28;
            ops.push(self.render_button(button_x, button_y, 20, 20, 0x4b, 0x92, 0xe8));
        }

        ops
    }

    fn render_button(&self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) -> DrawOp {
        DrawOp::RoundedRect {
            x,
            y,
            width: w,
            height: h,
            radius: 4,
            color: RgbaColor { r, g, b, a: 0xff },
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Move(x, y) => self.move_to(*x, *y),
            WindowEvent::Resize(w, h) => self.resize_to(*w, *h),
            WindowEvent::Minimize => self.minimize(),
            WindowEvent::Maximize => self.maximize(),
            WindowEvent::Restore => self.state = WindowState::Normal,
            WindowEvent::Close => self.close(),
            WindowEvent::Focus => self.flags.focused = true,
            WindowEvent::Blur => self.flags.focused = false,
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// WINDOW MANAGER
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct WindowManagerInspect {
    pub window_count: usize,
    pub focused_window: Option<WindowId>,
    pub hovered_window: Option<WindowId>,
}

pub struct WindowManager {
    windows: Vec<Window>,
    focused_window: Option<WindowId>,
    hovered_window: Option<WindowId>,
    next_window_id: u32,
    drag_state: Option<DragState>,
}

#[derive(Debug, Clone)]
struct DragState {
    window_id: WindowId,
    start_x: i32,
    start_y: i32,
    window_start_x: i32,
    window_start_y: i32,
    is_resize: bool,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows: Vec::new(),
            focused_window: None,
            hovered_window: None,
            next_window_id: 1,
            drag_state: None,
        }
    }

    pub fn create_window(
        &mut self,
        title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> WindowId {
        let id = WindowId::new(self.next_window_id);
        self.next_window_id += 1;

        let window = Window::new(id, title, x, y, width, height);
        self.windows.push(window);

        // Focus new window
        self.focus_window(id);

        id
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    pub fn close_window(&mut self, id: WindowId) -> bool {
        if let Some(idx) = self.windows.iter().position(|w| w.id == id) {
            self.windows.remove(idx);
            if self.focused_window == Some(id) {
                self.focused_window = None;
            }
            if self.hovered_window == Some(id) {
                self.hovered_window = None;
            }
            return true;
        }
        false
    }

    pub fn focus_window(&mut self, id: WindowId) {
        // Blur old focused window
        if let Some(focused_id) = self.focused_window {
            if let Some(window) = self.get_window_mut(focused_id) {
                window.handle_event(&WindowEvent::Blur);
            }
        }

        // Focus new window
        if let Some(window) = self.get_window_mut(id) {
            window.handle_event(&WindowEvent::Focus);
            self.focused_window = Some(id);

            // Move to top of stack
            if let Some(idx) = self.windows.iter().position(|w| w.id == id) {
                let window = self.windows.remove(idx);
                self.windows.push(window);
            }
        }
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent) {
        match event.event_type {
            MouseEventType::Move => {
                if let Some(drag) = self.drag_state.clone() {
                    if let Some(window) = self.get_window_mut(drag.window_id) {
                        let dx = event.x - drag.start_x;
                        let dy = event.y - drag.start_y;

                        if drag.is_resize {
                            window.resize_to(
                                (window.width as i32 + dx) as u32,
                                (window.height as i32 + dy) as u32,
                            );
                        } else {
                            window.move_to(drag.window_start_x + dx, drag.window_start_y + dy);
                        }
                    }
                } else {
                    // Find hovered window
                    self.hovered_window = None;
                    for window in self.windows.iter().rev() {
                        if window.contains_point(event.x, event.y) {
                            self.hovered_window = Some(window.id);
                            break;
                        }
                    }
                }
            }

            MouseEventType::ButtonPress(MouseButton::Left) => {
                if let Some(hovered_id) = self.hovered_window {
                    self.focus_window(hovered_id);

                    if let Some(window) = self.get_window(hovered_id) {
                        let title_bar = window.title_bar_rect();
                        if event.x >= title_bar.x as i32
                            && event.x < (title_bar.x + title_bar.width) as i32
                            && event.y >= title_bar.y as i32
                            && event.y < (title_bar.y + title_bar.height) as i32
                        {
                            // Start drag
                            self.drag_state = Some(DragState {
                                window_id: hovered_id,
                                start_x: event.x,
                                start_y: event.y,
                                window_start_x: window.x,
                                window_start_y: window.y,
                                is_resize: false,
                            });
                        }
                    }
                }
            }

            MouseEventType::ButtonRelease(MouseButton::Left) => {
                self.drag_state = None;
            }

            _ => {}
        }
    }

    pub fn handle_key(&mut self, _event: &KeyEvent) {
        // Keyboard shortcuts could be handled here
    }

    pub fn render_all(&self) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        for window in &self.windows {
            if window.state != WindowState::Minimized {
                ops.extend(window.render_chrome());
            }
        }
        ops
    }

    pub fn inspect(&self) -> WindowManagerInspect {
        WindowManagerInspect {
            window_count: self.windows.len(),
            focused_window: self.focused_window,
            hovered_window: self.hovered_window,
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_creation() {
        let window = Window::new(WindowId::new(1), "Test", 100, 100, 400, 300);
        assert_eq!(window.id.0, 1);
        assert_eq!(window.title, "Test");
        assert_eq!(window.x, 100);
        assert_eq!(window.y, 100);
        assert_eq!(window.width, 400);
        assert_eq!(window.height, 300);
    }

    #[test]
    fn window_contains_point() {
        let window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);
        assert!(window.contains_point(50, 50));
        assert!(!window.contains_point(150, 50));
    }

    #[test]
    fn window_move() {
        let mut window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);
        window.move_to(100, 200);
        assert_eq!(window.x, 100);
        assert_eq!(window.y, 200);
    }

    #[test]
    fn window_resize() {
        let mut window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);
        window.resize_to(200, 150);
        assert_eq!(window.width, 200);
        assert_eq!(window.height, 150);
    }

    #[test]
    fn window_resize_respected_flags() {
        let mut window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);
        window.flags.resizable = false;
        window.resize_to(200, 150);
        assert_eq!(window.width, 100); // Should not resize
    }

    #[test]
    fn window_minimize_maximize() {
        let mut window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);
        assert_eq!(window.state, WindowState::Normal);

        window.minimize();
        assert_eq!(window.state, WindowState::Minimized);

        // Can't maximize while minimized - restore first
        window.state = WindowState::Normal;
        window.maximize();
        assert_eq!(window.state, WindowState::Maximized);

        window.maximize();
        assert_eq!(window.state, WindowState::Normal); // Toggle back
    }

    #[test]
    fn window_manager_create_window() {
        let mut wm = WindowManager::new();
        let id = wm.create_window("Window 1", 100, 100, 400, 300);
        assert_eq!(id.0, 1);
        assert_eq!(wm.window_count(), 1);
        assert_eq!(wm.focused_window, Some(id));
    }

    #[test]
    fn window_manager_close_window() {
        let mut wm = WindowManager::new();
        let id = wm.create_window("Window 1", 0, 0, 100, 100);
        assert!(wm.close_window(id));
        assert_eq!(wm.window_count(), 0);
        assert_eq!(wm.focused_window, None);
    }

    #[test]
    fn window_manager_focus_order() {
        let mut wm = WindowManager::new();
        let id1 = wm.create_window("Window 1", 0, 0, 100, 100);
        let id2 = wm.create_window("Window 2", 10, 10, 100, 100);

        assert_eq!(wm.focused_window, Some(id2));

        wm.focus_window(id1);
        assert_eq!(wm.focused_window, Some(id1));

        // id1 should be at the end (top of stack)
        let windows: Vec<WindowId> = wm.windows.iter().map(|w| w.id).collect();
        assert_eq!(windows.last(), Some(&id1));
    }

    #[test]
    fn window_manager_render_all() {
        let mut wm = WindowManager::new();
        wm.create_window("Window 1", 0, 0, 100, 100);
        wm.create_window("Window 2", 50, 50, 100, 100);

        let ops = wm.render_all();
        assert!(!ops.is_empty());
    }

    #[test]
    fn window_manager_inspect() {
        let mut wm = WindowManager::new();
        let id = wm.create_window("Window 1", 0, 0, 100, 100);

        let inspect = wm.inspect();
        assert_eq!(inspect.window_count, 1);
        assert_eq!(inspect.focused_window, Some(id));
    }

    #[test]
    fn window_event_handling() {
        let mut window = Window::new(WindowId::new(1), "Test", 0, 0, 100, 100);

        window.handle_event(&WindowEvent::Move(50, 50));
        assert_eq!(window.x, 50);
        assert_eq!(window.y, 50);

        window.handle_event(&WindowEvent::Focus);
        assert!(window.flags.focused);

        window.handle_event(&WindowEvent::Blur);
        assert!(!window.flags.focused);
    }
}
