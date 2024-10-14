use std::collections::HashMap;

use encase::ShaderType;
use indexmap::IndexMap;
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, Extent3d, SamplerBindingType, ShaderStages, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
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
        scene::GpuScene,
    },
    WgpuRenderer,
};

#[derive(Default)]
pub struct RenderFlow {
    flow: IndexMap<Uuid, (Box<dyn RenderNode>, Vec<RenderMesh>)>,
    is_built: bool,
}

impl RenderFlow {
    #[inline]
    pub fn add<T: RenderNode + Default + 'static>(&mut self) -> Uuid {
        let uuid = Uuid::new_v4();
        self.flow.insert(uuid, (Box::new(T::default()), Vec::new()));
        uuid
    }

    #[inline]
    pub fn queue_mesh(&mut self, node: Uuid, mesh: StaticMesh) {
        if let Some((_, queue)) = self.flow.get_mut(&node) {
            queue.push(RenderMesh { mesh, offset: None });
        }
    }

    #[inline]
    pub fn queue_global(&mut self, mesh: StaticMesh) {
        self.flow
            .values_mut()
            .for_each(|(_, queue)| queue.push(RenderMesh { mesh, offset: None }));
    }

    #[inline]
    pub fn set_queue(&mut self, node: Uuid, meshes: Vec<StaticMesh>) {
        if let Some((_, queue)) = self.flow.get_mut(&node) {
            *queue = meshes
                .into_iter()
                .map(|mesh| RenderMesh { mesh, offset: None })
                .collect();
        }
    }

    #[inline]
    pub fn set_queue_global(&mut self, meshes: Vec<StaticMesh>) {
        self.flow.values_mut().for_each(|(_, queue)| {
            *queue = meshes
                .iter()
                .map(|mesh| RenderMesh {
                    mesh: *mesh,
                    offset: None,
                })
                .collect()
        });
    }

    #[inline]
    pub fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        if !self.is_built {
            self.force_build(renderer, scene, shader_defs, target);
            self.is_built = true;
        }
    }

    #[inline]
    pub fn force_build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        for (node, _) in self.flow.values_mut() {
            node.build(renderer, scene, shader_defs.clone(), target);
        }
    }

    #[inline]
    pub fn run(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, target: &RenderTargets) {
        for (node, queue) in self.flow.values_mut() {
            node.prepare(renderer, scene, queue, target);
        }

        for (node, queue) in self.flow.values_mut() {
            node.draw(renderer, scene, queue, target);
        }
    }
}

pub trait RenderNode {
    /// Build the node.
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    );
    /// Prepare bind groups and other assets for rendering.
    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        target: &RenderTargets,
    );
    /// Draw meshes.
    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[RenderMesh],
        target: &RenderTargets,
    );
}

/// Prepares camera, lights and post process bind groups.
#[derive(Default)]
pub struct GeneralNode;

impl RenderNode for GeneralNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        _target: &RenderTargets,
    ) {
        scene.assets.common_layout = Some(renderer.device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
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
            },
        ));

        scene.assets.lights_layout = Some(renderer.device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
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
            },
        ));
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        let GpuScene {
            assets, original, ..
        } = scene;

        assets.directional_light_uniforms.clear();
        assets.point_light_uniforms.clear();
        assets.spot_light_uniforms.clear();

        for light in original.directional_lights.values() {
            assets.directional_light_uniforms.push(light);
        }

        for light in original.point_lights.values() {
            assets.point_light_uniforms.push(light);
        }

        for light in original.spot_lights.values() {
            assets.spot_light_uniforms.push(light);
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

        assets
            .camera_uniform
            .write::<GpuCamera>(&renderer.device, &renderer.queue);
        assets
            .scene_desc_uniform
            .write::<GpuSceneDesc>(&renderer.device, &renderer.queue);
        assets
            .directional_light_uniforms
            .write::<GpuDirectionalLight>(&renderer.device, &renderer.queue);
        assets
            .point_light_uniforms
            .write::<GpuPointLight>(&renderer.device, &renderer.queue);
        assets
            .spot_light_uniforms
            .write::<GpuSpotLight>(&renderer.device, &renderer.queue);

        let (
            Some(bf_camera),
            Some(bf_gpu_scene_desc),
            Some(bf_dir_lights),
            Some(bf_point_lights),
            Some(bf_spot_lights),
        ) = (
            assets.camera_uniform.entire_binding(),
            assets.scene_desc_uniform.entire_binding(),
            assets.directional_light_uniforms.entire_binding(),
            assets.point_light_uniforms.entire_binding(),
            assets.spot_light_uniforms.entire_binding(),
        )
        else {
            return;
        };

        assets.common_bind_group = Some(renderer.device.create_bind_group(&BindGroupDescriptor {
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

        assets.light_bind_group = Some(renderer.device.create_bind_group(&BindGroupDescriptor {
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

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTargets,
    ) {
    }
}

/// Added the post process related bing group layouts.
#[derive(Default)]
pub struct PostProcessGeneralNode;

impl RenderNode for PostProcessGeneralNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        _target: &RenderTargets,
    ) {
        scene.assets.material_layouts.insert(
            POST_PROCESS_COLOR_LAYOUT_UUID,
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
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

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTargets,
    ) {
    }
}

#[derive(Default)]
pub struct ImageFallbackNode;

impl RenderNode for ImageFallbackNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        _target: &RenderTargets,
    ) {
        scene.assets.textures.insert(
            DUMMY_2D_TEX,
            renderer.device.create_texture_with_data(
                &renderer.queue,
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

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTargets,
    ) {
    }
}
