use crate::composition_agent::compose_surface;
use crate::surface_agent::{Surface, SurfaceError, SurfaceId};
use crate::surface_stack_agent::{StackInspect, SurfaceStack};
use alloc::string::ToString;
use alloc::{format, string::String, vec, vec::Vec};
use ngos_gfx_translate::{DrawOp, FrameProfile, FrameScript, RenderPassClass, RgbaColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositorError {
    EmptyStack,
    Surface(SurfaceError),
    InvalidFrameContract(String),
}

impl CompositorError {
    pub fn describe(&self) -> String {
        match self {
            Self::EmptyStack => String::from("compositor stack is empty — nothing to compose"),
            Self::Surface(e) => format!("surface error: {}", e.describe()),
            Self::InvalidFrameContract(s) => format!("invalid frame contract: {}", s),
        }
    }
}

impl From<SurfaceError> for CompositorError {
    fn from(e: SurfaceError) -> Self {
        CompositorError::Surface(e)
    }
}

pub struct CompositorInspect {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub stack: StackInspect,
    pub estimated_profile: FrameProfile,
}

pub struct Compositor {
    pub stack: SurfaceStack,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Compositor {
    pub fn new(width: u32, height: u32) -> Self {
        Compositor {
            stack: SurfaceStack::new(),
            viewport_width: width,
            viewport_height: height,
        }
    }

    pub fn push_surface(&mut self, surface: Surface) -> Result<(), CompositorError> {
        self.stack.push(surface).map_err(CompositorError::Surface)
    }

    pub fn remove_surface(&mut self, id: SurfaceId) -> Result<(), CompositorError> {
        self.stack.remove(id).map_err(CompositorError::Surface)
    }

    pub fn compose(
        &self,
        frame_tag: &str,
        queue: &str,
        present_mode: &str,
        completion: &str,
    ) -> Result<FrameScript, CompositorError> {
        if self.stack.is_empty() {
            return Err(CompositorError::EmptyStack);
        }
        if frame_tag.is_empty() {
            return Err(CompositorError::InvalidFrameContract(String::from(
                "frame_tag is empty",
            )));
        }

        let script = FrameScript {
            width: self.viewport_width,
            height: self.viewport_height,
            frame_tag: frame_tag.to_string(),
            queue: queue.to_string(),
            present_mode: present_mode.to_string(),
            completion: completion.to_string(),
            ops: self.inspect_ops(),
        };
        script
            .validate()
            .map_err(|e| CompositorError::InvalidFrameContract(format!("{:?}", e)))?;
        Ok(script)
    }

    pub fn inspect(&self) -> CompositorInspect {
        let estimated_profile = FrameProfile::from_ops(&self.inspect_ops());
        CompositorInspect {
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
            stack: self.stack.inspect(),
            estimated_profile,
        }
    }

    fn inspect_ops(&self) -> Vec<DrawOp> {
        let mut ops: Vec<DrawOp> = vec![DrawOp::Clear {
            color: RgbaColor {
                r: 0x0b,
                g: 0x11,
                b: 0x1a,
                a: 0xff,
            },
        }];

        for surface in self.stack.ordered() {
            ops.extend(compose_surface(surface));
        }

        ops.push(DrawOp::BeginPass {
            label: String::from("frame-present"),
            class: RenderPassClass::Presentation,
        });
        ops.push(DrawOp::SetPresentRegion {
            x: 0,
            y: 0,
            width: self.viewport_width,
            height: self.viewport_height,
        });
        ops.push(DrawOp::FlipRegion {
            x: 0,
            y: 0,
            width: self.viewport_width,
            height: self.viewport_height,
        });
        ops.push(DrawOp::EndPass);
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_agent::{Surface, SurfaceRect, SurfaceRole};

    fn rect(x: u32, y: u32, w: u32, h: u32) -> SurfaceRect {
        SurfaceRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn bg() -> Surface {
        Surface::new(1, SurfaceRole::Background, rect(0, 0, 1280, 720)).unwrap()
    }

    fn window(id: u32) -> Surface {
        Surface::new(id, SurfaceRole::Window, rect(100, 80, 800, 500)).unwrap()
    }

    fn compose(c: &Compositor) -> FrameScript {
        c.compose("test-frame", "graphics", "fifo", "wait-present")
            .unwrap()
    }

    #[test]
    fn empty_stack_is_refused() {
        let c = Compositor::new(1280, 720);
        let err = c
            .compose("f", "graphics", "fifo", "wait-present")
            .unwrap_err();
        assert!(matches!(err, CompositorError::EmptyStack));
        assert!(err.describe().contains("empty"));
    }

    #[test]
    fn empty_frame_tag_is_refused() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        let err = c
            .compose("", "graphics", "fifo", "wait-present")
            .unwrap_err();
        assert!(matches!(err, CompositorError::InvalidFrameContract(_)));
    }

    #[test]
    fn single_surface_produces_valid_script() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        let script = compose(&c);
        assert_eq!(script.width, 1280);
        assert_eq!(script.height, 720);
        assert!(!script.ops.is_empty());
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
    }

    #[test]
    fn composition_uses_push_pop_layer_for_each_surface() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        c.push_surface(window(2)).unwrap();
        let script = compose(&c);
        let push_count = script
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::PushLayer { .. }))
            .count();
        let pop_count = script
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::PopLayer))
            .count();
        assert_eq!(push_count, 2, "one PushLayer per surface");
        assert_eq!(pop_count, 2, "one PopLayer per surface");
    }

    #[test]
    fn composition_uses_set_clip_and_clear_clip() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        let script = compose(&c);
        assert!(
            script
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::SetClip { .. }))
        );
        assert!(script.ops.iter().any(|op| matches!(op, DrawOp::ClearClip)));
    }

    #[test]
    fn window_surface_has_chrome_in_output() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        c.push_surface(window(2)).unwrap();
        let script = compose(&c);
        assert!(
            script
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::ShadowRect { .. }))
        );
        assert!(
            script
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::RoundedRect { .. }))
        );
    }

    #[test]
    fn surfaces_ordered_bg_before_window_before_overlay() {
        let mut c = Compositor::new(1280, 720);
        let overlay = Surface::new(3, SurfaceRole::Overlay, rect(200, 200, 300, 200)).unwrap();
        c.push_surface(window(2)).unwrap();
        c.push_surface(bg()).unwrap();
        c.push_surface(overlay).unwrap();
        let ordered = c.stack.ordered();
        assert_eq!(ordered[0].role, SurfaceRole::Background);
        assert_eq!(ordered[1].role, SurfaceRole::Window);
        assert_eq!(ordered[2].role, SurfaceRole::Overlay);
    }

    #[test]
    fn remove_surface_excluded_from_composition() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(bg()).unwrap();
        c.push_surface(window(2)).unwrap();
        c.remove_surface(2).unwrap();
        let script = compose(&c);
        let push_count = script
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::PushLayer { .. }))
            .count();
        assert_eq!(push_count, 1, "only background after window removed");
    }

    #[test]
    fn duplicate_surface_id_refused() {
        let mut c = Compositor::new(1280, 720);
        c.push_surface(window(5)).unwrap();
        let err = c.push_surface(window(5)).unwrap_err();
        assert!(matches!(
            err,
            CompositorError::Surface(SurfaceError::DuplicateId { id: 5 })
        ));
    }

    #[test]
    fn remove_nonexistent_surface_refused() {
        let mut c = Compositor::new(1280, 720);
        let err = c.remove_surface(99).unwrap_err();
        assert!(matches!(
            err,
            CompositorError::Surface(SurfaceError::NotFound { id: 99 })
        ));
    }

    #[test]
    fn inspect_reports_viewport_and_stack() {
        let mut c = Compositor::new(1920, 1080);
        c.push_surface(bg()).unwrap();
        c.push_surface(window(2)).unwrap();
        let insp = c.inspect();
        assert_eq!(insp.viewport_width, 1920);
        assert_eq!(insp.viewport_height, 1080);
        assert_eq!(insp.stack.surface_count, 2);
        assert_eq!(insp.stack.background_count, 1);
        assert_eq!(insp.stack.window_count, 1);
    }
}
