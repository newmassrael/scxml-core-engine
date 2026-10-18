// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding (SCE_MESH.md §mesh-20): the session plumbing between a Zenoh network and the
//! router core, and nothing else.
//!
//! A machine opens one client session and holds three things on it: a subscriber on its own inbox,
//! a liveliness token that says it is up, and a liveliness subscriber on everyone's. What arrives
//! becomes [`LinkEvent`]s the host feeds to its [`Endpoint`](crate::endpoint::Endpoint) with
//! [`deliver`](crate::link::deliver), as with the WebSocket binding, so every decision stays in the
//! core. The core's sends reach the network through [`ZenohTransport`], which publishes the bytes
//! on the inbox of the peer they name.
//!
//! The router the session connects to carries the bytes. Which one it is does not matter to this
//! module: it speaks upstream's `zenoh` client, so it reaches any router of that protocol version
//! (§mesh-20.4).

use std::any::Any;
use std::sync::mpsc;
use std::time::Duration;

use ::zenoh::qos::CongestionControl;
use ::zenoh::sample::SampleKind;
use ::zenoh::{Config, Session, Wait};

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

/// Why a binding could not be opened.
#[derive(Debug)]
pub enum ZenohError {
    /// A machine name is not a key chunk as it stands (§mesh-20.1).
    NotAKeyChunk(String),
    /// The session, a subscriber or the token could not be opened; the words are Zenoh's.
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

fn session_error(error: impl core::fmt::Display) -> ZenohError {
    ZenohError::Session(error.to_string())
}

/// A client configuration that connects to `endpoints` (`tcp/<host>:<port>`) and nothing else: no
/// multicast scouting, so the machine joins the network it was told to and not one it happened
/// upon.
pub fn client_config(endpoints: &[&str]) -> Result<Config, ZenohError> {
    let mut config = Config::default();
    let list = format!(
        "[{}]",
        endpoints
            .iter()
            .map(|endpoint| format!("\"{endpoint}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    for (key, value) in [
        ("mode", "\"client\"".to_string()),
        ("connect/endpoints", list),
        ("scouting/multicast/enabled", "false".to_string()),
    ] {
        config
            .insert_json5(key, &value)
            .map_err(|e| session_error(format!("{key}: {e}")))?;
    }
    Ok(config)
}

/// The core's side of the session: bytes published for the peer they name.
pub struct ZenohTransport {
    session: Session,
    machine: String,
    /// What the session holds open for this machine; dropping the transport closes it, which is
    /// what takes the presence token away and tells the other machines this one is gone.
    _held: Vec<Box<dyn Any + Send>>,
}

impl Transport for ZenohTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if !is_path_segment(peer) {
            return Err(TransportFailure {
                retryable: false,
                message: Some(format!("peer name {peer:?} is not a key chunk")),
            });
        }
        // One envelope per put, and Block rather than drop under congestion (§mesh-20.2): an
        // envelope is the unit the core's delivery rules count, and one the network dropped
        // silently is one the core would wait for.
        self.session
            .put(inbox_key(peer, &self.machine), bytes.to_vec())
            .congestion_control(CongestionControl::Block)
            .wait()
            .map_err(|e| TransportFailure {
                retryable: true,
                message: Some(e.to_string()),
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
pub struct ZenohBinding {
    pub transport: ZenohTransport,
    pub events: ZenohEvents,
}

/// Open the binding for `machine` on a session made from `config`.
///
/// Readiness is the presence token (§mesh-20.3). The inbox is subscribed before the token is
/// declared, so that a machine which sees this
/// one come up can send to it at once and not into a subscriber that does not exist yet. The
/// presence subscriber asks for history, so a machine that opens after another learns it is up.
pub fn open(config: Config, machine: &str) -> Result<ZenohBinding, ZenohError> {
    if !is_path_segment(machine) {
        return Err(ZenohError::NotAKeyChunk(machine.to_string()));
    }
    let session = ::zenoh::open(config).wait().map_err(session_error)?;
    let (tx, rx) = mpsc::channel::<LinkEvent>();

    let inbox = {
        let tx = tx.clone();
        session
            .declare_subscriber(inbox_filter(machine))
            .callback(move |sample| {
                let sender = last_chunk(sample.key_expr().as_str());
                if is_path_segment(sender) {
                    let _ = tx.send(LinkEvent::Received(
                        sender.to_string(),
                        sample.payload().to_bytes().into_owned(),
                    ));
                }
            })
            .wait()
            .map_err(session_error)?
    };

    let presence = {
        let own = machine.to_string();
        session
            .liveliness()
            .declare_subscriber(presence_filter())
            .history(true)
            .callback(move |sample| {
                let peer = last_chunk(sample.key_expr().as_str());
                if peer == own || !is_path_segment(peer) {
                    return;
                }
                let _ = tx.send(match sample.kind() {
                    SampleKind::Put => LinkEvent::Ready(peer.to_string()),
                    SampleKind::Delete => {
                        LinkEvent::Lost(peer.to_string(), "its presence token ended".to_string())
                    }
                });
            })
            .wait()
            .map_err(session_error)?
    };

    let token = session
        .liveliness()
        .declare_token(presence_key(machine))
        .wait()
        .map_err(session_error)?;

    Ok(ZenohBinding {
        transport: ZenohTransport {
            session,
            machine: machine.to_string(),
            _held: vec![Box::new(inbox), Box::new(presence), Box::new(token)],
        },
        events: ZenohEvents { events: rx },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_name_that_is_not_a_key_chunk_is_refused_before_a_session_is_opened() {
        for bad in ["", "a/b", "a*b", "a$b", "a b", "a?b", "a#b"] {
            match open(Config::default(), bad) {
                Err(ZenohError::NotAKeyChunk(name)) => assert_eq!(name, bad),
                Err(other) => panic!("{bad:?} gave {other}"),
                Ok(_) => panic!("{bad:?} opened a binding"),
            }
        }
    }

    #[test]
    fn a_client_config_names_its_endpoints_and_does_not_scout() {
        let config = client_config(&["tcp/127.0.0.1:7447", "tcp/10.0.0.2:7447"]).unwrap();
        let text = format!("{config:?}");
        assert!(text.contains("127.0.0.1:7447") && text.contains("10.0.0.2:7447"));
    }
}
