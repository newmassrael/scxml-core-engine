// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding's own logic, and the four operations it asks of a client (§mesh-20).
//!
//! A machine opens one client session and holds three things on it: a subscriber on its own inbox,
//! a liveliness token that says it is up, and a liveliness subscriber on everyone's. What arrives
//! becomes [`LinkEvent`]s the host feeds to its [`Endpoint`] with [`deliver`], as with the
//! WebSocket binding, so every decision stays in the core. The core's sends reach the network
//! through [`ZenohTransport`], which publishes the bytes on the inbox of the peer they name.
//!
//! The session itself is not this module's. It is a [`ZenohClient`]: four synchronous operations
//! (subscribe to keys, subscribe to presence, declare presence, put) and nothing else. Everything
//! outside them is here and stays one copy whichever client is plugged in: the key layout, the
//! name check, the order the session is set up in, the filter that keeps a machine's own presence
//! from reaching it, the mapping from what the client reports to a [`LinkEvent`], and the reading
//! of a refused put. That is what lets the client be chosen: upstream's `zenoh` crate is the
//! implementation in this repository (module `zenoh`, feature `zenoh`), another client implements
//! the trait in its own repository, and the tests in module `conformance` (feature `conformance`)
//! hold either to the same behaviour.
//!
//! This module needs no feature and no dependency beyond the standard library and the core.

use std::any::Any;
use std::sync::mpsc;
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
/// nothing more. The binding owns the keys, the order and the meaning of what arrives; a client
/// only moves bytes and presence.
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

/// The core's side of the session: bytes published for the peer they name.
pub struct ZenohTransport<C> {
    client: C,
    machine: String,
    /// What the client holds open for this machine; dropping the transport closes it, which is
    /// what takes the presence token away and tells the other machines this one is gone.
    _held: Vec<Held>,
}

impl<C: ZenohClient> Transport for ZenohTransport<C> {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if !is_path_segment(peer) {
            return Err(TransportFailure {
                retryable: false,
                message: Some(format!("peer name {peer:?} is not a key chunk")),
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
}

impl ZenohEvents {
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
pub fn open_with<C: ZenohClient>(
    mut client: C,
    machine: &str,
) -> Result<ZenohBinding<C>, ZenohError> {
    check_machine_name(machine)?;
    let (tx, rx) = mpsc::channel::<LinkEvent>();

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
        client.subscribe_presence(&presence_filter(), true, move |key, kind| {
            let peer = last_chunk(key);
            if peer == own || !is_path_segment(peer) {
                return;
            }
            let _ = tx.send(match kind {
                PresenceKind::Put => LinkEvent::Ready(peer.to_string()),
                PresenceKind::Delete => {
                    LinkEvent::Lost(peer.to_string(), "its presence token ended".to_string())
                }
            });
        })?
    };

    let token = client.declare_presence(&presence_key(machine))?;

    Ok(ZenohBinding {
        transport: ZenohTransport {
            client,
            machine: machine.to_string(),
            _held: vec![inbox, presence, token],
        },
        events: ZenohEvents { events: rx },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type SampleCallback = Box<dyn FnMut(&str, &[u8]) + Send>;
    type PresenceCallback = Box<dyn FnMut(&str, PresenceKind) + Send>;

    /// What a [`Recorder`] was asked, and what it was told to do about it.
    #[derive(Default)]
    struct Record {
        calls: Vec<String>,
        inbox: Option<SampleCallback>,
        presence: Option<PresenceCallback>,
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
}
