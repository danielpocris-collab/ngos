//! NGOS Browser UI Shell
//!
//! Semantic orchestration surface over the FrameScript browser renderer.

pub use browser_core::{BrowserError, BrowserResult};
use browser_layout::{LayoutNode, LayoutTree, Rect};
use browser_paint::{FrameScriptRenderer, Renderer};
use ngos_gfx_translate::{DrawOp, EncodedFrame, FrameScript, RgbaColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserViewport {
    pub width: u32,
    pub height: u32,
}

impl BrowserViewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserSceneProfile {
    pub translucency: u8,
    pub pulse_stride: u8,
}

impl BrowserSceneProfile {
    pub fn premium_glass() -> Self {
        Self {
            translucency: 0x8E,
            pulse_stride: 3,
        }
    }

    pub fn encode_label(&self) -> String {
        format!(
            "browser-glass/translucency-{:02x}/pulse-{}",
            self.translucency, self.pulse_stride
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserFrame {
    pub viewport: BrowserViewport,
    pub profile: BrowserSceneProfile,
    pub scene: BrowserScene,
    pub script_text: String,
    pub script: FrameScript,
    pub encoded: EncodedFrame,
}

impl BrowserFrame {
    pub fn script_text(&self) -> &str {
        &self.script_text
    }
}

pub struct BrowserUiSurface {
    viewport: BrowserViewport,
    profile: BrowserSceneProfile,
    renderer: FrameScriptRenderer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserLayerRole {
    Root,
    Workspace,
    Panel,
    Overlay,
    Accent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSceneNode {
    pub role: BrowserLayerRole,
    pub depth: usize,
    pub focused: bool,
    pub rect: SceneRect,
    pub children: Vec<BrowserSceneNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserScene {
    pub viewport: BrowserViewport,
    pub nodes: Vec<BrowserSceneNode>,
}

impl BrowserUiSurface {
    pub fn new(viewport: BrowserViewport) -> Self {
        Self::with_profile(viewport, BrowserSceneProfile::premium_glass())
    }

    pub fn with_profile(viewport: BrowserViewport, profile: BrowserSceneProfile) -> Self {
        Self {
            viewport,
            profile,
            renderer: FrameScriptRenderer::new(viewport.width, viewport.height),
        }
    }

    pub fn viewport(&self) -> BrowserViewport {
        self.viewport
    }

    pub fn profile(&self) -> BrowserSceneProfile {
        self.profile
    }

    pub fn render_layout(&mut self, tree: &LayoutTree) -> BrowserResult<BrowserFrame> {
        self.renderer.render(tree)?;
        let text = self.renderer.get_output();
        let mut script = FrameScript::parse(text)
            .map_err(|error| BrowserError::Render(format!("invalid framescript: {error:?}")))?;
        let scene = BrowserScene::from_layout_tree(tree, self.viewport);
        apply_scene_overlays(&mut script, &scene, self.profile);
        let profile = self.profile.encode_label();
        let encoded = script.encode(&profile);
        Ok(BrowserFrame {
            viewport: self.viewport,
            profile: self.profile,
            scene,
            script_text: text.to_string(),
            script,
            encoded,
        })
    }
}

impl BrowserScene {
    pub fn from_layout_tree(tree: &LayoutTree, viewport: BrowserViewport) -> Self {
        let nodes = tree
            .root
            .as_ref()
            .map(|root| vec![scene_node_from_layout(root, 0, true)])
            .unwrap_or_default();
        Self { viewport, nodes }
    }
}

fn scene_node_from_layout(node: &LayoutNode, depth: usize, focused: bool) -> BrowserSceneNode {
    let role = classify_role(node, depth);
    let children = node
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| scene_node_from_layout(child, depth + 1, focused && index == 0))
        .collect();
    BrowserSceneNode {
        role,
        depth,
        focused,
        rect: scene_rect(node.rect),
        children,
    }
}

fn classify_role(node: &LayoutNode, depth: usize) -> BrowserLayerRole {
    let name = node.node.borrow().name.to_ascii_lowercase();
    if depth == 0 {
        BrowserLayerRole::Root
    } else if matches!(name.as_str(), "header" | "nav" | "aside" | "footer") {
        BrowserLayerRole::Panel
    } else if matches!(
        name.as_str(),
        "main" | "section" | "article" | "body" | "html"
    ) {
        BrowserLayerRole::Workspace
    } else if node.children.is_empty() {
        BrowserLayerRole::Accent
    } else {
        BrowserLayerRole::Overlay
    }
}

fn scene_rect(rect: Rect) -> SceneRect {
    SceneRect {
        x: rect.x.max(0.0).round() as u32,
        y: rect.y.max(0.0).round() as u32,
        width: rect.width.max(0.0).round() as u32,
        height: rect.height.max(0.0).round() as u32,
    }
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
}

fn apply_scene_overlays(
    script: &mut FrameScript,
    scene: &BrowserScene,
    profile: BrowserSceneProfile,
) {
    for node in &scene.nodes {
        append_scene_node_ops(&mut script.ops, node, profile);
    }
}

fn append_scene_node_ops(
    ops: &mut Vec<DrawOp>,
    node: &BrowserSceneNode,
    profile: BrowserSceneProfile,
) {
    let rect = node.rect;
    let inset = 6 + (node.depth as u32).min(8) * 3;
    let accent = match node.role {
        BrowserLayerRole::Root => rgba(0x92, 0xD9, 0xFF, 0x40),
        BrowserLayerRole::Workspace => rgba(0x76, 0xD8, 0xFF, 0x58),
        BrowserLayerRole::Panel => rgba(0x7C, 0xFF, 0xD5, 0x54),
        BrowserLayerRole::Overlay => rgba(0xFF, 0xBE, 0x72, 0x50),
        BrowserLayerRole::Accent => rgba(0xE5, 0xF2, 0xFF, 0x46),
    };
    let glow_alpha = profile
        .translucency
        .saturating_sub((node.depth as u8).saturating_mul(10))
        .max(0x2A);

    if rect.width > inset * 2 + 8 && rect.height > inset * 2 + 8 {
        ops.push(DrawOp::RoundedRect {
            x: rect.x + inset,
            y: rect.y + inset,
            width: rect.width - inset * 2,
            height: rect.height.saturating_sub(inset * 2),
            radius: 10u32.saturating_sub((node.depth as u32).min(4)).max(6),
            color: rgba(0xFA, 0xFD, 0xFF, 0x0C),
        });
        ops.push(DrawOp::Line {
            x0: rect.x + inset,
            y0: rect.y + inset,
            x1: rect.x + rect.width.saturating_sub(inset),
            y1: rect.y + inset,
            color: accent,
        });
        ops.push(DrawOp::Rect {
            x: rect.x + inset,
            y: rect.y + rect.height.saturating_sub(inset + 3),
            width: (rect.width / 3).max(12),
            height: 3,
            color: rgba(accent.r, accent.g, accent.b, glow_alpha),
        });
    }

    if node.focused && rect.width > 24 && rect.height > 24 {
        ops.push(DrawOp::ShadowRect {
            x: rect.x.saturating_sub(4),
            y: rect.y.saturating_sub(4),
            width: rect.width + 8,
            height: rect.height + 8,
            blur: 18,
            color: rgba(accent.r, accent.g, accent.b, 0x22),
        });
    }

    for child in &node.children {
        append_scene_node_ops(ops, child, profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use browser_dom::{NodeData, NodeType};
    use browser_layout::{LayoutNode, Rect};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn test_tree() -> LayoutTree {
        LayoutTree {
            root: Some(LayoutNode {
                node: Rc::new(RefCell::new(NodeData::new(NodeType::Element, "main"))),
                rect: Rect::new(32.0, 24.0, 640.0, 420.0),
                children: vec![LayoutNode {
                    node: Rc::new(RefCell::new(NodeData::new(NodeType::Element, "section"))),
                    rect: Rect::new(64.0, 80.0, 280.0, 180.0),
                    children: Vec::new(),
                }],
            }),
        }
    }

    #[test]
    fn render_layout_produces_valid_encoded_frame() {
        let mut surface = BrowserUiSurface::new(BrowserViewport::new(1280, 720));
        let frame = surface
            .render_layout(&test_tree())
            .expect("browser UI frame should render");

        assert_eq!(frame.viewport.width, 1280);
        assert_eq!(frame.script.width, 1280);
        assert_eq!(frame.script.height, 720);
        assert!(!frame.scene.nodes.is_empty());
        assert!(frame.script_text.contains("gradient-rect="));
        assert!(frame.encoded.payload.contains("op=gradient-rect"));
        assert!(frame.encoded.payload.contains("op=rounded-rect"));
        assert!(frame.encoded.payload.contains("op=shadow-rect"));
        assert!(frame.encoded.payload.contains("op=line"));
    }

    #[test]
    fn scene_graph_classifies_layout_roles() {
        let scene = BrowserScene::from_layout_tree(&test_tree(), BrowserViewport::new(1280, 720));
        let root = scene.nodes.first().expect("scene root should exist");
        assert_eq!(root.role, BrowserLayerRole::Root);
        assert!(root.focused);
        assert!(root.children.iter().any(|child| {
            matches!(
                child.role,
                BrowserLayerRole::Workspace
                    | BrowserLayerRole::Panel
                    | BrowserLayerRole::Overlay
                    | BrowserLayerRole::Accent
            )
        }));
    }
}
