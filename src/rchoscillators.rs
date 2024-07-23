/// A partial port of rchoscillators to Rust:
/// https://github.com/rcliftonharvey/rchoscillators
use std::cmp;

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

mod Decibels {
    use super::*;

    /// Turns Decibels into float gain factor
    pub(crate) fn ToGain(dB: f64) -> f32 {
        return (dB * M_20_LN10).exp() as f32;
    }

    /// Turns float gain factor into Decibels
    pub(crate) fn FromGain(Gain: f32) -> f64 {
        if Gain < 0.0000000298023223876953125 {
            return -150.0;
        }

        let dB = (Gain as f64).ln() * M_LN10_20;

        return dB.max(-150.0);
    }
}

struct SineOsc {
    /// Project samplerate
    samplerate: f64,
    /// Oscillator frequency
    frequency: f64,
    /// frequency / samplerate
    fraction_frequency: f64,

    /// Oscillator volume
    amplitude: f32,

    /// Last oscillator state (output value)
    state: f64,

    /// Last phase position in range [0,1]
    phase: f64,
    /// Phase start offset after oscillator was reset()
    phase_offset: f64,

    /// SAW modifier: 1.0 = rising, -1.0 = falling wave
    direction: f64,
    /// PULSE modifier: pulse width in range [0,1] per half phase
    width: f64,
}

impl Default for SineOsc {
    fn default() -> Self {
        Self {
            samplerate: 0.0,
            frequency: 0.0,
            fraction_frequency: 0.0,
            amplitude: 0.5,
            state: 0.0,
            phase: 0.0,
            phase_offset: 0.0,
            direction: 1.0,
            width: 1.0,
        }
    }
}

impl SineOsc {
    /// Call this whenever the sine stream should restart, e.g. before note on etc.
    /// Will reset state value to 0.0 and phase value to phaseOffset start value.
    fn reset(&mut self) {
        self.state = 0.0;
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

    /// Sets the oscillator amplitude as float gain (not dB).
    fn setAmplitude(&mut self, amp: f32) {
        // Only update if the new amplitude is different
        if (amp != self.amplitude) {
            self.amplitude = amp;
        }
    }

    /// Sets the oscillator volume in Decibels (use negative values).
    fn setVolume(&mut self, dB: f64) {
        // Convert dB to float gain and send to setAmplitude()
        self.setAmplitude(Decibels::ToGain(dB));
    }

    /// Sets the current oscillator sample state to a specific value manually.
    fn setState(&mut self, State: f64) {
        // Only update if the new state value is different from the current state
        if State != self.state {
            self.state = State;
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

    /// Sets a directional offset for saw oscillator.
    /// Range is [-1;1] where +1 is rising and -1 is falling wave.
    fn setDirection(&mut self, Direction: f64) {
        if (Direction != self.direction) {
            self.direction = Direction;
        }
    }

    /// Sets the pulse width for a pulse wave oscillator.
    /// Range is in [0,1] where 0 = silence and 0.5 = square wave.
    fn setPulseWidth(&mut self, Width: f64) {
        if (Width != self.width) {
            self.width = Width;
        }
    }

    /// Sets the pulse width for a square pulse wave oscillator.
    /// Range is in [0,1] where 0 = silence and 1 = square wave.
    fn setWidth(&mut self, Width: f64) {
        // Needs to be halved, since will be used per 1/2 cycle
        let newWidth: f64 = Width * 0.5;

        if (newWidth != self.width) {
            self.width = newWidth;
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

    /// Returns the currently set oscillator amplitude as float gain factor.
    fn getAmplitude(&self) -> f32 {
        return self.amplitude;
    }

    /// Returns the currently set oscillator volume in (negative) Decibels.
    fn getVolume(&self) -> f64 {
        return Decibels::FromGain(self.amplitude);
    }

    /// Returns the current oscillator sample state.
    /// Does NOT generate a new value, use .tick() for that.
    fn getState(&self) -> f64 {
        return self.state;
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

    /// Returns the current directional offset value.
    /// Only applies to SAW WAVE oscillators.
    fn getDirection(&self) -> f64 {
        return self.direction;
    }

    /// Returns the current pulse width modifier value.
    /// Only applies to PULSE WAVE oscillators.
    fn getPulseWidth(&self) -> f64 {
        return self.width;
    }

    /// Returns the current square pulse width modifier value.
    /// Only applies to SQUARE PULSE WAVE oscillators.
    fn getWidth(&self) -> f64 {
        // Must be doubled since stored value is per 1/2 cycle
        return self.width * 2.0;
    }
}
