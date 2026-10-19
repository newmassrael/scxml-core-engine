// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding's own logic, and what it asks of a client (§mesh-20).
//!
//! A machine opens one client session and holds three things on it: a subscriber on its own inbox,
//! a liveliness token that says it is up, and a liveliness subscriber on everyone's. What arrives
//! becomes [`LinkEvent`]s the host feeds to its [`Endpoint`] with [`deliver`], as with the
//! WebSocket binding, so every decision stays in the core. The core's sends reach the network
//! through [`ZenohTransport`], which publishes the bytes on the inbox of the peer they name.
//!
//! When the session ends, the binding tells the core every peer it had ready is lost, once, and
//! from then on answers a send with a retryable failure and reports the end through
//! [`ZenohEvents::session_ended`]. It does not reconnect: the `reconnect` module opens a binding
//! again on a fresh session each time one ends.
//!
//! The session itself is not this module's. It is a [`ZenohClient`]: four synchronous operations
//! (subscribe to keys, subscribe to presence, declare presence, put) and the notice that its
//! session ended, and nothing else. Everything outside them is here and stays one copy whichever
//! client is plugged in: the key layout, the
//! name check, the order the session is set up in, the filter that keeps a machine's own presence
//! from reaching it, the mapping from what the client reports to a [`LinkEvent`], and the reading
//! of a refused put. That is what lets the client be chosen: upstream's `zenoh` crate is the
//! implementation in this repository (module `zenoh`, feature `zenoh`), another client implements
//! the trait in its own repository, and the tests in module `conformance` (feature `conformance`)
//! hold either to the same behaviour.
//!
//! This module needs no feature and no dependency beyond the standard library and the core.

use std::any::Any;
use std::collections::BTreeSet;
use std::sync::{mpsc, Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::endpoint::{Endpoint, Environment, Transport, TransportFailure};
use crate::link::{deliver, is_path_segment, LinkEvent};

/// The key space every machine of one Mesh shares, and the version of this binding (§mesh-20.1).
pub const KEY_PREFIX: &str = "sce-mesh/1";

/// The key a machine's inbox for envelopes from `sender` is published on: what `sender` writes to
/// reach `receiver`. The sender's name is in the key because the router core must be told who an
/// envelope came from, and the router's own ACL can pin who may publish under a given name.
pub fn inbox_key(receiver: &str, sender: &str) -> String {
    format!("{KEY_PREFIX}/to/{receiver}/{sender}")
}

/// What a machine subscribes to for everything addressed to it.
pub fn inbox_filter(receiver: &str) -> String {
    format!("{KEY_PREFIX}/to/{receiver}/*")
}

/// The key of the liveliness token that says `machine` is up.
pub fn presence_key(machine: &str) -> String {
    format!("{KEY_PREFIX}/up/{machine}")
}

/// What a machine subscribes to for the presence of every machine.
pub fn presence_filter() -> String {
    format!("{KEY_PREFIX}/up/*")
}

/// The last chunk of a key: the machine name a presence or inbox key ends in.
fn last_chunk(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or("")
}

/// Something a client holds open for the binding. Dropping it undeclares what it stands for: the
/// subscriber stops delivering, the presence token ends.
pub type Held = Box<dyn Any + Send>;

/// What a presence subscription reports about one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceKind {
    /// A token appeared on the key, or was already there when the subscription asked for history.
    Put,
    /// The token on the key ended: its session closed, or the router's lease on it ran out.
    Delete,
}

/// Why a client could not declare a subscriber or a token; the words are the client's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientError {
    pub message: String,
}

impl ClientError {
    /// An error that says `words`.
    pub fn new(words: impl core::fmt::Display) -> Self {
        ClientError {
            message: words.to_string(),
        }
    }
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ClientError {}

/// Why a client refused a put, and the client's reading of whether the same put could succeed
/// later; the binding hands both to the core as a [`TransportFailure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutError {
    /// Whether sending the same bytes again could succeed.
    pub retryable: bool,
    /// The client's own words.
    pub message: String,
}

impl PutError {
    /// A refusal the same put may get past later: a congested or reconnecting link.
    pub fn retryable(words: impl core::fmt::Display) -> Self {
        PutError {
            retryable: true,
            message: words.to_string(),
        }
    }

    /// A refusal no repetition will change.
    pub fn terminal(words: impl core::fmt::Display) -> Self {
        PutError {
            retryable: false,
            message: words.to_string(),
        }
    }
}

impl core::fmt::Display for PutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PutError {}

/// The four operations the binding needs of a Zenoh client session (SCE_MESH.md section 20), and
/// the notice that the session ended ([`ZenohClient::on_session_end`]), and nothing more. The
/// binding owns the keys, the order and the meaning of what arrives; a client only moves bytes and
/// presence, and says when it lost its session.
///
/// The operations are synchronous: each returns once the client has done it. The callbacks run on
/// a thread of the client's choosing, one call at a time per subscription, and must not be
/// assumed to run before the declaring call returns.
///
/// Every holder a client returns is dropped by the binding when its transport is dropped, the
/// client itself first and the holders after it in the order they were declared. A client must
/// make presence end when those are gone: that is the only way the other machines learn this one
/// has left, and the binding keeps no keepalive of its own.
pub trait ZenohClient: Send + 'static {
    /// Subscribe to the keys that match `filter` (a key expression whose last chunk may be `*`),
    /// calling `on_sample` with the full key and the payload bytes of each put that matches.
    fn subscribe<F>(&mut self, filter: &str, on_sample: F) -> Result<Held, ClientError>
    where
        F: FnMut(&str, &[u8]) + Send + 'static;

    /// Subscribe to the presence (liveliness) tokens that match `filter`, calling `on_change`
    /// with the key of a token and whether it appeared or ended. With `history`, the tokens that
    /// are already there are reported as [`PresenceKind::Put`] too, so a machine that opens after
    /// another learns it is up.
    fn subscribe_presence<F>(
        &mut self,
        filter: &str,
        history: bool,
        on_change: F,
    ) -> Result<Held, ClientError>
    where
        F: FnMut(&str, PresenceKind) + Send + 'static;

    /// Declare a presence token on `key`: it exists for the other machines' presence
    /// subscriptions until the returned holder is dropped or the client's session ends.
    fn declare_presence(&mut self, key: &str) -> Result<Held, ClientError>;

    /// Publish `payload` as one put on `key`. Under congestion it must block until the put is
    /// accepted and never drop it (§mesh-20.2): an envelope is the unit the core's delivery rules
    /// count, and one the network dropped silently is one the core would wait for.
    fn put(&mut self, key: &str, payload: &[u8]) -> Result<(), PutError>;

    /// Tell the binding when the session this client holds has ended, by calling `on_end` with the
    /// client's own words.
    ///
    /// `on_end` is called at most once, from a thread of the client's choosing, when the session
    /// has ended and the client will not carry on by itself: the router went away, the link failed,
    /// or the session was closed from outside. A client that reconnects inside itself and
    /// re-declares what it held must call it when it loses the session, and not when it has the new
    /// one: the router ended this machine's presence token when the session went, and a client that
    /// replays only what exists now can never learn who left in the meantime, so the binding
    /// resyncs from scratch (it tells the core every peer it had ready is lost, once, and the
    /// presence history of a fresh session announces who is still there).
    ///
    /// The binding calls it first, before any declaration, so a session that ends while the
    /// binding is still being set up is not missed. An `Err` fails the open.
    ///
    /// A client that cannot detect the loss says so in its own documentation, and is not held to
    /// the resync part of the conformance kit.
    fn on_session_end<F>(&mut self, on_end: F) -> Result<(), ClientError>
    where
        F: FnOnce(String) + Send + 'static;
}

/// Why a binding could not be opened.
#[derive(Debug)]
pub enum ZenohError {
    /// A machine name is not a key chunk as it stands (§mesh-20.1).
    NotAKeyChunk(String),
    /// The session, a subscriber or the token could not be opened; the words are the client's.
    Session(String),
}

impl core::fmt::Display for ZenohError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAKeyChunk(name) => write!(f, "machine name {name:?} is not a key chunk"),
            Self::Session(why) => write!(f, "zenoh session: {why}"),
        }
    }
}

impl std::error::Error for ZenohError {}

impl From<ClientError> for ZenohError {
    fn from(error: ClientError) -> Self {
        ZenohError::Session(error.message)
    }
}

/// Refuse a machine name that is not a key chunk as it stands. [`open_with`] asks it first thing;
/// a caller that has to build a client before it can call [`open_with`] asks it before that, so a
/// bad name never costs a session.
pub fn check_machine_name(machine: &str) -> Result<(), ZenohError> {
    if is_path_segment(machine) {
        Ok(())
    } else {
        Err(ZenohError::NotAKeyChunk(machine.to_string()))
    }
}

/// What every peer is told when the session they were ready on ends.
const SESSION_ENDED_WORDS: &str = "the session to the router ended";

/// What a send answers once the session has ended.
const SESSION_HAS_ENDED_WORDS: &str = "the session to the router has ended";

/// What the binding knows of the session its client holds: the peers it has told the core are
/// ready, and whether the client has said the session ended. The client's callbacks run on
/// threads of its own, so this is shared behind a lock, and every change to it is made together
/// with the event it stands for, so what the core is told and what is recorded here cannot part.
#[derive(Debug, Default)]
pub(crate) struct SessionState {
    /// The peers whose presence was reported and has not ended since: what the core believes is
    /// ready, and so what an end of the session must tell it is lost.
    ready: BTreeSet<String>,
    /// The client's words, once it has said the session ended.
    ended: Option<String>,
}

impl SessionState {
    /// What a presence change of `peer` tells the core, or nothing once the session has ended: a
    /// client that carries on by itself after the end must not make peers ready on a binding that
    /// has already told the core they are lost.
    fn presence(&mut self, peer: &str, kind: PresenceKind) -> Option<LinkEvent> {
        if self.ended.is_some() {
            return None;
        }
        Some(match kind {
            PresenceKind::Put => {
                self.ready.insert(peer.to_string());
                LinkEvent::Ready(peer.to_string())
            }
            PresenceKind::Delete => {
                self.ready.remove(peer);
                LinkEvent::Lost(peer.to_string(), "its presence token ended".to_string())
            }
        })
    }

    /// Record that the session ended, saying `words`, and give back the peers the core must now be
    /// told are lost: every one it was told is ready and was not told is gone. The first end is the
    /// only one: a client that says it twice changes nothing the second time.
    fn end(&mut self, words: &str) -> Option<BTreeSet<String>> {
        if self.ended.is_some() {
            return None;
        }
        self.ended = Some(words.to_string());
        Some(std::mem::take(&mut self.ready))
    }
}

/// A session's state, shared between the binding's callbacks, its transport and its events.
pub(crate) type SharedSession = Arc<Mutex<SessionState>>;

fn locked(session: &SharedSession) -> MutexGuard<'_, SessionState> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The core's side of the session: bytes published for the peer they name.
pub struct ZenohTransport<C> {
    client: C,
    machine: String,
    session: SharedSession,
    /// What the client holds open for this machine; dropping the transport closes it, which is
    /// what takes the presence token away and tells the other machines this one is gone.
    _held: Vec<Held>,
}

impl<C> ZenohTransport<C> {
    /// The state of the session this transport belongs to.
    pub(crate) fn session_state(&self) -> SharedSession {
        self.session.clone()
    }
}

impl<C: ZenohClient> Transport for ZenohTransport<C> {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if !is_path_segment(peer) {
            return Err(TransportFailure {
                retryable: false,
                message: Some(format!("peer name {peer:?} is not a key chunk")),
            });
        }
        // A session that has ended carries nothing, and the client is not asked: the core keeps
        // the envelope and sends it again, on this binding's successor if there is one.
        let ended = locked(&self.session).ended.is_some();
        if ended {
            return Err(TransportFailure {
                retryable: true,
                message: Some(SESSION_HAS_ENDED_WORDS.to_string()),
            });
        }
        // One envelope per put, and Block rather than drop under congestion (SCE_MESH.md section
        // 20.2): the client is required to do that, see [`ZenohClient::put`].
        self.client
            .put(&inbox_key(peer, &self.machine), bytes)
            .map_err(|e| TransportFailure {
                retryable: e.retryable,
                message: Some(e.message),
            })
    }
}

/// What the session tells the host: who came up, what arrived, who went.
pub struct ZenohEvents {
    events: mpsc::Receiver<LinkEvent>,
    session: SharedSession,
}

impl ZenohEvents {
    /// Events that read `events`, with the end of the session recorded in `session`.
    pub(crate) fn new(events: mpsc::Receiver<LinkEvent>, session: SharedSession) -> Self {
        ZenohEvents { events, session }
    }

    /// Events whose session never ends for good: a binding that opens its own successors reads
    /// one channel through all of them, and answers [`session_ended`](Self::session_ended) with
    /// `None` throughout.
    pub(crate) fn unending(events: mpsc::Receiver<LinkEvent>) -> Self {
        Self::new(events, SharedSession::default())
    }

    /// The client's words if the session this binding was opened on has ended, and `None` while
    /// it has not. It never blocks. Once it answers, the binding has told the core every peer it
    /// had ready is lost, its transport refuses sends as retryable, and nothing more will come
    /// from this session: the host opens a new binding. The events of a reconnecting binding
    /// always answer `None`, because it opens its own successors.
    pub fn session_ended(&self) -> Option<String> {
        locked(&self.session).ended.clone()
    }

    /// The next event if one has arrived.
    pub fn try_next(&self) -> Option<LinkEvent> {
        self.events.try_recv().ok()
    }

    /// The next event, waiting up to `wait` for it.
    pub fn next_within(&self, wait: Duration) -> Option<LinkEvent> {
        self.events.recv_timeout(wait).ok()
    }

    /// Hand every event that has arrived to `endpoint`, returning how many. Call it on the thread
    /// that steps the engine; it never blocks.
    pub fn pump<T: Transport, E: Environment>(&self, endpoint: &mut Endpoint<T, E>) -> usize {
        let mut delivered = 0;
        while let Some(event) = self.try_next() {
            deliver(endpoint, event);
            delivered += 1;
        }
        delivered
    }
}

/// An open binding: the transport to give the [`Endpoint`], and the events to feed it.
pub struct ZenohBinding<C> {
    pub transport: ZenohTransport<C>,
    pub events: ZenohEvents,
}

/// Open the binding for `machine` on `client`.
///
/// Readiness is the presence token (§mesh-20.3). The inbox is subscribed before the token is
/// declared, so that a machine which sees this one come up can send to it at once and not into a
/// subscriber that does not exist yet. The presence subscriber asks for history, so a machine that
/// opens after another learns it is up.
///
/// The client's notice that its session ended is asked for before anything is declared. When it
/// comes, every peer that was ready is told to the core as lost, once, and the binding is over:
/// [`ZenohEvents::session_ended`] answers and sends are refused as retryable. This binding does
/// not reconnect; `reconnect::open_reconnecting` does.
pub fn open_with<C: ZenohClient>(client: C, machine: &str) -> Result<ZenohBinding<C>, ZenohError> {
    check_machine_name(machine)?;
    let (tx, rx) = mpsc::channel::<LinkEvent>();
    let transport = open_into(client, machine, tx, |_| {})?;
    let events = ZenohEvents::new(rx, transport.session_state());
    Ok(ZenohBinding { transport, events })
}

/// Set `machine` up on `client`, with every [`LinkEvent`] sent into `tx`, and `on_end` called with
/// the client's words after the binding has told the core the session ended.
///
/// This is the one path every binding is opened by, so the order of the declarations is one copy:
/// the end notice, then the inbox, then presence with history, then the token. [`open_with`] makes
/// the channel; the reconnecting binding gives each of its sessions the channel its host already
/// reads, so what one session said and what the next says arrive in order. The machine name is
/// the caller's to have checked.
pub(crate) fn open_into<C, N>(
    mut client: C,
    machine: &str,
    tx: mpsc::Sender<LinkEvent>,
    on_end: N,
) -> Result<ZenohTransport<C>, ZenohError>
where
    C: ZenohClient,
    N: FnOnce(String) + Send + 'static,
{
    let session: SharedSession = Arc::new(Mutex::new(SessionState::default()));

    {
        let session = session.clone();
        let tx = tx.clone();
        client.on_session_end(move |words| {
            let mut state = locked(&session);
            if let Some(peers) = state.end(&words) {
                for peer in peers {
                    let _ = tx.send(LinkEvent::Lost(peer, SESSION_ENDED_WORDS.to_string()));
                }
                drop(state);
                on_end(words);
            }
        })?;
    }

    let inbox = {
        let tx = tx.clone();
        client.subscribe(&inbox_filter(machine), move |key, payload| {
            let sender = last_chunk(key);
            if is_path_segment(sender) {
                let _ = tx.send(LinkEvent::Received(sender.to_string(), payload.to_vec()));
            }
        })?
    };

    let presence = {
        let own = machine.to_string();
        let session = session.clone();
        client.subscribe_presence(&presence_filter(), true, move |key, kind| {
            let peer = last_chunk(key);
            if peer == own || !is_path_segment(peer) {
                return;
            }
            // The event is sent under the lock that records it, so the end of the session cannot
            // fall between a peer being counted as ready and the core being told so.
            let mut state = locked(&session);
            if let Some(event) = state.presence(peer, kind) {
                let _ = tx.send(event);
            }
        })?
    };

    let token = client.declare_presence(&presence_key(machine))?;

    Ok(ZenohTransport {
        client,
        machine: machine.to_string(),
        session,
        _held: vec![inbox, presence, token],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    type SampleCallback = Box<dyn FnMut(&str, &[u8]) + Send>;
    type PresenceCallback = Box<dyn FnMut(&str, PresenceKind) + Send>;
    type EndCallback = Box<dyn FnOnce(String) + Send>;

    /// What a [`Recorder`] was asked, and what it was told to do about it.
    #[derive(Default)]
    struct Record {
        calls: Vec<String>,
        inbox: Option<SampleCallback>,
        presence: Option<PresenceCallback>,
        end: Option<EndCallback>,
        /// How many calls had been made when the end notice was asked for.
        end_asked_after: Option<usize>,
        /// Whether the end notice itself is refused.
        refuse_end: bool,
        puts: Vec<(String, Vec<u8>)>,
        refuse_put: Option<PutError>,
        /// The declaration (`inbox`, `presence` or `token`) that fails.
        fail_on: Option<&'static str>,
    }

    /// A holder whose drop is the undeclaration, logged.
    struct Undeclare {
        record: Arc<Mutex<Record>>,
        what: &'static str,
    }

    impl Drop for Undeclare {
        fn drop(&mut self) {
            let what = self.what;
            self.record
                .lock()
                .unwrap()
                .calls
                .push(format!("undeclare {what}"));
        }
    }

    /// A client that does nothing but remember how it was used.
    struct Recorder(Arc<Mutex<Record>>);

    impl Recorder {
        fn declare(&self, what: &'static str, detail: String) -> Result<Held, ClientError> {
            let mut record = self.0.lock().unwrap();
            record.calls.push(format!("declare {what} {detail}"));
            if record.fail_on == Some(what) {
                return Err(ClientError::new(format!("{what} refused")));
            }
            Ok(Box::new(Undeclare {
                record: self.0.clone(),
                what,
            }))
        }
    }

    impl Drop for Recorder {
        fn drop(&mut self) {
            self.0
                .lock()
                .unwrap()
                .calls
                .push("close client".to_string());
        }
    }

    impl ZenohClient for Recorder {
        fn subscribe<F>(&mut self, filter: &str, on_sample: F) -> Result<Held, ClientError>
        where
            F: FnMut(&str, &[u8]) + Send + 'static,
        {
            let held = self.declare("inbox", filter.to_string())?;
            self.0.lock().unwrap().inbox = Some(Box::new(on_sample));
            Ok(held)
        }

        fn subscribe_presence<F>(
            &mut self,
            filter: &str,
            history: bool,
            on_change: F,
        ) -> Result<Held, ClientError>
        where
            F: FnMut(&str, PresenceKind) + Send + 'static,
        {
            let held = self.declare("presence", format!("{filter} history={history}"))?;
            self.0.lock().unwrap().presence = Some(Box::new(on_change));
            Ok(held)
        }

        fn declare_presence(&mut self, key: &str) -> Result<Held, ClientError> {
            self.declare("token", key.to_string())
        }

        fn put(&mut self, key: &str, payload: &[u8]) -> Result<(), PutError> {
            let mut record = self.0.lock().unwrap();
            record.puts.push((key.to_string(), payload.to_vec()));
            match record.refuse_put.clone() {
                Some(refusal) => Err(refusal),
                None => Ok(()),
            }
        }

        fn on_session_end<F>(&mut self, on_end: F) -> Result<(), ClientError>
        where
            F: FnOnce(String) + Send + 'static,
        {
            let mut record = self.0.lock().unwrap();
            record.end_asked_after = Some(record.calls.len());
            if record.refuse_end {
                return Err(ClientError::new("end refused"));
            }
            record.end = Some(Box::new(on_end));
            Ok(())
        }
    }

    fn recorder() -> (Recorder, Arc<Mutex<Record>>) {
        let record = Arc::new(Mutex::new(Record::default()));
        (Recorder(record.clone()), record)
    }

    fn calls(record: &Arc<Mutex<Record>>) -> Vec<String> {
        record.lock().unwrap().calls.clone()
    }

    #[test]
    fn keys_carry_the_receiver_then_the_sender() {
        assert_eq!(
            inbox_key("server", "phone-1"),
            "sce-mesh/1/to/server/phone-1"
        );
        assert_eq!(inbox_filter("server"), "sce-mesh/1/to/server/*");
        assert_eq!(presence_key("phone-1"), "sce-mesh/1/up/phone-1");
        assert_eq!(presence_filter(), "sce-mesh/1/up/*");
    }

    #[test]
    fn a_machine_is_named_by_the_last_chunk_of_its_key() {
        assert_eq!(last_chunk("sce-mesh/1/to/server/phone-1"), "phone-1");
        assert_eq!(last_chunk("sce-mesh/1/up/server"), "server");
        assert_eq!(last_chunk(""), "");
    }

    #[test]
    fn a_name_that_is_not_a_key_chunk_is_refused_before_the_client_is_touched() {
        for bad in ["", "a/b", "a*b", "a$b", "a b", "a?b", "a#b"] {
            let (client, record) = recorder();
            match open_with(client, bad) {
                Err(ZenohError::NotAKeyChunk(name)) => assert_eq!(name, bad),
                Err(other) => panic!("{bad:?} gave {other}"),
                Ok(_) => panic!("{bad:?} opened a binding"),
            }
            assert_eq!(
                calls(&record),
                ["close client"],
                "{bad:?} declared something"
            );
        }
        assert!(check_machine_name("phone-1").is_ok());
    }

    #[test]
    fn the_inbox_is_subscribed_before_presence_is_declared() {
        let (client, record) = recorder();
        let _binding = open_with(client, "me").expect("the binding opens");
        assert_eq!(
            calls(&record),
            [
                "declare inbox sce-mesh/1/to/me/*",
                "declare presence sce-mesh/1/up/* history=true",
                "declare token sce-mesh/1/up/me",
            ]
        );
    }

    #[test]
    fn a_sample_on_the_inbox_is_received_from_the_last_chunk_of_its_key() {
        let (client, record) = recorder();
        let binding = open_with(client, "me").expect("the binding opens");
        let mut inbox = record.lock().unwrap().inbox.take().expect("subscribed");

        inbox("sce-mesh/1/to/me/alice", b"hello");
        inbox("sce-mesh/1/to/me/a b", b"unnamed");
        assert_eq!(
            binding.events.try_next(),
            Some(LinkEvent::Received("alice".to_string(), b"hello".to_vec()))
        );
        assert_eq!(
            binding.events.try_next(),
            None,
            "a sender that is not a key chunk is not an event"
        );
    }

    #[test]
    fn presence_is_ready_or_lost_and_never_ones_own() {
        let (client, record) = recorder();
        let binding = open_with(client, "me").expect("the binding opens");
        let mut presence = record.lock().unwrap().presence.take().expect("subscribed");

        presence("sce-mesh/1/up/alice", PresenceKind::Put);
        presence("sce-mesh/1/up/me", PresenceKind::Put);
        presence("sce-mesh/1/up/me", PresenceKind::Delete);
        presence("sce-mesh/1/up/a b", PresenceKind::Put);
        presence("sce-mesh/1/up/alice", PresenceKind::Delete);
        assert_eq!(
            binding.events.try_next(),
            Some(LinkEvent::Ready("alice".to_string()))
        );
        assert_eq!(
            binding.events.try_next(),
            Some(LinkEvent::Lost(
                "alice".to_string(),
                "its presence token ended".to_string()
            ))
        );
        assert_eq!(
            binding.events.try_next(),
            None,
            "the machine's own presence and an unnamed one are not events"
        );
    }

    #[test]
    fn a_transmit_is_one_put_on_the_peers_inbox_under_this_machines_name() {
        let (client, record) = recorder();
        let mut binding = open_with(client, "me").expect("the binding opens");

        binding
            .transport
            .transmit("alice", b"envelope")
            .expect("the put is accepted");
        assert_eq!(
            record.lock().unwrap().puts,
            [("sce-mesh/1/to/alice/me".to_string(), b"envelope".to_vec())]
        );
    }

    #[test]
    fn a_transmit_to_a_name_that_is_not_a_key_chunk_is_terminal_and_never_put() {
        let (client, record) = recorder();
        let mut binding = open_with(client, "me").expect("the binding opens");

        let failure = binding
            .transport
            .transmit("a/b", b"envelope")
            .expect_err("the name is refused");
        assert!(!failure.retryable);
        assert!(record.lock().unwrap().puts.is_empty());
    }

    #[test]
    fn a_refused_put_carries_the_clients_words_and_its_retry_reading() {
        for retryable in [true, false] {
            let (client, record) = recorder();
            let mut binding = open_with(client, "me").expect("the binding opens");
            record.lock().unwrap().refuse_put = Some(PutError {
                retryable,
                message: "no route".to_string(),
            });

            let failure = binding
                .transport
                .transmit("alice", b"envelope")
                .expect_err("the put is refused");
            assert_eq!(
                failure,
                TransportFailure {
                    retryable,
                    message: Some("no route".to_string())
                }
            );
        }
    }

    #[test]
    fn a_declaration_the_client_refuses_fails_the_open_and_releases_what_came_before() {
        let (client, record) = recorder();
        record.lock().unwrap().fail_on = Some("token");

        match open_with(client, "me") {
            Err(ZenohError::Session(why)) => assert_eq!(why, "token refused"),
            Err(other) => panic!("a refused token gave {other}"),
            Ok(_) => panic!("a refused token opened a binding"),
        }
        let calls = calls(&record);
        for released in ["undeclare inbox", "undeclare presence"] {
            assert!(
                calls.iter().any(|call| call == released),
                "{released} is missing from {calls:?}"
            );
        }
    }

    #[test]
    fn dropping_the_transport_closes_the_client_and_undeclares_all_three() {
        let (client, record) = recorder();
        let binding = open_with(client, "me").expect("the binding opens");
        let ZenohBinding { transport, events } = binding;
        drop(transport);
        drop(events);

        let calls = calls(&record);
        for gone in [
            "close client",
            "undeclare inbox",
            "undeclare presence",
            "undeclare token",
        ] {
            assert!(
                calls.iter().any(|call| call == gone),
                "{gone} is missing from {calls:?}"
            );
        }
    }

    fn drain(events: &ZenohEvents) -> Vec<LinkEvent> {
        std::iter::from_fn(|| events.try_next()).collect()
    }

    fn lost_on_end(peer: &str) -> LinkEvent {
        LinkEvent::Lost(peer.to_string(), SESSION_ENDED_WORDS.to_string())
    }

    #[test]
    fn the_end_notice_is_asked_for_before_anything_is_declared() {
        let (client, record) = recorder();
        let _binding = open_with(client, "me").expect("the binding opens");
        assert_eq!(record.lock().unwrap().end_asked_after, Some(0));
    }

    #[test]
    fn a_client_that_refuses_the_end_notice_fails_the_open_before_declaring() {
        let (client, record) = recorder();
        record.lock().unwrap().refuse_end = true;

        match open_with(client, "me") {
            Err(ZenohError::Session(why)) => assert_eq!(why, "end refused"),
            Err(other) => panic!("a refused end notice gave {other}"),
            Ok(_) => panic!("a refused end notice opened a binding"),
        }
        assert_eq!(calls(&record), ["close client"]);
    }

    #[test]
    fn the_end_of_the_session_tells_every_ready_peer_lost_once() {
        let (client, record) = recorder();
        let binding = open_with(client, "me").expect("the binding opens");
        let mut presence = record.lock().unwrap().presence.take().expect("subscribed");
        let end = record.lock().unwrap().end.take().expect("asked for");

        presence("sce-mesh/1/up/alice", PresenceKind::Put);
        presence("sce-mesh/1/up/bob", PresenceKind::Put);
        presence("sce-mesh/1/up/carol", PresenceKind::Put);
        // Bob left before the session ended, and neither the machine's own presence nor an
        // unnamed one is a peer.
        presence("sce-mesh/1/up/bob", PresenceKind::Delete);
        presence("sce-mesh/1/up/me", PresenceKind::Put);
        presence("sce-mesh/1/up/a b", PresenceKind::Put);
        assert_eq!(binding.events.session_ended(), None);

        end("link reset".to_string());
        assert_eq!(
            drain(&binding.events),
            [
                LinkEvent::Ready("alice".to_string()),
                LinkEvent::Ready("bob".to_string()),
                LinkEvent::Ready("carol".to_string()),
                LinkEvent::Lost("bob".to_string(), "its presence token ended".to_string()),
                lost_on_end("alice"),
                lost_on_end("carol"),
            ],
            "each peer ready at the end is lost once, bob who had left is not lost again, and dave \
             who was never seen is not lost at all"
        );
        assert_eq!(
            binding.events.session_ended(),
            Some("link reset".to_string())
        );
    }

    #[test]
    fn presence_after_the_end_tells_the_core_nothing() {
        let (client, record) = recorder();
        let binding = open_with(client, "me").expect("the binding opens");
        let mut presence = record.lock().unwrap().presence.take().expect("subscribed");
        let end = record.lock().unwrap().end.take().expect("asked for");

        end("link reset".to_string());
        presence("sce-mesh/1/up/alice", PresenceKind::Put);
        presence("sce-mesh/1/up/alice", PresenceKind::Delete);
        assert!(
            drain(&binding.events).is_empty(),
            "a client that carries on by itself does not make peers ready on an ended binding"
        );
    }

    #[test]
    fn a_transmit_after_the_end_is_a_retryable_failure_and_puts_nothing() {
        let (client, record) = recorder();
        let mut binding = open_with(client, "me").expect("the binding opens");
        let end = record.lock().unwrap().end.take().expect("asked for");

        end("link reset".to_string());
        let failure = binding
            .transport
            .transmit("alice", b"envelope")
            .expect_err("the session has ended");
        assert_eq!(
            failure,
            TransportFailure {
                retryable: true,
                message: Some("the session to the router has ended".to_string())
            }
        );
        assert!(
            record.lock().unwrap().puts.is_empty(),
            "the client was not asked"
        );
    }

    #[test]
    fn a_second_end_changes_nothing() {
        let mut state = SessionState::default();
        state.presence("alice", PresenceKind::Put);

        let first = state.end("first").expect("the first end counts");
        assert_eq!(first.into_iter().collect::<Vec<_>>(), ["alice"]);
        assert_eq!(state.end("second"), None, "a second end tells nobody");
        assert_eq!(state.ended.as_deref(), Some("first"));
    }

    #[test]
    fn the_hook_after_the_end_runs_once_with_the_clients_words() {
        let (client, record) = recorder();
        let (tx, rx) = mpsc::channel();
        let (told_tx, told_rx) = mpsc::channel::<String>();
        let transport = open_into(client, "me", tx, move |words| {
            told_tx.send(words).expect("the test is listening");
        })
        .expect("the binding opens");
        let end = record.lock().unwrap().end.take().expect("asked for");

        end("link reset".to_string());
        assert_eq!(told_rx.try_recv(), Ok("link reset".to_string()));
        assert!(told_rx.try_recv().is_err(), "the hook ran once");
        drop(transport);
        drop(rx);
    }
}
