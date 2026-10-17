// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every fuzz test of this crate does: feed a parser the corpus and what the mutator makes of it,
//! and fail, naming the input, if it panics.

#![allow(dead_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use sce_rust_assurance::{mutate, mutate_json, Rng};
use serde_json::Value;

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().take(400).map(|b| format!("{b:02x}")).collect()
}

/// A seed that is the same for a target on every machine and differs between targets.
fn seed_of(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Run `target` on every item of `corpus` as it is, and on `rounds` inputs made by changing items of
/// it: a byte-level change, or (when the item is JSON) a change by structure that still parses.
pub fn fuzz(name: &str, corpus: &[Vec<u8>], rounds: usize, mut target: impl FnMut(&[u8])) {
    assert!(!corpus.is_empty(), "{name}: no corpus");
    let mut run = |input: &[u8], what: &str| {
        if catch_unwind(AssertUnwindSafe(|| target(input))).is_err() {
            panic!(
                "{name}: {what}: the input (first 400 bytes, hex) {} made it panic",
                hex(input)
            );
        }
    };
    for (index, item) in corpus.iter().enumerate() {
        run(item, &format!("corpus item {index}"));
    }
    let mut rng = Rng(seed_of(name));
    // `SCE_FUZZ_FACTOR=50` runs fifty times as many rounds: the default is what a push can afford,
    // and a longer run is for a night.
    let factor: usize = std::env::var("SCE_FUZZ_FACTOR")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(1);
    for round in 0..rounds * factor {
        let item = &corpus[rng.below(corpus.len())];
        let input = match serde_json::from_slice::<Value>(item) {
            Ok(value) if rng.below(3) != 0 => {
                mutate_json(&value, &mut rng).to_string().into_bytes()
            }
            _ => mutate(item, &mut rng, corpus),
        };
        run(&input, &format!("round {round}"));
    }
}
