use engine_presentation::RenderBatch;
use engine_time::{LogicalTime, Tau};

use crate::{
    ControlTarget, LogicalTimeDelta, NormalizedPoint, SliderFocus, TauDelta, TimelineLayout,
    Viewport,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TimelineAxis {
    LogicalTime,
    Tau,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StepDirection {
    Backward,
    Forward,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlaybackMode {
    Automatic,
    Manual,
}

/// Reprojects unbounded absolute times onto a bounded slider around fixed
/// focus values. The time distance grows quadratically toward either edge.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParabolicProjection {
    logical_focus: LogicalTime,
    logical_horizon: LogicalTimeDelta,
    tau_focus: Tau,
    tau_horizon: TauDelta,
    slider_focus: SliderFocus,
}

impl ParabolicProjection {
    pub const fn new(
        logical_focus: LogicalTime,
        logical_horizon: LogicalTimeDelta,
        tau_focus: Tau,
        tau_horizon: TauDelta,
    ) -> Self {
        Self {
            logical_focus,
            logical_horizon,
            tau_focus,
            tau_horizon,
            slider_focus: SliderFocus::DEFAULT,
        }
    }

    pub const fn logical_focus(self) -> LogicalTime {
        self.logical_focus
    }

    pub const fn logical_horizon(self) -> LogicalTimeDelta {
        self.logical_horizon
    }

    pub const fn tau_focus(self) -> Tau {
        self.tau_focus
    }

    pub const fn tau_horizon(self) -> TauDelta {
        self.tau_horizon
    }

    pub const fn slider_focus(self) -> SliderFocus {
        self.slider_focus
    }

    pub const fn with_slider_focus(mut self, slider_focus: SliderFocus) -> Self {
        self.slider_focus = slider_focus;
        self
    }

    pub fn logical_fraction(self, value: LogicalTime) -> f32 {
        project_ticks(
            value.ticks(),
            self.logical_focus.ticks(),
            self.logical_horizon.ticks(),
            self.slider_focus,
        )
    }

    pub fn tau_fraction(self, value: Tau) -> f32 {
        project_ticks(
            value.ticks(),
            self.tau_focus.ticks(),
            self.tau_horizon.ticks(),
            self.slider_focus,
        )
    }

    pub fn logical_time_at_fraction(self, fraction: f32) -> LogicalTime {
        LogicalTime::from_ticks(unproject_ticks(
            fraction,
            self.logical_focus.ticks(),
            self.logical_horizon.ticks(),
            self.slider_focus,
        ))
    }

    pub fn tau_at_fraction(self, fraction: f32) -> Tau {
        Tau::from_ticks(unproject_ticks(
            fraction,
            self.tau_focus.ticks(),
            self.tau_horizon.ticks(),
            self.slider_focus,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimelineConfig {
    projection: ParabolicProjection,
    automatic_logical: LogicalTimeDelta,
    automatic_tau: TauDelta,
    manual_logical: LogicalTimeDelta,
    manual_tau: TauDelta,
}

impl TimelineConfig {
    pub const fn new(
        projection: ParabolicProjection,
        automatic_logical: LogicalTimeDelta,
        automatic_tau: TauDelta,
        manual_logical: LogicalTimeDelta,
        manual_tau: TauDelta,
    ) -> Self {
        Self {
            projection,
            automatic_logical,
            automatic_tau,
            manual_logical,
            manual_tau,
        }
    }

    pub const fn projection(self) -> ParabolicProjection {
        self.projection
    }

    pub const fn automatic_logical(self) -> LogicalTimeDelta {
        self.automatic_logical
    }

    pub const fn automatic_tau(self) -> TauDelta {
        self.automatic_tau
    }

    pub const fn manual_logical(self) -> LogicalTimeDelta {
        self.manual_logical
    }

    pub const fn manual_tau(self) -> TauDelta {
        self.manual_tau
    }
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self::new(
            ParabolicProjection::new(
                LogicalTime::zero(),
                LogicalTimeDelta::from_ticks(5_000),
                Tau::zero(),
                TauDelta::from_ticks(5_000),
            ),
            LogicalTimeDelta::from_ticks(16),
            TauDelta::from_ticks(16),
            LogicalTimeDelta::from_ticks(250),
            TauDelta::from_ticks(250),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TimelineError {
    LogicalTimeOverflow,
    TauOverflow,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerTarget {
    Timeline,
    World,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineControls {
    logical_time: LogicalTime,
    tau: Tau,
    mode: PlaybackMode,
    config: TimelineConfig,
    layout: TimelineLayout,
    dragging: Option<TimelineAxis>,
}

impl TimelineControls {
    pub fn new(logical_time: LogicalTime, tau: Tau, config: TimelineConfig) -> Self {
        Self {
            logical_time,
            tau,
            mode: PlaybackMode::Automatic,
            config,
            layout: TimelineLayout::default(),
            dragging: None,
        }
    }

    pub const fn with_layout(mut self, layout: TimelineLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.layout = TimelineLayout::auto_scale(viewport);
        self
    }

    pub const fn logical_time(self) -> LogicalTime {
        self.logical_time
    }

    pub const fn tau(self) -> Tau {
        self.tau
    }

    pub const fn mode(self) -> PlaybackMode {
        self.mode
    }

    pub const fn config(self) -> TimelineConfig {
        self.config
    }

    pub const fn layout(self) -> TimelineLayout {
        self.layout
    }

    pub const fn is_dragging(self) -> bool {
        self.dragging.is_some()
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.layout = TimelineLayout::auto_scale(viewport);
    }

    pub fn advance_automatic(&mut self) -> Result<bool, TimelineError> {
        if self.mode == PlaybackMode::Manual {
            return Ok(false);
        }

        let logical_time = self
            .logical_time
            .checked_add_ticks(self.config.automatic_logical.ticks())
            .ok_or(TimelineError::LogicalTimeOverflow)?;
        let tau = self
            .tau
            .checked_add_ticks(self.config.automatic_tau.ticks())
            .ok_or(TimelineError::TauOverflow)?;
        let changed = logical_time != self.logical_time || tau != self.tau;
        self.logical_time = logical_time;
        self.tau = tau;
        Ok(changed)
    }

    pub fn step(
        &mut self,
        axis: TimelineAxis,
        direction: StepDirection,
    ) -> Result<bool, TimelineError> {
        self.mode = PlaybackMode::Manual;
        let changed = match axis {
            TimelineAxis::LogicalTime => {
                let delta = signed_step(self.config.manual_logical.ticks(), direction)
                    .ok_or(TimelineError::LogicalTimeOverflow)?;
                let next = self
                    .logical_time
                    .checked_add_ticks(delta)
                    .ok_or(TimelineError::LogicalTimeOverflow)?;
                let changed = next != self.logical_time;
                self.logical_time = next;
                changed
            }
            TimelineAxis::Tau => {
                let delta = signed_step(self.config.manual_tau.ticks(), direction)
                    .ok_or(TimelineError::TauOverflow)?;
                let next = self
                    .tau
                    .checked_add_ticks(delta)
                    .ok_or(TimelineError::TauOverflow)?;
                let changed = next != self.tau;
                self.tau = next;
                changed
            }
        };
        Ok(changed)
    }

    pub fn set_logical_time(&mut self, logical_time: LogicalTime) -> bool {
        self.mode = PlaybackMode::Manual;
        let changed = self.logical_time != logical_time;
        self.logical_time = logical_time;
        changed
    }

    pub fn set_tau(&mut self, tau: Tau) -> bool {
        self.mode = PlaybackMode::Manual;
        let changed = self.tau != tau;
        self.tau = tau;
        changed
    }

    pub fn advance_tau(&mut self, delta: Tau) -> Result<Tau, TimelineError> {
        self.tau = self
            .tau
            .checked_add(delta)
            .ok_or(TimelineError::TauOverflow)?;
        Ok(self.tau)
    }

    pub fn reset_tau(&mut self) -> bool {
        let changed = self.tau != Tau::zero();
        self.tau = Tau::zero();
        changed
    }

    pub fn resume_from_world(&mut self) -> bool {
        let changed = self.mode != PlaybackMode::Automatic || self.dragging.is_some();
        self.mode = PlaybackMode::Automatic;
        self.dragging = None;
        changed
    }

    pub fn pause(&mut self) -> bool {
        let changed = self.mode != PlaybackMode::Manual;
        self.mode = PlaybackMode::Manual;
        changed
    }

    pub fn pointer_down(&mut self, point: NormalizedPoint) -> Result<PointerTarget, TimelineError> {
        match self.layout.hit_test(point) {
            Some(ControlTarget::Slider(axis)) => {
                self.dragging = Some(axis);
                self.set_fraction(axis, self.layout.slider_fraction(axis, point));
                Ok(PointerTarget::Timeline)
            }
            Some(ControlTarget::Step { axis, direction }) => {
                self.dragging = None;
                self.step(axis, direction)?;
                Ok(PointerTarget::Timeline)
            }
            None => {
                self.resume_from_world();
                Ok(PointerTarget::World)
            }
        }
    }

    pub fn pointer_move(&mut self, point: NormalizedPoint) -> Result<bool, TimelineError> {
        let Some(axis) = self.dragging else {
            return Ok(false);
        };
        let fraction = self.layout.slider_fraction(axis, point);
        Ok(self.set_fraction(axis, fraction))
    }

    pub fn pointer_up(&mut self, point: NormalizedPoint) -> Result<bool, TimelineError> {
        let Some(axis) = self.dragging.take() else {
            return Ok(false);
        };
        Ok(self.set_fraction(axis, self.layout.slider_fraction(axis, point)))
    }

    pub fn logical_fraction(self) -> f32 {
        self.config.projection.logical_fraction(self.logical_time)
    }

    pub fn tau_fraction(self) -> f32 {
        self.config.projection.tau_fraction(self.tau)
    }

    pub fn render(&self) -> RenderBatch {
        self.layout
            .render(self.logical_fraction(), self.tau_fraction(), self.mode)
    }

    fn set_fraction(&mut self, axis: TimelineAxis, fraction: f32) -> bool {
        match axis {
            TimelineAxis::LogicalTime => {
                self.set_logical_time(self.config.projection.logical_time_at_fraction(fraction))
            }
            TimelineAxis::Tau => self.set_tau(self.config.projection.tau_at_fraction(fraction)),
        }
    }
}

impl Default for TimelineControls {
    fn default() -> Self {
        Self::new(LogicalTime::zero(), Tau::zero(), TimelineConfig::default())
    }
}

fn signed_step(step: i64, direction: StepDirection) -> Option<i64> {
    match direction {
        StepDirection::Backward => step.checked_neg(),
        StepDirection::Forward => Some(step),
    }
}

fn project_ticks(value: i64, focus: i64, horizon: i64, slider_focus: SliderFocus) -> f32 {
    let delta = value as f64 - focus as f64;
    let magnitude = delta.abs();
    let slider_focus = slider_focus.as_f64();
    if magnitude == 0.0 {
        return slider_focus as f32;
    }

    let horizon = positive_magnitude(horizon);
    let ratio = magnitude / horizon;
    let radius = 2.0 / (1.0 + (1.0 + 4.0 / ratio).sqrt());
    let fraction = if delta.is_sign_negative() {
        slider_focus - radius * slider_focus
    } else {
        slider_focus + radius * (1.0 - slider_focus)
    };
    let fraction = fraction.clamp(0.0, 1.0) as f32;
    if delta.is_sign_negative() {
        fraction.max(f32::EPSILON)
    } else {
        fraction.min(1.0 - f32::EPSILON)
    }
}

fn unproject_ticks(fraction: f32, focus: i64, horizon: i64, slider_focus: SliderFocus) -> i64 {
    let slider_focus = slider_focus.as_f64();
    let fraction = if fraction.is_nan() {
        slider_focus
    } else {
        fraction.clamp(0.0, 1.0) as f64
    };
    let coordinate = if fraction < slider_focus {
        (fraction - slider_focus) / slider_focus
    } else {
        (fraction - slider_focus) / (1.0 - slider_focus)
    };
    if coordinate == 0.0 {
        return focus;
    }

    let radius = coordinate.abs();
    if radius >= 1.0 {
        return if coordinate.is_sign_negative() {
            i64::MIN
        } else {
            i64::MAX
        };
    }

    let distance = positive_magnitude(horizon) * radius * radius / (1.0 - radius);
    let value = focus as f64
        + if coordinate.is_sign_negative() {
            -distance
        } else {
            distance
        };
    value.round().clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

fn positive_magnitude(value: i64) -> f64 {
    let value = value as f64;
    if value.abs() < 1.0 {
        1.0
    } else {
        value.abs()
    }
}

#[cfg(test)]
mod tests {
    use engine_time::{LogicalTime, Tau};

    use super::{
        ParabolicProjection, PlaybackMode, PointerTarget, SliderFocus, StepDirection, TimelineAxis,
        TimelineConfig,
    };
    use crate::{LogicalTimeDelta, NormalizedPoint, TauDelta, TimelineControls, TimelineError};

    fn controls() -> TimelineControls {
        TimelineControls::new(
            LogicalTime::zero(),
            Tau::zero(),
            TimelineConfig::new(
                ParabolicProjection::new(
                    LogicalTime::zero(),
                    LogicalTimeDelta::from_ticks(1_000),
                    Tau::zero(),
                    TauDelta::from_ticks(1_000),
                ),
                LogicalTimeDelta::from_ticks(10),
                TauDelta::from_ticks(20),
                LogicalTimeDelta::from_ticks(100),
                TauDelta::from_ticks(200),
            ),
        )
    }

    #[test]
    fn starts_automatic_and_advances_both_axes() {
        let mut controls = controls();
        assert_eq!(controls.mode(), PlaybackMode::Automatic);
        assert!(controls.advance_automatic().expect("advance should fit"));
        assert_eq!(controls.logical_time(), LogicalTime::from_ticks(10));
        assert_eq!(controls.tau(), Tau::from_ticks(20));
    }

    #[test]
    fn step_controls_pause_and_move_each_axis_both_directions() {
        let mut controls = controls();
        controls
            .step(TimelineAxis::LogicalTime, StepDirection::Forward)
            .expect("logical forward step should fit");
        controls
            .step(TimelineAxis::Tau, StepDirection::Backward)
            .expect("tau backward step should fit");
        assert_eq!(controls.mode(), PlaybackMode::Manual);
        assert_eq!(controls.logical_time(), LogicalTime::from_ticks(100));
        assert_eq!(controls.tau(), Tau::from_ticks(-200));
        assert!(!controls
            .advance_automatic()
            .expect("manual advance is a no-op"));
    }

    #[test]
    fn slider_pointer_enters_manual_mode_and_maps_parabolic_endpoints() {
        let mut controls = controls();
        let slider = controls.layout().logical_slider();
        let point = NormalizedPoint::new(slider.max_x(), (slider.min_y() + slider.max_y()) * 0.5);
        assert_eq!(
            controls
                .pointer_down(point)
                .expect("slider pointer should be valid"),
            PointerTarget::Timeline
        );
        assert_eq!(controls.mode(), PlaybackMode::Manual);
        assert_eq!(controls.logical_time(), LogicalTime::from_ticks(i64::MAX));
        assert!(controls.is_dragging());
        controls
            .pointer_up(NormalizedPoint::new(
                slider.min_x(),
                (slider.min_y() + slider.max_y()) * 0.5,
            ))
            .expect("slider release should be valid");
        assert_eq!(controls.logical_time(), LogicalTime::from_ticks(i64::MIN));
        assert!(!controls.is_dragging());
    }

    #[test]
    fn world_pointer_resumes_automatic_mode() {
        let mut controls = controls();
        controls.pause();
        assert_eq!(
            controls
                .pointer_down(NormalizedPoint::new(0.0, 0.0))
                .expect("world pointer should be valid"),
            PointerTarget::World
        );
        assert_eq!(controls.mode(), PlaybackMode::Automatic);
    }

    #[test]
    fn slider_overflow_is_explicit() {
        let config = TimelineConfig::new(
            ParabolicProjection::new(
                LogicalTime::zero(),
                LogicalTimeDelta::from_ticks(1),
                Tau::zero(),
                TauDelta::from_ticks(1),
            ),
            LogicalTimeDelta::from_ticks(1),
            TauDelta::from_ticks(i64::MAX),
            LogicalTimeDelta::from_ticks(1),
            TauDelta::from_ticks(1),
        );
        let mut controls =
            TimelineControls::new(LogicalTime::zero(), Tau::from_ticks(i64::MAX), config);
        assert_eq!(
            controls.advance_automatic(),
            Err(TimelineError::TauOverflow)
        );
    }

    #[test]
    fn finite_times_stay_inside_the_parabolic_edges() {
        let mut controls = controls();
        controls.set_logical_time(LogicalTime::from_ticks(2_000));
        controls.set_tau(Tau::from_ticks(-2_000));
        assert!(controls.logical_fraction() > 0.35 && controls.logical_fraction() < 1.0);
        assert!(controls.tau_fraction() > 0.0 && controls.tau_fraction() < 0.35);
        controls.set_logical_time(LogicalTime::from_ticks(i64::MAX));
        controls.set_tau(Tau::from_ticks(i64::MIN));
        assert!(controls.logical_fraction() < 1.0);
        assert!(controls.tau_fraction() > 0.0);
        assert_eq!(
            controls.config().projection().logical_time_at_fraction(1.0),
            LogicalTime::from_ticks(i64::MAX)
        );
        assert_eq!(
            controls
                .config()
                .projection()
                .logical_time_at_fraction(f32::NAN),
            LogicalTime::zero()
        );
    }

    #[test]
    fn fixed_focus_places_zero_at_the_left_shifted_sweet_spot() {
        let projection = ParabolicProjection::new(
            LogicalTime::zero(),
            LogicalTimeDelta::from_ticks(1_000),
            Tau::zero(),
            TauDelta::from_ticks(1_000),
        );

        assert!((projection.logical_fraction(LogicalTime::zero()) - 0.35).abs() < 0.001);
        assert_eq!(projection.slider_focus(), SliderFocus::DEFAULT);
    }

    #[test]
    fn programmatic_tau_advance_preserves_automatic_mode() {
        let mut controls = controls();
        assert_eq!(controls.mode(), PlaybackMode::Automatic);
        assert_eq!(controls.advance_tau(Tau::from_ticks(7)), Ok(Tau::from_ticks(7)));
        assert_eq!(controls.mode(), PlaybackMode::Automatic);
    }
}
