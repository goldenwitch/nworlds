use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use caravan_demo::{RenderOutput, RenderTile};
use caravan_domain::Terrain;
use engine_sdk::Frame;
use nworlds_host::RenderSink;
use winit::window::Window;

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
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
    position: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [::wgpu::VertexAttribute; 2] =
        ::wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];

    const fn layout() -> ::wgpu::VertexBufferLayout<'static> {
        ::wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Self>() as ::wgpu::BufferAddress,
            step_mode: ::wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
pub enum WgpuRenderError {
    Surface(String),
    AdapterUnavailable,
    Device(String),
    SurfaceFormatUnavailable,
}

impl core::fmt::Display for WgpuRenderError {
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

impl std::error::Error for WgpuRenderError {}

pub struct WgpuRenderSink {
    window: Arc<Window>,
    surface: ::wgpu::Surface<'static>,
    device: ::wgpu::Device,
    queue: ::wgpu::Queue,
    config: ::wgpu::SurfaceConfiguration,
    pipeline: ::wgpu::RenderPipeline,
}

impl WgpuRenderSink {
    pub async fn new(window: Arc<Window>) -> Result<Self, WgpuRenderError> {
        let instance = ::wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| WgpuRenderError::Surface(error.to_string()))?;
        let adapter = instance
            .request_adapter(&::wgpu::RequestAdapterOptions {
                power_preference: ::wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| WgpuRenderError::AdapterUnavailable)?;
        let (device, queue) = adapter
            .request_device(&::wgpu::DeviceDescriptor {
                label: Some("caravan-device"),
                required_features: ::wgpu::Features::empty(),
                required_limits: ::wgpu::Limits::default(),
                experimental_features: ::wgpu::ExperimentalFeatures::disabled(),
                memory_hints: ::wgpu::MemoryHints::Performance,
                trace: ::wgpu::Trace::Off,
            })
            .await
            .map_err(|error| WgpuRenderError::Device(error.to_string()))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(::wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(WgpuRenderError::SurfaceFormatUnavailable)?;
        let size = window.inner_size();
        let config = ::wgpu::SurfaceConfiguration {
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
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
            label: Some("caravan-shader"),
            source: ::wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
            label: Some("caravan-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
            label: Some("caravan-pipeline"),
            layout: Some(&layout),
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
            depth_stencil: None,
            multisample: ::wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
        })
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
        }
        true
    }

    fn vertices(output: &RenderOutput) -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(output.tiles().len() * 18);
        for tile in output.tiles() {
            let center = tile_center(tile);
            let color = tile_color(tile);
            for corner in 0..6 {
                vertices.push(Vertex {
                    position: center,
                    color,
                });
                vertices.push(Vertex {
                    position: hex_corner(center, corner),
                    color,
                });
                vertices.push(Vertex {
                    position: hex_corner(center, corner + 1),
                    color,
                });
            }
        }
        vertices
    }
}

impl RenderSink<Frame<RenderOutput>> for WgpuRenderSink {
    fn submit(&mut self, frame: Frame<RenderOutput>) {
        if !self.configure_for_current_window_size() {
            return;
        }
        let vertices = Self::vertices(frame.payload());
        let vertex_buffer = ::wgpu::util::DeviceExt::create_buffer_init(
            &self.device,
            &::wgpu::util::BufferInitDescriptor {
                label: Some("caravan-vertices"),
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
                label: Some("caravan-render-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
                label: Some("caravan-render-pass"),
                color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: ::wgpu::Operations {
                        load: ::wgpu::LoadOp::Clear(::wgpu::Color {
                            r: 0.035,
                            g: 0.045,
                            b: 0.06,
                            a: 1.0,
                        }),
                        store: ::wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
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

fn tile_center(tile: &RenderTile) -> [f32; 2] {
    const HEX_RADIUS: f32 = 0.095;
    let q = tile.tile().q() as f32;
    let r = tile.tile().r() as f32;
    [HEX_RADIUS * 1.732 * (q + r * 0.5), HEX_RADIUS * 1.5 * r]
}

fn hex_corner(center: [f32; 2], index: usize) -> [f32; 2] {
    const HEX_RADIUS: f32 = 0.095;
    let angle = (30.0 + 60.0 * (index % 6) as f32).to_radians();
    [
        center[0] + HEX_RADIUS * angle.cos(),
        center[1] + HEX_RADIUS * angle.sin(),
    ]
}

fn tile_color(tile: &RenderTile) -> [f32; 3] {
    if tile.effect().fire_age().is_some() {
        return [0.9, 0.16, 0.04];
    }
    if tile.actor().is_some() {
        return [0.12, 0.72, 0.88];
    }
    match tile.terrain() {
        Terrain::Void => [0.18, 0.23, 0.3],
        Terrain::Wheat => [0.94, 0.68, 0.08],
        Terrain::Forest => [0.12, 0.56, 0.25],
    }
}
