// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The host's side of a relay: what carries a host's operations to a relay and a relay's to the
//! host's machine.
//!
//! A [`Courier`] holds a [`Trust`] (the host's: its log, its person's chain, its checkpoints) and a
//! [`Link`] to one relay, and a round of it does three things. It brings the two logs level (the
//! relay's entries come in and are checked; the entries the relay lacks go out). It gives the relay
//! what it lacks of what the host holds, by comparing heads and sending the missing ranges, so that
//! nothing needs a queue that could be lost in a crash: the host's own operations are in its
//! `Trust` from the moment they are made, and so is what its machine sent that its person may not
//! sign yet, which the round signs, in order, once its reading of the log and of the device's own
//! chain lets it ([`Trust::sign_waiting`]). And it reads what the relay holds after its cursor, takes
//! each record in as it would from anyone ([`Trust::take_operation`]), and returns the events the
//! machine is to be told.
//!
//! A round is a [`RoundMachine`]: a state machine that says which request it wants put to the relay
//! and is given the answer, and that reads no clock, opens nothing and waits on nothing. A
//! `Courier` is that machine driven over a synchronous link by the one driver ([`drive`]), for a
//! Rust host that has one; a host whose operating system decides when it may run drives the machine
//! itself, or drives it over a [`Transport`] with a deadline and a cancellation, and the state a
//! courier keeps between rounds ([`CourierState`]) is changed only where a step of a round was
//! completed, so that a round that does not end costs nothing but the round.
//!
//! A host connected to several relays has one `Trust` and a courier for each: what one relay did
//! not pass on, another gives. A host that has no hub puts [`Keep`] behind its machine, so that what
//! the machine sends is signed and remembered by the host ([`keep`]) and is carried by the courier.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_relay_transport::{Notice, Signal, Transport, TransportError};
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry, LogError};
use sce_rust_trust::message;
use sce_rust_trust::{Checkpoint, CheckpointError, Head, Sent, TakeError, Trust, Waited};
use serde_json::{json, Value};

use crate::driver::drive;
use crate::hub::Hub;
use crate::logs::list_digest;
use crate::protocol::{Code, Record, Request, Response};
use crate::relay::Relay;
use crate::store::Store;

/// A way to put a request to one relay and read its answer, as the protocol's values: the view of a
/// [`Transport`] a Rust host and a test speak, and what a [`Courier`] and [`drive`] carry an
/// exchange over. Every transport is a link (the texts written and read here); a link a test
/// writes over another, to change what a relay seems to answer, is one too.
pub trait Link {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError>;

    /// [`Self::call`], by `deadline` and giving up once `cancel` is raised. A link that does not
    /// block (a relay in this process, a test's) keeps this default, which calls; a transport
    /// honours both ([`Transport::exchange`]).
    fn call_within(
        &mut self,
        request: &Request,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<Response, LinkError> {
        let _ = (deadline, cancel);
        self.call(request)
    }
}

/// A transport is a link: the request is written as the protocol's text and the response read
/// from it, and a text that is no response of the protocol is a failure of the link, as the peer
/// core takes one. The relay's endpoint refusing the device's token before it carried the request
/// is the relay's refusal of the token, the answer it gives one that expires on a connection that
/// stays open, so that a host renews its token on one sign whichever way it was refused; and its
/// refusing the connection for the rate is the relay's `rate_limited` with the same wait, so that
/// the driver waits out either alike.
impl<T: Transport + ?Sized> Link for T {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
        self.call_within(request, None, &Signal::new())
    }

    fn call_within(
        &mut self,
        request: &Request,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<Response, LinkError> {
        match self.exchange(&request.to_json(), deadline, cancel) {
            Ok(text) => Response::from_json(&text).map_err(LinkError),
            Err(TransportError::Unauthorized(reason)) => {
                Ok(Response::refused(Code::Unauthorized, reason))
            }
            Err(TransportError::RateLimited {
                retry_after_ms,
                reason,
            }) => Ok(Response::refused(
                Code::RateLimited { retry_after_ms },
                reason,
            )),
            Err(error) => Err(LinkError(error.to_string())),
        }
    }
}

/// The link did not carry the request or the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError(pub String);

/// A transport to a relay in this process: the tests', and a host that runs its relay beside it.
/// It never blocks, holds no connection, and has nothing to tell its host. Made with the empty
/// token, it presents none, as a device that logs in does, from no address.
pub struct DirectLink<S: Store> {
    relay: Arc<Mutex<Relay<S>>>,
    token: String,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    network: Signal,
}

impl<S: Store> DirectLink<S> {
    pub fn new(
        relay: Arc<Mutex<Relay<S>>>,
        token: String,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Self {
        Self {
            relay,
            token,
            clock,
            network: Signal::new(),
        }
    }
}

impl<S: Store + Send + 'static> Transport for DirectLink<S> {
    fn exchange(
        &mut self,
        request: &str,
        _deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError> {
        if cancel.is_raised() {
            return Err(TransportError::Cancelled);
        }
        let now = (self.clock)();
        let token = Some(self.token.as_str()).filter(|token| !token.is_empty());
        Ok(self.relay.answer(token, None, request, now))
    }

    fn take_notices(&mut self) -> Vec<Notice> {
        Vec::new()
    }

    fn network(&self) -> Signal {
        self.network.clone()
    }

    fn set_token(&mut self, token: &str) -> bool {
        self.token = token.to_string();
        true
    }
}

/// What went wrong with a round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourierError {
    /// The link failed.
    Link(String),
    /// The relay refused the round as a whole: the token, the allowance, the store.
    Relay { code: Code, reason: String },
    /// The relay answered with something other than the answer to the request.
    Unexpected(String),
}

impl From<LinkError> for CourierError {
    fn from(error: LinkError) -> Self {
        CourierError::Link(error.0)
    }
}

/// What a round did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Round {
    /// The events for the machine to be told, in order: the roster when the log moved, then what
    /// an earlier round took in and did not hand over because it did not end ([`CourierState`]),
    /// then each operation the machine may be told now by the rule of causal delivery
    /// ([`Trust::deliver`]): every one the machine was not told whose context it was told, in an
    /// order that is a linear extension of the contexts, and every one of them for a resync.
    pub events: Vec<(String, String)>,
    /// How many operations and checkpoints the relay was given that it lacked.
    pub pushed: usize,
    /// What the host or the relay refused and why: each is a record that is not taken, and the
    /// rest of the round goes on.
    pub refused: Vec<String>,
    /// The key packages, commits and Welcomes of the list's MLS group that were read, in the order
    /// the relay holds them: for the host's MLS device to take in.
    pub messages: Vec<Record>,
    /// Operations (device, number) the host told its machine and stopped holding: an entry of the log
    /// put them past the last accepted of their device, or the device was found to have signed
    /// conflicting operations and nothing of it is held from the first of them on. The machine was
    /// told them and cannot take them back, so the host builds it again and calls
    /// [`Courier::resync`]. Empty when nothing was.
    pub evicted: Vec<(u64, u64)>,
    /// Operations (device, number) the host did not tell its machine and now does: the conflict that
    /// held a device back went away with the operations that made it. The machine is built again as
    /// for `evicted`.
    pub restored: Vec<(u64, u64)>,
    /// What the machine sent that waited to be signed ([`Kept::Waiting`]) and the round signed, a
    /// new operation each, in the order it was sent, each `(event, data)` as the machine sent it:
    /// the relay was given it with the rest of what it lacked.
    pub taken: Vec<(String, String)>,
    /// What the machine sent that waited to be signed and that the host held already when its turn
    /// came (the round read the device's own chain back, and it held the change): each
    /// `(event, data)`, in the order it was sent. Nothing was signed for it and the relay was given
    /// nothing new ([`sce_rust_trust::Waited::held`]).
    pub held: Vec<(String, String)>,
    /// What the machine sent that waited to be signed and never will be, each `(event, data, why)`:
    /// the log the round read says the person may not write (a reader, a device taken off). The
    /// machine took each in as made and no other copy will hold it, so the host builds the machine
    /// again without it, as for `evicted`, and a host that keeps its own log of commands takes it
    /// out of that log.
    pub dropped: Vec<(String, String, String)>,
    /// The round stopped before the end of what the relay holds: a record names an entry of the
    /// log the host does not have, or the device may not read yet. A later round goes on from there.
    pub stalled: bool,
    /// The number this host's person's own chain is frozen from, when the round froze it: the relay
    /// answered `conflict` to an operation of the device's own and the round took in what the relay
    /// held, or the round read an operation that conflicts with one of the device's. Every host
    /// that holds both freezes the device there; the device signs nothing more, and an
    /// administrator takes it off and grants a new one (ADR 0083 section 4). A round that a
    /// `conflict` froze ends there. `None` when the round froze nothing of the device's own.
    pub conflicted: Option<u64>,
}

/// What a courier keeps of one relay between its rounds, apart from the link it speaks over and the
/// [`Trust`] it holds: where it is in what the relay holds, what the machine was last told of the
/// roster, and what it holds back or has given.
///
/// A round changes it only where a step of the round was completed, so that a round that does not
/// end (the link fails, the answer never comes, the host abandons it or is stopped) leaves it as a
/// state the next round starts cleanly from:
///
/// - the cursor moves past a batch of the feed once every record of the batch was taken in, or to
///   the record a stall stopped at, and never past a record that was not taken in;
/// - the roster is counted as told only when a round ends and hands it over;
/// - a proof of a conflict is counted as given only once the relay accepted it;
/// - the amendments of the log are counted when the relay is read again from the start for them,
///   and a round that stops in that reading leaves the cursor where it stopped, so the next round
///   goes on with it;
/// - what a round took in and did not hand over, because it did not end, is kept here: the events
///   for the machine and the group's messages it read, and (from a host that composes rounds,
///   [`Self::hold`]) what the machine is to be rebuilt without or with, and what became of the sends
///   that waited to be signed. The next round hands them over first, the events after the roster. A
///   [`RoundKind::Resync`] reads everything again and drops the events and messages, and a
///   [`RoundKind::Reread`] drops the messages, which it reads again; what the machine is to be
///   rebuilt by, and what became of a send, is never dropped.
///
/// What a round took into the [`Trust`] stays there whether the round ends or not: the trust
/// holds what verified, and a record given again is taken as one held already.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CourierState {
    /// How many records of the relay were read.
    cursor: u64,
    /// How long the log was when the machine was last told the roster.
    roster_len: u64,
    /// Whether the machine is a hub's, which is told who ever wrote.
    hub: bool,
    /// Operations that were read and that this host holds no key of the epoch of: tried again each
    /// round, since the key may come.
    parked: Vec<(String, String)>,
    /// How many amendments of the log this courier has read the relay again for.
    amendments_seen: usize,
    /// The proof of a conflict this courier has given the relay (by the data it travels as).
    evidence_sent: BTreeSet<String>,
    /// Events for the machine that a round took in and did not hand over, in order.
    untold: Vec<(String, String)>,
    /// The group's messages that a round read and did not hand over, in the order read.
    unread: Vec<Record>,
    /// Operations (device, number) a round found the machine is to be rebuilt without, and did not
    /// hand over ([`Round::evicted`]).
    evicted: Vec<(u64, u64)>,
    /// Operations a round found the machine is to be rebuilt with, and did not hand over
    /// ([`Round::restored`]).
    restored: Vec<(u64, u64)>,
    /// Sends that waited, which a round signed and did not say so of ([`Round::taken`]).
    taken: Vec<(String, String)>,
    /// Sends that waited, which a round found held and did not say so of ([`Round::held`]).
    held: Vec<(String, String)>,
    /// Sends that waited, which a round refused for good and did not say so of
    /// ([`Round::dropped`]).
    dropped: Vec<(String, String, String)>,
    /// The instant the relay's history of the list began, as its heads last said: set for a list a
    /// device created with no token, which lapses and is created again with a new one.
    since: Option<u64>,
}

impl CourierState {
    /// Keep what `round` hands over (its events, the group's messages, what the machine is to be
    /// rebuilt without or with, and what became of the sends that waited) for the next round to
    /// hand over, before what is kept already: what a host that composes rounds of its own took
    /// from a round and did not hand over.
    pub fn hold(&mut self, round: Round) {
        let Round {
            mut events,
            mut messages,
            mut evicted,
            mut restored,
            mut taken,
            mut held,
            mut dropped,
            ..
        } = round;
        events.append(&mut self.untold);
        self.untold = events;
        messages.append(&mut self.unread);
        self.unread = messages;
        evicted.append(&mut self.evicted);
        self.evicted = evicted;
        restored.append(&mut self.restored);
        self.restored = restored;
        taken.append(&mut self.taken);
        self.taken = taken;
        held.append(&mut self.held);
        self.held = held;
        dropped.append(&mut self.dropped);
        self.dropped = dropped;
    }

    /// How many events for the machine are kept for the next round to hand over.
    pub fn untold(&self) -> usize {
        self.untold.len()
    }

    /// The state as one text, for a host's storage: a JSON object, every 64-bit number as text.
    pub fn to_text(&self) -> String {
        let pairs = |pairs: &[(String, String)]| {
            pairs
                .iter()
                .map(|(event, data)| json!([event, data]))
                .collect::<Vec<_>>()
        };
        let operations = |operations: &[(u64, u64)]| {
            operations
                .iter()
                .map(|(node, seq)| json!([node.to_string(), seq.to_string()]))
                .collect::<Vec<_>>()
        };
        let mut value = json!({
            "cursor": self.cursor.to_string(),
            "rosterLen": self.roster_len.to_string(),
            "hub": self.hub,
            "parked": pairs(&self.parked),
            "amendmentsSeen": self.amendments_seen.to_string(),
            "evidenceSent": self.evidence_sent.iter().collect::<Vec<_>>(),
            "untold": pairs(&self.untold),
            "unread": self.unread.iter().map(Record::to_value).collect::<Vec<_>>(),
            "evicted": operations(&self.evicted),
            "restored": operations(&self.restored),
            "taken": pairs(&self.taken),
            "dropped": self
                .dropped
                .iter()
                .map(|(event, data, why)| json!([event, data, why]))
                .collect::<Vec<_>>(),
        });
        // Written only for a list whose history at the relay has a beginning, so that the text of
        // every other list is the one it was.
        if let Some(since) = self.since {
            value["since"] = json!(since.to_string());
        }
        // Written only when a round left some, so that the text of a state that holds none is the
        // one it was before sends found held were told apart from sends signed.
        if !self.held.is_empty() {
            value["held"] = json!(pairs(&self.held));
        }
        value.to_string()
    }

    /// Read back what [`Self::to_text`] wrote; `None` when the text is not that. A text written
    /// before the sends that wait were kept holds no `taken` and no `dropped`, and one written
    /// before sends found held were told apart holds no `held`: each is read as none.
    pub fn from_text(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let number = |name: &str| -> Option<u64> { value.get(name)?.as_str()?.parse::<u64>().ok() };
        let pairs = |name: &str| -> Option<Vec<(String, String)>> {
            value
                .get(name)?
                .as_array()?
                .iter()
                .map(|pair| -> Option<(String, String)> {
                    match pair.as_array()?.as_slice() {
                        [event, data] => {
                            Some((event.as_str()?.to_string(), data.as_str()?.to_string()))
                        }
                        _ => None,
                    }
                })
                .collect()
        };
        let operations = |name: &str| -> Option<Vec<(u64, u64)>> {
            value
                .get(name)?
                .as_array()?
                .iter()
                .map(|pair| -> Option<(u64, u64)> {
                    match pair.as_array()?.as_slice() {
                        [node, seq] => {
                            Some((node.as_str()?.parse().ok()?, seq.as_str()?.parse().ok()?))
                        }
                        _ => None,
                    }
                })
                .collect()
        };
        let triples = |name: &str| -> Option<Vec<(String, String, String)>> {
            value
                .get(name)?
                .as_array()?
                .iter()
                .map(|triple| -> Option<(String, String, String)> {
                    match triple.as_array()?.as_slice() {
                        [event, data, why] => Some((
                            event.as_str()?.to_string(),
                            data.as_str()?.to_string(),
                            why.as_str()?.to_string(),
                        )),
                        _ => None,
                    }
                })
                .collect()
        };
        let absent = |name: &str| value.get(name).is_none();
        Some(Self {
            cursor: number("cursor")?,
            roster_len: number("rosterLen")?,
            hub: value.get("hub")?.as_bool()?,
            parked: pairs("parked")?,
            amendments_seen: usize::try_from(number("amendmentsSeen")?).ok()?,
            evidence_sent: value
                .get("evidenceSent")?
                .as_array()?
                .iter()
                .map(|data| data.as_str().map(str::to_string))
                .collect::<Option<BTreeSet<String>>>()?,
            untold: pairs("untold")?,
            unread: value
                .get("unread")?
                .as_array()?
                .iter()
                .map(Record::from_value)
                .collect::<Option<Vec<Record>>>()?,
            evicted: operations("evicted")?,
            restored: operations("restored")?,
            taken: if absent("taken") {
                Vec::new()
            } else {
                pairs("taken")?
            },
            held: if absent("held") {
                Vec::new()
            } else {
                pairs("held")?
            },
            dropped: if absent("dropped") {
                Vec::new()
            } else {
                triples("dropped")?
            },
            since: if absent("since") {
                None
            } else {
                Some(number("since")?)
            },
        })
    }

    /// The machine of this host is a hub's: it is told who ever wrote, not who writes now.
    pub fn for_a_hub(mut self) -> Self {
        self.hub = true;
        self
    }

    /// Whether the machine of this host is a hub's.
    pub fn is_for_a_hub(&self) -> bool {
        self.hub
    }

    /// How many records of the relay this courier has read: what a host keeps, to go on from there.
    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: u64) {
        self.cursor = cursor;
    }

    /// The relay holds a history of the list that began again, as its heads say (a list that
    /// lapsed there and that a device created again, `LoginMachine::created`, which holds the log
    /// as far as that device's grant and nothing more): the round reads the relay from its start,
    /// and gives it again the proof of a conflict as everything else it lacks; what the machine was
    /// told, and what waits to be handed over, stays as it was.
    fn begin_again(&mut self) {
        self.cursor = 0;
        self.evidence_sent.clear();
    }

    /// How many operations this courier holds back for want of the key of their epoch.
    pub fn parked(&self) -> usize {
        self.parked.len()
    }

    /// What a round that did not end took in and did not hand over, kept for the next one.
    fn keep_untold(&mut self, round: &mut Round) {
        self.untold.append(&mut round.events);
        self.unread.append(&mut round.messages);
        self.evicted.append(&mut round.evicted);
        self.restored.append(&mut round.restored);
        self.taken.append(&mut round.taken);
        self.held.append(&mut round.held);
        self.dropped.append(&mut round.dropped);
    }
}

/// Which round a [`RoundMachine`] makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundKind {
    /// The logs level, the relay given what it lacks, the host given what it lacks.
    Sync,
    /// Everything the relay holds read from the start, and every operation told to the machine,
    /// the ones the host held already included: for a machine that lost its state.
    Resync,
    /// Everything the relay holds read from the start, and only what the host did not hold told to
    /// the machine: for a host's MLS device that was not given a commit the first time (a feed does
    /// not give a record twice), and whose machine is not to be told the list again.
    Reread,
}

/// What a [`RoundMachine`] wants next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Put this request to the relay, and give the machine the answer ([`RoundMachine::respond`]).
    Request(Request),
    /// The round is over: what it did, or why it stopped.
    Done(Result<Round, CourierError>),
}

/// What a host gives a relay outside a round: a message of the list's MLS group, or a checkpoint of
/// its person. Each is one request ([`Publication::request`]) whose answer is checked by
/// [`Publication::check`]; a checkpoint is given after the relay is given what it lacks of the
/// operations the checkpoint names ([`Publication::pushes_first`]), so that a member who reads it
/// can fetch them. [`RoundMachine::publish`] makes the whole exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Publication {
    /// A key package of this host's device, for an administrator to add it with.
    KeyPackage(Vec<u8>),
    /// The commit that begins `epoch` (the log names its hash).
    Commit { epoch: u32, commit: Vec<u8> },
    /// The Welcome for the device of `node` that the commit of `epoch` added.
    Welcome {
        epoch: u32,
        node: u64,
        welcome: Vec<u8>,
    },
    /// The key of `epoch` given to the device of `node`: the text `Trust::grant_key` wrote, which
    /// that device's courier takes in when a round reads it ([`Record::KeyGrant`]).
    KeyGrant {
        epoch: u32,
        node: u64,
        grant: String,
    },
    /// What this host's person says their copy held of everything up to `before` (the digest the
    /// machine computed).
    Checkpoint { before: i64, digest: u32 },
}

impl Publication {
    /// The request that gives the relay this publication. A checkpoint is made here, signed by the
    /// host's person; a host that cannot make one is told why and asks nothing.
    pub fn request(&self, trust: &Trust) -> Result<Request, CourierError> {
        Ok(match self {
            Publication::KeyPackage(package) => Request::PublishKeyPackage {
                data: encoding::encode(package),
            },
            Publication::Commit { epoch, commit } => Request::PublishCommit {
                epoch: *epoch,
                data: encoding::encode(commit),
            },
            Publication::Welcome {
                epoch,
                node,
                welcome,
            } => Request::PublishWelcome {
                epoch: *epoch,
                node: *node,
                data: encoding::encode(welcome),
            },
            Publication::KeyGrant { epoch, node, grant } => Request::PublishKeyGrant {
                epoch: *epoch,
                node: *node,
                data: encoding::encode(grant.as_bytes()),
            },
            Publication::Checkpoint { before, digest } => {
                let checkpoint = trust.checkpoint(*before, *digest).map_err(|error| {
                    CourierError::Unexpected(format!("no checkpoint could be made: {error:?}"))
                })?;
                Request::Checkpoint {
                    checkpoint: checkpoint.to_json(),
                }
            }
        })
    }

    /// Whether the relay is given what it lacks before the publication.
    pub fn pushes_first(&self) -> bool {
        matches!(self, Publication::Checkpoint { .. })
    }

    /// What the relay's answer to [`Self::request`] says: taken, refused, or not an answer to it.
    pub fn check(response: Response) -> Result<(), CourierError> {
        match response {
            Response::Refused { code, reason } => Err(CourierError::Relay { code, reason }),
            Response::Accepted { .. } => Ok(()),
            other => Err(CourierError::Unexpected(format!("{other:?}"))),
        }
    }

    /// The publication as a JSON value, for a host that keeps what it is to give a relay in its
    /// storage: its kind, its bytes in the URL alphabet, and every 64-bit number as text, as
    /// [`Record::to_value`] writes a record.
    pub fn to_value(&self) -> Value {
        match self {
            Publication::KeyPackage(package) => {
                json!({"k": "kp", "d": encoding::encode(package)})
            }
            Publication::Commit { epoch, commit } => {
                json!({"k": "commit", "p": epoch, "d": encoding::encode(commit)})
            }
            Publication::Welcome {
                epoch,
                node,
                welcome,
            } => json!({"k": "welcome", "p": epoch, "n": node.to_string(),
                        "d": encoding::encode(welcome)}),
            Publication::KeyGrant { epoch, node, grant } => json!({"k": "key_grant", "p": epoch,
                        "n": node.to_string(), "d": encoding::encode(grant.as_bytes())}),
            Publication::Checkpoint { before, digest } => {
                json!({"k": "cp", "b": before.to_string(), "g": digest})
            }
        }
    }

    /// Read back what [`Self::to_value`] wrote; `None` when the value is not that.
    pub fn from_value(value: &Value) -> Option<Self> {
        let bytes = || encoding::decode(value.get("d")?.as_str()?);
        let epoch = || u32::try_from(value.get("p")?.as_u64()?).ok();
        match value.get("k")?.as_str()? {
            "kp" => Some(Publication::KeyPackage(bytes()?)),
            "commit" => Some(Publication::Commit {
                epoch: epoch()?,
                commit: bytes()?,
            }),
            "welcome" => Some(Publication::Welcome {
                epoch: epoch()?,
                node: value.get("n")?.as_str()?.parse().ok()?,
                welcome: bytes()?,
            }),
            "key_grant" => Some(Publication::KeyGrant {
                epoch: epoch()?,
                node: value.get("n")?.as_str()?.parse().ok()?,
                grant: String::from_utf8(bytes()?).ok()?,
            }),
            "cp" => Some(Publication::Checkpoint {
                before: value.get("b")?.as_str()?.parse().ok()?,
                digest: u32::try_from(value.get("g")?.as_u64()?).ok()?,
            }),
            _ => None,
        }
    }
}

/// One round of a courier, or one publication, with no link: it says which request it wants put to
/// the relay ([`Self::poll`]), is given the answer ([`Self::respond`]), and says what the round did
/// when it is over. It reads no clock, opens nothing and waits on nothing, so a host drives it as
/// its transport and its operating system let it: an answer may come minutes later, or never.
///
/// It makes the requests a [`Courier`] makes, in the same order, and comes to the same [`Round`]
/// and the same errors: a courier is this machine driven over a [`Link`]. It holds the
/// [`CourierState`] while it runs and gives it back ([`Self::into_state`]), changed only where a
/// step of the round was completed, as [`CourierState`] says; a host that abandons a round takes
/// the state back and starts the next round from it.
pub struct RoundMachine {
    trust: Trust,
    state: CourierState,
    mode: Mode,
    round: Round,
    phase: Phase,
    /// The request put to the relay and not answered yet.
    asked: Option<Request>,
    outcome: Option<Result<Round, CourierError>>,
    /// Where this host's person's own chain was frozen from when the machine was made: what a
    /// round that froze it lower is told apart by ([`Round::conflicted`]).
    frozen_at_start: Option<u64>,
}

/// Where this host's person's own chain is frozen from, when the trust holds operations of it that
/// conflict.
fn own_frozen(trust: &Trust) -> Option<u64> {
    let (own, _) = trust.person()?;
    trust
        .equivocating()
        .into_iter()
        .find(|(node, _)| *node == own)
        .map(|(_, from)| from)
}

/// What a machine is making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// A round, which tells the machine the roster again when `redeliver` (a resync, whose machine
    /// lost what it was told).
    Round { redeliver: bool },
    /// A publication, which tells the machine nothing.
    Publish,
}

/// Where a machine is.
enum Phase {
    /// Bringing the two logs level.
    Log(LogExchange),
    /// Giving the relay what it lacks.
    Push(Push),
    /// Reading what the relay holds after the cursor.
    Pull(Pull),
    /// Asking the relay for what a held operation waits on.
    Fetch(Fetch),
    /// Giving the relay one publication, and reading that it took it.
    Publish(Request),
    /// Everything was asked: the round is to be summed up.
    Finish,
    /// The outcome is known.
    Over,
}

/// The two logs brought level: the relay's entries asked for from what the host holds, until the
/// host takes no more of them, then the ones the relay lacks given to it.
struct LogExchange {
    stage: LogStage,
    then: AfterLog,
}

enum LogStage {
    /// The entries after the host's log are to be asked for.
    Ask,
    /// They were asked for from `from`.
    Asked { from: u64 },
    /// The entries the relay lacks are given, one request each, from `next`.
    Give { entries: Vec<String>, next: usize },
}

/// What follows the logs brought level.
enum AfterLog {
    /// The parked operations are tried again and the relay is given what it lacks: the start of a
    /// round.
    Push,
    /// A record of the feed named an entry the host lacked: it is taken again when the log moved
    /// since `before`, and the feed stalls on it when it did not.
    Retake { pull: Pull, before: u64 },
}

/// The relay given what it lacks: its heads asked for, what it holds of this host's own person's
/// chain past what the host holds read back, the operations of each device past the relay's head
/// given, then the proof of conflicts not given yet, then the host's checkpoints.
struct Push {
    stage: PushStage,
    then: AfterPush,
}

enum PushStage {
    Heads,
    /// The relay's heads came (`heads`): what they name of this host's own person's chain past what
    /// the host holds is read back first (`RoundMachine::read_own`). `after` is the number the
    /// last reading began at, so that a reading that brought nothing is not made again; `walk` is
    /// how far a device that awaits its own chain has read it back past where it and the relay
    /// agree.
    Own {
        heads: Vec<Head>,
        after: Option<u64>,
        walk: Walk,
    },
    /// The operations of `node`, this host's person, numbered `from` to `to` were asked for: as a
    /// step of the walk when `walking`, and as what the heads name past what the host holds when
    /// not.
    OwnAsked {
        heads: Vec<Head>,
        node: u64,
        from: u64,
        to: u64,
        walking: bool,
        walk: Walk,
    },
    Devices(Devices),
    Evidence {
        items: Vec<(String, String)>,
        next: usize,
    },
    Checkpoints {
        items: Vec<String>,
        next: usize,
    },
}

/// How far a device that awaits its own chain ([`Trust::await_own_chain`]) has read it back from
/// the relay past where it and the relay agree ([`Trust::own_chain_walk`]).
///
/// The relay's heads name of a device's chain only the operations it holds from 1 without a gap and
/// below the first conflict among them. A device restored from a store older than what it signed
/// cannot tell from them alone that the relay holds more of its chain: a second operation of its at
/// a number, which freezes its chain at the relay and stops the head there, with everything it
/// signed after; or its operations past a gap. Read back from the heads alone, such a device would
/// take the relay's head for the end of its chain and sign again, at numbers it used, what it signed
/// and lost: one more operation at the relay for each, at every restore. So it reads its chain from
/// where the two agree to where the relay holds nothing more, once, before it signs anything new.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Walk {
    /// Not begun: the first reading starts where the relay's heads and the host agree.
    Begin,
    /// The last reading brought operations the host did not hold, up to the number before this:
    /// the next reading starts here.
    From(u64),
    /// A reading brought nothing the host did not hold: the device holds of its chain what the relay
    /// does.
    Done,
}

/// The number of the operation `data` travels as, when it carries one.
fn seq_in(data: &str) -> Option<u64> {
    serde_json::from_str::<Value>(data)
        .ok()?
        .get(message::SEQ_FIELD)?
        .as_str()?
        .parse()
        .ok()
}

/// The operations of each device the host holds past what the relay holds of it.
struct Devices {
    /// The relay's head of each device.
    theirs: BTreeMap<u64, u64>,
    /// The host's heads, as they were when the relay's came.
    heads: Vec<Head>,
    /// The device whose operations are given.
    device: usize,
    /// Its operations, once read from the trust, and the next to give.
    ops: Option<Ops>,
}

struct Ops {
    list: Vec<(String, String)>,
    next: usize,
}

/// What follows the relay given what it lacks.
enum AfterPush {
    /// The feed is read, when the relay lets this device read.
    Pull,
    /// The publication is given, whether the relay let this device read or not.
    Publish(Request),
}

/// The operations a held one waits on, asked for one range at a time after the feed is read
/// ([`RoundMachine::drive_fetch`]): the ranges asked in this round.
#[derive(Default)]
struct Fetch {
    asked: BTreeSet<(u64, u64, u64)>,
}

/// What the relay holds after the cursor, read a batch at a time.
struct Pull {
    /// Whether this is the reading from the start that an amendment of the log asks for.
    second: bool,
    batch: Option<Batch>,
}

/// One batch of the feed, taken in a record at a time.
struct Batch {
    records: Vec<Record>,
    /// The cursor the batch was read from.
    base: u64,
    /// The record taken in next.
    index: usize,
    /// Whether the log was brought level once for this record already.
    retried: bool,
    next: u64,
    more: bool,
}

/// A refusal of a whole request, as an error of the round.
fn expected(response: Response) -> Result<Response, CourierError> {
    match response {
        Response::Refused { code, reason } => Err(CourierError::Relay { code, reason }),
        other => Ok(other),
    }
}

impl RoundMachine {
    /// A round of `kind` over `state` and the host's `trust`. What an earlier round took in and did
    /// not hand over is handed over by this one ([`CourierState`]).
    pub fn start(kind: RoundKind, mut state: CourierState, trust: Trust) -> Self {
        match kind {
            RoundKind::Sync => {}
            RoundKind::Resync => {
                state.cursor = 0;
                state.untold.clear();
                state.unread.clear();
                // The machine lost what it was told: the round ends by telling it every operation
                // the host holds, in causal order.
                trust.forget_delivered();
            }
            RoundKind::Reread => {
                state.cursor = 0;
                state.unread.clear();
            }
        }
        let round = Round {
            events: std::mem::take(&mut state.untold),
            messages: std::mem::take(&mut state.unread),
            evicted: std::mem::take(&mut state.evicted),
            restored: std::mem::take(&mut state.restored),
            taken: std::mem::take(&mut state.taken),
            held: std::mem::take(&mut state.held),
            dropped: std::mem::take(&mut state.dropped),
            ..Round::default()
        };
        Self {
            frozen_at_start: own_frozen(&trust),
            trust,
            state,
            mode: Mode::Round {
                redeliver: kind == RoundKind::Resync,
            },
            round,
            phase: Phase::Log(LogExchange {
                stage: LogStage::Ask,
                then: AfterLog::Push,
            }),
            asked: None,
            outcome: None,
        }
    }

    /// The exchange that gives the relay `publication`. Its round tells the machine nothing: it
    /// says what the relay was given before a checkpoint, and what it refused.
    pub fn publish(publication: &Publication, state: CourierState, trust: Trust) -> Self {
        let (phase, outcome) = match publication.request(&trust) {
            Ok(request) if publication.pushes_first() => (
                Phase::Push(Push {
                    stage: PushStage::Heads,
                    then: AfterPush::Publish(request),
                }),
                None,
            ),
            Ok(request) => (Phase::Publish(request), None),
            Err(error) => (Phase::Over, Some(Err(error))),
        };
        Self {
            frozen_at_start: own_frozen(&trust),
            trust,
            state,
            mode: Mode::Publish,
            round: Round::default(),
            phase,
            asked: None,
            outcome,
        }
    }

    /// The request to put to the relay next, or the outcome once the round is over. Asking again
    /// before an answer is given is asked the same request; asking after the round is over is told
    /// the same outcome.
    pub fn poll(&mut self) -> Step {
        loop {
            if let Some(outcome) = &self.outcome {
                return Step::Done(outcome.clone());
            }
            if let Some(request) = &self.asked {
                return Step::Request(request.clone());
            }
            let phase = std::mem::replace(&mut self.phase, Phase::Over);
            self.phase = match phase {
                Phase::Log(log) => self.drive_log(log),
                Phase::Push(push) => self.drive_push(push),
                Phase::Pull(pull) => self.drive_pull(pull),
                Phase::Fetch(fetch) => self.drive_fetch(fetch),
                Phase::Publish(request) => {
                    self.asked = Some(request.clone());
                    Phase::Publish(request)
                }
                Phase::Finish => self.finish(),
                Phase::Over => self.fail(CourierError::Unexpected(
                    "the round ended with no outcome".to_string(),
                )),
            };
        }
    }

    /// Whether a request was put and waits for its answer.
    pub fn is_asking(&self) -> bool {
        self.asked.is_some() && self.outcome.is_none()
    }

    /// The answer to the request [`Self::poll`] gave, or why the link did not carry it. An answer
    /// when no request waits for one is not taken.
    pub fn respond(&mut self, answer: Result<Response, LinkError>) {
        if self.outcome.is_some() || self.asked.take().is_none() {
            return;
        }
        let response = match answer {
            Ok(response) => response,
            Err(error) => {
                self.phase = self.fail(error.into());
                return;
            }
        };
        let phase = std::mem::replace(&mut self.phase, Phase::Over);
        self.phase = match phase {
            Phase::Log(log) => self.answer_log(log, response),
            Phase::Push(push) => self.answer_push(push, response),
            Phase::Pull(pull) => self.answer_pull(pull, response),
            Phase::Fetch(fetch) => self.answer_fetch(fetch, response),
            Phase::Publish(_) => match Publication::check(response) {
                Ok(()) => Phase::Finish,
                Err(error) => self.fail(error),
            },
            other @ (Phase::Finish | Phase::Over) => other,
        };
    }

    /// The state as the courier would be left with it if the round were abandoned now: what a host
    /// saves in the middle of a round.
    pub fn abandoned_state(&self) -> CourierState {
        let mut state = self.state.clone();
        if self.outcome.is_none() {
            state.keep_untold(&mut self.round.clone());
        }
        state
    }

    /// The state back, at the end of the round or in the middle of it: a round that did not end
    /// leaves what it took in and did not hand over in it, for the next round.
    pub fn into_state(mut self) -> CourierState {
        if self.outcome.is_none() {
            self.state.keep_untold(&mut self.round);
        }
        self.state
    }

    fn ask(&mut self, request: Request) {
        self.asked = Some(request);
    }

    /// The round stops with `error`; what it took in and did not hand over is kept.
    fn fail(&mut self, error: CourierError) -> Phase {
        self.state.keep_untold(&mut self.round);
        self.outcome = Some(Err(error));
        Phase::Over
    }

    /// The round sums up: what the trust stopped holding or holds again (after what an earlier
    /// round left to hand over), and the roster at the front of the events when the log moved or
    /// every operation is told again.
    fn finish(&mut self) -> Phase {
        let mut round = std::mem::take(&mut self.round);
        if let Mode::Round { redeliver } = self.mode {
            // What the machine may be told now, by the rule of causal delivery, after what an
            // earlier round left to hand over: every operation the round took in or repaired that
            // follows only what the machine was told.
            round.events.extend(self.trust.deliver());
            round.evicted.extend(self.trust.take_evicted());
            round.restored.extend(self.trust.take_restored());
            // The device's own chain froze in this round, lower than it was or where it was not.
            let froze = own_frozen(&self.trust).filter(|from| Some(*from) != self.frozen_at_start);
            if let Some(frozen) = froze {
                round.conflicted = Some(frozen);
                log::info!(
                    list = list_digest(&self.trust.list()).as_str(),
                    node = self.trust.person().map_or(0, |(own, _)| own),
                    frozen = frozen,
                    evicted = round.evicted.len();
                    "own_chain_frozen"
                );
            }
            let len = self.trust.log_len();
            if len != self.state.roster_len || redeliver {
                self.state.roster_len = len;
                let roster = if self.state.hub {
                    self.trust.hub_role_events()
                } else {
                    self.trust.role_events()
                };
                round.events.splice(0..0, roster);
            }
        }
        self.outcome = Some(Ok(round));
        Phase::Over
    }

    fn drive_log(&mut self, log: LogExchange) -> Phase {
        let LogExchange { stage, then } = log;
        match stage {
            // `Asked` is left by every answer, so it is not met here; were it, it asks again.
            LogStage::Ask | LogStage::Asked { .. } => {
                let from = self.trust.log_len();
                self.ask(Request::Entries { from });
                Phase::Log(LogExchange {
                    stage: LogStage::Asked { from },
                    then,
                })
            }
            LogStage::Give { entries, next } => match entries.get(next).cloned() {
                Some(entry) => {
                    self.ask(Request::Append { entry });
                    Phase::Log(LogExchange {
                        stage: LogStage::Give { entries, next },
                        then,
                    })
                }
                None => self.after_log(then),
            },
        }
    }

    fn answer_log(&mut self, log: LogExchange, response: Response) -> Phase {
        let LogExchange { stage, then } = log;
        match stage {
            LogStage::Asked { from } => {
                let (entries, len) = match expected(response) {
                    Ok(Response::Entries { entries, len }) => (entries, len),
                    Ok(_) => {
                        return self.fail(CourierError::Unexpected(
                            "it did not answer with entries".to_string(),
                        ))
                    }
                    Err(error) => return self.fail(error),
                };
                for text in entries {
                    match Entry::from_json(&text) {
                        Some(entry) => {
                            if let Err(error) = self.trust.append(&entry) {
                                self.round
                                    .refused
                                    .push(format!("an entry of the relay: {error:?}"));
                            }
                        }
                        None => self
                            .round
                            .refused
                            .push("an entry of the relay is not one".to_string()),
                    }
                }
                let now = self.trust.log_len();
                // Go on while the relay has more and the last call took some of it in.
                if now < len && now > from {
                    return Phase::Log(LogExchange {
                        stage: LogStage::Ask,
                        then,
                    });
                }
                let held = self.trust.entries();
                let start = usize::try_from(len).unwrap_or(held.len()).min(held.len());
                Phase::Log(LogExchange {
                    stage: LogStage::Give {
                        entries: held[start..].iter().map(Entry::to_json).collect(),
                        next: 0,
                    },
                    then,
                })
            }
            LogStage::Give { entries, next } => match expected(response) {
                Ok(Response::Appended { .. }) => {
                    self.round.pushed += 1;
                    Phase::Log(LogExchange {
                        stage: LogStage::Give {
                            entries,
                            next: next + 1,
                        },
                        then,
                    })
                }
                Ok(other) => self.fail(CourierError::Unexpected(format!("{other:?}"))),
                Err(error) => self.fail(error),
            },
            // Nothing is asked in it, so no answer is given to it.
            LogStage::Ask => Phase::Log(LogExchange {
                stage: LogStage::Ask,
                then,
            }),
        }
    }

    fn after_log(&mut self, then: AfterLog) -> Phase {
        match then {
            AfterLog::Push => {
                retry_parked(&self.trust, &mut self.state.parked, &mut self.round);
                // The log is level: a send that waited for the grant that lets the device write
                // is signed now, and given the relay with the rest of what it lacks.
                self.sign_waiting();
                Phase::Push(Push {
                    stage: PushStage::Heads,
                    then: AfterPush::Pull,
                })
            }
            AfterLog::Retake { pull, before } => {
                if self.trust.log_len() != before {
                    return Phase::Pull(pull);
                }
                if let Some(batch) = &pull.batch {
                    self.state.cursor = batch.base + batch.index as u64;
                }
                self.round.stalled = true;
                self.after_pull(pull.second)
            }
        }
    }

    fn drive_push(&mut self, push: Push) -> Phase {
        let Push { stage, then } = push;
        match stage {
            PushStage::Heads => {
                self.ask(Request::Heads);
                Phase::Push(Push {
                    stage: PushStage::Heads,
                    then,
                })
            }
            PushStage::Own { heads, after, walk } => self.read_own(heads, after, walk, then),
            // `OwnAsked` is left by its answer, so it is not met here; were it, it asks again.
            PushStage::OwnAsked {
                heads,
                node,
                from,
                to,
                walking,
                walk,
            } => {
                self.ask(Request::Operations { node, from, to });
                Phase::Push(Push {
                    stage: PushStage::OwnAsked {
                        heads,
                        node,
                        from,
                        to,
                        walking,
                        walk,
                    },
                    then,
                })
            }
            PushStage::Devices(devices) => self.drive_devices(devices, then),
            PushStage::Evidence { items, next } => {
                // What shows that a device signed conflicting operations is given to the relay
                // too, so that a host that was not given both of them is given them by the relay:
                // the proof is no host's own knowledge. Each is given once from this courier; the
                // relay holds it already when it was.
                let mut next = next;
                while items
                    .get(next)
                    .is_some_and(|(_, data)| self.state.evidence_sent.contains(data))
                {
                    next += 1;
                }
                match items.get(next).cloned() {
                    Some((event, data)) => {
                        self.ask(Request::Submit { event, data });
                        Phase::Push(Push {
                            stage: PushStage::Evidence { items, next },
                            then,
                        })
                    }
                    None => Phase::Push(Push {
                        stage: PushStage::Checkpoints {
                            items: self
                                .trust
                                .checkpoints()
                                .iter()
                                .map(Checkpoint::to_json)
                                .collect(),
                            next: 0,
                        },
                        then,
                    }),
                }
            }
            PushStage::Checkpoints { items, next } => match items.get(next).cloned() {
                Some(checkpoint) => {
                    self.ask(Request::Checkpoint { checkpoint });
                    Phase::Push(Push {
                        stage: PushStage::Checkpoints { items, next },
                        then,
                    })
                }
                None => self.after_push(true, then),
            },
        }
    }

    /// How far the chain of `node`, held up to `head`, may be given to the relay: all of it, but
    /// for this host's person's own chain while its host has not recorded an operation of it in
    /// its journal, which stops before the first such (docs/adr/0083, section 3.1;
    /// [`Trust::released_through`]). What the relay is not given now is given by the round after
    /// the host recorded it.
    fn released(&self, node: u64, head: u64) -> u64 {
        let own = self.trust.person().map(|(own, _)| own);
        match self.trust.released_through() {
            Some(released) if own == Some(node) => head.min(released),
            _ => head,
        }
    }

    fn drive_devices(&mut self, devices: Devices, then: AfterPush) -> Phase {
        let Devices {
            theirs,
            heads,
            mut device,
            ops,
        } = devices;
        let ops = match ops {
            Some(ops) => ops,
            None => loop {
                let Some(head) = heads.get(device).copied() else {
                    return self.evidence(then);
                };
                let held = theirs.get(&head.node).copied().unwrap_or(0);
                let top = self.released(head.node, head.seq);
                if top <= held {
                    device += 1;
                    continue;
                }
                break Ops {
                    list: self.trust.operations(head.node, held + 1, top),
                    next: 0,
                };
            },
        };
        match ops.list.get(ops.next).cloned() {
            Some((event, data)) => {
                self.ask(Request::Submit { event, data });
                Phase::Push(Push {
                    stage: PushStage::Devices(Devices {
                        theirs,
                        heads,
                        device,
                        ops: Some(ops),
                    }),
                    then,
                })
            }
            None => Phase::Push(Push {
                stage: PushStage::Devices(Devices {
                    theirs,
                    heads,
                    device: device + 1,
                    ops: None,
                }),
                then,
            }),
        }
    }

    /// The relay's heads came. What they name of this host's own person's chain past what the host
    /// holds (a device whose state came back from a store older than its last operation) is read
    /// back before the relay is given anything, so that the device's next operation is numbered past
    /// every one it made; in a round only, since a publication tells the machine nothing, and what it
    /// took in would not be told again. Once the host holds the chain as far as the relay does, a
    /// wait for it ends ([`Trust::confirm_own_chain`]); a reading that brought nothing new is not
    /// made again, and the device goes on waiting. Then the operations the relay lacks are given.
    ///
    /// A device that waits for its own chain does not read it back by the heads alone: it walks it
    /// ([`Walk`]), from where it and the relay agree, a reading at a time, to where the relay holds
    /// nothing it does not, and only then does the wait end and what waited get signed. A chain the
    /// relay holds past a conflict or a gap is so read back as well, and a device that signed two
    /// histories finds it before it signs a third, and signs nothing more.
    fn read_own(
        &mut self,
        heads: Vec<Head>,
        after: Option<u64>,
        walk: Walk,
        then: AfterPush,
    ) -> Phase {
        let reading = matches!(self.mode, Mode::Round { .. });
        let next = if !reading {
            None
        } else if self.trust.awaits_own_chain() {
            let from = match walk {
                Walk::Begin => self.trust.own_chain_walk(&heads),
                Walk::From(from) => self.trust.person().map(|(node, _)| (node, from)),
                Walk::Done => None,
            };
            from.map(|(node, from)| (node, from, u64::MAX, true))
        } else {
            self.trust
                .own_chain_behind(&heads)
                .filter(|(_, from, _)| after.is_none_or(|last| *from > last))
                .map(|(node, from, to)| (node, from, to, false))
        };
        match next {
            Some((node, from, to, walking)) => {
                self.ask(Request::Operations { node, from, to });
                Phase::Push(Push {
                    stage: PushStage::OwnAsked {
                        heads,
                        node,
                        from,
                        to,
                        walking,
                        walk,
                    },
                    then,
                })
            }
            None => {
                // A send that waited for the device's own chain is signed at the number after the
                // last of it, before the heads the relay is given from are read. A publication reads
                // nothing of the chain back, so it does not end the wait.
                //
                // What the chain brought back is told to the machine first: the device's own
                // column of the delivered vector says how far the machine holds its own chain, and
                // signing a new operation counts the machine as holding everything below it, so an
                // operation read back and told only when the round ends would be counted told and
                // never told.
                if reading {
                    self.trust.confirm_own_chain(&heads);
                    self.round.events.extend(self.trust.deliver());
                    self.sign_waiting();
                }
                let theirs = heads.iter().map(|head| (head.node, head.seq)).collect();
                Phase::Push(Push {
                    stage: PushStage::Devices(Devices {
                        theirs,
                        heads: self.trust.heads(),
                        device: 0,
                        ops: None,
                    }),
                    then,
                })
            }
        }
    }

    /// Sign what the machine sent that waits, as far as what the round has read of the log and of
    /// the device's own chain lets it ([`Trust::sign_waiting`]), and keep in the round what became
    /// of it.
    fn sign_waiting(&mut self) {
        let waited = self.trust.sign_waiting();
        note_waited(&self.trust, &waited);
        self.round.taken.extend(waited.taken);
        self.round.held.extend(waited.held);
        self.round.dropped.extend(waited.dropped);
    }

    /// The relay kept the operation it was given and answered `conflict` (`code`, ADR 0083 section
    /// 4): it conflicts with what the relay held of its author. What the relay held is taken in as
    /// from anyone, so that this host holds the proof the relay and every member hold, and the
    /// relay was given a record it lacked. When that froze this host's person's own chain since the
    /// exchange began, the device signs nothing more (the trust refuses it) and the exchange ends:
    /// a round is summed up, and says so ([`Round::conflicted`]); a publication is not given, and
    /// fails with the relay's answer. `None` when the exchange goes on: the author is another
    /// device, which this courier carried, or the device's chain was frozen there already.
    fn answer_conflict(&mut self, code: Code, reason: String) -> Option<Phase> {
        let Code::Conflict {
            node,
            seq,
            frozen,
            held,
            ..
        } = &code
        else {
            return None;
        };
        let (node, seq, frozen, given) = (*node, *seq, *frozen, held.len());
        let mut read = Round::default();
        for (event, data) in held {
            let record = Record::Operation {
                event: event.clone(),
                data: data.clone(),
            };
            take(&self.trust, &mut self.state.parked, &record, &mut read);
        }
        self.round.refused.append(&mut read.refused);
        self.round.pushed += 1;
        let own = self.trust.person().is_some_and(|(own, _)| own == node);
        let froze =
            own && own_frozen(&self.trust).is_some_and(|from| Some(from) != self.frozen_at_start);
        log::info!(
            list = list_digest(&self.trust.list()).as_str(),
            node = node,
            seq = seq,
            frozen = frozen,
            held = given,
            own = own,
            ends = froze;
            "conflict_answered"
        );
        if !froze {
            return None;
        }
        Some(match self.mode {
            Mode::Round { .. } => Phase::Finish,
            Mode::Publish => self.fail(CourierError::Relay { code, reason }),
        })
    }

    /// The devices are done: the proof of conflicts the trust holds is given next.
    fn evidence(&self, then: AfterPush) -> Phase {
        Phase::Push(Push {
            stage: PushStage::Evidence {
                items: self.trust.evidence(),
                next: 0,
            },
            then,
        })
    }

    fn answer_push(&mut self, push: Push, response: Response) -> Phase {
        let Push { stage, then } = push;
        match stage {
            PushStage::Heads => match response {
                Response::Heads { heads, since, .. } => {
                    // The relay's history of the list began again since this courier last read
                    // it (the list lapsed there and a device created it again): what the relay
                    // holds is read from its start, and it is given again what it lacks.
                    if since != self.state.since {
                        self.state.begin_again();
                        self.state.since = since;
                    }
                    Phase::Push(Push {
                        stage: PushStage::Own {
                            heads,
                            after: None,
                            walk: Walk::Begin,
                        },
                        then,
                    })
                }
                // The relay will not let this device read yet (it is not a member), in which case
                // there is nothing to read either.
                Response::Refused {
                    code: Code::Forbidden,
                    reason,
                } => {
                    self.round.stalled = true;
                    self.round.refused.push(reason);
                    self.after_push(false, then)
                }
                Response::Refused { code, reason } => {
                    self.fail(CourierError::Relay { code, reason })
                }
                other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
            },
            // Each operation read back is taken in as from anyone, and told to the machine when the
            // round ends if the machine was not told it: it is the device's own, and a machine
            // whose state came back with the trust's holds no more of it than the trust's delivered
            // vector says ([`Trust::deliver`]).
            //
            // A step of the walk goes on from the number after the highest it read, while it brings
            // an operation the host did not hold; one that brings none ends the walk.
            PushStage::OwnAsked {
                heads,
                from,
                walking,
                walk,
                ..
            } => match response {
                Response::Operations { operations } => {
                    let mut read = Round::default();
                    let mut brought = false;
                    let mut highest: Option<u64> = None;
                    for (event, data) in operations {
                        highest = highest.max(seq_in(&data));
                        let taken = take(
                            &self.trust,
                            &mut self.state.parked,
                            &Record::Operation { event, data },
                            &mut read,
                        );
                        brought |= matches!(taken, Taken::New);
                    }
                    self.round.refused.append(&mut read.refused);
                    let walk = if !walking {
                        walk
                    } else {
                        match highest {
                            Some(top) if brought && top >= from => {
                                top.checked_add(1).map_or(Walk::Done, Walk::From)
                            }
                            _ => Walk::Done,
                        }
                    };
                    Phase::Push(Push {
                        stage: PushStage::Own {
                            heads,
                            after: Some(from),
                            walk,
                        },
                        then,
                    })
                }
                // Not read back: the device goes on waiting for its chain, and the round goes on.
                Response::Refused { reason, .. } => {
                    self.round.refused.push(reason);
                    Phase::Push(Push {
                        stage: PushStage::Own {
                            heads,
                            after: Some(from),
                            walk: if walking { Walk::Done } else { walk },
                        },
                        then,
                    })
                }
                other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
            },
            // Nothing is asked in it, so no answer is given to it.
            PushStage::Own { heads, after, walk } => Phase::Push(Push {
                stage: PushStage::Own { heads, after, walk },
                then,
            }),
            PushStage::Devices(devices) => {
                let Devices {
                    theirs,
                    heads,
                    device,
                    ops,
                } = devices;
                match response {
                    Response::Accepted { new, .. } => {
                        self.round.pushed += usize::from(new);
                        Phase::Push(Push {
                            stage: PushStage::Devices(Devices {
                                theirs,
                                heads,
                                device,
                                ops: ops.map(|ops| Ops {
                                    list: ops.list,
                                    next: ops.next + 1,
                                }),
                            }),
                            then,
                        })
                    }
                    // Kept as proof: the round goes on to the next, unless it froze the device's own
                    // chain.
                    Response::Refused {
                        code: code @ Code::Conflict { .. },
                        reason,
                    } => match self.answer_conflict(code, reason) {
                        Some(end) => end,
                        None => Phase::Push(Push {
                            stage: PushStage::Devices(Devices {
                                theirs,
                                heads,
                                device,
                                ops: ops.map(|ops| Ops {
                                    list: ops.list,
                                    next: ops.next + 1,
                                }),
                            }),
                            then,
                        }),
                    },
                    // A reader holds operations and may not write them anywhere.
                    Response::Refused {
                        code: Code::Forbidden,
                        reason,
                    } => {
                        self.round.refused.push(reason);
                        self.evidence(then)
                    }
                    Response::Refused { reason, .. } => {
                        self.round.refused.push(reason);
                        Phase::Push(Push {
                            stage: PushStage::Devices(Devices {
                                theirs,
                                heads,
                                device: device + 1,
                                ops: None,
                            }),
                            then,
                        })
                    }
                    other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
                }
            }
            PushStage::Evidence { items, next } => match response {
                Response::Accepted { new, .. } => {
                    self.round.pushed += usize::from(new);
                    if let Some((_, data)) = items.get(next) {
                        self.state.evidence_sent.insert(data.clone());
                    }
                    Phase::Push(Push {
                        stage: PushStage::Evidence {
                            items,
                            next: next + 1,
                        },
                        then,
                    })
                }
                // The proof was kept, which is what it was given for.
                Response::Refused {
                    code: code @ Code::Conflict { .. },
                    reason,
                } => {
                    if let Some((_, data)) = items.get(next) {
                        self.state.evidence_sent.insert(data.clone());
                    }
                    match self.answer_conflict(code, reason) {
                        Some(end) => end,
                        None => Phase::Push(Push {
                            stage: PushStage::Evidence {
                                items,
                                next: next + 1,
                            },
                            then,
                        }),
                    }
                }
                Response::Refused { reason, .. } => {
                    self.round.refused.push(reason);
                    Phase::Push(Push {
                        stage: PushStage::Evidence {
                            items,
                            next: next + 1,
                        },
                        then,
                    })
                }
                other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
            },
            PushStage::Checkpoints { items, next } => match response {
                Response::Accepted { new, .. } => {
                    self.round.pushed += usize::from(new);
                    Phase::Push(Push {
                        stage: PushStage::Checkpoints {
                            items,
                            next: next + 1,
                        },
                        then,
                    })
                }
                Response::Refused { reason, .. } => {
                    self.round.refused.push(reason);
                    Phase::Push(Push {
                        stage: PushStage::Checkpoints {
                            items,
                            next: next + 1,
                        },
                        then,
                    })
                }
                other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
            },
        }
    }

    /// The relay was given what it lacks; `readable` is false when it would not let this device
    /// read, and a round then reads nothing.
    fn after_push(&mut self, readable: bool, then: AfterPush) -> Phase {
        match then {
            AfterPush::Pull if readable => Phase::Pull(Pull {
                second: false,
                batch: None,
            }),
            AfterPush::Pull => Phase::Finish,
            AfterPush::Publish(request) => Phase::Publish(request),
        }
    }

    /// Read what the relay holds after the cursor, and take each record in. A record that names an
    /// entry of the log the host lacks stops the round after one more try at the log, and the
    /// cursor stays before it.
    fn drive_pull(&mut self, pull: Pull) -> Phase {
        let Pull { second, batch } = pull;
        let Some(mut batch) = batch else {
            self.ask(Request::Feed {
                cursor: self.state.cursor,
                limit: FEED_BATCH,
            });
            return Phase::Pull(Pull {
                second,
                batch: None,
            });
        };
        while batch.index < batch.records.len() {
            let taken = take(
                &self.trust,
                &mut self.state.parked,
                &batch.records[batch.index],
                &mut self.round,
            );
            match taken {
                Taken::Done | Taken::New => {
                    batch.index += 1;
                    batch.retried = false;
                }
                Taken::Behind if !batch.retried => {
                    batch.retried = true;
                    let before = self.trust.log_len();
                    return Phase::Log(LogExchange {
                        stage: LogStage::Ask,
                        then: AfterLog::Retake {
                            pull: Pull {
                                second,
                                batch: Some(batch),
                            },
                            before,
                        },
                    });
                }
                Taken::Behind => {
                    self.state.cursor = batch.base + batch.index as u64;
                    self.round.stalled = true;
                    return self.after_pull(second);
                }
            }
        }
        self.state.cursor = batch.next;
        if batch.more {
            Phase::Pull(Pull {
                second,
                batch: None,
            })
        } else {
            self.after_pull(second)
        }
    }

    fn answer_pull(&mut self, pull: Pull, response: Response) -> Phase {
        let Pull { second, .. } = pull;
        match expected(response) {
            Ok(Response::Feed {
                records,
                next,
                more,
            }) => Phase::Pull(Pull {
                second,
                batch: Some(Batch {
                    records,
                    base: self.state.cursor,
                    index: 0,
                    retried: false,
                    next,
                    more,
                }),
            }),
            Ok(_) => self.fail(CourierError::Unexpected(
                "it did not answer with a feed".to_string(),
            )),
            Err(error) => self.fail(error),
        }
    }

    /// A reading of the feed ended. An amendment raised the last operation accepted of a device
    /// taken off: what the host refused of it before is true now, so it reads what the relay holds
    /// again from the start (the operations it already holds are not told to the machine twice).
    fn after_pull(&mut self, second: bool) -> Phase {
        if second {
            return Phase::Fetch(Fetch::default());
        }
        let amends = amendments(&self.trust);
        if amends > self.state.amendments_seen {
            self.state.amendments_seen = amends;
            self.state.cursor = 0;
            Phase::Pull(Pull {
                second: true,
                batch: None,
            })
        } else {
            Phase::Fetch(Fetch::default())
        }
    }

    /// Ask the relay for the next range of operations a held one waits on ([`Trust::lacking`]):
    /// the gap before a device's next operation, or what a context names that the host does not
    /// hold. A range is asked once in a round; one the relay does not fill is asked again by the
    /// next round, and what waits on it stays held, never skipped (docs/adr/0085, section 3).
    fn drive_fetch(&mut self, mut fetch: Fetch) -> Phase {
        let next = self
            .trust
            .lacking()
            .into_iter()
            .find(|range| !fetch.asked.contains(range));
        let Some(range) = next else {
            return Phase::Finish;
        };
        fetch.asked.insert(range);
        let (node, from, to) = range;
        self.ask(Request::Operations { node, from, to });
        Phase::Fetch(fetch)
    }

    /// The operations asked for, each taken in as from anyone. A relay that refuses the range is
    /// noted, and the round goes on: what waits on it stays held.
    fn answer_fetch(&mut self, fetch: Fetch, response: Response) -> Phase {
        match response {
            Response::Operations { operations } => {
                for (event, data) in operations {
                    take(
                        &self.trust,
                        &mut self.state.parked,
                        &Record::Operation { event, data },
                        &mut self.round,
                    );
                }
                Phase::Fetch(fetch)
            }
            Response::Refused { reason, .. } => {
                self.round.refused.push(reason);
                Phase::Fetch(fetch)
            }
            other => self.fail(CourierError::Unexpected(format!("{other:?}"))),
        }
    }
}

/// A host's courier to one relay.
pub struct Courier<L: Link> {
    trust: Trust,
    link: L,
    state: CourierState,
}

const FEED_BATCH: u32 = 200;

/// The most operations a courier parks for want of a key.
const MAX_PARKED: usize = 100_000;

/// How many entries of the log amend the last operation accepted of a device taken off.
fn amendments(trust: &Trust) -> usize {
    trust
        .entries()
        .iter()
        .filter(|entry| matches!(entry.body, Body::Amend { .. }))
        .count()
}

/// Try again the operations that were held back for want of a key: those that open now are held,
/// and told to the machine when the round ends ([`Trust::deliver`]).
fn retry_parked(trust: &Trust, parked: &mut Vec<(String, String)>, round: &mut Round) {
    let waiting = std::mem::take(parked);
    for (event, data) in waiting {
        match trust.take_operation(&event, &data) {
            Ok(_) => {}
            Err(TakeError::NoKey(_)) => parked.push((event, data)),
            Err(TakeError::NotReached(_)) => parked.push((event, data)),
            Err(TakeError::Refused(reason)) => round.refused.push(reason),
        }
    }
}

impl<L: Link> Courier<L> {
    pub fn new(trust: Trust, link: L) -> Self {
        Self {
            trust,
            link,
            state: CourierState::default(),
        }
    }

    /// How many operations this courier holds back for want of the key of their epoch.
    pub fn parked(&self) -> usize {
        self.state.parked()
    }

    /// The machine of this host is a hub's: it is told who ever wrote, not who writes now.
    pub fn for_a_hub(mut self) -> Self {
        self.state.hub = true;
        self
    }

    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// How many records of the relay this courier has read: what a host keeps, to go on from there.
    pub fn cursor(&self) -> u64 {
        self.state.cursor
    }

    pub fn set_cursor(&mut self, cursor: u64) {
        self.state.cursor = cursor;
    }

    /// One round: the logs level, the relay given what it lacks, the host given what it lacks.
    pub fn sync(&mut self) -> Result<Round, CourierError> {
        self.round(RoundKind::Sync)
    }

    /// A round that reads everything the relay holds from the start and tells the machine every
    /// operation, the ones the host held already included: a machine that lost its state is told
    /// the list again.
    pub fn resync(&mut self) -> Result<Round, CourierError> {
        self.round(RoundKind::Resync)
    }

    /// A round that reads everything the relay holds from the start and tells the machine only what
    /// the host did not hold: for a host's MLS device that was not given a commit the first time (a
    /// feed does not give a record twice), and whose machine is not to be told the list again.
    pub fn reread(&mut self) -> Result<Round, CourierError> {
        self.round(RoundKind::Reread)
    }

    fn round(&mut self, kind: RoundKind) -> Result<Round, CourierError> {
        let state = std::mem::take(&mut self.state);
        let machine = RoundMachine::start(kind, state, self.trust.clone());
        self.run(machine)
    }

    /// Drive `machine` over the link to its end, with no deadline and no cancellation, and keep
    /// the state it leaves.
    fn run(&mut self, mut machine: RoundMachine) -> Result<Round, CourierError> {
        drive(&mut machine, &mut self.link, None, &Signal::new());
        let outcome = match machine.poll() {
            Step::Done(outcome) => outcome,
            // `drive` returns once the machine asks nothing more.
            Step::Request(_) => Err(CourierError::Unexpected(
                "the round asked again after it was driven to its end".to_string(),
            )),
        };
        self.state = machine.into_state();
        outcome
    }

    fn publish(&mut self, publication: Publication) -> Result<(), CourierError> {
        let state = std::mem::take(&mut self.state);
        let machine = RoundMachine::publish(&publication, state, self.trust.clone());
        self.run(machine).map(|_| ())
    }

    /// Leave a key package of this host's device for an administrator to add it with.
    pub fn publish_key_package(&mut self, package: &[u8]) -> Result<(), CourierError> {
        self.publish(Publication::KeyPackage(package.to_vec()))
    }

    /// Give the relay the commit that begins `epoch` (the log names its hash).
    pub fn publish_commit(&mut self, epoch: u32, commit: &[u8]) -> Result<(), CourierError> {
        self.publish(Publication::Commit {
            epoch,
            commit: commit.to_vec(),
        })
    }

    /// Give the relay the Welcome for the device of `node` that the commit of `epoch` added.
    pub fn publish_welcome(
        &mut self,
        epoch: u32,
        node: u64,
        welcome: &[u8],
    ) -> Result<(), CourierError> {
        self.publish(Publication::Welcome {
            epoch,
            node,
            welcome: welcome.to_vec(),
        })
    }

    /// This host's person says what their copy held of everything up to `before` (the digest the
    /// machine computed), and the relay is given it. The same round gives the relay the operations
    /// the checkpoint names, so that a member who reads it can fetch them.
    pub fn publish_checkpoint(&mut self, before: i64, digest: u32) -> Result<(), CourierError> {
        self.publish(Publication::Checkpoint { before, digest })
    }

    /// The word the machine is told that an instant is settled, when the checkpoints this host
    /// holds make a settlement of it.
    pub fn floor_event(&self, before: i64, digest: u32) -> Option<(String, String)> {
        floor_event(&self.trust, before, digest)
    }
}

/// The word a machine is told that an instant is settled, when the checkpoints `trust` holds make a
/// settlement of it.
pub fn floor_event(trust: &Trust, before: i64, digest: u32) -> Option<(String, String)> {
    trust.settlement(before, digest)?;
    Some((
        message::FLOOR_EVENT.to_string(),
        // As the machine writes it: a 64-bit number as text, a 32-bit one as a number.
        format!("{{\"before\":\"{before}\",\"hash\":{digest}}}"),
    ))
}

enum Taken {
    /// Taken in, refused, parked or handed over: the reading goes on.
    Done,
    /// An operation the host did not hold before, taken in: the reading goes on.
    New,
    /// It names what the host does not hold yet (an entry of the log, a checkpoint's entry).
    Behind,
}

/// Take one record the relay gave in, as from anyone, and say whether the host was behind it. An
/// operation taken in is held by the trust and told to the machine when the round ends, by the
/// rule of causal delivery ([`Trust::deliver`]), not as it is read: what the relay gives in
/// another order, or withholds, is never told before what it follows.
fn take(
    trust: &Trust,
    parked: &mut Vec<(String, String)>,
    record: &Record,
    round: &mut Round,
) -> Taken {
    match record {
        Record::Entry(text) => match Entry::from_json(text) {
            Some(entry) => match trust.append(&entry) {
                Ok(_) => Taken::Done,
                Err(LogError::Gap { .. }) => Taken::Behind,
                Err(error) => {
                    round
                        .refused
                        .push(format!("an entry of the relay: {error:?}"));
                    Taken::Done
                }
            },
            None => {
                round
                    .refused
                    .push("an entry of the relay is not one".to_string());
                Taken::Done
            }
        },
        Record::Operation { event, data } => match trust.take_operation(event, data) {
            Ok(taken) => {
                if taken.new {
                    Taken::New
                } else {
                    Taken::Done
                }
            }
            Err(TakeError::NotReached(_)) => Taken::Behind,
            Err(TakeError::NoKey(_)) => {
                // The key may come (the group's commit reaches this host) or never (an epoch
                // before it joined): park the operation, retry it each round, and go on.
                // A range asked for again ([`RoundMachine::drive_fetch`]) may bring one parked
                // already, which is parked once.
                if parked.len() < MAX_PARKED && !parked.iter().any(|(e, d)| e == event && d == data)
                {
                    parked.push((event.clone(), data.clone()));
                }
                Taken::Done
            }
            Err(TakeError::Refused(reason)) => {
                round.refused.push(reason);
                Taken::Done
            }
        },
        Record::Checkpoint(text) => match Checkpoint::from_json(text) {
            Some(checkpoint) => match trust.take_checkpoint(&checkpoint) {
                Ok(_) => Taken::Done,
                Err(CheckpointError::LogNotHeld { .. }) => Taken::Behind,
                Err(error) => {
                    round
                        .refused
                        .push(format!("a checkpoint of the relay: {error:?}"));
                    Taken::Done
                }
            },
            None => {
                round
                    .refused
                    .push("a checkpoint of the relay is not one".to_string());
                Taken::Done
            }
        },
        // The group's messages are for the host's MLS device to take in, in the order the log
        // names the epochs; the courier hands them over as it read them.
        Record::KeyPackage { .. } | Record::Commit { .. } | Record::Welcome { .. } => {
            round.messages.push(record.clone());
            Taken::Done
        }
        Record::KeyGrant { node, data, .. } => {
            take_key_grant(trust, parked, *node, data, round);
            Taken::Done
        }
    }
}

/// Take in the key of an epoch a member gave the device of `node`, when it is this host's
/// person's: what was sealed under the epoch before the device was admitted opens with it, so the
/// operations held back for want of it are tried again at once. A key given to another device is
/// not this host's to open; one given to its own that does not open, or that is not from a member
/// or an administrator the log names, is a refusal ([`Trust::receive_grant`]).
fn take_key_grant(
    trust: &Trust,
    parked: &mut Vec<(String, String)>,
    node: u64,
    data: &str,
    round: &mut Round,
) {
    if trust.person().map(|(own, _)| own) != Some(node) {
        return;
    }
    let Some(grant) = encoding::decode(data).and_then(|bytes| String::from_utf8(bytes).ok()) else {
        round
            .refused
            .push("a key the relay holds for this device is not a text".to_string());
        return;
    };
    match trust.receive_grant(&grant) {
        Ok(_) => retry_parked(trust, parked, round),
        Err(error) => round
            .refused
            .push(format!("a key the relay holds for this device: {error:?}")),
    }
}

/// What became of what a machine sent, kept by a host that has no hub ([`keep`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kept {
    /// Signed and remembered: one change is one new operation of the host's chain, in its
    /// [`Trust`], and a courier carries it to the relay (docs/adr/0085, section 1).
    Taken,
    /// The host holds it already, as the operation it made or admitted before for the same change:
    /// nothing is signed, the operation keeps its number and its signature, and a courier gives the
    /// relay nothing it has. What a machine built again from its host's own log sends.
    Held,
    /// It is not an event that is signed: nothing is kept, and a relay carries nothing of it.
    NotSigned,
    /// The host would not sign it, and never will under the log it holds, and why: another content
    /// under a dot the device signed or that waits, another person's change the host holds no
    /// signature of, a change that names nothing or writes one register twice, a person the log
    /// does not let write (a reader, a device taken off), a frozen chain, a host with no person.
    Refused(String),
    /// The host may not sign it yet, and why: the device has not read the log that lets it write,
    /// the key of the epoch the log is in, or, after its state came back from a store, its own
    /// chain ([`Trust::await_own_chain`]). It is kept, with every send after it, in order; the
    /// round that reads what it waits for signs it and gives it the relay ([`Round::taken`]), or,
    /// when the log says the person never may, refuses it ([`Round::dropped`]).
    Waiting(String),
}

/// Sign and remember what a machine sent, as the host of a machine that has no hub does: the one
/// place a send becomes an operation of the host's chain ([`Trust::keep_sent`]), whether through
/// [`Keep`] behind a Mesh endpoint or through a host that is given the machine's sends itself. What
/// is signed does not depend on the target the machine addressed the event to: the event alone is
/// signed. A change the host holds already is signed once ([`Kept::Held`]), so a host may give it a
/// machine's sends again, as many times as the machine makes them; one the host may not sign yet is
/// kept until it may ([`Kept::Waiting`]).
pub fn keep(trust: &Trust, event: &str, data: &str) -> Kept {
    let kept = kept_of(trust.keep_sent(event, data));
    note_kept(trust, event, &kept);
    kept
}

/// Sign and remember a change its host keeps a journal of (`record_list.made`, docs/adr/0083,
/// section 3.1), as [`keep`] does a send: `change` is the change (`record_list.change`'s payload).
/// What is signed for it is a record the host writes to its journal ([`Trust::unrecorded`]) and
/// that a round gives no relay before the host says it recorded it ([`Trust::recorded`]); what
/// waits becomes a record when a round signs it.
pub fn keep_made(trust: &Trust, change: &str) -> Kept {
    let kept = kept_of(trust.keep_made(change));
    note_kept(trust, message::CHANGE_EVENT, &kept);
    kept
}

/// What became of a send, as a host is told it.
fn kept_of(sent: Sent) -> Kept {
    match sent {
        Sent::Signed => Kept::Taken,
        Sent::Held => Kept::Held,
        Sent::NotSigned => Kept::NotSigned,
        Sent::Waiting(reason) => Kept::Waiting(reason),
        Sent::Refused(reason) => Kept::Refused(reason),
    }
}

/// The word a record names what became of a send by.
fn kept_word(kept: &Kept) -> &'static str {
    match kept {
        Kept::Taken => "taken",
        Kept::Held => "held",
        Kept::NotSigned => "not_signed",
        Kept::Waiting(_) => "waiting",
        Kept::Refused(_) => "refused",
    }
}

/// Log what became of a send of the machine ([`keep`]), at `debug`, one record each: the event it
/// is, the decision, and why when it waits or is refused. The list is named by its digest
/// ([`list_digest`]), and nothing of what the send carries is written: a reason the trust gives names
/// an event, a node, a number of a chain, an entry of the log or an epoch, and never content.
fn note_kept(trust: &Trust, event: &str, kept: &Kept) {
    let list = list_digest(&trust.list());
    match kept {
        Kept::Waiting(reason) | Kept::Refused(reason) => log::debug!(
            list = list.as_str(),
            send = event,
            decision = kept_word(kept),
            reason = reason.as_str();
            "send_kept"
        ),
        Kept::Taken | Kept::Held | Kept::NotSigned => log::debug!(
            list = list.as_str(),
            send = event,
            decision = kept_word(kept);
            "send_kept"
        ),
    }
}

/// Log what a round's signing of the sends that waited did ([`Trust::sign_waiting`]): at `debug`,
/// one record for each send it decided (taken, held, or dropped with why) and one for what still
/// waits (how many, and why the first does); at `info`, one record of the counts. A signing that
/// found nothing waiting says nothing. The list is named by its digest, and no send's content is
/// written.
fn note_waited(trust: &Trust, waited: &Waited) {
    if waited.is_empty() && waited.waiting == 0 {
        return;
    }
    let list = list_digest(&trust.list());
    let decided = waited
        .taken
        .iter()
        .map(|(event, _)| (event, "taken"))
        .chain(waited.held.iter().map(|(event, _)| (event, "held")));
    for (event, decision) in decided {
        log::debug!(
            list = list.as_str(),
            send = event.as_str(),
            decision = decision;
            "send_waited"
        );
    }
    for (event, _, why) in &waited.dropped {
        log::debug!(
            list = list.as_str(),
            send = event.as_str(),
            decision = "dropped",
            reason = why.as_str();
            "send_waited"
        );
    }
    if let Some(why) = &waited.why_waiting {
        log::debug!(
            list = list.as_str(),
            decision = "waiting",
            count = waited.waiting,
            reason = why.as_str();
            "send_waited"
        );
    }
    log::info!(
        list = list.as_str(),
        taken = waited.taken.len(),
        held = waited.held.len(),
        dropped = waited.dropped.len(),
        waiting = waited.waiting;
        "waiting_signed"
    );
}

/// What stands behind the machine of a host that has no hub. The host signs and remembers what the
/// machine sends (the operation is in its [`Trust`] from then on, [`keep`]), and the machine's send
/// is not delivered to any peer: the courier carries it to the relay.
pub struct Keep {
    inner: Trust,
}

impl Keep {
    pub fn new(inner: Trust) -> Self {
        Self { inner }
    }
}

impl Sealer for Keep {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        match keep(&self.inner, event, data) {
            Kept::Taken => Err(format!(
                "{event}: kept for the relay, which carries what a member signed"
            )),
            Kept::Held => Err(format!(
                "{event}: held already, and signed once; the relay carries the operation as it was made"
            )),
            Kept::NotSigned => Err(format!(
                "{event}: this host has no hub, and a relay carries only what a member signed"
            )),
            Kept::Waiting(reason) => Err(format!(
                "{event}: kept until it may be signed, and the relay is given it then: {reason}"
            )),
            Kept::Refused(reason) => Err(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{encoding, CourierState, Publication};

    /// A list whose history began again at a relay is read from its start, and the proof of a
    /// conflict given it again; what the machine was told and what waits to be handed over is kept.
    #[test]
    fn a_state_that_begins_again_reads_from_the_start_and_keeps_what_waits() {
        let mut state = CourierState::default().for_a_hub();
        state.set_cursor(40);
        state.evidence_sent.insert("proof".to_string());
        state
            .untold
            .push(("record_list.change".to_string(), "{}".to_string()));
        state.roster_len = 3;
        state.begin_again();
        assert_eq!(state.cursor(), 0);
        assert!(state.evidence_sent.is_empty());
        assert_eq!(state.untold(), 1);
        assert_eq!(state.roster_len, 3);
        assert!(state.is_for_a_hub());

        // The instant a relay's history began is kept with the rest, and written only when known,
        // so that the text of a list that has none is the one it was.
        assert!(!state.to_text().contains("since"));
        state.since = Some(1_800_000_000_000);
        assert!(state.to_text().contains("\"since\":\"1800000000000\""));
        assert_eq!(CourierState::from_text(&state.to_text()), Some(state));
    }

    #[test]
    fn every_publication_reads_back_as_it_was_written() {
        for publication in [
            Publication::KeyPackage(vec![1, 2, 3]),
            Publication::Commit {
                epoch: 4,
                commit: vec![5, 6],
            },
            Publication::Welcome {
                epoch: 4,
                node: u64::MAX,
                welcome: vec![7],
            },
            Publication::KeyGrant {
                epoch: 3,
                node: u64::MAX,
                grant: "{\"epoch\":3}".to_string(),
            },
            Publication::Checkpoint {
                before: i64::MIN,
                digest: u32::MAX,
            },
        ] {
            let value = publication.to_value();
            assert_eq!(
                Publication::from_value(&value),
                Some(publication),
                "{value}"
            );
        }
        for value in [
            json!({"k": "commit", "p": 4}),
            json!({"k": "commit", "p": 4, "d": "not the URL alphabet!"}),
            json!({"k": "welcome", "p": 4, "n": 9, "d": "AA"}),
            json!({"k": "key_grant", "p": 4, "n": "9", "d": encoding::encode(&[0xff, 0xfe])}),
            json!({"k": "cp", "b": "1", "g": u64::MAX}),
            json!({"k": "nothing", "d": "AA"}),
        ] {
            assert_eq!(Publication::from_value(&value), None, "{value}");
        }
    }
}
