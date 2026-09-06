/// A non-negative screen distance measured in physical pixels.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Pixels(u32);

impl Pixels {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn max_one(self) -> Self {
        if self.0 == 0 {
            Self(1)
        } else {
            self
        }
    }
}

/// A pointer position measured in physical pixels from the viewport's top-left.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScreenPoint {
    x: Pixels,
    y: Pixels,
}

impl ScreenPoint {
    pub const fn new(x: u32, y: u32) -> Self {
        Self {
            x: Pixels::new(x),
            y: Pixels::new(y),
        }
    }

    pub const fn from_pixels(x: Pixels, y: Pixels) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> Pixels {
        self.x
    }

    pub const fn y(self) -> Pixels {
        self.y
    }
}

/// A physical viewport used to convert pointer coordinates and auto-scale controls.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Viewport {
    width: Pixels,
    height: Pixels,
}

impl Viewport {
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width: Pixels::new(width),
            height: Pixels::new(height),
        }
    }

    pub const fn from_pixels(width: Pixels, height: Pixels) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> Pixels {
        self.width
    }

    pub const fn height(self) -> Pixels {
        self.height
    }

    pub fn aspect(self) -> f32 {
        self.width.max_one().get() as f32 / self.height.max_one().get() as f32
    }
}

/// A signed logical-time distance, measured in engine ticks.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicalTimeDelta(i64);

impl LogicalTimeDelta {
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    pub const fn ticks(self) -> i64 {
        self.0
    }
}

/// A signed presentation-time distance, measured in engine ticks.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TauDelta(i64);

impl TauDelta {
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    pub const fn ticks(self) -> i64 {
        self.0
    }
}

/// A slider focus position in thousandths of the track width.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SliderFocus(u16);

impl SliderFocus {
    pub const DEFAULT: Self = Self(350);

    pub const fn from_milli(milli: u16) -> Self {
        if milli == 0 {
            Self(1)
        } else if milli >= 1_000 {
            Self(999)
        } else {
            Self(milli)
        }
    }

    pub const fn milli(self) -> u16 {
        self.0
    }

    pub const fn as_f64(self) -> f64 {
        self.0 as f64 / 1_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::{LogicalTimeDelta, Pixels, ScreenPoint, SliderFocus, TauDelta, Viewport};

    #[test]
    fn screen_units_are_explicit_and_stable() {
        let point = ScreenPoint::new(12, 24);
        let viewport = Viewport::new(960, 720);

        assert_eq!(point.x(), Pixels::new(12));
        assert_eq!(point.y(), Pixels::new(24));
        assert_eq!(viewport.width(), Pixels::new(960));
        assert_eq!(viewport.height(), Pixels::new(720));
        assert_eq!(viewport.aspect(), 4.0 / 3.0);
    }

    #[test]
    fn timeline_delta_units_do_not_hide_absolute_time_types() {
        assert_eq!(LogicalTimeDelta::from_ticks(4).ticks(), 4);
        assert_eq!(TauDelta::from_ticks(7).ticks(), 7);
    }

    #[test]
    fn slider_focus_is_a_bounded_fixed_point_unit() {
        assert_eq!(SliderFocus::DEFAULT.milli(), 350);
        assert_eq!(SliderFocus::from_milli(0).milli(), 1);
        assert_eq!(SliderFocus::from_milli(1_000).milli(), 999);
    }
}
