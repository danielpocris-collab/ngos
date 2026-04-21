use crate::SceneError;
use crate::camera_agent::Camera;
use crate::math_agent::{Mat4, Transform};
use crate::node_agent::NodeId;
use crate::scene_graph_agent::SceneGraph;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// A single node after world-transform resolution
#[derive(Debug, Clone)]
pub struct SubmittedNode {
    pub id: NodeId,
    pub world_transform: Transform,
    pub model_matrix: Mat4,
}

/// Output of a full scene submission
#[derive(Debug, Clone)]
pub struct SceneSubmission {
    pub nodes: Vec<SubmittedNode>,
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    /// view_projection = projection * view
    pub view_projection: Mat4,
}

impl SceneSubmission {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Submit the scene graph from the perspective of a camera node.
///
/// `camera_node_id` must be present in the graph.
/// All visible nodes are resolved to world-space and included.
pub fn submit(
    graph: &SceneGraph,
    camera: &Camera,
    camera_node_id: NodeId,
) -> Result<SceneSubmission, SceneError> {
    camera.validate()?;

    let camera_world = graph.world_transform(camera_node_id)?;
    let view_matrix = camera.view_matrix(camera_world);
    let projection_matrix = camera.projection_matrix();
    let view_projection = projection_matrix.mul(view_matrix);

    let mut submitted: Vec<SubmittedNode> = Vec::new();
    collect_visible(graph, &mut submitted)?;

    Ok(SceneSubmission {
        nodes: submitted,
        view_matrix,
        projection_matrix,
        view_projection,
    })
}

/// Traverse the entire graph and collect visible nodes with resolved world transforms
fn collect_visible(graph: &SceneGraph, out: &mut Vec<SubmittedNode>) -> Result<(), SceneError> {
    let ids = graph.all_node_ids();
    for id in ids {
        let node = graph.get(id).ok_or(SceneError::NodeNotFound { id })?;
        if !node.visible {
            continue;
        }
        let world = graph.world_transform(id)?;
        out.push(SubmittedNode {
            id,
            world_transform: world,
            model_matrix: world.to_mat4(),
        });
    }
    out.sort_by(|lhs, rhs| {
        match lhs
            .world_transform
            .translation
            .z
            .partial_cmp(&rhs.world_transform.translation.z)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => lhs.id.cmp(&rhs.id),
            other => other,
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_agent::PerspectiveCamera;
    use crate::math_agent::Vec3;
    use crate::scene_graph_agent::SceneGraph;

    fn perspective() -> Camera {
        Camera::Perspective(PerspectiveCamera {
            fov_y_rad: 1.047,
            aspect: 1.777,
            near: 0.1,
            far: 1000.0,
        })
    }

    fn translated(x: f32, y: f32, z: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, y, z),
            ..Transform::IDENTITY
        }
    }

    #[test]
    fn submit_single_root_node() {
        let mut g = SceneGraph::new();
        let cam_id = g.add(translated(0.0, 0.0, 5.0));
        let submission = submit(&g, &perspective(), cam_id).unwrap();
        assert_eq!(submission.node_count(), 1);
    }

    #[test]
    fn submit_three_nodes_all_visible() {
        let mut g = SceneGraph::new();
        let root = g.add_labeled(Transform::IDENTITY, "root");
        let a = g.add(translated(1.0, 0.0, 0.0));
        let b = g.add(translated(2.0, 0.0, 0.0));
        g.set_parent(a, root).unwrap();
        g.set_parent(b, root).unwrap();
        let cam_id = g.add(translated(0.0, 0.0, 10.0));
        let submission = submit(&g, &perspective(), cam_id).unwrap();
        // root + a + b + cam
        assert_eq!(submission.node_count(), 4);
    }

    #[test]
    fn submit_invisible_node_excluded() {
        let mut g = SceneGraph::new();
        let root = g.add(Transform::IDENTITY);
        let child = g.add(translated(1.0, 0.0, 0.0));
        g.set_parent(child, root).unwrap();
        g.get_mut(child).unwrap().visible = false;
        let cam_id = g.add(translated(0.0, 0.0, 5.0));
        let submission = submit(&g, &perspective(), cam_id).unwrap();
        // root + cam only (child excluded)
        assert_eq!(submission.node_count(), 2);
    }

    #[test]
    fn submit_invalid_camera_node_refused() {
        let mut g = SceneGraph::new();
        g.add(Transform::IDENTITY);
        let err = submit(&g, &perspective(), 999).unwrap_err();
        assert!(matches!(err, SceneError::NodeNotFound { id: 999 }));
    }

    #[test]
    fn submission_world_transform_accumulated() {
        let mut g = SceneGraph::new();
        let parent = g.add(translated(10.0, 0.0, 0.0));
        let child = g.add(translated(5.0, 0.0, 0.0));
        g.set_parent(child, parent).unwrap();
        let cam_id = g.add(translated(0.0, 0.0, 20.0));
        let submission = submit(&g, &perspective(), cam_id).unwrap();
        // find the child node
        let submitted_child = submission.nodes.iter().find(|n| n.id == child).unwrap();
        assert!(
            (submitted_child.world_transform.translation.x - 15.0).abs() < 1e-3,
            "got {}",
            submitted_child.world_transform.translation.x
        );
    }

    #[test]
    fn view_projection_not_identity() {
        let mut g = SceneGraph::new();
        let cam_id = g.add(translated(0.0, 0.0, 5.0));
        let sub = submit(&g, &perspective(), cam_id).unwrap();
        // VP matrix should not be identity
        let vp = sub.view_projection;
        let is_identity = (vp.cols[0][0] - 1.0).abs() < 1e-5
            && (vp.cols[1][1] - 1.0).abs() < 1e-5
            && (vp.cols[2][2] - 1.0).abs() < 1e-5
            && (vp.cols[3][3] - 1.0).abs() < 1e-5;
        assert!(!is_identity, "VP matrix should not be identity");
    }

    #[test]
    fn invalid_camera_params_refused() {
        let mut g = SceneGraph::new();
        let cam_id = g.add(Transform::IDENTITY);
        let bad_cam = Camera::Perspective(PerspectiveCamera {
            fov_y_rad: 1.047,
            aspect: 1.777,
            near: 0.0, // invalid
            far: 1000.0,
        });
        assert!(matches!(
            submit(&g, &bad_cam, cam_id),
            Err(SceneError::InvalidCamera { .. })
        ));
    }
}
