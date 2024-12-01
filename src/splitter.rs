use nih_plug::util::{db_to_gain, gain_to_db};

use crate::{
    biquad::{
        CookbookAP, CookbookHP, CookbookLP, FirstOrderAP, FixedQFilter, GainlessFilter,
        LinkwitzRileyHP, LinkwitzRileyLP,
    },
    svf::{GainFilter, HighShelf, LowShelf},
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
        self.lpf1.set_frequency(f);
        self.lpf2.set_frequency(f);
        self.hpf1.set_frequency(f);
        self.hpf2.set_frequency(f);
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf2.process_sample(self.lpf1.process_sample(sample));
        let high = self.hpf2.process_sample(self.hpf1.process_sample(sample));
        [low, high]
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
        self.lpf.set_frequency(f);
        self.hpf.set_frequency(f);
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 2] {
        let low = self.lpf.process_sample(sample);
        let high = self.hpf.process_sample(sample);
        [low, -high]
    }
}

pub(crate) struct MinimumThreeBand12Slope {
    f1: Precision,
    f2: Precision,
    lpf1: FixedQFilter<LinkwitzRileyLP>,
    hpf1: FixedQFilter<LinkwitzRileyHP>,
    lpf2: FixedQFilter<LinkwitzRileyLP>,
    hpf2: FixedQFilter<LinkwitzRileyHP>,
    apf: FixedQFilter<FirstOrderAP>,
}

impl MinimumThreeBand12Slope {
    pub(crate) fn new(crossover1: Precision, crossover2: Precision, sr: Precision) -> Self {
        Self {
            f1: crossover1,
            f2: crossover2,
            apf: FixedQFilter::new(crossover2, sr),
            lpf1: FixedQFilter::new(crossover1, sr),
            hpf1: FixedQFilter::new(crossover1, sr),
            lpf2: FixedQFilter::new(crossover2, sr),
            hpf2: FixedQFilter::new(crossover2, sr),
        }
    }

    pub(crate) fn set_frequencies(&mut self, f1: Precision, f2: Precision) {
        if self.f1 != f1 {
            self.f1 = f1;
            self.lpf1.set_frequency(f1);
            self.hpf1.set_frequency(f1);
        }

        if self.f2 != f2 {
            self.f2 = f2;
            self.apf.set_frequency(f2);
            self.lpf2.set_frequency(f2);
            self.hpf2.set_frequency(f2);
        }
    }

    pub(crate) fn split_bands(&mut self, sample: Precision) -> [Precision; 3] {
        let low = self.apf.process_sample(self.lpf1.process_sample(sample));
        let midhigh = -self.hpf1.process_sample(sample);
        let mid = self.lpf2.process_sample(midhigh);
        let high = -self.hpf2.process_sample(midhigh);
        [low, mid, high]
    }
}

pub(crate) struct MinimumThreeBand24Slope {
    f1: Precision,
    f2: Precision,
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
            f1: crossover1,
            f2: crossover2,
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
        if self.f1 != f1 {
            self.f1 = f1;
            self.lpf1.set_frequency(f1);
            self.lpf2.set_frequency(f1);
            self.hpf1.set_frequency(f1);
            self.hpf2.set_frequency(f1);
        }

        if self.f2 != f2 {
            self.f2 = f2;
            self.lpf3.set_frequency(f2);
            self.lpf4.set_frequency(f2);
            self.hpf3.set_frequency(f2);
            self.hpf4.set_frequency(f2);
            self.apf.set_frequency(f2);
        }
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
}

pub(crate) struct DynamicThreeBand24Slope {
    lowshelf: GainFilter<LowShelf>,
    highshelf: GainFilter<HighShelf>,
}

impl DynamicThreeBand24Slope {
    pub(crate) fn new(crossover1: Precision, crossover2: Precision, sr: Precision) -> Self {
        Self {
            lowshelf: GainFilter::new(crossover1, std::f64::consts::FRAC_1_SQRT_2, 1.0, sr),
            highshelf: GainFilter::new(crossover2, std::f64::consts::FRAC_1_SQRT_2, 1.0, sr),
        }
    }

    pub(crate) fn set_frequencies(&mut self, f1: Precision, f2: Precision) {
        self.lowshelf.set_frequency(f1);
        self.highshelf.set_frequency(f2);
    }

    pub(crate) fn apply_gain(
        &mut self,
        mut sample: Precision,
        gains: &[Precision; 3],
    ) -> Precision {
        // input gains are scalar, convert to db and do calculations
        let gains_db = gains.map(|x| gain_to_db(x as f32));
        let mid_gain_db = (gains_db[1]).clamp(-60.0, 60.0);
        let high_gain_db_relative = (gains_db[2] - mid_gain_db).clamp(-60.0, 60.0);
        let low_gain_db_relative = (gains_db[0] - mid_gain_db).clamp(-60.0, 60.0);

        // the final scalar gain to use
        let mid_gain = db_to_gain(mid_gain_db) as f64;
        let high_gain_relative = db_to_gain(high_gain_db_relative) as f64;
        let low_gain_relative = db_to_gain(low_gain_db_relative) as f64;

        // process shelf eqs first, using original volume
        self.lowshelf.set_gain(low_gain_relative);
        self.highshelf.set_gain(high_gain_relative);
        sample = self
            .highshelf
            .process_sample(self.lowshelf.process_sample(sample));

        // then process with manual overall gain
        sample *= mid_gain;

        sample
    }
}

pub(crate) struct SingleBand;

impl SingleBand {
    pub(crate) fn apply_gain(&mut self, sample: Precision, gain: &Precision) -> Precision {
        sample * gain
    }
}
