//! The deterministic random number generator: SplitMix64, as published by
//! Sebastiano Vigna (`splitmix64.c`). Only wrapping integer operations, so the
//! same seed gives the same sequence on every platform.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ z.wrapping_shr(30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ z.wrapping_shr(27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ z.wrapping_shr(31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first outputs of the reference `splitmix64.c` for seed 1234567.
    #[test]
    fn matches_the_reference_sequence() {
        let mut rng = Rng::new(1_234_567);
        let outputs: Vec<u64> = (0..5).map(|_| rng.next_u64()).collect();
        assert_eq!(
            outputs,
            [
                6_457_827_717_110_365_317,
                3_203_168_211_198_807_973,
                9_817_491_932_198_370_423,
                4_593_380_528_125_082_431,
                16_408_922_859_458_223_821,
            ]
        );
    }
}
