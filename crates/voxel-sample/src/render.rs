use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use nworlds_host::RenderSink;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::camera::Camera;
use crate::engine_integration::{RenderBatch, VoxelFrame};
use crate::world::{VoxelPosition, VoxelState};

const DEPTH_FORMAT: ::wgpu::TextureFormat = ::wgpu::TextureFormat::Depth24Plus;

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [::wgpu::VertexAttribute; 2] =
        ::wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

    const fn layout() -> ::wgpu::VertexBufferLayout<'static> {
        ::wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Self>() as ::wgpu::BufferAddress,
            step_mode: ::wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
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
        let pipeline_layout = device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
            label: Some("voxel-pipeline-layout"),
            bind_group_layouts: &[],
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

fn vertices(batch: &RenderBatch) -> Vec<Vertex> {
    batch
        .vertices()
        .iter()
        .map(|vertex| Vertex {
            position: vertex.position(),
            color: vertex.color(),
        })
        .collect()
}
