use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use nworlds_host::RenderSink;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::camera::Camera;
use crate::engine_integration::{VoxelFrame, VoxelRenderOutput};
use crate::world::{VoxelPosition, VoxelState};

const DEPTH_FORMAT: ::wgpu::TextureFormat = ::wgpu::TextureFormat::Depth24Plus;

const SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [::wgpu::VertexAttribute; 2] =
        ::wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const fn layout() -> ::wgpu::VertexBufferLayout<'static> {
        ::wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Self>() as ::wgpu::BufferAddress,
            step_mode: ::wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

#[derive(Debug)]
pub enum RenderError {
    Surface(String),
    AdapterUnavailable,
    Device(String),
    SurfaceFormatUnavailable,
}

impl core::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Surface(error) => write!(formatter, "could not create wgpu surface: {error}"),
            Self::AdapterUnavailable => formatter.write_str("no compatible wgpu adapter found"),
            Self::Device(error) => write!(formatter, "could not request wgpu device: {error}"),
            Self::SurfaceFormatUnavailable => {
                formatter.write_str("the wgpu surface reported no supported formats")
            }
        }
    }
}

impl std::error::Error for RenderError {}

pub struct VoxelRenderSink {
    window: Arc<Window>,
    surface: ::wgpu::Surface<'static>,
    device: ::wgpu::Device,
    queue: ::wgpu::Queue,
    config: ::wgpu::SurfaceConfiguration,
    depth_view: ::wgpu::TextureView,
    pipeline: ::wgpu::RenderPipeline,
    uniform_buffer: ::wgpu::Buffer,
    uniform_bind_group: ::wgpu::BindGroup,
    camera: Camera,
}

impl VoxelRenderSink {
    pub async fn new(window: Arc<Window>) -> Result<Self, RenderError> {
        let instance = ::wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| RenderError::Surface(error.to_string()))?;
        let adapter = instance
            .request_adapter(&::wgpu::RequestAdapterOptions {
                power_preference: ::wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|_| RenderError::AdapterUnavailable)?;
        let (device, queue) = adapter
            .request_device(&::wgpu::DeviceDescriptor {
                label: Some("voxel-device"),
                required_features: ::wgpu::Features::empty(),
                required_limits: ::wgpu::Limits::default(),
                experimental_features: ::wgpu::ExperimentalFeatures::disabled(),
                memory_hints: ::wgpu::MemoryHints::Performance,
                trace: ::wgpu::Trace::Off,
            })
            .await
            .map_err(|error| RenderError::Device(error.to_string()))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(::wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(RenderError::SurfaceFormatUnavailable)?;
        let size = window.inner_size();
        let config = surface_config(&capabilities, format, size);
        surface.configure(&device, &config);

        let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
            label: Some("voxel-shader"),
            source: ::wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_group_layout =
            device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
                label: Some("voxel-uniform-layout"),
                entries: &[::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ::wgpu::ShaderStages::VERTEX,
                    ty: ::wgpu::BindingType::Buffer {
                        ty: ::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let uniform_buffer = ::wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &::wgpu::util::BufferInitDescriptor {
                label: Some("voxel-camera-uniforms"),
                contents: bytemuck::bytes_of(&Uniforms {
                    view_proj: Camera::default().view_projection(),
                }),
                usage: ::wgpu::BufferUsages::UNIFORM | ::wgpu::BufferUsages::COPY_DST,
            },
        );
        let uniform_bind_group = device.create_bind_group(&::wgpu::BindGroupDescriptor {
            label: Some("voxel-uniform-bind-group"),
            layout: &bind_group_layout,
            entries: &[::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
            label: Some("voxel-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
            label: Some("voxel-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: ::wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::layout())],
                compilation_options: ::wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(::wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(::wgpu::ColorTargetState {
                    format,
                    blend: Some(::wgpu::BlendState::REPLACE),
                    write_mask: ::wgpu::ColorWrites::ALL,
                })],
                compilation_options: ::wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: ::wgpu::PrimitiveState::default(),
            depth_stencil: Some(::wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(::wgpu::CompareFunction::Less),
                stencil: ::wgpu::StencilState::default(),
                bias: ::wgpu::DepthBiasState::default(),
            }),
            multisample: ::wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mut camera = Camera::default();
        camera.set_aspect(config.width as f32 / config.height as f32);
        let depth_view = depth_view(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            camera,
        })
    }

    pub fn pick(&self, cursor: (f64, f64), state: &VoxelState) -> Option<VoxelPosition> {
        let size = self.window.inner_size();
        self.camera.pick(
            cursor.0 as f32,
            cursor.1 as f32,
            size.width as f32,
            size.height as f32,
            state,
        )
    }

    fn configure_for_current_window_size(&mut self) -> bool {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return false;
        }
        if self.config.width != size.width || self.config.height != size.height {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_view = depth_view(&self.device, &self.config);
            self.camera
                .set_aspect(size.width as f32 / size.height as f32);
        }
        true
    }
}

impl RenderSink<VoxelFrame> for VoxelRenderSink {
    fn submit(&mut self, frame: VoxelFrame) {
        if !self.configure_for_current_window_size() {
            return;
        }

        let view_proj = self.camera.view_projection();
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms { view_proj }),
        );
        let vertices = vertices(frame.payload());
        let vertex_buffer = ::wgpu::util::DeviceExt::create_buffer_init(
            &self.device,
            &::wgpu::util::BufferInitDescriptor {
                label: Some("voxel-vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: ::wgpu::BufferUsages::VERTEX,
            },
        );
        let output = match self.surface.get_current_texture() {
            ::wgpu::CurrentSurfaceTexture::Success(output)
            | ::wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            ::wgpu::CurrentSurfaceTexture::Lost | ::wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            ::wgpu::CurrentSurfaceTexture::Timeout
            | ::wgpu::CurrentSurfaceTexture::Occluded
            | ::wgpu::CurrentSurfaceTexture::Validation => return,
        };
        let view = output
            .texture
            .create_view(&::wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&::wgpu::CommandEncoderDescriptor {
                label: Some("voxel-render-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
                label: Some("voxel-render-pass"),
                color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: ::wgpu::Operations {
                        load: ::wgpu::LoadOp::Clear(::wgpu::Color {
                            r: 0.025,
                            g: 0.055,
                            b: 0.075,
                            a: 1.0,
                        }),
                        store: ::wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(::wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(::wgpu::Operations {
                        load: ::wgpu::LoadOp::Clear(1.0),
                        store: ::wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);
    }
}

fn surface_config(
    capabilities: &::wgpu::SurfaceCapabilities,
    format: ::wgpu::TextureFormat,
    size: PhysicalSize<u32>,
) -> ::wgpu::SurfaceConfiguration {
    ::wgpu::SurfaceConfiguration {
        usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        color_space: ::wgpu::SurfaceColorSpace::Auto,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == ::wgpu::PresentMode::Fifo)
            .unwrap_or(::wgpu::PresentMode::AutoVsync),
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

fn depth_view(
    device: &::wgpu::Device,
    config: &::wgpu::SurfaceConfiguration,
) -> ::wgpu::TextureView {
    device
        .create_texture(&::wgpu::TextureDescriptor {
            label: Some("voxel-depth"),
            size: ::wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: ::wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&::wgpu::TextureViewDescriptor::default())
}

fn vertices(output: &VoxelRenderOutput) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(output.voxels().len() * 36);
    for voxel in output.voxels() {
        let position = voxel.position();
        let scale = output.scale();
        let min = [
            position.x() as f32 * scale,
            position.y() as f32 * scale,
            position.z() as f32 * scale,
        ];
        let max = [min[0] + scale, min[1] + scale, min[2] + scale];
        let [min_x, min_y, min_z] = min;
        let [max_x, max_y, max_z] = max;
        let color = voxel.color();

        face(
            &mut vertices,
            [
                [min_x, min_y, min_z],
                [max_x, min_y, min_z],
                [max_x, min_y, max_z],
                [min_x, min_y, max_z],
            ],
            color,
            0.62,
        );
        face(
            &mut vertices,
            [
                [min_x, max_y, min_z],
                [min_x, max_y, max_z],
                [max_x, max_y, max_z],
                [max_x, max_y, min_z],
            ],
            color,
            1.12,
        );
        face(
            &mut vertices,
            [
                [min_x, min_y, min_z],
                [min_x, max_y, min_z],
                [max_x, max_y, min_z],
                [max_x, min_y, min_z],
            ],
            color,
            0.84,
        );
        face(
            &mut vertices,
            [
                [max_x, min_y, max_z],
                [max_x, max_y, max_z],
                [min_x, max_y, max_z],
                [min_x, min_y, max_z],
            ],
            color,
            0.93,
        );
        face(
            &mut vertices,
            [
                [min_x, min_y, max_z],
                [min_x, max_y, max_z],
                [min_x, max_y, min_z],
                [min_x, min_y, min_z],
            ],
            color,
            0.74,
        );
        face(
            &mut vertices,
            [
                [max_x, min_y, min_z],
                [max_x, max_y, min_z],
                [max_x, max_y, max_z],
                [max_x, min_y, max_z],
            ],
            color,
            1.0,
        );
    }
    vertices
}

fn face(vertices: &mut Vec<Vertex>, corners: [[f32; 3]; 4], color: [f32; 3], shade: f32) {
    let color = [
        (color[0] * shade).min(1.0),
        (color[1] * shade).min(1.0),
        (color[2] * shade).min(1.0),
    ];
    vertices.extend([
        Vertex {
            position: corners[0],
            color,
        },
        Vertex {
            position: corners[1],
            color,
        },
        Vertex {
            position: corners[2],
            color,
        },
        Vertex {
            position: corners[0],
            color,
        },
        Vertex {
            position: corners[2],
            color,
        },
        Vertex {
            position: corners[3],
            color,
        },
    ]);
}
