// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What one build-time lowering call costs, over the shared ECMA-262 table.
//!
//! The Rust half of the per-call price in
//! `docs/SCE_LUA_TRANSLATION_SEAM.md`. Its C++ counterpart is
//! `tests/benchmarks/EcmaLoweringPerCallBenchmark.cpp`, and the two are
//! meant to be read from one run of
//! `scripts/measure-lowering-per-call.sh`, which puts them on ONE host
//! inside ONE load window. A cross-machine comparison is not a
//! comparison: the first attempt at this measurement timed C++ locally
//! and Rust on the build machine and had to be thrown away.
//!
//! # Why this is an example and not a test
//!
//! A timing assertion on a shared build machine is a flake generator —
//! this repository runs 32 cores that several sessions divide between
//! them, and the same 21 gates have been measured at 529s and at 1161s.
//! So nothing here asserts a bound. What it does buy, by living in the
//! tree as a compiled target rather than in a throwaway script, is that
//! the number in the document has a command behind it: `cargo` builds
//! this on every `cargo test` of the crate, so it cannot rot silently,
//! and `cargo run --release --example lowering_per_call` reproduces the
//! figure. The first version of this measurement was a probe under
//! `/tmp` that was deleted when the round ended, which left three
//! numbers in a document that nobody could re-derive.
//!
//! # The trap this encodes
//!
//! The C++ side memoises inside the transformer, so a probe that reuses
//! one instance measures a hash lookup. The frontend has no such cache,
//! and this example proves that rather than asserting it: it reports the
//! first pass separately from the steady state, and the two agreeing is
//! what says there is no memo. If they ever diverge, the frontend grew
//! one and the document's comparison needs rewriting.

use std::time::Instant;

use sce_build::ecmascript::{to_lua_condition, to_lua_value, DocumentScope};

/// Passes over the whole table. The table is 98 cases, so the call count
/// is 98 * PASSES; a pass is the unit because it is what makes "first
/// pass vs the rest" answerable.
const PASSES: usize = 1000;

struct Case {
    source: String,
    scope: DocumentScope,
    is_condition: bool,
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf();
    let corpus = root.join("tests/ecmascript/ecma262_semantics.json");
    let text = std::fs::read_to_string(&corpus)
        .unwrap_or_else(|e| panic!("read {}: {}", corpus.display(), e));
    let table: serde_json::Value = serde_json::from_str(&text).expect("the shared table is JSON");
    let raw = table["cases"].as_array().expect("the table has `cases`");

    // The scope is built ONCE per case, outside the timed loop, because
    // that is where a real caller builds it: `DocumentScope::from_model`
    // runs once per document, not once per expression. Timing it here
    // would price a different question — the one the document still
    // records as unmeasured, under "the scope obligation at run time".
    let cases: Vec<Case> = raw
        .iter()
        .map(|c| {
            let mut scope = DocumentScope::installed();
            if let Some(setup) = c.get("setup").and_then(|s| s.as_str()) {
                scope.declare_chunk(setup);
            }
            Case {
                source: c["source"]
                    .as_str()
                    .expect("a case has a source")
                    .to_string(),
                scope,
                is_condition: c.get("form").and_then(|f| f.as_str()) == Some("condition"),
            }
        })
        .collect();

    assert!(
        cases.len() > 50,
        "only {} case(s) read from {} — the corpus walk is broken, and a \
         benchmark over an empty set reports a very good number",
        cases.len(),
        corpus.display()
    );

    // Accumulated so the optimiser cannot delete the call it is here to
    // price. Lowering returns a String; summing its length is enough to
    // keep the result live and costs the same on every pass.
    let mut sink: usize = 0;
    let mut ok = 0usize;
    let mut refused = 0usize;
    let mut first_pass_ns = 0u128;
    let started = Instant::now();

    for pass in 0..PASSES {
        let pass_start = Instant::now();
        for case in &cases {
            let out = if case.is_condition {
                to_lua_condition(&case.source, &case.scope)
            } else {
                to_lua_value(&case.source, &case.scope)
            };
            match out {
                Ok(lua) => {
                    sink = sink.wrapping_add(lua.len());
                    if pass == 0 {
                        ok += 1;
                    }
                }
                Err(_) => {
                    if pass == 0 {
                        refused += 1;
                    }
                }
            }
        }
        if pass == 0 {
            first_pass_ns = pass_start.elapsed().as_nanos();
        }
    }

    let total_ns = started.elapsed().as_nanos();
    let calls = (cases.len() * PASSES) as u128;
    let per_call = total_ns / calls;
    let first_per_call = first_pass_ns / cases.len() as u128;
    // Every pass after the first, which is what "steady state" means.
    let rest_per_call = (total_ns - first_pass_ns) / (calls - cases.len() as u128);

    // One line, in the shape the rest of this repository's gates print
    // their census, so a figure quoted in a document can be grepped out
    // of a run rather than retyped from a screenshot.
    println!(
        "LoweringPerCall census: path=sce-build-frontend population={} passes={} \
         calls={} ns_per_call={} first_pass_ns_per_call={} steady_ns_per_call={} \
         lowered={} refused={} sink={}",
        cases.len(),
        PASSES,
        calls,
        per_call,
        first_per_call,
        rest_per_call,
        ok,
        refused,
        sink
    );
}
