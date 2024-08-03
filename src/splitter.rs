use crate::biquad::{
    CookbookHP, CookbookLP, FixedQFilter, GainlessFilter, LinkwitzRileyHP, LinkwitzRileyLP,
};

type Precision = f64;

pub(crate) struct MinimumTwoBand24Slope {
    lpf1: GainlessFilter<CookbookLP>,
    lpf2: GainlessFilter<CookbookLP>,
    hpf1: GainlessFilter<CookbookHP>,
    hpf2: GainlessFilter<CookbookHP>,
}

impl MinimumTwoBand24Slope {
    fn new(crossover: Precision, sr: Precision) -> Self {
        Self {
            lpf1: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            lpf2: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf1: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf2: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
        }
    }

    fn set_frequency(&mut self, f: Precision) {
        self.lpf1.set_frequency(f.into());
        self.lpf2.set_frequency(f.into());
        self.hpf1.set_frequency(f.into());
        self.hpf2.set_frequency(f.into());
    }

    fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf2.process_sample(self.lpf1.process_sample(sample));
        let high = self.hpf2.process_sample(self.hpf1.process_sample(sample));
        [low, high]
    }

    fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 2]) -> Precision {
        let [low, high] = self.split_bands(sample);
        low * gains[0] + high * gains[1]
    }
}

pub(crate) struct MinimumTwoBand12Slope {
    lpf: FixedQFilter<LinkwitzRileyLP>,
    hpf: FixedQFilter<LinkwitzRileyHP>,
}

impl MinimumTwoBand12Slope {
    fn new(crossover: Precision, sr: Precision) -> Self {
        Self {
            lpf: FixedQFilter::new(crossover, sr),
            hpf: FixedQFilter::new(crossover, sr),
        }
    }

    fn set_frequency(&mut self, f: Precision) {
        self.lpf.set_frequency(f.into());
        self.hpf.set_frequency(f.into());
    }

    fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf.process_sample(sample);
        let high = self.hpf.process_sample(sample);
        [low, -high]
    }

    fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 2]) -> Precision {
        let [low, high] = self.split_bands(sample);
        low * gains[0] + high * gains[1]
    }
}
