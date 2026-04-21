use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use ngos_gfx_translate::RgbaColor;
use ngos_scene_graph::{NodeId, SceneGraph, Transform};

use crate::{
    RenderError,
    attachment_agent::{AttachmentStore, AttachmentStoreInspect},
    depth_buffer_agent::{DepthBuffer, DepthTest},
    lighting_agent::{Light, LightManager},
    material_agent::{Material, MaterialId, Texture, TextureId},
    mesh_agent::{Mesh, MeshId, Vertex},
    rasterizer_agent::{Rasterizer, RasterizerConfig},
    render_pass_agent::{
        AttachmentKind, FrameGraphInspect, PassId, PassType, PipelineInspect, RenderPass,
        RenderPipeline, RenderTargetConfig,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RendererConfig {
    pub width: u32,
    pub height: u32,
    pub backface_culling: bool,
    pub depth_test: DepthTest,
    pub clear_color: RgbaColor,
}

impl RendererConfig {
    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        Ok(RendererConfig {
            width,
            height,
            backface_culling: true,
            depth_test: DepthTest::default(),
            clear_color: RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        })
    }

    pub fn with_backface_culling(mut self, enabled: bool) -> Self {
        self.backface_culling = enabled;
        self
    }

    pub fn with_depth_test(mut self, depth_test: DepthTest) -> Self {
        self.depth_test = depth_test;
        self
    }

    pub fn with_clear_color(mut self, color: RgbaColor) -> Self {
        self.clear_color = color;
        self
    }
}

#[derive(Debug, Clone)]
pub struct RendererInspect {
    pub width: u32,
    pub height: u32,
    pub mesh_count: usize,
    pub material_count: usize,
    pub texture_count: usize,
    pub light_count: usize,
    pub pass_count: usize,
    pub enabled_pass_count: usize,
    pub frame_index: u64,
    pub pbr_heavy_material_count: usize,
    pub frame_history_depth: usize,
    pub scene_binding_count: usize,
    pub attachment_store: AttachmentStoreInspect,
    pub work_queue: RenderQueueInspect,
    pub pipeline: PipelineInspect,
    pub frame_graph: FrameGraphInspect,
    pub last_frame: Option<RendererFrameReport>,
    pub queue_budget_default: RenderQueueBudget,
}

#[derive(Debug, Clone)]
pub struct PassExecutionReport {
    pub pass_type: PassType,
    pub pass_label: alloc::string::String,
    pub mesh_draws: usize,
    pub scene_nodes: usize,
    pub enabled_lights: usize,
    pub material_bindings: usize,
}

#[derive(Debug, Clone)]
pub struct RendererFrameReport {
    pub frame_index: u64,
    pub pass_reports: Vec<PassExecutionReport>,
    pub total_mesh_draws: usize,
    pub scene_nodes: usize,
    pub enabled_lights: usize,
    pub bound_scene_nodes: usize,
    pub written_attachments: Vec<AttachmentKind>,
    pub scene_instances: usize,
}

#[derive(Debug, Clone)]
pub struct RenderWorkItem {
    pub tag: String,
    pub submission: RenderSubmission,
}

#[derive(Debug, Clone)]
pub struct RenderQueueInspect {
    pub queued_items: usize,
    pub queued_tags: Vec<String>,
    pub queued_instance_total: usize,
    pub queued_bucket_total: usize,
}

#[derive(Debug, Clone)]
pub struct RenderQueueDrainReport {
    pub drained_items: usize,
    pub drained_tags: Vec<String>,
    pub total_buckets: usize,
    pub total_instances: usize,
    pub total_mesh_draws: usize,
    pub budget_exhausted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderQueueBudget {
    pub max_items: usize,
    pub max_instances: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneRenderBinding {
    pub mesh_id: MeshId,
    pub material_id: Option<MaterialId>,
    pub tint: Option<RgbaColor>,
    pub opacity: u8,
    pub layer: i16,
    pub priority: u16,
}

#[derive(Debug, Clone)]
pub struct RenderInstance {
    pub node_id: Option<NodeId>,
    pub label: Option<String>,
    pub mesh_id: MeshId,
    pub material_id: Option<MaterialId>,
    pub transform: Transform,
    pub tint: Option<RgbaColor>,
    pub opacity: u8,
    pub layer: i16,
    pub priority: u16,
}

#[derive(Debug, Clone)]
pub struct SubmissionBucket {
    pub pass_type: PassType,
    pub label: String,
    pub sort_key: String,
    pub instance_count: usize,
    pub instance_meshes: Vec<MeshId>,
    pub material_groups: usize,
}

#[derive(Debug, Clone)]
pub struct RenderSubmission {
    pub scene_instances: Vec<RenderInstance>,
    pub buckets: Vec<SubmissionBucket>,
}

pub struct Renderer {
    config: RendererConfig,
    rasterizer: Rasterizer,
    depth_buffer: DepthBuffer,
    meshes: BTreeMap<MeshId, Mesh>,
    materials: BTreeMap<MaterialId, Material>,
    textures: BTreeMap<TextureId, Texture>,
    attachment_store: AttachmentStore,
    scene_bindings: BTreeMap<NodeId, SceneRenderBinding>,
    lights: LightManager,
    pipeline: RenderPipeline,
    frame_index: u64,
    last_frame_report: Option<RendererFrameReport>,
    frame_history: VecDeque<RendererFrameReport>,
    work_queue: VecDeque<RenderWorkItem>,
    queue_budget_default: RenderQueueBudget,
    next_mesh_id: u32,
    next_material_id: u32,
    next_texture_id: u32,
}

impl Renderer {
    fn sqrt_f32(x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        let mut guess = x / 2.0;
        for _ in 0..10 {
            guess = (guess + x / guess) / 2.0;
        }
        guess
    }

    pub fn new(config: RendererConfig) -> Result<Self, RenderError> {
        let rasterizer_config = RasterizerConfig::new(config.width, config.height)
            .with_backface_culling(config.backface_culling)
            .with_depth_test(config.depth_test);
        let rasterizer = Rasterizer::new(rasterizer_config)?;
        let depth_buffer = DepthBuffer::new(config.width, config.height)?;
        let mut renderer = Renderer {
            config,
            rasterizer,
            depth_buffer,
            meshes: BTreeMap::new(),
            materials: BTreeMap::new(),
            textures: BTreeMap::new(),
            attachment_store: AttachmentStore::new(),
            scene_bindings: BTreeMap::new(),
            lights: LightManager::new(),
            pipeline: RenderPipeline::new(),
            frame_index: 0,
            last_frame_report: None,
            frame_history: VecDeque::new(),
            work_queue: VecDeque::new(),
            queue_budget_default: RenderQueueBudget {
                max_items: 8,
                max_instances: 4096,
            },
            next_mesh_id: 1,
            next_material_id: 1,
            next_texture_id: 1,
        };
        renderer.ensure_monster_pipeline();
        Ok(renderer)
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    // Mesh management
    pub fn register_mesh(&mut self, mesh: Mesh) -> MeshId {
        let id = MeshId::new(self.next_mesh_id);
        self.next_mesh_id += 1;
        self.meshes.insert(id, mesh);
        id
    }

    pub fn get_mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(&id)
    }

    pub fn remove_mesh(&mut self, id: MeshId) -> Option<Mesh> {
        self.meshes.remove(&id)
    }

    // Material management
    pub fn register_material(&mut self, mut material: Material) -> MaterialId {
        let id = MaterialId::new(self.next_material_id);
        self.next_material_id += 1;
        material.id = id;
        self.materials.insert(id, material);
        id
    }

    pub fn get_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    pub fn remove_material(&mut self, id: MaterialId) -> Option<Material> {
        self.materials.remove(&id)
    }

    // Texture management
    pub fn register_texture(&mut self, texture: Texture) -> TextureId {
        let id = TextureId::new(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(id, texture);
        id
    }

    pub fn get_texture(&self, id: TextureId) -> Option<&Texture> {
        self.textures.get(&id)
    }

    pub fn remove_texture(&mut self, id: TextureId) -> Option<Texture> {
        self.textures.remove(&id)
    }

    pub fn bind_scene_node(&mut self, node_id: NodeId, mesh_id: MeshId) {
        self.scene_bindings.insert(
            node_id,
            SceneRenderBinding {
                mesh_id,
                material_id: None,
                tint: None,
                opacity: 255,
                layer: 0,
                priority: 100,
            },
        );
    }

    pub fn bind_scene_node_with_material(
        &mut self,
        node_id: NodeId,
        mesh_id: MeshId,
        material_id: MaterialId,
    ) {
        self.scene_bindings.insert(
            node_id,
            SceneRenderBinding {
                mesh_id,
                material_id: Some(material_id),
                tint: None,
                opacity: 255,
                layer: 0,
                priority: 100,
            },
        );
    }

    pub fn tune_scene_node_binding(
        &mut self,
        node_id: NodeId,
        tint: Option<RgbaColor>,
        opacity: u8,
        layer: i16,
        priority: u16,
    ) {
        if let Some(binding) = self.scene_bindings.get_mut(&node_id) {
            binding.tint = tint;
            binding.opacity = opacity;
            binding.layer = layer;
            binding.priority = priority;
        }
    }

    pub fn unbind_scene_node(&mut self, node_id: NodeId) -> Option<SceneRenderBinding> {
        self.scene_bindings.remove(&node_id)
    }

    pub fn scene_binding_count(&self) -> usize {
        self.scene_bindings.len()
    }

    // Light management
    pub fn add_light(&mut self, light: Light) {
        self.lights.add_light(light);
    }

    pub fn remove_light(&mut self, index: usize) -> Option<Light> {
        self.lights.remove_light(index)
    }

    pub fn set_ambient_light(&mut self, color: RgbaColor, intensity: f32) {
        use crate::lighting_agent::AmbientLight;
        self.lights
            .set_ambient(AmbientLight::new(color).with_intensity(intensity));
    }

    // Pipeline management
    pub fn create_pass(&mut self, pass_type: PassType) -> PassId {
        self.pipeline.create_pass(pass_type)
    }

    pub fn ensure_monster_pipeline(&mut self) {
        self.pipeline.ensure_monster_pipeline();
        self.attachment_store.ensure_for_pipeline(
            self.config.width,
            self.config.height,
            &self.pipeline,
        );
    }

    pub fn reconfigure_render_target(
        &mut self,
        target: RenderTargetConfig,
    ) -> Result<(), RenderError> {
        let rasterizer_config = RasterizerConfig::new(target.width, target.height)
            .with_backface_culling(self.config.backface_culling)
            .with_depth_test(self.config.depth_test);
        self.rasterizer = Rasterizer::new(rasterizer_config)?;
        self.depth_buffer = DepthBuffer::new(target.width, target.height)?;
        self.config.width = target.width;
        self.config.height = target.height;
        self.config.clear_color = target.clear_color;
        self.attachment_store
            .ensure_for_pipeline(target.width, target.height, &self.pipeline);
        self.attachment_store
            .resize_all(target.width, target.height);
        self.clear_with(target.clear_color, target.clear_depth);
        Ok(())
    }

    pub fn get_pass(&self, id: PassId) -> Option<&RenderPass> {
        self.pipeline.get_pass(id)
    }

    pub fn get_pass_mut(&mut self, id: PassId) -> Option<&mut RenderPass> {
        self.pipeline.get_pass_mut(id)
    }

    pub fn remove_pass(&mut self, id: PassId) -> Option<RenderPass> {
        self.pipeline.remove_pass(id)
    }

    // Rendering
    pub fn clear(&mut self) {
        self.rasterizer.clear(self.config.clear_color);
        self.depth_buffer.clear_default();
    }

    pub fn clear_with(&mut self, color: RgbaColor, depth: f32) {
        self.rasterizer.clear(color);
        self.depth_buffer.clear(depth);
    }

    fn transform_vertex(&self, vertex: &Vertex, transform: &Transform) -> Vertex {
        use ngos_scene_graph::Vec3;
        let pos = Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]);
        let mat = transform.to_mat4();
        let transformed = mat.transform_point(pos);
        Vertex::new(
            [transformed.x, transformed.y, transformed.z],
            vertex.normal,
            vertex.tex_coord,
            vertex.color,
        )
    }

    fn transform_vertex_for_instance(&self, vertex: &Vertex, instance: &RenderInstance) -> Vertex {
        let mut transformed = self.transform_vertex(vertex, &instance.transform);
        if let Some(tint) = instance.tint {
            transformed.color = RgbaColor {
                r: (((transformed.color.r as u16) * (tint.r as u16)) / 255) as u8,
                g: (((transformed.color.g as u16) * (tint.g as u16)) / 255) as u8,
                b: (((transformed.color.b as u16) * (tint.b as u16)) / 255) as u8,
                a: (((transformed.color.a as u16) * (tint.a as u16)) / 255) as u8,
            };
        }
        transformed.color.a =
            (((transformed.color.a as u16) * (instance.opacity as u16)) / 255) as u8;
        transformed
    }

    fn instance_likely_visible(&self, instance: &RenderInstance) -> bool {
        let Some(mesh) = self.meshes.get(&instance.mesh_id) else {
            return false;
        };
        let center = mesh.center();
        let translated = [
            center[0] + instance.transform.translation.x,
            center[1] + instance.transform.translation.y,
            center[2] + instance.transform.translation.z,
        ];
        translated[0] >= -256.0
            && translated[1] >= -256.0
            && translated[0] <= self.config.width as f32 + 256.0
            && translated[1] <= self.config.height as f32 + 256.0
            && translated[2] >= -2048.0
            && translated[2] <= 2048.0
    }

    pub fn render_mesh(
        &mut self,
        mesh_id: MeshId,
        material_id: Option<MaterialId>,
        transform: &Transform,
    ) -> Result<(), RenderError> {
        let mesh = self
            .meshes
            .get(&mesh_id)
            .ok_or(RenderError::MeshNotFound(mesh_id))?;
        let _material = material_id.and_then(|id| self.materials.get(&id));

        let triangle_count = mesh.triangle_count();
        for i in 0..triangle_count {
            if let Some(triangle) = mesh.get_triangle(i) {
                let v0 = self.transform_vertex(&triangle[0], transform);
                let v1 = self.transform_vertex(&triangle[1], transform);
                let v2 = self.transform_vertex(&triangle[2], transform);

                let _pixels =
                    self.rasterizer
                        .rasterize_triangle(&v0, &v1, &v2, &mut self.depth_buffer)?;
            }
        }

        Ok(())
    }

    pub fn render_instance(&mut self, instance: &RenderInstance) -> Result<usize, RenderError> {
        let mesh = self
            .meshes
            .get(&instance.mesh_id)
            .ok_or(RenderError::MeshNotFound(instance.mesh_id))?;
        let _material = instance.material_id.and_then(|id| self.materials.get(&id));

        let triangle_count = mesh.triangle_count();
        for i in 0..triangle_count {
            if let Some(triangle) = mesh.get_triangle(i) {
                let v0 = self.transform_vertex_for_instance(&triangle[0], instance);
                let v1 = self.transform_vertex_for_instance(&triangle[1], instance);
                let v2 = self.transform_vertex_for_instance(&triangle[2], instance);

                let _pixels =
                    self.rasterizer
                        .rasterize_triangle(&v0, &v1, &v2, &mut self.depth_buffer)?;
            }
        }

        Ok(triangle_count)
    }

    pub fn collect_scene_instances(
        &self,
        scene: &SceneGraph,
    ) -> Result<Vec<RenderInstance>, RenderError> {
        let mut instances = Vec::new();
        for node_id in scene.all_node_ids() {
            let Some(node) = scene.get(node_id) else {
                continue;
            };
            if !node.visible {
                continue;
            }
            let binding = if let Some(binding) = self.scene_bindings.get(&node_id).copied() {
                Some(binding)
            } else {
                node.label
                    .as_deref()
                    .and_then(parse_scene_binding_label)
                    .map(|(mesh_id, material_id)| SceneRenderBinding {
                        mesh_id,
                        material_id,
                        tint: None,
                        opacity: 255,
                        layer: 0,
                        priority: 100,
                    })
            };
            let Some(binding) = binding else {
                continue;
            };
            let world =
                scene
                    .world_transform(node_id)
                    .map_err(|error| RenderError::SceneGraphError {
                        reason: format!("{:?}", error),
                    })?;
            instances.push(RenderInstance {
                node_id: Some(node_id),
                label: node.label.clone(),
                mesh_id: binding.mesh_id,
                material_id: binding.material_id,
                transform: self.scene_transform_to_ngos(&world),
                tint: binding.tint,
                opacity: binding.opacity,
                layer: binding.layer,
                priority: binding.priority,
            });
        }
        instances.sort_by(|lhs, rhs| {
            lhs.layer
                .cmp(&rhs.layer)
                .then(lhs.priority.cmp(&rhs.priority))
                .then(lhs.mesh_id.cmp(&rhs.mesh_id))
        });
        Ok(instances)
    }

    pub fn render_scene(&mut self, scene: &SceneGraph) -> Result<(), RenderError> {
        for instance in self.collect_scene_instances(scene)? {
            let _ = self.render_instance(&instance)?;
        }
        Ok(())
    }

    pub fn build_submission(&self, scene: Option<&SceneGraph>) -> RenderSubmission {
        let scene_instances = scene
            .and_then(|graph| self.collect_scene_instances(graph).ok())
            .unwrap_or_default();
        let scene_instances_by_priority = {
            let mut sorted = scene_instances.clone();
            sorted.sort_by(|lhs, rhs| {
                lhs.priority
                    .cmp(&rhs.priority)
                    .then(lhs.layer.cmp(&rhs.layer))
                    .then(lhs.mesh_id.cmp(&rhs.mesh_id))
            });
            sorted
        };
        let mut buckets = Vec::new();
        for pass in self.pipeline.enabled_passes() {
            let instance_meshes = if pass.mesh_ids.is_empty()
                && matches!(pass.pass_type, PassType::Geometry | PassType::Shadow)
            {
                if scene_instances_by_priority.is_empty() {
                    self.meshes.keys().copied().collect()
                } else {
                    scene_instances_by_priority
                        .iter()
                        .map(|instance| instance.mesh_id)
                        .collect()
                }
            } else {
                pass.mesh_ids.clone()
            };
            let mut material_groups = BTreeSet::new();
            for instance in &scene_instances_by_priority {
                if instance_meshes.contains(&instance.mesh_id) {
                    material_groups.insert(instance.material_id.unwrap_or(MaterialId::new(0)));
                }
            }
            buckets.push(SubmissionBucket {
                pass_type: pass.pass_type,
                label: pass.label.clone(),
                sort_key: format!(
                    "{}:{}:{}",
                    pass.domain.name(),
                    pass.pass_type.name(),
                    pass.label
                ),
                instance_count: instance_meshes.len(),
                instance_meshes,
                material_groups: material_groups.len(),
            });
        }
        buckets.sort_by(|lhs, rhs| lhs.sort_key.cmp(&rhs.sort_key));
        RenderSubmission {
            scene_instances: scene_instances_by_priority,
            buckets,
        }
    }

    pub fn enqueue_submission(&mut self, tag: &str, submission: RenderSubmission) {
        self.work_queue.push_back(RenderWorkItem {
            tag: String::from(tag),
            submission,
        });
    }

    pub fn enqueue_scene(&mut self, tag: &str, scene: Option<&SceneGraph>) {
        let submission = self.build_submission(scene);
        self.enqueue_submission(tag, submission);
    }

    pub fn drain_and_render_work_queue(&mut self) -> Result<RenderQueueDrainReport, RenderError> {
        self.drain_and_render_work_queue_with_budget(self.queue_budget_default)
    }

    pub fn drain_and_render_work_queue_with_budget(
        &mut self,
        budget: RenderQueueBudget,
    ) -> Result<RenderQueueDrainReport, RenderError> {
        let drained = self.drain_work_queue();
        let mut total_buckets = 0usize;
        let mut total_instances = 0usize;
        let mut total_mesh_draws = 0usize;
        let mut drained_tags = Vec::new();
        let mut budget_exhausted = false;
        let mut processed_items = 0usize;

        for item in drained {
            if processed_items >= budget.max_items {
                self.work_queue.push_front(item);
                budget_exhausted = true;
                break;
            }
            if total_instances + item.submission.scene_instances.len() > budget.max_instances {
                self.work_queue.push_front(item);
                budget_exhausted = true;
                break;
            }
            total_buckets += item.submission.buckets.len();
            total_instances += item.submission.scene_instances.len();
            drained_tags.push(item.tag);
            processed_items += 1;
            for instance in &item.submission.scene_instances {
                if !self.instance_likely_visible(instance) {
                    continue;
                }
                let triangle_count = self.render_instance(instance)?;
                if triangle_count > 0 {
                    total_mesh_draws += 1;
                }
            }
        }

        Ok(RenderQueueDrainReport {
            drained_items: drained_tags.len(),
            drained_tags,
            total_buckets,
            total_instances,
            total_mesh_draws,
            budget_exhausted,
        })
    }

    pub fn drain_work_queue(&mut self) -> Vec<RenderWorkItem> {
        let mut drained = Vec::new();
        while let Some(item) = self.work_queue.pop_front() {
            drained.push(item);
        }
        drained
    }

    pub fn inspect_work_queue(&self) -> RenderQueueInspect {
        RenderQueueInspect {
            queued_items: self.work_queue.len(),
            queued_tags: self
                .work_queue
                .iter()
                .map(|item| item.tag.clone())
                .collect(),
            queued_instance_total: self
                .work_queue
                .iter()
                .map(|item| item.submission.scene_instances.len())
                .sum(),
            queued_bucket_total: self
                .work_queue
                .iter()
                .map(|item| item.submission.buckets.len())
                .sum(),
        }
    }

    pub fn set_default_queue_budget(&mut self, budget: RenderQueueBudget) {
        self.queue_budget_default = budget;
    }

    fn scene_transform_to_ngos(&self, transform: &Transform) -> Transform {
        *transform
    }

    fn modulate_channel(value: u8, tint: u8, factor: f32) -> u8 {
        let factor = factor.clamp(0.0, 2.0);
        let tint_mix = tint as f32 / 255.0;
        let boosted = (value as f32) * (0.75 + (0.25 * tint_mix) + factor);
        boosted.clamp(0.0, 255.0) as u8
    }

    fn apply_lighting_pass(&mut self) -> Result<(), RenderError> {
        let ambient = self.lights.ambient();
        let directional_energy: f32 = self
            .lights
            .directional_lights()
            .map(|light| light.intensity.max(0.0))
            .sum();
        let point_energy: f32 = self
            .lights
            .point_lights()
            .map(|light| light.intensity.max(0.0))
            .sum();
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let cx = width as f32 * 0.5;
        let cy = height as f32 * 0.5;
        let max_dist = Self::sqrt_f32(cx * cx + cy * cy).max(1.0);

        for y in 0..height {
            for x in 0..width {
                let Some(mut pixel) = self.rasterizer.get_pixel(x, y) else {
                    continue;
                };
                if pixel.a == 0 {
                    continue;
                }
                let depth = self.depth_buffer.get(x, y).unwrap_or(1.0);
                let horizon = 1.0 - (y as f32 / height as f32);
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let radial = 1.0 - (Self::sqrt_f32(dx * dx + dy * dy) / max_dist).clamp(0.0, 1.0);
                let depth_weight = (1.0 - depth).clamp(0.0, 1.0);
                let light_factor = (ambient.intensity * 0.18)
                    + (directional_energy * (0.08 + (0.18 * horizon)))
                    + (point_energy * (0.05 + (0.15 * radial)))
                    + (depth_weight * 0.12);

                pixel.r = Self::modulate_channel(pixel.r, ambient.color.r, light_factor);
                pixel.g = Self::modulate_channel(pixel.g, ambient.color.g, light_factor);
                pixel.b = Self::modulate_channel(pixel.b, ambient.color.b, light_factor);
                self.rasterizer.set_pixel(x, y, pixel)?;
            }
        }

        Ok(())
    }

    fn apply_post_process_pass(&mut self) -> Result<(), RenderError> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let history_weight = (self.frame_history.len() as f32 * 0.015).clamp(0.0, 0.08);
        let cx = width as f32 * 0.5;
        let cy = height as f32 * 0.5;
        let max_dist = Self::sqrt_f32(cx * cx + cy * cy).max(1.0);

        for y in 0..height {
            let scanline = if y % 2 == 0 { 0.97 } else { 0.92 };
            for x in 0..width {
                let Some(mut pixel) = self.rasterizer.get_pixel(x, y) else {
                    continue;
                };
                if pixel.a == 0 {
                    continue;
                }
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let vignette: f32 = 1.0 - ((Self::sqrt_f32(dx * dx + dy * dy) / max_dist) * 0.22);
                let blend = (scanline * vignette.clamp(0.75, 1.0)) + history_weight;
                pixel.r = ((pixel.r as f32) * blend).clamp(0.0, 255.0) as u8;
                pixel.g = ((pixel.g as f32) * blend).clamp(0.0, 255.0) as u8;
                pixel.b = ((pixel.b as f32) * blend).clamp(0.0, 255.0) as u8;
                self.rasterizer.set_pixel(x, y, pixel)?;
            }
        }

        Ok(())
    }

    fn apply_present_pass(&mut self) -> Result<(), RenderError> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let accent = RgbaColor {
            r: 0x73,
            g: 0xD5,
            b: 0xFF,
            a: 0xFF,
        };

        for x in 0..width {
            if let Some(mut pixel) = self.rasterizer.get_pixel(x, 0) {
                pixel.r = pixel.r.max(accent.r / 2);
                pixel.g = pixel.g.max(accent.g / 2);
                pixel.b = pixel.b.max(accent.b / 2);
                pixel.a = 255;
                self.rasterizer.set_pixel(x, 0, pixel)?;
            }
        }

        for y in 0..height {
            if let Some(mut pixel) = self.rasterizer.get_pixel(0, y) {
                pixel.r = pixel.r.max(accent.r / 3);
                pixel.g = pixel.g.max(accent.g / 3);
                pixel.b = pixel.b.max(accent.b / 3);
                pixel.a = 255;
                self.rasterizer.set_pixel(0, y, pixel)?;
            }
        }

        for y in 0..height {
            for x in 0..width {
                let Some(mut pixel) = self.rasterizer.get_pixel(x, y) else {
                    continue;
                };
                if pixel.a == 0 && (pixel.r != 0 || pixel.g != 0 || pixel.b != 0) {
                    pixel.a = 255;
                    self.rasterizer.set_pixel(x, y, pixel)?;
                }
            }
        }

        Ok(())
    }

    pub fn render_pass(
        &mut self,
        pass: &RenderPass,
        scene: Option<&SceneGraph>,
    ) -> Result<(), RenderError> {
        let _ = self.render_pass_report(pass, scene)?;
        Ok(())
    }

    pub fn render_pass_report(
        &mut self,
        pass: &RenderPass,
        scene: Option<&SceneGraph>,
    ) -> Result<PassExecutionReport, RenderError> {
        if !pass.enabled {
            return Ok(PassExecutionReport {
                pass_type: pass.pass_type,
                pass_label: pass.label.clone(),
                mesh_draws: 0,
                scene_nodes: 0,
                enabled_lights: self.lights.enabled_count(),
                material_bindings: 0,
            });
        }

        if pass.should_clear_color() {
            if let Some(color) = pass.clear_color {
                self.rasterizer.clear(color);
            }
        }

        if pass.should_clear_depth() {
            if let Some(depth) = pass.clear_depth {
                self.depth_buffer.clear(depth);
            }
        }

        let mut mesh_draws = 0usize;
        match pass.pass_type {
            PassType::Depth | PassType::Shadow | PassType::Geometry => {
                if let Some(scene) = scene {
                    self.render_scene(scene)?;
                }

                let mesh_ids: Vec<MeshId> = if pass.mesh_ids.is_empty()
                    && matches!(pass.pass_type, PassType::Geometry | PassType::Shadow)
                {
                    self.meshes.keys().copied().collect()
                } else {
                    pass.mesh_ids.clone()
                };
                for mesh_id in mesh_ids {
                    let material_id = pass.material_ids.first().copied();
                    let identity = Transform::IDENTITY;
                    self.render_mesh(mesh_id, material_id, &identity)?;
                    mesh_draws += 1;
                }
            }
            PassType::Lighting => {
                self.apply_lighting_pass()?;
            }
            PassType::PostProcess => {
                self.apply_post_process_pass()?;
            }
            PassType::Present => {
                self.apply_present_pass()?;
            }
        }

        Ok(PassExecutionReport {
            pass_type: pass.pass_type,
            pass_label: pass.label.clone(),
            mesh_draws,
            scene_nodes: scene.map(|graph| graph.inspect().node_count).unwrap_or(0),
            enabled_lights: self.lights.enabled_count(),
            material_bindings: pass.material_ids.len(),
        })
    }

    pub fn render_frame(&mut self, scene: Option<&SceneGraph>) -> Result<(), RenderError> {
        self.clear();

        let mut pass_reports = Vec::new();
        let submission = self.build_submission(scene);
        let pass_ids: Vec<_> = self.pipeline.enabled_passes().map(|p| p.id).collect();
        for pass_id in pass_ids {
            if let Some(pass) = self.pipeline.get_pass(pass_id).cloned() {
                pass_reports.push(self.render_pass_report(&pass, scene)?);
            }
        }

        self.frame_index = self.frame_index.wrapping_add(1);
        self.last_frame_report = Some(RendererFrameReport {
            frame_index: self.frame_index,
            total_mesh_draws: pass_reports.iter().map(|report| report.mesh_draws).sum(),
            scene_nodes: scene.map(|graph| graph.inspect().node_count).unwrap_or(0),
            enabled_lights: self.lights.enabled_count(),
            bound_scene_nodes: self.scene_bindings.len(),
            written_attachments: self
                .pipeline
                .passes()
                .iter()
                .flat_map(|pass| pass.writes.iter().copied())
                .collect(),
            scene_instances: submission.scene_instances.len(),
            pass_reports,
        });
        if let Some(last_frame) = &self.last_frame_report {
            for pass in self.pipeline.passes() {
                for attachment in &pass.writes {
                    self.attachment_store.mark_written(
                        *attachment,
                        last_frame.frame_index,
                        pass.pass_type,
                    );
                }
            }
        }
        if let Some(last_frame) = self.last_frame_report.clone() {
            self.frame_history.push_back(last_frame);
            while self.frame_history.len() > 8 {
                let _ = self.frame_history.pop_front();
            }
        }

        Ok(())
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<RgbaColor> {
        self.rasterizer.get_pixel(x, y)
    }

    pub fn pixels(&self) -> &[RgbaColor] {
        self.rasterizer.pixels()
    }

    pub fn inspect(&self) -> RendererInspect {
        RendererInspect {
            width: self.config.width,
            height: self.config.height,
            mesh_count: self.meshes.len(),
            material_count: self.materials.len(),
            texture_count: self.textures.len(),
            light_count: self.lights.light_count(),
            pass_count: self.pipeline.pass_count(),
            enabled_pass_count: self.pipeline.enabled_pass_count(),
            frame_index: self.frame_index,
            pbr_heavy_material_count: self
                .materials
                .values()
                .filter(|material| material.is_pbr_heavy())
                .count(),
            frame_history_depth: self.frame_history.len(),
            scene_binding_count: self.scene_bindings.len(),
            attachment_store: self.attachment_store.inspect(),
            work_queue: self.inspect_work_queue(),
            pipeline: self.pipeline.inspect(),
            frame_graph: self.pipeline.frame_graph(),
            last_frame: self.last_frame_report.clone(),
            queue_budget_default: self.queue_budget_default,
        }
    }
}

fn parse_scene_binding_label(label: &str) -> Option<(MeshId, Option<MaterialId>)> {
    let mut mesh_id = None;
    let mut material_id = None;
    for token in label.split('|') {
        let token = token.trim();
        if let Some(value) = token.strip_prefix("mesh:") {
            mesh_id = value.parse::<u32>().ok().map(MeshId::new);
        } else if let Some(value) = token.strip_prefix("material:") {
            material_id = value.parse::<u32>().ok().map(MaterialId::new);
        }
    }
    mesh_id.map(|mesh_id| (mesh_id, material_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_agent::Vertex;

    fn test_color() -> RgbaColor {
        RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn test_vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex::new([x, y, z], [0.0, 1.0, 0.0], [0.0, 0.0], test_color())
    }

    fn test_mesh() -> Mesh {
        let vertices = vec![
            test_vertex(0.0, 0.0, 0.0),
            test_vertex(1.0, 0.0, 0.0),
            test_vertex(0.0, 1.0, 0.0),
        ];
        Mesh::new(MeshId::new(1), vertices).unwrap()
    }

    fn test_material() -> Material {
        Material::new(MaterialId::new(1), test_color())
    }

    fn test_texture() -> Texture {
        let pixels = vec![test_color(); 4];
        Texture::new(TextureId::new(1), 2, 2, pixels).unwrap()
    }

    #[test]
    fn renderer_creation() {
        let config = RendererConfig::new(640, 480).unwrap();
        let renderer = Renderer::new(config).unwrap();
        assert_eq!(renderer.width(), 640);
        assert_eq!(renderer.height(), 480);
    }

    #[test]
    fn renderer_zero_dimensions_rejected() {
        let config = RendererConfig::new(0, 480);
        assert!(matches!(config, Err(RenderError::InvalidRenderTarget)));
    }

    #[test]
    fn renderer_config_builder() {
        let config = RendererConfig::new(800, 600)
            .unwrap()
            .with_backface_culling(false)
            .with_depth_test(DepthTest::Disabled)
            .with_clear_color(RgbaColor {
                r: 100,
                g: 100,
                b: 100,
                a: 255,
            });

        assert!(!config.backface_culling);
        assert!(matches!(config.depth_test, DepthTest::Disabled));
        assert_eq!(config.clear_color.r, 100);
    }

    #[test]
    fn renderer_register_mesh() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let mesh = test_mesh();
        let id = renderer.register_mesh(mesh);
        assert!(id.0 > 0);
        assert!(renderer.get_mesh(id).is_some());
    }

    #[test]
    fn renderer_register_material() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let material = test_material();
        let id = renderer.register_material(material);
        assert!(id.0 > 0);
        assert!(renderer.get_material(id).is_some());
    }

    #[test]
    fn renderer_register_texture() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let texture = test_texture();
        let id = renderer.register_texture(texture);
        assert!(id.0 > 0);
        assert!(renderer.get_texture(id).is_some());
    }

    #[test]
    fn renderer_add_light() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        renderer.add_light(Light::directional([0.0, -1.0, 0.0], test_color()));
        assert_eq!(renderer.lights.light_count(), 1);
    }

    #[test]
    fn renderer_set_ambient() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        renderer.set_ambient_light(
            RgbaColor {
                r: 50,
                g: 50,
                b: 50,
                a: 255,
            },
            0.3,
        );
        let ambient = renderer.lights.ambient();
        assert_eq!(ambient.color.r, 50);
        assert_eq!(ambient.intensity, 0.3);
    }

    #[test]
    fn renderer_create_pass() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let id = renderer.create_pass(PassType::Geometry);
        assert!(id.0 > 0);
        assert!(renderer.get_pass(id).is_some());
    }

    #[test]
    fn renderer_clear() {
        let config = RendererConfig::new(100, 100).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        renderer.clear();

        for &pixel in renderer.pixels() {
            assert_eq!(pixel, config.clear_color);
        }
    }

    #[test]
    fn renderer_clear_with() {
        let config = RendererConfig::new(100, 100).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let color = RgbaColor {
            r: 128,
            g: 64,
            b: 32,
            a: 255,
        };
        renderer.clear_with(color, 0.5);

        for &pixel in renderer.pixels() {
            assert_eq!(pixel, color);
        }
    }

    #[test]
    fn renderer_render_mesh() {
        let config = RendererConfig::new(100, 100)
            .unwrap()
            .with_backface_culling(false)
            .with_depth_test(DepthTest::Disabled);
        let mut renderer = Renderer::new(config).unwrap();
        let mesh = test_mesh();
        let mesh_id = renderer.register_mesh(mesh);
        let transform = Transform::IDENTITY;

        let result = renderer.render_mesh(mesh_id, None, &transform);
        assert!(result.is_ok());
    }

    #[test]
    fn renderer_render_nonexistent_mesh() {
        let config = RendererConfig::new(100, 100).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        let transform = Transform::IDENTITY;

        let result = renderer.render_mesh(MeshId::new(99), None, &transform);
        assert!(matches!(result, Err(RenderError::MeshNotFound(_))));
    }

    #[test]
    fn renderer_inspect() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        renderer.register_mesh(test_mesh());
        renderer.register_material(test_material());
        renderer.register_texture(test_texture());
        renderer.add_light(Light::directional([0.0, -1.0, 0.0], test_color()));
        renderer.create_pass(PassType::Geometry);

        let inspect = renderer.inspect();
        assert_eq!(inspect.width, 640);
        assert_eq!(inspect.height, 480);
        assert_eq!(inspect.mesh_count, 1);
        assert_eq!(inspect.material_count, 1);
        assert_eq!(inspect.texture_count, 1);
        assert_eq!(inspect.light_count, 1);
        assert!(inspect.pass_count >= 6);
        assert!(inspect.enabled_pass_count >= 6);
    }

    #[test]
    fn renderer_get_pixel() {
        let config = RendererConfig::new(100, 100).unwrap();
        let mut renderer = Renderer::new(config).unwrap();
        renderer.clear();

        let pixel = renderer.get_pixel(50, 50);
        assert!(pixel.is_some());
        assert_eq!(pixel.unwrap(), config.clear_color);
    }

    #[test]
    fn renderer_remove_resources() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        let mesh_id = renderer.register_mesh(test_mesh());
        let material_id = renderer.register_material(test_material());
        let texture_id = renderer.register_texture(test_texture());

        assert!(renderer.remove_mesh(mesh_id).is_some());
        assert!(renderer.remove_material(material_id).is_some());
        assert!(renderer.remove_texture(texture_id).is_some());

        assert!(renderer.get_mesh(mesh_id).is_none());
        assert!(renderer.get_material(material_id).is_none());
        assert!(renderer.get_texture(texture_id).is_none());
    }

    #[test]
    fn renderer_remove_pass() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        let pass_id = renderer.create_pass(PassType::Geometry);
        assert!(renderer.remove_pass(pass_id).is_some());
        assert!(renderer.get_pass(pass_id).is_none());
    }

    #[test]
    fn renderer_remove_light() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        renderer.add_light(Light::directional([0.0, -1.0, 0.0], test_color()));
        let removed = renderer.remove_light(0);
        assert!(removed.is_some());
        assert_eq!(renderer.lights.light_count(), 0);
    }

    #[test]
    fn renderer_reconfigure_render_target_resizes_runtime_surfaces() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        let target = RenderTargetConfig::new(320, 200)
            .unwrap()
            .with_clear_color(RgbaColor {
                r: 12,
                g: 34,
                b: 56,
                a: 255,
            })
            .with_clear_depth(0.25);
        renderer.reconfigure_render_target(target).unwrap();

        assert_eq!(renderer.width(), 320);
        assert_eq!(renderer.height(), 200);
        assert_eq!(renderer.pixels().len(), 320 * 200);
        assert_eq!(
            renderer.get_pixel(0, 0),
            Some(RgbaColor {
                r: 12,
                g: 34,
                b: 56,
                a: 255,
            })
        );

        let inspect = renderer.inspect();
        assert_eq!(inspect.width, 320);
        assert_eq!(inspect.height, 200);
        assert!(
            inspect
                .attachment_store
                .entries
                .iter()
                .all(|entry| entry.contains(":320x200#"))
        );
    }

    #[test]
    fn renderer_reconfigure_render_target_rejects_zero_dimensions() {
        let config = RendererConfig::new(640, 480).unwrap();
        let renderer = Renderer::new(config).unwrap();

        let result = RenderTargetConfig::new(0, 200);
        assert!(matches!(result, Err(RenderError::InvalidRenderTarget)));
        assert_eq!(renderer.width(), 640);
        assert_eq!(renderer.height(), 480);
    }

    #[test]
    fn renderer_reconfigure_render_target_can_restore_previous_extent() {
        let config = RendererConfig::new(640, 480).unwrap();
        let mut renderer = Renderer::new(config).unwrap();

        renderer
            .reconfigure_render_target(RenderTargetConfig::new(320, 200).unwrap())
            .unwrap();
        renderer
            .reconfigure_render_target(
                RenderTargetConfig::new(640, 480)
                    .unwrap()
                    .with_clear_color(test_color()),
            )
            .unwrap();

        assert_eq!(renderer.width(), 640);
        assert_eq!(renderer.height(), 480);
        assert_eq!(renderer.pixels().len(), 640 * 480);
        assert_eq!(renderer.get_pixel(10, 10), Some(test_color()));
    }
}
