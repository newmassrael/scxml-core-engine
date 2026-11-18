// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Seeded randomness: every choice a scenario makes at random comes from one seed, which the
//! scenario prints, so that a failure is drawn again by setting [`SEED_VARIABLE`].

/// The environment variable a scenario takes its seed from, when it is set.
pub const SEED_VARIABLE: &str = "SCE_SERVER_SUITE_SEED";

/// The seed of the scenario `name`: [`SEED_VARIABLE`] when it is set, otherwise one drawn from the
/// clock and the process. Printed either way, so that the run can be drawn again.
pub fn seed_for(name: &str) -> u64 {
    let seed = match std::env::var(SEED_VARIABLE) {
        Ok(text) => text
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{SEED_VARIABLE}={text} is not a seed")),
        Err(_) => {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or(0);
            // The low bits of the clock, mixed with the process, so that two scenarios started in
            // one instant still draw apart.
            (nanos as u64) ^ (u64::from(std::process::id()) << 32)
        }
    };
    eprintln!("{name}: seed {seed} ({SEED_VARIABLE}={seed} draws this run again)");
    seed
}

/// A stream of numbers drawn from one seed (SplitMix64): small, fast, and the same on every
/// platform, which is all a test's choices need.
#[derive(Debug, Clone)]
pub struct Draw {
    state: u64,
}

impl Draw {
    pub fn new(seed: u64) -> Draw {
        Draw { state: seed }
    }

    /// The next number of the stream.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`, or 0 when `bound` is 0.
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }

    /// A number in `low..=high`; `low` when `high` is below it.
    pub fn between(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            return low;
        }
        low + self.below(high - low + 1)
    }

    /// True `percent` times in a hundred.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// One of `items`, which must not be empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let at = self.below(items.len() as u64);
        &items[usize::try_from(at).unwrap_or(0)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_seed_draws_one_stream() {
        let stream = |seed: u64| {
            let mut draw = Draw::new(seed);
            (0..8).map(|_| draw.next_u64()).collect::<Vec<u64>>()
        };
        assert_eq!(stream(7), stream(7));
        assert_ne!(stream(7), stream(8));
    }

    #[test]
    fn a_number_drawn_between_two_stays_between_them() {
        let mut draw = Draw::new(1);
        for _ in 0..1000 {
            let n = draw.between(3, 5);
            assert!((3..=5).contains(&n), "{n}");
        }
        assert_eq!(draw.between(9, 2), 9);
        assert_eq!(draw.below(0), 0);
    }
}
