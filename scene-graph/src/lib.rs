#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: scene graph support
//! - owner layer: presentation support layer
//! - semantic owner: `scene-graph`
//! - truth path role: scene and camera composition support for user-facing
//!   rendering pipelines
//!
//! Canonical contract families defined here:
//! - scene graph contracts
//! - camera and transform support contracts
//! - scene submission support contracts
//!
//! This crate may define rendering-scene support behavior, but it must not
//! redefine kernel, runtime, or subsystem truth.

extern crate alloc;

mod camera_agent;
mod math_agent;
mod node_agent;
mod scene_graph_agent;
mod submission_agent;

pub use camera_agent::{Camera, OrthographicCamera, PerspectiveCamera};
pub use math_agent::{Mat4, Quat, Transform, Vec3};
pub use node_agent::{NodeId, SceneNode};
pub use scene_graph_agent::{GraphInspect, SceneGraph};
pub use submission_agent::{SceneSubmission, SubmittedNode, submit};

#[derive(Debug, Clone, PartialEq)]
pub enum SceneError {
    NodeNotFound { id: NodeId },
    SelfParent { id: NodeId },
    CyclicHierarchy { child: NodeId, parent: NodeId },
    InvalidCamera { reason: &'static str },
    EmptyGraph,
}

impl SceneError {
    pub fn describe(&self) -> alloc::string::String {
        use alloc::format;
        match self {
            SceneError::NodeNotFound { id } => format!("node {} not found in scene graph", id),
            SceneError::SelfParent { id } => format!("node {} cannot be its own parent", id),
            SceneError::CyclicHierarchy { child, parent } => {
                format!(
                    "setting parent {} for node {} would create a cycle",
                    parent, child
                )
            }
            SceneError::InvalidCamera { reason } => format!("invalid camera: {}", reason),
            SceneError::EmptyGraph => alloc::string::String::from("scene graph has no nodes"),
        }
    }
}
