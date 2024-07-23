use std::f64::consts::TAU;

pub(crate) trait Oscillator {
    /// Calculates and returns the next sample for this oscillator type.
    /// `phase` is in range 0.0 - 1.0
    fn osc(&mut self, phase: f64) -> f32;
}

pub(crate) struct Sine;

impl Oscillator for Sine {
    fn osc(&mut self, phase: f64) -> f32 {
        (phase * TAU).sin() as f32
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
    fn osc(&mut self, phase: f64) -> f32 {
        (((phase * 2.0) - 1.0) * self.multiplier) as f32
    }
}

pub(crate) struct Square;

impl Oscillator for Square {
    fn osc(&mut self, phase: f64) -> f32 {
        if phase < 0.5 {
            -1.0
        } else {
            1.0
        }
    }
}
