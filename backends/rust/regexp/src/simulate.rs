// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The simulation of an automaton over a text (Thompson 1968; Cox 2009): a set of states advanced
//! per code point, the set a sparse set (Briggs and Torczon 1993), each state entered at most
//! once per code point. Time O(n * m) for n code points and m states, space five words a state,
//! no heap and no recursion.

use crate::nfa::{Class, Kind, Range, State};
use crate::utf8;

const LINE_FEED: u32 = 0x0A;
const CARRIAGE_RETURN: u32 = 0x0D;

/// What a simulation counts: one step for each state it enters. A step is the unit of its time,
/// at most one per state of the automaton for each code point read, and one more set of steps
/// for the states it starts in.
pub trait Meter {
    /// A state entered.
    fn step(&mut self);
}

/// Counts nothing: the meter of [`Regexp::matches`] and [`Regexp::search`].
impl Meter for () {
    fn step(&mut self) {}
}

/// Counts every step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steps(pub u64);

impl Meter for Steps {
    fn step(&mut self) {
        self.0 += 1;
    }
}

/// A set of states: `dense[..len]` lists them and `sparse[s]` is where `s` is listed, so
/// membership, insertion and emptying each take constant time whatever the words held before.
#[derive(Debug)]
struct Set<'s> {
    dense: &'s mut [u32],
    sparse: &'s mut [u32],
    len: usize,
}

impl<'s> Set<'s> {
    fn new(dense: &'s mut [u32], sparse: &'s mut [u32]) -> Self {
        Set {
            dense,
            sparse,
            len: 0,
        }
    }

    fn contains(&self, state: u32) -> bool {
        let at = self.sparse[state as usize] as usize;
        at < self.len && self.dense[at] == state
    }

    fn insert(&mut self, state: u32) {
        self.sparse[state as usize] = self.len as u32;
        self.dense[self.len] = state;
        self.len += 1;
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// An automaton built by [`crate::compile`], with the storage its simulation keeps.
#[derive(Debug)]
pub struct Regexp<'s> {
    states: &'s [State],
    classes: &'s [Class],
    ranges: &'s [Range],
    start: u32,
    accept: u32,
    now: Set<'s>,
    next: Set<'s>,
    stack: &'s mut [u32],
}

impl<'s> Regexp<'s> {
    /// The automaton of `states`, and its simulation's storage cut from `scratch`, which holds
    /// at least five words a state.
    pub(crate) fn new(
        states: &'s [State],
        classes: &'s [Class],
        ranges: &'s [Range],
        start: u32,
        accept: u32,
        scratch: &'s mut [u32],
    ) -> Self {
        let count = states.len();
        let (now_dense, rest) = scratch.split_at_mut(count);
        let (now_sparse, rest) = rest.split_at_mut(count);
        let (next_dense, rest) = rest.split_at_mut(count);
        let (next_sparse, rest) = rest.split_at_mut(count);
        Regexp {
            states,
            classes,
            ranges,
            start,
            accept,
            now: Set::new(now_dense, now_sparse),
            next: Set::new(next_dense, next_sparse),
            stack: &mut rest[..count],
        }
    }

    /// The states of the automaton, its accepting state among them.
    pub fn states(&self) -> usize {
        self.states.len()
    }

    /// Whether the whole of `text` is in the language (RFC 9535, section 2.4.6). False for a
    /// byte sequence that is not UTF-8.
    pub fn matches(&mut self, text: &[u8]) -> bool {
        self.run(text, false, &mut ())
    }

    /// Whether some part of `text` is in the language, the text around it being any well-formed
    /// text (RFC 9535, section 2.4.7). False for a byte sequence that is not UTF-8, wherever the
    /// part is.
    pub fn search(&mut self, text: &[u8]) -> bool {
        self.run(text, true, &mut ())
    }

    /// [`Regexp::matches`], counting its steps on `meter`.
    pub fn matches_metered(&mut self, text: &[u8], meter: &mut impl Meter) -> bool {
        self.run(text, false, meter)
    }

    /// [`Regexp::search`], counting its steps on `meter`.
    pub fn search_metered(&mut self, text: &[u8], meter: &mut impl Meter) -> bool {
        self.run(text, true, meter)
    }

    /// Reads `text` one code point at a time, decoding its UTF-8 in the same pass. A search
    /// enters the start state again before each code point and at the end (Thompson 1968), so a
    /// part may begin anywhere, and once a part is found it reads the rest only to hold it to
    /// RFC 3629's grammar.
    fn run(&mut self, text: &[u8], search: bool, meter: &mut impl Meter) -> bool {
        self.now.clear();
        enter(self.states, &mut self.now, self.stack, self.start, meter);
        let mut rest = text;
        loop {
            if search && self.now.contains(self.accept) {
                return utf8::well_formed(rest);
            }
            if rest.is_empty() {
                return self.now.contains(self.accept);
            }
            if !search && self.now.is_empty() {
                return false;
            }
            let Some((c, width)) = utf8::decode(rest) else {
                return false;
            };
            rest = &rest[width..];
            self.next.clear();
            for &state in &self.now.dense[..self.now.len] {
                let State { kind, out } = self.states[state as usize];
                let reads = match kind {
                    Kind::Char(want) => want == c,
                    Kind::Dot => c != LINE_FEED && c != CARRIAGE_RETURN,
                    Kind::Class(class) => self.classes[class as usize].contains(self.ranges, c),
                    Kind::Split | Kind::Match => false,
                };
                if reads {
                    enter(self.states, &mut self.next, self.stack, out[0], meter);
                }
            }
            core::mem::swap(&mut self.now, &mut self.next);
            if search {
                enter(self.states, &mut self.now, self.stack, self.start, meter);
            }
        }
    }
}

/// Adds `state` to `set` with every state its splits reach, each once, by a stack rather than
/// recursion; every state is pushed at most once, so the stack holds one word a state.
fn enter(
    states: &[State],
    set: &mut Set<'_>,
    stack: &mut [u32],
    state: u32,
    meter: &mut impl Meter,
) {
    if set.contains(state) {
        return;
    }
    set.insert(state);
    meter.step();
    stack[0] = state;
    let mut top = 1;
    while top > 0 {
        top -= 1;
        let State { kind, out } = states[stack[top] as usize];
        if kind == Kind::Split {
            for next in out {
                if !set.contains(next) {
                    set.insert(next);
                    meter.step();
                    stack[top] = next;
                    top += 1;
                }
            }
        }
    }
}
