// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Thompson's construction over code points (Thompson 1968; Aho, Lam, Sethi and Ullman,
//! Algorithm 3.23), into storage the caller gives.
//!
//! A fragment is a start state and the list of its exits, the out-edges not yet aimed anywhere;
//! the list is threaded through those out-edges themselves, so joining two fragments takes no
//! storage and constant time. The states of the automaton are counted as they are made, written
//! or not: the count is the expression's, and storage past the bound is never written. A
//! fragment holding a state past the storage is `Frag::Beyond`; whatever is built from it is
//! too, so a build that ends with one has more states than the bound and is refused, and a
//! fragment that is only counted costs no work past the bound however large its count.

use core::cmp::Ordering;

use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};
use icu_properties::CodePointMapData;

/// The most states a storage is read as holding: a slot of an exit list is a state's index times
/// two plus one, and stays clear of `LINK`.
const MOST_STATES: usize = 1 << 29;

/// An out-edge aimed nowhere, and the end of an exit list.
pub(crate) const NONE: u32 = u32::MAX;

/// Marks an out-edge on an exit list: the rest of the word is the next slot of the list.
const LINK: u32 = 1 << 31;

/// What a state does with the code point read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Reads this code point.
    Char(u32),
    /// Reads any code point but line feed and carriage return (XSD-2, section F.1).
    Dot,
    /// Reads a code point of the class at this index.
    Class(u32),
    /// Reads nothing and goes both ways.
    Split,
    /// Accepts.
    Match,
}

/// A state of an automaton: what it reads and where it goes. The first out-edge follows a read,
/// and a split takes both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub(crate) kind: Kind,
    pub(crate) out: [u32; 2],
}

impl State {
    /// A state of storage that holds none yet.
    pub const VACANT: State = State {
        kind: Kind::Match,
        out: [NONE, NONE],
    };
}

/// The code points from `first` to `last`, both included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub(crate) first: u32,
    pub(crate) last: u32,
}

impl Range {
    /// A range of storage that holds none yet.
    pub const VACANT: Range = Range { first: 0, last: 0 };
}

/// A class: the ranges its text writes, sorted and merged, and the General Category values its
/// `\p{..}` and `\P{..}` name, as one group. A code point is in the class when it is in a
/// range or its General_Category is in the group, and not, for a class that begins `[^`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Class {
    first: u32,
    len: u32,
    categories: GeneralCategoryGroup,
    negated: bool,
}

impl Class {
    /// A class of storage that holds none yet.
    pub const VACANT: Class = Class {
        first: 0,
        len: 0,
        categories: GeneralCategoryGroup::empty(),
        negated: false,
    };

    /// Whether code point `c` is in the class: a binary search of its ranges, then the
    /// General_Category of `c` in the pinned Unicode data.
    pub(crate) fn contains(&self, ranges: &[Range], c: u32) -> bool {
        let own = &ranges[self.first as usize..(self.first + self.len) as usize];
        let written = own
            .binary_search_by(|range| {
                if range.last < c {
                    Ordering::Less
                } else if range.first > c {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .is_ok();
        let named = self.categories != GeneralCategoryGroup::empty()
            && self
                .categories
                .contains(CodePointMapData::<GeneralCategory>::new().get32(c));
        (written || named) != self.negated
    }
}

/// A part of an automaton under construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Frag {
    /// No state: the language of the empty text alone.
    Empty,
    /// A state past the storage: counted, never written.
    Beyond,
    /// States all in the storage, entered at `start`, with the exit list from slot `head` to
    /// slot `tail`.
    Built { start: u32, head: u32, tail: u32 },
}

/// The slot of out-edge `way` of state `state`.
const fn slot(state: u32, way: u32) -> u32 {
    state * 2 + way
}

/// The fragment whose one exit is the second out-edge of split `state`.
const fn exit_of(state: u32, start: u32) -> Frag {
    Frag::Built {
        start,
        head: slot(state, 1),
        tail: slot(state, 1),
    }
}

/// An out-edge of a state copied `delta` states on: its target, or the next slot of its list.
const fn shifted(out: u32, delta: u32) -> u32 {
    if out == NONE {
        NONE
    } else if out & LINK != 0 {
        LINK | ((out & !LINK) + 2 * delta)
    } else {
        out + delta
    }
}

/// A built fragment copied `delta` states on.
const fn moved(frag: Frag, delta: u32) -> Frag {
    match frag {
        Frag::Built { start, head, tail } => Frag::Built {
            start: start + delta,
            head: head + 2 * delta,
            tail: tail + 2 * delta,
        },
        other => other,
    }
}

/// The state a fragment is entered at, or an out-edge left open for an empty one.
const fn entry(frag: Frag) -> u32 {
    match frag {
        Frag::Built { start, .. } => start,
        Frag::Empty | Frag::Beyond => NONE,
    }
}

/// A fragment's exit list, or `own` alone for an empty one.
const fn exits(frag: Frag, own: u32) -> Option<(u32, u32)> {
    match frag {
        Frag::Built { head, tail, .. } => Some((head, tail)),
        Frag::Empty => Some((own, own)),
        Frag::Beyond => None,
    }
}

/// Sorts `ranges` and merges those that overlap or touch, in place; the count kept.
fn merge(ranges: &mut [Range]) -> usize {
    ranges.sort_unstable_by_key(|range| range.first);
    let mut kept = 0;
    let mut next = 0;
    while next < ranges.len() {
        let range = ranges[next];
        if kept > 0 && range.first <= ranges[kept - 1].last.saturating_add(1) {
            let last = &mut ranges[kept - 1].last;
            *last = (*last).max(range.last);
        } else {
            ranges[kept] = range;
            kept += 1;
        }
        next += 1;
    }
    kept
}

/// An automaton under construction.
pub(crate) struct Builder<'s> {
    states: &'s mut [State],
    classes: &'s mut [Class],
    ranges: &'s mut [Range],
    /// The states made so far, written or not.
    len: u64,
    class_len: usize,
    range_len: usize,
}

/// An automaton built: its storage back, with what was counted.
pub(crate) struct Built<'s> {
    pub(crate) states: &'s mut [State],
    pub(crate) classes: &'s mut [Class],
    pub(crate) ranges: &'s mut [Range],
    /// The states of the automaton, its accepting state among them.
    pub(crate) len: u64,
    /// The states the storage holds: the bound.
    pub(crate) cap: u64,
    pub(crate) class_len: usize,
    pub(crate) range_len: usize,
    pub(crate) start: u32,
    pub(crate) accept: u32,
}

impl<'s> Builder<'s> {
    pub(crate) fn new(
        states: &'s mut [State],
        classes: &'s mut [Class],
        ranges: &'s mut [Range],
    ) -> Self {
        Builder {
            states,
            classes,
            ranges,
            len: 0,
            class_len: 0,
            range_len: 0,
        }
    }

    /// The states made so far: where the next one goes.
    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    /// The ranges written so far: where the next class's first one goes.
    pub(crate) fn ranges_written(&self) -> usize {
        self.range_len
    }

    fn cap(&self) -> u64 {
        self.states.len().min(MOST_STATES) as u64
    }

    /// Makes a state, and writes it when the storage holds it.
    fn emit(&mut self, kind: Kind, out: [u32; 2]) -> Option<u32> {
        let at = self.len;
        self.len = self.len.saturating_add(1);
        if at < self.cap() {
            self.states[at as usize] = State { kind, out };
            Some(at as u32)
        } else {
            None
        }
    }

    fn out_mut(&mut self, slot: u32) -> &mut u32 {
        &mut self.states[(slot / 2) as usize].out[(slot % 2) as usize]
    }

    /// Aims every exit on the list from `head` at state `target`.
    fn patch(&mut self, head: u32, target: u32) {
        let mut at = head;
        loop {
            let out = self.out_mut(at);
            let next = *out;
            *out = target;
            if next == NONE {
                break;
            }
            at = next & !LINK;
        }
    }

    /// Appends the list from `head` to the list that ends at `tail`.
    fn link(&mut self, tail: u32, head: u32) {
        *self.out_mut(tail) = LINK | head;
    }

    /// A state that reads one code point: a character, a dot or a class.
    pub(crate) fn step(&mut self, kind: Kind) -> Frag {
        match self.emit(kind, [NONE, NONE]) {
            Some(state) => Frag::Built {
                start: state,
                head: slot(state, 0),
                tail: slot(state, 0),
            },
            None => Frag::Beyond,
        }
    }

    /// One range of the class being read.
    pub(crate) fn range(&mut self, first: u32, last: u32) {
        if let Some(range) = self.ranges.get_mut(self.range_len) {
            *range = Range { first, last };
        }
        self.range_len += 1;
    }

    /// The class whose ranges were written from index `from`, and its state.
    pub(crate) fn class(
        &mut self,
        from: usize,
        categories: GeneralCategoryGroup,
        negated: bool,
    ) -> Frag {
        let index = self.class_len;
        self.class_len += 1;
        let mut len = self.range_len - from;
        if self.range_len <= self.ranges.len() {
            len = merge(&mut self.ranges[from..self.range_len]);
        }
        if let Some(class) = self.classes.get_mut(index) {
            *class = Class {
                first: from as u32,
                len: len as u32,
                categories,
                negated,
            };
        }
        self.step(Kind::Class(index as u32))
    }

    /// `a` then `b`.
    pub(crate) fn concat(&mut self, a: Frag, b: Frag) -> Frag {
        match (a, b) {
            (Frag::Empty, other) | (other, Frag::Empty) => other,
            (
                Frag::Built { start, head, .. },
                Frag::Built {
                    start: next,
                    head: rest,
                    tail,
                },
            ) => {
                self.patch(head, next);
                Frag::Built {
                    start,
                    head: rest,
                    tail,
                }
            }
            _ => Frag::Beyond,
        }
    }

    /// `a` or `b`: one split.
    pub(crate) fn alternate(&mut self, a: Frag, b: Frag) -> Frag {
        let Some(split) = self.emit(Kind::Split, [entry(a), entry(b)]) else {
            return Frag::Beyond;
        };
        let (Some((head, a_tail)), Some((b_head, tail))) =
            (exits(a, slot(split, 0)), exits(b, slot(split, 1)))
        else {
            return Frag::Beyond;
        };
        self.link(a_tail, b_head);
        Frag::Built {
            start: split,
            head,
            tail,
        }
    }

    /// A split whose first way enters `e`: the split with `e`'s start and exits, or what the
    /// result is when `e` has no state to enter.
    fn split_into(&mut self, e: Frag) -> Result<(u32, u32, u32, u32), Frag> {
        match e {
            Frag::Empty => Err(Frag::Empty),
            Frag::Beyond => {
                self.len = self.len.saturating_add(1);
                Err(Frag::Beyond)
            }
            Frag::Built { start, head, tail } => self
                .emit(Kind::Split, [start, NONE])
                .map(|split| (split, start, head, tail))
                .ok_or(Frag::Beyond),
        }
    }

    /// `e*`.
    pub(crate) fn star(&mut self, e: Frag) -> Frag {
        match self.split_into(e) {
            Ok((split, _, head, _)) => {
                self.patch(head, split);
                exit_of(split, split)
            }
            Err(frag) => frag,
        }
    }

    /// `e+`.
    pub(crate) fn plus(&mut self, e: Frag) -> Frag {
        match self.split_into(e) {
            Ok((split, start, head, _)) => {
                self.patch(head, split);
                exit_of(split, start)
            }
            Err(frag) => frag,
        }
    }

    /// `e?`.
    pub(crate) fn optional(&mut self, e: Frag) -> Frag {
        match self.split_into(e) {
            Ok((split, _, head, tail)) => {
                self.link(tail, slot(split, 1));
                Frag::Built {
                    start: split,
                    head,
                    tail: slot(split, 1),
                }
            }
            Err(frag) => frag,
        }
    }

    /// `e{min,max}`, and `e{min,}` when `max` is `None`, for the atom `e` whose states are
    /// those made from state `first` on. Expanded: `min` copies of `e`, then `max - min`
    /// copies of `e?`, or the last of the `min` copies as `e+` (`e*` when `min` is 0) when
    /// there is no most.
    pub(crate) fn repeat(&mut self, first: u64, e: Frag, min: u64, max: Option<u64>) -> Frag {
        if e == Frag::Empty {
            return Frag::Empty;
        }
        let block = self.len.saturating_sub(first);
        let copies = max.unwrap_or(min.max(1));
        if copies == 0 {
            self.len = first;
            return Frag::Empty;
        }
        let splits = max.map_or(1, |max| max - min);
        let end = first.saturating_add(copies.saturating_mul(block).saturating_add(splits));
        if !matches!(e, Frag::Built { .. }) || end > self.cap() {
            self.len = end;
            return Frag::Beyond;
        }
        // Every copy is made from the block as it was read, before any exit of it is aimed.
        let (first, block) = (first as u32, block as u32);
        for _ in 1..copies {
            self.copy(first, block);
        }
        let mut whole = Frag::Empty;
        for index in 0..copies as u32 {
            let copy = moved(e, index * block);
            let piece = if max.is_none() && u64::from(index) + 1 == copies {
                if min == 0 {
                    self.star(copy)
                } else {
                    self.plus(copy)
                }
            } else if u64::from(index) >= min {
                self.optional(copy)
            } else {
                copy
            };
            whole = self.concat(whole, piece);
        }
        whole
    }

    /// Appends a copy of the `block` states from state `first`, each out-edge moved with it.
    fn copy(&mut self, first: u32, block: u32) {
        let to = self.len as u32;
        let delta = to - first;
        let (from, to, block) = (first as usize, to as usize, block as usize);
        self.states.copy_within(from..from + block, to);
        for state in &mut self.states[to..to + block] {
            state.out = state.out.map(|out| shifted(out, delta));
        }
        self.len += block as u64;
    }

    /// Ends the automaton with its accepting state, every exit of `whole` aimed at it.
    pub(crate) fn finish(mut self, whole: Frag) -> Built<'s> {
        let accept = self.emit(Kind::Match, [NONE, NONE]);
        let start = match (whole, accept) {
            (Frag::Built { start, head, .. }, Some(accept)) => {
                self.patch(head, accept);
                start
            }
            (_, accept) => accept.unwrap_or(NONE),
        };
        let cap = self.cap();
        Built {
            states: self.states,
            classes: self.classes,
            ranges: self.ranges,
            len: self.len,
            cap,
            class_len: self.class_len,
            range_len: self.range_len,
            start,
            accept: accept.unwrap_or(NONE),
        }
    }
}
