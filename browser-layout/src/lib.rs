//! NGOS Browser Layout Engine
//!
//! Block, inline, flexbox, grid layout - Proprietary

pub use browser_core::{BrowserError, BrowserResult};
pub use browser_css::{ComputedStyles, Stylesheet};
use browser_dom::NodeType;
pub use browser_dom::{Document, Node};

/// 2D size
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Layout context
pub struct LayoutContext {
    pub viewport: Size,
}

/// Layout tree
pub struct LayoutTree {
    pub root: Option<LayoutNode>,
}

/// Layout node
pub struct LayoutNode {
    pub node: Node,
    pub rect: Rect,
    pub children: Vec<LayoutNode>,
}

const ROOT_PADDING: f32 = 28.0;
const BLOCK_GAP: f32 = 14.0;
const INNER_PADDING: f32 = 18.0;
const TEXT_LINE_HEIGHT: f32 = 28.0;
const LEAF_HEIGHT: f32 = 72.0;

/// Build layout tree from DOM
pub fn build_layout_tree(
    doc: &Document,
    styles: &ComputedStyles,
    ctx: &LayoutContext,
) -> LayoutTree {
    let _ = styles;
    let Some(root) = doc.document_element.as_ref() else {
        return LayoutTree { root: None };
    };
    let viewport_width = ctx.viewport.width.max(64.0);
    let viewport_height = ctx.viewport.height.max(64.0);
    let root_rect = Rect::new(0.0, 0.0, viewport_width, viewport_height);
    let root_node = layout_node(root, root_rect, 0);
    LayoutTree {
        root: Some(root_node),
    }
}

/// Perform layout calculations
pub fn compute_layout(tree: &mut LayoutTree, ctx: &LayoutContext) {
    let Some(root) = tree.root.as_mut() else {
        return;
    };
    let rect = Rect::new(
        0.0,
        0.0,
        ctx.viewport.width.max(64.0),
        ctx.viewport.height.max(64.0),
    );
    *root = layout_node(&root.node.clone(), rect, 0);
}

fn layout_node(node: &Node, rect: Rect, depth: usize) -> LayoutNode {
    let node_ref = node.borrow();
    let available_width = rect.width.max(32.0);
    let mut cursor_y = rect.y + ROOT_PADDING.min(rect.height / 6.0);
    let content_x = rect.x + ROOT_PADDING.min(rect.width / 8.0);
    let content_width = (available_width - ROOT_PADDING * 2.0).max(24.0);
    let mut children = Vec::new();

    for child in &node_ref.children {
        let child_ref = child.borrow();
        let child_height = estimate_node_height(child, depth + 1);
        let child_indent = (depth as f32 * 14.0).min(content_width / 5.0);
        let child_rect = Rect::new(
            content_x + child_indent,
            cursor_y,
            (content_width - child_indent).max(24.0),
            child_height,
        );
        drop(child_ref);
        children.push(layout_node(child, child_rect, depth + 1));
        cursor_y += child_height + BLOCK_GAP;
    }

    let own_height = match node_ref.node_type {
        NodeType::Document => rect.height,
        NodeType::Text => rect.height.max(TEXT_LINE_HEIGHT),
        _ => {
            let children_extent = if children.is_empty() {
                0.0
            } else {
                cursor_y - rect.y - ROOT_PADDING.min(rect.height / 6.0) - BLOCK_GAP
            };
            let base = base_node_height(&node_ref.name, depth);
            (base + children_extent + INNER_PADDING * 2.0).max(rect.height.min(base))
        }
    };
    drop(node_ref);

    LayoutNode {
        node: node.clone(),
        rect: Rect::new(
            rect.x,
            rect.y,
            rect.width,
            own_height.min(rect.height.max(own_height)),
        ),
        children,
    }
}

fn estimate_node_height(node: &Node, depth: usize) -> f32 {
    let node_ref = node.borrow();
    match node_ref.node_type {
        NodeType::Text => {
            let text_len = node_ref
                .value
                .as_ref()
                .map(|value| value.len())
                .unwrap_or(0) as f32;
            TEXT_LINE_HEIGHT + (text_len / 42.0).floor() * 10.0
        }
        _ => {
            let child_count = node_ref.children.len() as f32;
            let base = base_node_height(&node_ref.name, depth);
            base + child_count * 20.0
        }
    }
}

fn base_node_height(name: &str, depth: usize) -> f32 {
    let depth_bias = (depth as f32 * 6.0).min(24.0);
    match name {
        "html" | "body" | "main" => 160.0 - depth_bias,
        "header" | "nav" | "section" | "article" => 124.0 - depth_bias,
        "aside" | "footer" => 112.0 - depth_bias,
        "div" | "span" => 96.0 - depth_bias,
        _ => LEAF_HEIGHT - depth_bias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use browser_dom::NodeType;

    #[test]
    fn create_layout_context() {
        let ctx = LayoutContext {
            viewport: Size::new(1920.0, 1080.0),
        };
        assert_eq!(ctx.viewport.width, 1920.0);
    }

    #[test]
    fn create_rect() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }

    #[test]
    fn builds_layout_tree_from_document_root() {
        let doc = Document::new();
        let html = doc.create_element("html");
        let body = doc.create_element("body");
        let section = doc.create_element("section");
        let text = doc.create_text_node("Next Gen OS browser frame");
        section.borrow_mut().append_child(text);
        body.borrow_mut().append_child(section);
        html.borrow_mut().append_child(body.clone());

        let mut doc = doc;
        doc.document_element = Some(html);
        doc.body = Some(body);

        let tree = build_layout_tree(
            &doc,
            &ComputedStyles::new(),
            &LayoutContext {
                viewport: Size::new(1280.0, 720.0),
            },
        );

        let root = tree.root.expect("layout tree should contain root");
        assert_eq!(root.node.borrow().node_type, NodeType::Element);
        assert_eq!(root.rect.width, 1280.0);
        assert!(!root.children.is_empty());
    }
}
