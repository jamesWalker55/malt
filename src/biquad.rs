type Precision = f64;
use std::f64::consts as C;

pub(crate) struct Biquad {
    b0: Precision,
    b1: Precision,
    b2: Precision,
    a1: Precision,
    a2: Precision,

    // past input samples, (n - 1) and (n - 2)
    x1: Precision,
    x2: Precision,
    // past output samples, (n - 1) and (n - 2)
    u1: Precision,
    u2: Precision,
}

impl Biquad {
    pub(crate) fn new(
        b0: Precision,
        b1: Precision,
        b2: Precision,
        a1: Precision,
        a2: Precision,
    ) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            u1: 0.0,
            u2: 0.0,
        }
    }

    pub(crate) fn set_coefficients(
        &mut self,
        b0: Precision,
        b1: Precision,
        b2: Precision,
        a1: Precision,
        a2: Precision,
    ) {
        self.b0 = b0;
        self.b1 = b1;
        self.b2 = b2;
        self.a1 = a1;
        self.a2 = a2;
    }

    pub(crate) fn is_stable(&self) -> bool {
        // |a1| < 2  &&  |a1| − 1 < a2 < 1
        (self.a1.abs() < 2.0) && ((self.a1.abs() - 1.0) < self.a2 && self.a2 < 1.0)
    }

    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        let u0 = x0 * self.b0 + self.x1 * self.b1 + self.x2 * self.b2
            - self.u1 * self.a1
            - self.u2 * self.a2;

        // clear sample if too low in volume
        // if u0 > -1e-10 && u0 < 1e-10 {
        //     u0 = 0.0;
        // }

        self.x2 = self.x1;
        self.x1 = x0;
        self.u2 = self.u1;
        self.u1 = u0;

        return u0;
    }
}

pub(crate) trait FixedQFilterKind {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5];
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
            biquad: Biquad::new(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]),
            f: frequency,
            sr: sample_rate,
            kind: std::marker::PhantomData,
        }
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        if f == self.f {
            return;
        }

        self.f = f;
        let coeffs = T::coefficients(f, self.sr);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }

    pub(crate) fn set_sample_rate(&mut self, sr: Precision) {
        if sr == self.sr {
            return;
        }

        self.sr = sr;
        let coeffs = T::coefficients(self.f, sr);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }
}

pub(crate) struct ButterworthLP;

impl FixedQFilterKind for ButterworthLP {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5] {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let c = 1.0 / (C::PI * f / sr).tan();
        let b0 = 1.0 / (1.0 + C::SQRT_2 * c + c.powi(2));
        let b1 = 2.0 * b0;
        let b2 = b0;
        let a1 = 2.0 * b0 * (1.0 - c.powi(2));
        let a2 = b0 * (1.0 - C::SQRT_2 * c + c.powi(2));

        [b0, b1, b2, a1, a2]
    }
}

pub(crate) struct LinkwitzRileyLP;

impl FixedQFilterKind for LinkwitzRileyLP {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5] {
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

        [b0, b1, b2, a1, a2]
    }
}

pub(crate) struct LinkwitzRileyHP;

impl FixedQFilterKind for LinkwitzRileyHP {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5] {
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

        [b0, b1, b2, a1, a2]
    }
}

pub(crate) struct FirstOrderLP;

impl FixedQFilterKind for FirstOrderLP {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5] {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let th = 2.0 * C::PI * f / sr;
        let g = th.cos() / (1.0 + th.sin());
        let b0 = (1.0 - g) / 2.0;
        let b1 = (1.0 - g) / 2.0;
        let b2 = 0.0;
        let a1 = -g;
        let a2 = 0.0;

        [b0, b1, b2, a1, a2]
    }
}

pub(crate) struct FirstOrderAP;

impl FixedQFilterKind for FirstOrderAP {
    fn coefficients(f: Precision, sr: Precision) -> [Precision; 5] {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let b = ((C::PI * f / sr).tan() - 1.0) / ((C::PI * f / sr).tan() + 1.0);
        let b0 = b;
        let b1 = 1.0;
        let b2 = 0.0;
        let a1 = b;
        let a2 = 0.0;

        [b0, b1, b2, a1, a2]
    }
}
