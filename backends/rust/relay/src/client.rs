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
//! `Trust` from the moment they are made. And it reads what the relay holds after its cursor, takes
//! each record in as it would from anyone ([`Trust::take_operation`]), and returns the events the
//! machine is to be told.
//!
//! A host connected to several relays has one `Trust` and a courier for each: what one relay did
//! not pass on, another gives. A host that has no hub puts [`Keep`] behind its machine, so that what
//! the machine sends is signed and remembered by the host and is carried by the courier.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry, LogError};
use sce_rust_trust::{TakeError, Trust};

use crate::protocol::{Code, Record, Request, Response};
use crate::relay::Relay;
use crate::store::Store;

/// A way to put a request to one relay and read its answer.
pub trait Link {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError>;
}

/// The link did not carry the request or the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError(pub String);

/// A link to a relay in this process: the tests', and a host that runs its relay beside it.
pub struct DirectLink<S: Store> {
    relay: Arc<Mutex<Relay<S>>>,
    token: String,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
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
        }
    }
}

impl<S: Store> Link for DirectLink<S> {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
        let now = (self.clock)();
        let text = self
            .relay
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .handle(&self.token, &request.to_json(), now);
        Response::from_json(&text).map_err(LinkError)
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
    /// The events for the machine to be told, in order: the roster when the log moved, then each
    /// operation the host did not hold.
    pub events: Vec<(String, String)>,
    /// How many operations and checkpoints the relay was given that it lacked.
    pub pushed: usize,
    /// What the host or the relay refused and why: each is a record that is not taken, and the
    /// rest of the round goes on.
    pub refused: Vec<String>,
    /// The key packages, commits and Welcomes of the list's MLS group that were read, in the order
    /// the relay holds them: for the host's MLS device to take in.
    pub messages: Vec<Record>,
    /// Operations (device, number) the host held and an entry of the log then put past the last
    /// accepted of their device: the machine was told them and cannot take them back, so the host
    /// builds it again and calls [`Courier::resync`]. Empty when nothing was.
    pub evicted: Vec<(u64, u64)>,
    /// The round stopped before the end of what the relay holds: a record names an entry of the
    /// log the host does not have, or the device may not read yet. A later round goes on from there.
    pub stalled: bool,
}

/// A host's courier to one relay.
pub struct Courier<L: Link> {
    trust: Trust,
    link: L,
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
}

const FEED_BATCH: u32 = 200;

/// The most operations a courier parks for want of a key.
const MAX_PARKED: usize = 100_000;

impl<L: Link> Courier<L> {
    pub fn new(trust: Trust, link: L) -> Self {
        Self {
            trust,
            link,
            cursor: 0,
            roster_len: 0,
            hub: false,
            parked: Vec::new(),
            amendments_seen: 0,
        }
    }

    /// How many entries of the log amend the last operation accepted of a device taken off.
    fn amendments(&self) -> usize {
        self.trust
            .entries()
            .iter()
            .filter(|entry| matches!(entry.body, Body::Amend { .. }))
            .count()
    }

    /// How many operations this courier holds back for want of the key of their epoch.
    pub fn parked(&self) -> usize {
        self.parked.len()
    }

    /// Try again the operations that were held back for want of a key, and tell the machine those
    /// that open now.
    fn retry_parked(&mut self, round: &mut Round) {
        let waiting = std::mem::take(&mut self.parked);
        for (event, data) in waiting {
            match self.trust.take_operation(&event, &data) {
                Ok(taken) => round.events.push((event, taken.told)),
                Err(TakeError::NoKey(_)) => self.parked.push((event, data)),
                Err(TakeError::NotReached(_)) => self.parked.push((event, data)),
                Err(TakeError::Refused(reason)) => round.refused.push(reason),
            }
        }
    }

    /// The machine of this host is a hub's: it is told who ever wrote, not who writes now.
    pub fn for_a_hub(mut self) -> Self {
        self.hub = true;
        self
    }

    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// How many records of the relay this courier has read: what a host keeps, to go on from there.
    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: u64) {
        self.cursor = cursor;
    }

    fn ask(&mut self, request: &Request) -> Result<Response, CourierError> {
        Ok(self.link.call(request)?)
    }

    /// One round: the logs level, the relay given what it lacks, the host given what it lacks.
    pub fn sync(&mut self) -> Result<Round, CourierError> {
        self.round(false)
    }

    /// A round that reads everything the relay holds from the start and tells the machine every
    /// operation, the ones the host held already included: a machine that lost its state is told
    /// the list again.
    pub fn resync(&mut self) -> Result<Round, CourierError> {
        self.cursor = 0;
        self.round(true)
    }

    fn round(&mut self, redeliver: bool) -> Result<Round, CourierError> {
        let mut round = Round::default();
        self.exchange_log(&mut round)?;
        self.retry_parked(&mut round);
        if self.push(&mut round)? {
            self.pull(redeliver, &mut round)?;
            // An amendment raised the last operation accepted of a device taken off: what the host
            // refused of it before is true now, so it reads what the relay holds again from the
            // start (the operations it already holds are not told to the machine twice).
            let amends = self.amendments();
            if amends > self.amendments_seen {
                self.amendments_seen = amends;
                self.cursor = 0;
                self.pull(false, &mut round)?;
            }
        }
        round.evicted = self.trust.take_evicted();
        let len = self.trust.log_len();
        if len != self.roster_len || redeliver {
            self.roster_len = len;
            let roster = if self.hub {
                self.trust.hub_role_events()
            } else {
                self.trust.role_events()
            };
            round.events.splice(0..0, roster);
        }
        Ok(round)
    }

    /// Take the relay's entries in and give it the ones it lacks.
    fn exchange_log(&mut self, round: &mut Round) -> Result<(), CourierError> {
        let relay_len = loop {
            let from = self.trust.log_len();
            let Response::Entries { entries, len } = self.expect(Request::Entries { from })? else {
                return Err(CourierError::Unexpected(
                    "it did not answer with entries".to_string(),
                ));
            };
            for text in entries {
                match Entry::from_json(&text) {
                    Some(entry) => {
                        if let Err(error) = self.trust.append(&entry) {
                            round
                                .refused
                                .push(format!("an entry of the relay: {error:?}"));
                        }
                    }
                    None => round
                        .refused
                        .push("an entry of the relay is not one".to_string()),
                }
            }
            let now = self.trust.log_len();
            // Go on while the relay has more and the last call took some of it in.
            if now < len && now > from {
                continue;
            }
            break len;
        };
        let held = self.trust.entries();
        let start = usize::try_from(relay_len)
            .unwrap_or(held.len())
            .min(held.len());
        for entry in &held[start..] {
            match self.expect(Request::Append {
                entry: entry.to_json(),
            })? {
                Response::Appended { .. } => round.pushed += 1,
                other => return Err(CourierError::Unexpected(format!("{other:?}"))),
            }
        }
        Ok(())
    }

    /// Ask, and turn a refusal of the whole request into an error.
    fn expect(&mut self, request: Request) -> Result<Response, CourierError> {
        match self.ask(&request)? {
            Response::Refused { code, reason } => Err(CourierError::Relay { code, reason }),
            other => Ok(other),
        }
    }

    /// Give the relay what it lacks. `false` when the relay will not let this device read yet (it
    /// is not a member), in which case there is nothing to read either.
    fn push(&mut self, round: &mut Round) -> Result<bool, CourierError> {
        let heads = match self.ask(&Request::Heads)? {
            Response::Heads { heads, .. } => heads,
            Response::Refused {
                code: Code::Forbidden,
                reason,
            } => {
                round.stalled = true;
                round.refused.push(reason);
                return Ok(false);
            }
            Response::Refused { code, reason } => return Err(CourierError::Relay { code, reason }),
            other => return Err(CourierError::Unexpected(format!("{other:?}"))),
        };
        let theirs: BTreeMap<u64, u64> = heads.iter().map(|head| (head.node, head.seq)).collect();
        'devices: for head in self.trust.heads() {
            let held = theirs.get(&head.node).copied().unwrap_or(0);
            if head.seq <= held {
                continue;
            }
            for (event, data) in self.trust.operations(head.node, held + 1, head.seq) {
                match self.ask(&Request::Submit { event, data })? {
                    Response::Accepted { new, .. } => round.pushed += usize::from(new),
                    Response::Refused {
                        code: Code::Forbidden,
                        reason,
                    } => {
                        // A reader holds operations and may not write them anywhere.
                        round.refused.push(reason);
                        break 'devices;
                    }
                    Response::Refused { reason, .. } => {
                        round.refused.push(reason);
                        continue 'devices;
                    }
                    other => return Err(CourierError::Unexpected(format!("{other:?}"))),
                }
            }
        }
        for checkpoint in self.trust.checkpoints() {
            match self.ask(&Request::Checkpoint {
                checkpoint: checkpoint.to_json(),
            })? {
                Response::Accepted { new, .. } => round.pushed += usize::from(new),
                Response::Refused { reason, .. } => round.refused.push(reason),
                other => return Err(CourierError::Unexpected(format!("{other:?}"))),
            }
        }
        Ok(true)
    }

    /// Read what the relay holds after the cursor, and take each record in. A record that names
    /// an entry of the log the host lacks stops the round after one more try at the log, and the
    /// cursor stays before it.
    fn pull(&mut self, redeliver: bool, round: &mut Round) -> Result<(), CourierError> {
        loop {
            let Response::Feed {
                records,
                next,
                more,
            } = self.expect(Request::Feed {
                cursor: self.cursor,
                limit: FEED_BATCH,
            })?
            else {
                return Err(CourierError::Unexpected(
                    "it did not answer with a feed".to_string(),
                ));
            };
            let base = self.cursor;
            for (index, record) in records.iter().enumerate() {
                let mut retried = false;
                loop {
                    match self.take(record, redeliver, round) {
                        Taken::Done => break,
                        Taken::Behind if !retried => {
                            retried = true;
                            let before = self.trust.log_len();
                            self.exchange_log(round)?;
                            if self.trust.log_len() == before {
                                self.cursor = base + index as u64;
                                round.stalled = true;
                                return Ok(());
                            }
                        }
                        Taken::Behind => {
                            self.cursor = base + index as u64;
                            round.stalled = true;
                            return Ok(());
                        }
                    }
                }
            }
            self.cursor = next;
            if !more {
                return Ok(());
            }
        }
    }

    fn take(&mut self, record: &Record, redeliver: bool, round: &mut Round) -> Taken {
        match record {
            Record::Entry(text) => match Entry::from_json(text) {
                Some(entry) => match self.trust.append(&entry) {
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
            Record::Operation { event, data } => match self.trust.take_operation(event, data) {
                Ok(taken) => {
                    if taken.new || redeliver {
                        round.events.push((event.clone(), taken.told));
                    }
                    Taken::Done
                }
                Err(TakeError::NotReached(_)) => Taken::Behind,
                Err(TakeError::NoKey(_)) => {
                    // The key may come (the group's commit reaches this host) or never (an epoch
                    // before it joined): park the operation, retry it each round, and go on.
                    if self.parked.len() < MAX_PARKED {
                        self.parked.push((event.clone(), data.clone()));
                    }
                    Taken::Done
                }
                Err(TakeError::Refused(reason)) => {
                    round.refused.push(reason);
                    Taken::Done
                }
            },
            Record::Checkpoint(text) => match sce_rust_trust::Checkpoint::from_json(text) {
                Some(checkpoint) => match self.trust.take_checkpoint(&checkpoint) {
                    Ok(_) => Taken::Done,
                    Err(sce_rust_trust::CheckpointError::LogNotHeld { .. }) => Taken::Behind,
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
        }
    }

    /// Leave a key package of this host's device for an administrator to add it with.
    pub fn publish_key_package(&mut self, package: &[u8]) -> Result<(), CourierError> {
        self.publish(Request::PublishKeyPackage {
            data: encoding::encode(package),
        })
    }

    /// Give the relay the commit that begins `epoch` (the log names its hash).
    pub fn publish_commit(&mut self, epoch: u32, commit: &[u8]) -> Result<(), CourierError> {
        self.publish(Request::PublishCommit {
            epoch,
            data: encoding::encode(commit),
        })
    }

    /// Give the relay the Welcome for the device of `node` that the commit of `epoch` added.
    pub fn publish_welcome(
        &mut self,
        epoch: u32,
        node: u64,
        welcome: &[u8],
    ) -> Result<(), CourierError> {
        self.publish(Request::PublishWelcome {
            epoch,
            node,
            data: encoding::encode(welcome),
        })
    }

    fn publish(&mut self, request: Request) -> Result<(), CourierError> {
        match self.expect(request)? {
            Response::Accepted { .. } => Ok(()),
            other => Err(CourierError::Unexpected(format!("{other:?}"))),
        }
    }

    /// This host's person says what their copy held of everything up to `before` (the digest the
    /// machine computed), and the relay is given it. The same round gives the relay the operations
    /// the checkpoint names, so that a member who reads it can fetch them.
    pub fn publish_checkpoint(&mut self, before: i64, digest: u32) -> Result<(), CourierError> {
        let checkpoint = self.trust.checkpoint(before, digest).map_err(|error| {
            CourierError::Unexpected(format!("no checkpoint could be made: {error:?}"))
        })?;
        let mut round = Round::default();
        self.push(&mut round)?;
        match self.expect(Request::Checkpoint {
            checkpoint: checkpoint.to_json(),
        })? {
            Response::Accepted { .. } => Ok(()),
            other => Err(CourierError::Unexpected(format!("{other:?}"))),
        }
    }

    /// The word the machine is told that an instant is settled, when the checkpoints this host
    /// holds make a settlement of it.
    pub fn floor_event(&self, before: i64, digest: u32) -> Option<(String, String)> {
        self.trust.settlement(before, digest)?;
        Some((
            "todo.floor".to_string(),
            // As the machine writes it: a 64-bit number as text, a 32-bit one as a number.
            format!("{{\"before\":\"{before}\",\"hash\":{digest}}}"),
        ))
    }
}

enum Taken {
    Done,
    Behind,
}

/// What stands behind the machine of a host that has no hub. The host signs and remembers what the
/// machine sends (the operation is in its [`Trust`] from then on), and the machine's send is not
/// delivered to any peer: the courier carries it to the relay.
pub struct Keep {
    inner: Trust,
}

impl Keep {
    pub fn new(inner: Trust) -> Self {
        Self { inner }
    }
}

impl Sealer for Keep {
    fn seal(&mut self, target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        match self.inner.seal(target, event, data) {
            Ok(Some(_)) => Err(format!(
                "{event}: kept for the relay, which carries what a member signed"
            )),
            Ok(None) => Err(format!(
                "{event}: this host has no hub, and a relay carries only what a member signed"
            )),
            Err(reason) => Err(reason),
        }
    }
}
