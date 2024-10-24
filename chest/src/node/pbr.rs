use std::{
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
};

use aurora_core::{
    render::{
        flow::RenderNode,
        mesh::CreateBindGroupLayout,
        resource::{DynamicGpuBuffer, RenderMesh, RenderTargets, Vertex},
        scene::{GpuScene, MaterialTypeId, TextureId},
    },
    util::ext::TypeIdAsUuid,
    WgpuRenderer,
};
use naga_oil::compose::{Composer, NagaModuleDescriptor, ShaderDefValue};
use uuid::Uuid;
use wgpu::{
    vertex_attr_array, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, Face,
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilState, StoreOp,
    VertexBufferLayout, VertexState, VertexStepMode,
};

use crate::{
    material::{PbrMaterial, PbrMaterialUniform},
    node::shadow_mapping::SHADOW_MAPPING,
    texture, util,
};

pub const TONY_MC_MAPFACE_LUT: TextureId =
    TextureId(Uuid::from_u128(7949841653150346834163056985041356));

#[derive(Default)]
pub struct PbrNode {
    mat_uuid: MaterialTypeId,
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for PbrNode {
    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.extend([
            ("LUT_TEX_BINDING".to_string(), ShaderDefValue::UInt(4)),
            ("LUT_SAMPLER_BINDING".to_string(), ShaderDefValue::UInt(5)),
        ]);
    }

    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: HashMap<String, ShaderDefValue>,
        target: &RenderTargets,
    ) {
        scene.assets.textures.insert(
            TONY_MC_MAPFACE_LUT,
            texture::load_dds_texture(renderer, "chest/assets/luts/tony_mc_mapface.dds"),
        );

        self.mat_uuid = MaterialTypeId(TypeId::of::<PbrMaterial>().to_uuid());
        PbrMaterial::create_layout(renderer, &mut scene.assets);

        let Some(l_shadow_map) = scene
            .assets
            .extra_layouts
            .get(&SHADOW_MAPPING.shadow_maps_layout)
        else {
            return;
        };

        let (l_camera, l_lights, l_material) = (
            scene.assets.common_layout.as_ref().unwrap(),
            scene.assets.lights_layout.as_ref().unwrap(),
            &scene.assets.material_layouts[&self.mat_uuid],
        );

        let mut composer = Composer::default();
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/math.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/common/common_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/common/common_binding.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/shadow/shadow_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/shadow/shadow_mapping.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/pbr/pbr_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/pbr/pbr_binding.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/pbr/pbr_function.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/tonemapping.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/pbr/pbr.wgsl"),
            shader_defs.clone(),
        );

        let shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("../shader/pbr/pbr.wgsl"),
                shader_defs,
                ..Default::default()
            })
            .unwrap();

        let module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("pbr_shader"),
                source: ShaderSource::Naga(Cow::Owned(shader)),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pbr_pipeline_layout"),
                bind_group_layouts: &[&l_camera, &l_lights, &l_material, &l_shadow_map],
                push_constant_ranges: &[],
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("pbr_pipeline"),
                    layout: Some(&layout),
                    cache: None,
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                // Position
                                0 => Float32x3,
                                // Normal
                                1 => Float32x3,
                                // UV
                                2 => Float32x2,
                                // Tangent
                                3 => Float32x4,
                            ],
                        }],
                    },
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: target.color_format,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    depth_stencil: Some(DepthStencilState {
                        format: target.depth_format.unwrap(),
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
                }),
        );
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        match scene.assets.material_uniforms.entry(self.mat_uuid) {
            Entry::Occupied(mut e) => e.get_mut().clear(),
            Entry::Vacant(e) => {
                e.insert(DynamicGpuBuffer::new(BufferUsages::UNIFORM));
            }
        }

        queue
            .iter_mut()
            .filter_map(|sm| {
                scene
                    .original
                    .materials
                    .get(&sm.mesh.material)
                    .map(|m| (m, sm))
            })
            .for_each(|(material, mesh)| {
                mesh.offset = Some(material.prepare(renderer, &mut scene.assets));
            });

        scene
            .assets
            .material_uniforms
            .get_mut(&self.mat_uuid)
            .unwrap()
            .write::<PbrMaterialUniform>(&renderer.device, &renderer.queue);

        queue
            .iter_mut()
            .filter_map(|sm| {
                scene
                    .original
                    .materials
                    .get(&sm.mesh.material)
                    .map(|m| (m, sm))
            })
            .for_each(|(material, mesh)| {
                material.create_bind_group(renderer, &mut scene.assets, mesh.mesh.material);
            });
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let assets = &scene.assets;

        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(b_camera), Some(b_lights), Some(b_shadow_maps)) = (
            &assets.common_bind_group,
            &assets.light_bind_group,
            assets
                .extra_bind_groups
                .get(&SHADOW_MAPPING.shadow_maps_bind_group),
        ) else {
            return;
        };

        let Some(pipeline) = &self.pipeline else {
            return;
        };

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pbr_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: target.depth.as_ref().unwrap(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, b_camera, &[]);
            pass.set_bind_group(1, b_lights, &[]);
            pass.set_bind_group(3, b_shadow_maps, &[]);

            for mesh in queue {
                let (Some(b_material), Some((vertices, count))) = (
                    assets.material_bind_groups.get(&mesh.mesh.material),
                    assets.vertex_buffers.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_bind_group(2, b_material, &[mesh.offset.unwrap()]);
                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..*count, 0..1);
            }
        }

        renderer.queue.submit([encoder.finish()]);
    }
}
