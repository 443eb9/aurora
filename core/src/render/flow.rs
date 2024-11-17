use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::HashMap,
};

use encase::ShaderType;
use indexmap::IndexMap;
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue,
};
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, Color, ColorTargetState,
    ColorWrites, Device, Extent3d, Features, FragmentState, Limits, LoadOp, Operations,
    PipelineLayoutDescriptor, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
    VertexFormat, VertexState,
};

use crate::{
    render::{
        helper::Camera,
        mesh::{GpuMesh, StaticMesh},
        resource::{
            GpuCamera, GpuDirectionalLight, GpuPointLight, GpuSceneDesc, GpuSpotLight, RenderMesh,
            RenderTargets, DUMMY_2D_TEX, POST_PROCESS_COLOR_LAYOUT_UUID,
            POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::{GpuScene, MeshInstanceId},
    },
    WgpuRenderer,
};

pub enum NodeExtraData {
    Int(i32),
    UInt(u32),
    Float(f32),
    String(String),
}

impl From<i32> for NodeExtraData {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl From<u32> for NodeExtraData {
    fn from(value: u32) -> Self {
        Self::UInt(value)
    }
}

impl From<f32> for NodeExtraData {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<String> for NodeExtraData {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for NodeExtraData {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

struct PackedRenderNode {
    pub node: Box<dyn RenderNode>,
    pub context: NodeContext,
}

#[derive(Default)]
pub struct NodeContext {
    pub shaders: Vec<ShaderModule>,
    pub meshes: Vec<RenderMesh>,
    pub pipelines: HashMap<MeshInstanceId, RenderPipeline>,
    pub extra_data: HashMap<&'static str, NodeExtraData>,
}

pub struct RenderContext<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub node: &'a mut NodeContext,
    pub targets: &'a RenderTargets<'a>,
}

pub struct PipelineCreationContext<'a> {
    pub device: &'a Device,
    pub targets: &'a RenderTargets<'a>,
    pub shaders: &'a Vec<ShaderModule>,
    pub meshes: &'a Vec<RenderMesh>,
    pub pipelines: &'a mut HashMap<MeshInstanceId, RenderPipeline>,
}

#[derive(Default)]
pub struct RenderFlow {
    flow: IndexMap<TypeId, PackedRenderNode>,
    is_built: bool,
}

impl RenderFlow {
    pub async fn request_renderer(
        &self,
        features: Option<Features>,
        limits: Option<Limits>,
    ) -> WgpuRenderer {
        let mut features = features.unwrap_or_default();
        let mut limits = limits.unwrap_or_default();

        for node in self.flow.values() {
            node.node.require_renderer_features(&mut features);
            node.node.require_renderer_limits(&mut limits);
        }
        WgpuRenderer::new(Some(features), Some(limits)).await
    }

    #[inline]
    pub fn add<T: RenderNode + Default + 'static>(&mut self) {
        self.flow.insert(
            TypeId::of::<T>(),
            PackedRenderNode {
                node: Box::new(T::default()),
                context: Default::default(),
            },
        );
    }

    #[inline]
    pub fn add_initialized<T: RenderNode>(&mut self, node: T) {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for (index, dep) in node.add_node_dependencies() {
            let elem = (
                dep.identifier(),
                PackedRenderNode {
                    node: dep,
                    context: Default::default(),
                },
            );

            match index {
                DependencyNodeIndex::Before => before.push(elem),
                DependencyNodeIndex::After => after.push(elem),
            }
        }

        self.flow.extend(before);
        self.flow.insert(
            TypeId::of::<T>(),
            PackedRenderNode {
                node: Box::new(node),
                context: Default::default(),
            },
        );
        self.flow.extend(after);
    }

    #[inline]
    pub fn set_queue(&mut self, meshes: Vec<StaticMesh>) {
        let meshes = meshes
            .iter()
            .map(|mesh| RenderMesh {
                mesh: *mesh,
                offset: None,
            })
            .collect::<Vec<_>>();

        self.flow.values_mut().for_each(|node| {
            node.context.meshes = meshes.clone();
        });
    }

    #[inline]
    pub fn add_extra_data<N: RenderNode>(&mut self, name: &'static str, value: NodeExtraData) {
        if let Some(node) = self.flow.get_mut(&TypeId::of::<N>()) {
            node.context.extra_data.insert(name, value);
        }
    }

    #[inline]
    pub fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        targets: &RenderTargets,
    ) {
        if !self.is_built {
            self.force_build(renderer, scene, shader_defs, targets);
            self.is_built = true;
        }
    }

    #[inline]
    pub fn force_build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        targets: &RenderTargets,
    ) {
        for PackedRenderNode { node, context } in self.flow.values_mut() {
            if let Some(restriction) = node.restrict_mesh_format() {
                for mesh in &context.meshes {
                    scene.assets.meshes[&mesh.mesh.mesh].assert_vertex(restriction);
                }
            }
        }

        let mut shader_defs = shader_defs.unwrap_or_default();
        for node in self.flow.values() {
            node.node.require_shader_defs(&mut shader_defs);
        }

        for PackedRenderNode { node, context } in self.flow.values_mut() {
            if let Some(shaders) = node.require_shaders() {
                let mut compiled = Vec::with_capacity(shaders.len());
                for (deps, main) in shaders {
                    let mut composer = Composer::default();
                    for dep in deps.into_iter() {
                        composer
                            .add_composable_module(ComposableModuleDescriptor {
                                source: dep,
                                shader_defs: shader_defs.clone(),
                                ..Default::default()
                            })
                            .expect(&format!(
                                "Error on building shader dependencies for node {}",
                                node.label()
                            ));
                    }
                    let module = composer
                        .make_naga_module(NagaModuleDescriptor {
                            source: &main,
                            shader_defs: shader_defs.clone(),
                            ..Default::default()
                        })
                        .expect(&format!(
                            "Error on building main shader for node {}",
                            node.label()
                        ));

                    compiled.push(
                        renderer
                            .device
                            .create_shader_module(ShaderModuleDescriptor {
                                label: None,
                                source: ShaderSource::Naga(Cow::Owned(module)),
                            }),
                    );
                }
                context.shaders = compiled;
            }

            node.create_pipelines(
                scene,
                PipelineCreationContext {
                    device: &renderer.device,
                    targets,
                    shaders: &context.shaders,
                    meshes: &context.meshes,
                    pipelines: &mut context.pipelines,
                },
            );

            node.build(
                scene,
                RenderContext {
                    device: &renderer.device,
                    queue: &renderer.queue,
                    node: context,
                    targets,
                },
            );
        }
    }

    #[inline]
    pub fn run(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, targets: &RenderTargets) {
        for node in self.flow.values_mut() {
            node.node.prepare(
                scene,
                RenderContext {
                    device: &renderer.device,
                    queue: &renderer.queue,
                    node: &mut node.context,
                    targets,
                },
            );
        }

        for node in self.flow.values_mut() {
            node.node.draw(
                scene,
                RenderContext {
                    device: &renderer.device,
                    queue: &renderer.queue,
                    node: &mut node.context,
                    targets,
                },
            );
        }
    }
}

pub enum DependencyNodeIndex {
    Before,
    After,
}

pub trait RenderNode: 'static {
    fn identifier(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Get label of this node.
    fn label(&self) -> &'static str {
        type_name::<Self>()
    }

    /// Restrict the format that this node accepts.
    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        None
    }

    /// Add nodes as dependencies.
    fn add_node_dependencies(&self) -> Vec<(DependencyNodeIndex, Box<dyn RenderNode>)> {
        Vec::new()
    }

    /// Add required features
    fn require_renderer_features(&self, _features: &mut Features) {}

    /// Add required limits
    fn require_renderer_limits(&self, _limits: &mut Limits) {}

    /// Add required shader defs.
    fn require_shader_defs(&self, _shader_defs: &mut HashMap<String, ShaderDefValue>) {}

    /// Construct required shader, returns (dependencies, main_shader)
    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        None
    }

    /// Create pipeline for meshes.
    fn create_pipelines(&mut self, _scene: &mut GpuScene, _context: PipelineCreationContext) {}

    /// Build the node.
    fn build(&mut self, _scene: &mut GpuScene, _context: RenderContext) {}

    /// Prepare bind groups and other assets for rendering.
    fn prepare(&mut self, _scene: &mut GpuScene, _context: RenderContext) {}

    /// Draw meshes.
    fn draw(&self, _scene: &mut GpuScene, _context: RenderContext) {}
}

/// Prepares camera, lights and post process bind groups.
#[derive(Default)]
pub struct GeneralNode;

impl RenderNode for GeneralNode {
    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext { device, .. }: RenderContext,
    ) {
        for (id, mesh) in &assets.meshes {
            if !assets.gpu_meshes.contains_key(id) {
                if let Some(vertex_buffer) = mesh.create_vertex_buffer(device) {
                    assets.gpu_meshes.insert(
                        *id,
                        GpuMesh {
                            vertex_buffer,
                            index_buffer: mesh.create_index_buffer(device),
                            vertices_count: mesh.vertices_count() as u32,
                        },
                    );
                }
            }
        }

        assets.common_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("common_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuCamera::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuSceneDesc::min_size()),
                    },
                    count: None,
                },
            ],
        }));

        assets.lights_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lights_layout"),
            entries: &[
                // Directional
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuDirectionalLight::min_size()),
                    },
                    count: None,
                },
                // Point
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuPointLight::min_size()),
                    },
                    count: None,
                },
                // Spot
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuSpotLight::min_size()),
                    },
                    count: None,
                },
            ],
        }));
    }

    fn prepare(
        &mut self,
        scene: &mut GpuScene,
        RenderContext { device, queue, .. }: RenderContext,
    ) {
        let GpuScene {
            assets, original, ..
        } = scene;

        assets.directional_light_buffer.clear();
        assets.point_light_buffer.clear();
        assets.spot_light_buffer.clear();

        for light in original.dir_lights.values() {
            assets.directional_light_buffer.push(light);
        }

        for light in original.point_lights.values() {
            assets.point_light_buffer.push(light);
        }

        for light in original.spot_lights.values() {
            assets.spot_light_buffer.push(light);
        }

        assets.camera_uniform.clear();
        assets
            .camera_uniform
            .push(&<Camera as Into<GpuCamera>>::into(original.camera));
        assets.scene_desc_uniform.clear();
        assets.scene_desc_uniform.push(&GpuSceneDesc {
            dir_lights: original.dir_lights.len() as u32,
            point_lights: original.point_lights.len() as u32,
            spot_lights: original.spot_lights.len() as u32,
        });

        assets.camera_uniform.write::<GpuCamera>(&device, &queue);
        assets
            .scene_desc_uniform
            .write::<GpuSceneDesc>(&device, &queue);
        assets
            .directional_light_buffer
            .write::<GpuDirectionalLight>(&device, &queue);
        assets
            .point_light_buffer
            .write::<GpuPointLight>(&device, &queue);
        assets
            .spot_light_buffer
            .write::<GpuSpotLight>(&device, &queue);

        let (
            Some(bf_camera),
            Some(bf_gpu_scene_desc),
            Some(bf_dir_lights),
            Some(bf_point_lights),
            Some(bf_spot_lights),
        ) = (
            assets.camera_uniform.entire_binding(),
            assets.scene_desc_uniform.entire_binding(),
            assets.directional_light_buffer.entire_binding(),
            assets.point_light_buffer.entire_binding(),
            assets.spot_light_buffer.entire_binding(),
        )
        else {
            return;
        };

        assets.common_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("common_bind_group"),
            layout: assets.common_layout.as_ref().unwrap(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bf_camera,
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bf_gpu_scene_desc,
                },
            ],
        }));

        assets.light_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("lights_bind_group"),
            layout: assets.lights_layout.as_ref().unwrap(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bf_dir_lights,
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bf_point_lights,
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bf_spot_lights,
                },
            ],
        }));
    }
}

/// Added the post process related bing group layouts.
#[derive(Default)]
pub struct PostProcessGeneralNode;

impl RenderNode for PostProcessGeneralNode {
    fn build(
        &mut self,
        scene: &mut GpuScene,
        RenderContext {
            device,
            queue: _,
            node: _,
            targets: _,
        }: RenderContext,
    ) {
        scene.assets.material_layouts.insert(
            POST_PROCESS_COLOR_LAYOUT_UUID,
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("post_process_color_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }),
        );

        scene.assets.material_layouts.insert(
            POST_PROCESS_DEPTH_LAYOUT_UUID,
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("post_process_depth_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }),
        );
    }
}

#[derive(Default)]
pub struct ImageFallbackNode;

impl RenderNode for ImageFallbackNode {
    fn build(
        &mut self,
        scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node: _,
            targets: _,
        }: RenderContext,
    ) {
        scene.assets.textures.insert(
            DUMMY_2D_TEX,
            device.create_texture_with_data(
                &queue,
                &TextureDescriptor {
                    label: Some("dummy_2d"),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[TextureFormat::Rgba8Unorm],
                },
                TextureDataOrder::MipMajor,
                &[255; 4],
            ),
        );
    }
}

#[derive(Default)]
pub struct PresentNode {
    pipeline: Option<RenderPipeline>,
    bind_group: Option<BindGroup>,
}

impl RenderNode for PresentNode {
    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[(&[], include_str!("present.wgsl"))])
    }

    fn build(
        &mut self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            node,
            targets,
            ..
        }: RenderContext,
    ) {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("present_sampler"),
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("present_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("present_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&targets.swap_chain.current_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("present_pipeline_layout"),
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("present_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[0],
                entry_point: "fragment",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: targets.color_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        });

        self.pipeline = Some(pipeline);
        self.bind_group = Some(bind_group);
    }

    fn draw(
        &self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("present_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.surface,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
