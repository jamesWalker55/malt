use crate::oscillator::Oscillator;

fn note_to_hz(note: f32) -> f32 {
    55.0 * 2.0_f32.powf((note - 33.0) / 12.0)
}

fn hz_to_note(hz: f32) -> f32 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

pub(crate) struct Voice<S: Oscillator> {
    /// Project samplerate
    samplerate: f32,

    /// The base MIDI note, will be converted to pitch using `note_to_hz`
    base_note: f32,
    /// An additional MIDI note offset to be added to the note, for pitch-wheel etc
    pitch_offset: f32,

    /// Last phase position in range [0,1]
    phase: f32,
    /// Phase start offset after oscillator was reset()
    phase_offset: f32,

    signal: S,

    // These are cache variables based on `samplerate`, `base_note`, and `pitch_offset`.
    // Update these whenever the above variables are changed.
    /// Should be note_to_hz(base_note + pitch_offset)
    frequency: f32,
    /// frequency / samplerate
    fraction_frequency: f32,
}

impl<S: Oscillator> Voice<S> {
    pub(crate) fn new(signal: S, samplerate: f32, note: f32, phase_offset: Option<f32>) -> Self {
        let phase = phase_offset.unwrap_or(0.0) % 1.0;
        let freq = note_to_hz(note);

        debug_assert!(
            samplerate > 0.0,
            "samplerate must be positive, got: {samplerate}",
        );
        debug_assert!(freq > 0.0, "frequency must be positive, got: {}", freq);
        debug_assert!(
            freq < samplerate,
            "frequency must be less than samplerate `{}`, got: {}",
            samplerate,
            freq,
        );
        debug_assert!(
            (0.0..=1.0).contains(&phase),
            "phase must be between 0.0 and 1.0, got: {}",
            phase,
        );

        Self {
            signal,
            samplerate,
            base_note: note,
            pitch_offset: 0.0,
            phase,
            phase_offset: phase,
            frequency: freq,
            fraction_frequency: freq / samplerate,
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
            ((self.phase >= 1.0) as u8 as f32 * -1.0) + ((self.phase < 0.0) as u8 as f32 * 1.0);

        self.signal.level(self.phase) as f32
    }

    pub(crate) fn set_samplerate(&mut self, sr: f32) {
        // Only update and recalculate if new SR value is different
        if sr != self.samplerate {
            self.samplerate = sr;
            self.fraction_frequency = self.frequency / sr;

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

    pub(crate) fn set_base_note(&mut self, note: f32) {
        // Only update and recalculate if new Hz value is different
        if note != self.base_note {
            self.base_note = note;
            self.frequency = note_to_hz(self.base_note + self.pitch_offset);
            self.fraction_frequency = self.frequency / self.samplerate;

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

    pub(crate) fn set_pitch_offset(&mut self, note_offset: f32) {
        // Only update and recalculate if new Hz value is different
        if note_offset != self.pitch_offset {
            self.pitch_offset = note_offset;
            self.frequency = note_to_hz(self.base_note + self.pitch_offset);
            self.fraction_frequency = self.frequency / self.samplerate;

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

    pub(crate) fn set_base_frequency(&mut self, hz: f32) {
        // Only update and recalculate if new Hz value is different
        if hz != self.frequency {
            self.set_base_note(hz_to_note(hz));
        }
    }

    pub(crate) fn set_phase_offset(&mut self, offset: f32) {
        debug_assert!(
            (0.0..=1.0).contains(&offset),
            "phase offset must be between 0.0 and 1.0, got: {offset}",
        );

        // Only update if new phase offset value is different
        if offset != self.phase_offset {
            self.phase_offset = offset;
        }
    }
}
