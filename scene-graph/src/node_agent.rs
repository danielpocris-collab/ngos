use crate::SceneError;
use crate::math_agent::Transform;
use alloc::vec::Vec;

pub type NodeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    pub id: NodeId,
    pub label: Option<alloc::string::String>,
    pub local_transform: Transform,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub visible: bool,
}

impl SceneNode {
    pub fn new(id: NodeId, local_transform: Transform) -> Self {
        SceneNode {
            id,
            label: None,
            local_transform,
            parent: None,
            children: Vec::new(),
            visible: true,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(alloc::string::String::from(label));
        self
    }

    pub fn add_child(&mut self, child_id: NodeId) -> Result<(), SceneError> {
        if child_id == self.id {
            return Err(SceneError::SelfParent { id: self.id });
        }
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
        Ok(())
    }

    pub fn remove_child(&mut self, child_id: NodeId) {
        self.children.retain(|&c| c != child_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_new_has_no_parent_or_children() {
        let n = SceneNode::new(1, Transform::IDENTITY);
        assert_eq!(n.id, 1);
        assert!(n.parent.is_none());
        assert!(n.children.is_empty());
        assert!(n.visible);
    }

    #[test]
    fn node_add_child_adds_id() {
        let mut n = SceneNode::new(1, Transform::IDENTITY);
        n.add_child(2).unwrap();
        assert!(n.children.contains(&2));
    }

    #[test]
    fn node_add_child_self_is_refused() {
        let mut n = SceneNode::new(1, Transform::IDENTITY);
        let err = n.add_child(1).unwrap_err();
        assert!(matches!(err, SceneError::SelfParent { id: 1 }));
    }

    #[test]
    fn node_remove_child_removes_id() {
        let mut n = SceneNode::new(1, Transform::IDENTITY);
        n.add_child(2).unwrap();
        n.remove_child(2);
        assert!(n.children.is_empty());
    }

    #[test]
    fn node_with_label() {
        let n = SceneNode::new(5, Transform::IDENTITY).with_label("root");
        assert_eq!(n.label.as_deref(), Some("root"));
    }
}
