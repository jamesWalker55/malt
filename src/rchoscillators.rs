/// A partial port of rchoscillators to Rust:
/// https://github.com/rcliftonharvey/rchoscillators
use std::cmp;

use nih_plug::buffer::Buffer;

/// 20 over LN10 (for volume conversion float to dB)
const M_LN10_20: f64 = 8.6858896380650365530225783783321;
/// LN10 over 20 (for volume conversion dB to float)
const M_20_LN10: f64 = 0.11512925464970228420089957273422;
/// PI ... om nom nom
const M_PI: f64 = 3.14159265358979323846264338327950288419716939937510582;
/// 2 * PI ... moar nom
const M_2PI: f64 = 6.283185307179586476925286766559005768394338798750211642;
/// 1/PI
const M_1_PI: f64 = 0.318309886183790671537767526745028724068919291480912898;
/// 2/PI
const M_2_PI: f64 = 0.636619772367581343075535053490057448137838582961825795;
/// 4/PI
const M_4_PI: f64 = 1.27323954473516268615107010698011489627567716592365;
/// 8 / (PI * PI)
const M_8_PIPI: f64 = 0.81056946913870217155103570567782111123487019737797;

/// Turns Decibels into float gain factor
fn db_to_gain(db: f64) -> f32 {
    return (db * M_20_LN10).exp() as f32;
}

/// Turns float gain factor into Decibels
fn gain_to_db(gain: f32) -> f64 {
    if gain < 0.0000000298023223876953125 {
        return -150.0;
    }

    let dB = (gain as f64).ln() * M_LN10_20;

    return dB.max(-150.0);
}

struct PhaseCounter {
    /// Project samplerate
    samplerate: f64,
    /// Oscillator frequency
    frequency: f64,
    /// frequency / samplerate
    fraction_frequency: f64,

    /// Last phase position in range [0,1]
    phase: f64,
    /// Phase start offset after oscillator was reset()
    phase_offset: f64,
}

impl PhaseCounter {
    fn new(samplerate: f64, frequency: f64, phase_offset: Option<f64>) -> Self {
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
            frequency < samplerate,
            "frequency must be less than samplerate `{samplerate}`, got: {frequency}",
        );
        debug_assert!(
            0.0 <= phase && phase <= 1.0,
            "phase must be between 0.0 and 1.0, got: {phase}",
        );

        Self {
            samplerate,
            frequency,
            fraction_frequency: frequency / samplerate,
            phase,
            phase_offset: phase,
        }
    }

    fn reset(&mut self) {
        self.phase = self.phase_offset;
    }

    fn tick(&mut self) {
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
    }

    fn phase(&self) -> f64 {
        return self.phase;
    }

    fn set_samplerate(&mut self, sr: f64) {
        // Only update and recalculate if new SR value is different
        if sr != self.samplerate {
            // Import number of samples per second
            self.samplerate = sr;

            // If the SR is changed while a Frequency was already set
            if (self.frequency > 0.0) {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency / self.samplerate;
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

    fn set_frequency(&mut self, hz: f64) {
        // Only update and recalculate if new Hz value is different
        if hz != self.frequency {
            // Import new center frequency
            self.frequency = hz;

            // If the center frequency is changed while SR was already set
            if self.samplerate > 0.0 {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency / self.samplerate;
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

    fn set_phase_offset(&mut self, offset: f64) {
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

trait Generator {
    /// Calculates and returns the next sample for this oscillator type.
    fn tick(&mut self) -> f64;

    /// Fills an entire Buffer of DOUBLE samples with the same mono oscillator wave on all channels.
    /// This will overwrite any signal previously in the Buffer.
    fn fill(&mut self, Buffer: &mut Buffer) {
        for channel_samples in Buffer.iter_samples() {
            // Fill each sample with the next oscillator tick sample
            let tick = self.tick() as f32;
            for sample in channel_samples {
                *sample = tick;
            }
        }
    }

    /// Adds the same mono oscillator wave to all channels of the passed Buffer of DOUBLE samples.
    /// This will keep any signal previously in the Buffer and add to it.
    fn add(&mut self, Buffer: &mut Buffer) {
        for channel_samples in Buffer.iter_samples() {
            // Fill each sample with the next oscillator tick sample
            let tick = self.tick() as f32;
            for sample in channel_samples {
                *sample += tick;
            }
        }
    }
}

struct Sine {}
