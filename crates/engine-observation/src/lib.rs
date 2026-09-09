#![forbid(unsafe_code)]

use std::{fmt, io::Cursor};

use engine_presentation::RenderBatch;
use engine_sdk::Frame;
use engine_time::{LogicalTime, Tau};
use image::{ImageBuffer, ImageFormat, Rgba};

/// Produces an owned frame for an explicit logical and presentation-time sample.
pub trait RenderSource {
    type Error: fmt::Display;

    fn render(
        &self,
        logical_time: LogicalTime,
        tau: Tau,
    ) -> Result<Frame<RenderBatch>, Self::Error>;
}

/// Inputs accepted by the reusable observation rasterizer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationRequest {
    pub width: u32,
    pub height: u32,
    pub background: [u8; 4],
}

impl ObservationRequest {
    pub const MAX_DIMENSION: u32 = 4_096;

    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background: [16, 20, 28, 255],
        }
    }

    fn validate(self) -> Result<Self, ObservationError<InfallibleError>> {
        if self.width == 0 || self.height == 0 {
            return Err(ObservationError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        if self.width > Self::MAX_DIMENSION || self.height > Self::MAX_DIMENSION {
            return Err(ObservationError::DimensionsTooLarge {
                width: self.width,
                height: self.height,
                maximum: Self::MAX_DIMENSION,
            });
        }
        Ok(self)
    }
}

impl Default for ObservationRequest {
    fn default() -> Self {
        Self::new(960, 720)
    }
}

/// A deterministic PNG snapshot and its reusable render metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderSnapshot {
    logical_time: LogicalTime,
    tau: Tau,
    width: u32,
    height: u32,
    vertex_count: usize,
    triangle_count: usize,
    png: Vec<u8>,
}

impl RenderSnapshot {
    pub fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    pub fn tau(&self) -> Tau {
        self.tau
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub const fn triangle_count(&self) -> usize {
        self.triangle_count
    }

    pub fn png(&self) -> &[u8] {
        &self.png
    }
}

/// Errors produced while turning a generic render source into an observation.
#[derive(Debug)]
pub enum ObservationError<E> {
    Source(E),
    InvalidDimensions {
        width: u32,
        height: u32,
    },
    DimensionsTooLarge {
        width: u32,
        height: u32,
        maximum: u32,
    },
    Image(image::ImageError),
}

impl<E: fmt::Display> fmt::Display for ObservationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "render source failed: {error}"),
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "snapshot dimensions must be non-zero, got {width}x{height}"
                )
            }
            Self::DimensionsTooLarge {
                width,
                height,
                maximum,
            } => write!(
                formatter,
                "snapshot dimensions {width}x{height} exceed maximum {maximum}"
            ),
            Self::Image(error) => write!(formatter, "PNG encoding failed: {error}"),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> std::error::Error for ObservationError<E> {}

/// Samples a generic render source and rasterizes its triangle-list output.
pub fn snapshot<S>(
    source: &S,
    logical_time: LogicalTime,
    tau: Tau,
    request: ObservationRequest,
) -> Result<RenderSnapshot, ObservationError<S::Error>>
where
    S: RenderSource,
{
    let request = request.validate().map_err(|error| match error {
        ObservationError::InvalidDimensions { width, height } => {
            ObservationError::InvalidDimensions { width, height }
        }
        ObservationError::DimensionsTooLarge {
            width,
            height,
            maximum,
        } => ObservationError::DimensionsTooLarge {
            width,
            height,
            maximum,
        },
        ObservationError::Source(_) | ObservationError::Image(_) => {
            unreachable!("dimension validation cannot produce source or image errors")
        }
    })?;
    let frame = source
        .render(logical_time, tau)
        .map_err(ObservationError::Source)?;
    let batch = frame.payload();
    let vertex_count = batch.len();
    let triangle_count = vertex_count / 3;
    let pixels = rasterize(batch, request);
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(request.width, request.height, pixels)
        .expect("validated image dimensions match raster buffer length");
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(ObservationError::Image)?;

    Ok(RenderSnapshot {
        logical_time,
        tau: frame.tau(),
        width: request.width,
        height: request.height,
        vertex_count,
        triangle_count,
        png,
    })
}

fn rasterize(batch: &RenderBatch, request: ObservationRequest) -> Vec<u8> {
    let mut pixels = vec![0_u8; (request.width * request.height * 4) as usize];
    let mut depths = vec![f32::INFINITY; (request.width * request.height) as usize];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&request.background);
    }

    for triangle in batch.vertices().chunks_exact(3) {
        let vertices = [triangle[0], triangle[1], triangle[2]].map(|vertex| {
            let position = vertex.position();
            (
                (position[0] * 0.5 + 0.5) * (request.width.saturating_sub(1) as f32),
                (1.0 - (position[1] * 0.5 + 0.5)) * (request.height.saturating_sub(1) as f32),
                position[2],
                vertex.color(),
            )
        });
        let area = edge(vertices[0], vertices[1], (vertices[2].0, vertices[2].1));
        if area.abs() <= f32::EPSILON {
            continue;
        }

        let min_x = vertices
            .iter()
            .map(|vertex| vertex.0)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_x = vertices
            .iter()
            .map(|vertex| vertex.0)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(request.width.saturating_sub(1) as f32) as u32;
        let min_y = vertices
            .iter()
            .map(|vertex| vertex.1)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_y = vertices
            .iter()
            .map(|vertex| vertex.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(request.height.saturating_sub(1) as f32) as u32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let point = (x as f32 + 0.5, y as f32 + 0.5);
                let weights = [
                    edge(vertices[1], vertices[2], point) / area,
                    edge(vertices[2], vertices[0], point) / area,
                    edge(vertices[0], vertices[1], point) / area,
                ];
                if weights.iter().any(|weight| *weight < 0.0) {
                    continue;
                }
                let depth = weights[0] * vertices[0].2
                    + weights[1] * vertices[1].2
                    + weights[2] * vertices[2].2;
                let index = (y * request.width + x) as usize;
                if depth >= depths[index] {
                    continue;
                }
                depths[index] = depth;
                let color = [
                    interpolate(weights, vertices, 0),
                    interpolate(weights, vertices, 1),
                    interpolate(weights, vertices, 2),
                    interpolate(weights, vertices, 3),
                ];
                let output = &mut pixels[index * 4..index * 4 + 4];
                for (channel, value) in output.iter_mut().zip(color) {
                    *channel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
    }
    pixels
}

fn interpolate(weights: [f32; 3], vertices: [(f32, f32, f32, [f32; 4]); 3], channel: usize) -> f32 {
    weights[0] * vertices[0].3[channel]
        + weights[1] * vertices[1].3[channel]
        + weights[2] * vertices[2].3[channel]
}

fn edge(
    first: (f32, f32, f32, [f32; 4]),
    second: (f32, f32, f32, [f32; 4]),
    point: (f32, f32),
) -> f32 {
    (point.0 - first.0) * (second.1 - first.1) - (point.1 - first.1) * (second.0 - first.0)
}

#[derive(Debug)]
struct InfallibleError;

impl fmt::Display for InfallibleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("infallible")
    }
}

#[cfg(test)]
mod tests {
    use super::{snapshot, ObservationRequest, RenderSource};
    use engine_presentation::{RenderBatch, RenderVertex};
    use engine_sdk::Frame;
    use engine_time::{LogicalTime, Tau};

    struct TriangleSource;

    impl RenderSource for TriangleSource {
        type Error = std::convert::Infallible;

        fn render(
            &self,
            _logical_time: LogicalTime,
            tau: Tau,
        ) -> Result<Frame<RenderBatch>, Self::Error> {
            Ok(Frame::new(
                tau,
                RenderBatch::new([
                    RenderVertex::new([-0.8, -0.8, 0.0], [1.0, 0.0, 0.0, 1.0]),
                    RenderVertex::new([0.8, -0.8, 0.0], [0.0, 1.0, 0.0, 1.0]),
                    RenderVertex::new([0.0, 0.8, 0.0], [0.0, 0.0, 1.0, 1.0]),
                ]),
            ))
        }
    }

    #[test]
    fn generic_source_becomes_a_png_with_stable_metadata() {
        let result = snapshot(
            &TriangleSource,
            LogicalTime::from_ticks(7),
            Tau::from_ticks(11),
            ObservationRequest::new(32, 24),
        )
        .expect("triangle source should rasterize");

        assert_eq!(result.logical_time(), LogicalTime::from_ticks(7));
        assert_eq!(result.tau(), Tau::from_ticks(11));
        assert_eq!(result.vertex_count(), 3);
        assert_eq!(result.triangle_count(), 1);
        assert_eq!(&result.png()[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn dimensions_are_validated_before_rendering() {
        let error = snapshot(
            &TriangleSource,
            LogicalTime::zero(),
            Tau::zero(),
            ObservationRequest::new(0, 10),
        )
        .expect_err("zero width should be rejected");
        assert!(matches!(
            error,
            super::ObservationError::InvalidDimensions { .. }
        ));
    }
}
