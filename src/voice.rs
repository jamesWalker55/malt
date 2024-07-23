use std::f64::consts::TAU;

pub(crate) trait Signal {
    /// Calculates and returns the next sample for this oscillator type.
    fn gen(&mut self, phase: f64) -> f32;
}

pub(crate) struct Sine;

impl Signal for Sine {
    fn gen(&mut self, phase: f64) -> f32 {
        (phase * TAU).sin() as f32
    }
}

pub(crate) struct Voice<S: Signal> {
    /// Project samplerate
    samplerate: f32,
    /// Oscillator frequency
    frequency: f32,
    /// frequency / samplerate
    fraction_frequency: f64,

    /// Last phase position in range [0,1]
    phase: f64,
    /// Phase start offset after oscillator was reset()
    phase_offset: f64,

    signal: S,
}

impl<S: Signal> Voice<S> {
    pub(crate) fn new(
        signal: S,
        samplerate: f32,
        frequency: f32,
        phase_offset: Option<f64>,
    ) -> Self {
        let phase = phase_offset.unwrap_or(0.0) % 1.0;

        debug_assert!(
            samplerate > 0.0,
            "samplerate must be positive, got: {samplerate}",
        );
        debug_assert!(
            frequency > 0.0,
            "frequency must be positive, got: {frequency}",
        );
        debug_assert!(
            frequency < samplerate.into(),
            "frequency must be less than samplerate `{samplerate}`, got: {frequency}",
        );
        debug_assert!(
            0.0 <= phase && phase <= 1.0,
            "phase must be between 0.0 and 1.0, got: {phase}",
        );

        Self {
            signal,
            samplerate,
            frequency,
            fraction_frequency: frequency as f64 / samplerate as f64,
            phase,
            phase_offset: phase,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.phase = self.phase_offset;
    }

    pub(crate) fn tick(&mut self) -> f32 {
        // Increase phase by +1 step
        self.phase += self.fraction_frequency;

        // Constrain/wrap phase value to sensible boundaries [0,1]
        //
        // if (phase >= 1.0)
        // {
        //     phase -= 1.0;
        // }
        // else if (phase < 0.0)
        // {
        //     phase += 1.0;
        // }
        //
        // IF-branches are slower than simple maths in time critical code, this does the same but faster
        self.phase +=
            ((self.phase >= 1.0) as u8 as f64 * -1.0) + ((self.phase < 0.0) as u8 as f64 * 1.0);

        self.signal.gen(self.phase)
    }

    pub(crate) fn set_samplerate(&mut self, sr: f32) {
        // Only update and recalculate if new SR value is different
        if sr != self.samplerate {
            // Import number of samples per second
            self.samplerate = sr;

            // If the SR is changed while a Frequency was already set
            if (self.frequency > 0.0) {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency as f64 / self.samplerate as f64;
            }

            debug_assert!(
                self.samplerate > 0.0,
                "samplerate must be positive, got: {}",
                self.samplerate
            );
            debug_assert!(
                self.frequency > 0.0,
                "frequency must be positive, got: {}",
                self.frequency,
            );
            debug_assert!(
                self.frequency < self.samplerate,
                "frequency must be less than samplerate `{}`, got: {}",
                self.samplerate,
                self.frequency,
            );
        }
    }

    pub(crate) fn set_frequency(&mut self, hz: f32) {
        // Only update and recalculate if new Hz value is different
        if hz != self.frequency {
            // Import new center frequency
            self.frequency = hz;

            // If the center frequency is changed while SR was already set
            if self.samplerate > 0.0 {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency as f64 / self.samplerate as f64;
            }

            debug_assert!(
                self.samplerate > 0.0,
                "samplerate must be positive, got: {}",
                self.samplerate
            );
            debug_assert!(
                self.frequency > 0.0,
                "frequency must be positive, got: {}",
                self.frequency,
            );
            debug_assert!(
                self.frequency < self.samplerate,
                "frequency must be less than samplerate `{}`, got: {}",
                self.samplerate,
                self.frequency,
            );
        }
    }

    pub(crate) fn set_phase_offset(&mut self, offset: f64) {
        debug_assert!(
            0.0 <= offset && offset <= 1.0,
            "phase offset must be between 0.0 and 1.0, got: {offset}",
        );

        // Only update if new phase offset value is different
        if offset != self.phase_offset {
            self.phase_offset = offset;
        }
    }
}
