// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The accepting side of the HTTPS binding (SCE_MESH.md section 22): the server machine's end,
//! where each client's envelopes wait in a mailbox until the client's next exchange carries them.
//!
//! The server of §mesh-22.8: hyper on tokio, HTTP/1.1 and HTTP/2 on one listener, TLS from
//! [`server_tls`], the mailbox as the endpoint's transport. For each exchange the server reads
//! the client's name from the path, checks the authenticator, drops what the client
//! acknowledged, hands the core what the request carried and waits until the macrosteps it
//! caused have run ([`ServerHost::deliver`]), then answers with what the mailbox holds, holding
//! the exchange while it holds nothing (§mesh-22.3).
//!
//! What the machine sends a client reaches the mailbox through [`MailboxTransport`], the
//! transport the host builds its endpoint on. A full mailbox refuses the envelope as a full queue
//! ([`TransportFailure::queue_full`]), which the core raises as SCE_MESH.md section 16.7 row 9.
//!
//! A mailbox numbers in an incarnation, drawn when its state starts empty and named on every `200`
//! (`Sce-Mesh-Epoch`), so that a client tells a mailbox that numbers anew from one it knows.
//!
//! What a client costs the server is bounded (docs/adr/0109, D1): a client's entry is removed
//! once it holds nothing (its mailbox empty, no exchange in flight, and not ready), so a name that
//! makes one exchange and goes away is forgotten after `absence_ms`; at most `max_clients` entries
//! are held, and an exchange for a name past it is answered `503`; a connection is closed at
//! accept past `max_connections` or `max_connections_per_address`, and when its TLS handshake, its
//! request head or its body takes longer than its bound; and a response carries at most
//! `max_response_bytes` of the mailbox, the rest at the next exchange.
//!
//! A host whose endpoint checkpoints (SCE_MESH.md section
//! 10.11) answers an exchange that carried envelopes only once a durable checkpoint holds them
//! ([`ServerHost::deliver`], [`Durability`], [`HttpsServer::durable`]); a keyed pool reports it
//! per exchange from the receipts of the envelopes it was handed ([`crate::pool::Receipt`]).
//!
//! The host is told what one client's exchanges carried one exchange at a time, in the order
//! decided, under that client's order, and what different clients' exchanges carried at once
//! (docs/adr/0109, D15): their sessions then run beside each other, and their checkpoints share
//! the pool's commits, while no `200` goes out before the commit that holds its envelopes.
//!
//! The server reports what it does through the metrics port its program gives it
//! (docs/adr/0109, D16; [`HttpsServer::observed`]): each exchange by the status it was answered,
//! the exchanges it holds open, each exchange it shed and each connection it closed at accept by
//! why, and each `401` by its authenticator. [`HttpsServer::health`] says whether it takes
//! exchanges in, whether a stop is going on, and whether the stores it reads answer.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Incoming;
use hyper::header::{
    HeaderName, HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE as CONTENT_TYPE_NAME, HOST,
};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::Notify;

use sce_rust_crypto::cose::{cose_key_ed25519_required, ed25519_of_cose_key};
use sce_rust_crypto::ed25519::PUBLIC_KEY_BYTES;
use sce_rust_metrics::{AuthenticatorKind, Counter, Gauge, Metrics, ResponseStatus, ServerShed};

use super::freshness::{Freshness, FRESH_HEADER, USE_FRESH};
use super::rate::{client_address, is_trusted, retry_seconds, Cidr, Edge, Verdict};
pub use super::signature::ResponseSigner;
use super::signature::{
    content_digest, digest_matches, key_of_offer, target_uri, Answered, RequestSignature,
    CONTENT_DIGEST_HEADER, DEFAULT_MAX_SKEW_MS, KEY_HEADER, KEY_SCHEME, NONCE_HEADER,
};
use super::{
    decode_install_secret, draw_epoch, install_digest, is_epoch, is_path_segment, split_sequence,
    LinkEvent, ACK_HEADER, BEARER_SCHEME, CONTENT_TYPE, EPOCH_HEADER, FIRST_HEADER, HOLD_HEADER,
    INSTALL_SCHEME, PATH_ROOT, VERSION,
};
use crate::endpoint::{Transport, TransportFailure};
use crate::health::Health;
use crate::outbox::{Outboxes, Wake};
use crate::peers::{EdgeRate, EdgeRates, HttpsAuthenticator, Machine, PeerLink};
use sce_rust_crypto::equal::same_secret;

// The credentials an authenticator reads (ADR 0109, D3): the pin store of `install`, its memory
// implementation, and the reference check of `bearer` over a credential store. The store contract
// is `sce-rust-session-store`'s, whose `SqliteStore` is the durable implementation.
pub use sce_rust_crypto::credential::StoredTokens;
pub use sce_rust_session_store::{MemoryCredentials, PinStore};

/// The bounds a server keeps: deploy.yaml's keys of the accepting side, and two of the host's own
/// for shedding load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    /// The path the binding is served under, before `/sce-mesh/`: empty, or `/` followed by
    /// segments, without a trailing `/`.
    pub base_path: String,
    /// deploy.yaml's `max_hold_ms`: the longest an exchange is held while the mailbox is empty.
    pub max_hold: Duration,
    /// deploy.yaml's `max_body_bytes`: the largest request body taken in.
    pub max_body_bytes: usize,
    /// deploy.yaml's `absence_ms`: how long a client may go without an exchange and stay ready.
    pub absence: Duration,
    /// deploy.yaml's `mailbox`: how many envelopes wait for one client at most.
    pub mailbox: usize,
    /// The most exchanges in flight at once; one more is answered `503`.
    pub max_exchanges: usize,
    /// The most exchanges in flight for one client; one more is answered `429`.
    pub max_exchanges_per_client: usize,
    /// What a `429` or `503` asks the client to wait (`Retry-After`, in whole seconds).
    pub retry_after: Duration,
    /// deploy.yaml's `max_clients`: the most client entries held in memory; an exchange for a name
    /// past it is answered `503` (docs/adr/0109, D1).
    pub max_clients: usize,
    /// deploy.yaml's `max_response_bytes`: the most mailbox bytes one response carries; the rest
    /// go at the next exchange (docs/adr/0109, D1).
    pub max_response_bytes: usize,
    /// The longest a TLS handshake may take; the connection is closed past it.
    pub handshake_timeout: Duration,
    /// The longest a request's head may take to arrive; the connection is closed past it.
    pub header_timeout: Duration,
    /// The longest a request's whole body may take to arrive; the exchange is answered `408`.
    pub body_timeout: Duration,
    /// The most connections open at once; one more is closed at accept.
    pub max_connections: usize,
    /// The most connections open at once from one address (an IPv4 address, an IPv6 /64); one
    /// more is closed at accept. A trusted proxy's connections, which carry many clients, are
    /// counted only toward `max_connections`.
    pub max_connections_per_address: usize,
    /// deploy.yaml's `rate:`: the edge's limiters (docs/adr/0109, D2). An exchange past one is
    /// answered `429` with `Retry-After`, the limiter's wait rounded up to a second.
    pub rates: EdgeRates,
    /// deploy.yaml's `trusted_proxies`: a connection from an address in one of these blocks has
    /// its client's address read from `Forwarded` (RFC 7239) (docs/adr/0109, D2).
    pub trusted_proxies: Vec<Cidr>,
    /// deploy.yaml's `sign:`, the key it names resolved by the host: every `200` answering a
    /// request that carries a nonce is signed by it (SCE_MESH.md section 22.9, docs/adr/0109 D9),
    /// so that a client under `verify:` tells it from what an intermediary wrote. `None` signs
    /// nothing.
    pub signer: Option<ResponseSigner>,
    /// deploy.yaml's `max_skew_ms`: how far a request's `created` may lie from the server's clock
    /// under the `key` authenticator (SCE_MESH.md section 22.9).
    pub max_skew: Duration,
    /// deploy.yaml's `fresh_ms`: the freshness step a request under the `key` authenticator is
    /// held to (docs/adr/0117, D4), its values made of a key the server draws when it is built;
    /// `None` holds a request to none.
    pub fresh: Option<Duration>,
}

/// deploy.yaml's `max_clients` when the binding sets none (docs/adr/0109, D1).
pub const DEFAULT_MAX_CLIENTS: usize = 100_000;

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            base_path: String::new(),
            max_hold: Duration::from_millis(u64::from(super::DEFAULT_MAX_HOLD_MS)),
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES as usize,
            absence: Duration::from_millis(u64::from(super::DEFAULT_ABSENCE_MS)),
            mailbox: super::DEFAULT_MAILBOX as usize,
            max_exchanges: 10_000,
            // The client's poller and its sender, and one more for an exchange the client gave up
            // on that the server has not yet seen end.
            max_exchanges_per_client: 3,
            retry_after: Duration::from_secs(1),
            max_clients: DEFAULT_MAX_CLIENTS,
            max_response_bytes: super::DEFAULT_MAX_BODY_BYTES as usize,
            handshake_timeout: Duration::from_secs(10),
            header_timeout: Duration::from_secs(10),
            body_timeout: Duration::from_secs(30),
            max_connections: 20_000,
            max_connections_per_address: 64,
            rates: EdgeRates::DEFAULT,
            trusted_proxies: Vec::new(),
            signer: None,
            max_skew: Duration::from_millis(u64::from(DEFAULT_MAX_SKEW_MS)),
            fresh: None,
        }
    }
}

/// Of two settings of one limiter, the one that admits less: the longer interval and the smaller
/// burst, and a limiter that is on wherever either is.
fn stricter(a: Option<EdgeRate>, b: Option<EdgeRate>) -> Option<EdgeRate> {
    match (a, b) {
        (Some(a), Some(b)) => Some(EdgeRate {
            interval_ms: a.interval_ms.max(b.interval_ms),
            burst: a.burst.min(b.burst),
        }),
        (one, None) | (None, one) => one,
    }
}

/// What a machine's peer table says of one address it listens on for the HTTPS binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listening {
    /// The bounds: of each, the smallest any binding on the address declares, so that no binding
    /// is given more than it declared.
    pub settings: ServerSettings,
    /// The authenticator every binding on the address names.
    pub authenticator: HttpsAuthenticator,
    /// The id of the `sign` key every binding on the address names (docs/adr/0109, D9), which
    /// the program resolves in its host's key ring into [`ServerSettings::signer`]; `None` signs
    /// nothing.
    pub sign: Option<&'static str>,
    /// The peers accepted on the address: the names a server built for it binds.
    pub peers: Vec<&'static str>,
}

impl Listening {
    /// What `machine` accepts over HTTPS on `listen` ([`PeerLink::HttpsAccept`]). `None` when no
    /// peer is accepted there, or when the bindings there name different authenticators,
    /// different `sign` keys or different freshness steps, which one server cannot keep by one
    /// rule.
    pub fn on(machine: &Machine, listen: &str) -> Option<Listening> {
        let mut found: Option<Listening> = None;
        for peer in machine.peers {
            let PeerLink::HttpsAccept {
                listen: at,
                authenticator,
                max_hold_ms,
                max_body_bytes,
                absence_ms,
                mailbox,
                max_clients,
                max_response_bytes,
                rate,
                trusted_proxies,
                sign,
                max_skew_ms,
                fresh_ms,
                // The outbox of a durable binding is the keyed pool's, kept in its store
                // (`HttpsServer::keep_outboxes`), not a bound of the server's own.
                receipt: _,
                retain_ms: _,
            } = peer.link
            else {
                continue;
            };
            if at != listen {
                continue;
            }
            let declared = ServerSettings {
                max_hold: Duration::from_millis(u64::from(max_hold_ms)),
                max_body_bytes: usize::try_from(max_body_bytes).unwrap_or(usize::MAX),
                absence: Duration::from_millis(u64::from(absence_ms)),
                mailbox: mailbox as usize,
                max_clients: max_clients as usize,
                max_response_bytes: usize::try_from(max_response_bytes).unwrap_or(usize::MAX),
                rates: rate,
                // The build checked every block.
                trusted_proxies: trusted_proxies
                    .iter()
                    .filter_map(|block| Cidr::parse(block))
                    .collect(),
                max_skew: Duration::from_millis(u64::from(max_skew_ms)),
                fresh: fresh_ms.map(|ms| Duration::from_millis(u64::from(ms))),
                ..ServerSettings::default()
            };
            if let Some(held) = found.as_mut() {
                // docs/adr/0117, D4: a request is held to one step, or to none, by the server.
                if held.authenticator != authenticator
                    || held.sign != sign
                    || held.settings.fresh != declared.fresh
                {
                    return None;
                }
                let settings = &mut held.settings;
                settings.max_hold = settings.max_hold.min(declared.max_hold);
                settings.max_body_bytes = settings.max_body_bytes.min(declared.max_body_bytes);
                settings.absence = settings.absence.min(declared.absence);
                settings.mailbox = settings.mailbox.min(declared.mailbox);
                settings.max_clients = settings.max_clients.min(declared.max_clients);
                settings.max_response_bytes =
                    settings.max_response_bytes.min(declared.max_response_bytes);
                let (held_rates, declared_rates) = (settings.rates, declared.rates);
                settings.rates = EdgeRates {
                    address: stricter(held_rates.address, declared_rates.address),
                    client: stricter(held_rates.client, declared_rates.client),
                    failed_auth: stricter(held_rates.failed_auth, declared_rates.failed_auth),
                    new_names: stricter(held_rates.new_names, declared_rates.new_names),
                    max_tracked: held_rates.max_tracked.min(declared_rates.max_tracked),
                };
                // A proxy is trusted on the address only when every binding there trusts it.
                settings
                    .trusted_proxies
                    .retain(|block| declared.trusted_proxies.contains(block));
                settings.max_skew = settings.max_skew.min(declared.max_skew);
                held.peers.push(peer.name);
            } else {
                found = Some(Listening {
                    settings: declared,
                    authenticator,
                    sign,
                    peers: alloc::vec![peer.name],
                });
            }
        }
        found
    }
}

/// A host's check of a bearer token: `(client name, token)`, true when the token proves the name.
pub type BearerCheck = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// A host's check of a client certificate: `(client name, the certificate's DER)`, true when the
/// certificate (which TLS already verified against the deployment's client authorities) names
/// the client.
pub type CertificateCheck = Arc<dyn Fn(&str, &[u8]) -> bool + Send + Sync>;

/// Who may claim a name (§mesh-22.5). A binding with none is refused at deploy validation, so a
/// server is never built without one.
#[derive(Clone)]
pub enum Authenticator {
    /// `Authorization: Bearer <token>`, verified by the host; [`Authenticator::stored_tokens`]
    /// verifies the tokens a host issued into a credential store.
    Bearer(BearerCheck),
    /// `Authorization: SCE-Install <secret>`: the first secret a name brings is pinned, by its
    /// SHA-256, at the first exchange answered `200`, and no other is taken for it after. The pins
    /// are kept in the store given: [`MemoryCredentials`] forgets them when the server stops, and
    /// the next secret a name brings is pinned again; the session store's SQLite file
    /// (`SqliteStore`) keeps them across a restart.
    Install(Arc<dyn PinStore>),
    /// A TLS client certificate, required by the TLS configuration ([`server_tls`] with client
    /// authorities) and checked against the name by the host.
    Mtls(CertificateCheck),
    /// A request signed by the client's Ed25519 key (SCE_MESH.md sections 22.5 and 22.9): the
    /// name is proved by a signature its pinned key verifies, made within `max_skew` of the
    /// server's clock.
    Key(KeyPins),
}

impl Authenticator {
    /// The `bearer` authenticator whose check is `tokens` (ADR 0109, D3): a token proves a name
    /// when the credential store holds its digest for that name, not revoked and not expired.
    pub fn stored_tokens(tokens: StoredTokens) -> Self {
        Self::Bearer(Arc::new(move |name: &str, token: &str| {
            tokens.check(name, token)
        }))
    }

    /// Which authenticator it is, as the server reports a failure of it (docs/adr/0109, D16).
    pub fn kind(&self) -> AuthenticatorKind {
        match self {
            Authenticator::Bearer(_) => AuthenticatorKind::Bearer,
            Authenticator::Install(_) => AuthenticatorKind::Install,
            Authenticator::Mtls(_) => AuthenticatorKind::Mtls,
            Authenticator::Key(_) => AuthenticatorKind::Key,
        }
    }

    /// The store of pins it reads, under `install` and `key`; none under `bearer` and `mtls`,
    /// whose check is the program's.
    fn pins(&self) -> Option<&Arc<dyn PinStore>> {
        match self {
            Authenticator::Install(pins) => Some(pins),
            Authenticator::Key(keys) => Some(&keys.pins),
            Authenticator::Bearer(_) | Authenticator::Mtls(_) => None,
        }
    }
}

/// Where the `key` authenticator keeps the key each name is proved by (ADR 0109, D3 and D9): the
/// credential store's pins, a name to the `COSE_Key` of its public key as RFC 9679 takes its
/// thumbprint ([`KeyPins::pin_of`]), which names the algorithm with the key and so is never read
/// as an installation digest.
#[derive(Clone)]
pub struct KeyPins {
    pins: Arc<dyn PinStore>,
    first_use: bool,
}

impl KeyPins {
    /// Pin a name to the key that signed its first exchange answered `200` and take no other for
    /// it after (trust on first use, as `install` pins a secret). The key comes offered in
    /// `Sce-Mesh-Key`, and is taken only when it made the request's signature.
    pub fn first_use(pins: Arc<dyn PinStore>) -> Self {
        KeyPins {
            pins,
            first_use: true,
        }
    }

    /// Take only a key the host registered for the name in `pins` ([`KeyPins::pin_of`]); a name
    /// pinned to none is refused.
    pub fn registered(pins: Arc<dyn PinStore>) -> Self {
        KeyPins {
            pins,
            first_use: false,
        }
    }

    /// The pin of `public_key`, which a host that registers a client's key writes for its name
    /// (`PinStore::pin`).
    pub fn pin_of(public_key: &[u8; PUBLIC_KEY_BYTES]) -> Vec<u8> {
        cose_key_ed25519_required(public_key)
    }
}

/// When a `200` may acknowledge the envelopes an exchange carried, as the host that took them in
/// says (SCE_MESH.md section 10.11: nothing is acknowledged before a checkpoint that holds it is
/// durable). The server waits as it says, and answers nothing the host has not vouched for: the
/// three answers of §mesh-22.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// Now, once the macrosteps have run: the host's endpoint does not checkpoint, or a durable
    /// checkpoint already holds every envelope the exchange carried (a keyed pool's session, whose
    /// checkpoint is written in the run, reports this from the envelopes' receipts,
    /// [`crate::pool::Receipt`]).
    Now,
    /// Once the host reports durable a checkpoint above this number ([`HttpsServer::durable`]):
    /// the endpoint's `sealed_through()` read just before the first envelope was admitted, under
    /// the exclusion the host admits and seals under, so that every envelope admitted after it is
    /// held by a checkpoint above it. For a host whose checkpoints become durable after the call
    /// returns. One not durable within `max_hold` is answered as [`Durability::NotDurable`] is.
    After(u64),
    /// Not by this exchange: the checkpoint that would hold the envelopes was not made durable
    /// (its write failed, or a fence refused it), or the session that took them in was dropped
    /// first. The exchange is answered `503` with `Retry-After`, as load shed (§mesh-22.4): the
    /// client carries the envelopes again, the section 10.5 window drops what was taken in, and a
    /// later exchange is answered once a checkpoint holds them.
    NotDurable,
}

/// What the server asks of its host (§mesh-22.8): tell the endpoint what the binding observed,
/// run the macrosteps it caused, and return once they have run. What the machine sent meanwhile
/// is then in the mailboxes, which is what lets an exchange answer with the replies its own
/// request caused.
///
/// Called on a thread of tokio's blocking pool, with the events of one client each time.
/// Calls for different clients are made at once, so that their checkpoints can share a commit
/// (docs/adr/0109, D15): a host serves them side by side (a keyed pool runs different keys in
/// parallel) or takes a lock of its own. Calls for one client are made one at a time, in the
/// order the binding observed that client's events, so what the host is told of a client
/// arrives in the order it was decided. A closure that takes the events is a host whose
/// endpoint does not checkpoint.
pub trait ServerHost: Send + Sync + 'static {
    /// Hand `events` to the endpoint, in order ([`super::deliver`]), and run the machine; answer
    /// when a `200` may acknowledge the envelopes among them ([`Durability`]). The server reads
    /// the answer only for an exchange that carried envelopes: a poll waits for no checkpoint.
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability;
}

impl<F: Fn(Vec<LinkEvent>) + Send + Sync + 'static> ServerHost for F {
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
        self(events);
        Durability::Now
    }
}

const HELD: &str = "the server's state is never left mid-update: nothing in it panics while held";

/// One client's mailbox and presence.
struct Client {
    /// The incarnation this mailbox numbers in (section 22.3): drawn when the entry was made with
    /// its mailbox empty, so an entry removed and made again numbers anew in another.
    epoch: String,
    /// The envelopes not yet acknowledged, by number, oldest first; the numbers are consecutive.
    entries: VecDeque<(u64, Vec<u8>)>,
    /// The number the next envelope gets; the first is 1.
    next: u64,
    ready: bool,
    in_flight: usize,
    last_seen: Instant,
    arrived: Arc<Notify>,
    /// The client's order (docs/adr/0109, D15): held while its readiness is decided and the host
    /// is told, so what the host is told of this client arrives in the order it was decided,
    /// while other clients' exchanges reach the host at once.
    order: Arc<tokio::sync::Mutex<()>>,
}

impl Client {
    fn new(epoch: String) -> Self {
        Client {
            epoch,
            entries: VecDeque::new(),
            next: 1,
            ready: false,
            in_flight: 0,
            last_seen: Instant::now(),
            arrived: Arc::new(Notify::new()),
            order: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Whether the entry holds nothing the server owes or is doing for the client (docs/adr/0109,
    /// D1): its mailbox is empty, no exchange of it is in flight, it is not ready, and nothing
    /// holds or waits for its order. Such an entry is removed; one whose order is held is not,
    /// so an entry made again for the client never tells the host beside one still telling it.
    fn holds_nothing(&self) -> bool {
        self.entries.is_empty()
            && self.in_flight == 0
            && !self.ready
            && Arc::strong_count(&self.order) == 1
    }

    /// The numbered envelopes a response carries: the oldest first, as many as fit in `most`
    /// bytes, and always the first, so that an envelope larger than the bound still leaves
    /// (docs/adr/0109, D1). The rest go at the next exchange.
    fn response(&self, most: usize) -> (Option<u64>, Vec<u8>) {
        let mut body = Vec::new();
        for (_, bytes) in &self.entries {
            if !body.is_empty() && body.len().saturating_add(bytes.len()) > most {
                break;
            }
            body.extend_from_slice(bytes);
        }
        (self.entries.front().map(|(number, _)| *number), body)
    }

    /// Drop what `ack` acknowledges (§mesh-22.3), when `epoch`, the incarnation the request says
    /// its acknowledgement counts in, is `current`, the incarnation this mailbox numbers in. An
    /// acknowledgement of another incarnation, or of none, counts in a numbering this mailbox
    /// does not hold, and drops nothing.
    fn acknowledge(&mut self, ack: u64, epoch: Option<&str>, current: &str) {
        if epoch != Some(current) {
            return;
        }
        while self
            .entries
            .front()
            .is_some_and(|(number, _)| *number <= ack)
        {
            self.entries.pop_front();
        }
    }
}

/// Every client's mailbox, shared by the server and the transport the endpoint sends through.
struct Mailboxes {
    capacity: usize,
    /// The most entries held at once (deploy.yaml's `max_clients`).
    max_clients: usize,
    clients: Mutex<HashMap<String, Client>>,
    /// The outboxes of the clients whose binding is durable (docs/adr/0109, D10), which a keyed
    /// pool keeps in its store ([`HttpsServer::keep_outboxes`]): such a client's mailbox is its
    /// outbox, and the entry here keeps only its presence.
    durable: OnceLock<Arc<Outboxes>>,
}

impl Mailboxes {
    /// The outboxes, when `client`'s binding keeps a durable one.
    fn outboxes_of(&self, client: &str) -> Option<&Arc<Outboxes>> {
        self.durable.get().filter(|outboxes| outboxes.keeps(client))
    }
}

/// Why no entry could be made for a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoEntry {
    /// The server holds `max_clients` entries.
    Full,
    /// No incarnation could be drawn for the new mailbox.
    Unrandom,
}

impl NoEntry {
    /// Why an exchange that could make no entry is shed (docs/adr/0109, D16).
    fn shed(self) -> ServerShed {
        match self {
            NoEntry::Full => ServerShed::MaxClients,
            NoEntry::Unrandom => ServerShed::RandomSource,
        }
    }
}

impl Mailboxes {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Client>> {
        self.clients.lock().expect(HELD)
    }

    /// `name`'s entry in `clients`, made with a fresh incarnation when there is none and fewer
    /// than `max_clients` are held (docs/adr/0109, D1).
    fn entry<'a>(
        &self,
        clients: &'a mut HashMap<String, Client>,
        name: &str,
    ) -> Result<&'a mut Client, NoEntry> {
        let held = clients.len();
        match clients.entry(name.to_string()) {
            Entry::Occupied(found) => Ok(found.into_mut()),
            Entry::Vacant(_) if held >= self.max_clients => Err(NoEntry::Full),
            Entry::Vacant(vacant) => {
                let epoch = draw_epoch().map_err(|_| NoEntry::Unrandom)?;
                Ok(vacant.insert(Client::new(epoch)))
            }
        }
    }

    /// Remove every entry that holds nothing (docs/adr/0109, D1): what a client that made an
    /// exchange and went away cost the server ends once it is absent.
    fn sweep(&self) {
        self.lock().retain(|_, client| !client.holds_nothing());
    }
}

/// The core's side of the server: what the machine sends a client waits in its mailbox, numbered
/// in arrival order and bounded by `mailbox` (§mesh-22.3).
#[derive(Clone)]
pub struct MailboxTransport {
    mailboxes: Arc<Mailboxes>,
}

impl Transport for MailboxTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        // docs/adr/0109, D10: a durable client's mailbox is its outbox, appended to only by the
        // write that keeps the checkpoint which holds the envelope; one handed to the transport
        // would be answered from nowhere, so it is refused aloud.
        if self.mailboxes.outboxes_of(peer).is_some() {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!(
                    "the binding to {peer} keeps a durable outbox, which only a checkpoint's \
                     write appends to"
                )),
                queue_full: None,
            });
        }
        let mut clients = self.mailboxes.lock();
        let client = match self.mailboxes.entry(&mut clients, peer) {
            Ok(client) => client,
            Err(why) => {
                return Err(TransportFailure {
                    retryable: false,
                    message: Some(match why {
                        NoEntry::Full => alloc::format!(
                            "the server holds {} clients, the most it holds, and {peer} is not \
                             one of them",
                            self.mailboxes.max_clients
                        ),
                        NoEntry::Unrandom => alloc::format!(
                            "no incarnation could be drawn for the mailbox of {peer}"
                        ),
                    }),
                    queue_full: None,
                })
            }
        };
        if client.entries.len() >= self.mailboxes.capacity {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!("the mailbox of {peer} is full")),
                queue_full: Some(u32::try_from(client.entries.len()).unwrap_or(u32::MAX)),
            });
        }
        client.entries.push_back((client.next, bytes.to_vec()));
        client.next += 1;
        client.arrived.notify_waiters();
        Ok(())
    }
}

struct Shared {
    settings: ServerSettings,
    authenticator: Authenticator,
    /// The freshness step of `settings.fresh` under the key the server drew (docs/adr/0117, D4).
    fresh: Option<Freshness>,
    is_bound: Box<dyn Fn(&str) -> bool + Send + Sync>,
    host: Arc<dyn ServerHost>,
    mailboxes: Arc<Mailboxes>,
    exchanges: AtomicUsize,
    /// The last checkpoint the host reported durable ([`HttpsServer::durable`]), and the signal
    /// an exchange waiting for one wakes on.
    durable_through: AtomicU64,
    durable_reported: Notify,
    /// The connections open, and how many come from each address ([`address_key`]).
    connections: Mutex<Connections>,
    /// The edge's limiters (docs/adr/0109, D2), and the moment their clock counts from.
    edge: Mutex<Edge>,
    started: Instant,
    /// Set by [`HttpsServer::stop`] (docs/adr/0109, D10): every new exchange is answered `503`
    /// with `Retry-After`, and the signal ends the accept loop and every held exchange.
    stopping: AtomicBool,
    stopped: Notify,
    /// What the server reports to (docs/adr/0109, D16).
    metrics: Arc<dyn Metrics>,
}

impl Shared {
    /// Milliseconds since the server started: the clock the edge's limiters read.
    fn now_ms(&self) -> i64 {
        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX)
    }

    /// Load shed for `reason` (SCE_MESH.md section 22.4): `status`, a `429` or `503`, with
    /// `Retry-After` asking for `wait`, reported as a shed of its reason (docs/adr/0109, D16).
    fn shed(&self, reason: ServerShed, status: ResponseStatus, wait: Duration) -> Answer {
        self.metrics.count(Counter::ServerSheds { reason }, 1);
        shed(status, wait)
    }

    /// A `401` naming the authenticator's scheme, reported as a failure of the authenticator
    /// (docs/adr/0109, D16).
    fn unauthorized(&self) -> Answer {
        self.metrics.count(
            Counter::AuthenticationFailures {
                authenticator: self.authenticator.kind(),
            },
            1,
        );
        unauthorized(&self.authenticator)
    }

    /// The present value of the freshness step for `name` (docs/adr/0117, D4), when the server
    /// holds requests to one: under the `key` authenticator, whose signature covers it.
    fn present_value(&self, name: &str) -> Option<String> {
        match (&self.fresh, &self.authenticator) {
            (Some(fresh), Authenticator::Key(_)) => Some(fresh.value(name, unix_millis())),
            _ => None,
        }
    }

    /// The present value of the freshness step for `name` when a request of it that signs
    /// `carried` (none when it carries no value) is not taken now (docs/adr/0117, D4); `None`
    /// when it is, or when the server holds requests to no step.
    fn refusing(&self, name: &str, carried: Option<&str>) -> Option<String> {
        let now = unix_millis();
        match (&self.fresh, &self.authenticator) {
            (Some(fresh), Authenticator::Key(_)) if !fresh.admits(name, carried, now) => {
                Some(fresh.value(name, now))
            }
            _ => None,
        }
    }

    /// Ask the edge `judge` at the server's clock.
    fn edge<T>(&self, judge: impl FnOnce(&mut Edge, i64) -> T) -> T {
        let now = self.now_ms();
        judge(&mut self.edge.lock().expect(HELD), now)
    }
}

/// The connections a server holds open (docs/adr/0109, D1).
#[derive(Default)]
struct Connections {
    open: usize,
    by_address: HashMap<IpAddr, usize>,
}

/// The key a remote address is counted and limited by (docs/adr/0109, D1 and D2): an IPv4
/// address as it is, an IPv6 address by its /64, the block one subscriber is given, so a client
/// does not escape a bound by moving within its own block. An IPv4 address mapped into IPv6 is
/// the IPv4 address.
pub fn address_key(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let mut segments = v6.segments();
                segments[4..].fill(0);
                IpAddr::V6(std::net::Ipv6Addr::from(segments))
            }
        },
    }
}

/// A connection counted open, released when it closes.
struct Open {
    shared: Arc<Shared>,
    /// The address it is counted under, `None` for a trusted proxy's.
    address: Option<IpAddr>,
}

impl Open {
    /// Count a connection from `address` open, unless the server holds `max_connections`, or
    /// `max_connections_per_address` from its address ([`address_key`]): the bound it met then.
    fn admit(shared: &Arc<Shared>, address: IpAddr) -> Result<Open, ServerShed> {
        let settings = &shared.settings;
        // A trusted proxy carries many clients on its connections: it is held to the server's
        // bound alone.
        let counted =
            (!is_trusted(&settings.trusted_proxies, address)).then(|| address_key(address));
        let mut connections = shared.connections.lock().expect(HELD);
        let from_address = counted
            .and_then(|key| connections.by_address.get(&key).copied())
            .unwrap_or(0);
        if connections.open >= settings.max_connections {
            return Err(ServerShed::MaxConnections);
        }
        if counted.is_some() && from_address >= settings.max_connections_per_address {
            return Err(ServerShed::MaxConnectionsPerAddress);
        }
        connections.open += 1;
        if let Some(key) = counted {
            connections.by_address.insert(key, from_address + 1);
        }
        Ok(Open {
            shared: Arc::clone(shared),
            address: counted,
        })
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let mut connections = self.shared.connections.lock().expect(HELD);
        connections.open = connections.open.saturating_sub(1);
        let Some(address) = self.address else {
            return;
        };
        if let Entry::Occupied(mut count) = connections.by_address.entry(address) {
            *count.get_mut() -= 1;
            if *count.get() == 0 {
                count.remove();
            }
        }
    }
}

/// The server: its mailboxes, its authenticator, and the host it tells.
#[derive(Clone)]
pub struct HttpsServer {
    shared: Arc<Shared>,
}

impl HttpsServer {
    /// A server with `settings`, letting a client in by `authenticator` when `is_bound` names it
    /// (the machine has a binding to it), and telling `host` what happens. Its mailboxes start
    /// empty, each numbering in an incarnation drawn from the operating system's random source
    /// when it is made; refused when the source could not be read now.
    pub fn new(
        settings: ServerSettings,
        authenticator: Authenticator,
        is_bound: impl Fn(&str) -> bool + Send + Sync + 'static,
        host: impl ServerHost,
    ) -> Result<Self, crate::tls::Error> {
        Self::observed(
            settings,
            authenticator,
            is_bound,
            host,
            sce_rust_metrics::none(),
        )
    }

    /// [`Self::new`], reporting what the server does to `metrics` (docs/adr/0109, D16).
    pub fn observed(
        settings: ServerSettings,
        authenticator: Authenticator,
        is_bound: impl Fn(&str) -> bool + Send + Sync + 'static,
        host: impl ServerHost,
        metrics: Arc<dyn Metrics>,
    ) -> Result<Self, crate::tls::Error> {
        // Every mailbox draws its incarnation from the source: one that cannot be read refuses
        // the server now, not its first client.
        draw_epoch()?;
        // docs/adr/0117, D4: the key the values of the freshness step are made of is the server's
        // own, drawn now and given to nobody.
        let fresh = settings.fresh.map(Freshness::draw).transpose()?;
        let mailboxes = Arc::new(Mailboxes {
            capacity: settings.mailbox,
            max_clients: settings.max_clients,
            clients: Mutex::new(HashMap::new()),
            durable: OnceLock::new(),
        });
        let settings_rates = settings.rates;
        Ok(HttpsServer {
            shared: Arc::new(Shared {
                settings,
                authenticator,
                fresh,
                is_bound: Box::new(is_bound),
                host: Arc::new(host),
                mailboxes,
                exchanges: AtomicUsize::new(0),
                durable_through: AtomicU64::new(0),
                durable_reported: Notify::new(),
                connections: Mutex::new(Connections::default()),
                edge: Mutex::new(Edge::new(&settings_rates)),
                started: Instant::now(),
                stopping: AtomicBool::new(false),
                stopped: Notify::new(),
                metrics,
            }),
        })
    }

    /// What the server says of itself to the program's readiness and liveness checks
    /// (docs/adr/0109, D16): it accepts until [`Self::stop`] is called, is stopping while an
    /// exchange it took in before is still in flight, and its stores are reachable when each it
    /// reads answers a read now: the pool's store, when it answers durable clients from their
    /// outboxes ([`Self::keep_outboxes`]), and the pin store of `install` and `key`. A `bearer`
    /// or `mtls` check is the program's, which reads its own store. The reads block for as long
    /// as a store does, so a program on an asynchronous runtime calls this on its blocking pool
    /// (`tokio::task::spawn_blocking`).
    pub fn health(&self) -> Health {
        let shared = &self.shared;
        let stopping = shared.stopping.load(Ordering::SeqCst);
        let outboxes = shared
            .mailboxes
            .durable
            .get()
            .is_none_or(|outboxes| outboxes.store_answers());
        // A name no client posts under, since it is no path segment: the read is the store's
        // answer and nothing else.
        let pins = shared
            .authenticator
            .pins()
            .is_none_or(|pins| pins.pinned("").is_ok());
        Health {
            accepting: !stopping,
            stopping: stopping && shared.exchanges.load(Ordering::Acquire) > 0,
            store_reachable: outboxes && pins,
        }
    }

    /// Answer the clients whose binding is durable from `outboxes`, the outboxes a keyed pool
    /// keeps in its store (`Pool::outboxes`; docs/adr/0109, D10; SCE_MESH.md section 22.3): such
    /// a client's exchange opens its outbox, its acknowledgement deletes in the store, and its
    /// response carries what the outbox holds, numbered in the outbox's incarnation, which a
    /// restart keeps. An append wakes the client's held exchange. Given once, before the server
    /// serves; a second call is ignored.
    pub fn keep_outboxes(&self, outboxes: Arc<Outboxes>) {
        let mailboxes = Arc::clone(&self.shared.mailboxes);
        let wake: Wake = Arc::new(move |client: &str| {
            if let Some(held) = mailboxes.lock().get(client) {
                held.arrived.notify_waiters();
            }
        });
        if self
            .shared
            .mailboxes
            .durable
            .set(Arc::clone(&outboxes))
            .is_ok()
        {
            outboxes.watch(wake);
        }
    }

    /// Stop the server in order (docs/adr/0109, D10): accept no connection more, answer every
    /// new exchange `503` with `Retry-After`, as load shed (SCE_MESH.md section 22.4), and answer
    /// every held exchange at once with what its mailbox holds; then wait, at most `within`, for
    /// the exchanges in flight to end. Answers whether they all did. What an exchange in flight
    /// carried is still handed to the host, which answers it as it stands; the host stops its
    /// pool after this (`Pool::stop`).
    pub async fn stop(&self, within: Duration) -> bool {
        self.shared.stopping.store(true, Ordering::SeqCst);
        self.shared.stopped.notify_waiters();
        let until = tokio::time::Instant::now() + within;
        loop {
            if self.shared.exchanges.load(Ordering::Acquire) == 0 {
                return true;
            }
            if tokio::time::Instant::now() >= until {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// The transport the host builds its endpoint on.
    pub fn transport(&self) -> MailboxTransport {
        MailboxTransport {
            mailboxes: Arc::clone(&self.shared.mailboxes),
        }
    }

    /// The incarnation `client`'s mailbox numbers in (`Sce-Mesh-Epoch`, §mesh-22.3), while the
    /// server holds an entry for it.
    pub fn epoch_of(&self, client: &str) -> Option<String> {
        self.shared
            .mailboxes
            .lock()
            .get(client)
            .map(|held| held.epoch.clone())
    }

    /// How many client entries the server holds (docs/adr/0109, D1).
    pub fn clients(&self) -> usize {
        self.shared.mailboxes.lock().len()
    }

    /// The host reported the checkpoint `through`, and every one before it, durable to its
    /// endpoint (`Endpoint::durable`, SCE_MESH.md section 10.11): an exchange whose envelopes such
    /// a checkpoint holds is answered now. A host whose endpoint checkpoints calls this after each
    /// report, from any thread.
    pub fn durable(&self, through: u64) {
        self.shared
            .durable_through
            .fetch_max(through, Ordering::AcqRel);
        self.shared.durable_reported.notify_waiters();
    }

    /// Accept on `listener` until it fails, or until the server is stopped ([`Self::stop`]),
    /// which answers an error of kind `Interrupted`, with TLS from `tls` (or none, for a loopback
    /// test), serving each connection on a task of its own, and watch for absent clients
    /// meanwhile.
    ///
    /// A connection past `max_connections`, or past `max_connections_per_address` from its
    /// address, is closed at accept, and one whose TLS handshake takes longer than
    /// `handshake_timeout` is closed (docs/adr/0109, D1).
    pub async fn serve(
        &self,
        listener: TcpListener,
        tls: Option<Arc<rustls::ServerConfig>>,
    ) -> std::io::Error {
        let watcher = AbortOnDrop(tokio::spawn({
            let server = self.clone();
            async move { server.watch_absence().await }
        }));
        let acceptor = tls.map(tokio_rustls::TlsAcceptor::from);
        // The scheme a request's target URI is read in (`@target-uri`, SCE_MESH.md section
        // 22.9): `http` only for the loopback test that serves without TLS.
        let scheme = if acceptor.is_some() { "https" } else { "http" };
        let handshake_timeout = self.shared.settings.handshake_timeout;
        let error = loop {
            // docs/adr/0109, D10: a stop ends the accept loop; the signal is enabled before the
            // flag is read, so a stop between the two is not missed.
            let stopped = self.shared.stopped.notified();
            tokio::pin!(stopped);
            stopped.as_mut().enable();
            if self.shared.stopping.load(Ordering::SeqCst) {
                break std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "the server was stopped",
                );
            }
            let accepted = tokio::select! {
                accepted = listener.accept() => accepted,
                _ = &mut stopped => continue,
            };
            let (stream, remote) = match accepted {
                Ok(accepted) => accepted,
                Err(error) => break error,
            };
            let open = match Open::admit(&self.shared, remote.ip()) {
                Ok(open) => open,
                Err(reason) => {
                    // docs/adr/0109, D16: a connection closed at accept is a shed of its bound.
                    self.shared
                        .metrics
                        .count(Counter::ServerSheds { reason }, 1);
                    drop(stream);
                    continue;
                }
            };
            let server = self.clone();
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                // Held for as long as the connection is served.
                let _open = open;
                match acceptor {
                    Some(acceptor) => {
                        let handshake =
                            tokio::time::timeout(handshake_timeout, acceptor.accept(stream)).await;
                        let Ok(Ok(stream)) = handshake else {
                            return;
                        };
                        let certificate = stream
                            .get_ref()
                            .1
                            .peer_certificates()
                            .and_then(|chain| chain.first())
                            .map(|certificate| certificate.as_ref().to_vec());
                        server
                            .serve_from(stream, certificate, Some(remote.ip()), scheme)
                            .await;
                    }
                    None => {
                        server
                            .serve_from(stream, None, Some(remote.ip()), scheme)
                            .await
                    }
                }
            });
        };
        drop(watcher);
        error
    }

    /// Serve the exchanges of one connection the host accepted (and, for TLS, already
    /// handshook), whose client presented `certificate` (DER), until it closes. A request head
    /// that takes longer than `header_timeout` closes the connection (docs/adr/0109, D1). A host
    /// that accepts itself runs [`watch_absence`](Self::watch_absence) beside it, and bounds its
    /// connections and handshakes as [`serve`](Self::serve) does.
    ///
    /// The connection's remote address is not known here, so its exchanges are held to no
    /// limiter of the address (docs/adr/0109, D2); a host that accepts itself and knows it calls
    /// [`serve_accepted`](Self::serve_accepted).
    ///
    /// The connection is one the client reached under `https://` (the host's TLS, or a proxy's in
    /// front of it), which is the scheme a request's target URI is read in (SCE_MESH.md section
    /// 22.9).
    pub async fn serve_connection<S>(&self, stream: S, certificate: Option<Vec<u8>>)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        self.serve_from(stream, certificate, None, "https").await;
    }

    /// [`serve_connection`](Self::serve_connection) for a connection from `remote`: its
    /// exchanges are held to the edge's limits of the address (docs/adr/0109, D2), read from
    /// `Forwarded` when `remote` is a trusted proxy.
    pub async fn serve_accepted<S>(&self, stream: S, certificate: Option<Vec<u8>>, remote: IpAddr)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        self.serve_from(stream, certificate, Some(remote), "https")
            .await;
    }

    async fn serve_from<S>(
        &self,
        stream: S,
        certificate: Option<Vec<u8>>,
        remote: Option<IpAddr>,
        scheme: &'static str,
    ) where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let shared = Arc::clone(&self.shared);
        let certificate = certificate.map(Arc::new);
        let service = hyper::service::service_fn(move |request| {
            let shared = Arc::clone(&shared);
            let certificate = certificate.clone();
            async move {
                let metrics = Arc::clone(&shared.metrics);
                let answered = exchange(shared, request, certificate, remote, scheme).await;
                // docs/adr/0109, D16: every exchange the server answers, by the status of its
                // answer, counted before the answer leaves.
                metrics.count(
                    Counter::ServerExchanges {
                        status: answered.status,
                    },
                    1,
                );
                Ok::<_, Infallible>(answered.response)
            }
        });
        let mut builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
        builder
            .http1()
            .timer(TokioTimer::new())
            .header_read_timeout(self.shared.settings.header_timeout);
        let _ = builder
            .serve_connection(TokioIo::new(stream), service)
            .await;
    }

    /// Tell the host, once per edge, of every ready client whose last exchange ended longer ago
    /// than `absence` and that has none in flight: SCE_MESH.md section 16.7 row 1; then remove
    /// every entry that holds nothing (docs/adr/0109, D1). Runs until dropped. This is the
    /// server's half of readiness in §mesh-22.4: an exchange in flight counts as presence, and a
    /// mailbox that holds envelopes stays.
    pub async fn watch_absence(&self) {
        let absence = self.shared.settings.absence;
        let every = (absence / 4).clamp(Duration::from_millis(10), Duration::from_secs(1));
        let mut ticker = tokio::time::interval(every);
        let is_absent = |client: &Client| {
            client.ready && client.in_flight == 0 && client.last_seen.elapsed() >= absence
        };
        loop {
            ticker.tick().await;
            let absent: Vec<(String, Arc<tokio::sync::Mutex<()>>)> = self
                .shared
                .mailboxes
                .lock()
                .iter()
                .filter(|&(_, client)| is_absent(client))
                .map(|(name, client)| (name.clone(), Arc::clone(&client.order)))
                .collect();
            // docs/adr/0109, D15: each client's absence is decided and told under its own
            // order, decided again once the order is held, since an exchange of the client may
            // have come between; other clients' exchanges reach the host meanwhile.
            for (name, order) in absent {
                let _told = order.lock().await;
                let lost = match self.shared.mailboxes.lock().get_mut(&name) {
                    Some(client) if Arc::ptr_eq(&client.order, &order) && is_absent(client) => {
                        client.ready = false;
                        true
                    }
                    _ => false,
                };
                if lost {
                    let reason =
                        alloc::format!("no exchange for more than {} ms", absence.as_millis());
                    tell(&self.shared, alloc::vec![LinkEvent::Lost(name, reason)]).await;
                }
            }
            // An entry whose order is held or waited for is not removed (`holds_nothing`), so
            // none is removed between an exchange deciding its client ready and the host being
            // told.
            self.shared.mailboxes.sweep();
        }
    }
}

/// Aborts a task when dropped, so that what [`HttpsServer::serve`] started ends with it.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Hand `events` to the host on the blocking pool, and wait until it has run them. Answers what
/// the host's [`ServerHost::deliver`] said of the envelopes among them.
async fn tell(shared: &Arc<Shared>, events: Vec<LinkEvent>) -> Durability {
    let host = Arc::clone(&shared.host);
    // A host that panicked has left its endpoint in a state nobody can read, so nothing says
    // whether the envelopes it was handed were taken, let alone made durable (SCE_MESH.md
    // §10.11): the exchange is shed as one the host could not make durable, the sender carries
    // the envelopes again, and the mailbox keeps what it holds for the next exchange. The panic
    // is the host's to see in its own log.
    tokio::task::spawn_blocking(move || host.deliver(events))
        .await
        .unwrap_or(Durability::NotDurable)
}

/// Wait until the host reported durable a checkpoint above `sealed`, at most `bound`. Whether it
/// did.
async fn durable_past(shared: &Shared, sealed: u64, bound: Duration) -> bool {
    let until = tokio::time::Instant::now() + bound;
    loop {
        let reported = shared.durable_reported.notified();
        tokio::pin!(reported);
        reported.as_mut().enable();
        if shared.durable_through.load(Ordering::Acquire) > sealed {
            return true;
        }
        if tokio::time::Instant::now() >= until {
            return false;
        }
        tokio::select! {
            _ = &mut reported => {}
            _ = tokio::time::sleep_until(until) => {}
        }
    }
}

/// Counts an exchange held open in `sce_server_held_exchanges` for as long as it lives
/// (docs/adr/0109, D16), however the hold ends: answered, or dropped with its connection.
struct Holding(Arc<dyn Metrics>);

impl Holding {
    fn new(metrics: Arc<dyn Metrics>) -> Self {
        metrics.adjust(Gauge::HeldExchanges, 1);
        Holding(metrics)
    }
}

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.adjust(Gauge::HeldExchanges, -1);
    }
}

/// Counts an exchange in flight, server-wide, for as long as it lives.
struct Counted<'a>(&'a AtomicUsize);

impl Drop for Counted<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Marks a client's exchange in flight for as long as it lives, and its end as the client's last
/// sight.
struct InFlight {
    mailboxes: Arc<Mailboxes>,
    name: String,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        let mut clients = self.mailboxes.lock();
        if let Some(client) = clients.get_mut(&self.name) {
            client.in_flight = client.in_flight.saturating_sub(1);
            client.last_seen = Instant::now();
        }
    }
}

/// The answer to an exchange: its response, built from one of the statuses the server sends
/// ([`ResponseStatus`]), so the status it is reported by (docs/adr/0109, D16) is the one it
/// carries and no other.
struct Answer {
    status: ResponseStatus,
    response: Response<Full<Bytes>>,
}

/// The HTTP status code of `status`.
fn status_code(status: ResponseStatus) -> StatusCode {
    match status {
        ResponseStatus::Ok => StatusCode::OK,
        ResponseStatus::BadRequest => StatusCode::BAD_REQUEST,
        ResponseStatus::Unauthorized => StatusCode::UNAUTHORIZED,
        ResponseStatus::NotFound => StatusCode::NOT_FOUND,
        ResponseStatus::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
        ResponseStatus::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
        ResponseStatus::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ResponseStatus::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ResponseStatus::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
        ResponseStatus::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// An answer of `status` with an empty body.
fn answer(status: ResponseStatus) -> Answer {
    let mut response = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = status_code(status);
    Answer { status, response }
}

/// Load shed (§mesh-22.4): `status`, a `429` or `503`, with `Retry-After` asking for `wait`,
/// rounded up to a whole second, which the client waits at least before it sends what the request
/// carried again.
fn shed(status: ResponseStatus, wait: Duration) -> Answer {
    let mut answered = answer(status);
    let seconds = retry_seconds(wait).to_string();
    if let Ok(value) = HeaderValue::from_str(&seconds) {
        answered.response.headers_mut().insert("Retry-After", value);
    }
    answered
}

fn unauthorized(authenticator: &Authenticator) -> Answer {
    let mut answered = answer(ResponseStatus::Unauthorized);
    // RFC 9110 section 15.5.2: a 401 names the scheme it asks for. A certificate is asked for by
    // TLS, not by a header, and is named here by the binding's own word.
    let scheme = match authenticator {
        Authenticator::Bearer(_) => BEARER_SCHEME,
        Authenticator::Install(_) => INSTALL_SCHEME,
        Authenticator::Mtls(_) => "SCE-Certificate",
        Authenticator::Key(_) => KEY_SCHEME,
    };
    answered
        .response
        .headers_mut()
        .insert("WWW-Authenticate", HeaderValue::from_static(scheme));
    answered
}

/// docs/adr/0117, D4: the refusal of a request for its freshness, `401` with the `key`
/// authenticator's scheme and the error `use_fresh` (as RFC 9449, section 9, refuses a request
/// without the nonce it was given), giving the present value `present` in `Sce-Mesh-Fresh`.
/// It changes what the client signs, so under `sign:` it is signed for the request it answers
/// as a `200` is.
fn stale(settings: &ServerSettings, present: &str, answered: Option<&Answered>) -> Answer {
    let mut refused = answer(ResponseStatus::Unauthorized);
    let headers = refused.response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&alloc::format!("{KEY_SCHEME} error=\"{USE_FRESH}\""))
    {
        headers.insert("WWW-Authenticate", value);
    }
    if let Ok(value) = HeaderValue::from_str(present) {
        headers.insert(FRESH_HEADER, value);
    }
    if let (Some(signer), Some(answered)) = (&settings.signer, answered) {
        let digest = content_digest(&[]);
        let fields = signer.sign_refusal(&digest, present, answered);
        for (name, value) in
            core::iter::once((CONTENT_DIGEST_HEADER.to_string(), digest)).chain(fields)
        {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                headers.insert(name, value);
            }
        }
    }
    refused
}

/// The client name a path posts under, when it is `<base>/sce-mesh/<version>/<name>` with the
/// version this server speaks; `None` is a `404`. The path is the only place the server learns
/// the client's name from (§mesh-22.1).
fn client_of<'a>(base_path: &str, path: &'a str) -> Option<&'a str> {
    let rest = path.strip_prefix(base_path)?.strip_prefix(PATH_ROOT)?;
    let (version, name) = rest.split_once('/')?;
    (version == VERSION && is_path_segment(name)).then_some(name)
}

/// A request header read as a number: absent is 0, anything but digits is `None` (a `400`, as
/// §mesh-22.1 answers a binding header that is not a number).
fn number_header(request: &Request<Incoming>, name: &str) -> Option<u64> {
    match request.headers().get(name) {
        None => Some(0),
        Some(value) => value.to_str().ok()?.trim().parse().ok(),
    }
}

/// The incarnation a request's acknowledgement counts in (`Sce-Mesh-Epoch`): `Some(None)` when
/// the request names none, `None` (a `400`) when the header is not an incarnation.
fn epoch_header(request: &Request<Incoming>) -> Option<Option<String>> {
    match request.headers().get(EPOCH_HEADER) {
        None => Some(None),
        Some(value) => {
            let text = value.to_str().ok()?.trim();
            is_epoch(text).then(|| Some(text.to_string()))
        }
    }
}

/// The credential text after `scheme` in the `Authorization` header.
fn credential<'a>(request: &'a Request<Incoming>, scheme: &str) -> Option<&'a str> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (given, text) = value.split_once(' ')?;
    given.eq_ignore_ascii_case(scheme).then(|| text.trim())
}

/// The value of the request's field `name` as a signature covers it (RFC 9421 section 2.1):
/// every line of it, each trimmed, joined by `", "`. `None` when there is none, or a line is not
/// text.
fn header_field(request: &Request<Incoming>, name: &str) -> Option<String> {
    let lines: Option<Vec<&str>> = request
        .headers()
        .get_all(name)
        .iter()
        .map(|value| value.to_str().ok().map(str::trim))
        .collect();
    lines
        .filter(|lines| !lines.is_empty())
        .map(|lines| lines.join(", "))
}

/// The request's target URI as both ends of the binding read it (`@target-uri`, SCE_MESH.md
/// section 22.9): its own scheme and authority when it names them (HTTP/2, or the absolute form),
/// else `scheme` and `Host`, with its path and query.
fn request_target(request: &Request<Incoming>, scheme: &str) -> Option<String> {
    let uri = request.uri();
    let authority = match uri.authority() {
        Some(authority) => authority.as_str().to_string(),
        None => request
            .headers()
            .get(HOST)?
            .to_str()
            .ok()?
            .trim()
            .to_string(),
    };
    let path_and_query = uri.path_and_query().map_or("/", |path| path.as_str());
    target_uri(
        uri.scheme_str().unwrap_or(scheme),
        &authority,
        path_and_query,
    )
}

/// The milliseconds since the Unix epoch on the server's clock: what an outbox is opened and
/// acknowledged at.
fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// The seconds since the Unix epoch on the server's clock, which a request's `created` is held
/// to.
fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// What the authenticator makes of a request, before its body is read: refused, admitted with
/// the pin still to write, or not judged because the pin store could not be read.
enum Admission {
    Refused,
    /// Admitted; `pin` is what the name is still to be pinned to (an installation digest, or a
    /// key), and `new` says the name is pinned to none yet.
    Admitted {
        pin: Option<Vec<u8>>,
        new: bool,
    },
    /// The pin store failed: the client is not at fault, and is answered as load shed is.
    Unavailable,
}

/// Where a request is judged: what the server reads of it beyond its headers.
struct Judged<'a> {
    /// The client's certificate, DER, under `mtls`.
    certificate: Option<&'a [u8]>,
    /// The request's target URI, which a request signature covers.
    target: Option<&'a str>,
    /// How far a request signature's `created` may lie from the server's clock.
    max_skew: Duration,
}

fn admit(
    authenticator: &Authenticator,
    name: &str,
    request: &Request<Incoming>,
    judged: &Judged<'_>,
) -> Admission {
    // §mesh-22.5: the name the path claims is proved by the deployment's one authenticator, a
    // bearer token, an installation secret checked against its pin, a client certificate, or a
    // signature of the key the name is pinned to.
    let certificate = judged.certificate;
    match authenticator {
        Authenticator::Bearer(check) => match credential(request, BEARER_SCHEME) {
            Some(token) if check(name, token) => Admission::Admitted {
                pin: None,
                new: false,
            },
            _ => Admission::Refused,
        },
        Authenticator::Install(pins) => {
            let Some(secret) = credential(request, INSTALL_SCHEME).and_then(decode_install_secret)
            else {
                return Admission::Refused;
            };
            let digest = install_digest(&secret);
            match pins.pinned(name) {
                Ok(Some(held)) if !same_secret(&held, &digest) => Admission::Refused,
                Ok(held) => Admission::Admitted {
                    pin: Some(digest.to_vec()),
                    new: held.is_none(),
                },
                Err(_) => Admission::Unavailable,
            }
        }
        Authenticator::Mtls(check) => match certificate {
            Some(certificate) if check(name, certificate) => Admission::Admitted {
                pin: None,
                new: false,
            },
            _ => Admission::Refused,
        },
        Authenticator::Key(keys) => admit_key(keys, name, request, judged),
    }
}

/// The `key` authenticator's judgment of a request (SCE_MESH.md section 22.9): a signature of the
/// profile, by the key the name is pinned to, or, for a name pinned to none under trust on first
/// use, by the key the request offers, made within `max_skew` of the server's clock. The body is
/// held to its signed `Content-Digest` once it has arrived.
fn admit_key(
    keys: &KeyPins,
    name: &str,
    request: &Request<Incoming>,
    judged: &Judged<'_>,
) -> Admission {
    // §mesh-22.9: a signature of the profile by the key the name is pinned to, or under trust on
    // first use by the key the request offers, made within `max_skew` of the server's clock.
    let field = |header: &str| header_field(request, header);
    let (Ok(signature), Some(target)) = (RequestSignature::read(&field), judged.target) else {
        return Admission::Refused;
    };
    let pinned = match keys.pins.pinned(name) {
        Ok(pinned) => pinned,
        Err(_) => return Admission::Unavailable,
    };
    let (public_key, new) = match pinned {
        // A pin this authenticator did not write (an installation digest) names no key.
        Some(pin) => match ed25519_of_cose_key(&pin) {
            Some(public_key) => (public_key, false),
            None => return Admission::Refused,
        },
        None if keys.first_use => match field(KEY_HEADER).and_then(|offer| key_of_offer(&offer)) {
            Some(public_key) => (public_key, true),
            None => return Admission::Refused,
        },
        None => return Admission::Refused,
    };
    let max_skew_s = judged.max_skew.as_secs();
    match signature.verify(&public_key, target, &field, unix_seconds(), max_skew_s) {
        Ok(()) => Admission::Admitted {
            pin: new.then(|| KeyPins::pin_of(&public_key)),
            new,
        },
        Err(_) => Admission::Refused,
    }
}

/// One exchange, in the order §mesh-22.3 gives: refused if a status of §mesh-22.4 says so, the
/// acknowledgement taken, the request's envelopes taken in, then answered with what the mailbox
/// holds.
async fn exchange(
    shared: Arc<Shared>,
    request: Request<Incoming>,
    certificate: Option<Arc<Vec<u8>>>,
    remote: Option<IpAddr>,
    scheme: &'static str,
) -> Answer {
    // docs/adr/0109, D10: a server that is stopping takes nothing in; the client carries what it
    // sent again, to the server that serves next.
    if shared.stopping.load(Ordering::SeqCst) {
        return shared.shed(
            ServerShed::Stopping,
            ResponseStatus::ServiceUnavailable,
            shared.settings.retry_after,
        );
    }
    // §mesh-22.4: the refusals below (`429`, `405`, `404`, `503`, `401`, `400`, `413`, `415`)
    // are each answered before the request's envelopes reach the core.
    //
    // docs/adr/0109, D2: every exchange is held to the rate of its remote address, read from
    // `Forwarded` only on a connection from a trusted proxy, before anything else is read of it.
    let address = remote.map(|peer| {
        let forwarded = request
            .headers()
            .get_all("forwarded")
            .iter()
            .filter_map(|value| value.to_str().ok());
        address_key(client_address(
            peer,
            forwarded,
            &shared.settings.trusted_proxies,
        ))
    });
    if let Verdict::Refused { wait } = shared.edge(|edge, now| edge.exchange(address, now)) {
        return shared.shed(ServerShed::Rate, ResponseStatus::TooManyRequests, wait);
    }
    if request.method() != Method::POST {
        let mut answered = answer(ResponseStatus::MethodNotAllowed);
        answered
            .response
            .headers_mut()
            .insert("Allow", HeaderValue::from_static("POST"));
        return answered;
    }
    let settings = &shared.settings;
    let Some(name) = client_of(&settings.base_path, request.uri().path()) else {
        return answer(ResponseStatus::NotFound);
    };
    let name = name.to_string();
    if !(shared.is_bound)(&name) {
        return answer(ResponseStatus::NotFound);
    }
    if shared.exchanges.fetch_add(1, Ordering::AcqRel) >= settings.max_exchanges {
        shared.exchanges.fetch_sub(1, Ordering::AcqRel);
        return shared.shed(
            ServerShed::Concurrency,
            ResponseStatus::ServiceUnavailable,
            settings.retry_after,
        );
    }
    let _counted = Counted(&shared.exchanges);
    // docs/adr/0109, D2: while the failed authentications of the name or of the address are past
    // their rate, the authenticator is not run, so a guess costs the guesser the rate and the
    // server no check.
    if let Verdict::Refused { wait } =
        shared.edge(|edge, now| edge.may_authenticate(&name, address, now))
    {
        return shared.shed(ServerShed::Rate, ResponseStatus::TooManyRequests, wait);
    }
    // §mesh-22.9: what a signature covers of the request, read before its body
    // takes it: the target URI, and the acknowledgement and nonce a response's signature answers.
    let target = request_target(&request, scheme);
    let answered = match (
        &target,
        header_field(&request, ACK_HEADER),
        header_field(&request, NONCE_HEADER),
    ) {
        (Some(target), Some(ack), Some(nonce)) => Some(Answered {
            target_uri: target.clone(),
            ack,
            nonce,
        }),
        _ => None,
    };
    let signed_digest = header_field(&request, CONTENT_DIGEST_HEADER);
    let judged = Judged {
        certificate: certificate.as_deref().map(Vec::as_slice),
        target: target.as_deref(),
        max_skew: settings.max_skew,
    };
    let (pin, new) = match admit(&shared.authenticator, &name, &request, &judged) {
        Admission::Refused => {
            shared.edge(|edge, now| edge.failed(&name, address, now));
            return shared.unauthorized();
        }
        Admission::Admitted { pin, new } => (pin, new),
        // A store that failed is no failed authentication of the client's: it counts against no
        // rate.
        Admission::Unavailable => {
            return shared.shed(
                ServerShed::Store,
                ResponseStatus::ServiceUnavailable,
                settings.retry_after,
            )
        }
    };
    if let Verdict::Refused { wait } = shared.edge(|edge, now| edge.client(&name, now)) {
        return shared.shed(ServerShed::Rate, ResponseStatus::TooManyRequests, wait);
    }
    // docs/adr/0117, D4: under the `key` authenticator a request signs a value of the freshness
    // step this server gave the name, of the present step or of the one before. One that signs an
    // older value, or none, is refused before anything of it is taken in (no pin, no body), and
    // given the present value to sign again. Its signature verified, so it is no failed
    // authentication and counts against no rate of failures.
    if let Some(present) = shared.refusing(&name, header_field(&request, FRESH_HEADER).as_deref()) {
        return stale(settings, &present, answered.as_ref());
    }
    // A name pinned for the first time is held to the rate at which its address pins new names.
    if new {
        if let Verdict::Refused { wait } = shared.edge(|edge, now| edge.new_name(address, now)) {
            return shared.shed(ServerShed::Rate, ResponseStatus::TooManyRequests, wait);
        }
    }
    let (Some(ack), Some(hold_ms), Some(acked_in)) = (
        number_header(&request, ACK_HEADER),
        number_header(&request, HOLD_HEADER),
        epoch_header(&request),
    ) else {
        return answer(ResponseStatus::BadRequest);
    };
    let declared = request
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if declared.is_some_and(|length| length > settings.max_body_bytes as u64) {
        return answer(ResponseStatus::PayloadTooLarge);
    }
    let typed = request
        .headers()
        .get(CONTENT_TYPE_NAME)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        });

    // The client's order is taken from the entry the exchange holds in flight, so it is the one
    // every other exchange of the client and its absence are told under.
    let (in_flight, order) = {
        let mut clients = shared.mailboxes.lock();
        // docs/adr/0109, D1: a name past `max_clients` is load shed, and so is one whose mailbox
        // could draw no incarnation.
        let client = match shared.mailboxes.entry(&mut clients, &name) {
            Ok(client) => client,
            Err(why) => {
                return shared.shed(
                    why.shed(),
                    ResponseStatus::ServiceUnavailable,
                    settings.retry_after,
                )
            }
        };
        if client.in_flight >= settings.max_exchanges_per_client {
            return shared.shed(
                ServerShed::Concurrency,
                ResponseStatus::TooManyRequests,
                settings.retry_after,
            );
        }
        client.in_flight += 1;
        client.last_seen = Instant::now();
        (
            InFlight {
                mailboxes: Arc::clone(&shared.mailboxes),
                name: name.clone(),
            },
            Arc::clone(&client.order),
        )
    };

    // docs/adr/0109, D1: a body that takes longer than `body_timeout` to arrive whole is answered
    // `408` (RFC 9110, section 15.5.9), and its connection is free again.
    let collected = tokio::time::timeout(
        settings.body_timeout,
        Limited::new(request.into_body(), settings.max_body_bytes).collect(),
    )
    .await;
    let body = match collected {
        Err(_) => return answer(ResponseStatus::RequestTimeout),
        Ok(Ok(collected)) => collected.to_bytes(),
        Ok(Err(error)) if error.downcast_ref::<LengthLimitError>().is_some() => {
            return answer(ResponseStatus::PayloadTooLarge);
        }
        // The connection ended or broke before the body was whole; nobody reads the answer.
        Ok(Err(_)) => return answer(ResponseStatus::BadRequest),
    };
    if !body.is_empty() && typed.as_deref() != Some(CONTENT_TYPE) {
        return answer(ResponseStatus::UnsupportedMediaType);
    }
    // §mesh-22.9: a signed request proves its body by the `Content-Digest` its
    // signature covers, so a body that is not the one signed is no request of the client's.
    if matches!(shared.authenticator, Authenticator::Key(_))
        && !signed_digest.is_some_and(|digest| digest_matches(&digest, &body))
    {
        shared.edge(|edge, now| edge.failed(&name, address, now));
        return shared.unauthorized();
    }
    // §mesh-22.5: the name is pinned to the installation secret's SHA-256, or to the key whose
    // signature proved it, in one step with the check, so of two first exchanges that bring
    // different secrets or keys one is refused `401`.
    if let Some(wanted) = pin {
        if let Some(pins) = shared.authenticator.pins() {
            match pins.pin(&name, &wanted) {
                Ok(held) if same_secret(&held, &wanted) => {}
                Ok(_) => {
                    shared.edge(|edge, now| edge.failed(&name, address, now));
                    return shared.unauthorized();
                }
                Err(_) => {
                    return shared.shed(
                        ServerShed::Store,
                        ResponseStatus::ServiceUnavailable,
                        settings.retry_after,
                    )
                }
            }
        }
    }

    // docs/adr/0109, D10: a durable client's mailbox is its outbox in the store, opened at its
    // exchange (in an incarnation drawn now when the store holds none for it) and acknowledged
    // there before the request's envelopes are taken in, the order of section 22.3. A store that
    // cannot be read or written is answered as load shed: the client is not at fault, and sends
    // again.
    // Named apart from the checkpoint's verdict below, which says whether the exchange's
    // envelopes are durable; this one is the outboxes the client is answered from.
    //
    // docs/adr/0109, D15: the acknowledgement is committed through the pool's committer with the
    // checkpoints handed meanwhile, and waits for that commit, so it runs on the blocking pool
    // rather than on the runtime's workers.
    let durable_outboxes = shared.mailboxes.outboxes_of(&name).cloned();
    if let Some(outboxes) = &durable_outboxes {
        let now = unix_millis();
        let Ok(drawn) = draw_epoch() else {
            return shared.shed(
                ServerShed::RandomSource,
                ResponseStatus::ServiceUnavailable,
                settings.retry_after,
            );
        };
        let outboxes = Arc::clone(outboxes);
        let client = name.clone();
        let acked_in = acked_in.clone();
        let kept = tokio::task::spawn_blocking(move || {
            let Ok(incarnation) = outboxes.open(&client, &drawn, now) else {
                return false;
            };
            let acknowledges = ack > 0 && acked_in.as_deref() == Some(incarnation.as_str());
            !acknowledges
                || outboxes
                    .acknowledge(&client, &incarnation, ack, now)
                    .is_ok()
        })
        .await
        .unwrap_or(false);
        if !kept {
            return shared.shed(
                ServerShed::Store,
                ResponseStatus::ServiceUnavailable,
                settings.retry_after,
            );
        }
    }

    // Taken in: the acknowledgement, readiness and the envelopes, told in the order decided
    // (§mesh-22.3), under the client's own order (docs/adr/0109, D15): the exchanges of other
    // clients reach the host at once. The body is split as §mesh-22.2 frames it, a CBOR sequence
    // whose unreadable tail reaches the core as one last envelope.
    let (arrived, held_by) = {
        let telling = order.lock().await;
        let (became_ready, arrived) = {
            let mut clients = shared.mailboxes.lock();
            // The exchange is in flight, so its entry is held: no sweep removes it.
            let client = match shared.mailboxes.entry(&mut clients, &name) {
                Ok(client) => client,
                Err(why) => {
                    return shared.shed(
                        why.shed(),
                        ResponseStatus::ServiceUnavailable,
                        settings.retry_after,
                    )
                }
            };
            let current = client.epoch.clone();
            client.acknowledge(ack, acked_in.as_deref(), &current);
            let became_ready = !client.ready;
            client.ready = true;
            (became_ready, Arc::clone(&client.arrived))
        };
        let envelopes = split_sequence(&body);
        let carried = !envelopes.is_empty();
        let mut events = Vec::new();
        if became_ready {
            events.push(LinkEvent::Ready(name.clone()));
        }
        events.extend(
            envelopes
                .into_iter()
                .map(|envelope| LinkEvent::Received(name.clone(), envelope)),
        );
        let durability = if events.is_empty() {
            Durability::Now
        } else {
            tell(&shared, events).await
        };
        drop(telling);
        // Only what the request carried waits for a checkpoint: a poll took nothing in.
        (arrived, if carried { durability } else { Durability::Now })
    };

    // A checkpointing host's envelopes are acknowledged by a `200` only once a durable checkpoint
    // holds them (SCE_MESH.md section 10.11). Ones the host could not make durable, or that are
    // not durable within the longest hold, are answered as load shed: the client carries them
    // again, and the duplicate window drops what was taken.
    let durable = match held_by {
        Durability::Now => true,
        Durability::After(sealed) => durable_past(&shared, sealed, settings.max_hold).await,
        Durability::NotDurable => false,
    };
    if !durable {
        return shared.shed(
            ServerShed::Durability,
            ResponseStatus::ServiceUnavailable,
            settings.retry_after,
        );
    }

    // Held while the mailbox is empty, for at most the shorter of the two bounds: the long poll
    // of §mesh-22.3, answered as soon as an envelope arrives.
    let hold = Duration::from_millis(hold_ms).min(settings.max_hold);
    let until = tokio::time::Instant::now() + hold;
    // docs/adr/0109, D16: counted held from its first wait until it is answered, or dropped
    // with its connection.
    let mut holding: Option<Holding> = None;
    loop {
        let notified = arrived.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        // docs/adr/0109, D10: a stop answers a held exchange at once, with what its mailbox
        // holds.
        let stopped = shared.stopped.notified();
        tokio::pin!(stopped);
        stopped.as_mut().enable();
        let waiting = match &durable_outboxes {
            Some(outboxes) => outboxes.holds(&name),
            None => shared
                .mailboxes
                .lock()
                .get(&name)
                .is_some_and(|client| !client.entries.is_empty()),
        };
        if waiting || shared.stopping.load(Ordering::SeqCst) || tokio::time::Instant::now() >= until
        {
            break;
        }
        if holding.is_none() {
            holding = Some(Holding::new(Arc::clone(&shared.metrics)));
        }
        tokio::select! {
            _ = &mut notified => {}
            _ = &mut stopped => {}
            _ = tokio::time::sleep_until(until) => {}
        }
    }
    drop(holding);

    // At most `max_response_bytes` of the mailbox (docs/adr/0109, D1); the rest at the next
    // exchange, which the numbering of §mesh-22.3 allows. A durable client's is its outbox
    // (docs/adr/0109, D10), numbered in the outbox's incarnation.
    let (epoch, first, body) = match &durable_outboxes {
        Some(outboxes) => match outboxes.response(&name, settings.max_response_bytes) {
            Some(carried) => (Some(carried.incarnation), carried.first, carried.body),
            None => (None, None, Vec::new()),
        },
        None => {
            let clients = shared.mailboxes.lock();
            match clients.get(&name) {
                Some(client) => {
                    let (first, body) = client.response(settings.max_response_bytes);
                    (Some(client.epoch.clone()), first, body)
                }
                None => (None, None, Vec::new()),
            }
        }
    };
    drop(in_flight);
    let Some(epoch) = epoch else {
        // The entry an exchange in flight holds is never removed, so a mailbox of the server's
        // own always numbers; a durable client's outbox the process let go (another process
        // appended to it) is read again at the next exchange. A response naming no incarnation
        // would number nothing, so this one is shed as a store not read.
        return shared.shed(
            ServerShed::Store,
            ResponseStatus::ServiceUnavailable,
            settings.retry_after,
        );
    };
    // §mesh-22.2: the body is the unacknowledged envelopes it carries as a CBOR sequence,
    // `Sce-Mesh-First` numbers its first (absent when it is empty), and every `200` names its
    // incarnation.
    // §mesh-22.9: under `sign:`, a `200` answering a request that carries a nonce is
    // signed over its status, its body's digest and its numbering, and over the request's method,
    // target, acknowledgement and nonce, so that it answers that request alone.
    let first = first.map(|first| first.to_string());
    // docs/adr/0117, D4: every `200` gives the present value of the freshness step, which the
    // client's next request signs, and is signed over it.
    let fresh = shared.present_value(&name);
    let signed = match (&settings.signer, &answered) {
        (Some(signer), Some(answered)) => {
            let digest = content_digest(&body);
            let fields = signer.sign(
                &digest,
                &epoch,
                first.as_deref(),
                fresh.as_deref(),
                answered,
            );
            Some((digest, fields))
        }
        _ => None,
    };
    let mut answered = answer(ResponseStatus::Ok);
    let response = &mut answered.response;
    *response.body_mut() = Full::new(Bytes::from(body));
    response
        .headers_mut()
        .insert(CONTENT_TYPE_NAME, HeaderValue::from_static(CONTENT_TYPE));
    if let Ok(value) = HeaderValue::from_str(&epoch) {
        response.headers_mut().insert(EPOCH_HEADER, value);
    }
    if let Some(first) = &first {
        if let Ok(value) = HeaderValue::from_str(first) {
            response.headers_mut().insert(FIRST_HEADER, value);
        }
    }
    if let Some(fresh) = &fresh {
        if let Ok(value) = HeaderValue::from_str(fresh) {
            response.headers_mut().insert(FRESH_HEADER, value);
        }
    }
    if let Some((digest, fields)) = signed {
        let digest = core::iter::once((CONTENT_DIGEST_HEADER.to_string(), digest));
        for (name, value) in digest.chain(fields) {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                response.headers_mut().insert(name, value);
            }
        }
    }
    answered
}

/// A TLS configuration for the server: `chain` (each certificate DER, the server's own first)
/// with `key` (DER: PKCS#8, PKCS#1 or SEC1), offering HTTP/2 and HTTP/1.1, on the cryptography
/// every binding runs on ([`crate::tls::provider`]). With `client_authorities` (each DER) every
/// client must present a certificate one of them issued, which is the `mtls` authenticator's
/// half that TLS keeps.
pub fn server_tls(
    chain: Vec<Vec<u8>>,
    key: Vec<u8>,
    client_authorities: Option<Vec<Vec<u8>>>,
) -> Result<Arc<rustls::ServerConfig>, crate::tls::Error> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use rustls::{RootCertStore, ServerConfig};

    let key =
        PrivateKeyDer::try_from(key).map_err(|why| rustls::Error::General(why.to_string()))?;
    let chain: Vec<CertificateDer<'static>> = chain.into_iter().map(CertificateDer::from).collect();
    let builder = ServerConfig::builder_with_provider(crate::tls::provider())
        .with_safe_default_protocol_versions()?;
    let mut config = match client_authorities {
        None => builder.with_no_client_auth().with_single_cert(chain, key)?,
        Some(authorities) => {
            let mut roots = RootCertStore::empty();
            for authority in authorities {
                roots.add(CertificateDer::from(authority))?;
            }
            let verifier = WebPkiClientVerifier::builder_with_provider(
                Arc::new(roots),
                crate::tls::provider(),
            )
            .build()
            .map_err(|why| rustls::Error::General(why.to_string()))?;
            builder
                .with_client_cert_verifier(verifier)
                .with_single_cert(chain, key)?
        }
    };
    config.alpn_protocols = alloc::vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_is_named_by_the_last_segment_of_its_versioned_path() {
        assert_eq!(client_of("", "/sce-mesh/1/phone"), Some("phone"));
        assert_eq!(client_of("/api", "/api/sce-mesh/1/phone"), Some("phone"));
        assert_eq!(client_of("", "/sce-mesh/2/phone"), None, "another version");
        assert_eq!(client_of("", "/sce-mesh/1/a/b"), None, "not one segment");
        assert_eq!(client_of("", "/sce-mesh/1/"), None, "no name");
        assert_eq!(
            client_of("/api", "/sce-mesh/1/phone"),
            None,
            "not under the base"
        );
    }

    const EPOCH: &str = "00112233445566778899aabbccddeeff";
    const EARLIER: &str = "ffeeddccbbaa99887766554433221100";

    /// An acknowledgement drops what it names, and only when it counts in the incarnation the
    /// mailbox numbers in: one of another incarnation, or of none, names a numbering the mailbox
    /// does not hold.
    #[test]
    fn an_acknowledgement_drops_what_it_names_in_this_incarnation_and_no_more() {
        let mut client = Client::new(EPOCH.to_string());
        for n in 0..5u8 {
            client.entries.push_back((client.next, alloc::vec![n]));
            client.next += 1;
        }
        client.acknowledge(3, None, EPOCH);
        assert_eq!(
            client.entries.len(),
            5,
            "a client that has seen no incarnation"
        );
        client.acknowledge(3, Some(EARLIER), EPOCH);
        assert_eq!(client.entries.len(), 5, "an earlier incarnation's numbers");
        client.acknowledge(3, Some(EPOCH), EPOCH);
        assert_eq!(client.entries.front().map(|(n, _)| *n), Some(4));
        client.acknowledge(5, Some(EPOCH), EPOCH);
        assert!(client.entries.is_empty());
        assert_eq!(client.next, 6, "numbers go on where they were");
    }

    #[test]
    fn a_full_mailbox_refuses_as_a_full_queue_with_its_depth() {
        let mailboxes = Arc::new(Mailboxes {
            capacity: 2,
            max_clients: 2,
            clients: Mutex::new(HashMap::new()),
            durable: OnceLock::new(),
        });
        let mut transport = MailboxTransport {
            mailboxes: Arc::clone(&mailboxes),
        };
        transport.transmit("phone", &[1]).unwrap();
        transport.transmit("phone", &[2]).unwrap();
        let refused = transport.transmit("phone", &[3]).unwrap_err();
        assert_eq!(refused.queue_full, Some(2));
        assert!(!refused.retryable);
        // Another client's mailbox is its own.
        transport.transmit("tablet", &[1]).unwrap();
        let clients = mailboxes.lock();
        let numbers: Vec<u64> = clients["phone"].entries.iter().map(|(n, _)| *n).collect();
        assert_eq!(numbers, [1, 2]);
        assert_ne!(
            clients["phone"].epoch, clients["tablet"].epoch,
            "each mailbox draws its own incarnation"
        );
    }

    /// docs/adr/0109, D1: at `max_clients` entries a send to a client the server holds none for
    /// is refused, not queued, and makes no entry. Red if the transport makes an entry past the
    /// bound.
    #[test]
    fn a_send_to_a_client_past_max_clients_makes_no_entry() {
        let mailboxes = Arc::new(Mailboxes {
            capacity: 4,
            max_clients: 1,
            clients: Mutex::new(HashMap::new()),
            durable: OnceLock::new(),
        });
        let mut transport = MailboxTransport {
            mailboxes: Arc::clone(&mailboxes),
        };
        transport.transmit("phone", &[1]).unwrap();
        let refused = transport.transmit("tablet", &[1]).unwrap_err();
        assert_eq!(refused.queue_full, None);
        assert!(!refused.retryable);
        assert_eq!(mailboxes.lock().len(), 1);
        transport
            .transmit("phone", &[2])
            .expect("a held client is still sent to");
    }

    /// docs/adr/0109, D1: an entry that holds nothing (an empty mailbox, no exchange in flight,
    /// not ready) is removed by a sweep, and one the server still owes envelopes, one with an
    /// exchange in flight and one that is ready are kept. Red if the sweep keeps an entry that
    /// holds nothing.
    #[test]
    fn a_sweep_removes_only_the_entries_that_hold_nothing() {
        let mailboxes = Mailboxes {
            capacity: 4,
            max_clients: 8,
            clients: Mutex::new(HashMap::new()),
            durable: OnceLock::new(),
        };
        {
            let mut clients = mailboxes.lock();
            for name in ["gone", "owed", "busy", "ready"] {
                mailboxes.entry(&mut clients, name).expect("room");
            }
            clients
                .get_mut("owed")
                .unwrap()
                .entries
                .push_back((1, alloc::vec![1]));
            clients.get_mut("busy").unwrap().in_flight = 1;
            clients.get_mut("ready").unwrap().ready = true;
        }
        mailboxes.sweep();
        let mut kept: Vec<String> = mailboxes.lock().keys().cloned().collect();
        kept.sort_unstable();
        assert_eq!(kept, ["busy", "owed", "ready"]);
    }

    /// docs/adr/0109, D1: a response carries the oldest envelopes that fit in
    /// `max_response_bytes`, always at least one, and names the first it carries; the rest wait
    /// for the next exchange. Red if a response carries the whole mailbox.
    #[test]
    fn a_response_carries_at_most_its_bound_of_the_mailbox() {
        let mut client = Client::new(EPOCH.to_string());
        for n in 1..=3u64 {
            client.entries.push_back((n, alloc::vec![n as u8; 4]));
        }
        assert_eq!(
            client.response(8),
            (Some(1), alloc::vec![1, 1, 1, 1, 2, 2, 2, 2])
        );
        assert_eq!(client.response(7), (Some(1), alloc::vec![1, 1, 1, 1]));
        assert_eq!(
            client.response(2),
            (Some(1), alloc::vec![1, 1, 1, 1]),
            "an envelope larger than the bound still leaves, alone"
        );
        assert_eq!(client.response(usize::MAX).1.len(), 12);
        client.acknowledge(1, Some(EPOCH), EPOCH);
        assert_eq!(
            client.response(8),
            (Some(2), alloc::vec![2, 2, 2, 2, 3, 3, 3, 3])
        );
    }

    /// docs/adr/0109, D1 and D2: an address is counted by the block one subscriber holds: IPv4
    /// as it is, IPv6 by its /64, an IPv4 address mapped into IPv6 as IPv4.
    #[test]
    fn an_address_is_counted_by_its_ipv4_address_or_ipv6_block() {
        let key = |text: &str| address_key(text.parse().expect("an address"));
        assert_eq!(key("192.0.2.7"), key("192.0.2.7"));
        assert_ne!(key("192.0.2.7"), key("192.0.2.8"));
        assert_eq!(key("2001:db8:1:2:aaaa::1"), key("2001:db8:1:2:bbbb::2"));
        assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
        assert_eq!(key("::ffff:192.0.2.7"), key("192.0.2.7"));
    }

    fn accepted(
        name: &'static str,
        listen: &'static str,
        authenticator: HttpsAuthenticator,
        mailbox: u32,
    ) -> crate::peers::Peer {
        crate::peers::Peer {
            name,
            config: crate::router::PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: crate::inbound::Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: crate::router::Responders::Named(&[]),
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsAccept {
                listen,
                authenticator,
                max_hold_ms: 20_000,
                max_body_bytes: 4096,
                absence_ms: 60_000,
                mailbox,
                // Told apart per binding by its mailbox, so the smallest is seen to be taken.
                max_clients: mailbox * 100,
                max_response_bytes: u64::from(mailbox) * 64,
                rate: crate::peers::EdgeRates::DEFAULT,
                trusted_proxies: &[],
                sign: None,
                max_skew_ms: 300_000,
                fresh_ms: None,
                receipt: crate::peers::Receipt::Transport,
                retain_ms: 2_592_000_000,
            },
        }
    }

    /// A server built for one address serves every peer accepted there, under the smallest bound
    /// any of their bindings declares; peers on another address are another server's.
    #[test]
    fn an_address_is_served_under_the_smallest_bounds_its_bindings_declare() {
        let peers: &'static [crate::peers::Peer] = Box::leak(Box::new([
            accepted("phone", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 64),
            accepted("tablet", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 16),
            accepted("kiosk", "0.0.0.0:9443", HttpsAuthenticator::Mtls, 8),
        ]));
        let machine = Machine {
            name: "cloud",
            dedup_window: 8,
            gap_timeout_ms: 50,
            peers,
            open_peers: &[],
            instances: crate::peers::Instances::One,
            sessions: None,
        };
        let listening = Listening::on(&machine, "0.0.0.0:8443").expect("two peers there");
        assert_eq!(listening.peers, ["phone", "tablet"]);
        assert_eq!(listening.authenticator, HttpsAuthenticator::Bearer);
        assert_eq!(listening.settings.mailbox, 16);
        assert_eq!(listening.settings.max_hold, Duration::from_millis(20_000));
        assert_eq!(listening.settings.max_body_bytes, 4096);
        assert_eq!(listening.settings.absence, Duration::from_millis(60_000));
        assert_eq!(listening.settings.max_clients, 1_600);
        assert_eq!(listening.settings.max_response_bytes, 1_024);
        assert_eq!(
            Listening::on(&machine, "0.0.0.0:9443").map(|l| l.peers),
            Some(alloc::vec!["kiosk"])
        );
        assert!(Listening::on(&machine, "0.0.0.0:1").is_none());

        let mixed: &'static [crate::peers::Peer] = Box::leak(Box::new([
            accepted("phone", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 64),
            accepted("tablet", "0.0.0.0:8443", HttpsAuthenticator::Install, 64),
        ]));
        let machine = Machine {
            peers: mixed,
            ..machine
        };
        assert!(
            Listening::on(&machine, "0.0.0.0:8443").is_none(),
            "two authenticators on one address"
        );
    }

    /// docs/adr/0109, D9: an address served by one server signs with the one key its bindings
    /// name and holds signed requests to the smallest skew any declares; bindings that name two
    /// keys cannot be one server's. Red if `Listening::on` drops `sign` or takes either key.
    #[test]
    fn an_address_signs_with_the_one_key_its_bindings_name() {
        let signing = |name: &'static str, sign: Option<&'static str>, max_skew_ms: u32| {
            let mut peer = accepted(name, "0.0.0.0:9443", HttpsAuthenticator::Key, 64);
            if let PeerLink::HttpsAccept {
                sign: at,
                max_skew_ms: skew,
                ..
            } = &mut peer.link
            {
                *at = sign;
                *skew = max_skew_ms;
            }
            peer
        };
        let peers: &'static [crate::peers::Peer] = Box::leak(Box::new([
            signing("phone", Some("generated:0a"), 300_000),
            signing("tablet", Some("generated:0a"), 60_000),
        ]));
        let machine = Machine {
            name: "cloud",
            dedup_window: 8,
            gap_timeout_ms: 50,
            peers,
            open_peers: &[],
            instances: crate::peers::Instances::One,
            sessions: None,
        };
        let listening = Listening::on(&machine, "0.0.0.0:9443").expect("two peers there");
        assert_eq!(listening.sign, Some("generated:0a"));
        assert_eq!(listening.settings.max_skew, Duration::from_secs(60));
        assert_eq!(
            listening.settings.signer, None,
            "the key is the host's to resolve"
        );

        let two: &'static [crate::peers::Peer] = Box::leak(Box::new([
            signing("phone", Some("generated:0a"), 300_000),
            signing("tablet", Some("generated:0b"), 300_000),
        ]));
        let machine = Machine {
            peers: two,
            ..machine
        };
        assert!(
            Listening::on(&machine, "0.0.0.0:9443").is_none(),
            "two signing keys on one address"
        );
    }

    /// The `Authorization` value of the installation secret of 32 bytes of `byte`.
    fn install(byte: u8) -> String {
        use base64::Engine as _;

        alloc::format!(
            "{INSTALL_SCHEME} {}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([byte; 32])
        )
    }

    /// §mesh-22.5, ADR 0109 D3: a name is pinned, in the store the server was given, to the first
    /// secret it brings, and any other secret is answered `401` naming the scheme; once the pin is
    /// removed from the store, the next secret the name brings is pinned.
    #[test]
    fn a_name_is_pinned_to_the_first_secret_and_refuses_any_other() {
        use super::super::HttpsTransport;

        let pins = MemoryCredentials::new();
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = serving_authenticated(
            Authenticator::Install(Arc::new(pins.clone())),
            |_events: Vec<LinkEvent>| {},
            Duration::from_secs(5),
        );
        let exchange = |byte: u8| {
            port.exchange(request_authorized(
                "phone",
                address,
                Vec::new(),
                install(byte).as_str(),
            ))
            .expect("a response")
        };
        assert_eq!(exchange(1).status, 200, "the first secret");
        assert_eq!(
            pins.pinned("phone").expect("the pin"),
            Some(install_digest(&[1; 32]).to_vec()),
            "the name is pinned to the secret's SHA-256 in the store given"
        );
        assert_eq!(exchange(1).status, 200, "the same secret again");
        let refused = exchange(2);
        assert_eq!(refused.status, 401, "a second secret");
        assert_eq!(refused.header("WWW-Authenticate"), Some(INSTALL_SCHEME));
        assert_eq!(
            pins.pinned("phone").expect("the pin"),
            Some(install_digest(&[1; 32]).to_vec()),
            "a second secret replaced nothing"
        );
        assert!(pins.unpin("phone").expect("the operator removes the pin"));
        assert_eq!(
            exchange(2).status,
            200,
            "an unpinned name pins the next secret it brings"
        );
        assert_eq!(exchange(1).status, 401, "and refuses the first now");
    }

    /// A pin store whose medium fails: on every read when `reads_fail`, else only when it is
    /// asked to pin.
    struct FailingPins {
        reads_fail: bool,
    }

    impl PinStore for FailingPins {
        fn pinned(&self, _: &str) -> Result<Option<Vec<u8>>, sce_rust_session_store::Failure> {
            if self.reads_fail {
                Err(sce_rust_session_store::Failure::new("the medium is gone"))
            } else {
                Ok(None)
            }
        }

        fn pin(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, sce_rust_session_store::Failure> {
            Err(sce_rust_session_store::Failure::new("the medium is gone"))
        }

        fn unpin(&self, _: &str) -> Result<bool, sce_rust_session_store::Failure> {
            Err(sce_rust_session_store::Failure::new("the medium is gone"))
        }

        fn pins(
            &self,
        ) -> Result<Vec<sce_rust_session_store::PinRow>, sce_rust_session_store::Failure> {
            Err(sce_rust_session_store::Failure::new("the medium is gone"))
        }

        fn repin(
            &self,
            _: &str,
            _: &[u8],
            _: &[u8],
        ) -> Result<bool, sce_rust_session_store::Failure> {
            Err(sce_rust_session_store::Failure::new("the medium is gone"))
        }
    }

    /// ADR 0109, D3: a pin store that cannot be read, or cannot keep the pin, answers the exchange
    /// `503` with `Retry-After`, as load shed: the client is not at fault, and is not told `401`.
    #[test]
    fn a_pin_store_that_fails_is_answered_as_load_shed() {
        use super::super::HttpsTransport;

        for reads_fail in [true, false] {
            let Serving {
                _runtime,
                server: _,
                address,
                port,
            } = serving_authenticated(
                Authenticator::Install(Arc::new(FailingPins { reads_fail })),
                |_events: Vec<LinkEvent>| {},
                Duration::from_secs(5),
            );
            let shed = port
                .exchange(request_authorized(
                    "phone",
                    address,
                    Vec::new(),
                    install(1).as_str(),
                ))
                .expect("a response");
            assert_eq!(shed.status, 503, "reads fail: {reads_fail}");
            assert_eq!(shed.header("Retry-After"), Some("1"));
        }
    }

    /// ADR 0109, D3: the `bearer` authenticator over a credential store admits a token the store
    /// holds for the name the path claims, and answers `401` for another name, an expired token
    /// and a revoked one.
    #[test]
    fn a_stored_token_admits_its_name_until_it_expires_or_is_revoked() {
        use super::super::HttpsTransport;
        use sce_rust_crypto::credential::{token_digest, token_text};
        use sce_rust_session_store::{CredentialStore, TokenRow, TokenStore};

        let store = MemoryCredentials::new();
        let token = token_text(&[0x42; 32]);
        let other = token_text(&[0x43; 32]);
        for (text, name) in [(token.as_str(), "phone"), (other.as_str(), "tablet")] {
            store
                .issue(&TokenRow {
                    digest: token_digest(text),
                    name: name.to_string(),
                    issued_ms: 1_000,
                    expires_ms: 2_000,
                    revoked: false,
                    sealed: Vec::new(),
                })
                .expect("issuing a token");
        }
        let now = Arc::new(AtomicU64::new(1_500));
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = serving_authenticated(
            Authenticator::stored_tokens(StoredTokens::with_clock(Arc::new(store.clone()), {
                let now = Arc::clone(&now);
                Box::new(move || now.load(Ordering::SeqCst))
            })),
            |_events: Vec<LinkEvent>| {},
            Duration::from_secs(5),
        );
        let status = |text: &str| {
            port.exchange(request_authorized(
                "phone",
                address,
                Vec::new(),
                &alloc::format!("{BEARER_SCHEME} {text}"),
            ))
            .expect("a response")
            .status
        };
        assert_eq!(status(token.as_str()), 200, "phone's token");
        assert_eq!(
            status(other.as_str()),
            401,
            "tablet's token claims no phone"
        );
        assert_eq!(status("t"), 401, "a text that is no token");
        now.store(2_000, Ordering::SeqCst);
        assert_eq!(status(token.as_str()), 401, "expired");
        now.store(1_500, Ordering::SeqCst);
        assert_eq!(status(token.as_str()), 200, "before its expiry again");
        store.revoke("phone").expect("revoking phone");
        assert_eq!(status(token.as_str()), 401, "revoked");
    }

    /// A host whose endpoint checkpoints, its endpoint sealed through `sealed` whenever envelopes
    /// are admitted; it counts the envelopes it is handed.
    struct Checkpointing {
        sealed: u64,
        received: Arc<AtomicUsize>,
    }

    impl ServerHost for Checkpointing {
        fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
            let carried = events
                .iter()
                .filter(|event| matches!(event, LinkEvent::Received(..)))
                .count();
            self.received.fetch_add(carried, Ordering::SeqCst);
            Durability::After(self.sealed)
        }
    }

    /// A host that could not make durable what it took in, as a keyed pool whose store failed
    /// the checkpoint's write reports it; it counts the envelopes it is handed.
    struct Unwritten {
        received: Arc<AtomicUsize>,
    }

    impl ServerHost for Unwritten {
        fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
            let carried = events
                .iter()
                .filter(|event| matches!(event, LinkEvent::Received(..)))
                .count();
            self.received.fetch_add(carried, Ordering::SeqCst);
            Durability::NotDurable
        }
    }

    /// A server on a loopback port, the runtime it runs on, and the reference adapter that reaches
    /// it directly.
    struct Serving {
        // Held so that the server runs for as long as the test does.
        _runtime: tokio::runtime::Runtime,
        server: HttpsServer,
        address: std::net::SocketAddr,
        port: Arc<super::super::adapter::HyperTransport>,
    }

    /// A server of `phone` on a loopback port, with `host` behind it, holding an exchange at most
    /// `max_hold`, admitting the bearer token `t` for `phone`.
    fn serving(host: impl ServerHost, max_hold: Duration) -> Serving {
        serving_with(
            ServerSettings {
                max_hold,
                ..ServerSettings::default()
            },
            |name: &str| name == "phone",
            host,
        )
    }

    /// A server with `settings` on a loopback port, with `host` behind it, binding every name
    /// `bound` names, each proving it with the token `t`.
    fn serving_with(
        settings: ServerSettings,
        bound: impl Fn(&str) -> bool + Send + Sync + 'static,
        host: impl ServerHost,
    ) -> Serving {
        let bound = Arc::new(bound);
        let checked = Arc::clone(&bound);
        let server = HttpsServer::new(
            settings,
            Authenticator::Bearer(Arc::new(move |name: &str, token: &str| {
                checked(name) && token == "t"
            })),
            move |name| bound(name),
            host,
        )
        .expect("a server");
        listening(server)
    }

    /// A server of `phone` on a loopback port, checking names with `authenticator`, with `host`
    /// behind it, holding an exchange at most `max_hold`.
    fn serving_authenticated(
        authenticator: Authenticator,
        host: impl ServerHost,
        max_hold: Duration,
    ) -> Serving {
        let server = HttpsServer::new(
            ServerSettings {
                max_hold,
                ..ServerSettings::default()
            },
            authenticator,
            |name| name == "phone",
            host,
        )
        .expect("a server");
        listening(server)
    }

    /// `server` on a loopback port, the runtime it runs on, and the reference adapter.
    fn listening(server: HttpsServer) -> Serving {
        use super::super::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime");
        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        runtime.spawn({
            let server = server.clone();
            async move {
                let _ = server.serve(listener, None).await;
            }
        });
        let port = Arc::new(
            HyperTransport::new(AdapterSettings {
                proxy: ProxyPolicy::Direct,
                ..AdapterSettings::default()
            })
            .expect("an adapter"),
        );
        Serving {
            _runtime: runtime,
            server,
            address,
            port,
        }
    }

    /// An exchange of `phone` with the server at `address`, carrying `body`, asking for no hold,
    /// with the bearer token `t`.
    fn request_to(address: std::net::SocketAddr, body: Vec<u8>) -> super::super::HttpsRequest {
        request_as("phone", address, body)
    }

    /// An exchange of `name` with the server at `address`, carrying `body`, asking for no hold,
    /// with the bearer token `t`.
    fn request_as(
        name: &str,
        address: std::net::SocketAddr,
        body: Vec<u8>,
    ) -> super::super::HttpsRequest {
        request_authorized(name, address, body, "Bearer t")
    }

    /// An exchange of `name` with the server at `address`, carrying `body`, asking for no hold,
    /// with `authorization` as its `Authorization` header.
    fn request_authorized(
        name: &str,
        address: std::net::SocketAddr,
        body: Vec<u8>,
        authorization: &str,
    ) -> super::super::HttpsRequest {
        super::super::HttpsRequest {
            url: alloc::format!("http://{address}/sce-mesh/1/{name}"),
            headers: alloc::vec![
                ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
                (ACK_HEADER.to_string(), "0".to_string()),
                (HOLD_HEADER.to_string(), "0".to_string()),
                ("Authorization".to_string(), authorization.to_string()),
            ],
            body,
            timeout: Duration::from_secs(10),
        }
    }

    /// A server whose host's endpoint checkpoints answers an exchange that carried envelopes only
    /// once the host reports durable a checkpoint above the one sealed when they were admitted
    /// (SCE_MESH.md section 10.11); an exchange that carried none is answered at once, and every
    /// `200` names the server's incarnation.
    #[test]
    fn a_checkpointing_host_answers_once_a_checkpoint_holding_the_envelopes_is_durable() {
        use super::super::HttpsTransport;

        let received = Arc::new(AtomicUsize::new(0));
        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving(
            Checkpointing {
                sealed: 4,
                received: Arc::clone(&received),
            },
            Duration::from_secs(5),
        );
        assert_eq!(
            server.epoch_of("phone"),
            None,
            "no entry before an exchange"
        );

        let poll = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(poll.status, 200, "a poll waits for no checkpoint");
        let epoch = server.epoch_of("phone").expect("an entry for the client");
        assert!(is_epoch(&epoch), "{epoch}");
        assert_eq!(poll.header(EPOCH_HEADER), Some(epoch.as_str()));

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let port = Arc::clone(&port);
            move || {
                let _ = tx.send(port.exchange(request_to(address, alloc::vec![0x01])));
            }
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "answered before a checkpoint holding the envelope was durable"
        );
        server.durable(4);
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "the checkpoint sealed before the envelope was admitted does not hold it"
        );
        server.durable(5);
        let answered = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("answered once durable")
            .expect("a response");
        assert_eq!(answered.status, 200);
        assert_eq!(answered.header(EPOCH_HEADER), Some(epoch.as_str()));
        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    /// An exchange whose envelopes the host could not make durable (its checkpoint's write failed,
    /// or a fence refused it) is answered `503` with `Retry-After` at once, as load shed, never
    /// `200` (SCE_MESH.md section 10.11): the client carries them again. A poll, which carries
    /// nothing, is answered `200` whatever the host says.
    #[test]
    fn an_exchange_whose_envelopes_the_host_could_not_make_durable_is_shed() {
        use super::super::HttpsTransport;

        let received = Arc::new(AtomicUsize::new(0));
        let max_hold = Duration::from_secs(5);
        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving(
            Unwritten {
                received: Arc::clone(&received),
            },
            max_hold,
        );

        let poll = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(poll.status, 200, "a poll waits for no checkpoint");
        assert_eq!(
            poll.header(EPOCH_HEADER),
            server.epoch_of("phone").as_deref()
        );

        for _ in 0..2 {
            let started = Instant::now();
            let shed = port
                .exchange(request_to(address, alloc::vec![0x01]))
                .expect("a response");
            assert_eq!(shed.status, 503, "acknowledged what is not durable");
            assert_eq!(shed.header("Retry-After"), Some("1"));
            assert_eq!(shed.header(EPOCH_HEADER), None, "a 503 numbers nothing");
            assert!(shed.body.is_empty());
            assert!(
                started.elapsed() < max_hold,
                "a host that could not make them durable is not waited for"
            );
        }
        assert_eq!(
            received.load(Ordering::SeqCst),
            2,
            "each copy reached the host"
        );
    }

    /// What [`Meeting`] saw of the calls its server made.
    #[derive(Default)]
    struct Met {
        /// The client of each call inside `deliver` now.
        inside: Mutex<Vec<String>>,
        changed: std::sync::Condvar,
        /// The most clients that were inside at once.
        most: AtomicUsize,
        /// Whether two calls for one client were ever inside at once.
        overlapped: AtomicBool,
    }

    /// A host that notes whose deliveries are inside it at once (docs/adr/0109, D15): each call
    /// waits, at most half a second, until a call for another client is inside too.
    struct Meeting(Arc<Met>);

    impl ServerHost for Meeting {
        fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
            let client = events
                .iter()
                .find_map(|event| match event {
                    LinkEvent::Ready(name)
                    | LinkEvent::Received(name, _)
                    | LinkEvent::Lost(name, _) => Some(name.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let clients = |inside: &[String]| {
                let mut names = inside.to_vec();
                names.sort_unstable();
                names.dedup();
                names.len()
            };
            let met = &self.0;
            let mut inside = met.inside.lock().expect(HELD);
            if inside.contains(&client) {
                met.overlapped.store(true, Ordering::SeqCst);
            }
            inside.push(client.clone());
            met.most.fetch_max(clients(&inside), Ordering::SeqCst);
            met.changed.notify_all();
            let until = Instant::now() + Duration::from_millis(500);
            while clients(&inside) < 2 {
                let left = until.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                inside = met.changed.wait_timeout(inside, left).expect(HELD).0;
            }
            if let Some(at) = inside.iter().position(|name| *name == client) {
                inside.remove(at);
            }
            met.changed.notify_all();
            Durability::Now
        }
    }

    /// docs/adr/0109, D15: the server tells its host the exchanges of different clients at once,
    /// and those of one client one at a time. Two clients that post together are inside the
    /// host's `deliver` together; two exchanges of one client never are. Red if the server holds
    /// one lock across every client's delivery (the two clients never meet: at most one is
    /// inside), or no order for a client (its two exchanges overlap).
    #[test]
    fn clients_are_delivered_at_once_and_each_one_at_a_time() {
        use super::super::HttpsTransport;

        let met = Arc::new(Met::default());
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = serving_with(
            ServerSettings {
                rates: EdgeRates::NONE,
                ..ServerSettings::default()
            },
            |name: &str| name == "phone" || name == "tablet",
            Meeting(Arc::clone(&met)),
        );
        let post = |name: &'static str| {
            let port = Arc::clone(&port);
            std::thread::spawn(move || {
                port.exchange(request_as(name, address, alloc::vec![0x01]))
                    .expect("a response")
                    .status
            })
        };

        for exchange in [post("phone"), post("tablet")] {
            assert_eq!(exchange.join().expect("no exchange panicked"), 200);
        }
        assert_eq!(
            met.most.load(Ordering::SeqCst),
            2,
            "two clients' exchanges did not reach the host at once"
        );

        for exchange in [post("phone"), post("phone")] {
            assert_eq!(exchange.join().expect("no exchange panicked"), 200);
        }
        assert!(
            !met.overlapped.load(Ordering::SeqCst),
            "two exchanges of one client were inside the host at once"
        );
    }

    /// Wait, at most five seconds, until `holds` is true of the server; whether it became so.
    fn eventually(server: &HttpsServer, holds: impl Fn(&HttpsServer) -> bool) -> bool {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if holds(server) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        holds(server)
    }

    /// docs/adr/0109, D1: a server shown ten thousand names that each make one exchange and go
    /// away holds none of them once they are absent: each entry is removed when it holds nothing.
    /// Red if an entry is never removed: ten thousand stay.
    #[test]
    fn names_that_make_one_exchange_and_go_away_cost_nothing_once_absent() {
        use super::super::HttpsTransport;

        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving_with(
            ServerSettings {
                absence: Duration::from_millis(100),
                // Ten thousand exchanges from one address as fast as the loopback carries them:
                // what is judged here is what they leave behind, not their rate.
                rates: EdgeRates::NONE,
                ..ServerSettings::default()
            },
            |name: &str| name.starts_with('n'),
            |_: Vec<LinkEvent>| {},
        );
        for n in 0..10_000u32 {
            let name = alloc::format!("n{n}");
            let answered = port
                .exchange(request_as(&name, address, Vec::new()))
                .expect("a response");
            assert_eq!(answered.status, 200, "{name}");
        }
        assert!(
            eventually(&server, |server| server.clients() == 0),
            "{} entries are still held",
            server.clients()
        );
    }

    /// docs/adr/0109, D1: at `max_clients` entries an exchange for a name the server holds none
    /// for is answered `503` with `Retry-After`, and the names it holds are still served. Red if
    /// an entry is made past the bound.
    #[test]
    fn an_exchange_for_a_name_past_max_clients_is_shed() {
        use super::super::HttpsTransport;

        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving_with(
            ServerSettings {
                max_clients: 2,
                ..ServerSettings::default()
            },
            |_: &str| true,
            |_: Vec<LinkEvent>| {},
        );
        for name in ["a", "b"] {
            let answered = port
                .exchange(request_as(name, address, Vec::new()))
                .expect("a response");
            assert_eq!(answered.status, 200, "{name}");
        }
        let shed = port
            .exchange(request_as("c", address, Vec::new()))
            .expect("a response");
        assert_eq!(shed.status, 503);
        assert_eq!(shed.header("Retry-After"), Some("1"));
        assert_eq!(server.clients(), 2);
        let served = port
            .exchange(request_as("a", address, Vec::new()))
            .expect("a response");
        assert_eq!(served.status, 200, "a held name is still served");
    }

    /// Bytes `request` written on a plain connection to `address`, and what the server sent back
    /// until it closed the connection, sent a whole response head, or `wait` passed.
    fn raw_exchange(address: std::net::SocketAddr, request: &[u8], wait: Duration) -> Vec<u8> {
        use std::io::{Read, Write};

        let mut stream = std::net::TcpStream::connect(address).expect("a connection");
        stream.write_all(request).expect("written");
        stream.set_read_timeout(Some(wait)).expect("a read timeout");
        let mut answer = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => answer.extend_from_slice(&buffer[..read]),
            }
            if answer.windows(4).any(|end| end == b"\r\n\r\n") {
                break;
            }
        }
        answer
    }

    /// docs/adr/0109, D1: a client that sends a request's head too slowly has its connection
    /// closed at `header_timeout`, and one that sends its body too slowly is answered `408` at
    /// `body_timeout`. Red if either bound is not applied: the connection then stays open, unread,
    /// until the test's wait of five seconds passes.
    #[test]
    fn a_slow_head_and_a_slow_body_are_cut_off_at_their_bounds() {
        let Serving {
            _runtime, address, ..
        } = serving_with(
            ServerSettings {
                header_timeout: Duration::from_millis(200),
                body_timeout: Duration::from_millis(200),
                ..ServerSettings::default()
            },
            |name: &str| name == "phone",
            |_: Vec<LinkEvent>| {},
        );

        let started = Instant::now();
        let head = raw_exchange(
            address,
            b"POST /sce-mesh/1/phone HTTP/1.1\r\nHost: x\r\n",
            Duration::from_secs(5),
        );
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "a head that never ends is cut off: {:?}",
            String::from_utf8_lossy(&head)
        );

        let started = Instant::now();
        let body = raw_exchange(
            address,
            b"POST /sce-mesh/1/phone HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer t\r\n\
              Content-Type: application/cbor-seq\r\nContent-Length: 10\r\n\r\n",
            Duration::from_secs(5),
        );
        assert!(
            body.starts_with(b"HTTP/1.1 408"),
            "{:?}",
            String::from_utf8_lossy(&body)
        );
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    /// docs/adr/0109, D1: a connection past `max_connections_per_address` from one address is
    /// closed at accept; once the first closes, the address is served again. Red if the bound is
    /// not applied: the second connection is then held open, unread, past the test's wait.
    #[test]
    fn a_connection_past_its_address_bound_is_closed_at_accept() {
        use super::super::HttpsTransport;

        let Serving {
            _runtime,
            address,
            port,
            ..
        } = serving_with(
            ServerSettings {
                max_connections_per_address: 1,
                ..ServerSettings::default()
            },
            |name: &str| name == "phone",
            |_: Vec<LinkEvent>| {},
        );
        let first = std::net::TcpStream::connect(address).expect("a connection");
        let started = Instant::now();
        let second = raw_exchange(address, b"", Duration::from_secs(5));
        assert!(second.is_empty());
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "the second connection was closed at accept"
        );
        drop(first);
        let mut served = None;
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if let Ok(answered) = port.exchange(request_to(address, Vec::new())) {
                served = Some(answered.status);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(served, Some(200), "the address is served again");
    }

    /// docs/adr/0109, D2: a `401` storm from one address stops reaching the authenticator once
    /// its failures are past their rate: each exchange after is answered `429` with
    /// `Retry-After`, the authenticator is not run for it, and an established client is still
    /// served. Red if the server runs the authenticator before asking the failure limiters.
    #[test]
    fn a_storm_of_failed_authentications_stops_reaching_the_authenticator() {
        use super::super::HttpsTransport;

        let checked = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&checked);
        let server = HttpsServer::new(
            ServerSettings {
                rates: EdgeRates {
                    failed_auth: Some(EdgeRate {
                        interval_ms: 60_000,
                        burst: 3,
                    }),
                    ..EdgeRates::NONE
                },
                ..ServerSettings::default()
            },
            Authenticator::Bearer(Arc::new(move |name: &str, token: &str| {
                counted.fetch_add(1, Ordering::SeqCst);
                name == "phone" && token == "t"
            })),
            |_: &str| true,
            |_: Vec<LinkEvent>| {},
        )
        .expect("a server");
        let Serving {
            _runtime,
            address,
            port,
            ..
        } = listening(server);
        let guess = |name: &str| {
            let mut request = request_as(name, address, Vec::new());
            for header in &mut request.headers {
                if header.0 == "Authorization" {
                    header.1 = "Bearer guess".to_string();
                }
            }
            port.exchange(request).expect("a response")
        };
        for _ in 0..3 {
            assert_eq!(guess("victim").status, 401);
        }
        assert_eq!(checked.load(Ordering::SeqCst), 3);
        for _ in 0..20 {
            let refused = guess("victim");
            assert_eq!(refused.status, 429);
            let wait: u64 = refused
                .header("Retry-After")
                .and_then(|value| value.parse().ok())
                .expect("a wait in seconds");
            assert!((1..=60).contains(&wait), "{wait}");
        }
        assert_eq!(
            checked.load(Ordering::SeqCst),
            3,
            "no guess past the rate reached the authenticator"
        );
        assert_eq!(
            guess("phone").status,
            429,
            "every name from the guessing address waits"
        );
    }

    /// docs/adr/0109, D2: every exchange from one address is held to the address's rate, and one
    /// past it is answered `429` with the limiter's wait rounded up to a second. Red if the
    /// address limiter is not asked.
    #[test]
    fn exchanges_past_the_rate_of_their_address_are_answered_429() {
        use super::super::HttpsTransport;

        let Serving {
            _runtime,
            address,
            port,
            ..
        } = serving_with(
            ServerSettings {
                rates: EdgeRates {
                    address: Some(EdgeRate {
                        interval_ms: 60_000,
                        burst: 2,
                    }),
                    ..EdgeRates::NONE
                },
                ..ServerSettings::default()
            },
            |name: &str| name == "phone",
            |_: Vec<LinkEvent>| {},
        );
        for _ in 0..2 {
            let answered = port
                .exchange(request_to(address, Vec::new()))
                .expect("a response");
            assert_eq!(answered.status, 200);
        }
        let refused = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(refused.status, 429);
        let wait: u64 = refused
            .header("Retry-After")
            .and_then(|value| value.parse().ok())
            .expect("a wait in seconds");
        assert!((59..=60).contains(&wait), "{wait}");
    }

    /// docs/adr/0109, D2: the server's bounds of one address merge the bindings there by taking
    /// the stricter of each limiter, and trusting only the proxies every binding trusts.
    #[test]
    fn an_address_takes_the_stricter_rates_of_its_bindings() {
        assert_eq!(
            stricter(
                Some(EdgeRate {
                    interval_ms: 10,
                    burst: 200
                }),
                Some(EdgeRate {
                    interval_ms: 50,
                    burst: 300
                })
            ),
            Some(EdgeRate {
                interval_ms: 50,
                burst: 200
            })
        );
        let on = Some(EdgeRate {
            interval_ms: 1,
            burst: 1,
        });
        assert_eq!(stricter(on, None), on);
        assert_eq!(stricter(None, on), on);
        assert_eq!(stricter(None, None), None);
    }

    use super::super::signature::{
        field_value, normalize_target_uri, verify_response, RequestSigner, SIGNATURE_HEADER,
    };
    use sce_rust_crypto::cose::TrustedKey;
    use sce_rust_crypto::ed25519::KeyPair;

    /// The nonce the signing tests send.
    const NONCE: &str = "AAECAwQFBgcICQoLDA0ODw";

    /// What a response to `request` answers, as the client that sent it reads it.
    fn answered_by(request: &super::super::HttpsRequest) -> Answered {
        let field = |name: &str| field_value(&request.headers, name).expect("a field it carried");
        Answered {
            target_uri: normalize_target_uri(&request.url).expect("a URL"),
            ack: field(ACK_HEADER),
            nonce: field(NONCE_HEADER),
        }
    }

    /// SCE_MESH.md section 22.9: under `sign:`, every `200` answering a request with a nonce is
    /// signed for that request, a poll's and one that carries the mailbox's envelope alike, and
    /// verifies under the trust set that holds the key by its id; it does not verify as the
    /// answer to another nonce. A request without a nonce, from a client that does not verify, is
    /// answered unsigned. Red if the server signs over another target or acknowledgement than the
    /// request's, or leaves a `200` unsigned.
    #[test]
    fn a_signing_server_signs_each_200_for_the_request_it_answers() {
        use super::super::HttpsTransport;

        let key = KeyPair::from_seed(&[3u8; 32]);
        let trusted = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: key.public_key(),
        }];
        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving_with(
            ServerSettings {
                max_hold: Duration::from_secs(5),
                signer: Some(ResponseSigner::new("server-2026", key)),
                ..ServerSettings::default()
            },
            |name: &str| name == "phone",
            |_: Vec<LinkEvent>| {},
        );
        let mut request = request_to(address, Vec::new());
        request
            .headers
            .push((NONCE_HEADER.to_string(), NONCE.to_string()));
        let poll = port.exchange(request.clone()).expect("a response");
        assert_eq!(poll.status, 200);
        let field = |name: &str| field_value(&poll.headers, name);
        assert_eq!(
            verify_response(
                &trusted,
                poll.status,
                &field,
                &poll.body,
                &answered_by(&request)
            ),
            Ok(())
        );
        let other = Answered {
            nonce: "BBECAwQFBgcICQoLDA0ODw".to_string(),
            ..answered_by(&request)
        };
        assert!(
            verify_response(&trusted, poll.status, &field, &poll.body, &other).is_err(),
            "a response answers its own request's nonce alone"
        );

        server
            .transport()
            .transmit("phone", &[0x05])
            .expect("room in the mailbox");
        let carrying = port.exchange(request.clone()).expect("a response");
        assert_eq!(carrying.body, [0x05]);
        assert_eq!(carrying.header(FIRST_HEADER), Some("1"));
        let field = |name: &str| field_value(&carrying.headers, name);
        assert_eq!(
            verify_response(
                &trusted,
                carrying.status,
                &field,
                &carrying.body,
                &answered_by(&request)
            ),
            Ok(())
        );

        let unsigned = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(unsigned.status, 200);
        assert_eq!(unsigned.header(SIGNATURE_HEADER), None);
    }

    /// A request of `name` to the server at `address`, carrying `body`, signed by `key` as made at
    /// `created`.
    fn signed_request(
        name: &str,
        address: std::net::SocketAddr,
        body: Vec<u8>,
        key: &KeyPair,
        created: i64,
    ) -> super::super::HttpsRequest {
        signed_fresh_request(name, address, body, key, created, None)
    }

    /// [`signed_request`], signing the value of the freshness step `fresh` when it carries one
    /// (docs/adr/0117, D4).
    fn signed_fresh_request(
        name: &str,
        address: std::net::SocketAddr,
        body: Vec<u8>,
        key: &KeyPair,
        created: i64,
        fresh: Option<&str>,
    ) -> super::super::HttpsRequest {
        let url = alloc::format!("http://{address}/sce-mesh/1/{name}");
        let mut headers = alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), "0".to_string()),
            (HOLD_HEADER.to_string(), "0".to_string()),
            (NONCE_HEADER.to_string(), NONCE.to_string()),
            (CONTENT_DIGEST_HEADER.to_string(), content_digest(&body)),
        ];
        if let Some(fresh) = fresh {
            headers.push((FRESH_HEADER.to_string(), fresh.to_string()));
        }
        let signed = RequestSigner::new(key.clone())
            .sign(
                &normalize_target_uri(&url).expect("a URL"),
                &headers,
                created,
            )
            .expect("every covered field");
        headers.extend(signed);
        super::super::HttpsRequest {
            url,
            headers,
            body,
            timeout: Duration::from_secs(10),
        }
    }

    /// SCE_MESH.md sections 22.5 and 22.9: the `key` authenticator pins a name to the key that
    /// signed its first exchange answered `200`, in the store given and as that key's `COSE_Key`;
    /// a request signed by a second key is refused `401` and replaces nothing; and a request whose
    /// body is not the one its signature's digest covers is refused. Red if the server pins the
    /// offered key of a name already pinned, or does not hold the body to its digest.
    #[test]
    fn a_name_is_pinned_to_the_key_of_its_first_200_and_refuses_a_second() {
        use super::super::HttpsTransport;

        let pins = MemoryCredentials::new();
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = serving_authenticated(
            Authenticator::Key(KeyPins::first_use(Arc::new(pins.clone()))),
            |_events: Vec<LinkEvent>| {},
            Duration::from_secs(5),
        );
        let first = KeyPair::from_seed(&[1u8; 32]);
        let second = KeyPair::from_seed(&[2u8; 32]);
        let send = |key: &KeyPair| {
            port.exchange(signed_request(
                "phone",
                address,
                Vec::new(),
                key,
                unix_seconds(),
            ))
            .expect("a response")
        };
        assert_eq!(send(&first).status, 200, "the first key");
        assert_eq!(
            pins.pinned("phone").expect("the pin"),
            Some(KeyPins::pin_of(&first.public_key())),
            "the name is pinned to the key in the store given"
        );
        assert_eq!(send(&first).status, 200, "the same key again");
        let refused = send(&second);
        assert_eq!(refused.status, 401, "a second key");
        assert_eq!(refused.header("WWW-Authenticate"), Some(KEY_SCHEME));
        assert_eq!(
            pins.pinned("phone").expect("the pin"),
            Some(KeyPins::pin_of(&first.public_key())),
            "a second key replaced nothing"
        );

        let mut altered =
            signed_request("phone", address, alloc::vec![0x05], &first, unix_seconds());
        altered.body = alloc::vec![0x06];
        assert_eq!(
            port.exchange(altered).expect("a response").status,
            401,
            "a body that is not the one signed"
        );
    }

    /// SCE_MESH.md section 22.9: under the `key` authenticator a request whose `created` lies
    /// further from the server's clock than `max_skew` is refused, whichever way; and a server
    /// that takes only registered keys refuses a name the host pinned to none and takes one it
    /// registered. Red if `created` is not held to the server's clock.
    #[test]
    fn a_request_created_too_far_from_the_clock_is_refused() {
        use super::super::HttpsTransport;

        let pins = MemoryCredentials::new();
        let server = HttpsServer::new(
            ServerSettings {
                max_skew: Duration::from_secs(300),
                ..ServerSettings::default()
            },
            Authenticator::Key(KeyPins::registered(Arc::new(pins.clone()))),
            |name| name == "phone",
            |_: Vec<LinkEvent>| {},
        )
        .expect("a server");
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = listening(server);
        let key = KeyPair::from_seed(&[1u8; 32]);
        let at = |created: i64| {
            port.exchange(signed_request("phone", address, Vec::new(), &key, created))
                .expect("a response")
                .status
        };
        assert_eq!(at(unix_seconds()), 401, "a key the host never registered");
        pins.pin("phone", &KeyPins::pin_of(&key.public_key()))
            .expect("the host registers the key");
        assert_eq!(at(unix_seconds()), 200, "the registered key");
        assert_eq!(at(unix_seconds() - 1_000), 401, "created too long ago");
        assert_eq!(at(unix_seconds() + 1_000), 401, "created too far ahead");
    }

    /// docs/adr/0117, D4: under the `key` authenticator and a freshness step, a request that signs
    /// no value, a value two steps old or a value of another name is refused `401` with
    /// `use_fresh`, takes nothing in, and is given the present value, signed under `sign:` for
    /// the request it answers; one that signs the value it was given, or the step before's, is
    /// taken, and its `200` gives the present value, signed. Red if `exchange` takes a request
    /// without a value, or `stale` gives none.
    #[test]
    fn a_request_signs_a_value_of_the_freshness_step_or_is_refused_and_given_it() {
        use super::super::signature::verify_refusal;
        use super::super::HttpsTransport;

        let pins = MemoryCredentials::new();
        let client = KeyPair::from_seed(&[1u8; 32]);
        pins.pin("phone", &KeyPins::pin_of(&client.public_key()))
            .expect("the host registers the key");
        let key = KeyPair::from_seed(&[3u8; 32]);
        let trusted = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: key.public_key(),
        }];
        let taken_in = Arc::new(Mutex::new(Vec::<LinkEvent>::new()));
        let server = HttpsServer::new(
            ServerSettings {
                // An hour, so that no step ends while the test runs.
                fresh: Some(Duration::from_secs(3_600)),
                signer: Some(ResponseSigner::new("server-2026", key)),
                ..ServerSettings::default()
            },
            Authenticator::Key(KeyPins::registered(Arc::new(pins.clone()))),
            |name| name == "phone",
            {
                let taken_in = Arc::clone(&taken_in);
                move |events: Vec<LinkEvent>| taken_in.lock().expect("held").extend(events)
            },
        )
        .expect("a server");
        let freshness = server.shared.fresh.clone().expect("a freshness step");
        let Serving {
            _runtime,
            server: _,
            address,
            port,
        } = listening(server);
        let send = |fresh: Option<&str>| {
            let request = signed_fresh_request(
                "phone",
                address,
                alloc::vec![0x05],
                &client,
                unix_seconds(),
                fresh,
            );
            let response = port.exchange(request.clone()).expect("a response");
            (request, response)
        };

        let (request, refused) = send(None);
        assert_eq!(refused.status, 401, "no value");
        assert_eq!(
            refused.header("WWW-Authenticate"),
            Some("SCE-Signature error=\"use_fresh\"")
        );
        let given = refused
            .header(FRESH_HEADER)
            .expect("the present value")
            .to_string();
        let field = |name: &str| field_value(&refused.headers, name);
        assert_eq!(
            verify_refusal(
                &trusted,
                refused.status,
                &field,
                &refused.body,
                &answered_by(&request)
            ),
            Ok(())
        );
        assert!(
            taken_in.lock().expect("held").is_empty(),
            "a refused request takes nothing in"
        );

        let (request, taken) = send(Some(given.as_str()));
        assert_eq!(taken.status, 200, "the value given");
        assert!(
            taken.header(FRESH_HEADER).is_some(),
            "a `200` gives the present value"
        );
        let field = |name: &str| field_value(&taken.headers, name);
        assert_eq!(
            verify_response(&trusted, 200, &field, &taken.body, &answered_by(&request)),
            Ok(())
        );
        assert!(taken_in
            .lock()
            .expect("held")
            .contains(&LinkEvent::Received("phone".to_string(), alloc::vec![0x05])));

        let present = freshness.step_at(unix_millis());
        let before = freshness.value_of("phone", present - 1);
        assert_eq!(send(Some(before.as_str())).1.status, 200, "the step before");
        let older = freshness.value_of("phone", present - 2);
        assert_eq!(send(Some(older.as_str())).1.status, 401, "two steps old");
        let another = freshness.value_of("tablet", present);
        assert_eq!(send(Some(another.as_str())).1.status, 401, "another name's");
    }

    /// docs/adr/0117, D4: an address holds requests to the one freshness step its bindings name;
    /// bindings that name two, or one and none, cannot be one server's. Red if `Listening::on`
    /// drops `fresh_ms` or takes either of two.
    #[test]
    fn an_address_holds_requests_to_the_one_freshness_step_its_bindings_name() {
        let fresh = |name: &'static str, fresh_ms: Option<u32>| {
            let mut peer = accepted(name, "0.0.0.0:9443", HttpsAuthenticator::Key, 64);
            if let PeerLink::HttpsAccept { fresh_ms: at, .. } = &mut peer.link {
                *at = fresh_ms;
            }
            peer
        };
        let machine = |peers: [crate::peers::Peer; 2]| Machine {
            name: "cloud",
            dedup_window: 8,
            gap_timeout_ms: 50,
            peers: Box::leak(Box::new(peers)),
            open_peers: &[],
            instances: crate::peers::Instances::One,
            sessions: None,
        };
        let one = machine([fresh("phone", Some(60_000)), fresh("tablet", Some(60_000))]);
        assert_eq!(
            Listening::on(&one, "0.0.0.0:9443").map(|l| l.settings.fresh),
            Some(Some(Duration::from_secs(60)))
        );
        let two = machine([fresh("phone", Some(60_000)), fresh("tablet", Some(30_000))]);
        assert!(Listening::on(&two, "0.0.0.0:9443").is_none(), "two steps");
        let some = machine([fresh("phone", Some(60_000)), fresh("tablet", None)]);
        assert!(
            Listening::on(&some, "0.0.0.0:9443").is_none(),
            "a step and none"
        );
    }

    /// An exchange of `phone` with the server at `address` acknowledging `ack` in `epoch` (none
    /// when `None`), asking for a hold of `hold_ms`, with the bearer token `t`.
    fn request_acknowledging(
        address: std::net::SocketAddr,
        ack: u64,
        epoch: Option<&str>,
        hold_ms: u64,
    ) -> super::super::HttpsRequest {
        let mut request = request_to(address, Vec::new());
        request
            .headers
            .retain(|(name, _)| name != ACK_HEADER && name != HOLD_HEADER);
        request
            .headers
            .push((ACK_HEADER.to_string(), ack.to_string()));
        request
            .headers
            .push((HOLD_HEADER.to_string(), hold_ms.to_string()));
        if let Some(epoch) = epoch {
            request
                .headers
                .push((EPOCH_HEADER.to_string(), epoch.to_string()));
        }
        request.timeout = Duration::from_secs(10).saturating_add(Duration::from_millis(hold_ms));
        request
    }

    /// The outboxes of `phone`, a client whose binding is durable, kept in `store`.
    fn phone_outboxes(store: &sce_rust_session_store::MemoryStore) -> Arc<Outboxes> {
        Arc::new(Outboxes::new(
            Arc::new(crate::commit::Committer::new(
                Arc::new(store.clone()),
                Duration::ZERO,
            )),
            [(
                "phone".to_string(),
                crate::outbox::OutboxBinding {
                    window: 8,
                    retain_ms: 2_592_000_000,
                },
            )]
            .into_iter()
            .collect(),
        ))
    }

    /// docs/adr/0109, D10: a durable client is answered from its outbox in the store, numbered
    /// in the outbox's incarnation; its acknowledgement deletes what it covers in the store; a
    /// server started again on the same store answers in the same incarnation and numbers on;
    /// and the transport refuses an envelope for it, which only a checkpoint's write appends.
    /// Red if the exchange answers a durable client from the in-memory mailbox, or acknowledges
    /// only there.
    #[test]
    fn a_durable_client_is_answered_from_its_outbox_and_acknowledges_in_the_store() {
        use super::super::HttpsTransport;
        use sce_rust_session_store::{Append, MemoryStore, SessionStore};

        let store = MemoryStore::new();
        // Two CBOR items, the envelopes a pool's writes appended.
        let appended = store
            .append(
                &[
                    Append {
                        client: "phone",
                        incarnation: EPOCH,
                        sender: Some("k"),
                        envelope: &[0x01],
                    },
                    Append {
                        client: "phone",
                        incarnation: EPOCH,
                        sender: Some("k"),
                        envelope: &[0x02],
                    },
                ],
                1_000,
            )
            .expect("two appends");
        assert_eq!(appended.len(), 2);

        let before = serving(|_: Vec<LinkEvent>| {}, Duration::from_secs(5));
        before.server.keep_outboxes(phone_outboxes(&store));
        assert!(
            before
                .server
                .transport()
                .transmit("phone", &[0x03])
                .is_err(),
            "the transport takes nothing for a durable client"
        );
        let first = before
            .port
            .exchange(request_acknowledging(before.address, 0, None, 0))
            .expect("a response");
        assert_eq!(first.status, 200);
        assert_eq!(first.header(EPOCH_HEADER), Some(EPOCH));
        assert_eq!(first.header(FIRST_HEADER), Some("1"));
        assert_eq!(first.body, alloc::vec![0x01, 0x02]);

        let acked = before
            .port
            .exchange(request_acknowledging(before.address, 1, Some(EPOCH), 0))
            .expect("a response");
        assert_eq!(acked.status, 200);
        assert_eq!(acked.header(FIRST_HEADER), Some("2"));
        assert_eq!(acked.body, alloc::vec![0x02]);
        let held = store
            .outbox("phone")
            .expect("the outbox read")
            .expect("the outbox");
        assert_eq!(
            held.entries
                .iter()
                .map(|entry| entry.number)
                .collect::<Vec<_>>(),
            alloc::vec![2],
            "the acknowledgement deleted entry 1 in the store"
        );
        drop(before);

        let again = serving(|_: Vec<LinkEvent>| {}, Duration::from_secs(5));
        again.server.keep_outboxes(phone_outboxes(&store));
        let restarted = again
            .port
            .exchange(request_acknowledging(again.address, 1, Some(EPOCH), 0))
            .expect("a response");
        assert_eq!(
            restarted.header(EPOCH_HEADER),
            Some(EPOCH),
            "the incarnation outlives the server"
        );
        assert_eq!(restarted.header(FIRST_HEADER), Some("2"));
        assert_eq!(restarted.body, alloc::vec![0x02]);
    }

    /// docs/adr/0109, D10: a stop answers a held exchange at once with what its mailbox holds,
    /// and takes no exchange in after it. Red if the hold waits out its bound, or a new exchange
    /// is answered `200`.
    #[test]
    fn a_stop_answers_a_held_exchange_at_once_and_takes_nothing_in_after_it() {
        use super::super::HttpsTransport;

        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving(|_: Vec<LinkEvent>| {}, Duration::from_secs(20));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let port = Arc::clone(&port);
            move || {
                let started = Instant::now();
                let answered = port.exchange(request_acknowledging(address, 0, None, 20_000));
                let _ = tx.send((answered, started.elapsed()));
            }
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "the exchange is held"
        );
        let ended = _runtime.block_on(server.stop(Duration::from_secs(5)));
        assert!(ended, "every exchange in flight ended");
        let (answered, took) = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the held exchange was answered");
        assert_eq!(answered.expect("a response").status, 200);
        assert!(
            took < Duration::from_secs(10),
            "answered at the stop, took {took:?}"
        );
        let after = port.exchange(request_acknowledging(address, 0, None, 0));
        assert!(
            !matches!(&after, Ok(response) if response.status == 200),
            "an exchange after the stop was taken in: {after:?}"
        );
    }

    /// A server of `phone` with `settings` on a loopback port, admitting the bearer token `t`,
    /// with `host` behind it, reporting to a recording adapter it answers with (docs/adr/0109,
    /// D16).
    fn serving_recorded(
        settings: ServerSettings,
        host: impl ServerHost,
    ) -> (Serving, Arc<sce_rust_metrics::recording::Recording>) {
        let recording = Arc::new(sce_rust_metrics::recording::Recording::new());
        let server = HttpsServer::observed(
            settings,
            Authenticator::Bearer(Arc::new(|name: &str, token: &str| {
                name == "phone" && token == "t"
            })),
            |name: &str| name == "phone",
            host,
            Arc::clone(&recording) as Arc<dyn Metrics>,
        )
        .expect("a server");
        (listening(server), recording)
    }

    /// Wait, at most five seconds, until `holds`; whether it did.
    fn within_five_seconds(holds: impl Fn() -> bool) -> bool {
        let until = Instant::now() + Duration::from_secs(5);
        while !holds() {
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }

    /// docs/adr/0109, D16: every exchange is counted by the status it was answered, a `401` by
    /// the authenticator that refused it, and an exchange whose envelopes the host could not make
    /// durable as a shed of durability; a refusal that sheds nothing (`404`, `401`) moves no shed.
    /// Red if the server counts an exchange before it knows the answer, or counts a `401` or a
    /// `404` as a shed.
    #[test]
    fn the_server_reports_each_exchange_by_status_and_each_401_by_its_authenticator() {
        use super::super::HttpsTransport;
        use sce_rust_metrics::SERVER_SHEDS;

        let (
            Serving {
                _runtime,
                address,
                port,
                ..
            },
            recording,
        ) = serving_recorded(
            ServerSettings::default(),
            Unwritten {
                received: Arc::new(AtomicUsize::new(0)),
            },
        );
        let status = |request: super::super::HttpsRequest| {
            port.exchange(request).expect("a response").status
        };
        assert_eq!(status(request_to(address, Vec::new())), 200, "a poll");
        assert_eq!(status(request_as("tablet", address, Vec::new())), 404);
        assert_eq!(
            status(request_authorized(
                "phone",
                address,
                Vec::new(),
                "Bearer guess"
            )),
            401
        );
        assert_eq!(
            status(request_to(address, alloc::vec![0x01])),
            503,
            "an envelope the host could not make durable"
        );

        let exchanges = |status| recording.counted(Counter::ServerExchanges { status });
        assert_eq!(exchanges(ResponseStatus::Ok), 1);
        assert_eq!(exchanges(ResponseStatus::NotFound), 1);
        assert_eq!(exchanges(ResponseStatus::Unauthorized), 1);
        assert_eq!(exchanges(ResponseStatus::ServiceUnavailable), 1);
        assert_eq!(
            recording.counted(Counter::AuthenticationFailures {
                authenticator: AuthenticatorKind::Bearer
            }),
            1
        );
        assert_eq!(
            recording.counted(Counter::ServerSheds {
                reason: ServerShed::Durability
            }),
            1
        );
        assert_eq!(recording.counted_in(&SERVER_SHEDS), 1);
    }

    /// docs/adr/0109, D16: an exchange past the address's rate is a shed of rate, one past
    /// `max_exchanges` a shed of concurrency, one whose name would pass `max_clients` a shed of
    /// that bound. Red if `Shared::shed` is handed another reason than the refusal it answers.
    #[test]
    fn the_server_reports_each_shed_by_the_limit_that_refused_it() {
        use super::super::HttpsTransport;

        let cases = [
            (
                ServerSettings {
                    rates: EdgeRates {
                        address: Some(EdgeRate {
                            interval_ms: 60_000,
                            burst: 1,
                        }),
                        ..EdgeRates::NONE
                    },
                    ..ServerSettings::default()
                },
                ServerShed::Rate,
                429,
            ),
            (
                ServerSettings {
                    max_exchanges: 0,
                    ..ServerSettings::default()
                },
                ServerShed::Concurrency,
                503,
            ),
            (
                ServerSettings {
                    max_clients: 0,
                    ..ServerSettings::default()
                },
                ServerShed::MaxClients,
                503,
            ),
        ];
        for (settings, reason, refused) in cases {
            let (
                Serving {
                    _runtime,
                    address,
                    port,
                    ..
                },
                recording,
            ) = serving_recorded(settings, |_: Vec<LinkEvent>| {});
            let mut statuses = Vec::new();
            for _ in 0..2 {
                statuses.push(
                    port.exchange(request_to(address, Vec::new()))
                        .expect("a response")
                        .status,
                );
            }
            assert_eq!(statuses.last(), Some(&refused), "{reason:?}: {statuses:?}");
            let shed = statuses.iter().filter(|status| **status == refused).count() as u64;
            assert_eq!(
                recording.counted(Counter::ServerSheds { reason }),
                shed,
                "{reason:?}"
            );
            assert_eq!(
                recording.counted_in(&sce_rust_metrics::SERVER_SHEDS),
                shed,
                "{reason:?}: no other reason"
            );
        }
    }

    /// docs/adr/0109, D16: a connection past `max_connections_per_address` is closed at accept
    /// and counted as a shed of that bound. Red if `Open::admit` refuses without the bound it met.
    #[test]
    fn a_connection_closed_at_accept_is_reported_as_a_shed_of_its_bound() {
        let (
            Serving {
                _runtime, address, ..
            },
            recording,
        ) = serving_recorded(
            ServerSettings {
                max_connections_per_address: 1,
                ..ServerSettings::default()
            },
            |_: Vec<LinkEvent>| {},
        );
        let _first = std::net::TcpStream::connect(address).expect("a connection");
        let second = raw_exchange(address, b"", Duration::from_secs(5));
        assert!(second.is_empty());
        let shed = Counter::ServerSheds {
            reason: ServerShed::MaxConnectionsPerAddress,
        };
        assert!(
            within_five_seconds(|| recording.counted(shed) == 1),
            "{:?}",
            recording.calls()
        );
        assert_eq!(
            recording.counted(Counter::ServerSheds {
                reason: ServerShed::MaxConnections
            }),
            0
        );
    }

    /// docs/adr/0109, D16: an exchange held while its client's mailbox is empty counts held
    /// until the stop answers it; after the stop the server neither accepts nor stops, and an
    /// exchange on a connection it accepted before is shed as the server stopping. Red if the
    /// held count is not given back when a hold ends, or a stopping server's refusal is counted
    /// under another reason.
    #[test]
    fn the_server_reports_its_held_exchanges_and_a_shed_while_stopping() {
        use super::super::HttpsTransport;
        use std::io::{Read, Write};

        let (
            Serving {
                _runtime,
                server,
                address,
                port,
            },
            recording,
        ) = serving_recorded(
            ServerSettings {
                max_hold: Duration::from_secs(20),
                ..ServerSettings::default()
            },
            |_: Vec<LinkEvent>| {},
        );
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let port = Arc::clone(&port);
            move || {
                let _ = tx.send(port.exchange(request_acknowledging(address, 0, None, 20_000)));
            }
        });
        assert!(
            within_five_seconds(|| recording.level(Gauge::HeldExchanges) == 1),
            "the poll is held"
        );
        // A connection the server accepted before its stop, served after it.
        let mut open = std::net::TcpStream::connect(address).expect("a connection");
        assert!(within_five_seconds(|| {
            server.shared.connections.lock().expect(HELD).open >= 2
        }));

        assert!(_runtime.block_on(server.stop(Duration::from_secs(5))));
        let answered = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the held exchange was answered")
            .expect("a response");
        assert_eq!(answered.status, 200);
        assert_eq!(recording.level(Gauge::HeldExchanges), 0);
        assert_eq!(recording.peak(Gauge::HeldExchanges), 1);
        let stopped = server.health();
        assert!(stopped.stopped(), "{stopped:?}");

        open.write_all(
            b"POST /sce-mesh/1/phone HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer t\r\n\
              Content-Length: 0\r\n\r\n",
        )
        .expect("written");
        open.set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a read timeout");
        let mut head = [0u8; 12];
        open.read_exact(&mut head).expect("a response head");
        assert_eq!(&head, b"HTTP/1.1 503");
        assert!(within_five_seconds(|| {
            recording.counted(Counter::ServerSheds {
                reason: ServerShed::Stopping,
            }) == 1
        }));
    }

    /// A host whose `deliver` waits until the test opens it.
    #[derive(Clone, Default)]
    struct Gate(Arc<(Mutex<bool>, std::sync::Condvar)>);

    impl Gate {
        fn set(&self, open: bool) {
            let (state, signal) = &*self.0;
            *state.lock().expect(HELD) = open;
            signal.notify_all();
        }
    }

    impl ServerHost for Gate {
        fn deliver(&self, _: Vec<LinkEvent>) -> Durability {
            let (state, signal) = &*self.0;
            let until = Instant::now() + Duration::from_secs(5);
            let mut open = state.lock().expect(HELD);
            while !*open && Instant::now() < until {
                open = signal
                    .wait_timeout(open, Duration::from_millis(10))
                    .expect(HELD)
                    .0;
            }
            Durability::Now
        }
    }

    /// docs/adr/0109, D16: a server accepts until its stop, is stopping while an exchange it took
    /// in is still in flight, and stopped once that exchange ended. Red if `HttpsServer::health`
    /// reads its state from the stop's flag alone.
    #[test]
    fn the_server_says_it_is_stopping_while_an_exchange_is_in_flight() {
        use super::super::HttpsTransport;

        let gate = Gate::default();
        gate.set(true);
        let (
            Serving {
                _runtime,
                server,
                address,
                port,
            },
            _recording,
        ) = serving_recorded(ServerSettings::default(), gate.clone());
        assert_eq!(
            port.exchange(request_to(address, Vec::new()))
                .expect("a response")
                .status,
            200,
            "the poll that makes the client ready"
        );
        assert!(server.health().ready());

        gate.set(false);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let port = Arc::clone(&port);
            move || {
                let _ = tx.send(port.exchange(request_to(address, alloc::vec![0x01])));
            }
        });
        assert!(within_five_seconds(|| server
            .shared
            .exchanges
            .load(Ordering::Acquire)
            == 1));
        assert!(
            !_runtime.block_on(server.stop(Duration::from_millis(100))),
            "the exchange in flight did not end"
        );
        let stopping = server.health();
        assert!(stopping.stopping && !stopping.accepting, "{stopping:?}");

        gate.set(true);
        rx.recv_timeout(Duration::from_secs(5))
            .expect("the exchange in flight ended")
            .expect("a response");
        assert!(within_five_seconds(|| server.health().stopped()));
    }

    /// docs/adr/0109, D16: a server whose pin store cannot be read is not ready, and one whose
    /// authenticator reads no store of the server's reaches every store it reads. Red if
    /// `HttpsServer::health` does not ask the pin store.
    #[test]
    fn a_server_whose_pin_store_cannot_be_read_is_not_ready() {
        for reads_fail in [true, false] {
            let server = HttpsServer::new(
                ServerSettings::default(),
                Authenticator::Install(Arc::new(FailingPins { reads_fail })),
                |name: &str| name == "phone",
                |_: Vec<LinkEvent>| {},
            )
            .expect("a server");
            let health = server.health();
            assert_eq!(
                health.store_reachable, !reads_fail,
                "reads fail: {reads_fail}"
            );
            assert_eq!(health.ready(), !reads_fail);
            assert!(health.accepting && !health.stopping);
        }
        let server = HttpsServer::new(
            ServerSettings::default(),
            Authenticator::Bearer(Arc::new(|_: &str, _: &str| false)),
            |_: &str| true,
            |_: Vec<LinkEvent>| {},
        )
        .expect("a server");
        assert!(server.health().ready());
    }
}
