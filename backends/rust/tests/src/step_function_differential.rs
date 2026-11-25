// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The drive that holds a document printed as its own step function to the
//! same document printed for the shared interpreter (docs/adr/0129).
//!
//! Both forms of one document are driven with one sequence of external events,
//! drawn from the document's own events by a seeded generator, so a sequence
//! is the same on every run and a failure names the seed that reproduces it.
//! Everything a host can observe is written to a journal, one line at a time
//! and in the order it happens:
//!
//! - each transition the engine takes, by the row of its transition table
//!   (docs/adr/0089), from the engine's transition observer;
//! - each call of the host's actions, from the host's own record of them;
//! - after the start, after each event and after the host stops the run: the
//!   configuration, the atomic state, whether the machine runs and where it
//!   ended, and the counts of discarded, unseen and unanswered error events,
//!   which is where an internal or sent event one form took and the other did
//!   not would show.
//!
//! The two journals must be the same line for line. The shared interpreter is
//! the reference: a difference is the step function's to explain.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, StatePolicy};

/// What one drive observed, one line per observation, in order. Shared by the
/// engine's transition observer, the host's actions and the drive.
pub type Journal = Arc<Mutex<Vec<String>>>;

/// An empty journal.
pub fn journal() -> Journal {
    Arc::new(Mutex::new(Vec::new()))
}

/// Write `line` at the end of `journal`.
pub fn note(journal: &Journal, line: String) {
    journal
        .lock()
        .expect("nothing panics while it writes the journal")
        .push(line);
}

/// The lines of `journal`, as they stand.
pub fn lines(journal: &Journal) -> Vec<String> {
    journal
        .lock()
        .expect("nothing panics while it writes the journal")
        .clone()
}

/// SplitMix64 (Steele, Lea and Flood, 2014): a generator whose whole state is
/// one word, so a seed alone fixes the sequence.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_word(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `bound`, which is not zero.
    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a count of events fits a u64");
        usize::try_from(self.next_word() % bound).expect("a number below a count fits a usize")
    }
}

/// `len` external events of `P`'s document, each drawn from every event it
/// names but its "no event", by the generator seeded with `seed`.
pub fn events<P: StatePolicy>(seed: u64, len: usize) -> Vec<&'static str> {
    let null = P::TABLES.null_event.index();
    let names: Vec<&'static str> = P::EVENTS
        .iter()
        .enumerate()
        .filter(|&(index, _)| index != null)
        .map(|(_, &event)| P::get_event_name(event))
        .collect();
    assert!(
        !names.is_empty(),
        "a document with no event has nothing to drive"
    );
    let mut draw = SplitMix64(seed);
    (0..len).map(|_| names[draw.below(names.len())]).collect()
}

/// Start `engine`, hand it each of `events` as an external event and let it
/// run to a stable configuration, then stop it; journal what a host observes
/// along the way.
pub fn drive<P: StatePolicy>(mut engine: Engine<P>, journal: &Journal, events: &[&str]) {
    let observed = Arc::clone(journal);
    engine.set_transition_observer(move |taken| {
        note(
            &observed,
            format!("take {}#{}", P::get_state_name(taken.source), taken.index),
        );
    });
    engine.initialize().expect("the machine starts");
    observe(&engine, journal, "the start");
    for &event in events {
        engine.raise_external_by_name(event, "");
        engine.step();
        observe(&engine, journal, event);
    }
    engine.stop();
    observe(&engine, journal, "the stop");
}

/// Journal what a host reads of `engine` after `what`.
fn observe<P: StatePolicy>(engine: &Engine<P>, journal: &Journal, what: &str) {
    let configuration: Vec<&str> = engine
        .get_active_states()
        .iter()
        .map(|&state| P::get_state_name(state))
        .collect();
    note(
        journal,
        format!(
            "after {what}: configuration {configuration:?}, current {}, running {}, \
             ended in {:?}, discarded {}, unseen {}, unanswered errors {}",
            P::get_state_name(engine.get_current_state()),
            engine.is_running(),
            engine.terminal_state().map(P::get_state_name),
            engine.discarded_external_events(),
            engine.unseen_external_events(),
            engine.unhandled_error_events(),
        ),
    );
}

/// Hold the journal of the printed form to the interpreted form's, line for
/// line, and name the first line they part at. `what` names the document and
/// the seed.
pub fn expect_same_journals(interpreted: &Journal, printed: &Journal, what: &str) {
    let interpreted = lines(interpreted);
    let printed = lines(printed);
    if let Some(at) = interpreted
        .iter()
        .zip(&printed)
        .position(|(expected, actual)| expected != actual)
    {
        let from = at.saturating_sub(3);
        panic!(
            "{what}: the step function parts from the shared interpreter at line {at}\n\
             interpreted: {}\n\
             printed:     {}\n\
             lines before it:\n  {}",
            interpreted[at],
            printed[at],
            interpreted[from..at].join("\n  "),
        );
    }
    assert_eq!(
        interpreted.len(),
        printed.len(),
        "{what}: one form observed more than the other"
    );
}

/// The transitions a journal says were taken, each once: what a set of drives
/// reached of the document's arms.
pub fn transitions_taken(journal: &Journal) -> Vec<String> {
    let mut taken: Vec<String> = lines(journal)
        .into_iter()
        .filter(|line| line.starts_with("take "))
        .collect();
    taken.sort();
    taken.dedup();
    taken
}
