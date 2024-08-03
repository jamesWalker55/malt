use crate::{
    biquad::{
        CookbookAP, CookbookHP, CookbookLP, FirstOrderAP, FixedQFilter, GainlessFilter,
        LinkwitzRileyHP, LinkwitzRileyLP,
    },
    svf::{self, AllPass},
};

type Precision = f64;

pub(crate) struct MinimumTwoBand24Slope {
    lpf1: GainlessFilter<CookbookLP>,
    lpf2: GainlessFilter<CookbookLP>,
    hpf1: GainlessFilter<CookbookHP>,
    hpf2: GainlessFilter<CookbookHP>,
}

impl MinimumTwoBand24Slope {
    pub(crate) fn new(crossover: Precision, sr: Precision) -> Self {
        Self {
            lpf1: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            lpf2: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf1: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf2: GainlessFilter::new(crossover, std::f64::consts::FRAC_1_SQRT_2, sr),
        }
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        self.lpf1.set_frequency(f.into());
        self.lpf2.set_frequency(f.into());
        self.hpf1.set_frequency(f.into());
        self.hpf2.set_frequency(f.into());
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf2.process_sample(self.lpf1.process_sample(sample));
        let high = self.hpf2.process_sample(self.hpf1.process_sample(sample));
        [low, high]
    }

    pub(crate) fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 2]) -> Precision {
        let [low, high] = self.split_bands(sample);
        low * gains[0] + high * gains[1]
    }
}

pub(crate) struct MinimumTwoBand12Slope {
    lpf: FixedQFilter<LinkwitzRileyLP>,
    hpf: FixedQFilter<LinkwitzRileyHP>,
}

impl MinimumTwoBand12Slope {
    pub(crate) fn new(crossover: Precision, sr: Precision) -> Self {
        Self {
            lpf: FixedQFilter::new(crossover, sr),
            hpf: FixedQFilter::new(crossover, sr),
        }
    }

    pub(crate) fn set_frequency(&mut self, f: Precision) {
        self.lpf.set_frequency(f.into());
        self.hpf.set_frequency(f.into());
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf.process_sample(sample);
        let high = self.hpf.process_sample(sample);
        [low, -high]
    }

    pub(crate) fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 2]) -> Precision {
        let [low, high] = self.split_bands(sample);
        low * gains[0] + high * gains[1]
    }
}

pub(crate) struct MinimumThreeBand12Slope {
    lpf1: FixedQFilter<LinkwitzRileyLP>,
    hpf1: FixedQFilter<LinkwitzRileyHP>,
    lpf2: FixedQFilter<LinkwitzRileyLP>,
    hpf2: FixedQFilter<LinkwitzRileyHP>,
    apf: FixedQFilter<FirstOrderAP>,
}

impl MinimumThreeBand12Slope {
    pub(crate) fn new(crossover1: Precision, crossover2: Precision, sr: Precision) -> Self {
        Self {
            apf: FixedQFilter::new(crossover2, sr),
            lpf1: FixedQFilter::new(crossover1, sr),
            hpf1: FixedQFilter::new(crossover1, sr),
            lpf2: FixedQFilter::new(crossover2, sr),
            hpf2: FixedQFilter::new(crossover2, sr),
        }
    }

    pub(crate) fn set_frequencies(&mut self, f1: Precision, f2: Precision) {
        self.apf.set_frequency(f2.into());
        self.lpf1.set_frequency(f1.into());
        self.hpf1.set_frequency(f1.into());
        self.lpf2.set_frequency(f2.into());
        self.hpf2.set_frequency(f2.into());
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 3] {
        let low = self.apf.process_sample(self.lpf1.process_sample(sample));
        let midhigh = -self.hpf1.process_sample(sample);
        let mid = self.lpf2.process_sample(midhigh);
        let high = -self.hpf2.process_sample(midhigh);
        [low, mid, high]
    }

    pub(crate) fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 3]) -> Precision {
        let [low, mid, high] = self.split_bands(sample);
        low * gains[0] + mid * gains[1] + high * gains[2]
    }
}

pub(crate) struct MinimumThreeBand24Slope {
    lpf1: GainlessFilter<CookbookLP>,
    lpf2: GainlessFilter<CookbookLP>,
    lpf3: GainlessFilter<CookbookLP>,
    lpf4: GainlessFilter<CookbookLP>,
    hpf1: GainlessFilter<CookbookHP>,
    hpf2: GainlessFilter<CookbookHP>,
    hpf3: GainlessFilter<CookbookHP>,
    hpf4: GainlessFilter<CookbookHP>,
    apf: GainlessFilter<CookbookAP>,
}

impl MinimumThreeBand24Slope {
    pub(crate) fn new(crossover1: Precision, crossover2: Precision, sr: Precision) -> Self {
        Self {
            lpf1: GainlessFilter::new(crossover1, std::f64::consts::FRAC_1_SQRT_2, sr),
            lpf2: GainlessFilter::new(crossover1, std::f64::consts::FRAC_1_SQRT_2, sr),
            lpf3: GainlessFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, sr),
            lpf4: GainlessFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf1: GainlessFilter::new(crossover1, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf2: GainlessFilter::new(crossover1, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf3: GainlessFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, sr),
            hpf4: GainlessFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, sr),
            apf: GainlessFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, sr),
        }
    }

    pub(crate) fn set_frequencies(&mut self, f1: Precision, f2: Precision) {
        self.lpf1.set_frequency(f1.into());
        self.lpf2.set_frequency(f1.into());
        self.lpf3.set_frequency(f2.into());
        self.lpf4.set_frequency(f2.into());
        self.hpf1.set_frequency(f1.into());
        self.hpf2.set_frequency(f1.into());
        self.hpf3.set_frequency(f2.into());
        self.hpf4.set_frequency(f2.into());
        self.apf.set_frequency(f2.into());
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 3] {
        let low = self
            .apf
            .process_sample(self.lpf2.process_sample(self.lpf1.process_sample(sample)));
        let midhigh = self.hpf2.process_sample(self.hpf1.process_sample(sample));
        let mid = self.lpf4.process_sample(self.lpf3.process_sample(midhigh));
        let high = self.hpf4.process_sample(self.hpf3.process_sample(midhigh));
        [low, mid, high]
    }

    pub(crate) fn apply_gain(&mut self, sample: Precision, gains: &[Precision; 3]) -> Precision {
        let [low, mid, high] = self.split_bands(sample);
        low * gains[0] + mid * gains[1] + high * gains[2]
    }
}
