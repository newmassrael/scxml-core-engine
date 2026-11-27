// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The contract every session machine of a keyed pool is held to
//! (docs/adr/0112): a kit of functions over a [`Subject`], run through a
//! [`Pool`] on a [`MemoryStore`] as a server runs one, so that what is held is
//! the contract as the pool reads it, whoever implements the machine: the
//! adapter of a generated machine (`crate::served`), or one a host writes by
//! hand.
//!
//! * [`a_restored_session_runs_on_as_one_never_saved`]: a session saved at an
//!   orderly stop and restored by the next process answers the rest of its
//!   requests as a session that never left memory does, and is saved as it is.
//! * [`a_restore_runs_no_initial_content`]: what the machine sends as its
//!   initial configuration is entered is sent at its start, once, and not again
//!   at a restore.
//! * [`a_finished_session_leaves_no_checkpoint`]: a session is kept while it
//!   runs and removed once it reaches a top-level final.
//! * [`a_state_of_another_document_is_refused_and_left_as_it_is`]: a request
//!   for a key whose checkpoint the machine cannot restore is answered as one
//!   the pool could not serve, and the checkpoint is left for a build that reads
//!   it (docs/adr/0109, D11).
//! * [`a_state_an_earlier_version_saved_is_carried_into_this_one`]: a state an
//!   earlier version of the document saved is restored through the migration
//!   this version declares, and saved under this version's shape.
//!
//! Each function panics, naming what it found, when the subject breaks the
//! contract. A test target calls each from a `#[test]` of its own.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use sce_forge_runtime::codec::SceCursor;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_session_store::{MemoryStore, SessionStore, Write};

use crate::checkpoint::{self, Checkpoint};
use crate::endpoint::{Endpoint, Environment, Transport, TransportFailure};
use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::generated::rpc_status::RpcStatus;
use crate::peers::Machine;
use crate::pool::{Pool, PoolConfig, SessionFactory};
use crate::router::within;

/// The peer every request of the kit comes from: the subject's deployment
/// binds a peer of this name, and pairs each request the kit sends with its
/// reply event.
pub const CLIENT: &str = "client";

/// The wall-clock moment every run of the kit starts at.
pub const T0: u64 = 1_700_000_000_000;

/// How long a lease of the kit's pools lasts.
const LEASE_TTL_MS: u64 = 60_000;

/// How long a stop may take.
const STOP_WITHIN: Duration = Duration::from_secs(30);

/// A request the kit sends a session: the event and its JSON data, which
/// names the session's key in the field the subject's deployment keys by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub event: String,
    pub data: String,
}

/// The machine under test, as a deployment serves it.
pub trait Subject {
    /// What builds each session's machine.
    type Factory: SessionFactory<KitWire, KitClock>;

    /// The machine as deploy.yaml declares it: a peer named [`CLIENT`] whose
    /// binding the kit's transport carries, and `sessions:` keyed by a field of
    /// what the client sends.
    fn machine(&self) -> Machine;

    /// The factory of a process's sessions: called once for each pool the kit
    /// starts, as each process of a server builds its own.
    fn factory(&self) -> Self::Factory;

    /// Requests that move a session of `key` on from where it starts, each one
    /// the deployment pairs with a reply: at least two, so that a session can
    /// be saved between them.
    fn steps(&self, key: &str) -> Vec<Request>;

    /// Requests that end a session of `key` at a top-level final, in order;
    /// none for a machine that has no final.
    fn ending(&self, key: &str) -> Vec<Request>;

    /// The event the machine sends [`CLIENT`] as its initial configuration is
    /// entered, when it sends one.
    fn greeting(&self) -> Option<String>;

    /// A saved state of `key` the machine must refuse: one of another document.
    fn foreign(&self, key: &str) -> SavedState;

    /// A state an earlier version of the document saved for `key`, which the
    /// migration this version declares carries; `None` for a machine with no
    /// earlier version.
    fn earlier(&self, key: &str) -> Option<SavedState>;
}

/// The wall clock of one run of the kit, which every pool of it reads, and
/// envelope ids that never repeat across them.
#[derive(Clone)]
pub struct KitClock {
    now: Arc<AtomicU64>,
    ids: Arc<AtomicU64>,
}

impl KitClock {
    fn new() -> Self {
        Self {
            now: Arc::new(AtomicU64::new(T0)),
            ids: Arc::new(AtomicU64::new(0)),
        }
    }

    fn advance(&self, ms: u64) {
        self.now.fetch_add(ms, Ordering::SeqCst);
    }

    fn at(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }
}

impl Environment for KitClock {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.at()).unwrap_or(i64::MAX)
    }
    fn now_unix_ms(&mut self) -> u64 {
        self.at()
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        let n = self.ids.fetch_add(1, Ordering::SeqCst) + 1;
        let mut id = [0x5A; 16];
        id[8..].copy_from_slice(&n.to_be_bytes());
        id
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

/// What the kit's transport was handed, by peer, in order, shared by its clones.
type Handed = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// The kit's transport: it keeps what it was handed, and its clones share
/// what they keep.
#[derive(Clone, Default)]
pub struct KitWire(Handed);

impl Transport for KitWire {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((peer.to_string(), bytes.to_vec()));
        Ok(())
    }
}

/// An envelope the kit's transport was handed, as much of it as the kit reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub peer: String,
    pub event: String,
    pub pattern: PatternKind,
    pub status: Option<RpcStatus>,
    /// The request a reply answers, by its wire invoke id.
    pub wire_id: Option<[u8; 16]>,
    pub data: String,
}

impl KitWire {
    /// What the transport was handed since the last call, in order.
    fn take(&self) -> Vec<Seen> {
        core::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
            .into_iter()
            .map(|(peer, bytes)| {
                let envelope =
                    Envelope::decode(&mut SceCursor::new(&bytes)).unwrap_or_else(|error| {
                        panic!("the pool sent bytes that are no envelope: {error:?}")
                    });
                Seen {
                    peer,
                    event: envelope.event_type.to_string(),
                    pattern: envelope.pattern,
                    status: envelope.rpc_status,
                    wire_id: envelope.invoke_id.map(Into::into),
                    data: String::from_utf8_lossy(envelope.data.into_inner()).into_owned(),
                }
            })
            .collect()
    }
}

/// One run of the kit: a store, the wall clock, and the requests sent so far.
struct Run<'s, S: Subject + ?Sized> {
    subject: &'s S,
    store: Arc<MemoryStore>,
    clock: KitClock,
    sent: u32,
}

/// One process serving the run's store.
struct Process<F: SessionFactory<KitWire, KitClock>> {
    pool: Pool<F, KitWire, KitClock>,
    wire: KitWire,
}

impl<'s, S: Subject + ?Sized> Run<'s, S> {
    fn new(subject: &'s S) -> Self {
        Self {
            subject,
            store: Arc::new(MemoryStore::new()),
            clock: KitClock::new(),
            sent: 0,
        }
    }

    /// A process serving the store, its link to the client up.
    fn process(&self, owner: &str) -> Process<S::Factory> {
        let wire = KitWire::default();
        let pool = Pool::new(
            self.subject.machine(),
            PoolConfig {
                owner: owner.to_string(),
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: LEASE_TTL_MS / 3,
            },
            self.subject.factory(),
            Arc::clone(&self.store) as Arc<dyn SessionStore>,
            wire.clone(),
            self.clock.clone(),
        )
        .unwrap_or_else(|error| {
            panic!("the subject's machine is not one a pool serves: {error:?}")
        });
        pool.peer_ready(CLIENT);
        Process { pool, wire }
    }

    /// Send `request` to `process` as the client's next request, run the
    /// session it names, and answer what the process sent since the last read.
    fn ask(&mut self, process: &Process<S::Factory>, request: &Request) -> Vec<Seen> {
        self.sent += 1;
        let mut envelope_id = [0x11; 16];
        envelope_id[12..].copy_from_slice(&self.sent.to_be_bytes());
        let mut wire_id = [0x22; 16];
        wire_id[12..].copy_from_slice(&self.sent.to_be_bytes());
        let bytes = Envelope {
            id: (&envelope_id).into(),
            source: within(CLIENT).expect("a peer name within its bound"),
            event_type: within(request.event.as_str())
                .unwrap_or_else(|_| panic!("the event {} is past its bound", request.event)),
            pattern: PatternKind::RpcRequest,
            datacontenttype: PayloadCodec::Json,
            data: within(request.data.as_bytes())
                .unwrap_or_else(|_| panic!("the data of {} is past its bound", request.event)),
            invoke_id: Some((&wire_id).into()),
            ..Envelope::new()
        }
        .encode_to_vec();
        let key = process.pool.receive(CLIENT, bytes).unwrap_or_else(|| {
            panic!(
                "{request:?} was placed in no session: {:?}",
                process.pool.take_errors()
            )
        });
        process.pool.run(&key);
        self.clock.advance(1);
        process.wire.take()
    }

    /// What `process` reported to its host since the last call, for a failure's message.
    fn errors(process: &Process<S::Factory>) -> Vec<String> {
        process
            .pool
            .take_errors()
            .into_iter()
            .map(|error| alloc::format!("{error:?}"))
            .collect()
    }

    /// Stop `process` in order: every session it holds is saved.
    fn stop(&self, process: Process<S::Factory>) -> Vec<Seen> {
        let report = process.pool.stop(Instant::now() + STOP_WITHIN);
        assert!(
            report.not_saved.is_empty(),
            "the stop left sessions unsaved: {:?}",
            report.not_saved
        );
        process.wire.take()
    }

    /// The checkpoint the store holds for `key`.
    fn checkpoint(&self, key: &str) -> Option<Checkpoint> {
        checkpoint::load(&*self.store, key)
            .unwrap_or_else(|error| panic!("the checkpoint of {key} does not read: {error}"))
            .map(|loaded| loaded.checkpoint)
    }

    /// The bytes the store holds for `key`.
    fn stored(&self, key: &str) -> Option<Vec<u8>> {
        self.store
            .get(key)
            .unwrap_or_else(|failure| panic!("the store does not answer: {failure}"))
            .map(|stored| stored.state)
    }

    /// Keep `state` as the clean checkpoint of `key`, as a process that has
    /// stopped leaves it: the machine's state and the section of an endpoint
    /// of the subject's machine that has exchanged nothing yet, which is what
    /// an earlier build's stop leaves for a session that had a state and no
    /// exchange in flight.
    fn plant(&self, key: &str, state: SavedState) {
        let earliest_due = state.earliest_due();
        let mut router =
            self.subject.machine().router().unwrap_or_else(|error| {
                panic!("the subject's machine builds no router: {error:?}")
            });
        router.enable_checkpoints();
        let section = Endpoint::new(router, KitWire::default(), self.clock.clone())
            .seal(1)
            .unwrap_or_else(|error| panic!("a fresh endpoint is sealed: {error:?}"));
        let text = Checkpoint {
            sequence: 1,
            clean: true,
            state,
            endpoints: alloc::vec![section],
            followed: alloc::collections::BTreeMap::new(),
        }
        .to_json();
        let now = self.clock.at();
        let lease = self
            .store
            .acquire(key, "kit-planter", LEASE_TTL_MS, now)
            .unwrap_or_else(|error| panic!("the store gives no lease on {key}: {error:?}"));
        self.store
            .write(
                key,
                &Write::put(text.as_bytes(), earliest_due, 0, lease.fence),
            )
            .unwrap_or_else(|error| panic!("the store keeps no checkpoint of {key}: {error:?}"));
        self.store
            .release(key, lease.fence)
            .unwrap_or_else(|error| panic!("the store does not release {key}: {error:?}"));
    }
}

/// The reply `seen` holds to the request it was sent after, which the kit
/// numbered `n`.
fn reply_to(seen: &[Seen], n: u32) -> Option<&Seen> {
    let mut wire_id = [0x22; 16];
    wire_id[12..].copy_from_slice(&n.to_be_bytes());
    seen.iter()
        .find(|seen| seen.pattern == PatternKind::RpcReply && seen.wire_id == Some(wire_id))
}

/// `state` with what differs between two sessions of one key that were each
/// started on their own: the session's id, which a start draws.
fn as_compared(mut state: SavedState) -> String {
    state.session_id = String::new();
    state.to_json()
}

/// The steps of the subject for `key`, refused when there are fewer than two.
fn steps_of<S: Subject + ?Sized>(subject: &S, key: &str) -> Vec<Request> {
    let steps = subject.steps(key);
    assert!(
        steps.len() >= 2,
        "the subject gives {} steps; the kit saves a session between two",
        steps.len()
    );
    steps
}

/// A session saved at an orderly stop between two of its requests, and
/// restored by the next process at the second, answers every request as a
/// session that never left memory answers it, and is then saved as that one
/// is.
pub fn a_restored_session_runs_on_as_one_never_saved<S: Subject + ?Sized>(subject: &S) {
    let key = "kit-restored";
    let steps = steps_of(subject, key);
    let half = steps.len() / 2;

    let mut kept = Run::new(subject);
    let process = kept.process("kept");
    let mut kept_replies = Vec::new();
    for step in &steps {
        let seen = kept.ask(&process, step);
        kept_replies
            .push(reply_to(&seen, kept.sent).map(|reply| (reply.status, reply.data.clone())));
    }
    kept.stop(process);
    let kept_state = kept
        .checkpoint(key)
        .expect("a session that ran is kept")
        .state;

    let mut moved = Run::new(subject);
    let first = moved.process("first");
    let mut moved_replies = Vec::new();
    for step in &steps[..half] {
        let seen = moved.ask(&first, step);
        moved_replies
            .push(reply_to(&seen, moved.sent).map(|reply| (reply.status, reply.data.clone())));
    }
    moved.stop(first);
    let saved = moved.checkpoint(key).expect("the stop saved the session");
    assert!(
        saved.clean,
        "an orderly stop saved {key} as a checkpoint that is not clean"
    );
    let second = moved.process("second");
    for step in &steps[half..] {
        let seen = moved.ask(&second, step);
        moved_replies
            .push(reply_to(&seen, moved.sent).map(|reply| (reply.status, reply.data.clone())));
    }
    moved.stop(second);
    let moved_state = moved
        .checkpoint(key)
        .expect("a restored session that ran is kept")
        .state;

    assert_eq!(
        moved_replies, kept_replies,
        "a session restored at step {half} answered otherwise than one that never left memory"
    );
    assert!(
        kept_replies
            .iter()
            .all(|reply| matches!(reply, Some((Some(RpcStatus::Ok), _)))),
        "a step was not answered Ok: {kept_replies:?}"
    );
    assert_eq!(
        as_compared(moved_state),
        as_compared(kept_state),
        "a session restored at step {half} was saved otherwise than one that never left memory"
    );
}

/// What the machine sends as its initial configuration is entered reaches the
/// client once, at the start, and not at a restore, which enters no state.
pub fn a_restore_runs_no_initial_content<S: Subject + ?Sized>(subject: &S) {
    let Some(greeting) = subject.greeting() else {
        return;
    };
    let key = "kit-greeted";
    let steps = steps_of(subject, key);
    let mut run = Run::new(subject);
    let first = run.process("first");
    let seen = run.ask(&first, &steps[0]);
    let greeted = seen.iter().filter(|seen| seen.event == greeting).count();
    assert_eq!(
        greeted, 1,
        "the start sent {greeting} {greeted} times: {seen:?}"
    );
    run.stop(first);
    let second = run.process("second");
    let seen = run.ask(&second, &steps[1]);
    assert!(
        seen.iter().all(|seen| seen.event != greeting),
        "the restore sent {greeting} again: {seen:?}"
    );
    assert!(
        reply_to(&seen, run.sent).is_some_and(|reply| reply.status == Some(RpcStatus::Ok)),
        "the restored session did not answer: {seen:?}"
    );
    run.stop(second);
}

/// A session is kept while it runs, and once it reaches a top-level final the
/// store holds nothing of it: the next request for its key starts a new one.
pub fn a_finished_session_leaves_no_checkpoint<S: Subject + ?Sized>(subject: &S) {
    let key = "kit-finished";
    let ending = subject.ending(key);
    if ending.is_empty() {
        return;
    }
    let steps = steps_of(subject, key);
    let mut run = Run::new(subject);
    let process = run.process("only");
    run.ask(&process, &steps[0]);
    assert!(
        run.checkpoint(key).is_some(),
        "a session that has not finished left no checkpoint"
    );
    for request in &ending {
        run.ask(&process, request);
    }
    assert!(
        run.checkpoint(key).is_none(),
        "a session that reached a top-level final is still kept"
    );
    run.stop(process);
    assert!(
        run.checkpoint(key).is_none(),
        "the stop kept a finished session"
    );
}

/// A request for a key whose checkpoint is a state of another document is
/// answered as one the pool could not serve, and the checkpoint is left as it
/// was, for a build that reads it (docs/adr/0109, D11).
pub fn a_state_of_another_document_is_refused_and_left_as_it_is<S: Subject + ?Sized>(subject: &S) {
    let key = "kit-foreign";
    let steps = steps_of(subject, key);
    let mut run = Run::new(subject);
    run.plant(key, subject.foreign(key));
    let planted = run.stored(key);
    let process = run.process("only");
    let seen = run.ask(&process, &steps[0]);
    let reply = reply_to(&seen, run.sent);
    assert!(
        reply.is_some_and(|reply| reply.status == Some(RpcStatus::InternalError)),
        "a request for a state of another document was answered {reply:?}"
    );
    run.stop(process);
    assert_eq!(
        run.stored(key),
        planted,
        "the checkpoint of another document was not left as it was"
    );
}

/// A state an earlier version of the document saved is restored through the
/// migration this version declares, answers as a session of this version, and
/// is saved under this version's shape (docs/adr/0109, D11).
pub fn a_state_an_earlier_version_saved_is_carried_into_this_one<S: Subject + ?Sized>(subject: &S) {
    let key = "kit-earlier";
    let Some(earlier) = subject.earlier(key) else {
        return;
    };
    let steps = steps_of(subject, key);
    let earlier_shape = earlier.shape.clone();

    let mut fresh = Run::new(subject);
    let process = fresh.process("fresh");
    fresh.ask(&process, &steps[0]);
    fresh.stop(process);
    let own_shape = fresh
        .checkpoint(key)
        .expect("a session that ran is kept")
        .state
        .shape;
    assert_ne!(
        own_shape, earlier_shape,
        "the earlier version's state has this version's shape: it tests no migration"
    );

    let mut run = Run::new(subject);
    run.plant(key, earlier);
    let process = run.process("only");
    let seen = run.ask(&process, &steps[0]);
    let reply = reply_to(&seen, run.sent);
    assert!(
        reply.is_some_and(|reply| reply.status == Some(RpcStatus::Ok)),
        "a request for a state the earlier version saved was answered {reply:?}; the pool \
         reported {:?}",
        Run::<S>::errors(&process)
    );
    run.stop(process);
    let saved = run
        .checkpoint(key)
        .expect("the migrated session is kept")
        .state;
    assert_eq!(
        saved.shape, own_shape,
        "a migrated session was saved under a shape not this version's"
    );
}
