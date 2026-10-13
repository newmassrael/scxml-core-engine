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
//! suppression (§mesh-10.5) answers.
//!
//! ⚠ Not modelled yet: a request's deadline and an ordering gap's timeout are
//! observed when the world's time reaches them by another thing happening; a
//! world that is otherwise quiet does not wake for them.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::{
    self, Endpoint, Environment, SharedEndpoint, Transport, TransportFailure,
};
use sce_rust_mesh::router::Router;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};

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
}

/// What one direction of a link does.
#[derive(Debug, Clone, Copy)]
struct Link {
    up: bool,
    latency_ms: i64,
    jitter_ms: i64,
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
            loss_pct: 0,
            duplicate_pct: 0,
        }
    }
}

struct NetState {
    flights: Vec<Flight>,
    links: BTreeMap<(String, String), Link>,
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

impl Transport for SimTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        let now = self.clock.now_ms();
        let mut net = self.net.lock();
        let link = net
            .links
            .get(&(self.from.clone(), peer.to_string()))
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
        let mut deliveries = 1;
        if net.rng.below(100) < link.loss_pct {
            net.trace.push(Traced {
                at_ms: now,
                from: self.from.clone(),
                to: peer.to_string(),
                fate: Fate::Lost,
                bytes: bytes.to_vec(),
            });
            return Ok(());
        }
        if net.rng.below(100) < link.duplicate_pct {
            deliveries = 2;
        }
        for copy in 0..deliveries {
            let jitter = i64::try_from(
                net.rng
                    .below(u64::try_from(link.jitter_ms + 1).unwrap_or(1)),
            )
            .unwrap_or(0);
            let due_ms = now + link.latency_ms + jitter;
            let seq = net.next_seq;
            net.next_seq += 1;
            net.flights.push(Flight {
                from: self.from.clone(),
                to: peer.to_string(),
                due_ms,
                seq,
                bytes: bytes.to_vec(),
            });
            net.trace.push(Traced {
                at_ms: now,
                from: self.from.clone(),
                to: peer.to_string(),
                fate: if copy == 0 {
                    Fate::Sent { due_ms }
                } else {
                    Fate::Duplicated { due_ms }
                },
                bytes: bytes.to_vec(),
            });
        }
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

/// One machine of a world: its engine, and the Mesh endpoint it is served by.
pub struct Actor<P: StatePolicy> {
    name: String,
    engine: Engine<P>,
    endpoint: ActorEndpoint,
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

    fn lock_endpoint(&self) -> std::sync::MutexGuard<'_, Endpoint<SimTransport, SimEnvironment>> {
        self.endpoint
            .lock()
            .expect("a Mesh endpoint is never left mid-update")
    }

    /// Let the machine take what it was given: drain its queues, hand its
    /// endpoint's news to it, and repeat until neither has more.
    fn pump(&mut self) {
        for _ in 0..PUMP_LIMIT {
            if P::NEEDS_EVENT_SCHEDULER {
                for _ in 0..SETTLE_TICKS {
                    self.engine.tick();
                }
            } else {
                self.engine.step();
            }
            let calls = self.lock_endpoint().take_calls();
            if calls.is_empty() {
                return;
            }
            endpoint::apply_to(&mut self.engine, calls);
        }
        panic!(
            "{} did not come to rest after {PUMP_LIMIT} rounds: it answers itself without end",
            self.name
        );
    }
}

/// What the world asks of an actor, whatever its machine.
trait Participant {
    fn name(&self) -> &str;
    fn receive(&mut self, from: &str, bytes: Vec<u8>);
    fn peer_ready(&mut self, peer: &str);
    fn peer_not_ready(&mut self, peer: &str);
    fn transmit_failed(&mut self, peer: &str, bytes: Vec<u8>);
    fn raise(&mut self, event: &str, data: &str);
    fn tick(&mut self);
    fn pump(&mut self);
    fn advance(&mut self, ms: u64);
    /// When this actor next needs the world's time to have reached, in the
    /// world's milliseconds: its engine's next scheduled thing, or its
    /// endpoint's next retry.
    fn next_due_ms(&self, now_ms: i64) -> Option<i64>;
    fn host_errors(&mut self) -> Vec<String>;
    fn as_any(&mut self) -> &mut dyn Any;
}

impl<P: StatePolicy + 'static> Participant for Actor<P> {
    fn name(&self) -> &str {
        &self.name
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

    fn raise(&mut self, event: &str, data: &str) {
        self.engine.raise_external_by_name(event, data);
    }

    fn tick(&mut self) {
        self.lock_endpoint().tick();
    }

    fn pump(&mut self) {
        Actor::pump(self);
    }

    fn advance(&mut self, ms: u64) {
        self.engine.advance_time_ms(ms);
    }

    fn next_due_ms(&self, now_ms: i64) -> Option<i64> {
        let engine = self
            .engine
            .time_until_next_scheduled_ms()
            .map(|wait| now_ms.saturating_add(i64::try_from(wait).unwrap_or(i64::MAX)));
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
        let endpoint: ActorEndpoint =
            Arc::new(Mutex::new(Endpoint::new(router, transport, environment)));
        let mut engine = Engine::new(policy);
        engine.set_clock(SceClock::Manual(
            u64::try_from(self.clock.now_ms()).unwrap_or(0),
        ));
        endpoint::register(&mut engine, &endpoint);
        engine.initialize();
        let mut actor = Actor {
            name: name.to_string(),
            engine,
            endpoint,
        };
        actor.pump();
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

    /// Spread the delay of every envelope between `a` and `b` over
    /// `0..=jitter_ms` more, drawn from the seed, in both directions.
    pub fn jitter(&mut self, a: &str, b: &str, jitter_ms: i64) {
        self.edit_both(a, b, |link| link.jitter_ms = jitter_ms);
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
        for flight in cut {
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
