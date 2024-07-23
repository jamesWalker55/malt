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

struct Skeleton {
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

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            samplerate: 0.0,
            frequency: 0.0,
            fraction_frequency: 0.0,
            phase: 0.0,
            phase_offset: 0.0,
        }
    }
}

impl Skeleton {
    /// Call this whenever the sine stream should restart, e.g. before note on etc.
    /// Will reset state value to 0.0 and phase value to phaseOffset start value.
    fn reset(&mut self) {
        self.phase = self.phase_offset;
    }

    /// Sets the oscillator sample rate in Hertz.
    fn setSampleRate(&mut self, sr: f64) {
        // Only update and recalculate if new SR value is different
        if sr != self.samplerate {
            // Import number of samples per second
            self.samplerate = sr;

            // If the SR is changed while a Frequency was already set
            if (self.frequency > 0.0) {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency / self.samplerate;
            }

            // Revert to reset state
            self.reset();
        }
    }

    /// Sets the oscillator center frequency in Hertz.
    fn setFrequency(&mut self, hz: f64) {
        // Only update and recalculate if new Hz value is different
        if hz != self.frequency {
            // Import new center frequency
            self.frequency = hz;

            // If the center frequency is changed while SR was already set
            if self.samplerate > 0.0 {
                // Recalculate the per-sample phase modifier
                self.fraction_frequency = self.frequency / self.samplerate;
            }

            // Revert to reset state
            self.reset();
        }
    }

    /// Sets a phase starting offset for the oscillator.
    /// Range is in [0,1] x 1 cycle.
    /// This is NOT the current phase step/state value.
    fn setPhaseOffset(&mut self, Offset: f64) {
        // Only update if new phase offset value is different
        if (Offset != self.phase_offset) {
            self.phase_offset = Offset;
        }
    }

    /// Returns the currently set oscillator sample rate.
    fn getSampleRate(&self) -> f64 {
        return self.samplerate;
    }

    /// Returns the currently set oscillator center frequency.
    fn getFrequency(&self) -> f64 {
        return self.frequency;
    }

    /// Returns the current oscillator phase value.
    /// This is the actual phase step, not the reset offset.
    fn getPhase(&self) -> f64 {
        return self.phase;
    }

    /// Returns the current oscillator phase reset offset.
    fn getPhaseOffset(&self) -> f64 {
        return self.phase_offset;
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
