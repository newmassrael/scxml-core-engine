// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The machine every session of the suite runs: a ledger, one per key.
//!
//! It is written by hand in the shape a generated machine has to the pool, as the pool's own tests
//! write theirs (`sce_rust_mesh::pool`): it reads what its endpoint tells it, answers requests
//! through its endpoint, keeps its delayed sends as a saved state keeps them (each with the
//! wall-clock moment it comes due), and is saved and restored through a [`SavedState`]. What it
//! does is what the scenarios ask of a server's document:
//!
//! * `service.request.take` (`{"key", "tag", "due_ms": [..]}`) records the tag, in the order taken,
//!   and schedules one delayed send `reminded` to the client for each due; the reply names the tag
//!   and its position. A request taken twice is recorded twice, which is how a scenario sees an
//!   effect applied twice.
//! * `service.request.join` (`{"key"}`) admits a member while the session holds fewer than
//!   [`LIMIT`]: the rule one key owns, which needs no lock because run to completion serialises
//!   a key's requests (docs/adr/0107, D3). It reads, waits and writes, so that two runs of one key
//!   at once would admit past the limit.
//! * `service.request.claim` (`{"key"}`) takes one of [`SEATS`] seats every session shares, held
//!   in the reference host's key-value store under [`SEATS_KEY`] and guarded by
//!   `sce-host-kv-put-if` (D6): read the count and its version, write the count above it if the
//!   version is still the one read, and read again when another session wrote between. Each start
//!   of either operation carries the invocation's identity (docs/SCE_HOST_FEATURES.md, section 1),
//!   so a repeat of one is answered with its first result.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use sce_rust_host_features::{
    HttpRequest, HttpResponse, Invocation, Outcome, Params, ReferenceHost, SystemClock,
    Transport as HttpTransport, TransportFailure as HttpFailure,
};
use sce_rust_mesh::checkpoint;
use sce_rust_mesh::endpoint::{
    EngineCall, Environment, Transport, MESH_ORIGIN_SCHEME, MESH_PROCESSOR_TYPE,
};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::peers::{HttpsAuthenticator, KeyFrom, Machine, Peer, PeerLink, Sessions};
use sce_rust_mesh::pool::{SessionEndpoint, SessionFactory, SessionMachine};
use sce_rust_mesh::router::PeerConfig;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::{SavedAct, SavedHostSend, SavedInvokes, SavedSend, SavedState};
use sce_rust_runtime::HostSendRequest;
use sce_rust_session_store::SessionStore;

/// The machine's name: the `source` of what its sessions send.
pub const MACHINE_NAME: &str = "ledger";

/// The one client every scenario's sessions answer.
pub const CLIENT: &str = "client";

/// The request that records a tag and schedules its delayed sends.
pub const TAKE: &str = "service.request.take";
/// Its reply.
pub const TAKEN: &str = "service.response.take";
/// The request that admits a member, up to [`LIMIT`].
pub const JOIN: &str = "service.request.join";
/// Its reply.
pub const JOINED: &str = "service.response.join";
/// The request that claims one of the [`SEATS`] every session shares.
pub const CLAIM: &str = "service.request.claim";
/// Its reply.
pub const CLAIMED: &str = "service.response.claim";
/// The delayed send a taken request schedules.
pub const REMINDED: &str = "reminded";

/// How many members one key admits: the rule the document states for one key.
pub const LIMIT: u32 = 3;

/// How many seats all sessions share: the rule the document states for a value they share.
pub const SEATS: u64 = 10;

/// The key-value store's key the shared count of seats taken is kept under.
pub const SEATS_KEY: &str = "seats";

/// deploy.yaml's `dedup.window_size` for the ledger: how many of one sender's envelopes a session
/// remembers, and so how many of its requests a duplicate is answered for with the first reply
/// (SCE_MESH.md section 10.5). Every scenario sends one session fewer requests than this.
pub const DEDUP_WINDOW: u32 = 128;

/// How the client's envelopes reach the machine: the binding's transport, as the rows of
/// SCE_MESH.md section 16.7 name it, and its link. The core reads neither to route; the host
/// that carries the bytes does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding {
    pub transport: &'static str,
    pub link: PeerLink,
}

/// The client reaches a server process over its standard streams: the server accepts, as a
/// binding the client dials does, and the parent carries the bytes.
pub const PIPE: Binding = Binding {
    transport: "custom_pipe",
    link: PeerLink::WssAccept { keepalive_ms: None },
};

/// The client reaches the server over the HTTPS binding (SCE_MESH.md section 22), proving its
/// name with a bearer token.
pub const HTTPS: Binding = Binding {
    transport: "https",
    link: PeerLink::HttpsAccept {
        listen: "127.0.0.1:0",
        authenticator: HttpsAuthenticator::Bearer,
        max_hold_ms: 2_000,
        max_body_bytes: 1_048_576,
        absence_ms: 120_000,
        mailbox: 1_024,
    },
};

/// The ledger as deploy.yaml would declare it: one peer, the client, reached over `binding`, and
/// a keyed pool whose key is the `key` field of what the client sends, a session staying in
/// memory for `idle_ms` after its last event.
pub fn machine(binding: Binding, idle_ms: u64) -> Machine {
    let peers: &'static [Peer] = Box::leak(Box::new([Peer {
        name: CLIENT,
        config: PeerConfig {
            transport: binding.transport,
            buffer: None,
            retry: None,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
            responders: &[CLIENT],
            deadline_ms: None,
            reply_events: &[TAKEN, JOINED, CLAIMED],
            causal: None,
        },
        link: binding.link,
    }]));
    Machine {
        name: MACHINE_NAME,
        dedup_window: DEDUP_WINDOW,
        gap_timeout_ms: 50,
        peers,
        sessions: Some(Sessions {
            key_from: KeyFrom::Field("key"),
            max_key_bytes: 32,
            idle_ms,
        }),
    }
}

/// The data of a `take` request for `key` with `tag`, scheduling a delayed send at each of `dues`.
pub fn take_data(key: &str, tag: &str, dues: &[u64]) -> String {
    let dues: Vec<String> = dues.iter().map(u64::to_string).collect();
    format!(
        r#"{{"key":{},"tag":{},"due_ms":[{}]}}"#,
        json::quote(key),
        json::quote(tag),
        dues.join(",")
    )
}

/// The data of a `join` or `claim` request for `key`.
pub fn key_data(key: &str) -> String {
    format!(r#"{{"key":{}}}"#, json::quote(key))
}

/// The tag of the `n`th delayed send a request tagged `tag` schedules, counted from 0.
pub fn reminder_tag(tag: &str, n: usize) -> String {
    format!("{tag}.{n}")
}

/// A host's network for the reference host of the key-value store: there is none, and a fetch
/// fails as a transport that cannot connect does. No session of the suite fetches.
pub struct NoNetwork;

impl HttpTransport for NoNetwork {
    fn exchange(&mut self, _request: &HttpRequest) -> Result<Option<HttpResponse>, HttpFailure> {
        Err(HttpFailure::Network)
    }
}

/// The reference host every session of a pool shares, whose key-value store holds the seats.
pub type SharedHost = Arc<Mutex<ReferenceHost<NoNetwork>>>;

/// A reference host with an empty store, read on the system's clock.
pub fn shared_host() -> SharedHost {
    Arc::new(Mutex::new(ReferenceHost::new(
        Box::new(SystemClock::new()),
        NoNetwork,
    )))
}

/// The record `outcome` completes with, or `None` for a refusal, a hold, or a failure (a record
/// whose `error` is not empty).
pub fn record_of(outcome: Option<Outcome>) -> Option<Value> {
    let Some(Outcome::Done(text)) = outcome else {
        return None;
    };
    let record = json::parse(&text).ok()?;
    match record.member("error") {
        Some(Value::Text(error)) if error.is_empty() => Some(record),
        _ => None,
    }
}

/// A meeting point: each session that reaches it waits, at most ten seconds, until `expected`
/// have. Sessions that claim a seat meet between their read and their write, so that every one of
/// them writes over a version another has already moved, and the race the conditional write
/// decides is certain to be run rather than left to the scheduler.
pub struct Rendezvous {
    expected: usize,
    arrived: Mutex<usize>,
    all_here: Condvar,
}

impl Rendezvous {
    pub fn new(expected: usize) -> Rendezvous {
        Rendezvous {
            expected,
            arrived: Mutex::new(0),
            all_here: Condvar::new(),
        }
    }

    fn meet(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut arrived = self.arrived.lock().unwrap_or_else(PoisonError::into_inner);
        *arrived += 1;
        self.all_here.notify_all();
        while *arrived < self.expected {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return;
            }
            arrived = self
                .all_here
                .wait_timeout(arrived, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// How the pool builds the ledger of a session: alone, or sharing a reference host's store with
/// the other sessions.
#[derive(Default, Clone)]
pub struct Ledgers {
    host: Option<SharedHost>,
    rendezvous: Option<Arc<Rendezvous>>,
    started: Arc<AtomicU64>,
    restored: Arc<AtomicU64>,
}

impl Ledgers {
    /// Ledgers whose seats are kept in `host`'s store, meeting at `rendezvous` before their first
    /// write.
    pub fn sharing(host: SharedHost, rendezvous: Arc<Rendezvous>) -> Ledgers {
        Ledgers {
            host: Some(host),
            rendezvous: Some(rendezvous),
            ..Ledgers::default()
        }
    }

    /// How many sessions were started from the initial datamodel.
    pub fn started(&self) -> u64 {
        self.started.load(Ordering::SeqCst)
    }

    /// How many sessions were restored from a checkpoint.
    pub fn restored(&self) -> u64 {
        self.restored.load(Ordering::SeqCst)
    }

    fn build<T: Transport, E: Environment>(
        &self,
        key: &str,
        held: Held,
        endpoint: &SessionEndpoint<T, E>,
    ) -> Ledger<T, E> {
        Ledger {
            key: key.to_string(),
            held,
            met: false,
            endpoint: Arc::clone(endpoint),
            host: self.host.clone(),
            rendezvous: self.rendezvous.clone(),
        }
    }
}

impl<T, E> SessionFactory<T, E> for Ledgers
where
    T: Transport + 'static,
    E: Environment + 'static,
{
    type Machine = Ledger<T, E>;

    fn start(
        &self,
        key: &str,
        endpoint: &SessionEndpoint<T, E>,
        _wall_now_ms: u64,
    ) -> Result<Ledger<T, E>, String> {
        self.started.fetch_add(1, Ordering::SeqCst);
        Ok(self.build(key, Held::default(), endpoint))
    }

    fn restore(
        &self,
        key: &str,
        state: &SavedState,
        endpoint: &SessionEndpoint<T, E>,
        _wall_now_ms: u64,
    ) -> Result<Ledger<T, E>, String> {
        let held = Held::from_state(state)?;
        self.restored.fetch_add(1, Ordering::SeqCst);
        Ok(self.build(key, held, endpoint))
    }
}

/// A delayed send the ledger waits to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    /// When it comes due, in milliseconds since the Unix epoch.
    pub due: u64,
    /// The tag it carries: the taken request's, and which of its sends it is.
    pub tag: String,
    /// The peer it goes to.
    pub peer: String,
}

/// What a ledger holds: its datamodel and its waiting sends, as its saved state keeps them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Held {
    /// The tags of the requests taken, in the order taken.
    pub taken: Vec<String>,
    /// The members admitted.
    pub members: u32,
    /// How many times the session started its `read` of the shared count.
    pub reads: u64,
    /// How many times the session started its conditional write of the shared count.
    pub claims: u64,
    /// The delayed sends waiting, earliest first.
    pub reminders: Vec<Reminder>,
}

impl Held {
    /// What `state`, a ledger's saved state, holds.
    pub fn from_state(state: &SavedState) -> Result<Held, String> {
        let number = |name: &str| -> Result<u64, String> {
            match state.variable(name) {
                Ok(Value::Number(digits)) => {
                    digits.parse().map_err(|_| format!("{name} is {digits}"))
                }
                other => Err(format!("{name} is {other:?}")),
            }
        };
        let taken = match state.variable("taken") {
            Ok(Value::Array(items)) => items
                .iter()
                .map(|item| match item {
                    Value::Text(tag) => Ok(tag.clone()),
                    other => Err(format!("a taken tag that is {other:?}")),
                })
                .collect::<Result<Vec<_>, _>>()?,
            other => return Err(format!("taken is {other:?}")),
        };
        let reminders = state
            .pending
            .iter()
            .map(|send| match &send.act {
                SavedAct::Host(host) => match host.target.strip_prefix('#') {
                    Some(peer) => Ok(Reminder {
                        due: send.due,
                        tag: host.send_id.clone(),
                        peer: peer.to_string(),
                    }),
                    None => Err(format!("a reminder to {}", host.target)),
                },
                other => Err(format!("a waiting send that is {other:?}")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Held {
            taken,
            members: u32::try_from(number("members")?).map_err(|e| e.to_string())?,
            reads: number("reads")?,
            claims: number("claims")?,
            reminders,
        })
    }

    /// The saved state of the ledger of `key` holding this, as a generated `save_at` writes a
    /// machine's: its variables, and each delayed send waiting, earliest first, so that the
    /// checkpoint's earliest due is the first reminder's.
    pub fn to_state(&self, key: &str) -> SavedState {
        let mut pending: Vec<SavedSend> = self
            .reminders
            .iter()
            .map(|reminder| SavedSend {
                due: reminder.due,
                act: SavedAct::Host(SavedHostSend {
                    processor_type: MESH_PROCESSOR_TYPE.to_string(),
                    event: REMINDED.to_string(),
                    target: format!("#{}", reminder.peer),
                    send_id: reminder.tag.clone(),
                    ..SavedHostSend::default()
                }),
            })
            .collect();
        pending.sort_by_key(|send| send.due);
        SavedState {
            shape: MACHINE_NAME.to_string(),
            configuration: vec!["open".to_string()],
            current: "open".to_string(),
            variables: vec![
                (
                    "taken".to_string(),
                    Value::Array(self.taken.iter().cloned().map(Value::Text).collect()),
                ),
                (
                    "members".to_string(),
                    Value::Number(self.members.to_string()),
                ),
                ("reads".to_string(), Value::Number(self.reads.to_string())),
                ("claims".to_string(), Value::Number(self.claims.to_string())),
            ],
            history: Vec::new(),
            pending,
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: session_id(key),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_starts: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
            features: None,
        }
    }
}

/// The session id of the ledger of `key`: what its saved state names it by, and what the
/// identity of each host operation it starts names it by.
pub fn session_id(key: &str) -> String {
    format!("ledger-{key}")
}

/// The ledger `store` holds under `key`, read from its last durable checkpoint, or `None` when the
/// store holds no session of that key.
pub fn read_held<S: SessionStore + ?Sized>(store: &S, key: &str) -> Result<Option<Held>, String> {
    let Some(loaded) = checkpoint::load(store, key).map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    Held::from_state(&loaded.checkpoint.state).map(Some)
}

/// One session's ledger.
pub struct Ledger<T, E> {
    key: String,
    held: Held,
    /// Whether the session has met the others at the rendezvous, which it does once.
    met: bool,
    endpoint: SessionEndpoint<T, E>,
    host: Option<SharedHost>,
    rendezvous: Option<Arc<Rendezvous>>,
}

impl<T: Transport, E: Environment> Ledger<T, E> {
    fn send(&self, peer: &str, event: &str, data: &str, invoke_id: &str) {
        let request = HostSendRequest {
            processor_type: MESH_PROCESSOR_TYPE.to_string(),
            event_name: event.to_string(),
            target: format!("#{peer}"),
            event_data: data.to_string(),
            invoke_id: invoke_id.to_string(),
            ..HostSendRequest::default()
        };
        self.endpoint
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(&request);
    }

    fn take(&mut self, peer: &str, invoke_id: &str, data: &Value) {
        let tag = match data.member("tag") {
            Some(Value::Text(tag)) => tag.clone(),
            _ => String::new(),
        };
        self.held.taken.push(tag.clone());
        if let Some(Value::Array(dues)) = data.member("due_ms") {
            for (n, due) in dues.iter().enumerate() {
                let Value::Number(digits) = due else {
                    continue;
                };
                let Ok(due) = digits.parse::<u64>() else {
                    continue;
                };
                self.held.reminders.push(Reminder {
                    due,
                    tag: reminder_tag(&tag, n),
                    peer: peer.to_string(),
                });
            }
        }
        let answer = format!(
            r#"{{"tag":{},"position":{}}}"#,
            json::quote(&tag),
            self.held.taken.len()
        );
        self.send(peer, TAKEN, &answer, invoke_id);
    }

    fn join(&mut self, peer: &str, invoke_id: &str) {
        // Read, wait, write: a second run of the session between the two would admit past the
        // limit.
        let members = self.held.members;
        std::thread::sleep(Duration::from_millis(2));
        let admitted = members < LIMIT;
        if admitted {
            self.held.members = members + 1;
        }
        let answer = format!(
            r#"{{"admitted":{admitted},"members":{}}}"#,
            self.held.members
        );
        self.send(peer, JOINED, &answer, invoke_id);
    }

    fn claim(&mut self, peer: &str, invoke_id: &str) {
        let (granted, seats) = match self.host.clone() {
            Some(host) => self.claim_seat(&host),
            None => (false, 0),
        };
        let answer = format!(r#"{{"granted":{granted},"seats":{seats}}}"#);
        self.send(peer, CLAIMED, &answer, invoke_id);
    }

    /// Take a seat: read the count of seats taken and its version, and write the count above it
    /// only if the version is still the one read; read again when another session wrote between.
    /// Answers whether a seat was taken, and the count this session last read or wrote.
    fn claim_seat(&mut self, host: &SharedHost) -> (bool, u64) {
        let session = session_id(&self.key);
        loop {
            self.held.reads += 1;
            let read = start(
                host,
                "sce-host-kv-get",
                &Invocation::new(&session, "read", self.held.reads),
                &[("key", SEATS_KEY)],
            );
            let Some(read) = read else {
                return (false, 0);
            };
            let taken = match read.member("value") {
                Some(Value::Text(value)) => value.parse::<u64>().unwrap_or(0),
                _ => 0,
            };
            let version = match read.member("version") {
                Some(Value::Text(version)) => version.clone(),
                _ => "0".to_string(),
            };
            if taken >= SEATS {
                return (false, taken);
            }
            if !self.met {
                self.met = true;
                if let Some(rendezvous) = &self.rendezvous {
                    rendezvous.meet();
                }
            }
            self.held.claims += 1;
            let above = (taken + 1).to_string();
            let written = start(
                host,
                "sce-host-kv-put-if",
                &Invocation::new(&session, "claim", self.held.claims),
                &[
                    ("key", SEATS_KEY),
                    ("value", above.as_str()),
                    ("expected_version", version.as_str()),
                ],
            );
            let Some(written) = written else {
                return (false, taken);
            };
            if matches!(written.member("written"), Some(Value::Bool(true))) {
                return (true, taken + 1);
            }
            // Another session wrote between the read and the write: read again.
        }
    }

    /// Make every delayed send due at `wall_now_ms`. True when one was made.
    fn deliver_due(&mut self, wall_now_ms: u64) -> bool {
        let (due, waiting): (Vec<Reminder>, Vec<Reminder>) =
            core::mem::take(&mut self.held.reminders)
                .into_iter()
                .partition(|reminder| reminder.due <= wall_now_ms);
        self.held.reminders = waiting;
        for reminder in &due {
            let data = format!(
                r#"{{"key":{},"tag":{},"due_ms":{},"sent_ms":{wall_now_ms}}}"#,
                json::quote(&self.key),
                json::quote(&reminder.tag),
                reminder.due
            );
            self.send(&reminder.peer, REMINDED, &data, "");
        }
        !due.is_empty()
    }
}

/// Start `operation` of `invocation` in the shared host with `params`, and answer its record.
fn start(
    host: &SharedHost,
    operation: &str,
    invocation: &Invocation,
    params: &[(&str, &str)],
) -> Option<Value> {
    let params: Params = params
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    let outcome = host
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .start(operation, invocation, &params);
    record_of(outcome)
}

impl<T, E> SessionMachine for Ledger<T, E>
where
    T: Transport + 'static,
    E: Environment + 'static,
{
    fn run(&mut self, calls: Vec<EngineCall>, wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            stepped = true;
            // The ledger starts no request of its own, so nothing ends one.
            let EngineCall::Raise(event) = call else {
                continue;
            };
            let peer = event
                .metadata
                .origin_address()
                .strip_prefix(MESH_ORIGIN_SCHEME)
                .unwrap_or_default()
                .to_string();
            let invoke_id = event.metadata.invoke_id.clone();
            let data = json::parse(&event.metadata.data).unwrap_or(Value::Null);
            match event.name.as_str() {
                TAKE => self.take(&peer, &invoke_id, &data),
                JOIN => self.join(&peer, &invoke_id),
                CLAIM => self.claim(&peer, &invoke_id),
                // A row the endpoint raised (`error.communication` when a link is lost), which
                // the ledger has no transition for.
                _ => {}
            }
        }
        stepped |= self.deliver_due(wall_now_ms);
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(self.held.to_state(&self.key))
    }

    fn finished(&self) -> bool {
        false
    }
}

/// The tags each key's ledger holds, as a map, for a scenario's check of the restored state.
pub fn taken_by_key<S: SessionStore + ?Sized>(
    store: &S,
    keys: impl IntoIterator<Item = String>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut taken = BTreeMap::new();
    for key in keys {
        let held = read_held(store, &key)?.unwrap_or_default();
        taken.insert(key, held.taken);
    }
    Ok(taken)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ledger_is_read_back_from_the_state_it_saves() {
        let held = Held {
            taken: vec!["a".to_string(), "b".to_string(), "a".to_string()],
            members: 2,
            reads: 5,
            claims: 4,
            reminders: vec![
                Reminder {
                    due: 20,
                    tag: "b.0".to_string(),
                    peer: CLIENT.to_string(),
                },
                Reminder {
                    due: 10,
                    tag: "a.0".to_string(),
                    peer: CLIENT.to_string(),
                },
            ],
        };
        let state = held.to_state("k1");
        assert_eq!(state.earliest_due(), Some(10));
        assert_eq!(state.session_id, "ledger-k1");
        let back = Held::from_state(&state).expect("a ledger's state");
        let mut sorted = held.clone();
        sorted.reminders.sort_by_key(|reminder| reminder.due);
        assert_eq!(back, sorted);
        let text = state.to_json();
        assert_eq!(
            Held::from_state(&SavedState::from_json(&text).expect("its text reads back")),
            Ok(sorted)
        );
    }

    #[test]
    fn a_request_names_its_key_and_its_dues() {
        let data = json::parse(&take_data("k1", "t", &[5, 9])).expect("JSON");
        assert_eq!(data.member("key"), Some(&Value::Text("k1".to_string())));
        assert_eq!(
            data.member("due_ms"),
            Some(&Value::Array(vec![
                Value::Number("5".to_string()),
                Value::Number("9".to_string())
            ]))
        );
        assert_eq!(
            take_data("k", "t", &[]),
            r#"{"key":"k","tag":"t","due_ms":[]}"#
        );
        assert_eq!(key_data("k"), r#"{"key":"k"}"#);
    }
}
