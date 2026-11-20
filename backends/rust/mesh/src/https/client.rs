// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The dialing side of the HTTPS binding (SCE_MESH.md section 22): a machine that reaches one
//! server machine by exchanges it opens, through the port [`HttpsTransport`]. It is the client
//! of §mesh-22.8: [`HttpsClient`] over the port, and [`HttpsClientTransport`], the core's
//! transport.
//!
//! Two threads, each with at most one exchange in flight, as §mesh-22.3 has a client keep them.
//! The poller holds an exchange open (`Sce-Mesh-Hold-Ms`) so that what the server sends reaches
//! the client as soon as it is made; the sender carries what the core transmits as soon as it is
//! transmitted, without waiting for a held exchange to end. Both acknowledge what the client has
//! received, and what either response carries is handed to the core once, in mailbox order.
//!
//! The binding keeps no decision of the core's. What it hands its host is [`LinkEvent`]s: the
//! server is ready (from the first exchange answered `200`), bytes arrived, an envelope the
//! exchange could not deliver (handed back as [`LinkEvent::Unsent`], which the core sends again
//! under the binding's retry policy or raises as SCE_MESH.md section 16.7 row 2 or row 3), the
//! link was interrupted (an exchange failed at the transport, while the server keeps the mailbox
//! and so every reply in it), the server started again without the mailbox (a response of a new
//! incarnation), and the server was lost (the client stopped, and takes nothing in any more).
//!
//! Message signatures (SCE_MESH.md section 22.9) are part of these semantics, so every adapter
//! gets them: under `verify:` ([`ClientSettings::verify`]) a `200` is taken only when a key of
//! the trust set signed it for the request it answers, and one that does not verify is taken
//! nothing of, acknowledges nothing, hands back what its request carried as not sent, and is told
//! once per edge ([`LinkEvent::Unauthenticated`], section 16.7 row 19); under the `key`
//! authenticator ([`Credential::Key`]) every request is signed by the client's key.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sce_rust_crypto::cose::TrustedKey;
use sce_rust_crypto::random::SystemRandom;
use sce_rust_metrics::{Counter, ExchangeStatus, Metrics};

use super::signature::{
    content_digest, draw_nonce, field_value, normalize_target_uri, verify_response, Answered,
    CONTENT_DIGEST_HEADER, NONCE_HEADER,
};
use super::{
    is_epoch, is_path_segment, retry_after, split_sequence, Credential, HttpsFailure, HttpsRequest,
    HttpsResponse, HttpsTransport, LinkEvent, ACK_HEADER, CONTENT_TYPE, EPOCH_HEADER, FIRST_HEADER,
    HOLD_HEADER, PATH_PREFIX,
};
use crate::endpoint::{Transport, TransportFailure};
use crate::peers::{HttpsAuthenticator, Peer, PeerLink, Receipt};
use crate::reconnect::Backoff;

/// How a client reaches its server.
#[derive(Debug, Clone)]
pub struct ClientSettings {
    /// The server's base: `https://<host>[:<port>][<base path>]`, before the binding's path, with
    /// no query or fragment. `http://` is for a loopback host only.
    pub base: String,
    /// This machine's name: the path segment it posts under, and the `source` its envelopes carry.
    pub own: String,
    /// The server machine's name: the peer the core addresses.
    pub server: String,
    /// How the client proves its name (§mesh-22.5).
    pub credential: Credential,
    /// deploy.yaml's `hold_ms`: how long a poll asks the server to hold it while nothing waits.
    pub hold: Duration,
    /// How long an exchange may take beyond the hold it asks for before it is a timeout.
    pub exchange_timeout: Duration,
    /// The most bytes of envelopes one exchange carries; an envelope larger than this is carried
    /// alone. The server's `max_body_bytes` by default.
    pub max_body_bytes: usize,
    /// The waits between polls after a failed one.
    pub backoff: Backoff,
    /// deploy.yaml's `verify:`: the keys of the trust set it names, by id, when the binding
    /// verifies what its server sends (SCE_MESH.md section 22.9). `None` takes a `200` as it
    /// comes.
    pub verify: Option<Vec<TrustedKey>>,
    /// deploy.yaml's `receipt: durable` (docs/adr/0109, D10; SCE_MESH.md section 10.11): every
    /// request acknowledges only what the host reported a durable checkpoint of its own holds
    /// ([`HttpsClient::held`]), and each `200` that carried envelopes is told to the host
    /// ([`LinkEvent::Carried`]), whose core keeps each in its checkpoint until then.
    pub durable: bool,
}

impl ClientSettings {
    /// The settings a deployment's defaults give a client of `own` reaching `server` at `base`.
    pub fn new(
        base: impl Into<String>,
        own: impl Into<String>,
        server: impl Into<String>,
        credential: Credential,
    ) -> Self {
        ClientSettings {
            base: base.into(),
            own: own.into(),
            server: server.into(),
            credential,
            hold: Duration::from_millis(u64::from(super::DEFAULT_HOLD_MS)),
            exchange_timeout: Duration::from_secs(30),
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES as usize,
            backoff: Backoff::default(),
            verify: None,
            durable: false,
        }
    }

    /// The settings the generated peer table gives the machine `own` for `peer`, when this
    /// machine dials it over HTTPS ([`PeerLink::HttpsDial`]), proving its name with `credential`,
    /// which the host holds, and verifying what the server sends under `trusted`, the keys of the
    /// trust set the link's `verify` names, which the host holds too. `None` for any other link,
    /// for a credential of another kind than the link's `authenticator`, which the server would
    /// refuse, and for keys given to a link that verifies nothing or none to one that verifies:
    /// a deployment that asks for verification is never run without it.
    pub fn from_peer(
        own: &str,
        peer: &Peer,
        credential: Credential,
        trusted: Option<Vec<TrustedKey>>,
    ) -> Option<ClientSettings> {
        let PeerLink::HttpsDial {
            url,
            authenticator,
            hold_ms,
            verify,
            receipt,
        } = peer.link
        else {
            return None;
        };
        // §mesh-22.5: the credential is of the one authenticator the binding names, which is the
        // one the server checks.
        let fits = matches!(
            (authenticator, &credential),
            (HttpsAuthenticator::Bearer, Credential::Bearer(_))
                | (HttpsAuthenticator::Install, Credential::Install(_))
                | (HttpsAuthenticator::Mtls, Credential::Mtls)
                | (HttpsAuthenticator::Key, Credential::Key(_))
        );
        // SCE_MESH.md section 22.9: the trust set is given exactly when the link names one.
        let verifies = verify.is_some() == trusted.is_some();
        (fits && verifies).then(|| ClientSettings {
            hold: Duration::from_millis(u64::from(hold_ms)),
            verify: trusted,
            durable: matches!(receipt, Receipt::Durable { .. }),
            ..ClientSettings::new(url, own, peer.name, credential)
        })
    }
}

/// Why a client could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The machine name is not a path segment as it stands (§mesh-22.1).
    NotAPathSegment(String),
    /// The base is not an `https://` URL, or an `http://` one for a loopback host, or carries a
    /// query or a fragment.
    BadBase(String),
}

/// What the two threads share.
struct Shared {
    settings: ClientSettings,
    /// The URL every exchange posts to.
    url: String,
    /// The same URL as a signature covers it (`@target-uri`, SCE_MESH.md section 22.9).
    target_uri: String,
    /// The `Authorization` header, when the credential is one.
    authorization: Option<String>,
    port: Arc<dyn HttpsTransport>,
    state: Mutex<State>,
    /// Wakes the sender when the core transmits, and either thread when the client stops.
    wake: Condvar,
    /// What the client reports each exchange to (docs/adr/0109, D16).
    metrics: Arc<dyn Metrics>,
}

type EventSink = Box<dyn FnMut(LinkEvent) + Send>;

struct State {
    /// What the core transmitted and no exchange has carried yet, oldest first.
    outbox: VecDeque<Vec<u8>>,
    /// What every request acknowledges: the highest mailbox number handed to the core, in the
    /// mailbox incarnation the client last saw.
    mark: Mark,
    ready: bool,
    stopped: bool,
    /// Whether the last `200` did not verify under `verify:`, so that a run of them is told once
    /// (SCE_MESH.md section 16.7 row 19, once per edge).
    unauthenticated: bool,
    /// Under `receipt: durable` (docs/adr/0109, D10), the highest mailbox number, in the
    /// incarnation of `mark`, that the host reported a durable checkpoint of its own holds
    /// ([`HttpsClient::held`]): what every request acknowledges, never above `mark.ack`. `None`
    /// on a binding that is not durable, whose requests acknowledge `mark.ack`.
    held: Option<u64>,
    /// Where the host is told what happened. Called with the state held, so that what the two
    /// threads observe reaches the host in the order it was observed.
    events: EventSink,
}

/// What a request was sent with that its response is read against: where its acknowledgement
/// stood, and, when the client verifies, what of it the response's signature covers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Sent {
    mark: Mark,
    answered: Option<Answered>,
}

/// Where an acknowledgement stands (§mesh-22.3): the highest mailbox number handed to the core,
/// and the incarnation of the mailbox it counts in, `None` before the first `200`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Mark {
    ack: u64,
    epoch: Option<String>,
}

const HELD: &str = "the client's state is never left mid-update: nothing in it panics while held";

impl State {
    /// What the next request acknowledges: the highest number handed to the core, or, under
    /// `receipt: durable`, the highest a durable checkpoint of the host holds (docs/adr/0109,
    /// D10), so that the server never deletes what a crash of this client would lose.
    fn acknowledging(&self) -> Mark {
        match self.held {
            Some(held) => Mark {
                ack: held.min(self.mark.ack),
                epoch: self.mark.epoch.clone(),
            },
            None => self.mark.clone(),
        }
    }
}

/// Where a client stands in its server's mailbox (SCE_MESH.md section 22.3): the incarnation
/// the mailbox numbers in (`None` before the first `200`) and the highest number the client
/// handed its host. Read by a host that checkpoints under `receipt: durable` before it applies
/// what the client handed it, and reported back once a checkpoint that holds it is durable
/// ([`HttpsClient::received`], [`HttpsClient::held`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Received {
    pub epoch: Option<String>,
    pub number: u64,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect(HELD)
    }

    /// The request that carries `body`, acknowledging `mark` and asking for `hold`, and what it
    /// was sent with. Refused as [`HttpsFailure::Other`] when a nonce it needs could not be drawn.
    fn request(
        &self,
        body: Vec<u8>,
        mark: &Mark,
        hold: Duration,
    ) -> Result<(HttpsRequest, Sent), HttpsFailure> {
        // §mesh-22.1: what every request carries besides its body, the media type of the
        // §mesh-22.2 framing, the acknowledgement and the incarnation it counts in (none before
        // the first `200`), the hold, and the credential.
        let mut headers = alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), mark.ack.to_string()),
            (HOLD_HEADER.to_string(), hold.as_millis().to_string()),
        ];
        if let Some(epoch) = &mark.epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.clone()));
        }
        if let Some(authorization) = &self.authorization {
            headers.push(("Authorization".to_string(), authorization.clone()));
        }
        // §mesh-22.9: a client that verifies or signs draws a nonce for each
        // exchange, which a response's signature covers so that it answers this request and no
        // other; a client of the `key` authenticator signs the request over its body's digest.
        let signer = self.settings.credential.request_signer();
        let mut answered = None;
        if self.settings.verify.is_some() || signer.is_some() {
            let nonce = draw_nonce(&mut SystemRandom).ok_or_else(|| {
                HttpsFailure::Other("the random source could not be read".to_string())
            })?;
            headers.push((NONCE_HEADER.to_string(), nonce.clone()));
            if self.settings.verify.is_some() {
                answered = Some(Answered {
                    target_uri: self.target_uri.clone(),
                    ack: mark.ack.to_string(),
                    nonce,
                });
            }
        }
        if let Some(signer) = signer {
            headers.push((CONTENT_DIGEST_HEADER.to_string(), content_digest(&body)));
            let created = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
                .unwrap_or(0);
            let signed = signer
                .sign(&self.target_uri, &headers, created)
                .ok_or_else(|| {
                    HttpsFailure::Other(
                        "the request lacks a field its signature covers".to_string(),
                    )
                })?;
            headers.extend(signed);
        }
        let request = HttpsRequest {
            url: self.url.clone(),
            headers,
            body,
            timeout: hold.saturating_add(self.settings.exchange_timeout),
        };
        Ok((
            request,
            Sent {
                mark: mark.clone(),
                answered,
            },
        ))
    }

    /// Why `response`, a `200`, is not one the client may take: `None` when the binding does not
    /// verify or the response verifies (SCE_MESH.md section 22.9).
    fn unverified(&self, sent: &Sent, response: &HttpsResponse) -> Option<String> {
        let trusted = self.settings.verify.as_ref()?;
        let Some(answered) = &sent.answered else {
            return Some("the request was sent without a nonce".to_string());
        };
        let field = |name: &str| field_value(&response.headers, name);
        verify_response(trusted, response.status, &field, &response.body, answered)
            .err()
            .map(|refusal| refusal.to_string())
    }
}

/// What an exchange's outcome asks of the thread that made it.
#[derive(Debug, PartialEq, Eq)]
enum Then {
    /// Go on at once.
    Continue,
    /// Wait at least this long first: the server asked (`Retry-After`) or the exchange failed.
    Wait(Duration),
}

/// Take what an exchange returned. `carried` are the envelopes the request carried and `sent` what
/// it was sent with. Returns what the thread should do next; a response the server shed puts
/// `carried` back at the front of the outbox, to go again after the wait.
fn take_outcome(
    shared: &Shared,
    carried: Vec<Vec<u8>>,
    sent: &Sent,
    outcome: Result<HttpsResponse, HttpsFailure>,
    failures: &mut u32,
) -> Then {
    // docs/adr/0109, D16: every exchange, by the status its server answered, or none.
    let status = match &outcome {
        Ok(response) => ExchangeStatus::of(response.status),
        Err(_) => ExchangeStatus::NoResponse,
    };
    shared.metrics.count(Counter::ClientExchanges { status }, 1);
    let server = shared.settings.server.clone();
    // Read before the state is held: a verification is the response's alone.
    let unverified = match &outcome {
        Ok(response) if response.status == 200 => shared.unverified(sent, response),
        _ => None,
    };
    let mut state = shared.lock();
    if state.stopped {
        // The client stopped while the exchange was in flight: it has told the host its server is
        // lost, and takes nothing more in. What the mailbox sent stays there, unacknowledged.
        for envelope in carried {
            (state.events)(LinkEvent::Unsent(server.clone(), envelope));
        }
        return Then::Continue;
    }
    match outcome {
        // §mesh-22.9: a `200` that did not verify proves nothing of the server. The
        // client takes nothing of it and acknowledges nothing, so what the mailbox holds stays
        // there for a response that verifies; what the request carried is handed back as not
        // sent, since nothing says the server took it; readiness stands as it was; and the host
        // is told once per edge (section 16.7 row 19). The client tries again on its backoff.
        Ok(_) if unverified.is_some() => {
            for envelope in carried {
                (state.events)(LinkEvent::Unsent(server.clone(), envelope));
            }
            if !state.unauthenticated {
                state.unauthenticated = true;
                let why = unverified.unwrap_or_default();
                (state.events)(LinkEvent::Unauthenticated(server, why));
            }
            let wait = shared.settings.backoff.delay(*failures);
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
        Ok(response) if response.status == 200 => {
            *failures = 0;
            state.unauthenticated = false;
            if !state.ready {
                state.ready = true;
                (state.events)(LinkEvent::Ready(server.clone()));
            }
            // docs/adr/0109, D10: under `receipt: durable` the server answers `200` only once a
            // durable checkpoint holds what the request carried, which is the binding's
            // acknowledgement of it: the host's core holds it no more.
            if shared.settings.durable && !carried.is_empty() {
                (state.events)(LinkEvent::Carried(server.clone(), carried));
            }
            take_mailbox(&mut state, &server, &sent.mark, &response);
            Then::Continue
        }
        // §mesh-22.4: load shed is neither a failure nor a lost server. What the request carried
        // goes again after at least the wait `Retry-After` asks for.
        Ok(response) if matches!(response.status, 429 | 503) => {
            for envelope in carried.into_iter().rev() {
                state.outbox.push_front(envelope);
            }
            let asked = response
                .header("Retry-After")
                .and_then(|value| retry_after(value, SystemTime::now()));
            let wait = asked.unwrap_or_else(|| shared.settings.backoff.delay(*failures));
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
        // A status the binding does not take in: the server answered, so the client stays as
        // ready as it was, and what the request carried was not taken (§mesh-22.4).
        Ok(_) => {
            for envelope in carried {
                (state.events)(LinkEvent::Unsent(server.clone(), envelope));
            }
            let wait = shared.settings.backoff.delay(*failures);
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
        // §mesh-22.4: a failure at the transport hands the envelopes back as not sent and ends
        // readiness until the next `200`.
        Err(failure) => {
            for envelope in carried {
                (state.events)(LinkEvent::Unsent(server.clone(), envelope));
            }
            // A failure at the transport ends readiness and loses nothing (section 22.4): the
            // server keeps the mailbox, a reply to a request this client waits on among it, and
            // a later exchange carries it, so the link is interrupted, not lost.
            if state.ready {
                state.ready = false;
                (state.events)(LinkEvent::Interrupted(server, failure.to_string()));
            }
            let wait = shared.settings.backoff.delay(*failures);
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
    }
}

/// Hand the core the envelopes of a `200` response, in mailbox order and each once, and move the
/// acknowledgement past them (§mesh-22.3). `sent` is what the response's request acknowledged.
///
/// The response's incarnation (`Sce-Mesh-Epoch`) says which numbering its numbers count in:
///
/// * the one the client last saw: what is at or below the client's acknowledgement (which the
///   other thread's response may already have handed on) is passed over;
/// * another, which the request was sent in: an incarnation the client has left since, because
///   the other thread's response named a newer one. What is above the request's acknowledgement is
///   handed on, the section 10.5 window dropping what reached the core before, and the client's
///   acknowledgement stays with the newer incarnation;
/// * any other: a mailbox that numbers anew (a server that started again with its mailboxes
///   empty). The acknowledgement starts over at 0 in it, so all of the response is new. When it
///   replaces an incarnation the client knew, what that mailbox held is gone, a reply among it:
///   the host is told the server restarted ([`LinkEvent::Restarted`]) before the response's
///   envelopes, so the core asks again what it waits on. The first incarnation the client learns
///   replaces none.
///
/// A response that names no incarnation says nothing of what its numbers count in, and a body
/// without a first number cannot be acknowledged: what a server that breaks the binding sends is
/// not taken.
fn take_mailbox(state: &mut State, server: &str, sent: &Mark, response: &HttpsResponse) {
    // §mesh-22.2: a client takes nothing of a `200` without `Sce-Mesh-Epoch`, reads the body as a
    // CBOR sequence, and numbers it from `Sce-Mesh-First`.
    let Some(epoch) = response
        .header(EPOCH_HEADER)
        .map(str::trim)
        .filter(|epoch| is_epoch(epoch))
    else {
        return;
    };
    let current = if state.mark.epoch.as_deref() == Some(epoch) {
        true
    } else if sent.epoch.as_deref() == Some(epoch) {
        false
    } else {
        let replaced = state.mark.epoch.is_some();
        state.mark = Mark {
            ack: 0,
            epoch: Some(epoch.to_string()),
        };
        // What a checkpoint held counted in the numbering that is gone.
        if let Some(held) = state.held.as_mut() {
            *held = 0;
        }
        if replaced {
            (state.events)(LinkEvent::Restarted(server.to_string()));
        }
        true
    };
    let envelopes = split_sequence(&response.body);
    if envelopes.is_empty() {
        return;
    }
    let first = response
        .header(FIRST_HEADER)
        .and_then(|value| value.trim().parse::<u64>().ok());
    let Some(first) = first.filter(|first| *first >= 1) else {
        return;
    };
    let taken = if current { state.mark.ack } else { sent.ack };
    let last = first.saturating_add(envelopes.len() as u64 - 1);
    for (number, envelope) in (first..).zip(envelopes) {
        if number <= taken {
            continue;
        }
        (state.events)(LinkEvent::Received(server.to_string(), envelope));
    }
    if current {
        state.mark.ack = state.mark.ack.max(last);
    }
}

/// The core's side of the client: what it transmits waits for the sender's next exchange.
#[derive(Clone)]
pub struct HttpsClientTransport {
    shared: Arc<Shared>,
}

impl Transport for HttpsClientTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if peer != self.shared.settings.server {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!("no HTTPS binding to {peer}")),
                queue_full: None,
            });
        }
        let mut state = self.shared.lock();
        if state.stopped {
            return Err(TransportFailure {
                retryable: true,
                message: Some("the HTTPS client has stopped".to_string()),
                queue_full: None,
            });
        }
        state.outbox.push_back(bytes.to_vec());
        self.shared.wake.notify_all();
        Ok(())
    }
}

/// A running client: its two threads, and the transport the core transmits through.
pub struct HttpsClient {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl HttpsClient {
    /// Start a client with `settings` over `port`, telling the host what happens through
    /// `events`, from the client's own threads. The first poll is made at once.
    pub fn start(
        settings: ClientSettings,
        port: Arc<dyn HttpsTransport>,
        events: impl FnMut(LinkEvent) + Send + 'static,
    ) -> Result<HttpsClient, ClientError> {
        Self::start_observed(settings, port, events, sce_rust_metrics::none())
    }

    /// [`Self::start`], reporting each exchange by the status its server answered to `metrics`
    /// (docs/adr/0109, D16).
    pub fn start_observed(
        settings: ClientSettings,
        port: Arc<dyn HttpsTransport>,
        events: impl FnMut(LinkEvent) + Send + 'static,
        metrics: Arc<dyn Metrics>,
    ) -> Result<HttpsClient, ClientError> {
        if !is_path_segment(&settings.own) {
            return Err(ClientError::NotAPathSegment(settings.own));
        }
        let base = settings.base.trim_end_matches('/');
        if !base_is_allowed(base) {
            return Err(ClientError::BadBase(settings.base));
        }
        // §mesh-22.1: every exchange posts to `<base>/sce-mesh/1/<client>`; the path is the only
        // place the client's machine name travels.
        let url = alloc::format!("{base}{PATH_PREFIX}{}", settings.own);
        let Some(target_uri) = normalize_target_uri(&url) else {
            return Err(ClientError::BadBase(settings.base));
        };
        let held = settings.durable.then_some(0);
        let shared = Arc::new(Shared {
            authorization: settings.credential.authorization(),
            url,
            target_uri,
            settings,
            port,
            state: Mutex::new(State {
                outbox: VecDeque::new(),
                mark: Mark::default(),
                ready: false,
                stopped: false,
                unauthenticated: false,
                held,
                events: Box::new(events),
            }),
            wake: Condvar::new(),
            metrics,
        });
        let poller = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || poll(&shared))
        };
        let sender = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || send(&shared))
        };
        Ok(HttpsClient {
            shared,
            threads: alloc::vec![poller, sender],
        })
    }

    /// The transport the core transmits to the server through.
    pub fn transport(&self) -> HttpsClientTransport {
        HttpsClientTransport {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Where the client stands in its server's mailbox: the incarnation and the highest number
    /// it handed its host (docs/adr/0109, D10). A host that checkpoints under `receipt: durable`
    /// reads this before it applies what the client handed it, so that what it applies holds at
    /// least this much, and reports it with [`Self::held`] once a checkpoint taken after is
    /// durable.
    pub fn received(&self) -> Received {
        let state = self.shared.lock();
        Received {
            epoch: state.mark.epoch.clone(),
            number: state.mark.ack,
        }
    }

    /// A durable checkpoint of the host holds every envelope up to `received`, read with
    /// [`Self::received`] before the host applied what it was handed (docs/adr/0109, D10): under
    /// `receipt: durable` every request acknowledges that much from now on, so the server
    /// deletes nothing a crash of this machine would lose. A mark of an incarnation the client
    /// has left counts in a numbering that is gone, and moves nothing; on a binding that is not
    /// durable this does nothing.
    pub fn held(&self, received: &Received) {
        let mut state = self.shared.lock();
        if state.mark.epoch != received.epoch {
            return;
        }
        if let Some(held) = state.held.as_mut() {
            *held = (*held).max(received.number);
        }
    }

    /// Stop the client: no exchange is begun after this, what waits in the outbox is handed back
    /// as [`LinkEvent::Unsent`], and the server is lost, ready or interrupted, since nothing it
    /// keeps for this machine is taken in any more. Returns once both threads ended, which is once
    /// the exchange each has in flight ended.
    pub fn stop(mut self) {
        self.signal_stop();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    fn signal_stop(&self) {
        let server = self.shared.settings.server.clone();
        let mut state = self.shared.lock();
        if state.stopped {
            return;
        }
        state.stopped = true;
        let left: Vec<Vec<u8>> = state.outbox.drain(..).collect();
        for envelope in left {
            (state.events)(LinkEvent::Unsent(server.clone(), envelope));
        }
        // A request waiting on the server can be answered only by an exchange this client makes,
        // and it makes none from now on: the link is lost whether or not it was ready.
        state.ready = false;
        (state.events)(LinkEvent::Lost(
            server,
            "the HTTPS client stopped".to_string(),
        ));
        self.shared.wake.notify_all();
    }
}

impl Drop for HttpsClient {
    /// A client dropped without [`stop`](Self::stop) stops as `stop` does, without waiting for its
    /// threads: each ends after the exchange it has in flight.
    fn drop(&mut self) {
        self.signal_stop();
    }
}

/// Whether `base` is a URL the binding posts to: `https://` with a host, or `http://` with a
/// loopback host, and in either no query or fragment (§mesh-22.1: `http://` without TLS is for a
/// loopback test only).
fn base_is_allowed(base: &str) -> bool {
    if base.contains(['?', '#']) {
        return false;
    }
    if let Some(rest) = base.strip_prefix("https://") {
        return !authority(rest).is_empty();
    }
    match base.strip_prefix("http://") {
        Some(rest) => is_loopback(host_of(authority(rest))),
        None => false,
    }
}

/// The authority of what follows a URL's scheme: everything before the first `/`.
fn authority(rest: &str) -> &str {
    rest.split('/').next().unwrap_or_default()
}

/// The host of an authority, without its port and without the brackets of an IPv6 literal.
fn host_of(authority: &str) -> &str {
    let without_user = authority.rsplit('@').next().unwrap_or_default();
    if let Some(literal) = without_user.strip_prefix('[') {
        return literal.split(']').next().unwrap_or_default();
    }
    without_user.split(':').next().unwrap_or_default()
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Wait `wait`, or less if the client stops meanwhile. Returns whether it stopped.
fn pause(shared: &Shared, wait: Duration) -> bool {
    let state = shared.lock();
    if state.stopped {
        return true;
    }
    let (state, _) = shared
        .wake
        .wait_timeout_while(state, wait, |state| !state.stopped)
        .expect(HELD);
    state.stopped
}

/// The poller: one held exchange after another, carrying nothing.
fn poll(shared: &Shared) {
    let mut failures = 0u32;
    loop {
        let mark = {
            let state = shared.lock();
            if state.stopped {
                return;
            }
            state.acknowledging()
        };
        let (outcome, sent) = exchange(shared, Vec::new(), &mark, shared.settings.hold);
        if let Then::Wait(wait) = take_outcome(shared, Vec::new(), &sent, outcome, &mut failures) {
            if pause(shared, wait) {
                return;
            }
        }
    }
}

/// Make one exchange carrying `body`, acknowledging `mark` and asking for `hold`: what the port
/// answered, and what the request was sent with. A request that could not be made is answered as
/// a failure at the transport, and was sent with nothing.
fn exchange(
    shared: &Shared,
    body: Vec<u8>,
    mark: &Mark,
    hold: Duration,
) -> (Result<HttpsResponse, HttpsFailure>, Sent) {
    match shared.request(body, mark, hold) {
        Ok((request, sent)) => (shared.port.exchange(request), sent),
        Err(failure) => (
            Err(failure),
            Sent {
                mark: mark.clone(),
                answered: None,
            },
        ),
    }
}

/// The sender: an exchange that asks for no hold whenever the outbox holds something.
fn send(shared: &Shared) {
    let mut failures = 0u32;
    loop {
        let (carried, mark) = {
            let state = shared.lock();
            let mut state = shared
                .wake
                .wait_while(state, |state| state.outbox.is_empty() && !state.stopped)
                .expect(HELD);
            if state.stopped {
                return;
            }
            let mut carried = Vec::new();
            let mut size = 0usize;
            while let Some(length) = state.outbox.front().map(Vec::len) {
                let grown = size.saturating_add(length);
                if !carried.is_empty() && grown > shared.settings.max_body_bytes {
                    break;
                }
                size = grown;
                if let Some(envelope) = state.outbox.pop_front() {
                    carried.push(envelope);
                }
            }
            (carried, state.acknowledging())
        };
        let body = super::join_sequence(carried.iter().map(Vec::as_slice));
        let (outcome, sent) = exchange(shared, body, &mark, Duration::ZERO);
        if let Then::Wait(wait) = take_outcome(shared, carried, &sent, outcome, &mut failures) {
            if pause(shared, wait) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    type Answers = Mutex<VecDeque<Result<HttpsResponse, HttpsFailure>>>;

    /// A port that answers from two scripts, one for the polls (requests that carry nothing) and
    /// one for the requests that carry envelopes, so a test can aim an answer at either thread;
    /// and that records what it was asked.
    #[derive(Default)]
    struct Scripted {
        polls: Answers,
        sends: Answers,
        asked: Mutex<Vec<HttpsRequest>>,
    }

    impl HttpsTransport for Scripted {
        fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
            self.asked.lock().unwrap().push(request.clone());
            let is_poll = request.body.is_empty();
            let script = if is_poll { &self.polls } else { &self.sends };
            let next = script.lock().unwrap().pop_front();
            match next {
                Some(answer) => answer,
                // Out of script: an exchange taken in with nothing to answer, after a little
                // while for a poll, so a test's loop does not spin.
                None => {
                    if is_poll {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Ok(ok(None, Vec::new()))
                }
            }
        }
    }

    /// The incarnation the scripted server's mailbox numbers in, and one it numbered in before it
    /// started again.
    const EPOCH: &str = "0123456789abcdef0123456789abcdef";
    const EARLIER: &str = "fedcba9876543210fedcba9876543210";

    /// A `200` of the mailbox incarnation `epoch`, its body's first number `first`.
    fn ok_in(epoch: &str, first: Option<u64>, body: Vec<u8>) -> HttpsResponse {
        let mut headers = alloc::vec![(EPOCH_HEADER.to_string(), epoch.to_string())];
        if let Some(first) = first {
            headers.push((FIRST_HEADER.to_string(), first.to_string()));
        }
        HttpsResponse {
            status: 200,
            headers,
            body,
        }
    }

    fn ok(first: Option<u64>, body: Vec<u8>) -> HttpsResponse {
        ok_in(EPOCH, first, body)
    }

    /// Where a request's acknowledgement stood, in `epoch`.
    fn mark(ack: u64, epoch: Option<&str>) -> Mark {
        Mark {
            ack,
            epoch: epoch.map(str::to_string),
        }
    }

    /// The value of the header `name` of `request`.
    fn header_of(request: &HttpsRequest, name: &str) -> Option<String> {
        request
            .headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    fn status(code: u16, headers: &[(&str, &str)]) -> HttpsResponse {
        HttpsResponse {
            status: code,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Vec::new(),
        }
    }

    fn settings() -> ClientSettings {
        ClientSettings {
            hold: Duration::from_millis(500),
            backoff: Backoff {
                initial: Duration::from_millis(1),
                max: Duration::from_millis(1),
                factor: 1,
                jitter: false,
            },
            ..ClientSettings::new(
                "https://cal.example/api",
                "phone",
                "cloud",
                Credential::Bearer("t".to_string()),
            )
        }
    }

    /// A bare state for the pure rules, its events into a channel.
    fn state() -> (State, mpsc::Receiver<LinkEvent>) {
        let (tx, rx) = mpsc::channel();
        (
            State {
                outbox: VecDeque::new(),
                mark: Mark::default(),
                ready: true,
                stopped: false,
                unauthenticated: false,
                held: None,
                events: Box::new(move |event| {
                    let _ = tx.send(event);
                }),
            },
            rx,
        )
    }

    fn received(rx: &mpsc::Receiver<LinkEvent>) -> Vec<Vec<u8>> {
        rx.try_iter()
            .map(|event| match event {
                LinkEvent::Received(peer, bytes) => {
                    assert_eq!(peer, "cloud");
                    bytes
                }
                other => panic!("expected only arrivals, got {other:?}"),
            })
            .collect()
    }

    /// Three one-byte CBOR items (the unsigned integers 1, 2 and 3), which split as three.
    fn three() -> Vec<u8> {
        alloc::vec![0x01, 0x02, 0x03]
    }

    #[test]
    fn a_response_that_continues_the_numbering_is_taken_once() {
        let (mut state, rx) = state();
        state.mark = mark(4, Some(EPOCH));
        let sent = state.mark.clone();
        // The request acknowledged 4; the mailbox answers 5, 6, 7.
        take_mailbox(&mut state, "cloud", &sent, &ok(Some(5), three()));
        assert_eq!(received(&rx), [[1], [2], [3]]);
        assert_eq!(state.mark, mark(7, Some(EPOCH)));
        // The other thread's request also acknowledged 4 and was answered 5 to 8: only 8 is new.
        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok(Some(5), alloc::vec![0x01, 0x02, 0x03, 0x04]),
        );
        assert_eq!(received(&rx), [[4]]);
        assert_eq!(state.mark, mark(8, Some(EPOCH)));
    }

    /// A server that started again with its mailboxes empty numbers them from 1 in a new
    /// incarnation: the acknowledgement starts over in it, so what it sends is all new though its
    /// numbers are below what the client acknowledged. The other thread's response to a request
    /// sent in the earlier incarnation, answered in the new one, is taken from where the new
    /// numbering stands; one answered in the earlier incarnation is handed on above what its
    /// request acknowledged, and leaves the acknowledgement in the new one. The host is told the
    /// server restarted once, when the new incarnation replaces the one the client knew, and
    /// before what it sends: neither the other thread's response in the new one nor the late one
    /// in the earlier one is a restart.
    #[test]
    fn a_new_incarnation_starts_the_acknowledgement_over() {
        let (mut state, rx) = state();
        state.mark = mark(57, Some(EARLIER));
        let sent = state.mark.clone();
        take_mailbox(&mut state, "cloud", &sent, &ok(Some(1), three()));
        assert_eq!(
            rx.try_recv().ok(),
            Some(LinkEvent::Restarted("cloud".to_string())),
            "the mailbox the client knew is gone"
        );
        assert_eq!(received(&rx), [[1], [2], [3]]);
        assert_eq!(state.mark, mark(3, Some(EPOCH)));

        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok(Some(1), alloc::vec![0x01, 0x02, 0x03, 0x04]),
        );
        assert_eq!(received(&rx), [[4]], "taken once in the new numbering");
        assert_eq!(state.mark, mark(4, Some(EPOCH)));

        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok_in(EARLIER, Some(57), alloc::vec![0x09, 0x0a]),
        );
        assert_eq!(
            received(&rx),
            [[0x0a]],
            "above what the request acknowledged"
        );
        assert_eq!(
            state.mark,
            mark(4, Some(EPOCH)),
            "the newer incarnation stays"
        );
    }

    #[test]
    fn a_body_without_a_number_or_an_incarnation_is_not_taken() {
        let (mut state, rx) = state();
        take_mailbox(&mut state, "cloud", &Mark::default(), &ok(None, three()));
        assert!(received(&rx).is_empty());
        assert_eq!(
            state.mark,
            mark(0, Some(EPOCH)),
            "the incarnation is learnt"
        );
        take_mailbox(&mut state, "cloud", &Mark::default(), &ok(Some(0), three()));
        assert!(received(&rx).is_empty(), "mailbox numbers start at 1");
        let unnamed = HttpsResponse {
            status: 200,
            headers: alloc::vec![(FIRST_HEADER.to_string(), "1".to_string())],
            body: three(),
        };
        take_mailbox(&mut state, "cloud", &Mark::default(), &unnamed);
        assert!(received(&rx).is_empty(), "no incarnation names the numbers");
        assert_eq!(state.mark, mark(0, Some(EPOCH)));
    }

    fn dialed(authenticator: HttpsAuthenticator) -> Peer {
        Peer {
            name: "cloud",
            config: crate::router::PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: crate::inbound::Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &["cloud"],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsDial {
                url: "https://cal.example/api",
                authenticator,
                hold_ms: 9_000,
                verify: None,
                receipt: Receipt::Transport,
            },
        }
    }

    /// A peer table's dialing link gives the client its URL, its server's name and its hold, and
    /// only with a credential of the kind the link's authenticator names.
    #[test]
    fn a_dialing_link_gives_the_settings_and_takes_only_its_own_credential() {
        let peer = dialed(HttpsAuthenticator::Install);
        let settings =
            ClientSettings::from_peer("phone", &peer, Credential::Install([1; 32]), None)
                .expect("an installation secret for an `install` link");
        assert_eq!(settings.base, "https://cal.example/api");
        assert_eq!(settings.own, "phone");
        assert_eq!(settings.server, "cloud");
        assert_eq!(settings.hold, Duration::from_millis(9_000));
        assert_eq!(settings.verify, None);
        assert!(ClientSettings::from_peer("phone", &peer, Credential::Mtls, None).is_none());
        let bearer = dialed(HttpsAuthenticator::Bearer);
        assert!(
            ClientSettings::from_peer("phone", &bearer, Credential::Bearer("t".into()), None)
                .is_some()
        );
        let accepted = Peer {
            link: PeerLink::ZenohInbox,
            ..bearer
        };
        assert!(ClientSettings::from_peer(
            "phone",
            &accepted,
            Credential::Bearer("t".into()),
            None
        )
        .is_none());
    }

    /// docs/adr/0109, D9: a link that names `verify` gives the client the trust set's keys and is
    /// refused without them, one that names none is refused keys, and the `key` authenticator
    /// takes a key credential. Red if a deployment that asks for verification can be run without
    /// it.
    #[test]
    fn a_dialing_link_that_verifies_is_never_run_without_its_trust_set() {
        let keys = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: [7u8; 32],
        }];
        let verifying = Peer {
            link: PeerLink::HttpsDial {
                url: "https://cal.example/api",
                authenticator: HttpsAuthenticator::Key,
                hold_ms: 9_000,
                verify: Some("servers"),
                receipt: Receipt::Transport,
            },
            ..dialed(HttpsAuthenticator::Key)
        };
        let key = || {
            Credential::Key(super::super::signature::RequestSigner::new(
                sce_rust_crypto::ed25519::KeyPair::from_seed(&[5u8; 32]),
            ))
        };
        let settings = ClientSettings::from_peer("phone", &verifying, key(), Some(keys.clone()))
            .expect("a key and the trust set");
        assert_eq!(settings.verify, Some(keys.clone()));
        assert!(
            ClientSettings::from_peer("phone", &verifying, key(), None).is_none(),
            "a link that verifies, given no trust set"
        );
        assert!(
            ClientSettings::from_peer("phone", &dialed(HttpsAuthenticator::Key), key(), Some(keys))
                .is_none(),
            "keys for a link that verifies nothing"
        );
        assert!(
            ClientSettings::from_peer(
                "phone",
                &dialed(HttpsAuthenticator::Key),
                Credential::Bearer("t".into()),
                None
            )
            .is_none(),
            "a token for the `key` authenticator"
        );
    }

    #[test]
    fn a_base_is_https_or_loopback_http_without_query_or_fragment() {
        for good in [
            "https://cal.example",
            "https://cal.example:8443/api",
            "http://127.0.0.1:9000",
            "http://localhost/base",
            "http://[::1]:80",
        ] {
            assert!(base_is_allowed(good), "{good}");
        }
        for bad in [
            "https://",
            "http://cal.example",
            "http://10.0.0.1:80",
            "https://cal.example/?q",
            "https://cal.example/#f",
            "wss://cal.example",
            "cal.example",
        ] {
            assert!(!base_is_allowed(bad), "{bad}");
        }
    }

    #[test]
    fn a_client_is_refused_a_name_or_a_base_the_binding_cannot_post_under() {
        let port: Arc<dyn HttpsTransport> = Arc::new(Scripted::default());
        let bad_name = ClientSettings {
            own: "a/b".to_string(),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_name, Arc::clone(&port), |_| {}),
            Err(ClientError::NotAPathSegment(_))
        ));
        let bad_base = ClientSettings {
            base: "http://cal.example".to_string(),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_base, port, |_| {}),
            Err(ClientError::BadBase(_))
        ));
    }

    fn next(rx: &mpsc::Receiver<LinkEvent>) -> LinkEvent {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("a link event within five seconds")
    }

    /// The first exchange answered `200` makes the server ready, a transport failure interrupts
    /// it (section 22.4: the mailbox and every reply in it stay on the server, so it is not lost)
    /// and hands back what the failed exchange carried, and the next `200` makes it ready again.
    #[test]
    fn readiness_follows_the_exchanges_and_a_failure_hands_back_what_it_carried() {
        let port = Arc::new(Scripted::default());
        port.polls.lock().unwrap().extend([
            Ok(ok(None, Vec::new())),
            Err(HttpsFailure::Connect),
            Ok(ok(Some(1), alloc::vec![0x05])),
        ]);
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert!(matches!(next(&rx), LinkEvent::Interrupted(peer, _) if peer == "cloud"));
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        let first = &asked[0];
        assert_eq!(first.url, "https://cal.example/api/sce-mesh/1/phone");
        assert!(first.body.is_empty(), "a poll carries nothing");
        let header = |name: &str| {
            first
                .headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(header("Content-Type").as_deref(), Some(CONTENT_TYPE));
        assert_eq!(header(ACK_HEADER).as_deref(), Some("0"));
        assert_eq!(header(HOLD_HEADER).as_deref(), Some("500"));
        assert_eq!(header("Authorization").as_deref(), Some("Bearer t"));
        assert_eq!(
            first.timeout,
            Duration::from_millis(500) + Duration::from_secs(30)
        );
        // The exchange after the one that took mailbox number 1 acknowledges it.
        assert!(asked[3..].iter().all(|request| request
            .headers
            .contains(&(ACK_HEADER.to_string(), "1".to_string()))));
    }

    /// A client that stops makes no exchange any more, so nothing the server keeps for it is
    /// taken in: its server is lost though the link was only interrupted when it stopped, and a
    /// request the core still waits on is let go. Red if a stop loses only a ready server.
    #[test]
    fn a_client_that_stops_while_interrupted_loses_its_server() {
        let port = Arc::new(Scripted::default());
        port.polls
            .lock()
            .unwrap()
            .extend([Ok(ok(None, Vec::new())), Err(HttpsFailure::Connect)]);
        let (tx, rx) = mpsc::channel();
        // A long backoff after the failure, so no poll makes the server ready again before the
        // stop; the stop cuts the wait short.
        let slow = ClientSettings {
            backoff: Backoff {
                initial: Duration::from_secs(60),
                max: Duration::from_secs(60),
                factor: 1,
                jitter: false,
            },
            ..settings()
        };
        let client = HttpsClient::start(slow, port, move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert!(matches!(next(&rx), LinkEvent::Interrupted(peer, _) if peer == "cloud"));
        client.stop();
        assert!(matches!(next(&rx), LinkEvent::Lost(peer, _) if peer == "cloud"));
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "lost once"
        );
    }

    /// A server that started again names a new incarnation: the client's acknowledgement starts
    /// over in it, the first envelope of the new numbering reaches the core though its number is
    /// one the client acknowledged before, a response that carries it again does not hand it on a
    /// second time, and every request names the incarnation its acknowledgement counts in.
    #[test]
    fn a_new_incarnation_starts_the_acknowledgement_over_and_delivers_once() {
        let port = Arc::new(Scripted::default());
        port.polls.lock().unwrap().extend([
            Ok(ok_in(EARLIER, Some(1), alloc::vec![0x05])),
            Ok(ok(Some(1), alloc::vec![0x06])),
            Ok(ok(Some(1), alloc::vec![0x06])),
        ]);
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        assert_eq!(next(&rx), LinkEvent::Restarted("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x06])
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while port.asked.lock().unwrap().len() < 4 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "carried again, and handed on once"
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        assert!(asked.len() >= 4, "{} exchanges", asked.len());
        let acknowledged = |at: usize| {
            (
                header_of(&asked[at], ACK_HEADER),
                header_of(&asked[at], EPOCH_HEADER),
            )
        };
        assert_eq!(acknowledged(0), (Some("0".to_string()), None));
        assert_eq!(
            acknowledged(1),
            (Some("1".to_string()), Some(EARLIER.to_string()))
        );
        assert_eq!(
            acknowledged(2),
            (Some("1".to_string()), Some(EPOCH.to_string()))
        );
        assert_eq!(
            acknowledged(3),
            (Some("1".to_string()), Some(EPOCH.to_string()))
        );
    }

    /// What the core transmits is carried by an exchange that asks for no hold; a refusal the
    /// server answered hands it back unsent and leaves the server ready.
    #[test]
    fn a_transmitted_envelope_is_carried_at_once_and_a_refusal_hands_it_back() {
        let port = Arc::new(Scripted::default());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));

        port.sends
            .lock()
            .unwrap()
            .push_back(Ok(status(401, &[("WWW-Authenticate", "Bearer")])));
        let mut transport = client.transport();
        transport.transmit("cloud", &[0x09]).unwrap();
        assert_eq!(
            next(&rx),
            LinkEvent::Unsent("cloud".to_string(), alloc::vec![0x09])
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "a refusal is not a lost server"
        );
        let carried = port
            .asked
            .lock()
            .unwrap()
            .iter()
            .find(|request| !request.body.is_empty())
            .cloned()
            .expect("an exchange carried the envelope");
        assert_eq!(carried.body, [0x09]);
        assert!(carried
            .headers
            .contains(&(HOLD_HEADER.to_string(), "0".to_string())));

        assert!(
            transport.transmit("elsewhere", &[0x01]).is_err(),
            "the binding reaches its one server"
        );
        client.stop();
        assert!(matches!(next(&rx), LinkEvent::Lost(peer, _) if peer == "cloud"));
        assert!(transport.transmit("cloud", &[0x01]).is_err(), "stopped");
    }

    /// docs/adr/0109, D10: under `receipt: durable` a client acknowledges only what its host
    /// reported a durable checkpoint holds, never merely what it handed on, and a mark of an
    /// incarnation it left moves nothing; each `200` that carried what the core transmitted is
    /// told to the host as `Carried`. Red if `State::acknowledging` answers `mark` alone, or the
    /// `200` branch of `take_outcome` tells no `Carried`.
    #[test]
    fn a_durable_client_acknowledges_only_what_its_host_holds_and_tells_what_a_200_carried() {
        let port = Arc::new(Scripted::default());
        port.polls
            .lock()
            .unwrap()
            .push_back(Ok(ok(Some(1), three())));
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(
            ClientSettings {
                durable: true,
                ..settings()
            },
            port.clone(),
            move |event| {
                let _ = tx.send(event);
            },
        )
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        for _ in 0..3 {
            assert!(matches!(next(&rx), LinkEvent::Received(..)));
        }
        let acknowledged = |port: &Scripted| {
            std::thread::sleep(Duration::from_millis(150));
            port.asked
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|request| request.body.is_empty())
                .and_then(|request| header_of(request, ACK_HEADER))
        };
        assert_eq!(
            acknowledged(&port),
            Some("0".to_string()),
            "nothing a checkpoint holds yet"
        );
        let received = client.received();
        assert_eq!(
            received,
            Received {
                epoch: Some(EPOCH.to_string()),
                number: 3
            }
        );
        client.held(&Received {
            epoch: Some(EARLIER.to_string()),
            number: 3,
        });
        assert_eq!(
            acknowledged(&port),
            Some("0".to_string()),
            "a mark of an incarnation the client left moves nothing"
        );
        client.held(&received);
        assert_eq!(acknowledged(&port), Some("3".to_string()));

        let mut transport = client.transport();
        transport.transmit("cloud", &[0x09]).unwrap();
        assert_eq!(
            next(&rx),
            LinkEvent::Carried("cloud".to_string(), alloc::vec![alloc::vec![0x09]])
        );
        client.stop();
    }

    /// A server that sheds load is waited out, and what the shed exchange carried goes again; it
    /// is neither unsent nor a lost server.
    #[test]
    fn a_shed_exchange_goes_again_after_retry_after() {
        let port = Arc::new(Scripted::default());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        {
            let mut sends = port.sends.lock().unwrap();
            sends.push_back(Ok(status(503, &[("Retry-After", "0")])));
            sends.push_back(Ok(ok(None, Vec::new())));
        }
        client.transport().transmit("cloud", &[0x0a]).unwrap();
        let carried = || {
            port.asked
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.body == [0x0a])
                .count()
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while carried() < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(carried(), 2, "carried, shed, carried again");
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "nothing to tell the host"
        );
        client.stop();
    }

    use super::super::signature::{
        digest_matches, RequestSignature, RequestSigner, ResponseSigner,
    };
    use sce_rust_crypto::ed25519::KeyPair;

    /// A port that answers every exchange as a server under `sign:` does: mailbox number 1,
    /// `[0x05]`, in [`EPOCH`] until a request acknowledges it, then nothing, each response signed
    /// for the request it answers. The first `tampered` responses that carry the envelope have
    /// `Sce-Mesh-First` rewritten after signing, as an intermediary that alters the exchange
    /// would. Records what it was asked.
    struct SigningPort {
        signer: ResponseSigner,
        tampered: Mutex<usize>,
        asked: Mutex<Vec<HttpsRequest>>,
    }

    impl HttpsTransport for SigningPort {
        fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
            self.asked.lock().unwrap().push(request.clone());
            let header = |name: &str| field_value(&request.headers, name).unwrap_or_default();
            let acknowledged = header(ACK_HEADER).parse::<u64>().unwrap_or(0) >= 1;
            let body = if acknowledged {
                // An empty mailbox holds a poll a little, so a test's loop does not spin.
                std::thread::sleep(Duration::from_millis(20));
                Vec::new()
            } else {
                alloc::vec![0x05]
            };
            let first = (!body.is_empty()).then_some("1");
            let answered = Answered {
                target_uri: normalize_target_uri(&request.url).expect("a URL the client posts to"),
                ack: header(ACK_HEADER),
                nonce: header(NONCE_HEADER),
            };
            let digest = content_digest(&body);
            let mut headers = alloc::vec![
                (EPOCH_HEADER.to_string(), EPOCH.to_string()),
                (CONTENT_DIGEST_HEADER.to_string(), digest.clone()),
            ];
            if let Some(first) = first {
                headers.push((FIRST_HEADER.to_string(), first.to_string()));
            }
            headers.extend(self.signer.sign(&digest, EPOCH, first, &answered));
            let mut tampered = self.tampered.lock().unwrap();
            if first.is_some() && *tampered > 0 {
                *tampered -= 1;
                for (name, value) in headers.iter_mut() {
                    if name == FIRST_HEADER {
                        *value = "2".to_string();
                    }
                }
            }
            Ok(HttpsResponse {
                status: 200,
                headers,
                body,
            })
        }
    }

    /// SCE_MESH.md section 22.9: under `verify:`, two responses whose `Sce-Mesh-First` an
    /// intermediary rewrote are taken nothing of: no envelope reaches the core, the server is not
    /// made ready, every request meanwhile acknowledges nothing, and the host is told once (row
    /// 19). The next response, which verifies, is taken as it is. Red if the client takes an
    /// unverified `200` (the envelope would arrive numbered 2 and be acknowledged), or tells each
    /// one.
    #[test]
    fn a_response_that_does_not_verify_is_taken_nothing_of_and_told_once() {
        let key = KeyPair::from_seed(&[3u8; 32]);
        let port = Arc::new(SigningPort {
            signer: ResponseSigner::new("server-2026", key.clone()),
            tampered: Mutex::new(2),
            asked: Mutex::new(Vec::new()),
        });
        let verifying = ClientSettings {
            verify: Some(alloc::vec![TrustedKey {
                id: "server-2026".to_string(),
                public_key: key.public_key(),
            }]),
            ..settings()
        };
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(verifying, port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert!(
            matches!(next(&rx), LinkEvent::Unauthenticated(peer, _) if peer == "cloud"),
            "told once, before anything is taken"
        );
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "the second altered response is not told again, and the envelope arrives once"
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        assert!(asked.len() >= 4, "{} exchanges", asked.len());
        for request in &asked[..3] {
            assert_eq!(
                header_of(request, ACK_HEADER).as_deref(),
                Some("0"),
                "nothing is acknowledged before a response verified"
            );
        }
        assert_eq!(header_of(&asked[3], ACK_HEADER).as_deref(), Some("1"));
        let nonces: Vec<String> = asked
            .iter()
            .map(|request| header_of(request, NONCE_HEADER).expect("a nonce on every request"))
            .collect();
        assert_ne!(nonces[0], nonces[1], "a nonce is drawn for each exchange");
    }

    /// SCE_MESH.md section 22.9: a client of the `key` authenticator signs every request, a poll
    /// and a send alike, over its body's digest, sends no `Authorization`, and offers its public
    /// key. Red if a request leaves unsigned or its signature misses a covered field.
    #[test]
    fn a_client_of_the_key_authenticator_signs_every_request() {
        let key = KeyPair::from_seed(&[6u8; 32]);
        let port = Arc::new(Scripted::default());
        let signing = ClientSettings {
            credential: Credential::Key(RequestSigner::new(key.clone())),
            ..settings()
        };
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(signing, port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        client.transport().transmit("cloud", &[0x09]).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !port
            .asked
            .lock()
            .unwrap()
            .iter()
            .any(|request| !request.body.is_empty())
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        client.stop();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let asked = port.asked.lock().unwrap();
        assert!(asked.iter().any(|request| !request.body.is_empty()));
        for request in asked.iter() {
            let field = |name: &str| field_value(&request.headers, name);
            assert_eq!(field("Authorization"), None);
            assert!(digest_matches(
                &field(CONTENT_DIGEST_HEADER).expect("a digest"),
                &request.body
            ));
            let target = normalize_target_uri(&request.url).expect("a target");
            let signature = RequestSignature::read(&field).expect("a signature of the profile");
            assert_eq!(
                signature.verify(&key.public_key(), &target, &field, now, 300),
                Ok(()),
                "{request:?}"
            );
        }
    }

    /// docs/adr/0109, D16: every exchange the client makes is reported by the status its server
    /// answered, and one that got no response as `none`. Red if `take_outcome` reports a failure
    /// at the transport under a status, or reports a `401` as anything but `401`.
    #[test]
    fn every_exchange_is_reported_by_the_status_its_server_answered() {
        use sce_rust_metrics::recording::Recording;

        let port = Arc::new(Scripted::default());
        {
            let mut polls = port.polls.lock().unwrap();
            polls.push_back(Ok(ok(None, Vec::new())));
            polls.push_back(Err(HttpsFailure::Connect));
            polls.push_back(Ok(status(401, &[("WWW-Authenticate", "Bearer")])));
            polls.push_back(Ok(status(502, &[])));
        }
        let recording = Arc::new(Recording::new());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start_observed(
            settings(),
            port.clone(),
            move |event| {
                let _ = tx.send(event);
            },
            Arc::clone(&recording) as Arc<dyn Metrics>,
        )
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        let counted = |status| recording.counted(Counter::ClientExchanges { status });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while counted(ExchangeStatus::Other) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        client.stop();
        assert!(counted(ExchangeStatus::Ok) >= 1);
        assert_eq!(counted(ExchangeStatus::NoResponse), 1);
        assert_eq!(counted(ExchangeStatus::Unauthorized), 1);
        assert_eq!(
            counted(ExchangeStatus::Other),
            1,
            "a 502 of an intermediary"
        );
        assert_eq!(
            recording.counted_in(&sce_rust_metrics::SERVER_EXCHANGES),
            0,
            "a client reports no exchange of a server's"
        );
    }
}
