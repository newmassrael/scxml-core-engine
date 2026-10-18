// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a host holds of one device's operations, and the one place that decides what of it is told to
//! the machine.
//!
//! A host holds **every operation of a device that holds under the log** (its author was a writer under
//! the entry it names, the administrators accepted its number, its signature is the key's): a set, which
//! does not depend on the order the operations were given in. Some of them conflict with each other. A
//! device that signs two operations at one number, one whose chain does not follow itself, a write whose
//! stamp goes back along the chain or that repeats the stamp of another, has signed two histories or
//! used a nonce twice, and the operations that show it are the proof. The proof is kept: it is held, it
//! travels to the hosts that were not given both, it is stored, restored and audited, and what the
//! machine is told is the device's operations **below the lowest position where any two conflict**
//! ([`Chain::frozen`]). That position is a function of the set and of nothing else, so two hosts that
//! were given the same operations hold the same of the device and freeze it at the same place, whichever
//! came first.
//!
//! The position of a conflict is the lower number of the two operations in it: from there nothing of the
//! device is told to the machine, the operation that was told first included, so that a host that saw one
//! of the two first and a host that saw the other first hold the same.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::encoding;
use crate::message::{Link, SIGNATURE_FIELD};

/// The id of an operation: what the next one in its chain names before it.
pub(crate) type Id = [u8; 32];

/// An operation of a device as it travels: the event as its author made it (sealed, when the list keeps
/// content from the hub), where it stands in the device's chain, and the signature the device made of
/// all of it.
#[derive(Clone)]
pub(crate) struct Wire {
    pub data: String,
    pub link: Link,
    pub signature: [u8; 64],
}

impl Wire {
    /// The data of the event as it goes on the wire: the event, its link and its signature.
    pub fn to_json(&self) -> Option<String> {
        let mut value: Value = serde_json::from_str(&self.data).ok()?;
        self.link.write_into(&mut value)?;
        value.as_object_mut()?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&self.signature)),
        );
        Some(value.to_string())
    }
}

/// What a host remembers of an operation of a device that it holds: its id, the one it names before it,
/// and, for a write, its stamp (so that a chain can be checked whatever order its operations arrive in)
/// and what names the write whose content is sealed under it; the key the host finds it by; and the
/// operation itself, as it travels, to give to a host that was not given it.
#[derive(Clone)]
pub(crate) struct Held {
    pub id: Id,
    pub prev: Id,
    /// A write's time and counter, which along one chain do not go back.
    pub stamp: Option<(i64, u64)>,
    /// What a write is sealed under, with the epoch and the list: its id, field, time, counter and
    /// device. A stamp that names two writes is a nonce used twice.
    pub wid: Option<Vec<u8>>,
    /// What the host keeps the operation by: the bytes of the write the machine holds, or the subject
    /// of a note.
    pub key: Vec<u8>,
    pub event: &'static str,
    pub wire: Wire,
}

impl Held {
    pub fn seq(&self) -> u64 {
        self.wire.link.seq
    }
}

/// How two operations of a device conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Why {
    /// Two operations at one number, or one that does not follow the one before it: the device's chain
    /// forked.
    Forked,
    /// A write whose time is before a write earlier in the same chain.
    GoesBack,
    /// Two writes under one stamp: the content of both is sealed under one nonce.
    Repeats,
}

/// The numbers a chain's segments begin and end at: the last operation each earlier grant of the device
/// ended (the cutoffs), in order.
pub(crate) fn segment(seq: u64, cutoffs: &[u64]) -> (u64, u64) {
    let start = cutoffs
        .iter()
        .rev()
        .find(|cutoff| **cutoff < seq)
        .map_or(1, |cutoff| cutoff + 1);
    let end = cutoffs
        .iter()
        .find(|cutoff| **cutoff >= seq)
        .copied()
        .unwrap_or(u64::MAX);
    (start, end)
}

/// What one operation did to a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Inserted {
    /// It was not held: it is held now, as a part of the chain or as proof.
    pub added: bool,
    /// Where the chain was frozen before and where it is now.
    pub before: Option<u64>,
    pub after: Option<u64>,
}

/// Every operation a host holds of one device.
#[derive(Default)]
pub(crate) struct Chain {
    /// The operations by number; more than one at a number when the device signed more than one.
    ops: BTreeMap<u64, Vec<Held>>,
    /// The writes by what they are sealed under: the numbers and ids that carry the stamp.
    stamps: BTreeMap<Vec<u8>, Vec<(u64, Id)>>,
    /// The highest time and counter among the writes of each segment, by the number it begins at.
    highest: BTreeMap<u64, (i64, u64)>,
    /// The lowest position of any conflict among what is held: nothing from there is told to the
    /// machine.
    frozen: Option<u64>,
}

impl Chain {
    /// Where the chain is frozen, when two of its operations conflict.
    pub fn frozen(&self) -> Option<u64> {
        self.frozen
    }

    /// The operations told to the machine, in the order of the chain: the ones below the first
    /// conflict.
    pub fn view(&self) -> impl DoubleEndedIterator<Item = &Held> {
        let below = self.frozen.unwrap_or(u64::MAX);
        self.ops
            .range(..below)
            .flat_map(|(_, held)| held.first().into_iter())
    }

    /// The operations told to the machine that are numbered `from` or higher. It reads what lies there
    /// and no more, so that asking about the end of a long chain costs the end and not the chain.
    pub fn view_from(&self, from: u64) -> impl Iterator<Item = &Held> {
        let below = self.frozen.unwrap_or(u64::MAX);
        let told = (from < below).then(|| self.ops.range(from..below));
        told.into_iter()
            .flatten()
            .flat_map(|(_, held)| held.first().into_iter())
    }

    /// The operations held that are not told to the machine: from the first conflict on, both of every
    /// two that conflict included.
    pub fn evidence(&self) -> impl DoubleEndedIterator<Item = &Held> {
        let from = self.frozen.unwrap_or(u64::MAX);
        self.ops.range(from..).flat_map(|(_, held)| held.iter())
    }

    /// Every operation held, the proof included.
    pub fn all(&self) -> impl DoubleEndedIterator<Item = &Held> {
        self.ops.values().flat_map(|held| held.iter())
    }

    /// The operations held at numbers `from` to `to`, the proof included.
    pub fn range(&self, from: u64, to: u64) -> impl DoubleEndedIterator<Item = &Held> {
        self.ops.range(from..=to).flat_map(|(_, held)| held.iter())
    }

    /// The operation of the chain at `seq` that is told to the machine, when there is one.
    pub fn at(&self, seq: u64) -> Option<&Held> {
        if self.frozen.is_some_and(|from| seq >= from) {
            return None;
        }
        self.ops.get(&seq)?.first()
    }

    /// Whether any operation held at `seq` has the id `id`.
    pub fn has(&self, seq: u64, id: &Id) -> bool {
        self.ops
            .get(&seq)
            .is_some_and(|held| held.iter().any(|one| one.id == *id))
    }

    /// The last operation told to the machine.
    pub fn last(&self) -> Option<&Held> {
        self.view().next_back()
    }

    /// The highest number `n` such that the operations 1 to `n` are all told to the machine.
    pub fn contiguous(&self) -> u64 {
        let mut head = 0;
        while self.at(head + 1).is_some() {
            head += 1;
        }
        head
    }

    /// The numbers of the operations told to the machine.
    pub fn view_numbers(&self) -> Vec<u64> {
        self.view().map(Held::seq).collect()
    }

    /// Take an operation in. The chain is as it would be if every operation it holds had been given in
    /// any other order.
    pub fn insert(&mut self, held: Held, cutoffs: &[u64]) -> Inserted {
        let before = self.frozen;
        if self.has(held.seq(), &held.id) {
            return Inserted {
                added: false,
                before,
                after: before,
            };
        }
        let found = self.conflict_with(&held, cutoffs).map(|(at, _)| at);
        self.put(held, cutoffs);
        if let Some(at) = found {
            self.frozen = Some(self.frozen.map_or(at, |from| from.min(at)));
        }
        Inserted {
            added: true,
            before,
            after: self.frozen,
        }
    }

    /// How `held` would conflict with what is held, were it taken in, and where; the lowest position
    /// when more than one. What a device asks before it signs.
    pub fn probe(&self, held: &Held, cutoffs: &[u64]) -> Option<(u64, Why)> {
        if self.has(held.seq(), &held.id) {
            return None;
        }
        self.conflict_with(held, cutoffs)
    }

    /// Forget the operations `gone` says, and judge the rest again: the position of the first conflict
    /// is the one the set that is left has.
    pub fn remove(&mut self, gone: impl Fn(&Held) -> bool, cutoffs: &[u64]) -> Vec<(u64, Id)> {
        let mut removed = Vec::new();
        let mut kept: Vec<Held> = Vec::new();
        for held in std::mem::take(&mut self.ops).into_values().flatten() {
            if gone(&held) {
                removed.push((held.seq(), held.id));
            } else {
                kept.push(held);
            }
        }
        self.rebuild(kept, cutoffs);
        removed
    }

    fn rebuild(&mut self, mut all: Vec<Held>, cutoffs: &[u64]) {
        self.ops = BTreeMap::new();
        self.stamps = BTreeMap::new();
        self.highest = BTreeMap::new();
        self.frozen = None;
        all.sort_by_key(Held::seq);
        for held in all {
            self.insert(held, cutoffs);
        }
    }

    fn put(&mut self, held: Held, cutoffs: &[u64]) {
        let seq = held.seq();
        if let Some(wid) = &held.wid {
            self.stamps
                .entry(wid.clone())
                .or_default()
                .push((seq, held.id));
        }
        if let Some(stamp) = held.stamp {
            let (start, _) = segment(seq, cutoffs);
            let top = self.highest.entry(start).or_insert(stamp);
            if stamp > *top {
                *top = stamp;
            }
        }
        self.ops.entry(seq).or_default().push(held);
    }

    /// The lowest position at which `held` conflicts with an operation held, and how. Every conflict is
    /// a pair of operations whose position is the lower number of the two, so what this finds for an
    /// operation as it is taken in, over the operations in the order they come, is the lowest position
    /// of the whole set whatever the order.
    fn conflict_with(&self, held: &Held, cutoffs: &[u64]) -> Option<(u64, Why)> {
        let seq = held.seq();
        let (start, end) = segment(seq, cutoffs);
        let mut found: Option<(u64, Why)> = None;
        let mut note = |at: u64, why: Why| {
            if found.is_none_or(|(lowest, _)| at < lowest) {
                found = Some((at, why));
            }
        };
        // Two operations at one number.
        if self
            .ops
            .get(&seq)
            .is_some_and(|others| others.iter().any(|other| other.id != held.id))
        {
            note(seq, Why::Forked);
        }
        // It names an operation before it, and the one held at the number before is another.
        if seq > start {
            if let Some(before) = self.ops.get(&(seq - 1)) {
                if !before.iter().any(|other| other.id == held.prev) {
                    note(seq - 1, Why::Forked);
                }
            }
        }
        // The one held at the next number names another before it.
        if seq < end {
            if let Some(after) = self.ops.get(&(seq + 1)) {
                if after.iter().any(|other| other.prev != held.id) {
                    note(seq, Why::Forked);
                }
            }
        }
        // A write's time does not go back along the chain: a write before it that has a later time, or
        // one after it that has an earlier one.
        if let Some(time) = held.stamp {
            // Before it: the highest time of the segment says whether to look at all, so that an
            // operation that comes at the end of a chain whose times do not go back is not compared
            // with every one before it.
            if self.highest.get(&start).is_some_and(|top| time < *top) {
                if let Some((at, _)) = self.ops.range(start..seq).find(|(_, others)| {
                    others
                        .iter()
                        .any(|other| other.stamp.is_some_and(|earlier| earlier > time))
                }) {
                    note(*at, Why::GoesBack);
                }
            }
            // After it: nothing is held there when it comes at the end.
            if seq < end
                && self.ops.range(seq + 1..=end).any(|(_, others)| {
                    others
                        .iter()
                        .any(|other| other.stamp.is_some_and(|later| later < time))
                })
            {
                note(seq, Why::GoesBack);
            }
        }
        // Two writes sealed under one stamp.
        if let Some(wid) = &held.wid {
            for (other_seq, other_id) in self.stamps.get(wid).into_iter().flatten() {
                if *other_id != held.id {
                    note(seq.min(*other_seq), Why::Repeats);
                }
            }
        }
        found
    }
}
