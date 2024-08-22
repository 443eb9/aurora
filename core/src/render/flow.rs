use std::collections::HashMap;

use dyn_clone::DynClone;
use encase::ShaderType;
use glam::Vec3;
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
        resource::{
            GpuAreaLight, GpuCamera, GpuDirectionalLight, GpuPointLight, GpuSpotLight, RenderMesh,
            RenderTarget, AREA_LIGHT_UUID, AREA_LIGHT_VERTICES_UUID, CAMERA_UUID, DIR_LIGHT_UUID,
            DUMMY_2D_TEX, LIGHTS_BIND_GROUP_UUID, POINT_LIGHT_UUID, POST_PROCESS_COLOR_LAYOUT_UUID,
            POST_PROCESS_DEPTH_LAYOUT_UUID, SPOT_LIGHT_UUID,
        },
        scene::GpuScene,
    },
    scene::entity::StaticMesh,
    WgpuRenderer,
};

#[derive(Default)]
pub struct RenderFlow {
    pub flow: IndexMap<Uuid, (Box<dyn RenderNode>, Vec<RenderMesh>)>,
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
        target: &RenderTarget,
    ) {
        for (node, _) in self.flow.values_mut() {
            node.build(renderer, scene, shader_defs.clone(), target);
        }
    }

    #[inline]
    pub fn run(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, target: &RenderTarget) {
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
        target: &RenderTarget,
    );
    /// Prepare bind groups and other assets for rendering.
    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        target: &RenderTarget,
    );
    /// Draw meshes.
    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        queue: &[RenderMesh],
        target: &RenderTarget,
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
        _target: &RenderTarget,
    ) {
        if !scene.assets.layouts.contains_key(&CAMERA_UUID) {
            let l_camera = renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("camera_layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(GpuCamera::min_size()),
                        },
                        count: None,
                    }],
                });

            scene.assets.layouts.insert(CAMERA_UUID, l_camera);
        }

        if !scene.assets.layouts.contains_key(&LIGHTS_BIND_GROUP_UUID) {
            let l_lights = renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                        // Area
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(GpuAreaLight::min_size()),
                            },
                            count: None,
                        },
                        // Area vertices
                        BindGroupLayoutEntry {
                            binding: 4,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(Vec3::min_size()),
                            },
                            count: None,
                        },
                    ],
                });

            scene
                .assets
                .layouts
                .insert(LIGHTS_BIND_GROUP_UUID, l_lights);
        }
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
        let assets = &mut scene.assets;

        let (
            Some(bf_camera),
            Some(bf_dir_lights),
            Some(bf_point_lights),
            Some(bf_spot_lights),
            Some(bf_area_lights),
            Some(bf_area_light_vertices),
        ) = (
            assets.buffers[&CAMERA_UUID].entire_binding(),
            assets.buffers[&DIR_LIGHT_UUID].entire_binding(),
            assets.buffers[&POINT_LIGHT_UUID].entire_binding(),
            assets.buffers[&SPOT_LIGHT_UUID].entire_binding(),
            assets.buffers[&AREA_LIGHT_UUID].entire_binding(),
            assets.buffers[&AREA_LIGHT_VERTICES_UUID].entire_binding(),
        )
        else {
            return;
        };

        let l_camera = &assets.layouts[&CAMERA_UUID];
        assets.bind_groups.insert(
            CAMERA_UUID,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("camera_bind_group"),
                layout: &l_camera,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: bf_camera,
                }],
            }),
        );

        let l_lights = &assets.layouts[&LIGHTS_BIND_GROUP_UUID];
        assets.bind_groups.insert(
            LIGHTS_BIND_GROUP_UUID,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("lights_bind_group"),
                layout: &l_lights,
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
                    BindGroupEntry {
                        binding: 3,
                        resource: bf_area_lights,
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: bf_area_light_vertices,
                    },
                ],
            }),
        );
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTarget,
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
        _target: &RenderTarget,
    ) {
        if !scene
            .assets
            .layouts
            .contains_key(&POST_PROCESS_COLOR_LAYOUT_UUID)
        {
            scene.assets.layouts.insert(
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
        }

        if !scene
            .assets
            .layouts
            .contains_key(&POST_PROCESS_DEPTH_LAYOUT_UUID)
        {
            scene.assets.layouts.insert(
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
    }

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTarget,
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
        _target: &RenderTarget,
    ) {
        if !scene.assets.textures.contains_key(&DUMMY_2D_TEX) {
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
    }

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _queue: &[RenderMesh],
        _target: &RenderTarget,
    ) {
    }
}
