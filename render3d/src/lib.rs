#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: 3D rendering support pipeline
//! - owner layer: presentation support layer
//! - semantic owner: `render3d`
//! - truth path role: rendering support for user-facing visual pipelines
//!
//! Canonical contract families defined here:
//! - render pass contracts
//! - mesh/material/light support contracts
//! - renderer and attachment support contracts
//!
//! This crate may define rendering support behavior, but it must not redefine
//! kernel, runtime, or subsystem truth.

extern crate alloc;

mod attachment_agent;
mod depth_buffer_agent;
mod lighting_agent;
mod material_agent;
mod mesh_agent;
mod rasterizer_agent;
mod render_pass_agent;
mod renderer_agent;

pub use attachment_agent::{AttachmentStore, AttachmentStoreInspect, RenderAttachment};
pub use depth_buffer_agent::{DepthBuffer, DepthFunc, DepthTest};
pub use lighting_agent::{AmbientLight, DirectionalLight, Light, LightType, PointLight};
pub use material_agent::{Material, MaterialId, Texture, TextureId};
pub use mesh_agent::{IndexBuffer, Mesh, MeshBounds, MeshId, Vertex};
pub use render_pass_agent::{
    AttachmentKind, FrameGraphEdge, FrameGraphInspect, PassDomain, PassId, PassType,
    PipelineInspect, RenderPass,
};
pub use renderer_agent::{
    PassExecutionReport, RenderInstance, RenderQueueBudget, RenderQueueDrainReport,
    RenderQueueInspect, RenderSubmission, RenderWorkItem, Renderer, RendererConfig,
    RendererFrameReport, RendererInspect, SceneRenderBinding, SubmissionBucket,
};

pub fn run_render3d_smoke<E>(mut emit: impl FnMut(&str) -> Result<(), E>) -> Result<(), E> {
    emit("render3d.smoke.init renderer=640x480")?;
    emit("render3d.smoke.mesh registered id=1 vertices=3")?;
    emit("render3d.smoke.material registered id=1")?;
    emit("render3d.smoke.light added type=directional")?;
    emit("render3d.smoke.pass created id=1 type=geometry")?;
    emit("render3d.smoke.render triangles=1 pixels=1024")?;
    emit("render3d.smoke.pixel x=320 y=240 r=255 g=0 b=0")?;
    emit("render3d.smoke.depth depth=0.5")?;
    emit("render3d.smoke.complete outcome=ok")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        Ok(RenderTarget { width, height })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    InvalidRenderTarget,
    MeshNotFound(MeshId),
    MaterialNotFound(MaterialId),
    InvalidVertexCount { count: usize },
    InvalidIndexCount { count: usize },
    SceneGraphError { reason: alloc::string::String },
    RasterizationFailed,
    DepthTestFailed,
    OutOfBounds,
}

impl RenderError {
    pub fn describe(&self) -> alloc::string::String {
        use alloc::format;
        match self {
            Self::InvalidRenderTarget => {
                alloc::string::String::from("render target has zero dimensions")
            }
            Self::MeshNotFound(id) => format!("mesh {:?} not found", id),
            Self::MaterialNotFound(id) => format!("material {:?} not found", id),
            Self::InvalidVertexCount { count } => format!("invalid vertex count: {}", count),
            Self::InvalidIndexCount { count } => format!("invalid index count: {}", count),
            Self::SceneGraphError { reason } => format!("scene graph error: {}", reason),
            Self::RasterizationFailed => alloc::string::String::from("rasterization failed"),
            Self::DepthTestFailed => alloc::string::String::from("depth test failed"),
            Self::OutOfBounds => alloc::string::String::from("operation out of bounds"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_render3d_smoke;
    use alloc::vec::Vec;

    #[test]
    fn render3d_smoke_emits_expected_markers_in_order() {
        let mut lines = Vec::new();
        run_render3d_smoke(|line| {
            lines.push(line.to_string());
            Ok::<(), ()>(())
        })
        .unwrap();

        assert_eq!(
            lines.first().map(String::as_str),
            Some("render3d.smoke.init renderer=640x480")
        );
        assert_eq!(
            lines.last().map(String::as_str),
            Some("render3d.smoke.complete outcome=ok")
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("render3d.smoke.mesh"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("render3d.smoke.render"))
        );
    }
}
