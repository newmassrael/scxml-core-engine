// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The peer core of a device that holds a shared list (ADR 0079): one library that composes the
//! device's trust (`sce-rust-trust`), the relay's client side (`sce-rust-relay`), the device of the
//! list's group (`sce-rust-mls`) and the state the device keeps, and that does no I/O itself.
//!
//! The machine that holds the list (the standard shared record list, generated in the
//! application's language) talks to the core only in the record list's own events, and the relay is
//! reached through whatever carries the relay protocol's texts. Every input and output is a text in
//! the form the Mesh and the relay protocol already use:
//!
//! - the machine's outbound `record_list.write` and `record_list.supersede`, as the event's name
//!   and its JSON payload, are signed and remembered ([`Peer::outbound`]); the core says whether it
//!   took one, held it already (a machine built again from its host's log sends its writes again,
//!   and a write is signed once), found it is no event that is signed, or refused it and why;
//! - a round with the relay ([`Peer::start`]) is a sequence of request texts the core gives
//!   ([`Peer::poll_request`]) and response texts the host gives back ([`Peer::respond`]), or a
//!   failure of the transport ([`Peer::fail`]); its outcome ([`Peer::take_outcome`]) is the events
//!   for the machine in order, the refusals, whether the machine is to be built again, whether the
//!   round stalled, and what the group's device did with what was read;
//! - what the device keeps between runs is one text ([`Peer::save`], [`Peer::restore`]).
//!
//! The core reads no clock, opens no socket, starts no thread and waits on nothing: a host may hold
//! a response for minutes, lose its connection, or be stopped by its operating system in the middle
//! of a round, and the core goes on from what it was given. Nothing the core does depends on the time
//! today. When it comes to (a write's stamp judged against the receiving clock, ADR 0034 section 3;
//! the instant at which the core next wants to be called, for a relay's allowance or an operation
//! parked for want of a key, ADR 0079 section 3), the time enters as an argument of the calls that
//! need it, read by the host and given with each call.
//!
//! The secrets of the device (the person's key, the keys of the epochs, the group's state) are not in
//! what [`Peer::save`] writes: the host gives them again ([`Peer::set_identity`], [`Peer::trust`],
//! [`Peer::with_group`]) until the device's sealed state is specified (ADR 0079 section 6).

#![forbid(unsafe_code)]

pub use sce_rust_mls::host::{Member, Step as GroupStep};
pub use sce_rust_mls::Device;
pub use sce_rust_relay::{CourierError, Kept, Publication, Round, RoundKind};

use sce_rust_relay::client::{floor_event, keep};
use sce_rust_relay::{CourierState, LinkError, Response, RoundMachine, Step};
use sce_rust_trust::membership::{Entry, Hash};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

/// The version of what [`Peer::save`] writes.
pub const SAVE_VERSION: u64 = 1;

/// What a round, or a publication, came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The round: the events for the machine in order, what the relay was given, the refusals, the
    /// group's messages read, what the machine is to be built again without or with, and whether it
    /// stalled. A publication's round tells the machine nothing.
    pub round: Round,
    /// The second read from the start that the group's device owed, when it owed one (a commit the
    /// log names that it dropped at its limit): its events are for the machine too, after the
    /// round's.
    pub reread: Option<Round>,
    /// What the group's device did with what was read, in order.
    pub steps: Vec<GroupStep>,
}

impl Outcome {
    /// Every event for the machine, in order.
    pub fn events(&self) -> Vec<(String, String)> {
        self.rounds()
            .flat_map(|round| round.events.iter().cloned())
            .collect()
    }

    /// Every refusal, in order: each is a record that was not taken, and the rest went on.
    pub fn refused(&self) -> Vec<String> {
        self.rounds()
            .flat_map(|round| round.refused.iter().cloned())
            .collect()
    }

    /// Whether the host is to build its machine again and make a [`RoundKind::Resync`]: the core
    /// found operations the machine was told and is to be without, or was not told and is to be
    /// with ([`Round::evicted`], [`Round::restored`]).
    pub fn rebuild(&self) -> bool {
        self.rounds()
            .any(|round| !round.evicted.is_empty() || !round.restored.is_empty())
    }

    /// Whether the round stopped before the end of what the relay holds; a later round goes on
    /// from there.
    pub fn stalled(&self) -> bool {
        self.round.stalled
    }

    fn rounds(&self) -> impl Iterator<Item = &Round> {
        std::iter::once(&self.round).chain(self.reread.iter())
    }
}

/// Why the core did not do what its host asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerError {
    /// No request of an exchange waits for an answer: the host answered before it asked
    /// ([`Peer::poll_request`]), or answered twice.
    NothingAsked,
    /// The outcome of the last exchange was not taken ([`Peer::take_outcome`]). The next exchange
    /// is not begun until it is, so that nothing it hands over is lost.
    OutcomeNotTaken,
    /// The core holds no device of the list's group ([`Peer::with_group`]).
    NoGroupDevice,
    /// The group's device refused, and why.
    Group(String),
    /// A saved text is not what [`Peer::save`] writes, or does not hold for this list, and why.
    Unreadable(String),
}

/// The peer core of one device of one list.
///
/// One exchange with the relay runs at a time: a round ([`Self::start`]) or a publication
/// ([`Self::publish`]). Beginning one while another runs abandons the one that runs, which loses
/// nothing: what it took in and did not hand over is handed over by the next round, and the state
/// it leaves is the one the relay's client keeps after a completed step ([`CourierState`]).
pub struct Peer {
    trust: Trust,
    /// What the relay's client keeps between exchanges; while one runs, its machine holds it.
    state: CourierState,
    running: Option<Box<Running>>,
    outcome: Option<Result<Outcome, CourierError>>,
    group: Option<Member>,
}

/// An exchange with the relay under way.
struct Running {
    machine: RoundMachine,
    purpose: Purpose,
}

enum Purpose {
    /// A round the host began.
    Round,
    /// The second read from the start that the group's device owed after `first`, which ended.
    Follow { first: Round, steps: Vec<GroupStep> },
    /// A publication.
    Publish,
}

impl Running {
    /// The state the relay's client is left with when the exchange stops here. A round that ended
    /// before a second read hands over what it found by the next round, before what the second
    /// read took in.
    fn into_state(self) -> CourierState {
        let Running { machine, purpose } = self;
        let mut state = machine.into_state();
        if let Purpose::Follow { first, .. } = purpose {
            state.hold(first);
        }
        state
    }

    /// The state [`Self::into_state`] would leave, with the exchange left running.
    fn saved_state(&self) -> CourierState {
        let mut state = self.machine.abandoned_state();
        if let Purpose::Follow { first, .. } = &self.purpose {
            state.hold(first.clone());
        }
        state
    }
}

impl Peer {
    /// The core of the device of whoever makes the list `list`: its trust takes the genesis it is
    /// first given (the one its owner made) and every entry after it ([`Trust::open`]).
    pub fn open(list: &str) -> Self {
        Self::over(Trust::open(list))
    }

    /// The core of a device invited to the list `list` with the hash of its genesis entry, which the
    /// person was told in a channel they trust: it takes that genesis and no other
    /// ([`Trust::invited`]).
    pub fn invited(list: &str, genesis: Hash) -> Self {
        Self::over(Trust::invited(list, genesis))
    }

    fn over(trust: Trust) -> Self {
        Self {
            trust,
            state: CourierState::default(),
            running: None,
            outcome: None,
            group: None,
        }
    }

    /// The device's person: the node the log names them by, and their key.
    pub fn set_identity(&mut self, node: u64, identity: Identity) {
        self.trust.set_identity(node, identity);
    }

    /// The machine is a hub's: it is told who ever wrote, not who writes now. Set before the first
    /// round.
    pub fn for_a_hub(mut self) -> Self {
        self.state = std::mem::take(&mut self.state).for_a_hub();
        self
    }

    /// The core holds `device`, the device of `node` in the list's MLS group: what a round reads of
    /// the group it takes in, in the order the log names the epochs, and it gives the trust the key
    /// of each epoch it reaches.
    pub fn with_group(mut self, device: Device, node: u64) -> Self {
        self.group = Some(Member::new(device, self.trust.clone(), node));
        self
    }

    /// The trust of the device: the membership log, the chains, the checkpoints and the keys of the
    /// epochs, for what the core does not do itself (an administrator's entries of the log).
    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// The device of the list's group, when the core holds one.
    pub fn group(&self) -> Option<&Member> {
        self.group.as_ref()
    }

    /// The device of the list's group, for an administrator's changes to it.
    pub fn group_mut(&mut self) -> Option<&mut Member> {
        self.group.as_mut()
    }

    /// What the machine sent to `#hub` or `#people`, as the event's name and its JSON payload: a
    /// `record_list.write` or a `record_list.supersede` is signed and remembered, and the next
    /// round carries it to the relay; what is not signed is kept nowhere; what the trust will not
    /// sign is refused with the reason. The one place this is done is the relay's client
    /// ([`keep`]), which a host with a Mesh endpoint reaches through `Keep`.
    ///
    /// **A replay writes nothing twice.** An application that keeps its own log of commands and
    /// builds its machine again from it gives the core the machine's sends again: a write or a note
    /// the device signed already is [`Kept::Held`], the operation it holds with its number and its
    /// signature, and nothing new reaches the relay. A write under a stamp the device signed with
    /// another content is refused, since it would be the device signing two things for one write.
    /// After [`Self::restore`], a send the device holds no operation for is refused until a round
    /// has read the device's own chain back from the relay (see there).
    pub fn outbound(&mut self, event: &str, data: &str) -> Kept {
        keep(&self.trust, event, data)
    }

    /// Begin a round of `kind`. Its requests are taken by [`Self::poll_request`].
    pub fn start(&mut self, kind: RoundKind) -> Result<(), PeerError> {
        self.begin(Purpose::Round, |state, trust| {
            RoundMachine::start(kind, state, trust)
        })
    }

    /// Begin the exchange that gives the relay `publication`.
    pub fn publish(&mut self, publication: &Publication) -> Result<(), PeerError> {
        self.begin(Purpose::Publish, |state, trust| {
            RoundMachine::publish(publication, state, trust)
        })
    }

    /// Begin the exchange that leaves a key package of the group's device with the relay, for an
    /// administrator to add it with.
    pub fn publish_key_package(&mut self) -> Result<(), PeerError> {
        self.ready()?;
        let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
        let package = member
            .device_mut()
            .key_package()
            .map_err(|error| PeerError::Group(format!("{error:?}")))?;
        self.publish(&Publication::KeyPackage(package))
    }

    /// The next request to put to the relay, as the protocol's text; `None` when no exchange runs
    /// or the one that ran is over, and its outcome waits ([`Self::take_outcome`]). Asked again
    /// before it is answered, it is the same request.
    pub fn poll_request(&mut self) -> Option<String> {
        loop {
            let running = self.running.as_mut()?;
            match running.machine.poll() {
                Step::Request(request) => return Some(request.to_json()),
                Step::Done(result) => self.conclude(result),
            }
        }
    }

    /// The relay's response text to the request [`Self::poll_request`] gave. A text that is not a
    /// response of the protocol is taken as a failure of the transport.
    pub fn respond(&mut self, text: &str) -> Result<(), PeerError> {
        self.answer(Response::from_json(text).map_err(LinkError))
    }

    /// The transport did not carry the request or its response: the exchange ends with
    /// [`CourierError::Link`], and what it took in is handed over by the next round.
    pub fn fail(&mut self, reason: &str) -> Result<(), PeerError> {
        self.answer(Err(LinkError(reason.to_string())))
    }

    /// The outcome of the exchange that ended, once.
    pub fn take_outcome(&mut self) -> Option<Result<Outcome, CourierError>> {
        self.outcome.take()
    }

    /// The word the machine is told that an instant is settled, when the checkpoints the device
    /// holds make a settlement of it.
    pub fn floor_event(&self, before: i64, digest: u32) -> Option<(String, String)> {
        floor_event(&self.trust, before, digest)
    }

    /// What the device keeps, as one text: the membership log, what the trust holds
    /// ([`Trust::to_text`]) and the relay's client's state, which an exchange that runs, or an
    /// outcome not taken, leaves as it would if it stopped here. A host that saves in the middle of
    /// a round and restarts from the text goes on where it was; what was handed over after the save
    /// is handed over again, and the machine's merge takes an event twice as once.
    pub fn save(&self) -> String {
        let mut state = match &self.running {
            Some(running) => running.saved_state(),
            None => self.state.clone(),
        };
        if let Some(Ok(outcome)) = &self.outcome {
            if let Some(reread) = &outcome.reread {
                state.hold(reread.clone());
            }
            state.hold(outcome.round.clone());
        }
        json!({
            "version": SAVE_VERSION,
            "log": self.trust.entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
            "trust": self.trust.to_text(),
            "courier": state.to_text(),
        })
        .to_string()
    }

    /// Take back what [`Self::save`] wrote, into a core just made for the same list, after its
    /// person and the keys it needs were given again: each entry of the log is checked as it is
    /// appended, each operation and checkpoint the log verifies is kept ([`Trust::restore_from_text`],
    /// whose count is returned), and the relay's client goes on from its state. An exchange that
    /// ran is dropped.
    ///
    /// **A restored device signs nothing new until its own chain is read back.** The core cannot
    /// tell the last save from an older one (a backup, a copy of the store, a save the device ran
    /// past before it was stopped), and a device that published operations past what the text holds
    /// would sign its next one at a number it used already, which every host that holds both takes
    /// as the device signing two histories and freezes it for. So the trust waits
    /// ([`Trust::await_own_chain`]): [`Self::outbound`] refuses, with that reason, a send that would
    /// make a new operation, and still gives what the device holds as [`Kept::Held`]. The next
    /// round reads the relay's heads, asks for the device's own operations the relay holds past what
    /// the core does (`Operations`, the device's node), takes them in and tells them to the machine,
    /// and the wait ends once the core holds the chain as far as the relay does
    /// ([`Trust::confirm_own_chain`]); the device then signs at the number after its last. The rule
    /// is the core's and not its host's, so that no host can forget it, and it asks the relay
    /// nothing the protocol did not already serve. A host that rebuilds its machine from its own log
    /// after a restore makes that round first, then gives the core the machine's sends, then makes a
    /// [`RoundKind::Resync`] to tell the rebuilt machine the list.
    pub fn restore(&mut self, text: &str) -> Result<usize, PeerError> {
        let unreadable = |why: &str| PeerError::Unreadable(why.to_string());
        let value: Value = serde_json::from_str(text).map_err(|_| unreadable("it is not JSON"))?;
        if value.get("version").and_then(Value::as_u64) != Some(SAVE_VERSION) {
            return Err(unreadable("it is not a version this core reads"));
        }
        let state = value
            .get("courier")
            .and_then(Value::as_str)
            .and_then(CourierState::from_text)
            .ok_or_else(|| unreadable("it holds no state of the relay's client"))?;
        let held = value
            .get("trust")
            .and_then(Value::as_str)
            .ok_or_else(|| unreadable("it holds nothing of the trust"))?;
        let entries = value
            .get("log")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("it holds no log"))?
            .iter()
            .map(|entry| entry.as_str().and_then(Entry::from_json))
            .collect::<Option<Vec<Entry>>>()
            .ok_or_else(|| unreadable("an entry of its log is not one"))?;
        for entry in &entries {
            self.trust.append(entry).map_err(|error| {
                PeerError::Unreadable(format!("an entry of its log does not hold: {error:?}"))
            })?;
        }
        let kept = self.trust.restore_from_text(held);
        self.trust.await_own_chain();
        self.running = None;
        self.outcome = None;
        self.state = state;
        Ok(kept)
    }

    fn ready(&self) -> Result<(), PeerError> {
        if self.outcome.is_some() {
            Err(PeerError::OutcomeNotTaken)
        } else {
            Ok(())
        }
    }

    fn begin(
        &mut self,
        purpose: Purpose,
        make: impl FnOnce(CourierState, Trust) -> RoundMachine,
    ) -> Result<(), PeerError> {
        self.ready()?;
        if let Some(running) = self.running.take() {
            self.state = (*running).into_state();
        }
        let state = std::mem::take(&mut self.state);
        self.running = Some(Box::new(Running {
            machine: make(state, self.trust.clone()),
            purpose,
        }));
        Ok(())
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) -> Result<(), PeerError> {
        let running = self.running.as_mut().ok_or(PeerError::NothingAsked)?;
        if !running.machine.is_asking() {
            return Err(PeerError::NothingAsked);
        }
        running.machine.respond(answer);
        Ok(())
    }

    /// The exchange that ran ended with `result`. A round the group's device took in and that left
    /// it owing a second read from the start (`sce_rust_mls::host::Member::follow`) goes on with
    /// that read before its outcome is given.
    fn conclude(&mut self, result: Result<Round, CourierError>) {
        let Some(running) = self.running.take() else {
            return;
        };
        let Running { machine, purpose } = *running;
        self.state = machine.into_state();
        let outcome = match (purpose, result) {
            (Purpose::Round, Ok(round)) => {
                let steps = self
                    .group
                    .as_mut()
                    .map(|member| member.take_round(&round))
                    .unwrap_or_default();
                if self.group.as_ref().is_some_and(Member::owes_reread) {
                    let state = std::mem::take(&mut self.state);
                    let machine = RoundMachine::start(RoundKind::Reread, state, self.trust.clone());
                    self.running = Some(Box::new(Running {
                        machine,
                        purpose: Purpose::Follow {
                            first: round,
                            steps,
                        },
                    }));
                    return;
                }
                Ok(Outcome {
                    round,
                    reread: None,
                    steps,
                })
            }
            (Purpose::Follow { first, mut steps }, Ok(again)) => {
                if let Some(member) = self.group.as_mut() {
                    steps.extend(member.take_round(&again));
                }
                Ok(Outcome {
                    round: first,
                    reread: Some(again),
                    steps,
                })
            }
            // A failure of the second read is left to the next round, as the group's device leaves
            // it when its host has a link: the commit is still named and still lacking, and it is
            // asked for again.
            (Purpose::Follow { first, steps }, Err(_)) => Ok(Outcome {
                round: first,
                reread: None,
                steps,
            }),
            (Purpose::Publish, Ok(round)) => Ok(Outcome {
                round,
                reread: None,
                steps: Vec::new(),
            }),
            (Purpose::Round | Purpose::Publish, Err(error)) => Err(error),
        };
        self.outcome = Some(outcome);
    }
}
