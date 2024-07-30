use nih_plug::buffer::{Buffer, ChannelSamples};

type Precision = f64;

struct Biquad {
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
    fn new(b0: Precision, b1: Precision, b2: Precision, a1: Precision, a2: Precision) -> Self {
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

    fn set_coefficients(
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

    fn is_stable(&self) -> bool {
        // |a1| < 2  &&  |a1| − 1 < a2 < 1
        (self.a1.abs() < 2.0) && ((self.a1.abs() - 1.0) < self.a2 && self.a2 < 1.0)
    }

    fn process_sample(&mut self, x0: Precision) -> Precision {
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
