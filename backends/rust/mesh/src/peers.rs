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

use alloc::string::String;

use crate::inbound::ConfigError;
use crate::router::{DurableSend, PeerConfig, Router};

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
        /// deploy.yaml's `verify`: the trust set of the host a response is verified under
        /// (docs/adr/0109, D9; SCE_MESH.md section 22.9), by its name; `None` verifies nothing.
        verify: Option<&'static str>,
        /// deploy.yaml's `receipt:` (docs/adr/0109, D10; SCE_MESH.md section 10.11): under
        /// [`Receipt::Durable`] the client acknowledges only what a durable checkpoint of its own
        /// holds, and keeps what it sends in its checkpoint until a `200` carried it.
        receipt: Receipt,
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
        /// deploy.yaml's `max_clients`: the client entries the server holds in memory at most
        /// (docs/adr/0109, D1).
        max_clients: u32,
        /// deploy.yaml's `max_response_bytes`: the mailbox bytes one response carries at most
        /// (docs/adr/0109, D1).
        max_response_bytes: u64,
        /// deploy.yaml's `rate:`: the edge's limiters (docs/adr/0109, D2).
        rate: EdgeRates,
        /// deploy.yaml's `trusted_proxies`: the CIDR blocks whose `Forwarded` header names the
        /// client's address (docs/adr/0109, D2), each as written and checked by the build.
        trusted_proxies: &'static [&'static str],
        /// deploy.yaml's `sign`: the `sign` key of the host every `200` is signed with
        /// (docs/adr/0109, D9; SCE_MESH.md section 22.9), by its id; `None` signs nothing.
        sign: Option<&'static str>,
        /// deploy.yaml's `max_skew_ms`: how far a signed request's `created` may lie from the
        /// server's clock under the `key` authenticator (docs/adr/0109, D9).
        max_skew_ms: u32,
        /// deploy.yaml's `fresh_ms`: the freshness step a request of an instance of an open
        /// machine under the `key` authenticator signs a value of (docs/adr/0117, D4); `None`
        /// on every other binding, whose requests sign none.
        fresh_ms: Option<u32>,
        /// deploy.yaml's `receipt:` (docs/adr/0109, D10; SCE_MESH.md section 10.11): under
        /// [`Receipt::Durable`] what the machine sends the peer is kept in the peer's outbox in
        /// the store until the peer acknowledges it, `mailbox` the durable window of it.
        receipt: Receipt,
        /// deploy.yaml's `retain_ms`: how long an envelope is kept in the peer's outbox unless
        /// the peer acknowledges it (docs/adr/0109, D10); read under [`Receipt::Durable`].
        retain_ms: u64,
    },
}

/// Which of SCE_MESH.md section 10.11's two receipt guarantees a binding gets: deploy.yaml's
/// `receipt:` (docs/adr/0109, D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Receipt {
    /// `receipt: transport`, the default: the transport's receipt, up to the receiving process.
    Transport,
    /// `receipt: durable`: nothing the receiver was sent is lost across a crash and a restore of
    /// either side. On the `https` binding the acknowledgement is the binding's own
    /// (`Sce-Mesh-Ack`, and the `200`), and the server keeps a durable outbox per client.
    /// `window` and `retry_ms` are the sending machine's `receipt: { window, retry_ms }` block
    /// (256 and 1000 when it has none): what a dialing side holds unacknowledged at most, and how
    /// long one it holds waits before it is sent again. The accepting side of `https` reads
    /// neither: its window is the binding's `mailbox`.
    Durable { window: u32, retry_ms: u32 },
}

/// One limiter of an HTTPS server's edge (docs/adr/0109, D2), judged by the generic cell rate
/// algorithm of `sce:std/rate/gcra_admit`: one permit regenerates every `interval_ms`, and at
/// most `burst` are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeRate {
    pub interval_ms: u64,
    pub burst: u32,
}

/// deploy.yaml's `rate:` of an HTTPS accepting binding (docs/adr/0109, D2): each limiter, `None`
/// when the deployment turned it off, and the most keys each tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeRates {
    /// Every exchange, keyed by the remote address (an IPv4 address, an IPv6 /64).
    pub address: Option<EdgeRate>,
    /// Every exchange that authenticated, keyed by the client's name.
    pub client: Option<EdgeRate>,
    /// Each `401`, keyed by the name and by the address.
    pub failed_auth: Option<EdgeRate>,
    /// Each name pinned for the first time, keyed by the address.
    pub new_names: Option<EdgeRate>,
    /// The most keys each limiter tracks.
    pub max_tracked: u32,
}

impl EdgeRates {
    /// deploy.yaml's defaults: `address` 100 a second with a burst of 200, `client` 20 a second
    /// with 40, `failed_auth` 10 a minute with 10, `new_names` 20 an hour with 20, each tracking
    /// 100000 keys at most.
    pub const DEFAULT: EdgeRates = EdgeRates {
        address: Some(EdgeRate {
            interval_ms: 10,
            burst: 200,
        }),
        client: Some(EdgeRate {
            interval_ms: 50,
            burst: 40,
        }),
        failed_auth: Some(EdgeRate {
            interval_ms: 6_000,
            burst: 10,
        }),
        new_names: Some(EdgeRate {
            interval_ms: 180_000,
            burst: 20,
        }),
        max_tracked: 100_000,
    };

    /// `rate: none`: no limiter.
    pub const NONE: EdgeRates = EdgeRates {
        address: None,
        client: None,
        failed_auth: None,
        new_names: None,
        max_tracked: 100_000,
    };
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
    /// Every request signed by the client's Ed25519 key, pinned at the first exchange or
    /// registered by the server's host (docs/adr/0109, D9; SCE_MESH.md section 22.9).
    Key,
}

/// One peer: the name `#<name>` addresses, how the core treats envelopes to
/// and from it, and how it is reached.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peer {
    pub name: &'static str,
    pub config: PeerConfig,
    pub link: PeerLink,
}

/// Where a keyed pool reads the key of the session an envelope is for
/// (deploy.yaml `sessions.key_from`, docs/adr/0107 decision D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFrom {
    /// `data.<field>`: the top-level field of the envelope's data, by its
    /// name. The build checked that every request the machine serves carries
    /// it, as a text or an integer.
    Field(&'static str),
    /// `client`: the name of the peer whose link the envelope arrived on.
    Client,
}

/// deploy.yaml's `sessions:`: the machine is served as a pool of sessions,
/// one per key, each loaded from and saved to the host's session store
/// ([`crate::pool`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sessions {
    /// Where a session's key is read.
    pub key_from: KeyFrom,
    /// The most bytes a key may hold: the bound of the key field's type, as
    /// the build read it from the request schemas, or the pool's ceiling for
    /// a client's name. A longer key names no session.
    pub max_key_bytes: u32,
    /// How long a session with no event and nothing pending stays in memory
    /// before it is saved and dropped, in milliseconds.
    pub idle_ms: u64,
    /// deploy.yaml's `sessions.max_live`: the most sessions the pool holds in
    /// memory (docs/adr/0109, D1).
    pub max_live: u32,
    /// deploy.yaml's `sessions.max_inbox`: the most envelopes that wait in one
    /// session's inbox (docs/adr/0109, D1).
    pub max_inbox: u32,
    /// deploy.yaml's `sessions.commit_delay_us`: how long, in microseconds,
    /// the pool's committer waits before it takes what was handed to it into
    /// one commit; `0` waits for nothing (docs/adr/0109, D15).
    pub commit_delay_us: u64,
    /// deploy.yaml's `follows:` (docs/adr/0114, D4): the journals whose
    /// entries the pool takes in, each delivered to the session its key field
    /// names ([`crate::pool::Pool::follow`]).
    pub follows: Follows,
    /// deploy.yaml's `sessions.position_from` (docs/adr/0116, D5): the
    /// top-level field of a request's data that may carry a position of the
    /// journals the pool follows, which an earlier answer told. The pool holds
    /// a request that carries one in its session's inbox until its reader has
    /// taken in every entry at or below it ([`crate::pool::Pool::follow`]).
    /// `None` for a pool whose requests carry none.
    pub position_from: Option<&'static str>,
}

/// The journals a keyed pool follows (deploy.yaml's `follows:`, docs/adr/0114,
/// D4), as the build read them against the document: each by its name, a
/// machine of the deployment or an observed stream, and the record each kind
/// of their entries is, which the pool reads an entry's data by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Follows {
    /// The machines and observed streams followed, by name.
    pub journals: &'static [&'static str],
    /// The kinds of entry they make, by the event each is delivered as.
    pub entries: &'static [FollowedEntry],
}

impl Follows {
    /// A pool that follows no journal.
    pub const NONE: Follows = Follows {
        journals: &[],
        entries: &[],
    };

    /// The kind of entry of `schema`, the id an entry names its schema by: the
    /// event the schema declares.
    pub fn entry(&self, schema: &str) -> Option<&FollowedEntry> {
        self.entries.iter().find(|entry| entry.event == schema)
    }

    /// Whether `journal` is one the pool follows.
    pub fn follows(&self, journal: &str) -> bool {
        self.journals.contains(&journal)
    }
}

/// One kind of entry a pool follows: the event its schema declares, which the
/// session is told it as, and the description of its record
/// (`sce_build::forge::payload_record::describe`), by which the entry's
/// deterministic CBOR is read into the data the event carries
/// (`sce_rust_runtime::record_cbor::decode`), as the event of a fact the
/// session said itself carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowedEntry {
    pub event: &'static str,
    pub record: &'static str,
}

/// How many copies of a machine the deployment runs (deploy.yaml's
/// `instances:`; docs/adr/0113, D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instances {
    /// The machine is one: its name is its identity.
    One,
    /// `instances: open`: any number of copies run, each an instance, and a
    /// sender of the machine is the pair (machine, instance), the instance
    /// being the id its host gives it ([`is_instance_id`]).
    Open,
}

/// The instance id's rule (docs/adr/0113, D1), the runtime's: the one rule the
/// send site that routes an instance's origin and this core read.
pub use sce_rust_runtime::helpers::send::{is_instance_id, MAX_INSTANCE_ID_CHARS};

/// The name an instance of the open machine `machine` is bound under (docs/adr/0113, D2 and D3):
/// the pair as the path its binding posts to carries it, `<machine>/<instance>`, which its origin
/// `mesh://<machine>/<instance>` names too. A machine's name is a segment of that path
/// ([`crate::link::is_path_segment`]) and so is an instance id ([`is_instance_id`]); neither
/// holds a `/`, so the name is one pair's and no declared peer's.
pub fn instance_peer(machine: &str, instance: &str) -> String {
    alloc::format!("{machine}/{instance}")
}

/// The pair (machine, instance) `name` holds, when it is the name of an instance
/// ([`instance_peer`]): the runtime's reading, which its send site routes an
/// instance's origin by.
pub fn instance_of(name: &str) -> Option<(&str, &str)> {
    sce_rust_runtime::helpers::send::instance_pair(name)
}

/// The terms a server holds the members of an open machine to (deploy.yaml's `members:`;
/// docs/adr/0113, D4 and D6), each the deployment's, none defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Members {
    /// How long a member a session kept a message of stays one past its last exchange.
    pub lease_ms: u64,
    /// How long a member no session kept a message of stays one past its last exchange, and how
    /// long an offer stands.
    pub provisional_ms: u64,
    /// The most members that stand at once.
    pub max_instances: u32,
}

impl Members {
    /// The terms as the membership store judges an exchange under them.
    pub fn terms(&self) -> sce_rust_session_store::Terms {
        sce_rust_session_store::Terms {
            lease_ms: self.lease_ms,
            provisional_ms: self.provisional_ms,
            max_members: u64::from(self.max_instances),
        }
    }
}

/// An open machine one machine is bound to (docs/adr/0113, D3): the one entry its instances are
/// bound under, and the terms its members are held to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenPeer {
    pub peer: Peer,
    pub members: Members,
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
    /// Every open machine the machine is bound to, by name (docs/adr/0113,
    /// D3): not one peer but any number of them, each an instance under the
    /// one [`PeerConfig`] here, whose responder is the instance itself
    /// ([`Responders::Instance`]), with the terms the deployment holds its
    /// members to. [`Self::router`] binds each ([`Router::open_machine`]).
    ///
    /// [`Responders::Instance`]: crate::router::Responders::Instance
    pub open_peers: &'static [OpenPeer],
    /// Whether this machine is one or an open set of instances.
    pub instances: Instances,
    /// The machine's keyed pool, when the deployment serves it as one; `None`
    /// for a machine that is one session.
    pub sessions: Option<Sessions>,
}

impl Machine {
    /// A router holding every peer this machine is bound to by name, and binding the instances of
    /// each open machine it is bound to as they arrive, at most its `max_instances` at once
    /// ([`Router::open_machine`], docs/adr/0113, D3 and D6). An open machine's name is no peer's
    /// ([`Self::open_peers`]), so a router refuses it as one it does not hold.
    ///
    /// A dialing `https` binding under `receipt: durable` (docs/adr/0109, D10) makes the router
    /// checkpoint and hold what it sends the peer until a `200` carried it, the binding's
    /// acknowledgement (SCE_MESH.md section 10.11): the host checkpoints the machine, as the
    /// deployment's `persistence: checkpoint` says it does. An open machine whose instances post
    /// to this one under `receipt: durable` makes the router checkpoint and judge each instance's
    /// requests by their completion records (docs/adr/0113, D5; [`Router::receive_durably`]).
    pub fn router(&self) -> Result<Router, ConfigError> {
        let mut router = Router::new(self.name, self.dedup_window, self.gap_timeout_ms)?;
        for peer in self.peers {
            router.add_peer(peer.name, peer.config);
        }
        for peer in self.peers {
            if let PeerLink::HttpsDial {
                receipt: Receipt::Durable { window, retry_ms },
                ..
            } = peer.link
            {
                router.enable_checkpoints();
                router
                    .send_durably(
                        peer.name,
                        DurableSend {
                            window: window as usize,
                            retry_ms: i64::from(retry_ms),
                        },
                    )
                    .map_err(|_| ConfigError::BadReceipt { peer: peer.name })?;
            }
        }
        for open in self.open_peers {
            router
                .open_machine(
                    open.peer.name,
                    open.peer.config,
                    open.members.max_instances as usize,
                )
                .map_err(|_| ConfigError::BadOpenMachine {
                    machine: open.peer.name,
                })?;
            if let PeerLink::HttpsAccept {
                receipt: Receipt::Durable { .. },
                ..
            } = open.peer.link
            {
                router.enable_checkpoints();
                router.receive_durably(open.peer.name).map_err(|_| {
                    ConfigError::BadOpenMachine {
                        machine: open.peer.name,
                    }
                })?;
            }
        }
        Ok(router)
    }

    /// The peer named `name`, if the machine is bound to one.
    pub fn peer(&self, name: &str) -> Option<&'static Peer> {
        self.peers.iter().find(|peer| peer.name == name)
    }

    /// The open machine named `name`, if the machine is bound to one: what
    /// each of its instances is a peer under, and the terms its members are
    /// held to.
    pub fn open_peer(&self, name: &str) -> Option<&'static OpenPeer> {
        self.open_peers.iter().find(|open| open.peer.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::Delivery;
    use crate::router::Responders;

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
                responders: Responders::Named(&["cloud"]),
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::WssDial {
                url: "wss://cal.example",
                keepalive_ms: None,
            },
        }],
        open_peers: &[],
        instances: Instances::One,
        sessions: None,
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
                responders: Responders::Named(&["phone"]),
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::ZenohInbox,
        }],
        open_peers: &[],
        instances: Instances::One,
        sessions: None,
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
                responders: Responders::Named(&["phone"]),
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
                max_clients: 100_000,
                max_response_bytes: 1_048_576,
                rate: EdgeRates::DEFAULT,
                trusted_proxies: &[],
                sign: None,
                max_skew_ms: 300_000,
                fresh_ms: None,
                receipt: Receipt::Transport,
                retain_ms: 2_592_000_000,
            },
        }],
        open_peers: &[],
        instances: Instances::One,
        sessions: None,
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

    /// docs/adr/0113, D1: an instance id is 1 to 64 characters of a closed set, every one of
    /// which a path segment carries as it stands. Red if the length is not held, or a character
    /// outside the set is taken.
    #[test]
    fn an_instance_id_is_one_to_sixty_four_characters_of_a_path_segment() {
        let longest = "a".repeat(MAX_INSTANCE_ID_CHARS);
        for id in ["0", "A-z_9.b", longest.as_str()] {
            assert!(is_instance_id(id), "{id}");
            assert!(crate::link::is_path_segment(id), "{id}");
        }
        let long = "a".repeat(MAX_INSTANCE_ID_CHARS + 1);
        for id in ["", "a b", "a/b", "a~b", "a%2Fb", "é", long.as_str()] {
            assert!(!is_instance_id(id), "{id}");
        }
    }

    /// docs/adr/0113, D2 and D3: an instance is bound under the pair as its path carries it, and
    /// the name is read back as that pair alone; a machine's name, a name with no instance id, and
    /// one with a segment more are no instance's. Red if the pair is read from a name that is not
    /// its form.
    #[test]
    fn an_instance_is_named_by_its_pair_and_no_other_name_reads_as_one() {
        assert_eq!(instance_peer("phone", "a1"), "phone/a1");
        assert_eq!(instance_of("phone/a1"), Some(("phone", "a1")));
        for name in [
            "phone",
            "phone/",
            "/a1",
            "phone/a1/x",
            "phone/a 1",
            "ph one/a1",
        ] {
            assert_eq!(instance_of(name), None, "{name}");
        }
    }

    /// docs/adr/0113, D3 and D6: an open machine is held beside the peers and not among them,
    /// under its one configuration, whose responder is the instance itself, with the terms of its
    /// members; the machine's name binds no peer of the router, an instance of it is bound as it
    /// arrives, and none past the deployment's `max_instances`. Red if `Machine::router` binds
    /// an open machine by its name, or does not register it with its bound.
    #[test]
    fn an_open_machine_is_bound_by_its_instances_up_to_its_bound() {
        const SERVER: Machine = Machine {
            peers: &[],
            open_peers: &[OpenPeer {
                peer: Peer {
                    config: PeerConfig {
                        responders: Responders::Instance,
                        ..HTTPS_SERVER.peers[0].config
                    },
                    ..HTTPS_SERVER.peers[0]
                },
                members: Members {
                    lease_ms: 60_000,
                    provisional_ms: 1_000,
                    max_instances: 1,
                },
            }],
            ..HTTPS_SERVER
        };
        let mut router = SERVER.router().expect("a valid configuration");
        assert!(router.peer_ready("phone", 0).is_err());
        assert!(
            router.peer_ready("phone/a1", 0).is_ok(),
            "an instance is bound"
        );
        assert!(
            matches!(
                router.peer_ready("phone/b2", 0),
                Err(crate::router::RouterError::InstancesFull {
                    max_instances: 1,
                    ..
                })
            ),
            "no instance past max_instances"
        );
        assert!(SERVER.peer("phone").is_none());
        let open = SERVER.open_peer("phone").expect("the open machine");
        assert_eq!(open.peer.config.responders, Responders::Instance);
        assert_eq!(
            open.members.terms(),
            sce_rust_session_store::Terms {
                lease_ms: 60_000,
                provisional_ms: 1_000,
                max_members: 1,
            }
        );
        assert!(SERVER.open_peer("elsewhere").is_none());
        assert_eq!(SERVER.instances, Instances::One, "the server is one");
    }

    /// docs/adr/0113, D5: an open machine whose instances post under `receipt: durable` makes the
    /// router checkpoint and judge each instance's requests by their completion records, so an
    /// envelope handed without the incarnation its edge took it in under is refused as unjudged.
    /// Red if `Machine::router` leaves the open machine to a window of ids.
    #[test]
    fn a_durable_open_machine_is_judged_by_its_completion_records() {
        const DURABLE: Machine = Machine {
            peers: &[],
            open_peers: &[OpenPeer {
                peer: Peer {
                    name: "phone",
                    config: PeerConfig {
                        responders: Responders::Instance,
                        ..HTTPS_SERVER.peers[0].config
                    },
                    link: PeerLink::HttpsAccept {
                        listen: "0.0.0.0:8443",
                        authenticator: HttpsAuthenticator::Key,
                        max_hold_ms: 30_000,
                        max_body_bytes: 1_048_576,
                        absence_ms: 120_000,
                        mailbox: 1024,
                        max_clients: 100_000,
                        max_response_bytes: 1_048_576,
                        rate: EdgeRates::DEFAULT,
                        trusted_proxies: &[],
                        sign: None,
                        max_skew_ms: 300_000,
                        fresh_ms: None,
                        receipt: Receipt::Durable {
                            window: 1024,
                            retry_ms: 1_000,
                        },
                        retain_ms: 2_592_000_000,
                    },
                },
                members: Members {
                    lease_ms: 60_000,
                    provisional_ms: 1_000,
                    max_instances: 4,
                },
            }],
            ..HTTPS_SERVER
        };
        let mut router = DURABLE.router().expect("a valid configuration");
        assert!(router.is_checkpointed(), "the host checkpoints the machine");
        let envelope = crate::generated::envelope::Envelope {
            source: crate::router::within("phone").unwrap(),
            instance: Some(crate::router::within("a1").unwrap()),
            event_type: crate::router::within("hello").unwrap(),
            sequence_no: Some(1),
            ..crate::generated::envelope::Envelope::new()
        }
        .encode_to_vec();
        assert_eq!(
            router.receive("phone/a1", envelope, 0),
            Err(crate::router::RouterError::Unjudged("phone/a1".to_string()))
        );
    }

    /// docs/adr/0113, D6: an open machine with room for no instance is refused when its router is
    /// built, as the build refuses it. Red if `Machine::router` skips the open machines.
    #[test]
    fn an_open_machine_with_room_for_none_is_refused_at_router_time() {
        const NONE: Machine = Machine {
            peers: &[],
            open_peers: &[OpenPeer {
                peer: Peer {
                    config: PeerConfig {
                        responders: Responders::Instance,
                        ..HTTPS_SERVER.peers[0].config
                    },
                    ..HTTPS_SERVER.peers[0]
                },
                members: Members {
                    lease_ms: 60_000,
                    provisional_ms: 1_000,
                    max_instances: 0,
                },
            }],
            ..HTTPS_SERVER
        };
        assert_eq!(
            NONE.router().err(),
            Some(ConfigError::BadOpenMachine { machine: "phone" })
        );
    }

    /// A client of an `https` server whose binding declares `receipt: durable`.
    fn durable_dialer(window: u32) -> Machine {
        let peers: &'static [Peer] = Box::leak(Box::new([Peer {
            name: "cloud",
            config: PeerConfig {
                transport: "https",
                ..MACHINE.peers[0].config
            },
            link: PeerLink::HttpsDial {
                url: "https://cal.example",
                authenticator: HttpsAuthenticator::Bearer,
                hold_ms: 25_000,
                verify: None,
                receipt: Receipt::Durable {
                    window,
                    retry_ms: 1_000,
                },
            },
        }]));
        Machine { peers, ..MACHINE }
    }

    /// docs/adr/0109, D10: a dialing `https` binding under `receipt: durable` makes the router
    /// checkpoint and hold what it sends, after the checkpoint that produced it is durable, until
    /// an exchange answered `200` carried it, which the binding tells as `Carried`; a window of 0,
    /// which the build refuses, is refused here too. Red if `Machine::router` applies no receipt,
    /// or `Router::carried` acknowledges nothing.
    #[test]
    fn a_durable_dialing_binding_holds_what_it_sends_until_an_exchange_carried_it() {
        use crate::checkpoint::Clocks;
        use crate::router::Effect;
        use sce_rust_runtime::HostSendRequest;

        let mut router = durable_dialer(4).router().expect("a valid configuration");
        assert!(router.is_checkpointed());
        assert!(router.holds_until_acknowledged("cloud"));
        router.peer_ready("cloud", 0).expect("a bound peer");
        let produced = router
            .send(
                &HostSendRequest {
                    event_name: "note".to_string(),
                    target: "#cloud".to_string(),
                    ..HostSendRequest::default()
                },
                [7; 16],
                0,
            )
            .expect("a send");
        assert!(produced.is_empty(), "held for its checkpoint");
        router
            .seal(
                1,
                Clocks {
                    monotonic_ms: 0,
                    wall_ms: 0,
                },
            )
            .expect("sealed");
        let released = router.durable(1, 0).expect("durable");
        let bytes = match released.as_slice() {
            [Effect::Transmit { bytes, .. }] => bytes.clone(),
            other => panic!("the send leaves once its checkpoint is durable, not {other:?}"),
        };
        assert!(
            router.holds_for_durability(),
            "held until an exchange carried it"
        );
        router.carried("cloud", &[bytes]);
        assert!(!router.holds_for_durability(), "the 200 acknowledged it");

        assert_eq!(
            durable_dialer(0).router().err(),
            Some(ConfigError::BadReceipt { peer: "cloud" })
        );
    }
}
