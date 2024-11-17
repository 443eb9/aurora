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
        ShaderDefEnum,
    },
    util::ext::TypeIdAsUuid,
};
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Face, FragmentState, Limits, LoadOp, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, StencilState, StoreOp, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::{
    material::{PbrMaterial, PbrMaterialUniform},
    node::{shadow_mapping::SHADOW_MAPPING, DEPTH_PREPASS_TEXTURE, ENV_MAPPING, SSAO},
    shader_defs::{PbrDiffuse, PbrSpecular},
    texture,
};

pub const TONY_MC_MAPFACE_LUT: TextureId =
    TextureId(Uuid::from_u128(7949841653150346834163056985041356));

bitflags::bitflags! {
    #[derive(Default)]
    pub struct PbrNodeConfig: u32 {
        const SHADOW_MAPPING = 1 << 0;
        const ENVIRONMENT_MAPPING = 1 << 1;
        const SSAO = 1 << 2;
    }
}

#[derive(Default)]
pub struct PbrNode {
    pub diffuse: PbrDiffuse,
    pub specular: PbrSpecular,
    pub node_cfg: PbrNodeConfig,

    pub mat_uuid: MaterialTypeId,
    pub shadow_mapping_index: u32,
    pub env_mapping_index: u32,
    pub ssao_index: u32,
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

    fn require_renderer_limits(&self, limits: &mut Limits) {
        limits.max_bind_groups = limits.max_bind_groups.max(6);
    }

    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.extend([
            ("LUT_TEX_BINDING".to_string(), ShaderDefValue::UInt(4)),
            ("LUT_SAMPLER_BINDING".to_string(), ShaderDefValue::UInt(5)),
            self.diffuse.to_def(),
            self.specular.to_def(),
        ]);

        let mut bind_groups = 3;
        if self.node_cfg.contains(PbrNodeConfig::SHADOW_MAPPING) {
            shader_defs.insert(
                "SHADOW_MAPPING".to_string(),
                ShaderDefValue::UInt(bind_groups),
            );
            bind_groups += 1;
        }
        if self.node_cfg.contains(PbrNodeConfig::SSAO) {
            shader_defs.insert("SSAO".to_string(), ShaderDefValue::UInt(bind_groups));
            // bind_groups += 1;
        }
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[(
            &[
                include_str!("../shader/math.wgsl"),
                include_str!("../shader/hash.wgsl"),
                include_str!("../shader/common/common_type.wgsl"),
                include_str!("../shader/common/common_binding.wgsl"),
                include_str!("../shader/shadow/shadow_type.wgsl"),
                include_str!("../shader/shadow/shadow_mapping.wgsl"),
                include_str!("../shader/post_processing/ssao.wgsl"),
                include_str!("../shader/pbr/pbr_type.wgsl"),
                include_str!("../shader/pbr/pbr_binding.wgsl"),
                include_str!("../shader/pbr/pbr_function.wgsl"),
                include_str!("../shader/env_mapping/env_mapping_type.wgsl"),
                include_str!("../shader/env_mapping/env_mapping_binding.wgsl"),
                include_str!("../shader/env_mapping/env_mapping.wgsl"),
                include_str!("../shader/tonemapping.wgsl"),
                include_str!("../shader/pbr/pbr.wgsl"),
            ],
            include_str!("../shader/pbr/pbr.wgsl"),
        )])
    }

    fn create_pipelines(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        PipelineCreationContext {
            device,
            targets,
            shaders: shader,
            meshes,
            pipelines,
        }: PipelineCreationContext,
    ) {
        PbrMaterial::create_layout(device, assets);

        let (l_camera, l_lights, l_material) = (
            assets.common_layout.as_ref().unwrap(),
            assets.lights_layout.as_ref().unwrap(),
            &assets.material_layouts[&MaterialTypeId(TypeId::of::<PbrMaterial>().to_uuid())],
        );

        let mut bind_group_layouts = vec![l_camera, l_lights, l_material];
        if self.node_cfg.contains(PbrNodeConfig::SHADOW_MAPPING) {
            self.shadow_mapping_index = bind_group_layouts.len() as u32;
            bind_group_layouts.push(&assets.extra_layouts[&SHADOW_MAPPING.shadow_maps_layout]);
        }
        if self.node_cfg.contains(PbrNodeConfig::SSAO) {
            self.ssao_index = bind_group_layouts.len() as u32;
            bind_group_layouts.push(&assets.extra_layouts[&SSAO.ssao_layout]);
        }
        // if config.contains(PbrNodeConfig::ENVIRONMENT_MAPPING) {
        //     bind_group_layouts.push(&assets.extra_layouts[&ENV_MAPPING.env_mapping_layout]);
        // }

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_pipeline_layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        for mesh in meshes {
            let instance = &assets.meshes[&mesh.mesh.mesh];
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("pbr_pipeline"),
                layout: Some(&layout),
                cache: None,
                vertex: VertexState {
                    module: &shader[0],
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
                    module: &shader[0],
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
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(b_camera), Some(b_lights)) =
            (&assets.common_bind_group, &assets.light_bind_group)
        else {
            return;
        };

        let b_shadow_maps = self
            .node_cfg
            .contains(PbrNodeConfig::SHADOW_MAPPING)
            .then(|| &assets.extra_bind_groups[&SHADOW_MAPPING.shadow_maps_bind_group]);

        let b_env_mapping = self
            .node_cfg
            .contains(PbrNodeConfig::ENVIRONMENT_MAPPING)
            .then(|| &assets.extra_bind_groups[&ENV_MAPPING.env_mapping_bind_group]);

        let b_ssao = self
            .node_cfg
            .contains(PbrNodeConfig::SSAO)
            .then(|| &assets.extra_bind_groups[&SSAO.ssao_bind_group]);

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
                    view: &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_bind_group(0, b_camera, &[]);
            pass.set_bind_group(1, b_lights, &[]);
            if self.node_cfg.contains(PbrNodeConfig::SHADOW_MAPPING) {
                pass.set_bind_group(self.shadow_mapping_index, b_shadow_maps.unwrap(), &[]);
            }
            if self.node_cfg.contains(PbrNodeConfig::ENVIRONMENT_MAPPING) {
                pass.set_bind_group(self.env_mapping_index, b_env_mapping.unwrap(), &[]);
            }
            if self.node_cfg.contains(PbrNodeConfig::SSAO) {
                pass.set_bind_group(self.ssao_index, b_ssao.unwrap(), &[]);
            }

            for mesh in &node.meshes {
                let (Some(b_material), Some(instance), Some(pipeline)) = (
                    assets.material_bind_groups.get(&mesh.mesh.material),
                    assets.gpu_meshes.get(&mesh.mesh.mesh),
                    node.pipelines.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_pipeline(pipeline);
                pass.set_bind_group(2, b_material, &[mesh.offset.unwrap()]);
                pass.set_vertex_buffer(0, instance.vertex_buffer.slice(..));
                if let Some(indices) = &instance.index_buffer {
                    pass.set_index_buffer(indices.buffer.slice(..), indices.format);
                    pass.draw_indexed(0..indices.count, 0, 0..1);
                } else {
                    pass.draw(0..instance.vertices_count, 0..1);
                }
            }
        }

        queue.submit([encoder.finish()]);
    }
}
