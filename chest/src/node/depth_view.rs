use std::{borrow::Cow, collections::HashMap};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{RenderMesh, RenderTargets, POST_PROCESS_DEPTH_LAYOUT_UUID},
        scene::GpuScene,
    },
    WgpuRenderer,
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue, ShaderLanguage,
    ShaderType,
};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, FilterMode, FragmentState, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, StoreOp, TextureFormat,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

use crate::node::shadow_mapping::SHADOW_MAPPING;

#[derive(Default)]
pub struct DepthViewNode {
    pipeline: Option<RenderPipeline>,
    sampler: Option<Sampler>,
}

impl RenderNode for DepthViewNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _shader_defs: HashMap<String, ShaderDefValue>,
        target: &RenderTargets,
    ) {
        let Some(l_post_process) = scene
            .assets
            .material_layouts
            .get(&POST_PROCESS_DEPTH_LAYOUT_UUID)
        else {
            return;
        };

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("depth_view_pipeline_layout"),
                bind_group_layouts: &[l_post_process],
                push_constant_ranges: &[],
            });

        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("../shader/fullscreen.wgsl"),
                file_path: "",
                language: ShaderLanguage::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
                as_name: None,
            })
            .unwrap();

        let vert_shader = Composer::default()
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("../shader/fullscreen.wgsl"),
                file_path: "",
                shader_type: ShaderType::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
            })
            .unwrap();
        let vert_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("fullscreen_vertex_shader"),
                source: ShaderSource::Naga(Cow::Owned(vert_shader)),
            });

        let frag_shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("../shader/depth_view.wgsl"),
                file_path: "",
                shader_type: ShaderType::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
            })
            .unwrap();
        let frag_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("depth_view_shader"),
                source: ShaderSource::Naga(Cow::Owned(frag_shader)),
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("depth_view_pipeline"),
                    layout: Some(&layout),
                    cache: None,
                    vertex: VertexState {
                        module: &vert_module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(FragmentState {
                        module: &frag_module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: target.color_format,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                }),
        );

        self.sampler = Some(renderer.device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }));
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
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(pipeline), Some(l_screen), Some(sampler)) = (
            &self.pipeline,
            scene
                .assets
                .material_layouts
                .get(&POST_PROCESS_DEPTH_LAYOUT_UUID),
            &self.sampler,
        ) else {
            return;
        };

        // As the targets changes every frame, we need to create the bind group for each frame.
        let b_screen = renderer.device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen_bind_group"),
            layout: l_screen,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &scene.assets.textures[&SHADOW_MAPPING.directional_shadow_map].create_view(
                            &TextureViewDescriptor {
                                format: Some(TextureFormat::Depth32Float),
                                dimension: Some(TextureViewDimension::D2),
                                base_array_layer: 0,
                                array_layer_count: Some(1),
                                ..Default::default()
                            },
                        ),
                        // target.depth.as_ref().unwrap(),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("depth_view_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &b_screen, &[]);
            pass.draw(0..3, 0..1);
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}
