// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The causal route's delivery layer (docs/adr/0085, sections 3, 3.2, 3.3
//! and 4.2): one copy's side of a causal broadcast group.
//!
//! A change this copy makes is offered here and answered with a verdict:
//! [`Verdict::Accepted`] with the dot the layer tagged it with, or
//! [`Verdict::Refused`] when the log has no room for one more change not yet
//! stable. An accepted change is released to the peers only once the host
//! reports its journal record committed ([`CausalLayer::committed`]), so a
//! number no peer has seen is the only one a crash can lose. A change that
//! arrives is delivered once, and only after every change in its context;
//! one that is not yet deliverable is held, and a gap is repaired by
//! exchanging vectors, never skipped: there is no gap timeout. Every change
//! sent or delivered is kept until the stable cut passes it.
//!
//! Every decision is a call into the generated standard rules
//! (`sce:std/mesh`): whether a change is dropped, delivered or held
//! (`causal_admit`), the merge of two vectors (`causal_raise`), whether a
//! vector lacks a change (`causal_lacks`) and the stable cut (`causal_cut`).
//! What this module adds is the state those rules are applied to and the
//! messages themselves while they are held or logged. It is generic over the
//! message (`M`), which the router instantiates with an envelope's bytes, and
//! transport-free, like the rest of the core.
//!
//! The author of a dot is a replica: one incarnation of one member, a UUID v7
//! the host draws when an incarnation begins and keeps in the copy's journal
//! beside the highest number it used ([`Start`]). A copy that cannot know its
//! highest number begins a new incarnation, never one asked of its peers.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_portable_bytes::SceOwnedList;

use crate::generated::causal_admission::CausalAdmission;
use crate::generated::causal_admit::causal_admit;
use crate::generated::causal_cut::causal_cut;
use crate::generated::causal_entry::CausalEntryPayload;
use crate::generated::causal_known::CausalKnownPayload;
use crate::generated::causal_lacks::causal_lacks;
use crate::generated::causal_raise::causal_raise;

/// One entry of a version vector, and the dot of one change: a replica and
/// a number among its changes (`sce:std/mesh/causal_entry`).
pub type Entry = CausalEntryPayload;

/// The most entries a vector holds: the bound of a view, of the envelope's
/// `causal_vector` and of the rules' lists.
pub const VECTOR_BOUND: usize = 64;

/// A replica identity: one incarnation of one member of a group, a 16-byte
/// UUID v7 unique within the group and never used again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Replica(pub [u8; 16]);

impl Replica {
    /// This replica's entry at `seq`: its identity as the rules hold it, its
    /// first eight bytes and its last eight, big-endian.
    pub fn entry(self, seq: u64) -> Entry {
        let mut hi = [0u8; 8];
        let mut lo = [0u8; 8];
        hi.copy_from_slice(&self.0[..8]);
        lo.copy_from_slice(&self.0[8..]);
        Entry {
            hi: u64::from_be_bytes(hi),
            lo: u64::from_be_bytes(lo),
            seq,
        }
    }

    /// The replica an entry names.
    pub fn of(entry: &Entry) -> Self {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&entry.hi.to_be_bytes());
        id[8..].copy_from_slice(&entry.lo.to_be_bytes());
        Self(id)
    }
}

/// The dot a change is tagged with: its author and its number, from 1 with
/// no gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    pub replica: Replica,
    pub seq: u64,
}

impl Dot {
    /// This dot as an entry of a vector.
    pub fn entry(self) -> Entry {
        self.replica.entry(self.seq)
    }

    /// The dot an entry names.
    pub fn of(entry: &Entry) -> Self {
        Self {
            replica: Replica::of(entry),
            seq: entry.seq,
        }
    }
}

/// A causal binding's group as the deployment gives it: its name and the view
/// of epoch 0, the only view until view changes land (docs/adr/0085, section
/// 3.3). Each entry of `view` names one instance as `<device>/<machine>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CausalGroup {
    pub name: &'static str,
    pub epoch: u64,
    pub view: &'static [&'static str],
}

/// What a tagged envelope or a `causalVector` carries: the view's epoch
/// (envelope key 19) and a vector (key 20) whose first entry is its sender's
/// own. On a change that entry is the change's dot and the rest its context;
/// on a `causalVector` the vector is the sender's delivered vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub epoch: u64,
    pub vector: Vec<Entry>,
}

/// What the host reads from the copy's own journal when a copy starts: the
/// replica identity of its incarnation and its delivered vector, its own
/// entry being the highest number it used.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalRecord {
    pub replica: Replica,
    pub delivered: Vec<Entry>,
}

/// How an incarnation starts (docs/adr/0085, section 3.3.2).
#[derive(Debug, Clone, PartialEq)]
pub enum Start {
    /// A normal restart: the copy's journal says who it is and what it
    /// delivered, and it continues after its highest number.
    Journal(JournalRecord),
    /// A copy that cannot know its own highest number begins a new
    /// incarnation, with an identity the host has just drawn.
    NewIncarnation(Replica),
}

impl Start {
    /// The start of a copy whose journal gave `journal`, or none: a copy
    /// restored from an older save, or set up again from its seed, has no
    /// journal record of its own, and begins under `fresh` instead. It never
    /// asks its peers for its number: the change with the highest one may be
    /// held only by a member that is offline.
    pub fn from_journal(journal: Option<JournalRecord>, fresh: Replica) -> Self {
        match journal {
            Some(record) => Self::Journal(record),
            None => Self::NewIncarnation(fresh),
        }
    }
}

/// The layer's answer to a change its copy offers (docs/adr/0093,
/// section 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Tagged with this dot; the host records it in the journal, then calls
    /// [`CausalLayer::committed`].
    Accepted(Dot),
    /// Not tagged: it took no number, and no copy takes it in.
    Refused(Refusal),
}

/// Why an offer was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The log holds as many changes not yet stable as the deployment allows;
    /// the list tells it as `noRoomForHistory`, and stability frees room.
    NoRoomForHistory,
    /// The incarnation used every number a dot can carry.
    NumbersExhausted,
}

/// Why an offer could not be answered.
#[derive(Debug, Clone, PartialEq)]
pub enum OfferError<E> {
    /// The message for the tagged change could not be made; no number was
    /// taken.
    Build(E),
    /// A rule refused its input.
    Rule(AlgorithmError),
}

/// Why something that arrived was not taken in.
#[derive(Debug, Clone, PartialEq)]
pub enum Rejected {
    /// Tagged with another epoch than the current view's: raised, not
    /// delivered.
    OtherEpoch { epoch: u64 },
    /// Tagged by a replica the view does not hold: raised, not delivered.
    NotInView { replica: Replica },
    /// A vector with no sender entry, or more entries than a view holds.
    Malformed,
    /// A rule refused its input.
    Rule(AlgorithmError),
}

/// A change in the log: its dot, the message as it was sent, and whether it
/// has been released to the peers.
#[derive(Debug, Clone)]
struct Logged<M> {
    dot: Entry,
    message: M,
    released: bool,
}

/// A change held until it is deliverable: its tag and its message.
#[derive(Debug, Clone)]
struct Held<M> {
    dot: Entry,
    context: Vec<Entry>,
    message: M,
}

/// One copy's causal delivery layer for one group.
#[derive(Debug, Clone)]
pub struct CausalLayer<M> {
    group: CausalGroup,
    replica: Replica,
    /// What this copy delivered, its own entry among them.
    delivered: Vec<Entry>,
    /// The latest delivered vector known of each other member, by replica.
    known: BTreeMap<Replica, Vec<Entry>>,
    held: Vec<Held<M>>,
    log: Vec<Logged<M>>,
    /// The most changes the log may hold not yet stable.
    room: usize,
}

impl<M: Clone> CausalLayer<M> {
    /// A layer for `group`, started as `start` says, whose log holds at most
    /// `room` changes not yet stable.
    pub fn new(group: CausalGroup, start: Start, room: usize) -> Self {
        let (replica, mut delivered) = match start {
            Start::Journal(record) => (record.replica, record.delivered),
            Start::NewIncarnation(replica) => (replica, Vec::new()),
        };
        // The copy's own entry is always present, at 0 before its first
        // change, so its vector says who sent it.
        if !delivered.iter().any(|e| Replica::of(e) == replica) {
            delivered.insert(0, replica.entry(0));
        }
        Self {
            group,
            replica,
            delivered,
            known: BTreeMap::new(),
            held: Vec::new(),
            log: Vec::new(),
            room,
        }
    }

    /// The replica identity this incarnation tags its changes with.
    pub fn replica(&self) -> Replica {
        self.replica
    }

    /// What this copy delivered, its own entry among them.
    pub fn delivered(&self) -> &[Entry] {
        &self.delivered
    }

    /// How many changes are held until they are deliverable.
    pub fn held_len(&self) -> usize {
        self.held.len()
    }

    /// How many changes the log keeps until the stable cut passes them.
    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    /// The highest number this incarnation used.
    pub fn own_seq(&self) -> u64 {
        seen(&self.delivered, self.replica)
    }

    /// Offer a change this copy made. When the log has room, the layer tags
    /// it with the next dot and its delivered vector, asks `build` for the
    /// message that carries that tag, counts the change delivered here and
    /// logs it; the message is released by [`Self::committed`]. When the log
    /// has no room the change is refused and takes no number, and when
    /// `build` fails nothing is taken either.
    pub fn offer<E>(
        &mut self,
        build: impl FnOnce(&Tag) -> Result<M, E>,
    ) -> Result<Verdict, OfferError<E>> {
        if self.log.len() >= self.room {
            return Ok(Verdict::Refused(Refusal::NoRoomForHistory));
        }
        let Some(seq) = self.own_seq().checked_add(1) else {
            return Ok(Verdict::Refused(Refusal::NumbersExhausted));
        };
        let dot = self.replica.entry(seq);
        let raised = raise(&self.delivered, &[dot]).map_err(OfferError::Rule)?;
        let tag = self.tag_of(dot, &raised);
        let message = build(&tag).map_err(OfferError::Build)?;
        self.delivered = raised;
        self.log.push(Logged {
            dot,
            message,
            released: false,
        });
        Ok(Verdict::Accepted(Dot {
            replica: self.replica,
            seq,
        }))
    }

    /// The host committed the journal record of `dot`: every accepted change
    /// of this incarnation up to it may now leave, in the order it was
    /// accepted. These are the messages to send to the group.
    pub fn committed(&mut self, dot: Dot) -> Vec<M> {
        if dot.replica != self.replica {
            return Vec::new();
        }
        let replica = self.replica;
        self.log
            .iter_mut()
            .filter(|logged| {
                !logged.released && Replica::of(&logged.dot) == replica && logged.dot.seq <= dot.seq
            })
            .map(|logged| {
                logged.released = true;
                logged.message.clone()
            })
            .collect()
    }

    /// A change that arrived, tagged `tag`: what it lets this copy deliver,
    /// in a causal order. A change delivered already is dropped, the next
    /// change of its author whose context is delivered is delivered with
    /// every held change that then follows, and any other is held. An echo
    /// of this copy's own change is dropped.
    pub fn receive(&mut self, tag: &Tag, message: M) -> Result<Vec<M>, Rejected> {
        let sender = self.sender_of(tag)?;
        let replica = Replica::of(&sender);
        if replica == self.replica {
            return Ok(Vec::new());
        }
        self.admit_member(replica)?;
        self.note_known(replica, &tag.vector)?;
        let mut out = Vec::new();
        match causal_admit(&self.delivered, sender, &tag.vector).map_err(Rejected::Rule)? {
            CausalAdmission::Drop => {}
            CausalAdmission::Hold => {
                let already = self.held.iter().any(|h| {
                    h.dot.hi == sender.hi && h.dot.lo == sender.lo && h.dot.seq == sender.seq
                });
                // A hold past the log's room is let go: the change is still
                // the sender's, and a later vector exchange brings it again.
                if !already && self.held.len() < self.room {
                    self.held.push(Held {
                        dot: sender,
                        context: tag.vector.clone(),
                        message,
                    });
                }
            }
            CausalAdmission::Deliver => {
                self.deliver(sender, message, &mut out)?;
                self.drain_held(&mut out)?;
                self.prune()?;
            }
        }
        Ok(out)
    }

    /// This copy's `causalVector`: the epoch and its delivered vector, its
    /// own entry first. Sent on a period and whenever a link comes back.
    pub fn summary(&self) -> Tag {
        let own = self.replica.entry(self.own_seq());
        self.tag_of(own, &self.delivered)
    }

    /// A member's `causalVector`: recorded as that member's latest known
    /// vector (its mark, which moves the stable cut), and answered with
    /// every released change of the log the vector lacks, in the order the
    /// log holds them (a causal order), as they were sent.
    pub fn receive_summary(&mut self, tag: &Tag) -> Result<Vec<M>, Rejected> {
        let sender = self.sender_of(tag)?;
        let replica = Replica::of(&sender);
        if replica == self.replica {
            return Ok(Vec::new());
        }
        self.admit_member(replica)?;
        self.note_known(replica, &tag.vector)?;
        let answer = self
            .log
            .iter()
            .filter(|logged| logged.released && causal_lacks(&tag.vector, logged.dot))
            .map(|logged| logged.message.clone())
            .collect();
        self.prune()?;
        Ok(answer)
    }

    /// The stable cut: the pointwise minimum, over the view's members, of the
    /// latest delivered vector known from each, with a member not heard from
    /// at 0 (`causal_cut`).
    pub fn stable_cut(&self) -> Result<Vec<Entry>, Rejected> {
        let mut known: Vec<CausalKnownPayload> = Vec::new();
        let rows = core::iter::once((self.replica, self.delivered.as_slice())).chain(
            self.known
                .iter()
                .map(|(replica, vector)| (*replica, vector.as_slice())),
        );
        for (holder, vector) in rows {
            let id = holder.entry(0);
            known.extend(vector.iter().map(|e| CausalKnownPayload {
                holder_hi: id.hi,
                holder_lo: id.lo,
                hi: e.hi,
                lo: e.lo,
                seq: e.seq,
            }));
        }
        let members = u32::try_from(self.group.view.len()).unwrap_or(u32::MAX);
        let mut cut: SceOwnedList<Entry, VECTOR_BOUND> = SceOwnedList::new();
        causal_cut(members, &known, &mut cut)
            .map_err(|_| Rejected::Rule(AlgorithmError::CapacityExceeded))?;
        Ok(cut.as_slice().to_vec())
    }

    /// The tag a message of this copy carries: the epoch, and `own` first,
    /// then every other entry of `vector`.
    fn tag_of(&self, own: Entry, vector: &[Entry]) -> Tag {
        let mut entries = Vec::with_capacity(vector.len());
        entries.push(own);
        entries.extend(
            vector
                .iter()
                .filter(|e| Replica::of(e) != self.replica)
                .copied(),
        );
        Tag {
            epoch: self.group.epoch,
            vector: entries,
        }
    }

    /// The sender entry of a tag of this view, or why it is not one.
    fn sender_of(&self, tag: &Tag) -> Result<Entry, Rejected> {
        if tag.epoch != self.group.epoch {
            return Err(Rejected::OtherEpoch { epoch: tag.epoch });
        }
        if tag.vector.len() > VECTOR_BOUND {
            return Err(Rejected::Malformed);
        }
        tag.vector.first().copied().ok_or(Rejected::Malformed)
    }

    /// Hold `replica` to the view: a member already heard from, or one more
    /// while the view has a member no replica holds yet. Until view changes
    /// land the view is epoch 0's, and each of its members is held by the
    /// first replica that presents itself for it; a replica past the view's
    /// size is a second incarnation of a member, which is not delivered.
    fn admit_member(&self, replica: Replica) -> Result<(), Rejected> {
        if self.known.contains_key(&replica) || 1 + self.known.len() < self.group.view.len() {
            Ok(())
        } else {
            Err(Rejected::NotInView { replica })
        }
    }

    /// Raise the latest vector known of `replica` by `vector`.
    fn note_known(&mut self, replica: Replica, vector: &[Entry]) -> Result<(), Rejected> {
        let current = self.known.get(&replica).map(Vec::as_slice).unwrap_or(&[]);
        let raised = raise(current, vector).map_err(Rejected::Rule)?;
        self.known.insert(replica, raised);
        Ok(())
    }

    /// Deliver the change `dot`: raise the delivered vector by it, log it and
    /// hand its message on.
    fn deliver(&mut self, dot: Entry, message: M, out: &mut Vec<M>) -> Result<(), Rejected> {
        self.delivered = raise(&self.delivered, &[dot]).map_err(Rejected::Rule)?;
        self.log.push(Logged {
            dot,
            message: message.clone(),
            released: true,
        });
        out.push(message);
        Ok(())
    }

    /// Deliver every held change that is now deliverable, each after the ones
    /// its context names, and let go of every held change now delivered.
    fn drain_held(&mut self, out: &mut Vec<M>) -> Result<(), Rejected> {
        loop {
            let mut next = None;
            let mut index = 0;
            while index < self.held.len() {
                let held = &self.held[index];
                match causal_admit(&self.delivered, held.dot, &held.context)
                    .map_err(Rejected::Rule)?
                {
                    CausalAdmission::Drop => {
                        self.held.remove(index);
                    }
                    CausalAdmission::Deliver => {
                        next = Some(index);
                        break;
                    }
                    CausalAdmission::Hold => index += 1,
                }
            }
            let Some(index) = next else {
                return Ok(());
            };
            let held = self.held.remove(index);
            self.deliver(held.dot, held.message, out)?;
        }
    }

    /// Drop every logged change the stable cut covers: it is delivered at
    /// every member of the view, so no repair can ask for it.
    fn prune(&mut self) -> Result<(), Rejected> {
        let cut = self.stable_cut()?;
        self.log.retain(|logged| causal_lacks(&cut, logged.dot));
        Ok(())
    }
}

/// `vector` raised by `other` (`causal_raise`).
fn raise(vector: &[Entry], other: &[Entry]) -> Result<Vec<Entry>, AlgorithmError> {
    let mut out: SceOwnedList<Entry, VECTOR_BOUND> = SceOwnedList::new();
    causal_raise(vector, other, &mut out).map_err(|_| AlgorithmError::CapacityExceeded)?;
    Ok(out.as_slice().to_vec())
}

/// The number `vector` gives `replica`, 0 when it names none.
fn seen(vector: &[Entry], replica: Replica) -> u64 {
    vector
        .iter()
        .find(|e| Replica::of(e) == replica)
        .map_or(0, |e| e.seq)
}

// The tests of docs/adr/0085, section 9.2 for the Rust core. Each names the
// change to the rule that turns it red.
#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;

    const VIEW: CausalGroup = CausalGroup {
        name: "people",
        epoch: 0,
        view: &["phone/list", "laptop/list", "tablet/list"],
    };
    const PAIR: CausalGroup = CausalGroup {
        name: "pair",
        epoch: 0,
        view: &["phone/list", "laptop/list"],
    };

    /// A message as the tests carry it: the tag it was sent with and a name.
    type Msg = (Tag, u32);

    fn replica(n: u8) -> Replica {
        Replica([n; 16])
    }

    fn layer(group: CausalGroup, n: u8, room: usize) -> CausalLayer<Msg> {
        CausalLayer::new(group, Start::NewIncarnation(replica(n)), room)
    }

    /// Offer the change `name` and commit its journal record: the message
    /// that leaves for the group.
    fn make(at: &mut CausalLayer<Msg>, name: u32) -> Msg {
        let verdict = at
            .offer(|tag| Ok::<Msg, ()>((tag.clone(), name)))
            .expect("an offer is answered");
        let Verdict::Accepted(dot) = verdict else {
            panic!("expected the change accepted, got {verdict:?}");
        };
        let mut sent = at.committed(dot);
        assert_eq!(sent.len(), 1, "one change released per commit");
        sent.remove(0)
    }

    /// Give `message` to `at`: the names it delivers, in order.
    fn give(at: &mut CausalLayer<Msg>, message: &Msg) -> Vec<u32> {
        at.receive(&message.0, message.clone())
            .expect("a change of the view is taken in")
            .into_iter()
            .map(|(_, name)| name)
            .collect()
    }

    /// Give `to` the `causalVector` of `from`: the names `to` answers with.
    fn exchange(from: &CausalLayer<Msg>, to: &mut CausalLayer<Msg>) -> Vec<Msg> {
        to.receive_summary(&from.summary())
            .expect("a vector of the view is taken in")
    }

    fn column(at: &CausalLayer<Msg>, of: Replica) -> u64 {
        seen(at.delivered(), of)
    }

    /// Drop. Red if `dot.seq <= seen` becomes `<` in causal_admit: the
    /// change given again is then held, not dropped.
    #[test]
    fn a_change_given_again_is_dropped_and_not_held() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let c1 = make(&mut p, 1);
        assert_eq!(give(&mut q, &c1), [1]);
        assert_eq!(give(&mut q, &c1), Vec::<u32>::new());
        assert_eq!(q.held_len(), 0);
        assert_eq!(column(&q, replica(1)), 1);
    }

    /// Deliver. Red if the delivered vector is not raised by the change
    /// delivered: the author's next change is then held.
    #[test]
    fn the_next_change_is_delivered_once_and_raises_its_authors_column() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        assert_eq!(give(&mut q, &c1), [1]);
        assert_eq!(column(&q, replica(1)), 1);
        assert_eq!(give(&mut q, &c2), [2]);
        assert_eq!(column(&q, replica(1)), 2);
    }

    /// Hold, a gap of the author. Red if `dot.seq === seen + 1` is weakened
    /// to `dot.seq > seen`: the change past the gap is then delivered first.
    #[test]
    fn a_change_past_a_gap_of_its_author_is_held_until_the_gap_fills() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        assert_eq!(give(&mut q, &c2), Vec::<u32>::new());
        assert_eq!(q.held_len(), 1);
        assert_eq!(give(&mut q, &c1), [1, 2]);
        assert_eq!(q.held_len(), 0);
    }

    /// Hold, an uncovered context. Red if the check `C[b] <= D[b]` is
    /// removed from causal_admit: the change is then delivered before the
    /// change of another author it depends on.
    #[test]
    fn a_change_whose_context_is_not_delivered_is_held_until_it_is() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let mut r = layer(VIEW, 3, 16);
        let c1 = make(&mut p, 1);
        assert_eq!(give(&mut r, &c1), [1]);
        let d1 = make(&mut r, 10);
        assert_eq!(give(&mut q, &d1), Vec::<u32>::new());
        assert_eq!(give(&mut q, &c1), [1, 10]);
    }

    /// No gap timeout. Red if anything other than the change before it
    /// releases a held change, as a gap timeout that fast-forwards the
    /// author's column would.
    #[test]
    fn a_held_change_stays_held_however_many_vectors_pass() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let mut r = layer(VIEW, 3, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        assert_eq!(give(&mut q, &c2), Vec::<u32>::new());
        for _ in 0..1000 {
            exchange(&r, &mut q);
            exchange(&q, &mut r);
        }
        assert_eq!(q.held_len(), 1);
        assert_eq!(column(&q, replica(1)), 0);
        assert_eq!(give(&mut q, &c1), [1, 2]);
    }

    /// Repair by vector. Red if what a vector lacks is computed as empty:
    /// the lost change is then never sent again.
    #[test]
    fn a_lost_change_is_received_after_one_vector_exchange() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let _lost = make(&mut p, 1);
        let answer = exchange(&q, &mut p);
        assert_eq!(answer.len(), 1);
        assert_eq!(give(&mut q, &answer[0]), [1]);
    }

    /// The stable cut. Red if the minimum becomes a maximum (the cut is then
    /// {p:2}), or if a member whose vector is unknown is left out (the cut
    /// is then not empty before the third member is heard from).
    #[test]
    fn the_stable_cut_is_the_minimum_over_the_view_with_an_unknown_member_at_zero() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let mut r = layer(VIEW, 3, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        give(&mut q, &c1);
        give(&mut q, &c2);
        exchange(&q, &mut p);
        assert_eq!(p.stable_cut().unwrap(), Vec::<Entry>::new());
        give(&mut r, &c1);
        exchange(&r, &mut p);
        assert_eq!(p.stable_cut().unwrap(), [replica(1).entry(1)]);
    }

    /// The mark. Red if a received vector is not recorded as its member's
    /// latest: members that never wrote then never move the cut, and the
    /// log keeps the change.
    #[test]
    fn members_that_never_wrote_move_the_cut_by_their_vectors_alone() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let mut r = layer(VIEW, 3, 16);
        let c1 = make(&mut p, 1);
        give(&mut q, &c1);
        give(&mut r, &c1);
        assert_eq!(p.log_len(), 1);
        exchange(&q, &mut p);
        exchange(&r, &mut p);
        assert_eq!(p.log_len(), 0);
    }

    /// Retention. Red if the log is pruned at the copy's own delivered
    /// vector instead of the cut: the change a member still lacks is then
    /// gone, and its repair is not answered.
    #[test]
    fn the_log_drops_a_change_at_the_cut_and_answers_a_repair_before_it() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let mut r = layer(VIEW, 3, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        give(&mut q, &c1);
        give(&mut q, &c2);
        give(&mut r, &c1);
        exchange(&q, &mut p);
        let answer = exchange(&r, &mut p);
        assert_eq!(p.log_len(), 1, "c1 is stable, c2 is not");
        assert_eq!(answer.len(), 1);
        assert_eq!(give(&mut r, &answer[0]), [2]);
    }

    /// The verdict. Red if the number is taken before the refusal: the next
    /// accepted change then leaves a gap, and the peer holds it.
    #[test]
    fn an_offer_past_the_room_is_refused_and_takes_no_number() {
        let mut p = layer(PAIR, 1, 1);
        let mut q = layer(PAIR, 2, 16);
        let c1 = make(&mut p, 1);
        let refused = p.offer(|tag| Ok::<Msg, ()>((tag.clone(), 99))).unwrap();
        assert_eq!(refused, Verdict::Refused(Refusal::NoRoomForHistory));
        assert_eq!(p.own_seq(), 1);
        assert_eq!(give(&mut q, &c1), [1]);
        exchange(&q, &mut p);
        assert_eq!(p.log_len(), 0, "stability frees room");
        let c2 = make(&mut p, 2);
        assert_eq!(c2.0.vector[0], replica(1).entry(2));
        assert_eq!(give(&mut q, &c2), [2]);
    }

    /// Write-ahead. Red if the layer releases a change when it tags it: a
    /// repair asked before the journal commit is then answered with it.
    #[test]
    fn an_accepted_change_leaves_only_once_its_journal_record_is_committed() {
        let mut p = layer(VIEW, 1, 16);
        let q = layer(VIEW, 2, 16);
        let verdict = p.offer(|tag| Ok::<Msg, ()>((tag.clone(), 1))).unwrap();
        let Verdict::Accepted(dot) = verdict else {
            panic!("expected the change accepted, got {verdict:?}");
        };
        assert_eq!(dot.seq, 1);
        assert!(exchange(&q, &mut p).is_empty());
        let sent = p.committed(dot);
        assert_eq!(sent.len(), 1);
        assert!(p.committed(dot).is_empty(), "a change is released once");
        assert_eq!(exchange(&q, &mut p).len(), 1);
    }

    /// Incarnation. Red if a copy without its journal reuses its previous
    /// identity, or continues from a number any peer reports: its next
    /// change then carries the dot of the change an offline member holds.
    #[test]
    fn a_copy_without_its_journal_begins_a_new_incarnation() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let c1 = make(&mut p, 1);
        let c2 = make(&mut p, 2);
        give(&mut q, &c1);
        // c2 reached only the tablet, which is offline now. The phone is
        // restored from a save taken after c1: it cannot know its highest.
        // A copy whose journal is its own continues from it.
        let record = JournalRecord {
            replica: replica(1),
            delivered: alloc::vec![replica(1).entry(2)],
        };
        assert_eq!(
            Start::from_journal(Some(record.clone()), replica(9)),
            Start::Journal(record.clone())
        );
        let mut resumed = CausalLayer::new(VIEW, Start::Journal(record), 16);
        assert_eq!(make(&mut resumed, 4).0.vector[0], replica(1).entry(3));
        let mut restored = CausalLayer::new(VIEW, Start::from_journal(None, replica(9)), 16);
        assert_eq!(restored.replica(), replica(9));
        let next = make(&mut restored, 3);
        let dots: BTreeSet<(Replica, u64)> = [&c1, &c2, &next]
            .iter()
            .map(|m| (Replica::of(&m.0.vector[0]), m.0.vector[0].seq))
            .collect();
        assert_eq!(dots.len(), 3, "no dot names two contents");
        assert_eq!(next.0.vector[0], replica(9).entry(1));
        // Until view changes land, the laptop, which knows the phone's
        // first incarnation and the tablet, does not deliver the second.
        let r = layer(VIEW, 3, 16);
        exchange(&r, &mut q);
        assert_eq!(
            q.receive(&next.0, next.clone()),
            Err(Rejected::NotInView {
                replica: replica(9)
            })
        );
    }

    /// The view. Red if the epoch check is removed: a change of another
    /// epoch is then delivered.
    #[test]
    fn a_change_of_another_epoch_is_rejected_and_not_delivered() {
        let mut p = layer(VIEW, 1, 16);
        let mut q = layer(VIEW, 2, 16);
        let (mut tag, name) = make(&mut p, 1);
        tag.epoch = 1;
        assert_eq!(
            q.receive(&tag, (tag.clone(), name)),
            Err(Rejected::OtherEpoch { epoch: 1 })
        );
        assert_eq!(
            q.receive_summary(&Tag {
                epoch: 1,
                vector: alloc::vec![replica(1).entry(0)],
            }),
            Err(Rejected::OtherEpoch { epoch: 1 })
        );
        assert_eq!(column(&q, replica(1)), 0);
    }

    /// A small deterministic generator for the world below.
    struct Rng(u64);

    impl Rng {
        fn step(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.step() % n as u64) as usize
        }

        fn chance(&mut self, percent: u64) -> bool {
            self.step() % 100 < percent
        }
    }

    enum Packet {
        Change(Msg),
        Vector { from: usize, tag: Tag },
    }

    /// The world. Three copies on links that lose, duplicate and reorder
    /// deliver every change exactly once, in an order that is a linear
    /// extension of the contexts, with no gap skipped, and their logs drop
    /// to the cut once the links are quiet. Red if the hold branch delivers
    /// (the order check fails), or if repair is removed (a lost change then
    /// never arrives).
    #[test]
    fn three_copies_on_lossy_reordering_links_deliver_every_change_once_in_causal_order() {
        const CHANGES_EACH: u32 = 20;
        for seed in [0x5EED_0001_u64, 0x5EED_0002, 0x5EED_0003] {
            let mut rng = Rng(seed);
            let mut copies: Vec<CausalLayer<Msg>> =
                (0..3u8).map(|n| layer(VIEW, n + 1, 1000)).collect();
            let mut delivered: Vec<Vec<Msg>> = alloc::vec![Vec::new(); 3];
            let mut made = [0u32; 3];
            let mut wire: Vec<(usize, Packet)> = Vec::new();

            let send = |wire: &mut Vec<(usize, Packet)>,
                        rng: &mut Rng,
                        to: usize,
                        packet: Packet,
                        lossy: bool| {
                if lossy && rng.chance(30) {
                    return;
                }
                if let Packet::Change(message) = &packet {
                    if lossy && rng.chance(10) {
                        wire.push((to, Packet::Change(message.clone())));
                    }
                }
                wire.push((to, packet));
            };

            let deliver_one = |copies: &mut Vec<CausalLayer<Msg>>,
                               delivered: &mut Vec<Vec<Msg>>,
                               wire: &mut Vec<(usize, Packet)>,
                               rng: &mut Rng,
                               lossy: bool| {
                let (to, packet) = wire.remove(rng.below(wire.len()));
                match packet {
                    Packet::Change(message) => {
                        let out = copies[to]
                            .receive(&message.0, message.clone())
                            .expect("a change of the view");
                        delivered[to].extend(out);
                    }
                    Packet::Vector { from, tag } => {
                        let answer = copies[to].receive_summary(&tag).expect("a vector");
                        for message in answer {
                            send(wire, rng, from, Packet::Change(message), lossy);
                        }
                    }
                }
            };

            // The busy part: changes made, delivered out of order, lost and
            // repeated, with vectors exchanged now and then.
            while made.iter().any(|&m| m < CHANGES_EACH) || !wire.is_empty() {
                match rng.below(10) {
                    0..=2 => {
                        let at = rng.below(3);
                        if made[at] < CHANGES_EACH {
                            made[at] += 1;
                            let name = (at as u32) * 1000 + made[at];
                            let message = make(&mut copies[at], name);
                            delivered[at].push(message.clone());
                            for to in (0..3).filter(|&to| to != at) {
                                send(
                                    &mut wire,
                                    &mut rng,
                                    to,
                                    Packet::Change(message.clone()),
                                    true,
                                );
                            }
                        }
                    }
                    3 => {
                        let from = rng.below(3);
                        let tag = copies[from].summary();
                        for to in (0..3).filter(|&to| to != from) {
                            let packet = Packet::Vector {
                                from,
                                tag: tag.clone(),
                            };
                            send(&mut wire, &mut rng, to, packet, true);
                        }
                    }
                    _ => {
                        if !wire.is_empty() {
                            deliver_one(&mut copies, &mut delivered, &mut wire, &mut rng, true);
                        }
                    }
                }
            }

            // The quiet part: links that lose nothing, a vector from each
            // copy per round, until every copy holds every change.
            for _ in 0..20 {
                let tags: Vec<Tag> = copies.iter().map(CausalLayer::summary).collect();
                for (from, tag) in tags.into_iter().enumerate() {
                    for to in (0..3).filter(|&to| to != from) {
                        let packet = Packet::Vector {
                            from,
                            tag: tag.clone(),
                        };
                        send(&mut wire, &mut rng, to, packet, false);
                    }
                }
                while !wire.is_empty() {
                    deliver_one(&mut copies, &mut delivered, &mut wire, &mut rng, false);
                }
            }

            for (at, order) in delivered.iter().enumerate() {
                assert_eq!(
                    order.len(),
                    3 * CHANGES_EACH as usize,
                    "seed {seed:#x}: copy {at} delivers every change"
                );
                let mut columns: BTreeMap<Replica, u64> = BTreeMap::new();
                for (tag, name) in order {
                    let dot = tag.vector[0];
                    let author = Replica::of(&dot);
                    let before = columns.get(&author).copied().unwrap_or(0);
                    assert_eq!(
                        dot.seq,
                        before + 1,
                        "seed {seed:#x}: copy {at} delivers {name} once and skips no gap"
                    );
                    for needed in &tag.vector[1..] {
                        let have = columns.get(&Replica::of(needed)).copied().unwrap_or(0);
                        assert!(
                            needed.seq <= have,
                            "seed {seed:#x}: copy {at} delivers {name} before its context"
                        );
                    }
                    columns.insert(author, dot.seq);
                }
                assert_eq!(copies[at].held_len(), 0, "seed {seed:#x}: nothing held");
                assert_eq!(
                    copies[at].log_len(),
                    0,
                    "seed {seed:#x}: the log is at the cut"
                );
            }
        }
    }
}
