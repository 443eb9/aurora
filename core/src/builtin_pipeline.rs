use std::collections::HashMap;

use naga_oil::compose::ShaderDefValue;
use wgpu::*;

use crate::{
    render::{
        ComposableShader, OwnedBindGroups, OwnedRenderPassDescriptor, RenderTargets, ShaderData,
        Vertex,
    },
    scene::render::{
        entity::{GpuCamera, GpuDirectionalLight},
        GpuScene,
    },
};

pub trait AuroraPipeline<'a> {
    fn build(&mut self, device: &Device, shader_defs: HashMap<String, ShaderDefValue>);
    fn cache(&self) -> Option<&RenderPipeline>;
    fn create_pass(&'a self, targets: RenderTargets<'a>) -> OwnedRenderPassDescriptor;

    fn bind(&'a self, scene: Option<&'a GpuScene>) -> Option<OwnedBindGroups> {
        let scene = scene.expect("GpuScene is required for default implementation.");
        let (Some(b_camera), Some(b_lights)) =
            (&scene.b_camera.bind_group, &scene.b_lights.bind_group)
        else {
            log::error!("Scene haven't written yet");
            return None;
        };

        Some(OwnedBindGroups {
            value: vec![(b_camera, None), (b_lights, None)],
        })
    }

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

pub struct PbrPipeline<'a> {
    pub camera_layout: BindGroupLayout,
    pub lights_layout: BindGroupLayout,

    pipeline_layout: PipelineLayout,
    target: TextureFormat,
    shader: ComposableShader<'a>,

    cache: Option<RenderPipeline>,
}

impl<'a> AuroraPipeline<'a> for PbrPipeline<'a> {
    fn build(&mut self, device: &Device, shader_defs: HashMap<String, ShaderDefValue>) {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pbr_shader"),
            source: ShaderSource::Naga(std::borrow::Cow::Owned(
                self.shader.compose(shader_defs).unwrap(),
            )),
        });

        self.cache = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pbr_pipeline"),
            layout: Some(&self.pipeline_layout),
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
            primitive: PrimitiveState::default(),
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

    fn cache(&self) -> Option<&RenderPipeline> {
        self.cache.as_ref()
    }

    fn create_pass(&'a self, targets: RenderTargets<'a>) -> OwnedRenderPassDescriptor {
        OwnedRenderPassDescriptor {
            label: None,
            color_attachments: Box::new([Some(RenderPassColorAttachment {
                view: &targets.color,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })]),
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets
                    .depth
                    .expect("Depth target is required for PbrPipeline."),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.),
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }
    }
}

impl<'a> PbrPipeline<'a> {
    pub fn new(device: &Device, target: TextureFormat) -> Self {
        let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pbr_camera_layout"),
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

        let lights_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pbr_lights_layout"),
            entries: &[
                // Directional
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: GpuDirectionalLight::min_binding_size(),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_pipeline_layout"),
            bind_group_layouts: &[&camera_layout, &lights_layout],
            push_constant_ranges: &[],
        });

        let shader = ComposableShader::new(include_str!("shaders/pbr/pbr.wgsl"), "pbr.wgsl");

        Self {
            camera_layout,
            lights_layout,

            pipeline_layout,
            target,
            shader,

            cache: None,
        }
    }
}

pub struct DepthPassPipeline {
    pub pipeline: RenderPipeline,
    pub layout: BindGroupLayout,
}

impl<'a> AuroraPipeline<'a> for DepthPassPipeline {
    fn build(&mut self, device: &Device, shader_defs: HashMap<String, ShaderDefValue>) {}

    fn cache(&self) -> Option<&RenderPipeline> {
        Some(&self.pipeline)
    }

    fn create_pass(&self, targets: RenderTargets) -> OwnedRenderPassDescriptor {
        todo!()
    }

    fn bind(&self, scene: Option<&GpuScene>) -> Option<OwnedBindGroups> {
        todo!()
    }

    fn draw(&self, pass: &mut RenderPass, scene: Option<&GpuScene>) {
        todo!()
    }
}

impl DepthPassPipeline {
    pub fn new(device: &Device, target: TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("depth_pass_layout"),
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
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("depth_pass_pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let vertex_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("depth_pass_vertex"),
            source: ShaderSource::Wgsl(include_str!("shaders/fullscreen.wgsl").into()),
        });

        let fragment_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("depth_pass_fragment"),
            source: ShaderSource::Wgsl(include_str!("shaders/depth_pass.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("depth_pass_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_module,
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fragment_module,
                entry_point: "fragment",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(target.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self { pipeline, layout }
    }
}
