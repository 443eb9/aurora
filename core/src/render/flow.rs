use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, BufferUsages, ShaderStages,
};

use crate::{
    render::{
        resource::{
            DynamicGpuBuffer, GpuCamera, GpuDirectionalLight, RenderTarget, CAMERA_UUID,
            DIR_LIGHT_UUID, LIGHTS_BIND_GROUP_UUID,
        },
        scene::GpuScene,
        ShaderData,
    },
    scene::entity::StaticMesh,
    WgpuRenderer,
};

#[derive(Default)]
pub struct RenderFlow {
    pub flow: IndexMap<Uuid, Box<dyn RenderNode>>,
    pub queue: Vec<StaticMesh>,
}

impl RenderFlow {
    #[inline]
    pub fn add<T: RenderNode + Default + 'static>(&mut self) -> Uuid {
        let uuid = Uuid::new_v4();
        self.flow.insert(uuid, Box::new(T::default()));
        uuid
    }

    #[inline]
    pub fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
    ) {
        for node in self.flow.values_mut() {
            node.build(renderer, scene, shader_defs.clone());
        }
    }

    #[inline]
    pub fn run(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, target: &RenderTarget) {
        for node in self.flow.values_mut() {
            node.prepare(renderer, scene, &self.queue);
            node.draw(renderer, scene, &self.queue, target);
        }
    }
}

pub trait RenderNode {
    /// Build the node.
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
    );
    /// Prepare bind groups and other assets for rendering.
    fn prepare(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, queue: &[StaticMesh]);
    /// Draw meshes.
    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[StaticMesh],
        target: &RenderTarget,
    );
}

/// Prepares camera, lights and materials.
#[derive(Default)]
pub struct GeneralNode;

impl RenderNode for GeneralNode {
    fn build(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
    ) {
    }

    fn prepare(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, queue: &[StaticMesh]) {
        let assets = &mut scene.assets;
        if !assets.layouts.contains_key(&CAMERA_UUID) {
            let l_camera = renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
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

            assets.layouts.insert(CAMERA_UUID, l_camera);
        }

        if !assets.layouts.contains_key(&LIGHTS_BIND_GROUP_UUID) {
            let l_lights = renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
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

            assets.layouts.insert(LIGHTS_BIND_GROUP_UUID, l_lights);
        }

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
                label: None,
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
                label: None,
                layout: &l_lights,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: bf_dir_lights,
                }],
            }),
        );

        queue
            .iter()
            .map(|m| m.material)
            .collect::<HashSet<_>>()
            .into_iter()
            .for_each(|uuid| {
                let Some(material) = scene.materials.get(&uuid) else {
                    return;
                };

                material.prepare(renderer, assets);
            });
    }

    fn draw(
        &self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[StaticMesh],
        _target: &RenderTarget,
    ) {
    }
}
