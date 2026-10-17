// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A world: several generated machines, each on its own Mesh endpoint, joined by
//! a network this crate owns.
//!
//! The scenarios every backend replays judge one machine at a time. They cannot
//! say whether machines that talk to each other still agree once the network
//! between them is slow, loses a link, delivers twice, or a machine is
//! restarted. A [`World`] runs them together in one process so that can be
//! asked, and so that the answer is the same every time it is asked:
//!
//! * **Time is virtual.** Every engine runs on `SceClock::Manual` and every
//!   endpoint reads one shared [`Clock`]. Nothing waits; the world jumps to the
//!   next thing that happens.
//! * **Every random choice is drawn from the seed** ([`Rng`]): delays, losses,
//!   duplicates, the ids and retry jitter of each endpoint. A failing run is
//!   replayed by its seed.
//! * **The seams are the ones the machines already have.** A `<send>` to a peer
//!   reaches the endpoint as the runtime's `HostSendRequest`; what the endpoint
//!   transmits goes through the host core's `Transport` and `Environment`, which
//!   it leaves open for a host to supply. This crate adds no seam and changes no
//!   machine: a machine in a world is the machine that ships.
//!
//! What a link does is the Mesh core's to answer, not this crate's. A link that is
//! down refuses a transmission as a transport does (retryable), so the core backs
//! off and sends again (§mesh-10.10); an envelope in flight when its link goes
//! down is failed back to its sender as a closed link fails one (§mesh-18.3).
//! A link may also lose or duplicate an envelope silently, as a transport that
//! does not order or deliver once does, which is what the core's duplicate
//! suppression (§mesh-10.5) answers. A link keeps the order its envelopes were
//! sent in, as a WebSocket does, however much their delays are spread; one that
//! does not ([`World::reorder`]) is the case the Mesh core orders for. A topic
//! ([`World::topic`]) is the one-to-many a machine sends to by one name.
//!
//! An actor need not be a Rust machine. [`World::add_external`] hosts one that runs in
//! another process, such as a generated Kotlin machine in a JVM, behind the same Mesh
//! endpoint: the world owns time, the network and the endpoint, and the process owns the
//! machine and is driven over a line protocol ([`external`]). Copies in different
//! languages then exchange real traffic through the one simulated network, and the
//! oracles that read that network judge them together.
//!
//! ⚠ The Mesh core takes the first sequence number it sees from a sender as the
//! start of that sender's stream. An envelope cut in flight before any other has
//! arrived, and sent again later, is below that start by then and is dropped for
//! good, so a run that orders its bindings begins by delivering one envelope each
//! way (the row tests do).
//!
//! A run can be read back one machine at a time ([`World::add_read`],
//! [`World::record_of`]): every event a machine was given from outside, by its host
//! or by a peer, in the order it took them, with what it held and what it sent its
//! peers after each. That is the form of the scenarios every backend replays
//! (docs/SCE_ACCEPTED_SUBSET.md §2.15), so a run of several machines becomes one
//! scenario for each of them, and the engines that cannot run in this world are
//! held to what the machines did in it.
//!
//! A machine that waits on a clock is recorded with the time that passed: the
//! world's time that moved between two events a machine took is a step of its own
//! (`advance_ms`), so a delayed send that came due in it is replayed by the same
//! wait. ⚠ A wait is one step however many times the world's time moved in it, as
//! a scenario cannot say more, so a delayed send whose delivery arms another that
//! falls due in the same wait is not replayed as the world ran it; the machines of
//! this world arm one delayed send from an event, and none from a delivery.
//!
//! ⚠ Not modelled yet: a request's deadline and an ordering gap's timeout are
//! observed when the world's time reaches them by another thing happening; a
//! world that is otherwise quiet does not wake for them.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::{
    self, Endpoint, EngineCall, Environment, SharedEndpoint, Transport, TransportFailure,
};
use sce_rust_mesh::router::Router;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};

pub mod external;
use external::{ExternalActor, ExternalMachine};

/// How many times a machine's queues and its endpoint are drained in a row
/// before the world calls the machine a loop. A machine that answers itself
/// without end is a defect to report, not a world to hang.
const PUMP_LIMIT: usize = 10_000;

/// The ticks a machine with a scheduler is given to settle: one round to take
/// what arrived and one to report a child's end, with margin. The same number the
/// scenarios replay with.
const SETTLE_TICKS: usize = 5;

/// The wall-clock time at the world's origin, in milliseconds since the Unix
/// epoch. A request's deadline is written on this clock (§mesh-9.5); the value
/// only has to be one every endpoint agrees on.
const UNIX_ORIGIN_MS: u64 = 1_700_000_000_000;

/// A deterministic generator (SplitMix64): the same seed draws the same
/// sequence on every platform.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`; `0` for an `n` of zero.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
}

/// The world's time, in milliseconds from its origin. Shared by every endpoint.
#[derive(Debug, Clone, Default)]
pub struct Clock(Arc<AtomicI64>);

impl Clock {
    pub fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }

    fn set(&self, ms: i64) {
        self.0.store(ms, Ordering::SeqCst);
    }
}

/// What happened to an envelope on the network, for a run to be read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// Handed to the network by `from`, to arrive at `due_ms`.
    Sent { due_ms: i64 },
    /// Arrived at `to`.
    Delivered,
    /// Never arrived: the link lost it silently.
    Lost,
    /// In flight when its link went down; failed back to its sender.
    Cut,
    /// Delivered a second time by the link.
    Duplicated { due_ms: i64 },
}

/// One line of a run's record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Traced {
    pub at_ms: i64,
    pub from: String,
    pub to: String,
    pub fate: Fate,
    pub bytes: Vec<u8>,
}

/// An envelope on its way.
struct Flight {
    from: String,
    to: String,
    due_ms: i64,
    seq: u64,
    bytes: Vec<u8>,
    /// Whether it is one copy of what was published to a topic. A copy cut in flight
    /// is lost, as a datagram is: the publisher sent to the topic, not to this
    /// receiver, and has nothing to send again to it.
    topic: bool,
}

/// What one direction of a link does.
#[derive(Debug, Clone, Copy)]
struct Link {
    up: bool,
    latency_ms: i64,
    jitter_ms: i64,
    /// Whether envelopes arrive in the order they were sent, however much the delay
    /// of each is spread: a WebSocket does, and so does any transport that is one
    /// ordered stream. A link that does not is one a deployment has to tell the Mesh
    /// core to order for it.
    fifo: bool,
    /// Percent of envelopes lost silently.
    loss_pct: u64,
    /// Percent of envelopes delivered twice.
    duplicate_pct: u64,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            up: true,
            latency_ms: 1,
            jitter_ms: 0,
            fifo: true,
            loss_pct: 0,
            duplicate_pct: 0,
        }
    }
}

struct NetState {
    flights: Vec<Flight>,
    links: BTreeMap<(String, String), Link>,
    /// When the last envelope put on each directed link is due, so the next one on a
    /// link that keeps order is due no earlier.
    last_due: BTreeMap<(String, String), i64>,
    /// The topics, by publisher and name: who each is delivered to.
    topics: BTreeMap<(String, String), Vec<String>>,
    rng: Rng,
    next_seq: u64,
    trace: Vec<Traced>,
}

/// The network every transport of a world sends into.
#[derive(Clone)]
struct Network(Arc<Mutex<NetState>>);

impl Network {
    fn new(seed: u64) -> Self {
        Self(Arc::new(Mutex::new(NetState {
            flights: Vec::new(),
            links: BTreeMap::new(),
            last_due: BTreeMap::new(),
            topics: BTreeMap::new(),
            rng: Rng::new(seed),
            next_seq: 0,
            trace: Vec::new(),
        })))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, NetState> {
        self.0.lock().expect("the network is never left mid-update")
    }
}

/// An actor's way onto the network: what its endpoint transmits is queued as a
/// flight, after the link's latency, unless the link is down.
pub struct SimTransport {
    from: String,
    net: Network,
    clock: Clock,
}

impl NetState {
    /// Put one envelope on the link from `from` to `to`, as the link treats it: lost,
    /// delivered once, or delivered twice, each after its latency and a jitter drawn
    /// from the seed. `topic` is whether it is a copy of what was published to a topic.
    fn carry(&mut self, now: i64, from: &str, to: &str, bytes: &[u8], topic: bool) {
        let direction = (from.to_string(), to.to_string());
        let lost = |net: &mut Self| {
            net.trace.push(Traced {
                at_ms: now,
                from: from.to_string(),
                to: to.to_string(),
                fate: Fate::Lost,
                bytes: bytes.to_vec(),
            });
        };
        // Only a topic gets here with a link that is missing or down: a transmission to
        // a peer is refused before it, so the link is there and up.
        let Some(link) = self.links.get(&direction).copied().filter(|link| link.up) else {
            lost(self);
            return;
        };
        if self.rng.below(100) < link.loss_pct {
            lost(self);
            return;
        }
        let deliveries = if self.rng.below(100) < link.duplicate_pct {
            2
        } else {
            1
        };
        for copy in 0..deliveries {
            let jitter = i64::try_from(
                self.rng
                    .below(u64::try_from(link.jitter_ms + 1).unwrap_or(1)),
            )
            .unwrap_or(0);
            let mut due_ms = now + link.latency_ms + jitter;
            if link.fifo {
                due_ms = due_ms.max(self.last_due.get(&direction).copied().unwrap_or(0));
            }
            self.last_due.insert(direction.clone(), due_ms);
            let seq = self.next_seq;
            self.next_seq += 1;
            self.flights.push(Flight {
                from: from.to_string(),
                to: to.to_string(),
                due_ms,
                seq,
                bytes: bytes.to_vec(),
                topic,
            });
            self.trace.push(Traced {
                at_ms: now,
                from: from.to_string(),
                to: to.to_string(),
                fate: if copy == 0 {
                    Fate::Sent { due_ms }
                } else {
                    Fate::Duplicated { due_ms }
                },
                bytes: bytes.to_vec(),
            });
        }
    }
}

impl Transport for SimTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        let now = self.clock.now_ms();
        let mut net = self.net.lock();
        let topic = (self.from.clone(), peer.to_string());
        if let Some(subscribers) = net.topics.get(&topic).cloned() {
            // A topic is not one link: each subscriber is reached by its own, which
            // loses, repeats and delays its copy as it does anything else it carries,
            // and a subscriber whose link is down is not reached at all. Nothing is
            // refused and nothing is sent again, since what is published to a topic
            // is not addressed to anyone in particular.
            for subscriber in subscribers {
                net.carry(now, &self.from, &subscriber, bytes, true);
            }
            return Ok(());
        }
        let link = net
            .links
            .get(&topic)
            .copied()
            .ok_or_else(|| TransportFailure {
                retryable: false,
                message: Some(format!("no link from {} to {peer}", self.from)),
            })?;
        if !link.up {
            return Err(TransportFailure {
                retryable: true,
                message: Some(format!("the link from {} to {peer} is down", self.from)),
            });
        }
        net.carry(now, &self.from, peer, bytes, false);
        Ok(())
    }
}

/// An actor's time and randomness: the world's clock, and ids and jitter drawn
/// from the seed and the actor's name.
pub struct SimEnvironment {
    clock: Clock,
    rng: Rng,
}

impl Environment for SimEnvironment {
    fn now_ms(&mut self) -> i64 {
        self.clock.now_ms()
    }

    fn now_unix_ms(&mut self) -> u64 {
        UNIX_ORIGIN_MS.saturating_add(u64::try_from(self.clock.now_ms()).unwrap_or(0))
    }

    /// A UUID v7 (§mesh-7.5): the millisecond clock, then drawn bits.
    fn envelope_id(&mut self) -> [u8; 16] {
        let ms = u64::try_from(self.clock.now_ms()).unwrap_or(0);
        let (a, b) = (self.rng.next_u64(), self.rng.next_u64());
        let mut id = [0u8; 16];
        id[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
        id[6..14].copy_from_slice(&a.to_be_bytes());
        id[14..].copy_from_slice(&b.to_be_bytes()[..2]);
        id[6] = 0x70 | (id[6] & 0x0F);
        id[8] = 0x80 | (id[8] & 0x3F);
        id
    }

    fn jitter_draw(&mut self) -> i64 {
        i64::try_from(self.rng.next_u64() >> 1).unwrap_or(0)
    }
}

/// A name hashed to a number, so two actors of one world draw different ids from
/// one seed (FNV-1a).
fn hash_name(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    })
}

type ActorEndpoint = SharedEndpoint<SimTransport, SimEnvironment>;

/// What a machine sent a peer: the target and the event as its document writes them,
/// and the data as the engine serialised it for the event (JSON, or empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub target: String,
    pub event: String,
    pub data: String,
}

/// One step of a machine's record: an event it was given from outside, or time that
/// moved, and what the machine was once the macrostep that started had run to
/// quiescence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The event's name and its data (JSON, or empty); `None` for a step that moves
    /// time, and for the first step, which is the machine as its start left it.
    pub event: Option<(String, String)>,
    /// How far the world's time moved, in milliseconds, since the step before, for a
    /// step that moves time; `0` for every other. Only a machine that runs on the
    /// event scheduler has such steps: one that does not cannot tell that time moved.
    /// A step of this kind is not an event and carries none, as a scenario's step
    /// states one or the other.
    pub advance_ms: u64,
    /// How long after the step the machine's scheduler has something next to deliver,
    /// in milliseconds; `None` when it has nothing waiting. A replay on a real clock
    /// reads a step correctly only when this is well clear of the time it waits.
    pub next_due_ms: Option<u64>,
    /// Whether the machine dropped the event because none of its events matches the
    /// name (§scxml-3.12.1).
    pub dropped: bool,
    /// What the machine sent its peers in this step, oldest first.
    pub sent: Vec<Sent>,
    /// The machine as its reader gave it back after the step: its saved state.
    pub saved: String,
}

/// How a recorded machine is read back after each step.
type Reader<P> = Box<dyn Fn(&Engine<P>) -> String>;

/// The record of one machine: how it is read, what it sent since it was last read,
/// the time that moved since its last step, and the steps so far.
struct Record<P: StatePolicy> {
    read: Reader<P>,
    sent: Arc<Mutex<Vec<Sent>>>,
    /// Milliseconds the world's time has moved since the last step was written. It
    /// is written as a step of its own when the machine is next given an event, or
    /// when the record is read, and not each time the world's time moves: with no
    /// event between them the moves are one wait, and the machine does the same on
    /// every engine whether it is told so at once or in parts.
    waited_ms: u64,
    steps: Vec<Taken>,
}

/// One machine of a world: its engine, and the Mesh endpoint it is served by.
pub struct Actor<P: StatePolicy> {
    name: String,
    engine: Engine<P>,
    endpoint: ActorEndpoint,
    /// What the host gave the machine that it has not taken yet, oldest first.
    given: Vec<(String, String)>,
    record: Option<Record<P>>,
}

impl<P: StatePolicy> Actor<P> {
    /// The machine's engine, to read it by.
    pub fn engine(&self) -> &Engine<P> {
        &self.engine
    }

    /// The machine's policy: its fields.
    pub fn policy(&self) -> &P {
        self.engine.policy()
    }

    /// Run the machine until it has nothing more to do at the present time. The
    /// generator says which call a machine needs: `step` drains the queues and
    /// nothing else, and a machine with a delayed send or a child session is driven
    /// by `tick`.
    fn rest(&mut self) {
        if P::NEEDS_EVENT_SCHEDULER {
            for _ in 0..SETTLE_TICKS {
                self.engine.tick();
            }
        } else {
            self.engine.step();
        }
    }

    /// Write the step that just ran into the machine's record, when it keeps one.
    fn note(&mut self, event: Option<(String, String)>) {
        self.write_step(event, 0);
    }

    /// Write the time that moved since the machine's last step as a step of its
    /// own, if any did. Called when the machine is about to be given an event, so
    /// the step reads the machine as the wait left it and the event is then judged
    /// on top of that.
    fn note_wait(&mut self) {
        let Some(record) = &mut self.record else {
            return;
        };
        let waited = std::mem::take(&mut record.waited_ms);
        if waited > 0 {
            self.write_step(None, waited);
        }
    }

    fn write_step(&mut self, event: Option<(String, String)>, advance_ms: u64) {
        let Some(record) = &mut self.record else {
            return;
        };
        let dropped = event
            .as_ref()
            .is_some_and(|(name, _)| P::resolve_event_by_name(name).is_none());
        let sent = std::mem::take(
            &mut *record
                .sent
                .lock()
                .expect("the list of what was sent is never left mid-update"),
        );
        let saved = (record.read)(&self.engine);
        let next_due_ms = self.engine.time_until_next_scheduled_ms();
        record.steps.push(Taken {
            event,
            advance_ms,
            next_due_ms,
            dropped,
            sent,
            saved,
        });
    }

    /// Let the machine take what it was given, one event at a time and each to
    /// quiescence before the next, as the steps of a scenario are replayed: first
    /// what its host gave it, then its endpoint's news, and again until neither has
    /// more.
    fn pump(&mut self) {
        for _ in 0..PUMP_LIMIT {
            self.rest();
            let given = std::mem::take(&mut self.given);
            let calls = self.lock_endpoint().take_calls();
            if given.is_empty() && calls.is_empty() {
                return;
            }
            self.note_wait();
            for (event, data) in given {
                self.engine.raise_external_by_name(&event, &data);
                self.rest();
                self.note(Some((event, data)));
            }
            for call in calls {
                let event = match &call {
                    EngineCall::Raise(event) => {
                        Some((event.name.clone(), event.metadata.data.to_string()))
                    }
                    EngineCall::EndInvoke { .. } => {
                        assert!(
                            self.record.is_none(),
                            "{} is recorded, and the end of a Mesh request is not an event a scenario can give a machine",
                            self.name
                        );
                        None
                    }
                };
                endpoint::apply_to(&mut self.engine, vec![call]);
                self.rest();
                if event.is_some() {
                    self.note(event);
                }
            }
        }
        panic!(
            "{} did not come to rest after {PUMP_LIMIT} rounds: it answers itself without end",
            self.name
        );
    }
}

/// What the world asks of an actor, whatever its machine and wherever it runs.
///
/// An actor is a machine and the Mesh endpoint that serves it. What concerns the
/// endpoint is the same for every machine, so it is answered here once; what an actor
/// brings is the machine's side.
trait Participant {
    fn name(&self) -> &str;
    /// The actor's Mesh endpoint.
    fn endpoint(&self) -> &ActorEndpoint;
    fn raise(&mut self, event: &str, data: &str);
    fn pump(&mut self);
    fn advance(&mut self, ms: u64);
    /// When the machine's own scheduler next has something to deliver, in the world's
    /// milliseconds.
    fn engine_due_ms(&self, now_ms: i64) -> Option<i64>;
    fn as_any(&mut self) -> &mut dyn Any;

    /// The machine's saved state, for an actor whose machine is another process's and
    /// cannot be reached through a typed engine; `None` for one that can.
    fn saved_state(&mut self) -> Option<String> {
        None
    }

    /// Stop the machine and start it again from `saved`, or from what it saved of
    /// itself when none is given, if it is another process's; whether it was.
    fn restart_external(&mut self, _saved: Option<&str>) -> bool {
        false
    }

    fn lock_endpoint(&self) -> std::sync::MutexGuard<'_, Endpoint<SimTransport, SimEnvironment>> {
        self.endpoint()
            .lock()
            .expect("a Mesh endpoint is never left mid-update")
    }

    fn receive(&mut self, from: &str, bytes: Vec<u8>) {
        self.lock_endpoint().receive(from, bytes);
    }

    fn peer_ready(&mut self, peer: &str) {
        self.lock_endpoint().peer_ready(peer);
    }

    fn peer_not_ready(&mut self, peer: &str) {
        self.lock_endpoint().peer_not_ready(peer);
    }

    fn transmit_failed(&mut self, peer: &str, bytes: Vec<u8>) {
        self.lock_endpoint().transmit_failed(
            peer,
            bytes,
            TransportFailure {
                retryable: true,
                message: Some(format!(
                    "the link to {peer} closed with the envelope in flight"
                )),
            },
        );
    }

    fn tick(&mut self) {
        self.lock_endpoint().tick();
    }

    /// When this actor next needs the world's time to have reached, in the
    /// world's milliseconds: its engine's next scheduled thing, or its
    /// endpoint's next retry.
    fn next_due_ms(&self, now_ms: i64) -> Option<i64> {
        let engine = self.engine_due_ms(now_ms);
        let retry = self.lock_endpoint().next_retry_due_ms();
        match (engine, retry) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    fn host_errors(&mut self) -> Vec<String> {
        self.lock_endpoint()
            .take_host_errors()
            .into_iter()
            .map(|error| format!("{error:?}"))
            .collect()
    }
}

impl<P: StatePolicy + 'static> Participant for Actor<P> {
    fn name(&self) -> &str {
        &self.name
    }

    fn endpoint(&self) -> &ActorEndpoint {
        &self.endpoint
    }

    fn raise(&mut self, event: &str, data: &str) {
        self.given.push((event.to_string(), data.to_string()));
    }

    fn pump(&mut self) {
        Actor::pump(self);
    }

    fn advance(&mut self, ms: u64) {
        self.engine.advance_time_ms(ms);
        // A machine that does not run on the scheduler cannot tell that time moved, so
        // its record does not hold it.
        if P::NEEDS_EVENT_SCHEDULER {
            if let Some(record) = &mut self.record {
                record.waited_ms = record.waited_ms.saturating_add(ms);
            }
        }
    }

    fn engine_due_ms(&self, now_ms: i64) -> Option<i64> {
        self.engine
            .time_until_next_scheduled_ms()
            .map(|wait| now_ms.saturating_add(i64::try_from(wait).unwrap_or(i64::MAX)))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

/// How a run of the world ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing was left to happen: no envelope in flight, nothing waiting on a
    /// timer. The world stands at the time of the last thing that did.
    Quiet,
    /// The time limit came first, with something still to happen.
    Limit,
}

/// Several machines and the network between them.
pub struct World {
    seed: u64,
    clock: Clock,
    net: Network,
    actors: Vec<Box<dyn Participant>>,
}

impl World {
    /// An empty world whose every random choice is drawn from `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            clock: Clock::default(),
            net: Network::new(seed),
            actors: Vec::new(),
        }
    }

    /// The world's time.
    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    /// Add the machine `policy` under `name`, served by an endpoint that holds
    /// `router`. The machine is started and left at rest.
    pub fn add<P: StatePolicy + 'static>(&mut self, name: &str, policy: P, router: Router) {
        self.add_actor(name, policy, router, None);
    }

    /// [`add`](Self::add), keeping a record of the machine
    /// ([`record_of`](Self::record_of)): `read` gives the machine back after each
    /// step, as the text it saves of itself. A generated machine saves through a
    /// trait of its own, which is why the reader is the caller's to write.
    ///
    /// A machine that runs on the event scheduler is recorded too: the time the
    /// world's clock moves between two events it takes is a step of its own
    /// ([`Taken::advance_ms`]), which is how a scenario states a wait.
    pub fn add_read<P: StatePolicy + 'static>(
        &mut self,
        name: &str,
        policy: P,
        router: Router,
        read: impl Fn(&Engine<P>) -> String + 'static,
    ) {
        self.add_actor(name, policy, router, Some(Box::new(read)));
    }

    /// The record of the machine `name`, which was added by
    /// [`add_read`](Self::add_read): every step so far, the start first. Time that
    /// moved since the machine last took an event is the last step.
    pub fn record_of<P: StatePolicy + 'static>(&mut self, name: &str) -> Vec<Taken> {
        let actor = self.actor::<P>(name);
        assert!(
            actor.record.is_some(),
            "{name} keeps no record: it was not added to be read"
        );
        actor.note_wait();
        actor
            .record
            .as_ref()
            .expect("a record was asserted above")
            .steps
            .clone()
    }

    /// Stop the machine `name` and start it again from what it saved of itself, the
    /// way a process that is killed and comes back does. `restore` is given the
    /// machine as it stands and the world's clock, and gives back the machine a
    /// restore of its saved state makes, measuring time by that clock: a generated
    /// machine restores through a trait of its own, which is why the call is the
    /// caller's to write (`|engine, clock| Engine::restore_with(TodoPeer::new(),
    /// &engine.save_at(0).unwrap(), clock, 0).unwrap()`).
    ///
    /// What is kept is the machine's connection to its peers — the endpoint, its
    /// links and what it was already told — and what is lost is whatever the machine
    /// had been given and not yet taken, as a process loses what it had not read. The
    /// Mesh router and the world's clock are set on the new machine as `add` sets
    /// them; no `<onentry>` runs and no `<data>` is evaluated, as the restore says.
    ///
    /// # Panics
    ///
    /// For a machine that keeps a record: what a restored machine did is not what a
    /// record of the events its first run was given would replay.
    pub fn restart<P: StatePolicy + 'static>(
        &mut self,
        name: &str,
        restore: impl FnOnce(&Engine<P>, SceClock) -> Engine<P>,
    ) {
        let now_ms = u64::try_from(self.clock.now_ms()).unwrap_or(0);
        let actor = self.actor::<P>(name);
        assert!(
            actor.record.is_none(),
            "{name} is recorded, and a restart is not an event a scenario can give a machine"
        );
        let mut engine = restore(&actor.engine, SceClock::Manual(now_ms));
        endpoint::register(&mut engine, &actor.endpoint);
        actor.engine = engine;
        actor.given.clear();
    }

    /// Add an actor whose machine runs in another process, `machine`, under `name`,
    /// served by an endpoint that holds `router` as any actor is. The machine is started
    /// afresh, at the world's time, and left at rest. A world is not given a handle that
    /// another world is using: see [`ExternalMachine`].
    ///
    /// Everything the world does to an actor it does to this one: links, partitions and
    /// the rest act on its endpoint, and what the machine sends goes to that endpoint
    /// as what a Rust machine sends does. What it cannot do is be recorded
    /// ([`add_read`](Self::add_read)): the record is what the Rust engine did.
    pub fn add_external(&mut self, name: &str, machine: &ExternalMachine, router: Router) {
        let endpoint = self.endpoint_for(name, router);
        let actor = ExternalActor::begin(name, machine, endpoint, self.clock.clone());
        self.actors.push(Box::new(actor));
    }

    /// Stop the machine of the external actor `name` and start it again from what it
    /// saved of itself, as [`restart`](Self::restart) does a Rust machine: the endpoint
    /// and its links are kept, and what the machine had been given and not yet taken is
    /// lost.
    ///
    /// # Panics
    ///
    /// When `name` is not an external actor.
    pub fn restart_external(&mut self, name: &str) {
        self.restart_external_from(name, None);
    }

    /// Stop the machine of the external actor `name` and start it again from `saved`,
    /// text it saved earlier ([`external_saved`](Self::external_saved)) and that is older
    /// than what it holds now: a copy that comes back from an old save has lost what it
    /// learned since, which the endpoint's retries do not give back.
    ///
    /// # Panics
    ///
    /// When `name` is not an external actor, or `saved` is not a state it can restore.
    pub fn restore_external(&mut self, name: &str, saved: &str) {
        self.restart_external_from(name, Some(saved));
    }

    fn restart_external_from(&mut self, name: &str, saved: Option<&str>) {
        let mut external = false;
        self.with_actor(name, |actor| external = actor.restart_external(saved));
        assert!(external, "{name} is not hosted by another process");
    }

    /// What the machine of the external actor `name` saves of itself, as the text it
    /// saves; `None` for an actor that is not external, whose machine is read through
    /// [`actor`](Self::actor).
    pub fn external_saved(&mut self, name: &str) -> Option<String> {
        let mut saved = None;
        self.with_actor(name, |actor| saved = actor.saved_state());
        saved
    }

    /// The endpoint for the actor `name`, on the world's network and clock, and its own
    /// randomness drawn from the seed and its name.
    fn endpoint_for(&self, name: &str, router: Router) -> ActorEndpoint {
        assert!(
            self.actors.iter().all(|actor| actor.name() != name),
            "the world already holds an actor named {name}"
        );
        let transport = SimTransport {
            from: name.to_string(),
            net: self.net.clone(),
            clock: self.clock.clone(),
        };
        let environment = SimEnvironment {
            clock: self.clock.clone(),
            rng: Rng::new(self.seed ^ hash_name(name)),
        };
        Arc::new(Mutex::new(Endpoint::new(router, transport, environment)))
    }

    fn add_actor<P: StatePolicy + 'static>(
        &mut self,
        name: &str,
        policy: P,
        router: Router,
        read: Option<Reader<P>>,
    ) {
        let endpoint = self.endpoint_for(name, router);
        let mut engine = Engine::new(policy);
        engine.set_clock(SceClock::Manual(
            u64::try_from(self.clock.now_ms()).unwrap_or(0),
        ));
        endpoint::register(&mut engine, &endpoint);
        let record = read.map(|read| {
            // The router the endpoint registered is replaced by one that writes the
            // send down before handing it on: the record holds what the machine asked
            // to be sent, whatever the network then does with it.
            let sent = Arc::new(Mutex::new(Vec::new()));
            let (log, sender) = (Arc::clone(&sent), Arc::clone(&endpoint));
            engine.register_mesh_router(move |request| {
                log.lock()
                    .expect("the list of what was sent is never left mid-update")
                    .push(Sent {
                        target: request.target.clone(),
                        event: request.event_name.clone(),
                        data: request.event_data.clone(),
                    });
                sender
                    .lock()
                    .expect("a Mesh endpoint is never left mid-update")
                    .send(&request);
                Vec::new()
            });
            Record {
                read,
                sent,
                waited_ms: 0,
                steps: Vec::new(),
            }
        });
        engine.initialize();
        let mut actor = Actor {
            name: name.to_string(),
            engine,
            endpoint,
            given: Vec::new(),
            record,
        };
        actor.pump();
        actor.note(None);
        self.actors.push(Box::new(actor));
    }

    /// Join `a` and `b` by a link up in both directions, `latency_ms` each way. Each
    /// side is told its peer is ready, as a transport tells its endpoint when a
    /// connection is made: an endpoint that keeps what it sends while a peer is not
    /// ready would otherwise keep it for good.
    pub fn link(&mut self, a: &str, b: &str, latency_ms: i64) {
        {
            let mut net = self.net.lock();
            for key in [(a, b), (b, a)] {
                net.links.insert(
                    (key.0.to_string(), key.1.to_string()),
                    Link {
                        latency_ms,
                        ..Link::default()
                    },
                );
            }
        }
        self.with_actor(a, |actor| actor.peer_ready(b));
        self.with_actor(b, |actor| actor.peer_ready(a));
    }

    /// Make `topic` a name `publisher` can send to, that every one of `subscribers`
    /// receives: a Zenoh key a machine puts to and others have declared a subscriber
    /// on, or a DDS topic, where the publisher does not list its readers
    /// (SCE_MESH.md, "SCE does not enumerate peers"). The publisher's router binds the
    /// topic as a peer, and each subscriber's binds the publisher, since that is
    /// whom the copy it receives is from.
    ///
    /// Each copy travels the link from the publisher to its subscriber
    /// ([`link`](Self::link) first), so it is delayed, lost and repeated as that link
    /// does, and a subscriber whose link is down receives nothing: a publication is
    /// not addressed, so it is not failed back and not sent again. A machine whose
    /// state is soft, which the next publication repairs, is what that suits.
    ///
    /// # Panics
    ///
    /// When a subscriber has no link from the publisher, or the topic is already
    /// declared.
    pub fn topic(&mut self, publisher: &str, topic: &str, subscribers: &[&str]) {
        {
            let mut net = self.net.lock();
            for subscriber in subscribers {
                assert!(
                    net.links
                        .contains_key(&(publisher.to_string(), (*subscriber).to_string())),
                    "no link from {publisher} to {subscriber}: link them before the topic"
                );
            }
            let key = (publisher.to_string(), topic.to_string());
            assert!(
                !net.topics.contains_key(&key),
                "{publisher} already has a topic named {topic}"
            );
            net.topics.insert(
                key,
                subscribers
                    .iter()
                    .map(|subscriber| (*subscriber).to_string())
                    .collect(),
            );
        }
        self.with_actor(publisher, |actor| actor.peer_ready(topic));
    }

    /// Spread the delay of every envelope between `a` and `b` over
    /// `0..=jitter_ms` more, drawn from the seed, in both directions. A link keeps
    /// the order its envelopes were sent in unless it is told otherwise
    /// ([`reorder`](Self::reorder)): a later envelope waits for the one before.
    pub fn jitter(&mut self, a: &str, b: &str, jitter_ms: i64) {
        self.edit_both(a, b, |link| link.jitter_ms = jitter_ms);
    }

    /// Let the link between `a` and `b` deliver envelopes out of the order they were
    /// sent in, as a transport that is more than one stream or datagrams do.
    pub fn reorder(&mut self, a: &str, b: &str) {
        self.edit_both(a, b, |link| link.fifo = false);
    }

    /// Make the link between `a` and `b` lose `loss_pct` percent of envelopes
    /// silently and deliver `duplicate_pct` percent twice, in both directions.
    pub fn degrade(&mut self, a: &str, b: &str, loss_pct: u64, duplicate_pct: u64) {
        self.edit_both(a, b, |link| {
            link.loss_pct = loss_pct;
            link.duplicate_pct = duplicate_pct;
        });
    }

    fn edit_both(&mut self, a: &str, b: &str, edit: impl Fn(&mut Link)) {
        let mut net = self.net.lock();
        for key in [(a, b), (b, a)] {
            let link = net
                .links
                .get_mut(&(key.0.to_string(), key.1.to_string()))
                .unwrap_or_else(|| panic!("no link from {} to {}", key.0, key.1));
            edit(link);
        }
    }

    /// Cut the link between `a` and `b`. An envelope in flight on it fails back
    /// to its sender as one on a closed link does, and each side is told its peer
    /// is no longer ready.
    pub fn partition(&mut self, a: &str, b: &str) {
        let now = self.clock.now_ms();
        let cut: Vec<Flight> = {
            let mut net = self.net.lock();
            for key in [(a, b), (b, a)] {
                if let Some(link) = net.links.get_mut(&(key.0.to_string(), key.1.to_string())) {
                    link.up = false;
                }
            }
            let (kept, cut): (Vec<_>, Vec<_>) = std::mem::take(&mut net.flights)
                .into_iter()
                .partition(|flight| {
                    !((flight.from == a && flight.to == b) || (flight.from == b && flight.to == a))
                });
            net.flights = kept;
            for flight in &cut {
                net.trace.push(Traced {
                    at_ms: now,
                    from: flight.from.clone(),
                    to: flight.to.clone(),
                    fate: Fate::Cut,
                    bytes: flight.bytes.clone(),
                });
            }
            cut
        };
        self.with_actor(a, |actor| actor.peer_not_ready(b));
        self.with_actor(b, |actor| actor.peer_not_ready(a));
        for flight in cut.into_iter().filter(|flight| !flight.topic) {
            let (from, to, bytes) = (flight.from, flight.to, flight.bytes);
            self.with_actor(&from, |actor| actor.transmit_failed(&to, bytes));
        }
    }

    /// Restore the link between `a` and `b`; each side is told its peer is ready.
    pub fn heal(&mut self, a: &str, b: &str) {
        {
            let mut net = self.net.lock();
            for key in [(a, b), (b, a)] {
                if let Some(link) = net.links.get_mut(&(key.0.to_string(), key.1.to_string())) {
                    link.up = true;
                }
            }
        }
        self.with_actor(a, |actor| actor.peer_ready(b));
        self.with_actor(b, |actor| actor.peer_ready(a));
    }

    /// Give the machine `actor` an external event, as its host would.
    pub fn raise(&mut self, actor: &str, event: &str, data: &str) {
        self.with_actor(actor, |actor| actor.raise(event, data));
    }

    /// The host of `actor` forbids the link `peer` to deliver the events `events` to the machine
    /// (an empty list lifts it): what arrives on that link under one of those names is dropped
    /// before the machine is told ([`Endpoint::forbid_events_from`]).
    pub fn forbid_events_from(&mut self, actor: &str, peer: &str, events: &[&str]) {
        self.with_actor(actor, |actor| {
            actor.lock_endpoint().forbid_events_from(peer, events)
        });
    }

    /// The actor `name`, as the machine it was added as.
    pub fn actor<P: StatePolicy + 'static>(&mut self, name: &str) -> &mut Actor<P> {
        self.actors
            .iter_mut()
            .find(|actor| actor.name() == name)
            .unwrap_or_else(|| panic!("the world holds no actor named {name}"))
            .as_any()
            .downcast_mut::<Actor<P>>()
            .unwrap_or_else(|| panic!("{name} is not the machine it was asked for"))
    }

    /// What went wrong that only a host can act on, from every endpoint, since
    /// it was last asked. A run that expects none asserts this empty.
    pub fn host_errors(&mut self) -> Vec<String> {
        self.actors
            .iter_mut()
            .flat_map(|actor| {
                let name = actor.name().to_string();
                actor
                    .host_errors()
                    .into_iter()
                    .map(move |error| format!("{name}: {error}"))
            })
            .collect()
    }

    /// Everything that happened on the network, in the order it did.
    pub fn trace(&self) -> Vec<Traced> {
        self.net.lock().trace.clone()
    }

    fn with_actor(&mut self, name: &str, body: impl FnOnce(&mut dyn Participant)) {
        let actor = self
            .actors
            .iter_mut()
            .find(|actor| actor.name() == name)
            .unwrap_or_else(|| panic!("the world holds no actor named {name}"));
        body(actor.as_mut());
    }

    /// Hand every envelope that is due to its receiver; how many.
    fn deliver_due(&mut self) -> usize {
        let now = self.clock.now_ms();
        let due: Vec<Flight> = {
            let mut net = self.net.lock();
            let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut net.flights)
                .into_iter()
                .partition(|flight| flight.due_ms <= now);
            net.flights = waiting;
            let mut due = due;
            due.sort_by_key(|flight| (flight.due_ms, flight.seq));
            for flight in &due {
                net.trace.push(Traced {
                    at_ms: now,
                    from: flight.from.clone(),
                    to: flight.to.clone(),
                    fate: Fate::Delivered,
                    bytes: flight.bytes.clone(),
                });
            }
            due
        };
        let delivered = due.len();
        for flight in due {
            let (from, to, bytes) = (flight.from, flight.to, flight.bytes);
            self.with_actor(&to, |actor| actor.receive(&from, bytes));
        }
        delivered
    }

    /// Run everything that can happen at the world's present time.
    fn settle(&mut self) {
        loop {
            for actor in &mut self.actors {
                actor.tick();
                actor.pump();
            }
            if self.deliver_due() == 0 {
                return;
            }
        }
    }

    /// The earliest time at which something is due.
    fn next_due_ms(&self) -> Option<i64> {
        let now = self.clock.now_ms();
        let flight = self.net.lock().flights.iter().map(|f| f.due_ms).min();
        self.actors
            .iter()
            .filter_map(|actor| actor.next_due_ms(now))
            .chain(flight)
            .min()
    }

    fn advance_to(&mut self, to_ms: i64) {
        let delta = u64::try_from(to_ms - self.clock.now_ms()).unwrap_or(0);
        self.clock.set(to_ms);
        for actor in &mut self.actors {
            actor.advance(delta);
        }
    }

    /// Let `ms` of virtual time pass, running everything that falls due in it. Unlike
    /// [`run_until_quiet`](Self::run_until_quiet) it does not stop early when nothing
    /// is left to happen: an edit made later by the clock needs the time to have gone.
    pub fn advance_ms(&mut self, ms: i64) {
        let deadline = self.clock.now_ms() + ms;
        loop {
            self.settle();
            match self.next_due_ms() {
                Some(at) if at <= deadline => self.advance_to(at.max(self.clock.now_ms())),
                _ => {
                    self.advance_to(deadline);
                    self.settle();
                    return;
                }
            }
        }
    }

    /// Run until nothing is left to happen, or `limit_ms` of virtual time has
    /// passed.
    pub fn run_until_quiet(&mut self, limit_ms: i64) -> Outcome {
        let deadline = self.clock.now_ms() + limit_ms;
        loop {
            self.settle();
            match self.next_due_ms() {
                None => return Outcome::Quiet,
                Some(at) if at > deadline => {
                    self.advance_to(deadline);
                    self.settle();
                    return Outcome::Limit;
                }
                Some(at) => self.advance_to(at.max(self.clock.now_ms())),
            }
        }
    }
}
