use std::{
    any::TypeId,
    collections::{hash_map::Entry, HashMap},
};

use aurora_core::{
    render::{
        flow::{PipelineCreationContext, RenderContext, RenderNode},
        mesh::CreateBindGroupLayout,
        resource::DynamicGpuBuffer,
        scene::{GpuScene, MaterialTypeId, TextureId},
    },
    util::ext::TypeIdAsUuid,
};
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Face, FragmentState, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, StencilState, StoreOp, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::{
    material::{PbrMaterial, PbrMaterialUniform},
    node::shadow_mapping::SHADOW_MAPPING,
    texture,
};

pub const TONY_MC_MAPFACE_LUT: TextureId =
    TextureId(Uuid::from_u128(7949841653150346834163056985041356));

#[derive(Default)]
pub struct PbrNode {
    mat_uuid: MaterialTypeId,
}

impl RenderNode for PbrNode {
    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        Some(&[
            VertexFormat::Float32x3,
            VertexFormat::Float32x3,
            VertexFormat::Float32x2,
            VertexFormat::Float32x4,
        ])
    }

    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.extend([
            ("LUT_TEX_BINDING".to_string(), ShaderDefValue::UInt(4)),
            ("LUT_SAMPLER_BINDING".to_string(), ShaderDefValue::UInt(5)),
        ]);
    }

    fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
        Some((
            &[
                include_str!("../shader/math.wgsl"),
                include_str!("../shader/common/common_type.wgsl"),
                include_str!("../shader/common/common_binding.wgsl"),
                include_str!("../shader/shadow/shadow_type.wgsl"),
                include_str!("../shader/shadow/shadow_mapping.wgsl"),
                include_str!("../shader/pbr/pbr_type.wgsl"),
                include_str!("../shader/pbr/pbr_binding.wgsl"),
                include_str!("../shader/pbr/pbr_function.wgsl"),
                include_str!("../shader/tonemapping.wgsl"),
                include_str!("../shader/pbr/pbr.wgsl"),
            ],
            include_str!("../shader/pbr/pbr.wgsl"),
        ))
    }

    fn create_pipelines(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        PipelineCreationContext {
            device,
            targets,
            shader,
            meshes,
            pipelines,
        }: PipelineCreationContext,
    ) {
        PbrMaterial::create_layout(device, assets);

        let Some(l_shadow_map) = assets.extra_layouts.get(&SHADOW_MAPPING.shadow_maps_layout)
        else {
            return;
        };

        let (l_camera, l_lights, l_material) = (
            assets.common_layout.as_ref().unwrap(),
            assets.lights_layout.as_ref().unwrap(),
            &assets.material_layouts[&MaterialTypeId(TypeId::of::<PbrMaterial>().to_uuid())],
        );

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_pipeline_layout"),
            bind_group_layouts: &[&l_camera, &l_lights, &l_material, &l_shadow_map],
            push_constant_ranges: &[],
        });

        for mesh in meshes {
            if pipelines.contains_key(&mesh.mesh.mesh) {
                continue;
            }

            let instance = &assets.meshes[&mesh.mesh.mesh];
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("pbr_pipeline"),
                layout: Some(&layout),
                cache: None,
                vertex: VertexState {
                    module: shader,
                    entry_point: "vertex",
                    compilation_options: PipelineCompilationOptions::default(),
                    buffers: &[VertexBufferLayout {
                        array_stride: instance.vertex_stride(),
                        step_mode: VertexStepMode::Vertex,
                        attributes: &instance.vertex_attributes(),
                    }],
                },
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: "fragment",
                    compilation_options: PipelineCompilationOptions::default(),
                    targets: &[Some(ColorTargetState {
                        format: targets.color_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                depth_stencil: Some(DepthStencilState {
                    format: targets.depth_format.unwrap(),
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                multiview: None,
            });
            pipelines.insert(mesh.mesh.mesh, pipeline);
        }
    }

    fn build(&mut self, scene: &mut GpuScene, RenderContext { device, queue, .. }: RenderContext) {
        scene.assets.textures.insert(
            TONY_MC_MAPFACE_LUT,
            texture::load_dds_texture(device, queue, "chest/assets/luts/tony_mc_mapface.dds"),
        );
        self.mat_uuid = MaterialTypeId(TypeId::of::<PbrMaterial>().to_uuid());
    }

    fn prepare(
        &mut self,
        scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            ..
        }: RenderContext,
    ) {
        match scene.assets.material_uniforms.entry(self.mat_uuid) {
            Entry::Occupied(mut e) => e.get_mut().clear(),
            Entry::Vacant(e) => {
                e.insert(DynamicGpuBuffer::new(BufferUsages::UNIFORM));
            }
        }

        node.meshes
            .iter_mut()
            .filter_map(|rm| {
                scene
                    .original
                    .materials
                    .get(&rm.mesh.material)
                    .map(|m| (m, rm))
            })
            .for_each(|(material, mesh)| {
                mesh.offset = Some(material.prepare(device, &mut scene.assets));
            });

        scene
            .assets
            .material_uniforms
            .get_mut(&self.mat_uuid)
            .unwrap()
            .write::<PbrMaterialUniform>(&device, &queue);

        node.meshes
            .iter_mut()
            .filter_map(|rm| {
                scene
                    .original
                    .materials
                    .get(&rm.mesh.material)
                    .map(|m| (m, rm))
            })
            .for_each(|(material, mesh)| {
                material.create_bind_group(device, &mut scene.assets, mesh.mesh.material);
            });
    }

    fn draw(
        &self,
        scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
        let assets = &scene.assets;

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(b_camera), Some(b_lights), Some(b_shadow_maps)) = (
            &assets.common_bind_group,
            &assets.light_bind_group,
            assets
                .extra_bind_groups
                .get(&SHADOW_MAPPING.shadow_maps_bind_group),
        ) else {
            return;
        };

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pbr_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: targets.depth.as_ref().unwrap(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_bind_group(0, b_camera, &[]);
            pass.set_bind_group(1, b_lights, &[]);
            pass.set_bind_group(3, b_shadow_maps, &[]);

            for mesh in &node.meshes {
                let (Some(b_material), Some(instance), Some(pipeline)) = (
                    assets.material_bind_groups.get(&mesh.mesh.material),
                    assets.meshes.get(&mesh.mesh.mesh),
                    node.pipelines.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_pipeline(pipeline);
                pass.set_bind_group(2, b_material, &[mesh.offset.unwrap()]);
                pass.set_vertex_buffer(0, instance.create_buffer(device).unwrap().slice(..));
                pass.draw(0..instance.vertices_count() as u32, 0..1);
            }
        }

        queue.submit([encoder.finish()]);
    }
}
