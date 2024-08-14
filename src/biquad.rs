type Precision = f64;
use std::f64::consts as C;

pub(crate) struct BiquadCoefficients {
    pub(crate) b0: Precision,
    pub(crate) b1: Precision,
    pub(crate) b2: Precision,
    pub(crate) a1: Precision,
    pub(crate) a2: Precision,
}

pub(crate) struct Biquad {
    coeff: BiquadCoefficients,
    // past input samples, (n - 1) and (n - 2)
    x1: Precision,
    x2: Precision,
    // past output samples, (n - 1) and (n - 2)
    u1: Precision,
    u2: Precision,
}

impl Biquad {
    pub(crate) fn new(coeff: BiquadCoefficients) -> Self {
        Self {
            coeff,
            x1: 0.0,
            x2: 0.0,
            u1: 0.0,
            u2: 0.0,
        }
    }

    pub(crate) fn set_coefficients(&mut self, coeff: BiquadCoefficients) {
        self.coeff = coeff;
    }

    pub(crate) fn is_stable(&self) -> bool {
        // |a1| < 2  &&  |a1| − 1 < a2 < 1
        (self.coeff.a1.abs() < 2.0)
            && ((self.coeff.a1.abs() - 1.0) < self.coeff.a2 && self.coeff.a2 < 1.0)
    }

    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        let u0 = x0 * self.coeff.b0 + self.x1 * self.coeff.b1 + self.x2 * self.coeff.b2
            - self.u1 * self.coeff.a1
            - self.u2 * self.coeff.a2;

        // clear sample if too low in volume
        // if u0 > -1e-10 && u0 < 1e-10 {
        //     u0 = 0.0;
        // }

        self.x2 = self.x1;
        self.x1 = x0;
        self.u2 = self.u1;
        self.u1 = u0;

        u0
    }
}

pub(crate) trait FixedQFilterKind {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients;
}

pub(crate) struct FixedQFilter<T: FixedQFilterKind> {
    biquad: Biquad,
    f: Precision,
    sr: Precision,
    kind: std::marker::PhantomData<T>,
}

impl<T: FixedQFilterKind> FixedQFilter<T> {
    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        self.biquad.process_sample(x0)
    }

    pub(crate) fn new(frequency: Precision, sample_rate: Precision) -> Self {
        let coeffs = T::coefficients(frequency, sample_rate);
        Self {
            biquad: Biquad::new(coeffs),
            f: frequency,
            sr: sample_rate,
            kind: std::marker::PhantomData,
        }
    }

    fn update_coefficients(&mut self) {
        let coeffs = T::coefficients(self.f, self.sr);
        self.biquad.set_coefficients(coeffs);
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        if f == self.f {
            return;
        }

        self.f = f;
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

pub(crate) trait GainlessFilterKind {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> BiquadCoefficients;
}

pub(crate) struct GainlessFilter<T: GainlessFilterKind> {
    biquad: Biquad,
    f: Precision,
    q: Precision,
    sr: Precision,
    kind: std::marker::PhantomData<T>,
}

impl<T: GainlessFilterKind> GainlessFilter<T> {
    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        self.biquad.process_sample(x0)
    }

    pub(crate) fn new(frequency: Precision, q: Precision, sample_rate: Precision) -> Self {
        let coeffs = T::coefficients(frequency, q, sample_rate);
        Self {
            biquad: Biquad::new(coeffs),
            f: frequency,
            q,
            sr: sample_rate,
            kind: std::marker::PhantomData,
        }
    }

    fn update_coefficients(&mut self) {
        let coeffs = T::coefficients(self.f, self.q, self.sr);
        self.biquad.set_coefficients(coeffs);
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

pub(crate) struct ButterworthLP;

impl FixedQFilterKind for ButterworthLP {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let c = 1.0 / (C::PI * f / sr).tan();
        let b0 = 1.0 / (1.0 + C::SQRT_2 * c + c.powi(2));
        let b1 = 2.0 * b0;
        let b2 = b0;
        let a1 = 2.0 * b0 * (1.0 - c.powi(2));
        let a2 = b0 * (1.0 - C::SQRT_2 * c + c.powi(2));

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct LinkwitzRileyLP;

impl FixedQFilterKind for LinkwitzRileyLP {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let th = C::PI * f / sr;
        let wc = C::PI * f;
        let k = wc / th.tan();

        let d = k.powi(2) + wc.powi(2) + 2.0 * k * wc;
        let b0 = wc.powi(2) / d;
        let b1 = 2.0 * wc.powi(2) / d;
        let b2 = b0;
        let a1 = (-2.0 * k.powi(2) + 2.0 * wc.powi(2)) / d;
        let a2 = (-2.0 * k * wc + k.powi(2) + wc.powi(2)) / d;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct LinkwitzRileyHP;

impl FixedQFilterKind for LinkwitzRileyHP {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let th = C::PI * f / sr;
        let wc = C::PI * f;
        let k = wc / th.tan();

        let d = k.powi(2) + wc.powi(2) + 2.0 * k * wc;
        let b0 = k.powi(2) / d;
        let b1 = -2.0 * k.powi(2) / d;
        let b2 = b0;
        let a1 = (-2.0 * k.powi(2) + 2.0 * wc.powi(2)) / d;
        let a2 = (-2.0 * k * wc + k.powi(2) + wc.powi(2)) / d;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct FirstOrderLP;

impl FixedQFilterKind for FirstOrderLP {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let th = 2.0 * C::PI * f / sr;
        let g = th.cos() / (1.0 + th.sin());
        let b0 = (1.0 - g) / 2.0;
        let b1 = (1.0 - g) / 2.0;
        let b2 = 0.0;
        let a1 = -g;
        let a2 = 0.0;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct FirstOrderAP;

impl FixedQFilterKind for FirstOrderAP {
    fn coefficients(f: Precision, sr: Precision) -> BiquadCoefficients {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let b = ((C::PI * f / sr).tan() - 1.0) / ((C::PI * f / sr).tan() + 1.0);
        let b0 = b;
        let b1 = 1.0;
        let b2 = 0.0;
        let a1 = b;
        let a2 = 0.0;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct CookbookLP;

impl GainlessFilterKind for CookbookLP {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> BiquadCoefficients {
        // code from https://github.com/robbert-vdh/nih-plug/blob/master/plugins/crossover/src/crossover/iir/biquad.rs

        let omega0 = C::TAU * (f / sr);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        let b0 = ((1.0 - cos_omega0) / 2.0) / a0;
        let b1 = (1.0 - cos_omega0) / a0;
        let b2 = ((1.0 - cos_omega0) / 2.0) / a0;
        let a1 = (-2.0 * cos_omega0) / a0;
        let a2 = (1.0 - alpha) / a0;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct CookbookHP;

impl GainlessFilterKind for CookbookHP {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> BiquadCoefficients {
        // code from https://github.com/robbert-vdh/nih-plug/blob/master/plugins/crossover/src/crossover/iir/biquad.rs

        let omega0 = C::TAU * (f / sr);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        let b0 = ((1.0 + cos_omega0) / 2.0) / a0;
        let b1 = -(1.0 + cos_omega0) / a0;
        let b2 = ((1.0 + cos_omega0) / 2.0) / a0;
        let a1 = (-2.0 * cos_omega0) / a0;
        let a2 = (1.0 - alpha) / a0;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}

pub(crate) struct CookbookAP;

impl GainlessFilterKind for CookbookAP {
    fn coefficients(f: Precision, q: Precision, sr: Precision) -> BiquadCoefficients {
        // code from https://github.com/robbert-vdh/nih-plug/blob/master/plugins/crossover/src/crossover/iir/biquad.rs

        let omega0 = C::TAU * (f / sr);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        let b0 = (1.0 - alpha) / a0;
        let b1 = (-2.0 * cos_omega0) / a0;
        let b2 = (1.0 + alpha) / a0;
        let a1 = (-2.0 * cos_omega0) / a0;
        let a2 = (1.0 - alpha) / a0;

        BiquadCoefficients { b0, b1, b2, a1, a2 }
    }
}
