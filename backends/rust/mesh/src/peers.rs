// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A machine's Mesh peers as its deployment binds them (SCE_MESH.md
//! §mesh-19): the shape `sce-codegen generate --deploy --lang rust` fills in
//! `<machine>_mesh_peers.rs`, and the router it builds.
//!
//! The generated file is data only. Every decision about a binding — whether
//! its envelopes are deduplicated, ordered, buffered, retried — was taken by
//! the build from deploy.yaml, with the same function that decides it for
//! the C++ router, and lands here as a [`PeerConfig`]. What a host still
//! does by hand is open the sockets each [`PeerLink`] names.

use crate::inbound::ConfigError;
use crate::router::{PeerConfig, Router};

/// How the link to a peer is opened. The core never reads it; the host's
/// transport adapter does, to know which connections to open. (Not
/// [`crate::signal::Binding`], which is what a §mesh-16.7 row names.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerLink {
    /// §mesh-18.1: this machine dials the peer at `url`
    /// (`wss://<host>[:<port>][<base>]`, before the binding's own path).
    WssDial {
        url: &'static str,
        /// The §mesh-18.3 ping interval, or `None` for the binding's 30 s.
        keepalive_ms: Option<u32>,
    },
    /// §mesh-18.1: the peer dials this machine.
    WssAccept {
        /// The §mesh-18.3 ping interval, or `None` for the binding's 30 s.
        keepalive_ms: Option<u32>,
    },
    /// §mesh-20: the host's one Zenoh client session reaches the peer, by the
    /// key derived from the two machine names (section 20.1). The link carries
    /// nothing: the router endpoints are the host's own client configuration
    /// (the `zenoh` module's `client_config`), not a per-peer binding, and
    /// who is ready is the peer's liveliness token (section 20.3).
    ZenohInbox,
    /// The HTTPS binding (SCE_MESH.md section 22): this machine posts its exchanges to the peer
    /// at `url` (`https://<host>[:<port>][<base>]`, before the binding's path).
    HttpsDial {
        url: &'static str,
        /// How this machine proves its name.
        authenticator: HttpsAuthenticator,
        /// deploy.yaml's `hold_ms`: how long a poll asks to be held.
        hold_ms: u32,
    },
    /// The HTTPS binding: the peer posts its exchanges to this machine, which listens on
    /// `listen` (`<host>:<port>`) and keeps the peer's mailbox.
    HttpsAccept {
        listen: &'static str,
        /// Who may claim the peer's name.
        authenticator: HttpsAuthenticator,
        /// deploy.yaml's `max_hold_ms`.
        max_hold_ms: u32,
        /// deploy.yaml's `max_body_bytes`.
        max_body_bytes: u64,
        /// deploy.yaml's `absence_ms`.
        absence_ms: u32,
        /// deploy.yaml's `mailbox`: the envelopes that wait for the peer at most.
        mailbox: u32,
    },
}

/// Who may claim a name on an HTTPS binding (SCE_MESH.md section 22.5): deploy.yaml's
/// `authenticator`. The credential or the check itself is the host's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpsAuthenticator {
    /// `Authorization: Bearer <token>`.
    Bearer,
    /// `Authorization: SCE-Install <secret>`, pinned at the first exchange.
    Install,
    /// A TLS client certificate.
    Mtls,
}

/// One peer: the name `#<name>` addresses, how the core treats envelopes to
/// and from it, and how it is reached.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peer {
    pub name: &'static str,
    pub config: PeerConfig,
    pub link: PeerLink,
}

/// One machine's side of a deployment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// The machine's name: the `source` its envelopes carry.
    pub name: &'static str,
    /// deploy.yaml's `dedup.window_size` (§mesh-10.5).
    pub dedup_window: u32,
    /// deploy.yaml's `ordering.gap_timeout_ms` (§mesh-10.6.1).
    pub gap_timeout_ms: i64,
    /// Every peer the machine is bound to, by name.
    pub peers: &'static [Peer],
}

impl Machine {
    /// A router holding every peer this machine is bound to.
    pub fn router(&self) -> Result<Router, ConfigError> {
        let mut router = Router::new(self.name, self.dedup_window, self.gap_timeout_ms)?;
        for peer in self.peers {
            router.add_peer(peer.name, peer.config);
        }
        Ok(router)
    }

    /// The peer named `name`, if the machine is bound to one.
    pub fn peer(&self, name: &str) -> Option<&'static Peer> {
        self.peers.iter().find(|peer| peer.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::Delivery;

    const MACHINE: Machine = Machine {
        name: "phone",
        dedup_window: 8,
        gap_timeout_ms: 50,
        peers: &[Peer {
            name: "cloud",
            config: PeerConfig {
                transport: "wss",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &["cloud"],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::WssDial {
                url: "wss://cal.example",
                keepalive_ms: None,
            },
        }],
    };

    #[test]
    fn a_machine_builds_a_router_holding_its_peers() {
        let mut router = MACHINE.router().expect("a valid configuration");
        // A bound peer is one the router can mark ready; an unbound one is
        // refused, so this tells the two apart.
        assert!(router.peer_ready("cloud", 0).is_ok());
        assert!(router.peer_ready("elsewhere", 0).is_err());
        assert_eq!(MACHINE.peer("cloud").map(|p| p.name), Some("cloud"));
        assert!(MACHINE.peer("elsewhere").is_none());
    }

    const ZENOH_MACHINE: Machine = Machine {
        name: "server",
        dedup_window: 8,
        gap_timeout_ms: 50,
        peers: &[Peer {
            name: "phone",
            config: PeerConfig {
                transport: "zenoh_inbox",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &["phone"],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::ZenohInbox,
        }],
    };

    #[test]
    fn a_zenoh_inbox_peer_is_held_like_any_other_and_its_link_has_no_parameters() {
        let mut router = ZENOH_MACHINE.router().expect("a valid configuration");
        assert!(router.peer_ready("phone", 0).is_ok());
        assert!(router.peer_ready("elsewhere", 0).is_err());
        // The link says only that the host's Zenoh session reaches the peer;
        // there is no address in it to read.
        assert_eq!(
            ZENOH_MACHINE.peer("phone").map(|p| p.link),
            Some(PeerLink::ZenohInbox)
        );
    }

    const HTTPS_SERVER: Machine = Machine {
        name: "cloud",
        dedup_window: 8,
        gap_timeout_ms: 50,
        peers: &[Peer {
            name: "phone",
            config: PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &["phone"],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsAccept {
                listen: "0.0.0.0:8443",
                authenticator: HttpsAuthenticator::Install,
                max_hold_ms: 30_000,
                max_body_bytes: 1_048_576,
                absence_ms: 120_000,
                mailbox: 1024,
            },
        }],
    };

    #[test]
    fn an_https_peer_is_held_like_any_other_and_its_link_carries_the_binding_keys() {
        let mut router = HTTPS_SERVER.router().expect("a valid configuration");
        assert!(router.peer_ready("phone", 0).is_ok());
        let Some(Peer {
            link:
                PeerLink::HttpsAccept {
                    authenticator,
                    mailbox,
                    ..
                },
            ..
        }) = HTTPS_SERVER.peer("phone")
        else {
            panic!("the server accepts its client");
        };
        assert_eq!(*authenticator, HttpsAuthenticator::Install);
        assert_eq!(*mailbox, 1024);
    }

    #[test]
    fn a_machine_the_core_cannot_hold_is_refused_at_router_time() {
        let empty_window = Machine {
            dedup_window: 0,
            ..MACHINE
        };
        assert!(empty_window.router().is_err());
    }
}
