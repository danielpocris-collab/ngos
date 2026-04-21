use crate::{
    RenderError, depth_buffer_agent::DepthTest, material_agent::MaterialId, mesh_agent::MeshId,
};
use alloc::{format, string::String, vec, vec::Vec};
use ngos_gfx_translate::RgbaColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassId(pub u32);

impl PassId {
    pub fn new(id: u32) -> Self {
        PassId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassType {
    Geometry,
    Depth,
    Shadow,
    Lighting,
    PostProcess,
    Present,
}

impl PassType {
    pub fn name(&self) -> &'static str {
        match self {
            PassType::Geometry => "geometry",
            PassType::Depth => "depth",
            PassType::Shadow => "shadow",
            PassType::Lighting => "lighting",
            PassType::PostProcess => "post_process",
            PassType::Present => "present",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassDomain {
    Visibility,
    Shadowing,
    Material,
    Lighting,
    PostFx,
    Presentation,
}

impl PassDomain {
    pub fn name(&self) -> &'static str {
        match self {
            PassDomain::Visibility => "visibility",
            PassDomain::Shadowing => "shadowing",
            PassDomain::Material => "material",
            PassDomain::Lighting => "lighting",
            PassDomain::PostFx => "postfx",
            PassDomain::Presentation => "presentation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttachmentKind {
    Color,
    Depth,
    ShadowMap,
    Normal,
    Material,
    Lighting,
    PostFx,
    History,
    Present,
}

impl AttachmentKind {
    pub fn name(&self) -> &'static str {
        match self {
            AttachmentKind::Color => "color",
            AttachmentKind::Depth => "depth",
            AttachmentKind::ShadowMap => "shadow-map",
            AttachmentKind::Normal => "normal",
            AttachmentKind::Material => "material",
            AttachmentKind::Lighting => "lighting",
            AttachmentKind::PostFx => "postfx",
            AttachmentKind::History => "history",
            AttachmentKind::Present => "present",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineInspect {
    pub total_passes: usize,
    pub enabled_passes: usize,
    pub geometry_passes: usize,
    pub depth_passes: usize,
    pub shadow_passes: usize,
    pub lighting_passes: usize,
    pub post_process_passes: usize,
    pub present_passes: usize,
    pub ordered_pass_names: Vec<String>,
    pub ordered_domains: Vec<String>,
    pub attachment_writes: Vec<String>,
    pub attachment_reads: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameGraphEdge {
    pub from: PassType,
    pub to: PassType,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameGraphInspect {
    pub pass_order: Vec<PassType>,
    pub edges: Vec<FrameGraphEdge>,
}

#[derive(Debug, Clone)]
pub struct RenderPass {
    pub id: PassId,
    pub pass_type: PassType,
    pub domain: PassDomain,
    pub label: String,
    pub enabled: bool,
    pub clear_color: Option<RgbaColor>,
    pub clear_depth: Option<f32>,
    pub depth_test: DepthTest,
    pub mesh_ids: Vec<MeshId>,
    pub material_ids: Vec<MaterialId>,
    pub reads_depth: bool,
    pub writes_depth: bool,
    pub requires_history: bool,
    pub reads: Vec<AttachmentKind>,
    pub writes: Vec<AttachmentKind>,
}

impl RenderPass {
    pub fn new(id: PassId, pass_type: PassType) -> Self {
        RenderPass {
            id,
            pass_type,
            domain: default_domain_for_pass(pass_type),
            label: String::from(pass_type.name()),
            enabled: true,
            clear_color: None,
            clear_depth: None,
            depth_test: DepthTest::default(),
            mesh_ids: Vec::new(),
            material_ids: Vec::new(),
            reads_depth: matches!(
                pass_type,
                PassType::Shadow | PassType::Geometry | PassType::Lighting
            ),
            writes_depth: matches!(
                pass_type,
                PassType::Depth | PassType::Shadow | PassType::Geometry
            ),
            requires_history: matches!(pass_type, PassType::PostProcess | PassType::Present),
            reads: default_reads_for_pass(pass_type),
            writes: default_writes_for_pass(pass_type),
        }
    }

    pub fn with_clear_color(mut self, color: RgbaColor) -> Self {
        self.clear_color = Some(color);
        self
    }

    pub fn with_clear_depth(mut self, depth: f32) -> Self {
        self.clear_depth = Some(depth);
        self
    }

    pub fn with_depth_test(mut self, depth_test: DepthTest) -> Self {
        self.depth_test = depth_test;
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = String::from(label);
        self
    }

    pub fn with_domain(mut self, domain: PassDomain) -> Self {
        self.domain = domain;
        self
    }

    pub fn with_mesh(mut self, mesh_id: MeshId) -> Self {
        self.mesh_ids.push(mesh_id);
        self
    }

    pub fn with_material(mut self, material_id: MaterialId) -> Self {
        self.material_ids.push(material_id);
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn reads_depth(mut self, reads_depth: bool) -> Self {
        self.reads_depth = reads_depth;
        self
    }

    pub fn writes_depth(mut self, writes_depth: bool) -> Self {
        self.writes_depth = writes_depth;
        self
    }

    pub fn requires_history(mut self, requires_history: bool) -> Self {
        self.requires_history = requires_history;
        self
    }

    pub fn read_attachment(mut self, attachment: AttachmentKind) -> Self {
        if !self.reads.contains(&attachment) {
            self.reads.push(attachment);
        }
        self
    }

    pub fn write_attachment(mut self, attachment: AttachmentKind) -> Self {
        if !self.writes.contains(&attachment) {
            self.writes.push(attachment);
        }
        self
    }

    pub fn should_clear_color(&self) -> bool {
        self.clear_color.is_some()
    }

    pub fn should_clear_depth(&self) -> bool {
        self.clear_depth.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct RenderPipeline {
    passes: Vec<RenderPass>,
    next_pass_id: u32,
}

impl RenderPipeline {
    pub fn new() -> Self {
        RenderPipeline {
            passes: Vec::new(),
            next_pass_id: 1,
        }
    }

    pub fn add_pass(&mut self, pass: RenderPass) -> PassId {
        let id = pass.id;
        self.passes.push(pass);
        id
    }

    pub fn create_pass(&mut self, pass_type: PassType) -> PassId {
        let id = PassId::new(self.next_pass_id);
        self.next_pass_id += 1;
        let pass = RenderPass::new(id, pass_type);
        self.add_pass(pass);
        id
    }

    pub fn remove_pass(&mut self, pass_id: PassId) -> Option<RenderPass> {
        if let Some(index) = self.passes.iter().position(|p| p.id == pass_id) {
            Some(self.passes.remove(index))
        } else {
            None
        }
    }

    pub fn get_pass(&self, pass_id: PassId) -> Option<&RenderPass> {
        self.passes.iter().find(|p| p.id == pass_id)
    }

    pub fn get_pass_mut(&mut self, pass_id: PassId) -> Option<&mut RenderPass> {
        self.passes.iter_mut().find(|p| p.id == pass_id)
    }

    pub fn enabled_passes(&self) -> impl Iterator<Item = &RenderPass> {
        self.passes.iter().filter(|p| p.enabled)
    }

    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }

    pub fn enabled_pass_count(&self) -> usize {
        self.passes.iter().filter(|p| p.enabled).count()
    }

    pub fn clear(&mut self) {
        self.passes.clear();
        self.next_pass_id = 1;
    }

    pub fn passes(&self) -> &[RenderPass] {
        &self.passes
    }

    pub fn inspect(&self) -> PipelineInspect {
        let mut geometry_passes = 0;
        let mut depth_passes = 0;
        let mut shadow_passes = 0;
        let mut lighting_passes = 0;
        let mut post_process_passes = 0;
        let mut present_passes = 0;
        let mut ordered_pass_names = Vec::new();
        let mut ordered_domains = Vec::new();
        let mut attachment_writes = Vec::new();
        let mut attachment_reads = Vec::new();
        for pass in &self.passes {
            ordered_pass_names.push(pass.label.clone());
            ordered_domains.push(String::from(pass.domain.name()));
            for attachment in &pass.writes {
                attachment_writes.push(format!("{}:{}", pass.label, attachment.name()));
            }
            for attachment in &pass.reads {
                attachment_reads.push(format!("{}:{}", pass.label, attachment.name()));
            }
            match pass.pass_type {
                PassType::Geometry => geometry_passes += 1,
                PassType::Depth => depth_passes += 1,
                PassType::Shadow => shadow_passes += 1,
                PassType::Lighting => lighting_passes += 1,
                PassType::PostProcess => post_process_passes += 1,
                PassType::Present => present_passes += 1,
            }
        }
        PipelineInspect {
            total_passes: self.passes.len(),
            enabled_passes: self.enabled_pass_count(),
            geometry_passes,
            depth_passes,
            shadow_passes,
            lighting_passes,
            post_process_passes,
            present_passes,
            ordered_pass_names,
            ordered_domains,
            attachment_writes,
            attachment_reads,
        }
    }

    pub fn frame_graph(&self) -> FrameGraphInspect {
        let mut pass_order = Vec::new();
        let mut edges = Vec::new();
        let ordered_types: Vec<_> = self.passes.iter().map(|pass| pass.pass_type).collect();
        for pass_type in &ordered_types {
            pass_order.push(*pass_type);
        }
        for (index, pass_type) in ordered_types.iter().enumerate() {
            if let Some(prev) = index
                .checked_sub(1)
                .and_then(|i| ordered_types.get(i))
                .copied()
            {
                edges.push(FrameGraphEdge {
                    from: prev,
                    to: *pass_type,
                    reason: String::from("ordered-pass-chain"),
                });
            }
            for dependency in pass_dependencies(*pass_type) {
                if ordered_types.contains(&dependency) {
                    edges.push(FrameGraphEdge {
                        from: *dependency,
                        to: *pass_type,
                        reason: String::from(pass_dependency_reason(*pass_type)),
                    });
                }
            }
        }
        FrameGraphInspect { pass_order, edges }
    }

    pub fn ensure_monster_pipeline(&mut self) {
        if !self.passes.is_empty() {
            return;
        }
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::Depth)
                .with_label("depth-prepass")
                .with_clear_depth(1.0),
        );
        self.next_pass_id += 1;
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::Shadow)
                .with_label("shadow-cascade")
                .with_clear_depth(1.0),
        );
        self.next_pass_id += 1;
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::Geometry)
                .with_label("geometry-gbuffer")
                .with_clear_color(RgbaColor {
                    r: 0x04,
                    g: 0x08,
                    b: 0x12,
                    a: 0xff,
                }),
        );
        self.next_pass_id += 1;
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::Lighting)
                .with_label("lighting-resolve"),
        );
        self.next_pass_id += 1;
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::PostProcess)
                .with_label("postfx-stack"),
        );
        self.next_pass_id += 1;
        self.add_pass(
            RenderPass::new(PassId::new(self.next_pass_id), PassType::Present)
                .with_label("present-swapchain"),
        );
        self.next_pass_id += 1;
    }

    pub fn reorder_passes(&mut self, new_order: Vec<PassId>) -> Result<(), RenderError> {
        if new_order.len() != self.passes.len() {
            return Err(RenderError::OutOfBounds);
        }

        let mut new_passes = Vec::with_capacity(self.passes.len());
        for pass_id in &new_order {
            if let Some(index) = self.passes.iter().position(|p| p.id == *pass_id) {
                new_passes.push(self.passes.remove(index));
            } else {
                return Err(RenderError::MeshNotFound(crate::mesh_agent::MeshId::new(0)));
            }
        }
        self.passes = new_passes;
        Ok(())
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

fn pass_dependencies(pass_type: PassType) -> &'static [PassType] {
    match pass_type {
        PassType::Depth => &[],
        PassType::Shadow => &[PassType::Depth],
        PassType::Geometry => &[PassType::Depth],
        PassType::Lighting => &[PassType::Geometry, PassType::Shadow],
        PassType::PostProcess => &[PassType::Lighting],
        PassType::Present => &[PassType::PostProcess],
    }
}

fn pass_dependency_reason(pass_type: PassType) -> &'static str {
    match pass_type {
        PassType::Depth => "root-pass",
        PassType::Shadow => "shadow-map-requires-depth",
        PassType::Geometry => "geometry-requires-depth-priming",
        PassType::Lighting => "lighting-requires-geometry-and-shadow",
        PassType::PostProcess => "post-process-requires-lighting",
        PassType::Present => "present-requires-post-process",
    }
}

fn default_domain_for_pass(pass_type: PassType) -> PassDomain {
    match pass_type {
        PassType::Depth | PassType::Geometry => PassDomain::Visibility,
        PassType::Shadow => PassDomain::Shadowing,
        PassType::Lighting => PassDomain::Lighting,
        PassType::PostProcess => PassDomain::PostFx,
        PassType::Present => PassDomain::Presentation,
    }
}

fn default_reads_for_pass(pass_type: PassType) -> Vec<AttachmentKind> {
    match pass_type {
        PassType::Depth => Vec::new(),
        PassType::Shadow => vec![AttachmentKind::Depth],
        PassType::Geometry => vec![AttachmentKind::Depth],
        PassType::Lighting => vec![
            AttachmentKind::Color,
            AttachmentKind::Normal,
            AttachmentKind::Material,
            AttachmentKind::ShadowMap,
        ],
        PassType::PostProcess => vec![AttachmentKind::Lighting, AttachmentKind::History],
        PassType::Present => vec![AttachmentKind::PostFx],
    }
}

fn default_writes_for_pass(pass_type: PassType) -> Vec<AttachmentKind> {
    match pass_type {
        PassType::Depth => vec![AttachmentKind::Depth],
        PassType::Shadow => vec![AttachmentKind::ShadowMap],
        PassType::Geometry => vec![
            AttachmentKind::Color,
            AttachmentKind::Normal,
            AttachmentKind::Material,
        ],
        PassType::Lighting => vec![AttachmentKind::Lighting],
        PassType::PostProcess => vec![AttachmentKind::PostFx, AttachmentKind::History],
        PassType::Present => vec![AttachmentKind::Present],
    }
}

#[derive(Debug, Clone)]
pub struct RenderTargetConfig {
    pub width: u32,
    pub height: u32,
    pub clear_color: RgbaColor,
    pub clear_depth: f32,
}

impl RenderTargetConfig {
    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        Ok(RenderTargetConfig {
            width,
            height,
            clear_color: RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            clear_depth: 1.0,
        })
    }

    pub fn with_clear_color(mut self, color: RgbaColor) -> Self {
        self.clear_color = color;
        self
    }

    pub fn with_clear_depth(mut self, depth: f32) -> Self {
        self.clear_depth = depth;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{material_agent::MaterialId, mesh_agent::MeshId};

    #[test]
    fn pass_id_creation() {
        let id = PassId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn pass_type_name() {
        assert_eq!(PassType::Geometry.name(), "geometry");
        assert_eq!(PassType::Depth.name(), "depth");
        assert_eq!(PassType::Shadow.name(), "shadow");
        assert_eq!(PassType::Lighting.name(), "lighting");
        assert_eq!(PassType::PostProcess.name(), "post_process");
        assert_eq!(PassType::Present.name(), "present");
    }

    #[test]
    fn render_pass_creation() {
        let pass = RenderPass::new(PassId::new(1), PassType::Geometry);
        assert_eq!(pass.id.0, 1);
        assert_eq!(pass.pass_type, PassType::Geometry);
        assert!(pass.enabled);
        assert!(pass.mesh_ids.is_empty());
    }

    #[test]
    fn render_pass_builder_pattern() {
        let pass = RenderPass::new(PassId::new(1), PassType::Geometry)
            .with_clear_color(RgbaColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            })
            .with_clear_depth(0.5)
            .with_depth_test(DepthTest::Disabled)
            .with_mesh(MeshId::new(10))
            .with_mesh(MeshId::new(20))
            .with_material(MaterialId::new(5));

        assert!(pass.should_clear_color());
        assert!(pass.should_clear_depth());
        assert_eq!(pass.clear_depth, Some(0.5));
        assert!(matches!(pass.depth_test, DepthTest::Disabled));
        assert_eq!(pass.mesh_ids.len(), 2);
        assert_eq!(pass.material_ids.len(), 1);
    }

    #[test]
    fn render_pass_enable_disable() {
        let pass = RenderPass::new(PassId::new(1), PassType::Geometry).disable();
        assert!(!pass.enabled);
        let pass = pass.enable();
        assert!(pass.enabled);
    }

    #[test]
    fn render_pipeline_creation() {
        let pipeline = RenderPipeline::new();
        assert_eq!(pipeline.pass_count(), 0);
        assert_eq!(pipeline.enabled_pass_count(), 0);
    }

    #[test]
    fn render_pipeline_create_pass() {
        let mut pipeline = RenderPipeline::new();
        let id = pipeline.create_pass(PassType::Geometry);
        assert_eq!(id.0, 1);
        assert_eq!(pipeline.pass_count(), 1);
    }

    #[test]
    fn render_pipeline_add_pass() {
        let mut pipeline = RenderPipeline::new();
        let pass = RenderPass::new(PassId::new(5), PassType::Lighting);
        pipeline.add_pass(pass);
        assert_eq!(pipeline.pass_count(), 1);
        assert!(pipeline.get_pass(PassId::new(5)).is_some());
    }

    #[test]
    fn render_pipeline_remove_pass() {
        let mut pipeline = RenderPipeline::new();
        let id = pipeline.create_pass(PassType::Geometry);
        let removed = pipeline.remove_pass(id);
        assert!(removed.is_some());
        assert_eq!(pipeline.pass_count(), 0);
    }

    #[test]
    fn render_pipeline_remove_nonexistent() {
        let mut pipeline = RenderPipeline::new();
        let removed = pipeline.remove_pass(PassId::new(99));
        assert!(removed.is_none());
    }

    #[test]
    fn render_pipeline_enabled_passes() {
        let mut pipeline = RenderPipeline::new();
        pipeline.create_pass(PassType::Geometry);
        let pass2 = pipeline.create_pass(PassType::Lighting);
        pipeline.create_pass(PassType::Present);

        if let Some(p) = pipeline.get_pass_mut(pass2) {
            p.enabled = false;
        }

        assert_eq!(pipeline.pass_count(), 3);
        assert_eq!(pipeline.enabled_pass_count(), 2);
        assert_eq!(pipeline.enabled_passes().count(), 2);
    }

    #[test]
    fn render_pipeline_clear() {
        let mut pipeline = RenderPipeline::new();
        pipeline.create_pass(PassType::Geometry);
        pipeline.create_pass(PassType::Lighting);
        pipeline.clear();
        assert_eq!(pipeline.pass_count(), 0);
        assert_eq!(pipeline.next_pass_id, 1);
    }

    #[test]
    fn render_pipeline_reorder_passes() {
        let mut pipeline = RenderPipeline::new();
        let id1 = pipeline.create_pass(PassType::Geometry);
        let id2 = pipeline.create_pass(PassType::Lighting);
        let id3 = pipeline.create_pass(PassType::Present);

        pipeline.reorder_passes(vec![id3, id1, id2]).unwrap();

        assert_eq!(pipeline.passes[0].id, id3);
        assert_eq!(pipeline.passes[1].id, id1);
        assert_eq!(pipeline.passes[2].id, id2);
    }

    #[test]
    fn render_pipeline_reorder_wrong_count() {
        let mut pipeline = RenderPipeline::new();
        pipeline.create_pass(PassType::Geometry);
        pipeline.create_pass(PassType::Lighting);

        let result = pipeline.reorder_passes(vec![PassId::new(1)]);
        assert!(matches!(result, Err(RenderError::OutOfBounds)));
    }

    #[test]
    fn render_pipeline_reorder_invalid_id() {
        let mut pipeline = RenderPipeline::new();
        pipeline.create_pass(PassType::Geometry);

        let result = pipeline.reorder_passes(vec![PassId::new(99)]);
        assert!(result.is_err());
    }

    #[test]
    fn render_target_config_creation() {
        let config = RenderTargetConfig::new(640, 480).unwrap();
        assert_eq!(config.width, 640);
        assert_eq!(config.height, 480);
        assert_eq!(
            config.clear_color,
            RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
        assert_eq!(config.clear_depth, 1.0);
    }

    #[test]
    fn render_target_config_zero_dimensions_rejected() {
        assert!(matches!(
            RenderTargetConfig::new(0, 480),
            Err(RenderError::InvalidRenderTarget)
        ));
    }

    #[test]
    fn render_target_config_builder() {
        let config = RenderTargetConfig::new(800, 600)
            .unwrap()
            .with_clear_color(RgbaColor {
                r: 100,
                g: 100,
                b: 100,
                a: 255,
            })
            .with_clear_depth(0.8);

        assert_eq!(config.clear_color.r, 100);
        assert_eq!(config.clear_depth, 0.8);
    }
}
