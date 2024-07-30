use crate::biquad::{Biquad, Precision, C};

pub(crate) struct ButterworthLPF {
    biquad: Biquad,
    fc: Precision,
    fs: Precision,
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

    pub(crate) fn new(fc: Precision, fs: Precision) -> Self {
        let coeffs = Self::coefficients(fc, fs);
        Self {
            biquad: Biquad::new(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]),
            fc,
            fs,
        }
    }

    pub(crate) fn set_fc(&mut self, fc: Precision) {
        self.fc = fc;
        let coeffs = Self::coefficients(fc, self.fs);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }

    pub(crate) fn set_fs(&mut self, fs: Precision) {
        self.fs = fs;
        let coeffs = Self::coefficients(self.fc, fs);
        self.biquad
            .set_coefficients(coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    }
}
