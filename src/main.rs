use nih_plug::prelude::*;
use sai_sampler::SaiSampler;

fn main() {
    nih_export_standalone::<SaiSampler>();
}
