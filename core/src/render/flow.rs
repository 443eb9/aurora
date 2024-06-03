use std::collections::HashMap;

use indexmap::IndexMap;
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, SamplerBindingType, ShaderStages, TextureSampleType,
    TextureViewDimension,
};

use crate::{
    render::{
        resource::{
            GpuCamera, GpuDirectionalLight, RenderMesh, RenderTarget, CAMERA_UUID, DIR_LIGHT_UUID,
            LIGHTS_BIND_GROUP_UUID, POST_PROCESS_COLOR_LAYOUT_UUID, POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::GpuScene,
        ShaderData,
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
                            min_binding_size: GpuCamera::min_binding_size(),
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
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: GpuDirectionalLight::min_binding_size(),
                        },
                        count: None,
                    }],
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

        let (Some(bf_camera), Some(bf_dir_lights)) = (
            assets.buffers[&CAMERA_UUID].binding(),
            assets.buffers[&DIR_LIGHT_UUID].binding(),
        ) else {
            return;
        };

        let l_camera = assets.layouts.get(&CAMERA_UUID).unwrap();
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

        let l_lights = assets.layouts.get(&LIGHTS_BIND_GROUP_UUID).unwrap();
        assets.bind_groups.insert(
            LIGHTS_BIND_GROUP_UUID,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("lights_bind_group"),
                layout: &l_lights,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: bf_dir_lights,
                }],
            }),
        );
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
<<<<<<< Updated upstream
        _scene: &mut GpuScene,
=======
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
>>>>>>> Stashed changes
        _queue: &[RenderMesh],
        _target: &RenderTarget,
    ) {
    }
}
