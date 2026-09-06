use engine_presentation::{RenderBatch, RenderVertex};

use crate::{PlaybackMode, ScreenPoint, StepDirection, TimelineAxis, Viewport};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedPoint {
    x: f32,
    y: f32,
}

impl NormalizedPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_screen(viewport: Viewport, point: ScreenPoint) -> Self {
        let width = viewport.width().max_one().get() as f32;
        let height = viewport.height().max_one().get() as f32;
        Self::new(
            2.0 * point.x().get() as f32 / width - 1.0,
            1.0 - 2.0 * point.y().get() as f32 / height,
        )
    }

    pub fn from_pixels(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self::from_screen(Viewport::new(width, height), ScreenPoint::new(x, y))
    }

    pub const fn x(self) -> f32 {
        self.x
    }

    pub const fn y(self) -> f32 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlRect {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl ControlRect {
    pub const fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn contains(self, point: NormalizedPoint) -> bool {
        point.x() >= self.min_x
            && point.x() <= self.max_x
            && point.y() >= self.min_y
            && point.y() <= self.max_y
    }

    pub const fn min_x(self) -> f32 {
        self.min_x
    }

    pub const fn min_y(self) -> f32 {
        self.min_y
    }

    pub const fn max_x(self) -> f32 {
        self.max_x
    }

    pub const fn max_y(self) -> f32 {
        self.max_y
    }

    pub const fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    pub const fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControlTarget {
    Slider(TimelineAxis),
    Step {
        axis: TimelineAxis,
        direction: StepDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLayout {
    logical_slider: ControlRect,
    tau_slider: ControlRect,
    logical_backward: ControlRect,
    logical_forward: ControlRect,
    tau_backward: ControlRect,
    tau_forward: ControlRect,
}

impl TimelineLayout {
    /// Builds the default two-row layout from a fixed pixel design and a viewport.
    ///
    /// The design is scaled uniformly from 960x720 and anchored to the bottom
    /// edge, so wide viewports do not stretch the controls horizontally.
    pub fn auto_scale(viewport: Viewport) -> Self {
        const DESIGN_WIDTH: f32 = 960.0;
        const DESIGN_HEIGHT: f32 = 720.0;
        const MARGIN_X: f32 = 24.0;
        const BUTTON_WIDTH: f32 = 48.0;
        const GAP: f32 = 12.0;
        const TRACK_X: f32 = MARGIN_X + BUTTON_WIDTH + GAP;
        const TRACK_WIDTH: f32 = DESIGN_WIDTH - MARGIN_X * 2.0 - BUTTON_WIDTH * 2.0 - GAP * 2.0;
        const BOTTOM_MARGIN: f32 = 20.0;
        const ROW_HEIGHT: f32 = 32.0;
        const ROW_GAP: f32 = 12.0;
        const LOGICAL_TOP: f32 = DESIGN_HEIGHT - BOTTOM_MARGIN - ROW_HEIGHT * 2.0 - ROW_GAP;
        const TAU_TOP: f32 = DESIGN_HEIGHT - BOTTOM_MARGIN - ROW_HEIGHT;

        let width = viewport.width().max_one().get() as f32;
        let height = viewport.height().max_one().get() as f32;
        let scale = (width / DESIGN_WIDTH).min(height / DESIGN_HEIGHT);
        let offset_x = (width - DESIGN_WIDTH * scale) * 0.5;
        let offset_y = height - DESIGN_HEIGHT * scale;
        let rect = |x: f32, y: f32, rect_width: f32, rect_height: f32| {
            let min_x = offset_x + x * scale;
            let max_x = min_x + rect_width * scale;
            let top = offset_y + y * scale;
            let bottom = top + rect_height * scale;
            ControlRect::new(
                2.0 * min_x / width - 1.0,
                1.0 - 2.0 * bottom / height,
                2.0 * max_x / width - 1.0,
                1.0 - 2.0 * top / height,
            )
        };

        Self::new(
            rect(TRACK_X, LOGICAL_TOP, TRACK_WIDTH, ROW_HEIGHT),
            rect(TRACK_X, TAU_TOP, TRACK_WIDTH, ROW_HEIGHT),
            rect(MARGIN_X, LOGICAL_TOP, BUTTON_WIDTH, ROW_HEIGHT),
            rect(
                DESIGN_WIDTH - MARGIN_X - BUTTON_WIDTH,
                LOGICAL_TOP,
                BUTTON_WIDTH,
                ROW_HEIGHT,
            ),
            rect(MARGIN_X, TAU_TOP, BUTTON_WIDTH, ROW_HEIGHT),
            rect(
                DESIGN_WIDTH - MARGIN_X - BUTTON_WIDTH,
                TAU_TOP,
                BUTTON_WIDTH,
                ROW_HEIGHT,
            ),
        )
    }

    pub const fn new(
        logical_slider: ControlRect,
        tau_slider: ControlRect,
        logical_backward: ControlRect,
        logical_forward: ControlRect,
        tau_backward: ControlRect,
        tau_forward: ControlRect,
    ) -> Self {
        Self {
            logical_slider,
            tau_slider,
            logical_backward,
            logical_forward,
            tau_backward,
            tau_forward,
        }
    }

    pub const fn logical_slider(self) -> ControlRect {
        self.logical_slider
    }

    pub const fn tau_slider(self) -> ControlRect {
        self.tau_slider
    }

    pub const fn step_rect(self, axis: TimelineAxis, direction: StepDirection) -> ControlRect {
        match (axis, direction) {
            (TimelineAxis::LogicalTime, StepDirection::Backward) => self.logical_backward,
            (TimelineAxis::LogicalTime, StepDirection::Forward) => self.logical_forward,
            (TimelineAxis::Tau, StepDirection::Backward) => self.tau_backward,
            (TimelineAxis::Tau, StepDirection::Forward) => self.tau_forward,
        }
    }

    pub fn hit_test(self, point: NormalizedPoint) -> Option<ControlTarget> {
        if self.logical_backward.contains(point) {
            return Some(ControlTarget::Step {
                axis: TimelineAxis::LogicalTime,
                direction: StepDirection::Backward,
            });
        }
        if self.logical_forward.contains(point) {
            return Some(ControlTarget::Step {
                axis: TimelineAxis::LogicalTime,
                direction: StepDirection::Forward,
            });
        }
        if self.tau_backward.contains(point) {
            return Some(ControlTarget::Step {
                axis: TimelineAxis::Tau,
                direction: StepDirection::Backward,
            });
        }
        if self.tau_forward.contains(point) {
            return Some(ControlTarget::Step {
                axis: TimelineAxis::Tau,
                direction: StepDirection::Forward,
            });
        }
        if self.logical_slider.contains(point) {
            return Some(ControlTarget::Slider(TimelineAxis::LogicalTime));
        }
        if self.tau_slider.contains(point) {
            return Some(ControlTarget::Slider(TimelineAxis::Tau));
        }
        None
    }

    pub fn slider_fraction(self, axis: TimelineAxis, point: NormalizedPoint) -> f32 {
        let rect = match axis {
            TimelineAxis::LogicalTime => self.logical_slider,
            TimelineAxis::Tau => self.tau_slider,
        };
        let fraction = (point.x() - rect.min_x()) / rect.width();
        if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        }
    }

    pub fn append_vertices(
        self,
        vertices: &mut Vec<RenderVertex>,
        logical_fraction: f32,
        tau_fraction: f32,
        mode: PlaybackMode,
    ) {
        let logical_color = [0.20, 0.74, 0.86, 1.0];
        let tau_color = [0.96, 0.56, 0.20, 1.0];
        let inactive = [0.08, 0.11, 0.15, 1.0];
        let mode_color = match mode {
            PlaybackMode::Automatic => [0.32, 0.86, 0.60, 1.0],
            PlaybackMode::Manual => [0.98, 0.78, 0.28, 1.0],
        };

        append_slider(
            vertices,
            self.logical_slider,
            logical_fraction,
            logical_color,
            mode_color,
        );
        append_slider(
            vertices,
            self.tau_slider,
            tau_fraction,
            tau_color,
            mode_color,
        );
        append_step_button(
            vertices,
            self.logical_backward,
            StepDirection::Backward,
            logical_color,
            inactive,
        );
        append_step_button(
            vertices,
            self.logical_forward,
            StepDirection::Forward,
            logical_color,
            inactive,
        );
        append_step_button(
            vertices,
            self.tau_backward,
            StepDirection::Backward,
            tau_color,
            inactive,
        );
        append_step_button(
            vertices,
            self.tau_forward,
            StepDirection::Forward,
            tau_color,
            inactive,
        );
    }
}

impl Default for TimelineLayout {
    fn default() -> Self {
        Self::auto_scale(Viewport::new(960, 720))
    }
}

impl TimelineLayout {
    pub fn render(
        self,
        logical_fraction: f32,
        tau_fraction: f32,
        mode: PlaybackMode,
    ) -> RenderBatch {
        let mut vertices = Vec::new();
        self.append_vertices(&mut vertices, logical_fraction, tau_fraction, mode);
        RenderBatch::new(vertices)
    }
}

const CONTROL_Z: f32 = 0.01;
const INNER_Z: f32 = 0.009;

fn append_slider(
    vertices: &mut Vec<RenderVertex>,
    rect: ControlRect,
    fraction: f32,
    color: [f32; 4],
    knob_color: [f32; 4],
) {
    let fraction = if fraction.is_nan() {
        0.0
    } else {
        fraction.clamp(0.0, 1.0)
    };
    let fill_end = rect.min_x() + rect.width() * fraction;
    quad(
        vertices,
        rect.min_x(),
        rect.max_x(),
        rect.min_y(),
        rect.max_y(),
        CONTROL_Z,
        [0.05, 0.07, 0.10, 1.0],
    );
    quad(
        vertices,
        rect.min_x() + 0.01,
        fill_end.max(rect.min_x() + 0.01),
        rect.min_y() + 0.01,
        rect.max_y() - 0.01,
        INNER_Z,
        color,
    );
    quad(
        vertices,
        (fill_end - 0.012).max(rect.min_x()),
        (fill_end + 0.012).min(rect.max_x()),
        rect.min_y() - 0.012,
        rect.max_y() + 0.012,
        CONTROL_Z - 0.001,
        knob_color,
    );
}

fn append_step_button(
    vertices: &mut Vec<RenderVertex>,
    rect: ControlRect,
    direction: StepDirection,
    color: [f32; 4],
    background: [f32; 4],
) {
    quad(
        vertices,
        rect.min_x(),
        rect.max_x(),
        rect.min_y(),
        rect.max_y(),
        CONTROL_Z,
        background,
    );
    let center_x = (rect.min_x() + rect.max_x()) * 0.5;
    let center_y = (rect.min_y() + rect.max_y()) * 0.5;
    let half_width = rect.width() * 0.26;
    let half_height = rect.height() * 0.30;
    match direction {
        StepDirection::Backward => triangle(
            vertices,
            [center_x - half_width, center_y, INNER_Z],
            [center_x + half_width, center_y + half_height, INNER_Z],
            [center_x + half_width, center_y - half_height, INNER_Z],
            color,
        ),
        StepDirection::Forward => triangle(
            vertices,
            [center_x + half_width, center_y, INNER_Z],
            [center_x - half_width, center_y + half_height, INNER_Z],
            [center_x - half_width, center_y - half_height, INNER_Z],
            color,
        ),
    }
}

fn quad(
    vertices: &mut Vec<RenderVertex>,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    z: f32,
    color: [f32; 4],
) {
    triangle(
        vertices,
        [min_x, min_y, z],
        [max_x, min_y, z],
        [max_x, max_y, z],
        color,
    );
    triangle(
        vertices,
        [min_x, min_y, z],
        [max_x, max_y, z],
        [min_x, max_y, z],
        color,
    );
}

fn triangle(
    vertices: &mut Vec<RenderVertex>,
    first: [f32; 3],
    second: [f32; 3],
    third: [f32; 3],
    color: [f32; 4],
) {
    vertices.extend([
        RenderVertex::new(first, color),
        RenderVertex::new(second, color),
        RenderVertex::new(third, color),
    ]);
}

#[cfg(test)]
mod tests {
    use super::{ControlRect, ControlTarget, NormalizedPoint, TimelineLayout};
    use crate::Viewport;
    use crate::{StepDirection, TimelineAxis};

    #[test]
    fn default_layout_hits_all_four_step_controls_and_both_sliders() {
        let layout = TimelineLayout::default();
        let rows = [
            (layout.logical_slider(), TimelineAxis::LogicalTime),
            (layout.tau_slider(), TimelineAxis::Tau),
        ];
        for (y, axis) in rows {
            let y = (y.min_y() + y.max_y()) * 0.5;
            assert_eq!(
                layout.hit_test(NormalizedPoint::new(-0.90, y)),
                Some(ControlTarget::Step {
                    axis,
                    direction: StepDirection::Backward,
                })
            );
            assert_eq!(
                layout.hit_test(NormalizedPoint::new(0.90, y)),
                Some(ControlTarget::Step {
                    axis,
                    direction: StepDirection::Forward,
                })
            );
            assert_eq!(
                layout.hit_test(NormalizedPoint::new(0.0, y)),
                Some(ControlTarget::Slider(axis))
            );
        }
        assert_eq!(layout.hit_test(NormalizedPoint::new(0.0, 0.0)), None);
    }

    #[test]
    fn slider_fraction_is_clamped_to_the_track() {
        let layout = TimelineLayout::default();
        assert_eq!(
            layout.slider_fraction(TimelineAxis::LogicalTime, NormalizedPoint::new(-2.0, 0.0)),
            0.0
        );
        assert_eq!(
            layout.slider_fraction(TimelineAxis::Tau, NormalizedPoint::new(2.0, 0.0)),
            1.0
        );
    }

    #[test]
    fn render_is_owned_and_deterministic() {
        let layout = TimelineLayout::default();
        let first = layout.render(0.25, 0.75, crate::PlaybackMode::Automatic);
        let second = layout.render(0.25, 0.75, crate::PlaybackMode::Automatic);
        assert!(!first.is_empty());
        assert_eq!(first, second);
    }

    #[test]
    fn rectangles_report_dimensions_and_containment() {
        let rect = ControlRect::new(-1.0, -0.5, 1.0, 0.5);
        assert_eq!(rect.width(), 2.0);
        assert_eq!(rect.height(), 1.0);
        assert!(rect.contains(NormalizedPoint::new(0.0, 0.0)));
    }

    #[test]
    fn auto_scale_preserves_design_proportions_across_viewports() {
        let reference = TimelineLayout::auto_scale(Viewport::new(960, 720));
        let wide = TimelineLayout::auto_scale(Viewport::new(1920, 720));
        let compact = TimelineLayout::auto_scale(Viewport::new(480, 360));

        assert!(
            (reference.logical_slider().width() - compact.logical_slider().width()).abs() < 0.001
        );
        assert!(wide.logical_slider().width() < reference.logical_slider().width());
        assert!(wide.logical_slider().min_x() > reference.logical_slider().min_x());
    }
}
