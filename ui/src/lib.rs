#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: UI presentation support
//! - owner layer: presentation support layer
//! - semantic owner: `ui`
//! - truth path role: presentation and composition support for `ngos`
//!   user-facing surfaces
//!
//! Canonical contract families handled here:
//! - compositor presentation contracts
//! - UI presenter contracts
//! - boot/desktop presentation support contracts
//!
//! This crate may render and compose user-facing surfaces, but it must not
//! redefine kernel, runtime, or subsystem truth.

extern crate alloc;
#[cfg(feature = "skia-preview")]
extern crate std;

#[cfg(feature = "skia-preview")]
use alloc::string::String;
use alloc::vec::Vec;
use ngos_compositor::{
    Compositor, CompositorError, CompositorInspect, Surface, SurfaceRect, SurfaceRole,
};
use ngos_gfx_translate::FrameScript;
use ngos_scene_graph::{
    Camera, OrthographicCamera, SceneGraph, Transform, Vec3, submit as submit_scene,
};

pub mod boot_screen;
mod desktop;
mod logo;
mod presenter;
mod sidebar;
#[cfg(feature = "skia-preview")]
mod skia_preview;
mod start_menu;
mod taskbar;
mod top_bar;
mod window_manager;

pub use boot_screen::{BootScreen, BootStage};
pub use desktop::{Desktop, DesktopIcon};
pub use logo::{LogoSize, NGOSLogo};
pub use presenter::{UiPresentationBackend, UiPresenter};
pub use sidebar::{Sidebar, SidebarItem};
#[cfg(feature = "skia-preview")]
pub use skia_preview::UiSkiaPreview;
pub use start_menu::{StartMenu, StartMenuItem};
pub use taskbar::{Taskbar, TaskbarItem};
pub use top_bar::{TopBar, TopBarMenu};
pub use window_manager::{UIManager, UIWindow, WindowState};

/// NGOS User Interface System
///
/// Provides native UI rendering using FrameScript and DrawOp
pub struct UserInterface {
    width: u32,
    height: u32,
    boot_screen: BootScreen,
    taskbar: Taskbar,
    window_manager: UIManager,
    start_menu: StartMenu,
    logo: NGOSLogo,
    desktop: Desktop,
    top_bar: TopBar,
    sidebar: Sidebar,
    notification_center_visible: bool,
    control_center_visible: bool,
    widgets_panel_visible: bool,
}

pub struct DesktopRenderInspect {
    pub frame_tag: alloc::string::String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub surface_count: usize,
    pub scene_node_count: usize,
    pub scene_depth: usize,
    pub compositor: CompositorInspect,
}

#[derive(Debug, Clone, Copy)]
struct UiScale {
    width: u32,
    height: u32,
}

impl UiScale {
    fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl UserInterface {
    pub fn new(width: u32, height: u32) -> Self {
        UserInterface {
            width,
            height,
            boot_screen: BootScreen::new(width, height),
            taskbar: Taskbar::new(width, height),
            window_manager: UIManager::new(width, height),
            start_menu: StartMenu::new(width, height),
            logo: NGOSLogo::new(150),
            desktop: Desktop::new(width, height),
            top_bar: TopBar::new(width, height),
            sidebar: Sidebar::new(width, height),
            notification_center_visible: false,
            control_center_visible: false,
            widgets_panel_visible: false,
        }
    }

    /// Render boot screen
    pub fn render_boot(&self, stage: BootStage) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.boot_screen.render(stage)
    }

    #[cfg(feature = "skia-preview")]
    pub fn render_boot_png(&self, stage: BootStage, output: &str) -> Result<(), String> {
        presenter::UiPresenter::present_ops(
            &self.render_boot(stage),
            self.width,
            self.height,
            output,
        )
    }

    /// Render desktop with all components
    pub fn render_desktop(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.compose_desktop_frame()
            .map(|script| script.ops)
            .unwrap_or_else(|_| {
                let scale = UiScale::new(self.width, self.height);
                let mut fallback = Vec::new();
                fallback.extend(self.desktop.render_scaled(scale.width, scale.height));
                fallback.extend(self.top_bar.render_scaled(scale.width, scale.height));
                fallback.extend(self.sidebar.render_scaled(scale.width, scale.height));
                fallback.extend(
                    self.window_manager
                        .render_all_scaled(scale.width, scale.height),
                );
                if self.widgets_panel_visible {
                    fallback.extend(self.render_widgets_panel());
                }
                if self.notification_center_visible {
                    fallback.extend(self.render_notification_center());
                }
                if self.control_center_visible {
                    fallback.extend(self.render_control_center());
                }
                fallback.extend(self.taskbar.render_scaled(scale.width, scale.height));
                if self.start_menu.is_visible() {
                    fallback.extend(self.start_menu.render_scaled(scale.width, scale.height));
                }
                fallback
            })
    }

    pub fn compose_desktop_frame(&self) -> Result<FrameScript, CompositorError> {
        let scale = UiScale::new(self.width, self.height);
        let mut compositor = Compositor::new(scale.width, scale.height);
        for surface in self.desktop_surfaces(scale.width, scale.height) {
            compositor.push_surface(surface)?;
        }
        compositor.compose("ngos-desktop", "graphics", "mailbox", "wait-present")
    }

    pub fn inspect_desktop_render(&self) -> Result<DesktopRenderInspect, CompositorError> {
        let scale = UiScale::new(self.width, self.height);
        let (graph, camera_node, scene_entries) =
            self.desktop_scene_plan(scale.width, scale.height);
        let mut compositor = Compositor::new(scale.width, scale.height);
        let surfaces = self.resolve_desktop_scene_surfaces(
            &graph,
            camera_node,
            scene_entries,
            scale.width,
            scale.height,
        );
        let surface_count = surfaces.len();
        for surface in surfaces {
            compositor.push_surface(surface)?;
        }
        let scene = graph.inspect();
        Ok(DesktopRenderInspect {
            frame_tag: alloc::string::String::from("ngos-desktop"),
            viewport_width: scale.width,
            viewport_height: scale.height,
            surface_count,
            scene_node_count: scene.node_count,
            scene_depth: scene.max_depth,
            compositor: compositor.inspect(),
        })
    }

    #[cfg(feature = "skia-preview")]
    pub fn render_desktop_png(&self, output: &str) -> Result<(), String> {
        presenter::UiPresenter::present_ops(&self.render_desktop(), self.width, self.height, output)
    }

    #[cfg(feature = "skia-preview")]
    pub fn render_suite_png(&self, output: &str) -> Result<(), String> {
        skia_preview::UiSkiaPreview::render_suite_to_png(self, output)
    }

    #[cfg(feature = "skia-preview")]
    pub fn render_master_suite_png(&self, output: &str) -> Result<(), String> {
        skia_preview::UiSkiaPreview::render_master_suite_to_png(self, output)
    }

    /// Render NGOS logo centered
    pub fn render_logo(&self, x: u32, y: u32) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.logo.render(x, y)
    }

    /// Handle mouse input
    pub fn handle_mouse(
        &mut self,
        x: i32,
        y: i32,
        button: ngos_input_translate::mouse_agent::MouseButton,
        pressed: bool,
    ) {
        // Check taskbar click
        if let Some(action) = self.taskbar.handle_click(x, y, button, pressed) {
            match action {
                TaskbarAction::OpenStart => self.start_menu.toggle(),
                TaskbarAction::OpenApp(app_id) => {
                    self.window_manager.open_window(app_id);
                }
            }
        }

        // Quick desktop-region interactions for panel toggles.
        if pressed {
            if x >= self.width.saturating_sub(60) as i32 && y <= 80 {
                self.toggle_notification_center();
            } else if x >= self.width.saturating_sub(120) as i32
                && x < self.width.saturating_sub(60) as i32
                && y <= 80
            {
                self.toggle_control_center();
            } else if x <= 90 && y <= 80 {
                self.toggle_widgets_panel();
            }
        }

        // Handle window drag/move
        self.window_manager.handle_mouse(x, y, button, pressed);
    }

    /// Handle keyboard input
    pub fn handle_key(
        &mut self,
        key: ngos_input_translate::keyboard_agent::KeyCode,
        pressed: bool,
    ) {
        if pressed {
            match key {
                ngos_input_translate::keyboard_agent::KeyCode::Super => self.start_menu.toggle(),
                ngos_input_translate::keyboard_agent::KeyCode::Escape => {
                    self.start_menu = StartMenu::new(self.width, self.height);
                    self.notification_center_visible = false;
                    self.control_center_visible = false;
                    self.widgets_panel_visible = false;
                }
                _ => {}
            }
        }
    }

    /// Update UI state
    pub fn update(&mut self, delta_ms: u32) {
        self.boot_screen.update(delta_ms);
        self.window_manager.update(delta_ms);
    }

    /// Get boot progress (0-100)
    pub fn boot_progress(&self) -> u8 {
        self.boot_screen.progress()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn toggle_widgets_panel(&mut self) {
        self.widgets_panel_visible = !self.widgets_panel_visible;
        if self.widgets_panel_visible {
            self.notification_center_visible = false;
            self.control_center_visible = false;
        }
    }

    pub fn toggle_notification_center(&mut self) {
        self.notification_center_visible = !self.notification_center_visible;
        if self.notification_center_visible {
            self.widgets_panel_visible = false;
        }
    }

    pub fn toggle_control_center(&mut self) {
        self.control_center_visible = !self.control_center_visible;
        if self.control_center_visible {
            self.widgets_panel_visible = false;
        }
    }

    pub fn open_settings_panel(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.render_settings_panel()
    }

    pub fn open_context_menu(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.render_context_menu()
    }

    fn desktop_surfaces(&self, width: u32, height: u32) -> Vec<Surface> {
        let (graph, camera_node, scene_entries) = self.desktop_scene_plan(width, height);
        self.resolve_desktop_scene_surfaces(&graph, camera_node, scene_entries, width, height)
    }

    fn desktop_scene_plan(
        &self,
        width: u32,
        height: u32,
    ) -> (SceneGraph, u32, Vec<(u32, Surface)>) {
        let mut graph = SceneGraph::new();
        let root = graph.add_labeled(Transform::IDENTITY, "desktop-root");
        let camera_node = graph.add_labeled(
            Transform {
                translation: Vec3::new((width / 2) as f32, (height / 2) as f32, 240.0),
                ..Transform::IDENTITY
            },
            "desktop-camera",
        );
        let _ = graph.set_parent(camera_node, root);

        let mut scene_entries: Vec<(u32, Surface)> = Vec::new();

        let mut background = Surface::new(
            1,
            SurfaceRole::Background,
            SurfaceRect {
                x: 0,
                y: 0,
                width,
                height,
            },
        )
        .expect("background surface should be valid");
        background.pass_name = alloc::string::String::from("desktop-background");
        background.content = self.desktop.render_scaled(width, height);
        self.push_desktop_scene_surface(
            &mut graph,
            &mut scene_entries,
            root,
            "desktop-background",
            0.0,
            background,
        );

        let top_bar_h = (height / 13).max(60);
        let mut top_bar = Surface::new(
            2,
            SurfaceRole::Panel,
            SurfaceRect {
                x: 0,
                y: 0,
                width,
                height: top_bar_h,
            },
        )
        .expect("top bar surface should be valid");
        top_bar.pass_name = alloc::string::String::from("desktop-top-bar");
        top_bar.content = self.top_bar.render_scaled(width, height);
        self.push_desktop_scene_surface(
            &mut graph,
            &mut scene_entries,
            root,
            "desktop-top-bar",
            20.0,
            top_bar,
        );

        let sidebar_w = (width / 5).max(240);
        let mut sidebar = Surface::new(
            3,
            SurfaceRole::Panel,
            SurfaceRect {
                x: 0,
                y: top_bar_h,
                width: sidebar_w,
                height: height.saturating_sub(top_bar_h),
            },
        )
        .expect("sidebar surface should be valid");
        sidebar.pass_name = alloc::string::String::from("desktop-sidebar");
        sidebar.content = self.sidebar.render_scaled(width, height);
        self.push_desktop_scene_surface(
            &mut graph,
            &mut scene_entries,
            root,
            "desktop-sidebar",
            24.0,
            sidebar,
        );

        for (window_index, surface) in self
            .window_manager
            .surface_list_scaled(width, height)
            .into_iter()
            .enumerate()
        {
            self.push_desktop_scene_surface(
                &mut graph,
                &mut scene_entries,
                root,
                "desktop-window",
                60.0 + window_index as f32,
                surface,
            );
        }

        let taskbar_h = (height / 11).max(78);
        let mut taskbar = Surface::new(
            100,
            SurfaceRole::Panel,
            SurfaceRect {
                x: 0,
                y: height.saturating_sub(taskbar_h),
                width,
                height: taskbar_h,
            },
        )
        .expect("taskbar surface should be valid");
        taskbar.pass_name = alloc::string::String::from("desktop-taskbar");
        taskbar.content = self.taskbar.render_scaled(width, height);
        self.push_desktop_scene_surface(
            &mut graph,
            &mut scene_entries,
            root,
            "desktop-taskbar",
            26.0,
            taskbar,
        );

        if self.start_menu.is_visible() {
            let mut start_menu = Surface::new(
                101,
                SurfaceRole::Overlay,
                SurfaceRect {
                    x: 0,
                    y: height.saturating_sub(taskbar_h + height / 3),
                    width: (width / 3).max(360),
                    height: (height / 3).max(320),
                },
            )
            .expect("start menu surface should be valid");
            start_menu.pass_name = alloc::string::String::from("desktop-start-menu");
            start_menu.content = self.start_menu.render_scaled(width, height);
            self.push_desktop_scene_surface(
                &mut graph,
                &mut scene_entries,
                root,
                "desktop-start-menu",
                100.0,
                start_menu,
            );
        }

        if self.widgets_panel_visible {
            let mut widgets = Surface::new(
                102,
                SurfaceRole::Overlay,
                SurfaceRect {
                    x: 0,
                    y: 0,
                    width: (width / 3).max(320),
                    height: (height / 2).max(320),
                },
            )
            .expect("widgets surface should be valid");
            widgets.pass_name = alloc::string::String::from("desktop-widgets");
            widgets.content = self.render_widgets_panel();
            self.push_desktop_scene_surface(
                &mut graph,
                &mut scene_entries,
                root,
                "desktop-widgets",
                110.0,
                widgets,
            );
        }

        if self.notification_center_visible {
            let mut notifications = Surface::new(
                103,
                SurfaceRole::Overlay,
                SurfaceRect {
                    x: width.saturating_sub((width / 4).max(300)),
                    y: top_bar_h,
                    width: (width / 4).max(300),
                    height: (height / 2).max(320),
                },
            )
            .expect("notification center surface should be valid");
            notifications.pass_name = alloc::string::String::from("desktop-notifications");
            notifications.content = self.render_notification_center();
            self.push_desktop_scene_surface(
                &mut graph,
                &mut scene_entries,
                root,
                "desktop-notifications",
                112.0,
                notifications,
            );
        }

        if self.control_center_visible {
            let mut control_center = Surface::new(
                104,
                SurfaceRole::Overlay,
                SurfaceRect {
                    x: width.saturating_sub((width / 4).max(320)),
                    y: top_bar_h,
                    width: (width / 4).max(320),
                    height: (height / 2).max(340),
                },
            )
            .expect("control center surface should be valid");
            control_center.pass_name = alloc::string::String::from("desktop-control-center");
            control_center.content = self.render_control_center();
            self.push_desktop_scene_surface(
                &mut graph,
                &mut scene_entries,
                root,
                "desktop-control-center",
                114.0,
                control_center,
            );
        }

        (graph, camera_node, scene_entries)
    }

    fn push_desktop_scene_surface(
        &self,
        graph: &mut SceneGraph,
        scene_entries: &mut Vec<(u32, Surface)>,
        parent: u32,
        label: &str,
        depth: f32,
        surface: Surface,
    ) {
        let node_id = graph.add_labeled(
            Transform {
                translation: Vec3::new(surface.rect.x as f32, surface.rect.y as f32, depth),
                ..Transform::IDENTITY
            },
            label,
        );
        let _ = graph.set_parent(node_id, parent);
        scene_entries.push((node_id, surface));
    }

    fn resolve_desktop_scene_surfaces(
        &self,
        graph: &SceneGraph,
        camera_node: u32,
        scene_entries: Vec<(u32, Surface)>,
        width: u32,
        height: u32,
    ) -> Vec<Surface> {
        let camera = Camera::Orthographic(OrthographicCamera {
            left: 0.0,
            right: width.max(1) as f32,
            bottom: height.max(1) as f32,
            top: 0.0,
            near: 0.1,
            far: 1024.0,
        });
        if let Ok(submission) = submit_scene(graph, &camera, camera_node) {
            let mut ordered = Vec::new();
            for node in submission.nodes {
                if let Some((_, surface)) = scene_entries.iter().find(|(id, _)| *id == node.id) {
                    ordered.push(surface.clone());
                }
            }
            if ordered.len() == scene_entries.len() {
                return ordered;
            }
        }
        scene_entries
            .into_iter()
            .map(|(_, surface)| surface)
            .collect()
    }

    pub fn render_snap_overlay(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        self.render_snap_overlay_panel()
    }

    fn render_notification_center(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let width = (((380.0 * scale) + 0.5) as u32).max(300);
        let panel_h = (((500.0 * scale) + 0.5) as u32).max(380);
        let margin = (((20.0 * scale) + 0.5) as u32).max(16);
        let x = self.width.saturating_sub(width + margin);
        let y = margin;
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width,
            height: panel_h,
            radius: (((20.0 * scale) + 0.5) as u32).max(14),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        ops.push(DrawOp::Text {
            text: "Notifications".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((18.0 * scale) + 0.5) as u32).max(14),
            size: (((16.0 * scale) + 0.5) as u32).max(13),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((56.0 * scale) + 0.5) as u32).max(46),
            width: width.saturating_sub((((40.0 * scale) + 0.5) as u32).max(32)),
            height: (((34.0 * scale) + 0.5) as u32).max(28),
            radius: (((8.0 * scale) + 0.5) as u32).max(6),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        let items = [
            (
                "System Update Available",
                "GlassOS v2.5.0 is ready to install",
            ),
            ("New Message", "John sent you a file: Report.pdf"),
            (
                "Security Scan Complete",
                "No threats detected. System is secure.",
            ),
        ];
        for (i, (title, body)) in items.iter().enumerate() {
            let top = y
                + (((108.0 * scale) + 0.5) as u32).max(88)
                + (i as u32 * ((((110.0 * scale) + 0.5) as u32).max(88)));
            ops.push(DrawOp::RoundedRect {
                x: x + (((20.0 * scale) + 0.5) as u32).max(16),
                y: top,
                width: width.saturating_sub((((40.0 * scale) + 0.5) as u32).max(32)),
                height: (((94.0 * scale) + 0.5) as u32).max(72),
                radius: (((14.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
            ops.push(DrawOp::Text {
                text: (*title).into(),
                x: x + (((86.0 * scale) + 0.5) as u32).max(64),
                y: top + (((16.0 * scale) + 0.5) as u32).max(12),
                size: (((14.0 * scale) + 0.5) as u32).max(11),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
            ops.push(DrawOp::Text {
                text: (*body).into(),
                x: x + (((86.0 * scale) + 0.5) as u32).max(64),
                y: top + (((38.0 * scale) + 0.5) as u32).max(28),
                size: (((12.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0xc8,
                    g: 0xd2,
                    b: 0xe1,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops
    }

    fn render_control_center(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let width = (((340.0 * scale) + 0.5) as u32).max(260);
        let panel_h = (((500.0 * scale) + 0.5) as u32).max(380);
        let margin = (((20.0 * scale) + 0.5) as u32).max(16);
        let x = self.width.saturating_sub(
            width
                + margin
                + (((20.0 * scale) + 0.5) as u32).max(10)
                + (((380.0 * scale) + 0.5) as u32).max(300),
        );
        let y = margin;
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width,
            height: panel_h,
            radius: (((20.0 * scale) + 0.5) as u32).max(14),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        ops.push(DrawOp::Text {
            text: "Quick Settings".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((18.0 * scale) + 0.5) as u32).max(14),
            size: (((16.0 * scale) + 0.5) as u32).max(13),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        let labels = [
            "WiFi",
            "Bluetooth",
            "Dark Mode",
            "Location",
            "Airplane",
            "Hotspot",
        ];
        for (i, label) in labels.iter().enumerate() {
            let col = (i % 3) as u32;
            let row = (i / 3) as u32;
            let bx = x
                + (((20.0 * scale) + 0.5) as u32).max(16)
                + col * ((((100.0 * scale) + 0.5) as u32).max(84));
            let by = y
                + (((60.0 * scale) + 0.5) as u32).max(48)
                + row * ((((88.0 * scale) + 0.5) as u32).max(72));
            ops.push(DrawOp::RoundedRect {
                x: bx,
                y: by,
                width: (((84.0 * scale) + 0.5) as u32).max(64),
                height: (((70.0 * scale) + 0.5) as u32).max(56),
                radius: (((14.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
            ops.push(DrawOp::Text {
                text: (*label).into(),
                x: bx + (((12.0 * scale) + 0.5) as u32).max(10),
                y: by + (((26.0 * scale) + 0.5) as u32).max(20),
                size: (((11.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops
    }

    fn render_widgets_panel(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let x = (((20.0 * scale) + 0.5) as u32).max(16);
        let y = (((20.0 * scale) + 0.5) as u32).max(16);
        let panel_w = (((350.0 * scale) + 0.5) as u32).max(280);
        let panel_h = self
            .height
            .saturating_sub(y + (((140.0 * scale) + 0.5) as u32).max(100));
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width: panel_w,
            height: panel_h,
            radius: (((20.0 * scale) + 0.5) as u32).max(14),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        ops.push(DrawOp::Text {
            text: "Weather".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((20.0 * scale) + 0.5) as u32).max(16),
            size: (((16.0 * scale) + 0.5) as u32).max(13),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "22° Partly Cloudy".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((56.0 * scale) + 0.5) as u32).max(44),
            size: (((30.0 * scale) + 0.5) as u32).max(20),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Bucharest, Romania".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((92.0 * scale) + 0.5) as u32).max(72),
            size: (((12.0 * scale) + 0.5) as u32).max(10),
            color: RgbaColor {
                r: 0xc8,
                g: 0xd2,
                b: 0xe1,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((132.0 * scale) + 0.5) as u32).max(104),
            width: panel_w.saturating_sub((((40.0 * scale) + 0.5) as u32).max(32)),
            height: (((160.0 * scale) + 0.5) as u32).max(120),
            radius: (((16.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        ops.push(DrawOp::Text {
            text: "Calendar".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((306.0 * scale) + 0.5) as u32).max(240),
            size: (((16.0 * scale) + 0.5) as u32).max(13),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((344.0 * scale) + 0.5) as u32).max(272),
            width: panel_w.saturating_sub((((40.0 * scale) + 0.5) as u32).max(32)),
            height: (((180.0 * scale) + 0.5) as u32).max(132),
            radius: (((16.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        ops.push(DrawOp::Text {
            text: "System Stats".into(),
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((540.0 * scale) + 0.5) as u32).max(420),
            size: (((16.0 * scale) + 0.5) as u32).max(13),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((20.0 * scale) + 0.5) as u32).max(16),
            y: y + (((578.0 * scale) + 0.5) as u32).max(448),
            width: panel_w.saturating_sub((((40.0 * scale) + 0.5) as u32).max(32)),
            height: (((160.0 * scale) + 0.5) as u32).max(120),
            radius: (((16.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        ops
    }

    fn render_settings_panel(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let width = (((800.0 * scale) + 0.5) as u32).max(620);
        let height = (((600.0 * scale) + 0.5) as u32).max(480);
        let x = (self.width.saturating_sub(width)) / 2;
        let y = (self.height.saturating_sub(height)) / 2;
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width,
            height,
            radius: (((20.0 * scale) + 0.5) as u32).max(14),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width: (((220.0 * scale) + 0.5) as u32).max(170),
            height,
            radius: (((20.0 * scale) + 0.5) as u32).max(14),
            color: RgbaColor {
                r: 0x13,
                g: 0x1b,
                b: 0x28,
                a: 0xee,
            },
        });
        let items = ["Themes", "Wallpaper", "Display", "System"];
        for (i, item) in items.iter().enumerate() {
            let top = y
                + (((80.0 * scale) + 0.5) as u32).max(64)
                + (i as u32 * ((((56.0 * scale) + 0.5) as u32).max(42)));
            ops.push(DrawOp::RoundedRect {
                x: x + (((14.0 * scale) + 0.5) as u32).max(12),
                y: top,
                width: (((192.0 * scale) + 0.5) as u32).max(150),
                height: (((42.0 * scale) + 0.5) as u32).max(34),
                radius: (((12.0 * scale) + 0.5) as u32).max(10),
                color: if i == 0 {
                    RgbaColor {
                        r: 0x00,
                        g: 0xd4,
                        b: 0xff,
                        a: 0x30,
                    }
                } else {
                    RgbaColor {
                        r: 0x00,
                        g: 0x00,
                        b: 0x00,
                        a: 0x00,
                    }
                },
            });
            ops.push(DrawOp::Text {
                text: (*item).into(),
                x: x + (((36.0 * scale) + 0.5) as u32).max(28),
                y: top + (((12.0 * scale) + 0.5) as u32).max(10),
                size: (((13.0 * scale) + 0.5) as u32).max(11),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops.push(DrawOp::Text {
            text: "Choose Your Theme".into(),
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((36.0 * scale) + 0.5) as u32).max(28),
            size: (((24.0 * scale) + 0.5) as u32).max(18),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Customize the appearance of your desktop".into(),
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((68.0 * scale) + 0.5) as u32).max(54),
            size: (((13.0 * scale) + 0.5) as u32).max(11),
            color: RgbaColor {
                r: 0xc8,
                g: 0xd2,
                b: 0xe1,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Color Theme".into(),
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((110.0 * scale) + 0.5) as u32).max(88),
            size: (((14.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((126.0 * scale) + 0.5) as u32).max(100),
            width: (((500.0 * scale) + 0.5) as u32).max(380),
            height: (((90.0 * scale) + 0.5) as u32).max(72),
            radius: (((16.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        ops.push(DrawOp::Text {
            text: "Color Theme".into(),
            x: x + (((270.0 * scale) + 0.5) as u32).max(216),
            y: y + (((150.0 * scale) + 0.5) as u32).max(120),
            size: (((14.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        let theme_labels = ["Dark", "Light", "Purple", "Green"];
        for (i, label) in theme_labels.iter().enumerate() {
            let tx = x
                + (((270.0 * scale) + 0.5) as u32).max(216)
                + (i as u32 * ((((118.0 * scale) + 0.5) as u32).max(92)));
            ops.push(DrawOp::RoundedRect {
                x: tx,
                y: y + (((180.0 * scale) + 0.5) as u32).max(144),
                width: (((100.0 * scale) + 0.5) as u32).max(78),
                height: (((88.0 * scale) + 0.5) as u32).max(70),
                radius: (((14.0 * scale) + 0.5) as u32).max(10),
                color: if i == 0 {
                    RgbaColor {
                        r: 0x00,
                        g: 0xd4,
                        b: 0xff,
                        a: 0x30,
                    }
                } else {
                    RgbaColor {
                        r: 0x30,
                        g: 0x30,
                        b: 0x50,
                        a: 0x50,
                    }
                },
            });
            ops.push(DrawOp::Text {
                text: (*label).into(),
                x: tx + (((24.0 * scale) + 0.5) as u32).max(18),
                y: y + (((232.0 * scale) + 0.5) as u32).max(184),
                size: (((12.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops.push(DrawOp::Text {
            text: "Wallpaper".into(),
            x: x + (((270.0 * scale) + 0.5) as u32).max(216),
            y: y + (((232.0 * scale) + 0.5) as u32).max(184),
            size: (((14.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        for i in 0..6 {
            let wx = x
                + (((270.0 * scale) + 0.5) as u32).max(216)
                + ((i % 3) as u32 * (((160.0 * scale) + 0.5) as u32).max(124));
            let wy = y
                + (((260.0 * scale) + 0.5) as u32).max(208)
                + ((i / 3) as u32 * (((110.0 * scale) + 0.5) as u32).max(88));
            ops.push(DrawOp::RoundedRect {
                x: wx,
                y: wy,
                width: (((140.0 * scale) + 0.5) as u32).max(110),
                height: (((90.0 * scale) + 0.5) as u32).max(72),
                radius: (((12.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
        }
        ops.push(DrawOp::Text {
            text: "Display".into(),
            x: x + (((270.0 * scale) + 0.5) as u32).max(216),
            y: y + (((480.0 * scale) + 0.5) as u32).max(384),
            size: (((14.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        let toggles = [
            "Dark Mode",
            "Full Screen",
            "Night Light",
            "Notifications",
            "Security Scan",
            "Auto Update",
        ];
        for (i, toggle) in toggles.iter().enumerate() {
            let ty = y
                + (((510.0 * scale) + 0.5) as u32).max(408)
                + (i as u32 * ((((22.0 * scale) + 0.5) as u32).max(18)));
            ops.push(DrawOp::Text {
                text: (*toggle).into(),
                x: x + (((270.0 * scale) + 0.5) as u32).max(216),
                y: ty,
                size: (((11.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0xc8,
                    g: 0xd2,
                    b: 0xe1,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops.push(DrawOp::Text {
            text: "System Settings".into(),
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((430.0 * scale) + 0.5) as u32).max(344),
            size: (((14.0 * scale) + 0.5) as u32).max(12),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: FontFamily::SansSerif,
        });
        ops.push(DrawOp::RoundedRect {
            x: x + (((250.0 * scale) + 0.5) as u32).max(200),
            y: y + (((446.0 * scale) + 0.5) as u32).max(356),
            width: (((500.0 * scale) + 0.5) as u32).max(380),
            height: (((44.0 * scale) + 0.5) as u32).max(34),
            radius: (((12.0 * scale) + 0.5) as u32).max(10),
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x50,
            },
        });
        ops
    }

    fn render_context_menu(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let x = (((60.0 * scale) + 0.5) as u32).max(48);
        let y = (((110.0 * scale) + 0.5) as u32).max(88);
        let width = (((240.0 * scale) + 0.5) as u32).max(200);
        let height = (((260.0 * scale) + 0.5) as u32).max(210);
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width,
            height,
            radius: (((14.0 * scale) + 0.5) as u32).max(10),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        let items = [
            "View",
            "Sort by",
            "Refresh",
            "New",
            "Widgets",
            "Display Settings",
            "Personalize",
        ];
        for (i, item) in items.iter().enumerate() {
            ops.push(DrawOp::Text {
                text: (*item).into(),
                x: x + (((18.0 * scale) + 0.5) as u32).max(14),
                y: y + (((22.0 * scale) + 0.5) as u32).max(18)
                    + (i as u32 * ((((28.0 * scale) + 0.5) as u32).max(22))),
                size: (((12.0 * scale) + 0.5) as u32).max(10),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        ops.push(DrawOp::Rect {
            x: x + (((10.0 * scale) + 0.5) as u32).max(8),
            y: y + (((36.0 * scale) + 0.5) as u32).max(28),
            width: width.saturating_sub((((20.0 * scale) + 0.5) as u32).max(16)),
            height: 1,
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0xff,
            },
        });
        ops.push(DrawOp::Rect {
            x: x + (((10.0 * scale) + 0.5) as u32).max(8),
            y: y + (((148.0 * scale) + 0.5) as u32).max(118),
            width: width.saturating_sub((((20.0 * scale) + 0.5) as u32).max(16)),
            height: 1,
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0xff,
            },
        });
        ops.push(DrawOp::RoundedRect {
            x: x + width.saturating_sub((((30.0 * scale) + 0.5) as u32).max(24)),
            y: y + (((94.0 * scale) + 0.5) as u32).max(74),
            width: (((20.0 * scale) + 0.5) as u32).max(16),
            height: (((20.0 * scale) + 0.5) as u32).max(16),
            radius: (((10.0 * scale) + 0.5) as u32).max(8),
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x80,
            },
        });
        ops
    }

    fn render_snap_overlay_panel(&self) -> alloc::vec::Vec<ngos_gfx_translate::DrawOp> {
        use ngos_gfx_translate::{DrawOp, RgbaColor};
        let mut ops = alloc::vec::Vec::new();
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let width = (((260.0 * scale) + 0.5) as u32).max(200);
        let height = (((82.0 * scale) + 0.5) as u32).max(64);
        let x = self.width.saturating_sub(width) / 2;
        ops.push(DrawOp::RoundedRect {
            x,
            y: (((20.0 * scale) + 0.5) as u32).max(16),
            width,
            height,
            radius: (((14.0 * scale) + 0.5) as u32).max(10),
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        let bx0 = x + (((18.0 * scale) + 0.5) as u32).max(14);
        for i in 0..3 {
            let bx = bx0 + (i * ((((68.0 * scale) + 0.5) as u32).max(52)));
            ops.push(DrawOp::RoundedRect {
                x: bx,
                y: (((40.0 * scale) + 0.5) as u32).max(32),
                width: (((56.0 * scale) + 0.5) as u32).max(44),
                height: (((38.0 * scale) + 0.5) as u32).max(30),
                radius: (((8.0 * scale) + 0.5) as u32).max(6),
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
        }
        ops
    }
}

/// Taskbar actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarAction {
    OpenStart,
    OpenApp(u8),
}
