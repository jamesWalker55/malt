use std::f32::consts::PI;

use crate::pattern::Pattern;

#[derive(Debug)]
pub(crate) struct Envelope {
    sr: f32,

    // I'm storing samples, because the samplerate shouldn't change in the middle of the song
    delay: f32,             // samples
    delay_remaining: f32,   // samples
    attack: f32,            // samples
    attack_remaining: f32,  // samples
    release: f32,           // samples
    release_remaining: f32, // samples

    // curves that define this envelope
    attack_curve: Curve,
    release_curve: Curve,
}

impl Envelope {
    /// Arguments are in seconds
    pub(crate) fn new(
        sample_rate: f32,
        delay: f32,
        attack: f32,
        release: f32,
        attack_curve: Curve,
        release_curve: Curve,
    ) -> Self {
        // convert seconds to samples
        let delay = sample_rate * delay;
        let attack = sample_rate * attack;
        let release = sample_rate * release;

        Self {
            sr: sample_rate,
            delay,
            delay_remaining: delay,
            attack,
            attack_remaining: attack,
            release,
            release_remaining: release,
            attack_curve,
            release_curve,
        }
    }

    pub(crate) fn from_latency(
        sr: f32,
        latency_seconds: f32,
        precomp: f32,
        decay: f32,
        attack_curve: Curve,
        release_curve: Curve,
    ) -> Self {
        Self::new(
            sr,
            latency_seconds - precomp,
            precomp,
            decay,
            attack_curve,
            release_curve,
        )
    }

    // This method is not recommended!
    // If you have a multiband setup, where each band has a different attack speed,
    // this may only update some bands' attack and not other bands.
    //
    // /// Update the attack duration of the envelope (in seconds).
    // /// If the envelope is still in delay, this will attempt to decrease the delay and increase the attack, keeping the total sum (delay + attack) the same.
    // /// If the envelope is already in attack, this does nothing.
    // pub(crate) fn set_attack(&mut self, attack: f32) {
    //     // convert seconds to samples
    //     let attack = self.sr * attack;

    //     // do nothing if attack is unchanged
    //     if attack == self.attack {
    //         return;
    //     }

    //     if self.delay_remaining > 0.0 || self.attack == self.attack_remaining {
    //         // still in delay stage || beginning of attack stage, but not done anything yet

    //         if attack >= self.attack {
    //             // new attack is longer
    //             // only allowed if remaining delay is lage enough
    //             let diff = attack - self.attack;
    //             if diff <= self.delay_remaining {
    //                 self.delay_remaining -= diff;
    //                 self.delay -= diff;
    //                 self.attack_remaining += diff;
    //                 self.attack += diff;
    //             }
    //         } else {
    //             // new attack is shorter
    //             // this is always allowed
    //             let diff = self.attack - attack;
    //             self.delay_remaining += diff;
    //             self.delay += diff;
    //             self.attack_remaining -= diff;
    //             self.attack -= diff;
    //         }
    //     } else {
    //         // in attack stage or past it, nothing can be done
    //     }
    // }

    /// Update the release duration of the envelope (in seconds).
    /// If the envelope is still in attack/delay, this will reset the duration
    /// If the envelope is already releasing, only the remaining duration will be affected.
    pub(crate) fn set_release(&mut self, release: f32) {
        // convert seconds to samples
        let release = self.sr * release;

        // do nothing if release is unchanged
        if release == self.release {
            return;
        }

        if (
            // still in attack/delay stage
            self.delay_remaining > 0.0 || self.attack_remaining > 0.0
        ) || (
            // beginning of release stage, but not done anything yet
            self.release == self.release_remaining
        ) {
            // reset the release to the new value
            self.release_remaining = release;
            self.release = release;
        } else if self.release_remaining > 0.0 {
            // in release stage, stretch the remaining release duration
            let ratio = release / self.release;
            self.release_remaining *= ratio;

            // now we can update release as usual
            self.release = release;
        } else {
            // envelope has ended, do nothing
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.delay_remaining <= 0.0 && self.attack_remaining <= 0.0 && self.release_remaining <= 0.0
    }

    pub(crate) fn duration_samples(&self) -> f32 {
        self.delay + self.attack + self.release
    }

    /// Return the progress of this envelope in percentage (0.0 to 1.0)
    pub(crate) fn progress(&self) -> f32 {
        1.0 - ((self.delay_remaining + self.attack_remaining + self.release_remaining)
            / (self.delay + self.attack + self.release))
    }

    /// Get the current value (from 0.0 -- 1.0), then increment the state.
    /// If the envelope has completed, return `None`.
    ///
    /// Note: This should be called once per sample.
    pub(crate) fn tick(&mut self) -> Option<f32> {
        if self.delay_remaining > 0.0 {
            // in delay phase
            self.delay_remaining -= 1.0;
            Some(0.0)
        } else if self.attack_remaining > 0.0 {
            // in attack phase
            let x = 1.0 - self.attack_remaining / self.attack;
            let y = self.attack_curve.get_y(x);

            self.attack_remaining -= 1.0;

            Some(y)
        } else if self.release_remaining > 0.0 {
            // in release phase
            let x = 1.0 - self.release_remaining / self.release;
            let y = 1.0 - self.release_curve.get_y(x);

            self.release_remaining -= 1.0;

            Some(y)
        } else {
            // is completed
            None
        }
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            sr: Default::default(),
            delay: Default::default(),
            delay_remaining: Default::default(),
            attack: Default::default(),
            attack_remaining: Default::default(),
            release: Default::default(),
            release_remaining: Default::default(),
            attack_curve: Curve::EaseInSine,
            release_curve: Curve::EaseInOutSine,
        }
    }
}

/// This should define a graph that starts from 0.0 to 1.0.
#[derive(Debug)]
pub(crate) enum Curve {
    EaseInOutSine,
    EaseInSine,
    Pattern(Pattern),
}

impl Curve {
    /// Range of `x` is 0.0 to 1.0
    ///
    /// Output should be in range 0.0 to 1.0
    fn get_y(&self, x: f32) -> f32 {
        match self {
            Curve::EaseInOutSine => {
                // https://easings.net/#easeInOutSine
                -((PI * x).cos() - 1.0) / 2.0
            }
            Curve::EaseInSine => {
                // https://easings.net/#easeInOutSine
                1.0 - ((x * PI) / 2.0).cos()
            }
            Curve::Pattern(pattern) => pattern.get_y_at(x as f64) as f32,
        }
    }
}
