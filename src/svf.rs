//! This module is based on:
//! https://github.com/SamiPerttu/fundsp

type Precision = f64;
use std::f64::consts as C;

struct Svf {
    ic1eq: Precision,
    ic2eq: Precision,

    // coefficients
    a1: Precision,
    a2: Precision,
    a3: Precision,
    m0: Precision,
    m1: Precision,
    m2: Precision,
}

impl Svf {
    pub(crate) fn new(
        a1: Precision,
        a2: Precision,
        a3: Precision,
        m0: Precision,
        m1: Precision,
        m2: Precision,
    ) -> Self {
        Self {
            a1,
            a2,
            a3,
            m0,
            m1,
            m2,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    pub(crate) fn set_coefficients(
        &mut self,
        a1: Precision,
        a2: Precision,
        a3: Precision,
        m0: Precision,
        m1: Precision,
        m2: Precision,
    ) {
        self.a1 = a1;
        self.a2 = a2;
        self.a3 = a3;
        self.m0 = m0;
        self.m1 = m1;
        self.m2 = m2;
    }

    pub(crate) fn process_sample(&mut self, v0: Precision) -> Precision {
        let v3 = v0 - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        self.m0 * v0 + self.m1 * v1 + self.m2 * v2
    }
}

pub(crate) trait GainlessFilterKind {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6];
}

/// E.g. low-pass, high-pass, all-pass, notch, peak
pub(crate) struct GainlessFilter<T: GainlessFilterKind> {
    svf: Svf,
    f: Precision,
    q: Precision,
    sr: Precision,
    kind: std::marker::PhantomData<T>,
}

impl<T: GainlessFilterKind> GainlessFilter<T> {
    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        self.svf.process_sample(x0)
    }

    pub(crate) fn new(frequency: Precision, q: Precision, sample_rate: Precision) -> Self {
        let coeffs = T::coefficients(frequency, q, sample_rate);
        Self {
            svf: Svf::new(
                coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5],
            ),
            f: frequency,
            q,
            sr: sample_rate,
            kind: std::marker::PhantomData,
        }
    }

    fn update_coefficients(&mut self) {
        let coeffs = T::coefficients(self.f, self.q, self.sr);
        self.svf.set_coefficients(
            coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5],
        );
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        if f == self.f {
            return;
        }

        self.f = f;
        self.update_coefficients();
    }

    pub(crate) fn set_q(&mut self, q: Precision) {
        if q == self.q {
            return;
        }

        self.q = q;
        self.update_coefficients();
    }

    pub(crate) fn set_sample_rate(&mut self, sr: Precision) {
        if sr == self.sr {
            return;
        }

        self.sr = sr;
        self.update_coefficients();
    }
}

pub(crate) trait GainFilterKind {
    fn coefficients(f: Precision, q: Precision, gain: Precision, sr: Precision) -> [Precision; 6];
}

/// E.g. bell, low-shelf, high-shelf
pub(crate) struct GainFilter<T: GainFilterKind> {
    svf: Svf,
    f: Precision,
    q: Precision,
    gain: Precision,
    sr: Precision,
    kind: std::marker::PhantomData<T>,
}

impl<T: GainFilterKind> GainFilter<T> {
    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        self.svf.process_sample(x0)
    }

    pub(crate) fn new(
        frequency: Precision,
        q: Precision,
        gain: Precision,
        sample_rate: Precision,
    ) -> Self {
        let coeffs = T::coefficients(frequency, q, gain, sample_rate);
        Self {
            svf: Svf::new(
                coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5],
            ),
            f: frequency,
            gain,
            q,
            sr: sample_rate,
            kind: std::marker::PhantomData,
        }
    }

    fn update_coefficients(&mut self) {
        let coeffs = T::coefficients(self.f, self.q, self.gain, self.sr);
        self.svf.set_coefficients(
            coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5],
        );
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        if f == self.f {
            return;
        }

        self.f = f;
        self.update_coefficients();
    }

    pub(crate) fn set_q(&mut self, q: Precision) {
        if q == self.q {
            return;
        }

        self.q = q;
        self.update_coefficients();
    }

    pub(crate) fn set_gain(&mut self, gain: Precision) {
        if gain == self.gain {
            return;
        }

        self.gain = gain;
        self.update_coefficients();
    }

    pub(crate) fn set_sample_rate(&mut self, sr: Precision) {
        if sr == self.sr {
            return;
        }

        self.sr = sr;
        let coeffs = T::coefficients(self.f, self.q, self.gain, sr);
        self.svf.set_coefficients(
            coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5],
        );
    }
}

pub(crate) struct LowPass;

impl GainlessFilterKind for LowPass {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 0.0;
        let m1 = 0.0;
        let m2 = 1.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct HighPass;

impl GainlessFilterKind for HighPass {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = -k;
        let m2 = -1.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct BandPass;

impl GainlessFilterKind for BandPass {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 0.0;
        let m1 = 1.0;
        let m2 = 0.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct Notch;

impl GainlessFilterKind for Notch {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = -k;
        let m2 = 0.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct Peak;

impl GainlessFilterKind for Peak {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = -k;
        let m2 = -2.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct AllPass;

impl GainlessFilterKind for AllPass {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> [Precision; 6] {
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = -2.0 * k;
        let m2 = 0.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct Bell;

impl GainFilterKind for Bell {
    fn coefficients(f: Precision, q: Precision, gain: Precision, sr: Precision) -> [Precision; 6] {
        let a = (gain).sqrt();
        let g = (C::PI * f / sr).tan();
        let k = 1.0 / (q * a);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = k * (a * a - 1.0);
        let m2 = 0.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct LowShelf;

impl GainFilterKind for LowShelf {
    fn coefficients(f: Precision, q: Precision, gain: Precision, sr: Precision) -> [Precision; 6] {
        let a = (gain).sqrt();
        let g = (C::PI * f / sr).tan() / (a).sqrt();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = 1.0;
        let m1 = k * (a - 1.0);
        let m2 = a * a - 1.0;

        [a1, a2, a3, m0, m1, m2]
    }
}

pub(crate) struct HighShelf;

impl GainFilterKind for HighShelf {
    fn coefficients(f: Precision, q: Precision, gain: Precision, sr: Precision) -> [Precision; 6] {
        let a = (gain).sqrt();
        let g = (C::PI * f / sr).tan() * (a).sqrt();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let m0 = a * a;
        let m1 = k * (1.0 - a) * a;
        let m2 = 1.0 - a * a;

        [a1, a2, a3, m0, m1, m2]
    }
}
