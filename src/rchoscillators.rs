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

struct Skeleton {
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

impl Default for Skeleton {
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

impl Skeleton {
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

trait Generator {
    /// Calculates and returns the next sample for this oscillator type.
    fn tick(&self) -> f64;

    /// Fills an entire Buffer of DOUBLE samples with the same mono oscillator wave on all channels.
    /// This will overwrite any signal previously in the Buffer.
    fn fill(&self, Buffer: &mut Buffer) {
        for channel_samples in Buffer.iter_samples() {
            // Fill each sample with the next oscillator tick sample
            let tick = self.tick() as f32;
            for sample in channel_samples {
                *sample = tick;
            }
        }

        // The passed Buffer now contains the oscillator signal on all its channels
    }

    /// Adds the same mono oscillator wave to all channels of the passed Buffer of DOUBLE samples.
    /// This will keep any signal previously in the Buffer and add to it.
    fn add(&self, Buffer: &mut Buffer) {
        for channel_samples in Buffer.iter_samples() {
            // Fill each sample with the next oscillator tick sample
            let tick = self.tick() as f32;
            for sample in channel_samples {
                *sample += tick;
            }
        }

        // The passed Buffer now contains its original signal plus the oscillator signal on all channels
    }
}

struct Wrapper {
    generator: Skeleton,
}

impl Wrapper {
    fn new() -> Self {
        Self {
            generator: Skeleton::default(),
        }
    }

    /// Resets the current oscillator state to the start of a new cycle.
    fn reset(&mut self) {
        self.generator.reset();
    }

    /// Sets the oscillator sample rate in Hertz.
    fn setSampleRate(&mut self, SR: f64) {
        self.generator.setSampleRate(SR);
    }

    /// Sets the oscillator center frequency in Hertz.
    fn setFrequency(&mut self, Hz: f64) {
        self.generator.setFrequency(Hz);
    }

    /// Sets the oscillator amplitude as float gain (not dB).
    fn setAmplitude(&mut self, Amplitude: f32) {
        self.generator.setAmplitude(Amplitude);
    }

    /// Sets the oscillator volume in Decibels (use negative values).
    fn setVolume(&mut self, dB: f64) {
        self.generator.setVolume(dB);
    }

    /// Sets the current oscillator sample state to a specific value manually.
    fn setState(&mut self, State: f64) {
        self.generator.setState(State);
    }

    /// Sets a phase starting offset for the oscillator.
    /// Range is in [0,1] x 1 cycle.
    /// This is NOT the current phase step/state value.
    fn setPhaseOffset(&mut self, Offset: f64) {
        self.generator.setPhaseOffset(Offset);
    }

    /// Sets a directional offset for saw oscillator.
    /// Range is [-1;1] where +1 is rising and -1 is falling wave.
    fn setDirection(&mut self, Direction: f64) {
        self.generator.setDirection(Direction);
    }

    /// Sets the pulse width for a pulse wave oscillator.
    /// Range is in [0,1] where 0 = silence and 0.5 = square wave.
    fn setPulseWidth(&mut self, PulseWidth: f64) {
        self.generator.setPulseWidth(PulseWidth);
    }

    /// Sets the pulse width for a square pulse wave oscillator.
    /// Range is in [0,1] where 0 = silence and 1 = square wave.
    fn setWidth(&mut self, PulseWidth: f64) {
        self.generator.setWidth(PulseWidth);
    }

    // /// Band-limited oscillators only! Sets the amount of harmonics that will be
    // /// calculated. Less = lighter on CPU, more = higher precision, default is 7.
    // fn setAccuracy(&mut self, Quality: u32) {
    //     self.generator.setMaxHarmonics(Quality);
    // }

    /// Convenience function to set up most parameters at once.
    /// Accepts an optional double Phase Offset parameter at the end.
    fn setup(&mut self, SR: f64, Hz: f64, Amplitude: f32, Phase: Option<f64>) {
        let Phase = Phase.unwrap_or(0.0);

        self.setSampleRate(SR);
        self.setFrequency(Hz);
        self.setAmplitude(Amplitude);

        if (Phase != 0.0) {
            self.setPhaseOffset(Phase);
        }
    }

    // /// Convenience function to set up most parameters at once.
    // /// Accepts an optional double Phase Offset parameter at the end.
    // fn setup(&mut self, SR: f64, Hz: f64, Volume: f64, Phase: Option<f64>) {
    //     let Phase = Phase.unwrap_or(0.0);

    //     self.setSampleRate(SR);
    //     self.setFrequency(Hz);
    //     self.setVolume(Volume);

    //     if (Phase != 0.0) {
    //         self.setPhaseOffset(Phase);
    //     }
    // }

    /// Returns the currently set oscillator sample rate.
    fn getSampleRate(&self) -> f64 {
        return self.generator.getSampleRate();
    }

    /// Returns the currently set oscillator frequency.
    fn getFrequency(&self) -> f64 {
        return self.generator.getFrequency();
    }

    /// Returns the currently set oscillator amplitude as float gain factor.
    fn getAmplitude(&self) -> f32 {
        return self.generator.getAmplitude();
    }

    /// Returns the currently set oscillator volume in (negative) Decibels.
    fn getVolume(&self) -> f64 {
        return self.generator.getVolume();
    }

    /// Returns the current oscillator sample state.
    /// Does NOT generate a new value, use .tick() for that.
    fn getState(&self) -> f64 {
        return self.generator.getState();
    }

    /// Returns the current oscillator phase value.
    /// This is the actual phase step, not the reset offset.
    fn getPhase(&self) -> f64 {
        return self.generator.getPhase();
    }

    /// Returns the current oscillator phase reset offset.
    fn getPhaseOffset(&self) -> f64 {
        return self.generator.getPhaseOffset();
    }

    /// Returns the current directional offset value.
    /// Only applies to SAW WAVE oscillators.
    fn getDirection(&self) -> f64 {
        return self.generator.getDirection();
    }

    /// Returns the current pulse width modifier value.
    /// Only applies to PULSE WAVE oscillators.
    fn getPulseWidth(&self) -> f64 {
        return self.generator.getPulseWidth();
    }

    /// Returns the current square pulse width modifier value.
    /// Only applies to SQUARE PULSE WAVE oscillators.
    fn getWidth(&self) -> f64 {
        return self.generator.getWidth();
    }

    // fn getAccuracy(&self) -> u32 {
    //     return self.generator.getMaxHarmonics();
    // }
}
