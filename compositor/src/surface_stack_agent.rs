use crate::surface_agent::{Surface, SurfaceError, SurfaceId, SurfaceRole};
use alloc::vec::Vec;

pub struct SurfaceStack {
    surfaces: Vec<Surface>,
}

pub struct StackInspect {
    pub surface_count: usize,
    pub background_count: usize,
    pub panel_count: usize,
    pub window_count: usize,
    pub overlay_count: usize,
    pub cursor_count: usize,
    pub focused_id: Option<SurfaceId>,
}

impl SurfaceStack {
    pub fn new() -> Self {
        SurfaceStack {
            surfaces: Vec::new(),
        }
    }

    pub fn push(&mut self, surface: Surface) -> Result<(), SurfaceError> {
        if self.surfaces.iter().any(|s| s.id == surface.id) {
            return Err(SurfaceError::DuplicateId { id: surface.id });
        }
        self.surfaces.push(surface);
        self.surfaces.sort_by_key(|s| s.role as u8);
        Ok(())
    }

    pub fn remove(&mut self, id: SurfaceId) -> Result<(), SurfaceError> {
        let pos = self
            .surfaces
            .iter()
            .position(|s| s.id == id)
            .ok_or(SurfaceError::NotFound { id })?;
        self.surfaces.remove(pos);
        Ok(())
    }

    pub fn get(&self, id: SurfaceId) -> Option<&Surface> {
        self.surfaces.iter().find(|s| s.id == id)
    }

    pub fn ordered(&self) -> &[Surface] {
        &self.surfaces
    }

    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    pub fn inspect(&self) -> StackInspect {
        let mut background_count = 0;
        let mut panel_count = 0;
        let mut window_count = 0;
        let mut overlay_count = 0;
        let mut cursor_count = 0;
        let mut focused_id = None;
        for s in &self.surfaces {
            if s.focused {
                focused_id = Some(s.id);
            }
            match s.role {
                SurfaceRole::Background => background_count += 1,
                SurfaceRole::Panel => panel_count += 1,
                SurfaceRole::Window => window_count += 1,
                SurfaceRole::Overlay => overlay_count += 1,
                SurfaceRole::Cursor => cursor_count += 1,
            }
        }
        StackInspect {
            surface_count: self.surfaces.len(),
            background_count,
            panel_count,
            window_count,
            overlay_count,
            cursor_count,
            focused_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_agent::{Surface, SurfaceRect, SurfaceRole};

    fn rect(w: u32, h: u32) -> SurfaceRect {
        SurfaceRect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    }

    fn window(id: SurfaceId) -> Surface {
        Surface::new(id, SurfaceRole::Window, rect(400, 300)).unwrap()
    }

    fn bg(id: SurfaceId) -> Surface {
        Surface::new(id, SurfaceRole::Background, rect(1920, 1080)).unwrap()
    }

    fn overlay(id: SurfaceId) -> Surface {
        Surface::new(id, SurfaceRole::Overlay, rect(200, 100)).unwrap()
    }

    #[test]
    fn push_maintains_z_order() {
        let mut stack = SurfaceStack::new();
        stack.push(window(2)).unwrap();
        stack.push(bg(1)).unwrap();
        stack.push(overlay(3)).unwrap();
        let ordered = stack.ordered();
        assert_eq!(ordered[0].role, SurfaceRole::Background);
        assert_eq!(ordered[1].role, SurfaceRole::Window);
        assert_eq!(ordered[2].role, SurfaceRole::Overlay);
    }

    #[test]
    fn push_rejects_duplicate_id() {
        let mut stack = SurfaceStack::new();
        stack.push(window(1)).unwrap();
        let err = stack.push(window(1)).unwrap_err();
        assert!(matches!(err, SurfaceError::DuplicateId { id: 1 }));
        assert!(err.describe().contains("already exists"));
    }

    #[test]
    fn remove_existing_surface() {
        let mut stack = SurfaceStack::new();
        stack.push(window(1)).unwrap();
        stack.push(bg(2)).unwrap();
        stack.remove(1).unwrap();
        assert_eq!(stack.ordered().len(), 1);
        assert_eq!(stack.ordered()[0].id, 2);
    }

    #[test]
    fn remove_nonexistent_surface() {
        let mut stack = SurfaceStack::new();
        let err = stack.remove(99).unwrap_err();
        assert!(matches!(err, SurfaceError::NotFound { id: 99 }));
        assert!(err.describe().contains("not found"));
    }

    #[test]
    fn inspect_counts_by_role() {
        let mut stack = SurfaceStack::new();
        stack.push(bg(1)).unwrap();
        stack.push(window(2)).unwrap();
        stack.push(window(3)).unwrap();
        stack.push(overlay(4)).unwrap();
        let insp = stack.inspect();
        assert_eq!(insp.surface_count, 4);
        assert_eq!(insp.background_count, 1);
        assert_eq!(insp.window_count, 2);
        assert_eq!(insp.overlay_count, 1);
        assert_eq!(insp.panel_count, 0);
        assert_eq!(insp.focused_id, None);
    }

    #[test]
    fn inspect_reports_focused() {
        let mut stack = SurfaceStack::new();
        let mut w = window(5);
        w.focused = true;
        stack.push(w).unwrap();
        let insp = stack.inspect();
        assert_eq!(insp.focused_id, Some(5));
    }
}
