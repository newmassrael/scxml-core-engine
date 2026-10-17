// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Every place the code of a shared list reads what someone else wrote, as a named target.
//!
//! A [`Target`] is a corpus (what the place reads when it is true), a number of rounds a seeded run
//! gives it, and the function that feeds it one input and checks what must hold of the answer: it may
//! refuse, it may not panic, and what it accepts it writes back as it read it. Two drivers use the same
//! targets, so that what one finds the other finds too: the seeded tests of this crate (a mutator over
//! the corpus, the same on every machine, which a push can afford) and the coverage-guided fuzzer under
//! `fuzz/` (libFuzzer, which keeps the inputs that reach new code and is for a night).

mod mls;
mod relay;
mod trust;

use std::cell::RefCell;
use std::collections::HashMap;

/// What a place does with one input, and what it checks of the answer.
type Run = Box<dyn FnMut(&[u8])>;

/// One place that reads what another party wrote.
pub struct Target {
    pub name: &'static str,
    /// How many inputs a seeded run makes of the corpus.
    pub rounds: usize,
    /// What the place reads when it is true.
    pub corpus: Vec<Vec<u8>>,
    run: Run,
}

impl Target {
    fn new(
        name: &'static str,
        rounds: usize,
        corpus: Vec<Vec<u8>>,
        run: impl FnMut(&[u8]) + 'static,
    ) -> Self {
        assert!(!corpus.is_empty(), "{name}: no corpus");
        Self {
            name,
            rounds,
            corpus,
            run: Box::new(run),
        }
    }

    /// Give the place one input. A panic is a finding.
    pub fn feed(&mut self, input: &[u8]) {
        (self.run)(input);
    }
}

/// Every target there is, in the order they are described in the docs.
pub const NAMES: [&str; 21] = [
    "entries",
    "checkpoints",
    "settlements",
    "encoding",
    "operations-hub",
    "operations-member",
    "floors",
    "store",
    "grant",
    "backup",
    "audit",
    "requests",
    "responses",
    "relay",
    "store-file",
    "device-state",
    "welcome",
    "join-commit",
    "apply-commit",
    "key-package",
    "capi",
];

/// The target of `NAMES` that is called `name`, with its fixtures made.
pub fn build(name: &str) -> Target {
    match name {
        "entries" => trust::entries(),
        "checkpoints" => trust::checkpoints(),
        "settlements" => trust::settlements(),
        "encoding" => trust::encodings(),
        "operations-hub" => trust::operations("operations-hub", None),
        "operations-member" => trust::operations("operations-member", Some(3)),
        "floors" => trust::floors(),
        "store" => trust::store(),
        "grant" => trust::grant(),
        "backup" => trust::backup(),
        "audit" => trust::audit(),
        "requests" => relay::requests(),
        "responses" => relay::responses(),
        "relay" => relay::relay(),
        "store-file" => relay::store_file(),
        "device-state" => mls::device_state(),
        "welcome" => mls::welcome(),
        "join-commit" => mls::join_commit(),
        "apply-commit" => mls::apply_commit(),
        "key-package" => mls::key_package(),
        "capi" => mls::capi(),
        other => panic!("no fuzz target is called {other}"),
    }
}

thread_local! {
    static BUILT: RefCell<HashMap<&'static str, Target>> = RefCell::new(HashMap::new());
}

/// Give the target called `name` one input, making the target on the first use in this thread (a
/// coverage-guided fuzzer calls this once per input, and the fixtures are not made again).
pub fn feed(name: &'static str, input: &[u8]) {
    BUILT.with(|built| {
        built
            .borrow_mut()
            .entry(name)
            .or_insert_with(|| build(name))
            .feed(input);
    });
}

fn bytes_of(texts: impl IntoIterator<Item = String>) -> Vec<Vec<u8>> {
    texts.into_iter().map(String::into_bytes).collect()
}

fn text(input: &[u8]) -> String {
    crate::lossy(input)
}
