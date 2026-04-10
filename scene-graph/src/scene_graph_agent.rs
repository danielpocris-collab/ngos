use crate::SceneError;
use crate::math_agent::Transform;
use crate::node_agent::{NodeId, SceneNode};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

pub struct SceneGraph {
    nodes: BTreeMap<NodeId, SceneNode>,
    next_id: NodeId,
    root: Option<NodeId>,
}

pub struct GraphInspect {
    pub node_count: usize,
    pub root: Option<NodeId>,
    pub max_depth: usize,
}

impl SceneGraph {
    pub fn new() -> Self {
        SceneGraph {
            nodes: BTreeMap::new(),
            next_id: 1,
            root: None,
        }
    }

    /// Add a node with given transform; returns its NodeId
    pub fn add(&mut self, local_transform: Transform) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let node = SceneNode::new(id, local_transform);
        if self.root.is_none() {
            self.root = Some(id);
        }
        self.nodes.insert(id, node);
        id
    }

    /// Add a labeled node
    pub fn add_labeled(&mut self, local_transform: Transform, label: &str) -> NodeId {
        let id = self.add(local_transform);
        if let Some(n) = self.nodes.get_mut(&id) {
            n.label = Some(alloc::string::String::from(label));
        }
        id
    }

    /// Remove a node and detach from parent/children
    pub fn remove(&mut self, id: NodeId) -> Result<(), SceneError> {
        let node = self
            .nodes
            .remove(&id)
            .ok_or(SceneError::NodeNotFound { id })?;
        // Detach from parent
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.remove_child(id);
            }
        }
        // Detach children (they become orphans — callers must reparent)
        for child_id in &node.children {
            if let Some(child) = self.nodes.get_mut(child_id) {
                child.parent = None;
            }
        }
        // Update root
        if self.root == Some(id) {
            self.root = None;
        }
        Ok(())
    }

    /// Attach child to parent
    pub fn set_parent(&mut self, child_id: NodeId, parent_id: NodeId) -> Result<(), SceneError> {
        if child_id == parent_id {
            return Err(SceneError::SelfParent { id: child_id });
        }
        if !self.nodes.contains_key(&child_id) {
            return Err(SceneError::NodeNotFound { id: child_id });
        }
        if !self.nodes.contains_key(&parent_id) {
            return Err(SceneError::NodeNotFound { id: parent_id });
        }
        // Detect cycles: parent must not be a descendant of child
        if self.is_descendant(parent_id, child_id) {
            return Err(SceneError::CyclicHierarchy {
                child: child_id,
                parent: parent_id,
            });
        }
        // Remove from old parent
        let old_parent = self.nodes[&child_id].parent;
        if let Some(op) = old_parent {
            if let Some(p) = self.nodes.get_mut(&op) {
                p.remove_child(child_id);
            }
        }
        // Set new parent
        self.nodes.get_mut(&child_id).unwrap().parent = Some(parent_id);
        self.nodes
            .get_mut(&parent_id)
            .unwrap()
            .add_child(child_id)?;
        Ok(())
    }

    pub fn get(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) -> Result<(), SceneError> {
        if !self.nodes.contains_key(&id) {
            return Err(SceneError::NodeNotFound { id });
        }
        self.root = Some(id);
        Ok(())
    }

    /// Compute world transform for a node by traversing parent chain
    pub fn world_transform(&self, id: NodeId) -> Result<Transform, SceneError> {
        let mut chain: Vec<NodeId> = Vec::new();
        let mut current = id;
        loop {
            if !self.nodes.contains_key(&current) {
                return Err(SceneError::NodeNotFound { id: current });
            }
            chain.push(current);
            match self.nodes[&current].parent {
                None => break,
                Some(p) => current = p,
            }
        }
        // Apply transforms root→leaf
        chain.reverse();
        let mut world = Transform::IDENTITY;
        for nid in chain {
            let local = self.nodes[&nid].local_transform;
            world = Transform::combine(world, local);
        }
        Ok(world)
    }

    /// Returns all node IDs in the graph
    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    pub fn inspect(&self) -> GraphInspect {
        let max_depth = if let Some(root) = self.root {
            self.subtree_depth(root)
        } else {
            0
        };
        GraphInspect {
            node_count: self.nodes.len(),
            root: self.root,
            max_depth,
        }
    }

    // Returns true if `candidate` is a descendant of `ancestor`
    fn is_descendant(&self, candidate: NodeId, ancestor: NodeId) -> bool {
        let mut current = candidate;
        loop {
            if current == ancestor {
                return true;
            }
            match self.nodes.get(&current).and_then(|n| n.parent) {
                Some(p) => current = p,
                None => return false,
            }
        }
    }

    fn subtree_depth(&self, id: NodeId) -> usize {
        match self.nodes.get(&id) {
            None => 0,
            Some(n) if n.children.is_empty() => 1,
            Some(n) => {
                let max_child = n
                    .children
                    .iter()
                    .map(|&c| self.subtree_depth(c))
                    .max()
                    .unwrap_or(0);
                1 + max_child
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_agent::Vec3;

    fn identity() -> Transform {
        Transform::IDENTITY
    }

    fn translated(x: f32, y: f32, z: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, y, z),
            ..Transform::IDENTITY
        }
    }

    #[test]
    fn add_creates_node() {
        let mut g = SceneGraph::new();
        let id = g.add(identity());
        assert!(g.get(id).is_some());
    }

    #[test]
    fn first_node_is_root() {
        let mut g = SceneGraph::new();
        let id = g.add(identity());
        assert_eq!(g.root(), Some(id));
    }

    #[test]
    fn remove_node_cleans_up() {
        let mut g = SceneGraph::new();
        let id = g.add(identity());
        g.remove(id).unwrap();
        assert!(g.get(id).is_none());
    }

    #[test]
    fn remove_nonexistent_is_error() {
        let mut g = SceneGraph::new();
        assert!(matches!(
            g.remove(999),
            Err(SceneError::NodeNotFound { id: 999 })
        ));
    }

    #[test]
    fn set_parent_links_nodes() {
        let mut g = SceneGraph::new();
        let parent = g.add(identity());
        let child = g.add(identity());
        g.set_parent(child, parent).unwrap();
        let p_node = g.get(parent).unwrap();
        assert!(p_node.children.contains(&child));
        assert_eq!(g.get(child).unwrap().parent, Some(parent));
    }

    #[test]
    fn set_parent_self_refused() {
        let mut g = SceneGraph::new();
        let id = g.add(identity());
        assert!(matches!(
            g.set_parent(id, id),
            Err(SceneError::SelfParent { .. })
        ));
    }

    #[test]
    fn set_parent_cycle_refused() {
        let mut g = SceneGraph::new();
        let a = g.add(identity());
        let b = g.add(identity());
        g.set_parent(b, a).unwrap();
        // Now try to make a child of b — would create cycle
        assert!(matches!(
            g.set_parent(a, b),
            Err(SceneError::CyclicHierarchy { .. })
        ));
    }

    #[test]
    fn world_transform_single_node() {
        let mut g = SceneGraph::new();
        let id = g.add(translated(3.0, 0.0, 0.0));
        let wt = g.world_transform(id).unwrap();
        assert!((wt.translation.x - 3.0).abs() < 1e-4);
    }

    #[test]
    fn world_transform_parent_child() {
        let mut g = SceneGraph::new();
        let parent = g.add(translated(10.0, 0.0, 0.0));
        let child = g.add(translated(5.0, 0.0, 0.0));
        g.set_parent(child, parent).unwrap();
        let wt = g.world_transform(child).unwrap();
        assert!((wt.translation.x - 15.0).abs() < 1e-4);
    }

    #[test]
    fn inspect_node_count() {
        let mut g = SceneGraph::new();
        g.add(identity());
        g.add(identity());
        g.add(identity());
        assert_eq!(g.inspect().node_count, 3);
    }

    #[test]
    fn inspect_max_depth() {
        let mut g = SceneGraph::new();
        let a = g.add(identity());
        let b = g.add(identity());
        let c = g.add(identity());
        g.set_parent(b, a).unwrap();
        g.set_parent(c, b).unwrap();
        g.set_root(a).unwrap();
        let insp = g.inspect();
        assert_eq!(insp.max_depth, 3);
    }
}
