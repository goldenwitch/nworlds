/// One owned vertex in the host-owned renderer-agnostic triangle vocabulary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl RenderVertex {
    /// Creates one vertex in normalized clip-space coordinates.
    pub const fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, color }
    }

    /// Returns the target-neutral vertex position.
    pub const fn position(self) -> [f32; 3] {
        self.position
    }

    /// Returns the target-neutral RGBA color.
    pub const fn color(self) -> [f32; 4] {
        self.color
    }
}

/// Owned, disposable, renderer-agnostic triangle-list draw intent.
///
/// Three consecutive vertices form one triangle. The batch contains only
/// target-neutral geometry and appearance data. It does not carry game state,
/// input, journals, worldlines, branch selection, devices, or host-clock data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderBatch {
    vertices: Vec<RenderVertex>,
}

impl RenderBatch {
    /// Creates a batch from owned triangle-list vertices.
    pub fn new(vertices: impl Into<Vec<RenderVertex>>) -> Self {
        Self {
            vertices: vertices.into(),
        }
    }

    /// Creates an empty batch.
    pub const fn empty() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    /// Returns the owned triangle-list vertices in submission order.
    pub fn vertices(&self) -> &[RenderVertex] {
        &self.vertices
    }

    /// Returns the number of vertices in the batch.
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Reports whether the batch contains no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderBatch, RenderVertex};

    #[test]
    fn batch_is_owned_triangle_data() {
        let batch = RenderBatch::new([
            RenderVertex::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0]),
            RenderVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 1.0]),
            RenderVertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]),
        ]);

        assert_eq!(batch.len(), 3);
        assert_eq!(batch.vertices()[0].position(), [0.0, 0.0, 0.0]);
        assert_eq!(batch.vertices()[2].color(), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn empty_batch_is_explicit() {
        let batch = RenderBatch::empty();

        assert!(batch.is_empty());
        assert_eq!(batch.vertices(), &[]);
    }

    fn assert_fire_and_forget<T: Send + Sync + 'static>() {}

    #[test]
    fn batch_is_send_sync_static_data() {
        assert_fire_and_forget::<RenderBatch>();
        assert_fire_and_forget::<RenderVertex>();
    }
}
