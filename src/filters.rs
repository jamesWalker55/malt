use crate::biquad::{Biquad, Precision, C};

pub(crate) struct ButterworthLPF {
    biquad: Biquad,
    f: Precision,
    sr: Precision,
}

impl ButterworthLPF {
    pub(crate) fn process_sample(&mut self, x0: Precision) -> Precision {
        self.biquad.process_sample(x0)
    }

    pub(crate) fn coefficients(fc: Precision, fs: Precision) -> [Precision; 5] {
        // Code from https://github.com/dimtass/DSP-Cpp-filters
        let c = 1.0 / (C::PI * fc / fs).tan();
        let b0 = 1.0 / (1.0 + C::SQRT_2 * c + c.powi(2));
        let b1 = 2.0 * b0;
        let b2 = b0;
        let a1 = 2.0 * b0 * (1.0 - c.powi(2));
        let a2 = b0 * (1.0 - C::SQRT_2 * c + c.powi(2));

        [b0, b1, b2, a1, a2]
    }

    pub(crate) fn new(frequency: Precision, sample_rate: Precision) -> Self {
        let coeffs = Self::coefficients(frequency, sample_rate);
        Self {
            biquad: Biquad::new(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]),
            f: frequency,
            sr: sample_rate,
        }
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        if f == self.f {
            return;
        }

        self.f = f;
        let coeffs = Self::coefficients(f, self.sr);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }

    pub(crate) fn set_sample_rate(&mut self, sr: Precision) {
        if sr == self.sr {
            return;
        }

        self.sr = sr;
        let coeffs = Self::coefficients(self.f, sr);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }
}
