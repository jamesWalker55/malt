use std::f64::consts::TAU;

pub(crate) trait Oscillator {
    /// Calculates and returns the next sample for this oscillator type.
    /// `phase` is in range 0.0 - 1.0
    fn level(&mut self, phase: f64) -> f64;
}

pub(crate) struct Sine;

impl Oscillator for Sine {
    fn level(&mut self, phase: f64) -> f64 {
        (phase * TAU).sin()
    }
}

pub(crate) struct Triangle;

impl Oscillator for Triangle {
    fn level(&mut self, phase: f64) -> f64 {
        if phase < 0.5 {
            4.0 * phase - 1.0
        } else {
            1.0 - 4.0 * (phase - 0.5)
        }
    }
}

pub(crate) struct Saw {
    multiplier: f64,
}

impl Saw {
    pub(crate) fn new(rising: bool) -> Self {
        if rising {
            Self { multiplier: 1.0 }
        } else {
            Self { multiplier: -1.0 }
        }
    }
}

impl Oscillator for Saw {
    fn level(&mut self, phase: f64) -> f64 {
        ((phase * 2.0) - 1.0) * self.multiplier
    }
}

pub(crate) struct Square;

impl Oscillator for Square {
    fn level(&mut self, phase: f64) -> f64 {
        if phase < 0.5 {
            -1.0
        } else {
            1.0
        }
    }
}

pub(crate) struct Pulse {
    width: f64,
}

impl Pulse {
    pub(crate) fn new(width: f64) -> Self {
        Self { width }
    }
}

impl Oscillator for Pulse {
    fn level(&mut self, phase: f64) -> f64 {
        if phase < self.width {
            -1.0
        } else {
            1.0
        }
    }
}
