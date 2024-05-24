use std::{borrow::Cow, collections::HashMap};

use indexmap::IndexMap;
use naga_oil::compose::ShaderDefValue;

use crate::{
    pipeline::{DepthPassPipeline, PbrPipeline},
    render::{ComposableShader, OwnedRenderPassDescriptor, RenderTargets, Vertex},
    scene::render::GpuScene,
};

use wgpu::*;

#[derive(Default)]
pub struct AuroraRenderFlow {
    pub(crate) flow: IndexMap<String, Box<dyn for<'a> AuroraRenderNode<'a>>>,
}

impl AuroraRenderFlow {
    pub fn add(&mut self, label: String, node: Box<dyn for<'a> AuroraRenderNode<'a>>) {
        self.flow.insert(label, node);
    }

    pub fn build(&mut self, device: &Device, shader_defs: Option<HashMap<String, ShaderDefValue>>) {
        for node in self.flow.values_mut() {
            node.build(device, shader_defs.clone());
        }
    }

    pub fn prepare(&mut self, device: &Device, targets: &RenderTargets, scene: Option<&GpuScene>) {
        for node in self.flow.values_mut() {
            node.prepare(device, targets, scene);
        }
    }
}

pub trait AuroraRenderNode<'n>: Send + Sync {
    fn build(&mut self, device: &Device, shader_defs: Option<HashMap<String, ShaderDefValue>>);
    fn pipeline(&self) -> Option<&RenderPipeline>;
    fn describe_pass(&self, targets: &RenderTargets<'n>, desc: &mut OwnedRenderPassDescriptor<'n>);
    fn prepare(&mut self, device: &Device, targets: &'n RenderTargets, scene: Option<&'n GpuScene>);
    fn bind<'b>(&'b self, pass: &mut RenderPass<'b>, scene: Option<&'b GpuScene>);

    fn draw(&self, pass: &mut RenderPass, scene: Option<&GpuScene>) {
        for mesh in &scene
            .expect("GpuScene is required for default implementation.")
            .meshes
        {
            pass.set_vertex_buffer(0, unsafe {
                std::mem::transmute(mesh.vertex_buffer.slice(..))
            });
            pass.draw(0..mesh.vertex_count, 0..1);
        }
    }
}

pub struct PbrNode {
    pipeline: Option<PbrPipeline>,
    render_pipeline: Option<RenderPipeline>,

    target: TextureFormat,
    shader: ComposableShader,
}

impl PbrNode {
    pub fn new(target: TextureFormat) -> Self {
        Self {
            pipeline: None,
            render_pipeline: None,
            target,
            shader: ComposableShader::new(),
        }
    }
}

impl<'n> AuroraRenderNode<'n> for PbrNode {
    fn build(&mut self, device: &Device, shader_defs: Option<HashMap<String, ShaderDefValue>>) {
        self.pipeline = Some(PbrPipeline::new(device));

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pbr_shader"),
            source: ShaderSource::Naga(std::borrow::Cow::Owned(
                self.shader
                    .compose(
                        include_str!("shaders/pbr/pbr.wgsl"),
                        shader_defs.unwrap_or_default(),
                    )
                    .unwrap(),
            )),
        });

        self.render_pipeline = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pbr_pipeline"),
            layout: Some(&self.pipeline.as_ref().unwrap().pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(self.target.into())],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        }));
    }

    fn pipeline(&self) -> Option<&RenderPipeline> {
        self.render_pipeline.as_ref()
    }

    fn describe_pass(&self, targets: &RenderTargets<'n>, desc: &mut OwnedRenderPassDescriptor<'n>) {
        desc.color_attachments = Box::new([Some(RenderPassColorAttachment {
            view: &targets.color,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })]);
        desc.depth_stencil_attachment = Some(RenderPassDepthStencilAttachment {
            view: &targets
                .depth
                .expect("Depth target is required for PbrPipeline."),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        });
    }

    fn prepare(
        &mut self,
        _device: &Device,
        _targets: &'n RenderTargets,
        _scene: Option<&GpuScene>,
    ) {
    }

    fn bind<'b>(&'b self, pass: &mut RenderPass<'b>, scene: Option<&'b GpuScene>) {
        let scene = scene.unwrap();
        let (Some(b_camera), Some(b_lights)) =
            (&scene.b_camera.bind_group, &scene.b_lights.bind_group)
        else {
            log::error!("Scene haven't written yet");
            return;
        };

        pass.set_bind_group(0, b_camera, &[]);
        pass.set_bind_group(1, b_lights, &[]);
    }
}

pub struct DepthPassNode {
    pub bind_group: Option<BindGroup>,
    pub render_pipeline: Option<RenderPipeline>,
    pub target: TextureFormat,
    pub pipeline: Option<DepthPassPipeline>,
    pub shader: ComposableShader,
}

impl DepthPassNode {
    pub fn new(target: TextureFormat) -> Self {
        let mut shader = ComposableShader::new();
        shader
            .add_shader(include_str!("shaders/fullscreen.wgsl"))
            .unwrap();

        Self {
            bind_group: None,
            render_pipeline: None,
            target,
            pipeline: None,
            shader,
        }
    }
}

impl<'n> AuroraRenderNode<'n> for DepthPassNode {
    fn build(&mut self, device: &Device, _shader_defs: Option<HashMap<String, ShaderDefValue>>) {
        self.pipeline = Some(DepthPassPipeline::new(device));

        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("depth_pass_fragment"),
            source: ShaderSource::Naga(Cow::Owned(
                self.shader
                    .compose(include_str!("shaders/depth_pass.wgsl"), Default::default())
                    .unwrap(),
            )),
        });

        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Naga(Cow::Owned(
                ComposableShader::new()
                    .compose(include_str!("shaders/fullscreen.wgsl"), Default::default())
                    .unwrap(),
            )),
        });

        self.render_pipeline = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("depth_pass_pipeline"),
            layout: Some(&self.pipeline.as_ref().unwrap().pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: "fragment",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(self.target.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        }));
    }

    fn pipeline(&self) -> Option<&RenderPipeline> {
        self.render_pipeline.as_ref()
    }

    fn describe_pass(&self, targets: &RenderTargets<'n>, desc: &mut OwnedRenderPassDescriptor<'n>) {
        desc.color_attachments = Box::new([Some(RenderPassColorAttachment {
            view: targets.color,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })]);
    }

    fn prepare(&mut self, device: &Device, targets: &'n RenderTargets, _scene: Option<&GpuScene>) {
        let pipeline = self.pipeline.as_ref().expect("Pipeline not built");

        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(targets.depth.as_ref().unwrap()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        }));
    }

    fn bind<'b>(&'b self, pass: &mut RenderPass<'b>, _scene: Option<&'b GpuScene>) {
        pass.set_bind_group(0, &self.bind_group.as_ref().unwrap(), &[]);
    }

    fn draw(&self, pass: &mut RenderPass, _scene: Option<&GpuScene>) {
        pass.draw(0..3, 0..1);
    }
}
