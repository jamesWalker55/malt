use std::f32::consts::PI;

pub(crate) struct Envelope {
    // I'm storing samples, because the samplerate shouldn't change in the middle of the song
    delay: u32,             // samples
    delay_remaining: u32,   // samples
    attack: u32,            // samples
    attack_remaining: u32,  // samples
    release: u32,           // samples
    release_remaining: u32, // samples
}

impl Envelope {
    /// The function that defines the easing curve
    fn ease(x: f32) -> f32 {
        // https://easings.net/#easeInOutSine
        -((PI * x).cos() - 1.0) / 2.0
    }

    /// Arguments are in seconds
    pub(crate) fn new(sample_rate: f32, delay: f32, attack: f32, release: f32) -> Self {
        let delay = (sample_rate * delay).round() as u32;
        let attack = (sample_rate * attack).round() as u32;
        let release = (sample_rate * release).round() as u32;
        Self {
            delay,
            delay_remaining: delay,
            attack,
            attack_remaining: attack,
            release,
            release_remaining: release,
        }
    }

    fn is_complete(&self) -> bool {
        self.delay_remaining == 0 && self.attack_remaining == 0 && self.release_remaining == 0
    }

    fn duration_samples(&self) -> u32 {
        self.delay + self.attack + self.release
    }

    /// Get the current value (from 0.0 -- 1.0), then increment the state.
    /// If the envelope has completed, return `None`.
    ///
    /// Note: This should be called once per sample.
    pub(crate) fn tick(&mut self) -> Option<f32> {
        if self.delay_remaining != 0 {
            // in delay phase
            self.delay_remaining -= 1;
            Some(0.0)
        } else if self.attack_remaining != 0 {
            // in attack phase
            let x = self.attack_remaining as f32 / self.attack as f32;
            let y = Self::ease(x);

            self.attack_remaining -= 1;

            Some(y)
        } else if self.release_remaining != 0 {
            // in release phase
            let x = self.release_remaining as f32 / self.release as f32;
            let y = Self::ease(x);

            self.release_remaining -= 1;

            Some(y)
        } else {
            // is completed
            None
        }
    }
}
