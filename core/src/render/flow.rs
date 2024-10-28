use std::{borrow::Cow, collections::HashMap};

use encase::ShaderType;
use indexmap::IndexMap;
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue,
};
use uuid::Uuid;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, Device, Extent3d, Features, Limits, Queue, RenderPipeline,
    SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDimension, VertexFormat,
};

use crate::{
    render::{
        helper::Camera,
        mesh::StaticMesh,
        resource::{
            GpuCamera, GpuDirectionalLight, GpuPointLight, GpuSceneDesc, GpuSpotLight, RenderMesh,
            RenderTargets, DUMMY_2D_TEX, POST_PROCESS_COLOR_LAYOUT_UUID,
            POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::{GpuScene, MeshInstanceId},
    },
    WgpuRenderer,
};

struct PackedRenderNode {
    pub node: Box<dyn RenderNode>,
    pub context: NodeContext,
}

#[derive(Default)]
pub struct NodeContext {
    pub shader: Option<ShaderModule>,
    pub meshes: Vec<RenderMesh>,
    pub pipelines: HashMap<MeshInstanceId, RenderPipeline>,
}

pub struct RenderContext<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub node: &'a mut NodeContext,
    pub targets: &'a RenderTargets,
}

pub struct PipelineCreationContext<'a> {
    pub device: &'a Device,
    pub targets: &'a RenderTargets,
    pub shader: &'a ShaderModule,
    pub meshes: &'a Vec<RenderMesh>,
    pub pipelines: &'a mut HashMap<MeshInstanceId, RenderPipeline>,
}

#[derive(Default)]
pub struct RenderFlow {
    flow: IndexMap<Uuid, PackedRenderNode>,
    is_built: bool,
}

impl RenderFlow {
    pub async fn request_renderer(&self) -> WgpuRenderer {
        let (features, limits) = self.flow.values().fold(
            (Default::default(), Default::default()),
            |(mut feat, mut lim), node| {
                node.node.require_renderer_features(&mut feat);
                node.node.require_renderer_limits(&mut lim);
                (feat, lim)
            },
        );
        WgpuRenderer::new(Some(features), Some(limits)).await
    }

    #[inline]
    pub fn add<T: RenderNode + Default + 'static>(&mut self) -> Uuid {
        let uuid = Uuid::new_v4();
        self.flow.insert(
            uuid,
            PackedRenderNode {
                node: Box::new(T::default()),
                context: Default::default(),
            },
        );
        uuid
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
        for node in self.flow.values() {
            if let Some(restriction) = node.node.restrict_mesh_format() {
                for mesh in &node.context.meshes {
                    scene.assets.meshes[&mesh.mesh.mesh].assert_vertex(restriction);
                }
            }
        }

        let mut shader_defs = shader_defs.unwrap_or_default();
        for node in self.flow.values() {
            node.node.require_shader_defs(&mut shader_defs);
        }

        for PackedRenderNode { node, context } in self.flow.values_mut() {
            if let Some((deps, main)) = node.require_shader() {
                let mut composer = Composer::default();
                for dep in deps {
                    composer
                        .add_composable_module(ComposableModuleDescriptor {
                            source: dep,
                            shader_defs: shader_defs.clone(),
                            ..Default::default()
                        })
                        .unwrap();
                }
                let module = composer
                    .make_naga_module(NagaModuleDescriptor {
                        source: &main,
                        shader_defs: shader_defs.clone(),
                        ..Default::default()
                    })
                    .unwrap();

                context.shader = Some(renderer.device.create_shader_module(
                    ShaderModuleDescriptor {
                        label: None,
                        source: ShaderSource::Naga(Cow::Owned(module)),
                    },
                ));
            }

            if let Some(shader) = &context.shader {
                node.create_pipelines(
                    scene,
                    PipelineCreationContext {
                        device: &renderer.device,
                        targets,
                        shader,
                        meshes: &context.meshes,
                        pipelines: &mut context.pipelines,
                    },
                );
            }

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

pub trait RenderNode {
    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        None
    }
    /// Add required features
    fn require_renderer_features(&self, _features: &mut Features) {}
    /// Add required limits
    fn require_renderer_limits(&self, _limits: &mut Limits) {}
    /// Add required shader defs.
    fn require_shader_defs(&self, _shader_defs: &mut HashMap<String, ShaderDefValue>) {}
    fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
        None
    }
    /// Create pipeline for meshes.
    fn create_pipelines(&self, _scene: &mut GpuScene, _context: PipelineCreationContext) {}
    /// Build the node.
    fn build(&mut self, scene: &mut GpuScene, context: RenderContext);
    /// Prepare bind groups and other assets for rendering.
    fn prepare(&mut self, _scene: &mut GpuScene, _context: RenderContext) {}
    /// Draw meshes.
    fn draw(&self, _scene: &mut GpuScene, _context: RenderContext) {}
}

/// Prepares camera, lights and post process bind groups.
#[derive(Default)]
pub struct GeneralNode;

impl RenderNode for GeneralNode {
    fn build(&mut self, scene: &mut GpuScene, RenderContext { device, .. }: RenderContext) {
        scene.assets.common_layout =
            Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        scene.assets.lights_layout =
            Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        for light in original.directional_lights.values() {
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
            dir_lights: original.directional_lights.len() as u32,
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
