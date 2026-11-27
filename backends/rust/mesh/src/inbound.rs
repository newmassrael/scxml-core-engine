// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The router core's receive half: an envelope off the wire, through
//! duplicate suppression (SCE_MESH.md §mesh-10.5) and receiver ordering
//! (§mesh-10.6), to the envelopes the engine may now see, in order.
//!
//! Every decision is a call into the generated standard rules
//! (`sce:std/mesh`): whether an id was seen, where an ahead-of-sequence
//! envelope is held, how far a release drains, when a gap has waited long
//! enough. What this module adds is the state those rules are applied to,
//! kept per sender, and the envelopes themselves while they are held — the
//! rules see only `{seq, arrived_at}`, because an envelope is not a value a
//! list of records can carry.
//!
//! A sender is the link an envelope arrived on, which its host authenticated
//! (SCE_MESH.md section 10.9, invariant 10), never the `source` the envelope
//! writes: the windows are as many as the links the caller admits from, and
//! no envelope can make one for a name of its own choosing (section 10.5).
//!
//! The semantics are those of the C++ `OrderingBuffer` and `DedupRouter`
//! (sce/include/mesh/), so a Rust receiver and a C++ one deliver the same
//! stream the same way.
//!
//! A held envelope is kept as the bytes it arrived as, not as a decoded
//! value: the generated [`Envelope`] borrows from its input, and re-reading
//! the bytes on release costs one decode where an owned copy would be a
//! second shape of the envelope to keep in step with the document.
//!
//! A request's window entry also keeps what the responder answered it with
//! (§mesh-10.5, the reply cache): a duplicate of a request whose reply left is
//! answered with that reply, and a duplicate of one still being handled is
//! dropped. The cache lives and dies with the window: an id the window lets go
//! takes its reply with it.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_forge_runtime::codec::{CodecError, SceCursor};
use sce_portable_bytes::SceOwnedList;

use crate::checkpoint::{
    CheckpointError, Clocks, HeldState, RecordState, RecordsState, ReplyState, SenderState,
};
use crate::generated::dedup_admit::dedup_admit;
use crate::generated::dedup_holds::dedup_holds;
use crate::generated::envelope::Envelope;
use crate::generated::envelope_id::EnvelopeIdPayload;
use crate::generated::ordering_drain::ordering_drain;
use crate::generated::ordering_gap_end::ordering_gap_end;
use crate::generated::ordering_hold::ordering_hold;
use crate::generated::ordering_prune::ordering_prune;
use crate::generated::ordering_slot::OrderingSlotPayload;
use crate::generated::pattern_kind::PatternKind;
use crate::signal::Signal;

/// What one binding asks of the envelopes it receives. Deployment decides
/// both, per binding: dedup only where the transport does not already
/// suppress duplicates (§mesh-10.5), ordering only where the binding
/// declares it and the transport does not supply it (§mesh-10.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivery {
    /// Drop an envelope whose id this sender's window already holds.
    pub dedup: bool,
    /// Release envelopes in their sender's `sequence_no` order.
    pub ordered: bool,
}

/// Whether an envelope of `pattern` is in its sender's sequence on a binding
/// that orders (§mesh-10.6.3, §mesh-10.6.4): stamped by the sender from the
/// binding's counter, and released by the receiver in that order. The one
/// answer the send half and the receive half both ask, so a number is taken
/// exactly by the envelopes a receiver waits for.
///
/// A reply (`RpcReply`) and a field notification (`FieldNotify`) find what
/// they answer by its id, not by their place in the stream: on a binding
/// without durable receipt they take no number, and the receiver passes them
/// past its ordering buffer (still through the duplicate window), whether or
/// not their sender numbered them. On a binding with durable receipt
/// (`durable`, SCE_MESH.md section 10.11) the number is also what the
/// receiver acknowledges, so every envelope takes one and every envelope is
/// ordered.
pub(crate) fn sequenced(pattern: PatternKind, durable: bool) -> bool {
    durable || !matches!(pattern, PatternKind::RpcReply | PatternKind::FieldNotify)
}

/// An envelope the engine may now see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    bytes: Vec<u8>,
}

impl Received {
    /// The envelope, read from the bytes it arrived as.
    pub fn envelope(&self) -> Envelope<'_> {
        // These bytes decoded once already, when they were admitted, and
        // decoding is a function of the bytes alone.
        Envelope::decode(&mut SceCursor::new(&self.bytes))
            .expect("admitted envelope bytes decode as they did on admission")
    }

    /// The bytes as they arrived.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// What one call released, in the order the engine must see it, and what
/// it signalled.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub released: Vec<Received>,
    pub signals: Vec<Signal>,
    /// Replies to send again, each the bytes of the reply a request's first
    /// copy was answered with, because a duplicate of that request arrived
    /// (§mesh-10.5). They go back on the binding the duplicate came in on.
    pub replays: Vec<Vec<u8>>,
}

/// What a responder keeps of a request it admitted, in the request's window
/// entry (§mesh-10.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kept {
    /// No reply has left for it: a duplicate is dropped, and the reply goes
    /// once, when it is made.
    Handling,
    /// The reply was made and is held until the checkpoint that holds it is
    /// durable (SCE_MESH.md section 10.11): `sealed` is that checkpoint, `None`
    /// until one sealed it. A duplicate is dropped; the reply leaves once.
    Held { reply: Vec<u8>, sealed: Option<u64> },
    /// The reply left, as these bytes: a duplicate is answered with them.
    Sent(Vec<u8>),
}

/// A request in its sender's window, with what the responder keeps of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cached {
    /// The request's wire `invoke_id`, which its reply carries back
    /// (SCE_MESH.md section 10.7) and by which a reply finds its entry.
    pub invoke: [u8; 16],
    pub kept: Kept,
}

/// What the store holds of a member of an open machine, as the host was last told it
/// (docs/adr/0113, D4 and D5): the incarnation it stands under, and its low-water mark, the lowest
/// number of that incarnation whose answer the instance has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standing {
    pub incarnation: u64,
    pub mark: u64,
}

/// What a responder keeps of one instance of an open machine whose binding receives durably
/// (docs/adr/0113, D5): a completion record of each request of its incarnation that was taken in,
/// by the number the instance gave it, with the answer it was given. A repeat is answered from
/// its record, however long ago it was taken in, so no window of ids is what makes it at most
/// once; a record is dropped when the store's mark passes it, since the instance has its answer,
/// and with every other one when the store holds the member under a later incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Records {
    /// The incarnation the records are of: the latest the host told.
    incarnation: u64,
    /// The member's low-water mark as the store holds it: a mark a process saw and a crash may
    /// take back drops nothing, or a repeat taken in under the earlier one would run again.
    mark: u64,
    /// The records, by number.
    kept: BTreeMap<u64, Record>,
    /// Whether an envelope was taken in since the checkpoint last sealed: the write that keeps the
    /// checkpoint is made under the incarnation.
    unsealed: bool,
}

/// A completion record: the request's wire `invoke_id`, when it is a request a reply answers, and
/// what the responder keeps of it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    invoke: Option<[u8; 16]>,
    kept: Kept,
}

/// Where a request a reply may answer is kept: in its sender's window, by its envelope id
/// (SCE_MESH.md section 10.5), or among its sender's completion records, by its number
/// (docs/adr/0113, D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Window([u8; 16]),
    Record(u64),
}

impl Records {
    fn new(standing: Standing) -> Self {
        Self {
            incarnation: standing.incarnation,
            mark: standing.mark,
            kept: BTreeMap::new(),
            unsealed: false,
        }
    }

    /// Take `standing`, the store's answer for the member. A later incarnation drops every record
    /// of the one before, whose requests are fenced; a higher mark drops the records below it. An
    /// earlier incarnation is news of the past and changes nothing, and a mark never falls.
    fn stand(&mut self, standing: Standing, answering: &mut BTreeMap<[u8; 16], (String, Entry)>) {
        if standing.incarnation < self.incarnation {
            return;
        }
        if standing.incarnation > self.incarnation {
            self.incarnation = standing.incarnation;
            self.mark = standing.mark;
            self.drop_where(|_| true, answering);
            return;
        }
        self.mark = self.mark.max(standing.mark);
        let mark = self.mark;
        self.drop_where(|number| number < mark, answering);
    }

    /// Drop the records whose number `dropped` holds of, and where a reply would find them.
    fn drop_where(
        &mut self,
        dropped: impl Fn(u64) -> bool,
        answering: &mut BTreeMap<[u8; 16], (String, Entry)>,
    ) {
        self.kept.retain(|number, record| {
            let drop = dropped(*number);
            if drop {
                if let Some(invoke) = record.invoke {
                    answering.remove(&invoke);
                }
            }
            !drop
        });
    }
}

/// What became of an envelope of an instance judged by its completion records
/// ([`Inbound::admit_member`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Judged {
    /// Taken in, or answered or dropped as a repeat: what it released.
    Admitted(Outcome),
    /// It was made under an incarnation that is not the one the member stands under, `current`:
    /// nothing of it is taken in.
    Fenced { current: u64 },
}

/// Why an envelope was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmitError {
    /// The bytes are not an envelope the standard document accepts.
    Malformed(CodecError),
    /// The binding is ordered, the envelope is one it orders (`sequenced`),
    /// and it carries no `sequence_no` (§mesh-10.6.3). Refused before the
    /// dedup window sees its id, so a refused envelope does not take a window
    /// slot.
    Unstamped,
    /// A rule refused its input: a sequence at the `u64` maximum, or a clock
    /// reading that cannot be subtracted.
    Rule(AlgorithmError),
    /// A sender has more envelopes held ahead of sequence than the rule's
    /// list can carry.
    HoldFull,
}

/// Why a receive core could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// deploy.yaml's `window_size` is 0; §mesh-10.5 needs at least one id.
    EmptyDedupWindow,
    /// deploy.yaml's `gap_timeout_ms` is negative.
    NegativeGapTimeout,
    /// The `receipt:` the binding to `peer` declares cannot be kept: a `window` or `retry_ms` of
    /// 0, which the build refuses (docs/adr/0109, D10), in a table written by hand.
    BadReceipt { peer: &'static str },
}

/// One sender's receive state.
struct Sender {
    window: SceOwnedList<EnvelopeIdPayload, 256>,
    /// Where a rule writes the window it gives back ([`rewrite`]).
    window_next: SceOwnedList<EnvelopeIdPayload, 256>,
    /// The sequence this sender is expected to send next; `None` until its
    /// first ordered envelope, which anchors it (a receiver that joins
    /// mid-stream did not miss what was sent before it listened).
    next: Option<u64>,
    pending: SceOwnedList<OrderingSlotPayload, 256>,
    /// Where a rule writes the held slots it gives back ([`rewrite`]).
    pending_next: SceOwnedList<OrderingSlotPayload, 256>,
    held: BTreeMap<u64, Received>,
    /// The reply cache: each request of this sender's window that carries a
    /// wire `invoke_id`, by the request's envelope id (§mesh-10.5).
    replies: BTreeMap<[u8; 16], Cached>,
    /// Whether this sender's stream has durable receipt (SCE_MESH.md section
    /// 10.11): a gap in it is an envelope its sender still holds and will send
    /// again, so it never times out.
    durable: bool,
    /// The completion records of an instance of an open machine whose binding
    /// receives durably (docs/adr/0113, D5), in place of a window.
    records: Option<Records>,
}

/// Replace `list` with what `rule` writes from it, or leave it as it was when
/// the rule fails. A generated rule writes the list it returns into a
/// destination its caller holds (docs/adr/0082) while it reads `list`, so it
/// writes into `next`, and only a list it wrote takes `list`'s place. This crate
/// holds heap lists (it requires `alloc`), so the swap moves three words.
fn rewrite<T, const N: usize, E>(
    list: &mut SceOwnedList<T, N>,
    next: &mut SceOwnedList<T, N>,
    rule: impl FnOnce(&[T], &mut SceOwnedList<T, N>) -> Result<(), E>,
) -> Result<(), E>
where
    T: Clone,
{
    rule(list.as_slice(), next)?;
    core::mem::swap(list, next);
    Ok(())
}

impl Sender {
    fn new() -> Self {
        Self {
            window: SceOwnedList::new(),
            window_next: SceOwnedList::new(),
            next: None,
            pending: SceOwnedList::new(),
            pending_next: SceOwnedList::new(),
            held: BTreeMap::new(),
            replies: BTreeMap::new(),
            durable: false,
            records: None,
        }
    }

    /// Release everything held that follows on from `start` without a gap,
    /// and expect what comes after it.
    fn release_from(&mut self, start: u64, out: &mut Outcome) -> Result<(), AdmitError> {
        let drained = ordering_drain(self.pending.as_slice(), start).map_err(AdmitError::Rule)?;
        let still_held = self.held.split_off(&drained);
        out.released
            .extend(core::mem::replace(&mut self.held, still_held).into_values());
        rewrite(&mut self.pending, &mut self.pending_next, |pending, out| {
            ordering_prune(pending, drained, out)
        })
        .map_err(|_| AdmitError::HoldFull)?;
        self.next = Some(drained);
        Ok(())
    }
}

/// The receive half of a router: every sender's window and held envelopes,
/// one sender per link.
pub struct Inbound {
    dedup_window: u32,
    gap_timeout_ms: i64,
    senders: BTreeMap<String, Sender>,
    /// Where each cached request is, by its wire `invoke_id`: its sender and
    /// its entry there. What a reply finds its request's entry by.
    answering: BTreeMap<[u8; 16], (String, Entry)>,
}

impl Inbound {
    /// A core with deploy.yaml's `window_size` (§mesh-10.5) and
    /// `gap_timeout_ms` (§mesh-10.6.1).
    pub fn new(dedup_window: u32, gap_timeout_ms: i64) -> Result<Self, ConfigError> {
        if dedup_window == 0 {
            return Err(ConfigError::EmptyDedupWindow);
        }
        if gap_timeout_ms < 0 {
            return Err(ConfigError::NegativeGapTimeout);
        }
        Ok(Self {
            dedup_window,
            gap_timeout_ms,
            senders: BTreeMap::new(),
            answering: BTreeMap::new(),
        })
    }

    /// Admit the envelope `bytes` that arrived on the link of `link`, from a
    /// binding asking `delivery`, at `now_ms` on the host's monotonic clock.
    ///
    /// The window and the ordering state the envelope is judged by are
    /// `link`'s, whatever `source` it writes (SCE_MESH.md section 10.5): a
    /// caller passes only a link it admitted, so the senders this half keeps
    /// are as many as those links.
    ///
    /// SCE_MESH.md §mesh-10.6.4: an ordered envelope below its sender's
    /// expected sequence is dropped, one above it is held, and the one it
    /// expects is released together with every held envelope that now
    /// follows on.
    pub fn admit(
        &mut self,
        link: &str,
        bytes: Vec<u8>,
        delivery: Delivery,
        now_ms: i64,
    ) -> Result<Outcome, AdmitError> {
        self.admit_from(link, bytes, delivery, None, now_ms)
    }

    /// How many senders this half keeps state for: one per link it admitted
    /// an envelope from and has not forgotten.
    pub fn senders(&self) -> usize {
        self.senders.len()
    }

    /// [`Self::admit`], for a stream that is durable when `durable_from` is
    /// the sequence it is expected to send first (SCE_MESH.md section 10.11):
    /// an ordered durable stream is anchored there rather than at the first
    /// envelope that arrives, since what came before is still held by its
    /// sender and will be sent again, and none of its gaps times out.
    pub(crate) fn admit_from(
        &mut self,
        link: &str,
        bytes: Vec<u8>,
        delivery: Delivery,
        durable_from: Option<u64>,
        now_ms: i64,
    ) -> Result<Outcome, AdmitError> {
        let (raw_id, sequence, request, pattern) = {
            let envelope =
                Envelope::decode(&mut SceCursor::new(&bytes)).map_err(AdmitError::Malformed)?;
            // A request a reply can answer: one that carries the wire id the
            // reply carries back (SCE_MESH.md section 10.7).
            let request = (envelope.pattern == PatternKind::RpcRequest)
                .then_some(envelope.invoke_id)
                .flatten()
                .map(<[u8; 16]>::from);
            (
                <[u8; 16]>::from(envelope.id),
                envelope.sequence_no,
                request,
                envelope.pattern,
            )
        };
        let id = envelope_id(&raw_id);
        // §mesh-10.6.4: a reply or a field notification on a binding without
        // durable receipt passes the ordering buffer, numbered or not; every
        // other envelope of an ordered binding is placed by its number.
        let ordered = delivery.ordered && sequenced(pattern, durable_from.is_some());
        if ordered && sequence.is_none() {
            return Err(AdmitError::Unstamped);
        }

        let source = link.to_string();
        let mut out = Outcome::default();
        let sender = self
            .senders
            .entry(source.clone())
            .or_insert_with(Sender::new);
        if durable_from.is_some() {
            sender.durable = true;
        }

        if delivery.dedup {
            if dedup_holds(sender.window.as_slice(), id) {
                // §mesh-10.5: a duplicate of a request whose reply left is
                // answered with that reply; any other duplicate is dropped.
                if let Some(Cached {
                    kept: Kept::Sent(reply),
                    ..
                }) = sender.replies.get(&raw_id)
                {
                    out.replays.push(reply.clone());
                }
                return Ok(out);
            }
            let full = sender.window.as_slice().len() >= self.dedup_window as usize;
            let capacity = self.dedup_window;
            rewrite(
                &mut sender.window,
                &mut sender.window_next,
                |window, out| dedup_admit(window, id, capacity, out),
            )
            .map_err(AdmitError::Rule)?;
            if full {
                out.signals.push(Signal::DedupWindowOverflow {
                    source: source.clone(),
                    window_size: self.dedup_window,
                });
            }
            if let Some(invoke) = request {
                sender.replies.insert(
                    raw_id,
                    Cached {
                        invoke,
                        kept: Kept::Handling,
                    },
                );
                self.answering
                    .insert(invoke, (source.clone(), Entry::Window(raw_id)));
            }
            // The cache is bounded by the window: a request whose id the
            // window let go takes its reply with it.
            let window = sender.window.as_slice();
            let answering = &mut self.answering;
            sender.replies.retain(|cached_id, cached| {
                let kept = dedup_holds(window, envelope_id(cached_id));
                if !kept {
                    answering.remove(&cached.invoke);
                }
                kept
            });
        }

        let Some(seq) = sequence.filter(|_| ordered) else {
            out.released.push(Received { bytes });
            return Ok(out);
        };
        let next = *sender.next.get_or_insert(durable_from.unwrap_or(seq));
        if seq < next {
            return Ok(out);
        }
        if seq > next {
            rewrite(
                &mut sender.pending,
                &mut sender.pending_next,
                |pending, out| ordering_hold(pending, seq, now_ms, out),
            )
            .map_err(|_| AdmitError::HoldFull)?;
            // A re-delivery of a held sequence keeps its first copy, as
            // ordering_hold keeps its first arrival time.
            sender.held.entry(seq).or_insert(Received { bytes });
            return Ok(out);
        }
        out.released.push(Received { bytes });
        let start = seq
            .checked_add(1)
            .ok_or(AdmitError::Rule(AlgorithmError::Overflow))?;
        sender.release_from(start, &mut out)?;
        Ok(out)
    }

    /// Forget `source`: its dedup window and every envelope held behind a gap are dropped. What it
    /// sends after this is taken as from a sender not heard from before, so an envelope it sent
    /// earlier is delivered again if it is sent again. For a sender that has left for good.
    pub fn forget(&mut self, source: &str) {
        if let Some(sender) = self.senders.remove(source) {
            for cached in sender.replies.values() {
                self.answering.remove(&cached.invoke);
            }
            for record in sender
                .records
                .iter()
                .flat_map(|records| records.kept.values())
            {
                if let Some(invoke) = record.invoke {
                    self.answering.remove(&invoke);
                }
            }
        }
    }

    /// Admit the envelope `bytes` of the instance whose link is `link`, an instance of an open
    /// machine whose binding receives durably (docs/adr/0113, D5), which its edge took in under
    /// the incarnation `made_under`, while the store holds its member as `standing`: the latest
    /// answer the host has, at least as late as the one the edge took it in under.
    ///
    /// The incarnation and the mark are judged again here, as the envelope runs, since either may
    /// have moved while it waited: one made under an incarnation the member no longer stands under
    /// is fenced, and one numbered below the mark is one whose answer the instance has, and is
    /// dropped. A repeat of one taken in is answered from its completion record when its reply
    /// left, and dropped while it is handled or when it has none; any other is taken in, and its
    /// record made, in the state the next checkpoint holds. Every envelope of the stream carries
    /// its number, and is released as it arrives: what orders a stream spread over sessions is no
    /// one receiver's.
    pub(crate) fn admit_member(
        &mut self,
        link: &str,
        bytes: Vec<u8>,
        made_under: u64,
        standing: Standing,
    ) -> Result<Judged, AdmitError> {
        let (number, request) = {
            let envelope =
                Envelope::decode(&mut SceCursor::new(&bytes)).map_err(AdmitError::Malformed)?;
            let request = (envelope.pattern == PatternKind::RpcRequest)
                .then_some(envelope.invoke_id)
                .flatten()
                .map(<[u8; 16]>::from);
            (envelope.sequence_no, request)
        };
        let number = number.ok_or(AdmitError::Unstamped)?;
        let Self {
            senders, answering, ..
        } = self;
        let sender = senders.entry(link.to_string()).or_insert_with(Sender::new);
        sender.durable = true;
        let records = sender.records.get_or_insert_with(|| Records::new(standing));
        records.stand(standing, answering);
        if made_under != records.incarnation {
            return Ok(Judged::Fenced {
                current: records.incarnation,
            });
        }
        let mut out = Outcome::default();
        if number < records.mark {
            return Ok(Judged::Admitted(out));
        }
        if let Some(record) = records.kept.get(&number) {
            if let Kept::Sent(reply) = &record.kept {
                out.replays.push(reply.clone());
            }
            return Ok(Judged::Admitted(out));
        }
        records.kept.insert(
            number,
            Record {
                invoke: request,
                kept: Kept::Handling,
            },
        );
        if let Some(invoke) = request {
            answering.insert(invoke, (link.to_string(), Entry::Record(number)));
        }
        records.unsealed = true;
        out.released.push(Received { bytes });
        Ok(Judged::Admitted(out))
    }

    /// Take `standing`, the store's latest answer for the member whose instance's link is `link`
    /// (docs/adr/0113, D5): its records below the mark are dropped, and all of them when it
    /// stands under a later incarnation. A link that keeps no records keeps none from this.
    pub(crate) fn stand(&mut self, link: &str, standing: Standing) {
        let Self {
            senders, answering, ..
        } = self;
        if let Some(records) = senders
            .get_mut(link)
            .and_then(|sender| sender.records.as_mut())
        {
            records.stand(standing, answering);
        }
    }

    /// What every request entry, of a window or a record, keeps of the reply to it.
    fn kept_mut(&mut self) -> impl Iterator<Item = &mut Kept> + '_ {
        self.senders.values_mut().flat_map(|sender| {
            let windows = sender.replies.values_mut().map(|cached| &mut cached.kept);
            let records = sender
                .records
                .iter_mut()
                .flat_map(|records| records.kept.values_mut().map(|record| &mut record.kept));
            windows.chain(records)
        })
    }

    /// The machine answered the request whose wire `invoke_id` is `invoke`
    /// with `reply`, the bytes of the reply envelope as they leave: keep it in
    /// that request's window entry, so a duplicate of the request is answered
    /// with it (§mesh-10.5). Only the first reply is kept; a request no entry
    /// holds (its sender's binding suppresses no duplicates, or the window let
    /// it go) keeps nothing. `held` when the router holds what it produces
    /// until a checkpoint that holds it is durable (SCE_MESH.md section
    /// 10.11): the reply has not left, so a duplicate is still dropped.
    pub(crate) fn answered(&mut self, invoke: &[u8; 16], reply: &[u8], held: bool) {
        let Some((source, entry)) = self.answering.get(invoke) else {
            return;
        };
        let Some(sender) = self.senders.get_mut(source) else {
            return;
        };
        let kept = match entry {
            Entry::Window(request) => sender
                .replies
                .get_mut(request)
                .map(|cached| &mut cached.kept),
            Entry::Record(number) => sender
                .records
                .as_mut()
                .and_then(|records| records.kept.get_mut(number))
                .map(|record| &mut record.kept),
        };
        let Some(kept) = kept else {
            return;
        };
        if *kept == Kept::Handling {
            *kept = if held {
                Kept::Held {
                    reply: reply.to_vec(),
                    sealed: None,
                }
            } else {
                Kept::Sent(reply.to_vec())
            };
        }
    }

    /// The checkpoint `sequence` was sealed: it holds every reply made since
    /// the one before. Answers each incarnation of an instance whose envelope
    /// was taken in since the seal before, with the instance's link: what the
    /// run the checkpoint keeps was made under (docs/adr/0113, D5).
    pub(crate) fn seal(&mut self, sequence: u64) -> Vec<(String, u64)> {
        for kept in self.kept_mut() {
            if let Kept::Held { sealed, .. } = kept {
                if sealed.is_none() {
                    *sealed = Some(sequence);
                }
            }
        }
        self.senders
            .iter_mut()
            .filter_map(|(source, sender)| {
                let records = sender.records.as_mut()?;
                core::mem::take(&mut records.unsealed)
                    .then(|| (source.clone(), records.incarnation))
            })
            .collect()
    }

    /// The checkpoint `sequence` is durable: every reply it and the ones
    /// before it hold has left, and answers a duplicate from now on.
    pub(crate) fn released_through(&mut self, sequence: u64) {
        for kept in self.kept_mut() {
            let left = match &mut *kept {
                Kept::Held {
                    reply,
                    sealed: Some(at),
                } if *at <= sequence => Some(core::mem::take(reply)),
                _ => None,
            };
            if let Some(reply) = left {
                *kept = Kept::Sent(reply);
            }
        }
    }

    /// Every sender's receive state, as a checkpoint holds it; `clocks` say
    /// what the monotonic moments an envelope arrived at are on the wall.
    pub(crate) fn states(&self, clocks: Clocks) -> Vec<SenderState> {
        self.senders
            .iter()
            .map(|(source, sender)| SenderState {
                source: source.clone(),
                window: sender.window.as_slice().iter().map(id_bytes).collect(),
                next: sender.next,
                held: sender
                    .pending
                    .as_slice()
                    .iter()
                    .filter_map(|slot| {
                        sender.held.get(&slot.seq).map(|received| HeldState {
                            sequence: slot.seq,
                            arrived: clocks.to_wall(slot.arrived_at_ms),
                            envelope: received.bytes.clone(),
                        })
                    })
                    .collect(),
                replies: sender
                    .replies
                    .iter()
                    .map(|(request, cached)| ReplyState {
                        request: *request,
                        invoke: cached.invoke,
                        reply: match &cached.kept {
                            Kept::Handling => None,
                            Kept::Held { reply, .. } | Kept::Sent(reply) => Some(reply.clone()),
                        },
                    })
                    .collect(),
                durable: sender.durable,
                records: sender.records.as_ref().map(|records| RecordsState {
                    incarnation: records.incarnation,
                    mark: records.mark,
                    kept: records
                        .kept
                        .iter()
                        .map(|(number, record)| RecordState {
                            number: *number,
                            invoke: record.invoke,
                            reply: match &record.kept {
                                Kept::Handling => None,
                                Kept::Held { reply, .. } | Kept::Sent(reply) => Some(reply.clone()),
                            },
                        })
                        .collect(),
                }),
            })
            .collect()
    }

    /// Take back the receive state a checkpoint holds, in place of what this
    /// half holds. A window wider than this receiver's keeps its newest ids,
    /// and a cached reply whose request it no longer holds is let go with it.
    pub(crate) fn restore(
        &mut self,
        states: &[SenderState],
        clocks: Clocks,
    ) -> Result<(), CheckpointError> {
        let full = |what: &str| {
            CheckpointError::Malformed(format!("a sender's {what} holds more than a list carries"))
        };
        self.senders.clear();
        self.answering.clear();
        for state in states {
            let mut sender = Sender::new();
            sender.next = state.next;
            sender.durable = state.durable;
            let skip = state
                .window
                .len()
                .saturating_sub(self.dedup_window as usize);
            for id in &state.window[skip..] {
                sender
                    .window
                    .push(envelope_id(id))
                    .map_err(|_| full("window"))?;
            }
            for held in &state.held {
                sender
                    .pending
                    .push(OrderingSlotPayload {
                        seq: held.sequence,
                        arrived_at_ms: clocks.to_monotonic(held.arrived),
                    })
                    .map_err(|_| full("held envelopes"))?;
                sender.held.insert(
                    held.sequence,
                    Received {
                        bytes: held.envelope.clone(),
                    },
                );
            }
            for reply in &state.replies {
                if !dedup_holds(sender.window.as_slice(), envelope_id(&reply.request)) {
                    continue;
                }
                // A reply the checkpoint holds is released by its restore.
                let kept = reply.reply.clone().map_or(Kept::Handling, Kept::Sent);
                sender.replies.insert(
                    reply.request,
                    Cached {
                        invoke: reply.invoke,
                        kept,
                    },
                );
                self.answering.insert(
                    reply.invoke,
                    (state.source.clone(), Entry::Window(reply.request)),
                );
            }
            if let Some(held) = &state.records {
                let mut records = Records::new(Standing {
                    incarnation: held.incarnation,
                    mark: held.mark,
                });
                for record in &held.kept {
                    // A reply the checkpoint holds is released by its restore.
                    let kept = record.reply.clone().map_or(Kept::Handling, Kept::Sent);
                    records.kept.insert(
                        record.number,
                        Record {
                            invoke: record.invoke,
                            kept,
                        },
                    );
                    if let Some(invoke) = record.invoke {
                        self.answering
                            .insert(invoke, (state.source.clone(), Entry::Record(record.number)));
                    }
                }
                sender.records = Some(records);
            }
            self.senders.insert(state.source.clone(), sender);
        }
        Ok(())
    }

    /// End every gap that has waited `gap_timeout_ms` by `now_ms`: signal the
    /// sequences lost and release what was held behind them. The host calls
    /// this on its own schedule, so a gap ends even when nothing else
    /// arrives from that sender.
    pub fn tick(&mut self, now_ms: i64) -> Result<Outcome, AdmitError> {
        let mut out = Outcome::default();
        for (source, sender) in &mut self.senders {
            // A durable stream's gap is an envelope its sender still holds.
            if sender.durable {
                continue;
            }
            let Some(next) = sender.next else {
                continue;
            };
            let end =
                ordering_gap_end(sender.pending.as_slice(), next, now_ms, self.gap_timeout_ms)
                    .map_err(AdmitError::Rule)?;
            if end == next {
                continue;
            }
            out.signals.push(Signal::OrderingGap {
                source: source.clone(),
                lost_lo: next,
                lost_hi: end - 1,
            });
            sender.release_from(end, &mut out)?;
        }
        Ok(out)
    }
}

/// The 16 bytes of an id as the rules hold it: [`envelope_id`] read back.
fn id_bytes(id: &EnvelopeIdPayload) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&id.hi.to_be_bytes());
    out[8..].copy_from_slice(&id.lo.to_be_bytes());
    out
}

/// An envelope id as the rules hold it: its first eight bytes and its last
/// eight, big-endian (`sce:std/mesh/envelope_id`). The envelope document
/// fixes `id` at exactly 16 bytes, which its type holds (docs/adr/0077 §3).
fn envelope_id(id: &[u8; 16]) -> EnvelopeIdPayload {
    let mut hi = [0u8; 8];
    let mut lo = [0u8; 8];
    hi.copy_from_slice(&id[..8]);
    lo.copy_from_slice(&id[8..]);
    EnvelopeIdPayload {
        hi: u64::from_be_bytes(hi),
        lo: u64::from_be_bytes(lo),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::pattern_kind::PatternKind;
    use crate::generated::payload_codec::PayloadCodec;
    use sce_forge_runtime::codec::Bounded;

    const PLAIN: Delivery = Delivery {
        dedup: false,
        ordered: false,
    };
    const DEDUP: Delivery = Delivery {
        dedup: true,
        ordered: false,
    };
    const ORDERED: Delivery = Delivery {
        dedup: false,
        ordered: true,
    };

    fn wire(source: &str, id: u8, sequence_no: Option<u64>) -> Vec<u8> {
        wire_as(PatternKind::FireForget, source, id, sequence_no)
    }

    /// [`wire`], of the pattern `pattern`.
    fn wire_as(pattern: PatternKind, source: &str, id: u8, sequence_no: Option<u64>) -> Vec<u8> {
        let id = [id; 16];
        Envelope {
            id: (&id).into(),
            source: Bounded::new(source).expect("a source within its bound"),
            event_type: Bounded::new("evt").expect("an event type within its bound"),
            pattern,
            datacontenttype: PayloadCodec::None,
            sequence_no,
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    fn sequences(out: &Outcome) -> Vec<u64> {
        out.released
            .iter()
            .map(|r| {
                r.envelope()
                    .sequence_no
                    .expect("ordered envelopes carry one")
            })
            .collect()
    }

    fn core() -> Inbound {
        Inbound::new(2, 50).expect("a valid configuration")
    }

    #[test]
    fn an_unordered_envelope_is_released_as_it_arrives() {
        let mut inbound = core();
        let bytes = wire("ecu", 1, None);
        let out = inbound.admit("ecu", bytes.clone(), PLAIN, 0).unwrap();
        assert_eq!(out.released.len(), 1);
        assert_eq!(out.released[0].bytes(), bytes.as_slice());
        assert_eq!(out.released[0].envelope().source, "ecu");
    }

    #[test]
    fn a_duplicate_id_from_the_same_sender_is_dropped_and_another_senders_is_not() {
        let mut inbound = core();
        assert_eq!(
            inbound
                .admit("a", wire("a", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
        assert!(inbound
            .admit("a", wire("a", 1, None), DEDUP, 0)
            .unwrap()
            .released
            .is_empty());
        assert_eq!(
            inbound
                .admit("b", wire("b", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
    }

    #[test]
    fn a_novel_id_admitted_to_a_full_window_signals_the_overflow() {
        let mut inbound = core();
        for id in [1, 2] {
            assert!(inbound
                .admit("a", wire("a", id, None), DEDUP, 0)
                .unwrap()
                .signals
                .is_empty());
        }
        let out = inbound.admit("a", wire("a", 3, None), DEDUP, 0).unwrap();
        assert_eq!(
            out.signals,
            vec![Signal::DedupWindowOverflow {
                source: "a".to_string(),
                window_size: 2,
            }]
        );
        // The oldest id left the window, so it is novel again.
        assert_eq!(
            inbound
                .admit("a", wire("a", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
    }

    #[test]
    fn the_first_ordered_envelope_anchors_its_sender() {
        let mut inbound = core();
        let out = inbound
            .admit("a", wire("a", 7, Some(7)), ORDERED, 0)
            .unwrap();
        assert_eq!(sequences(&out), vec![7]);
    }

    #[test]
    fn an_envelope_ahead_of_sequence_waits_for_the_gap_to_fill() {
        let mut inbound = core();
        assert_eq!(
            sequences(
                &inbound
                    .admit("a", wire("a", 1, Some(1)), ORDERED, 0)
                    .unwrap()
            ),
            vec![1]
        );
        assert!(inbound
            .admit("a", wire("a", 4, Some(4)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        assert!(inbound
            .admit("a", wire("a", 3, Some(3)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        let out = inbound
            .admit("a", wire("a", 2, Some(2)), ORDERED, 0)
            .unwrap();
        assert_eq!(sequences(&out), vec![2, 3, 4]);
    }

    #[test]
    fn an_envelope_below_the_expected_sequence_is_dropped() {
        let mut inbound = core();
        inbound
            .admit("a", wire("a", 5, Some(5)), ORDERED, 0)
            .unwrap();
        assert!(inbound
            .admit("a", wire("a", 4, Some(4)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        assert!(inbound
            .admit("a", wire("a", 5, Some(5)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
    }

    #[test]
    fn a_gap_that_waits_out_its_timeout_is_signalled_and_released_past() {
        let mut inbound = core();
        inbound
            .admit("a", wire("a", 1, Some(1)), ORDERED, 0)
            .unwrap();
        inbound
            .admit("a", wire("a", 4, Some(4)), ORDERED, 100)
            .unwrap();
        inbound
            .admit("a", wire("a", 5, Some(5)), ORDERED, 120)
            .unwrap();
        assert_eq!(inbound.tick(149).unwrap(), Outcome::default());
        let out = inbound.tick(150).unwrap();
        assert_eq!(
            out.signals,
            vec![Signal::OrderingGap {
                source: "a".to_string(),
                lost_lo: 2,
                lost_hi: 3
            }]
        );
        assert_eq!(sequences(&out), vec![4, 5]);
        // The sender now expects 6, and a late 3 is a straggler.
        assert!(inbound
            .admit("a", wire("a", 3, Some(3)), ORDERED, 200)
            .unwrap()
            .released
            .is_empty());
        assert_eq!(
            sequences(
                &inbound
                    .admit("a", wire("a", 6, Some(6)), ORDERED, 200)
                    .unwrap()
            ),
            vec![6]
        );
    }

    #[test]
    fn an_ordered_binding_refuses_an_unstamped_envelope_before_dedup_sees_it() {
        let mut inbound = core();
        let both = Delivery {
            dedup: true,
            ordered: true,
        };
        assert_eq!(
            inbound.admit("a", wire("a", 1, None), both, 0),
            Err(AdmitError::Unstamped)
        );
        // Its id took no window slot: the same id stamped is novel.
        assert_eq!(
            sequences(&inbound.admit("a", wire("a", 1, Some(1)), both, 0).unwrap()),
            vec![1]
        );
    }

    /// §mesh-10.6.4: a reply and a field notification are not in their
    /// sender's sequence. Unnumbered on an ordered binding, each is released
    /// at once, past a gap the stream is waiting on, and the duplicate window
    /// still drops a second copy. Red if the receiver asks them for a number:
    /// each is then refused as `Unstamped` (SCE_MESH.md section 16.7, row 11).
    #[test]
    fn an_unnumbered_reply_or_field_notification_passes_an_ordered_binding() {
        let mut inbound = Inbound::new(8, 50).expect("a valid configuration");
        let both = Delivery {
            dedup: true,
            ordered: true,
        };
        assert_eq!(
            sequences(&inbound.admit("a", wire("a", 1, Some(1)), both, 0).unwrap()),
            vec![1]
        );
        assert!(inbound
            .admit("a", wire("a", 3, Some(3)), both, 0)
            .unwrap()
            .released
            .is_empty());
        for (id, pattern) in [(10, PatternKind::RpcReply), (11, PatternKind::FieldNotify)] {
            let bytes = wire_as(pattern, "a", id, None);
            let out = inbound.admit("a", bytes.clone(), both, 0).unwrap();
            assert_eq!(out.released.len(), 1, "{pattern:?} is released at once");
            assert_eq!(out.released[0].bytes(), bytes.as_slice());
            assert!(
                inbound
                    .admit("a", bytes, both, 0)
                    .unwrap()
                    .released
                    .is_empty(),
                "a second copy of {pattern:?} is a duplicate"
            );
        }
        // The stream still waits for 2, and 3 follows it.
        assert_eq!(
            sequences(&inbound.admit("a", wire("a", 2, Some(2)), both, 0).unwrap()),
            vec![2, 3]
        );
    }

    /// §mesh-10.6.4: a reply its sender numbered anyway is passed as one that
    /// carries no number. It neither anchors the stream nor is held behind a
    /// gap, and its number is never waited for, so no gap times out. Red if a
    /// numbered reply is ordered: the first one would anchor the stream at 9
    /// and the envelope numbered 1 would be dropped as a straggler.
    #[test]
    fn a_numbered_reply_takes_no_place_in_the_stream() {
        let mut inbound = Inbound::new(8, 50).expect("a valid configuration");
        let reply = wire_as(PatternKind::RpcReply, "a", 9, Some(9));
        let out = inbound.admit("a", reply.clone(), ORDERED, 0).unwrap();
        assert_eq!(out.released.len(), 1);
        assert_eq!(out.released[0].bytes(), reply.as_slice());
        for seq in [1, 2] {
            assert_eq!(
                sequences(
                    &inbound
                        .admit("a", wire("a", seq as u8, Some(seq)), ORDERED, 10)
                        .unwrap()
                ),
                vec![seq]
            );
        }
        let between = wire_as(PatternKind::RpcReply, "a", 20, Some(3));
        assert_eq!(
            inbound
                .admit("a", between, ORDERED, 20)
                .unwrap()
                .released
                .len(),
            1
        );
        assert_eq!(
            sequences(
                &inbound
                    .admit("a", wire("a", 3, Some(3)), ORDERED, 30)
                    .unwrap()
            ),
            vec![3]
        );
        assert_eq!(inbound.tick(10_000).unwrap(), Outcome::default());
    }

    /// SCE_MESH.md section 10.11: on a durable stream the number is what the
    /// receiver acknowledges, so a reply keeps its place: one ahead of the
    /// stream waits for the envelope before it, and one with no number is
    /// refused. Red if the exemption of §mesh-10.6.4 reached a durable stream.
    #[test]
    fn on_a_durable_stream_a_reply_keeps_its_place() {
        let mut inbound = Inbound::new(8, 50).expect("a valid configuration");
        let reply = wire_as(PatternKind::RpcReply, "a", 2, Some(2));
        assert!(inbound
            .admit_from("a", reply, ORDERED, Some(1), 0)
            .unwrap()
            .released
            .is_empty());
        assert_eq!(
            sequences(
                &inbound
                    .admit_from("a", wire("a", 1, Some(1)), ORDERED, Some(1), 0)
                    .unwrap()
            ),
            vec![1, 2]
        );
        assert_eq!(
            inbound.admit_from(
                "a",
                wire_as(PatternKind::RpcReply, "a", 3, None),
                ORDERED,
                Some(3),
                0
            ),
            Err(AdmitError::Unstamped)
        );
    }

    #[test]
    fn bytes_that_are_not_an_envelope_are_refused() {
        let mut inbound = core();
        assert!(matches!(
            inbound.admit("a", vec![0xFF], PLAIN, 0),
            Err(AdmitError::Malformed(_))
        ));
    }

    /// SCE_MESH.md section 10.5: the windows are as many as the links. Ten thousand envelopes on
    /// one link, each claiming a source of its own, make one sender, whose window holds what they
    /// carried; a second link is a second sender. Red if the window is keyed by the envelope's
    /// `source`: every claimed name then makes a window of its own.
    #[test]
    fn envelopes_claiming_many_sources_on_one_link_make_one_window() {
        let mut inbound = Inbound::new(4, 50).expect("a valid configuration");
        for n in 0..10_000u32 {
            let mut id = [0x5A; 16];
            id[12..].copy_from_slice(&n.to_be_bytes());
            let claimed = alloc::format!("s{n}");
            let bytes = Envelope {
                id: (&id).into(),
                source: Bounded::new(claimed.as_str()).expect("a short source"),
                event_type: Bounded::new("evt").expect("an event type within its bound"),
                pattern: PatternKind::FireForget,
                datacontenttype: PayloadCodec::None,
                ..Envelope::new()
            }
            .encode_to_vec();
            inbound.admit("phone", bytes, DEDUP, 0).unwrap();
        }
        assert_eq!(inbound.senders(), 1);
        let held = inbound.states(Clocks {
            monotonic_ms: 0,
            wall_ms: 0,
        });
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].source, "phone");
        assert_eq!(
            held[0].window.len(),
            4,
            "the window keeps its configured size"
        );
        inbound
            .admit("tablet", wire("x", 1, None), DEDUP, 0)
            .unwrap();
        assert_eq!(inbound.senders(), 2);
        inbound.forget("phone");
        assert_eq!(inbound.senders(), 1);
    }

    #[test]
    fn a_configuration_the_rules_cannot_run_is_refused() {
        assert!(matches!(
            Inbound::new(0, 50),
            Err(ConfigError::EmptyDedupWindow)
        ));
        assert!(matches!(
            Inbound::new(1, -1),
            Err(ConfigError::NegativeGapTimeout)
        ));
    }
}
