//! NGOS Browser DOM
//!
//! DOM Level 4 implementation - 100% Proprietary
//!
//! Canonical subsystem role:
//! - subsystem: browser DOM support
//! - owner layer: application support layer
//! - semantic owner: `browser-dom`
//! - truth path role: browser-facing DOM support for browser application flows
//!
//! Canonical contract families defined here:
//! - DOM node contracts
//! - document/tree support contracts
//! - browser DOM manipulation support contracts
//!
//! This crate may define browser DOM support behavior, but it must not
//! redefine kernel, runtime, or product-level OS truth.

pub use browser_core::{BrowserError, BrowserResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// DOM Node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Element,
    Attribute,
    Text,
    CDataSection,
    EntityReference,
    Entity,
    ProcessingInstruction,
    Comment,
    Document,
    DocumentType,
    DocumentFragment,
    Notation,
}

/// DOM Node
pub type Node = Rc<RefCell<NodeData>>;

/// Node data
pub struct NodeData {
    pub node_type: NodeType,
    pub name: String,
    pub namespace: Option<String>,
    pub value: Option<String>,
    pub children: Vec<Node>,
    pub parent: Option<Node>,
    pub attributes: HashMap<String, String>,
}

impl NodeData {
    pub fn new(node_type: NodeType, name: &str) -> Self {
        Self {
            node_type,
            name: String::from(name),
            namespace: None,
            value: None,
            children: Vec::new(),
            parent: None,
            attributes: HashMap::new(),
        }
    }

    pub fn append_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.attributes
            .insert(String::from(name), String::from(value));
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }
}

/// DOM Document
pub struct Document {
    pub document_element: Option<Node>,
    pub head: Option<Node>,
    pub body: Option<Node>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            document_element: None,
            head: None,
            body: None,
        }
    }

    /// Create element node
    pub fn create_element(&self, tag_name: &str) -> Node {
        Rc::new(RefCell::new(NodeData::new(NodeType::Element, tag_name)))
    }

    /// Create text node
    pub fn create_text_node(&self, data: &str) -> Node {
        let mut node = NodeData::new(NodeType::Text, "#text");
        node.value = Some(String::from(data));
        Rc::new(RefCell::new(node))
    }

    /// Get element by ID
    pub fn get_element_by_id(&self, id: &str) -> Option<Node> {
        if let Some(ref root) = self.document_element {
            Self::find_by_id(root, id)
        } else {
            None
        }
    }

    fn find_by_id(node: &Node, id: &str) -> Option<Node> {
        let node_ref = node.borrow();

        if node_ref.get_attribute("id") == Some(id) {
            return Some(Rc::clone(node));
        }

        for child in &node_ref.children {
            if let Some(found) = Self::find_by_id(child, id) {
                return Some(found);
            }
        }

        None
    }

    /// Get elements by tag name
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<Node> {
        let mut result = Vec::new();
        if let Some(ref root) = self.document_element {
            Self::collect_by_tag(root, tag, &mut result);
        }
        result
    }

    fn collect_by_tag(node: &Node, tag: &str, result: &mut Vec<Node>) {
        let node_ref = node.borrow();
        if node_ref.name.eq_ignore_ascii_case(tag) {
            result.push(Rc::clone(node));
        }

        for child in &node_ref.children {
            Self::collect_by_tag(child, tag, result);
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_document() {
        let doc = Document::new();
        assert!(doc.document_element.is_none());
    }

    #[test]
    fn create_element() {
        let doc = Document::new();
        let elem = doc.create_element("div");
        assert_eq!(elem.borrow().name, "div");
        assert_eq!(elem.borrow().node_type, NodeType::Element);
    }

    #[test]
    fn create_text_node() {
        let doc = Document::new();
        let text = doc.create_text_node("Hello World");
        assert_eq!(text.borrow().node_type, NodeType::Text);
        assert_eq!(text.borrow().value, Some(String::from("Hello World")));
    }

    #[test]
    fn set_get_attribute() {
        let doc = Document::new();
        let elem = doc.create_element("a");
        elem.borrow_mut()
            .set_attribute("href", "https://example.com");
        assert_eq!(
            elem.borrow().get_attribute("href"),
            Some("https://example.com")
        );
    }
}
