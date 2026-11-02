// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The WebSocket binding of the relay: a request text goes in as a text message and the response
//! text comes back as one, in order, one connection for one device.
//!
//! A device connects to [`PATH`] with the token its login earned as `Authorization: Bearer
//! <token>`. The upgrade is refused (401) when the relay did not issue the token or it has expired,
//! and every request on the connection is judged again by the relay, so a token that expires on a
//! connection that stays open stops being answered and the device reconnects with a new one. A
//! relay that offers login (ADR 0080, [`Hub::open_anonymous`]) takes as well an upgrade that
//! presents no token, or one it issued that expired: the connection is anonymous, asks for a
//! challenge and logs in, and is held to its own bounds (a few requests, a short idle time, and its
//! source's allowance, the source read from `X-Forwarded-For` only behind a proxy the server is told
//! to trust, [`Server::trusting`]); one that presents a header the relay did not issue is still
//! refused, so that a device knows its token went stale. A source that opened as many anonymous
//! connections as it may this minute is refused (429). A request to log in is answered on any
//! connection ([`Hub::answer`]). A
//! binary message, or a text one longer than the relay takes, closes the connection. The certificate
//! is the deployer's: [`serve_tls`] terminates TLS with it (on the configuration
//! `crate::tls::server_config` makes, `ring`'s cryptography), and [`serve_connection`] runs on any
//! stream the host hands it, one its own TLS acceptor already wrapped included. The client
//! ([`WssLink`], `crate::wss`) dials `ws://` or `wss://`.
//!
//! The relay's work is short and takes a lock, so it runs on a blocking thread and never holds up
//! the connections that are waiting. What answers is a [`Hub`]: a relay behind one lock, or
//! [`crate::hub::Shards`], whose requests for different lists run at once.
//!
//! [`serve`] and [`serve_tls`] run for as long as their listener lives and hold a connection to
//! nothing but its messages. A relay that is deployed runs a [`Server`]: the connections it holds
//! open at once, the time a connection has to finish its handshake and the time it may wait for a
//! request are bounded ([`Bounds`]), and a [`Stop`] closes its listeners and then each connection
//! once it has answered the request it is answering, which [`Server::drain`] waits for.

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::hub::Hub;
use crate::limits::Limits;
use crate::login::{self, Opened};
use crate::tls::ServerConfig;
use crate::token::TokenError;
// The path a device connects to, and the client that connects there: the binding's two halves are
// named from either module.
pub use crate::wss::{WssLink, PATH};

/// Why a connection did not run.
#[derive(Debug)]
pub enum ServeError {
    /// The upgrade did not complete: not the relay's path, or no token the relay issued.
    Handshake(String),
}

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

fn refusal(status: http::StatusCode, why: &str) -> ErrorResponse {
    let mut response = http::Response::new(Some(why.to_string()));
    *response.status_mut() = status;
    response
}

/// Run one connection until it closes: answer each text message with the relay's response.
pub async fn serve_connection<S, H>(
    stream: S,
    hub: Arc<H>,
    clock: Clock,
    max_message: usize,
) -> Result<(), ServeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
    run_connection(stream, None, hub, clock, max_message).await
}

/// [`serve_connection`] for a stream from `peer`, when the listener said where it came from.
async fn run_connection<S, H>(
    stream: S,
    peer: Option<SocketAddr>,
    hub: Arc<H>,
    clock: Clock,
    max_message: usize,
) -> Result<(), ServeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
    let (ws, caller) = upgrade(stream, hub.as_ref(), &clock, max_message, peer, &[]).await?;
    exchange(ws, &caller, &hub, &clock, Watch::default()).await;
    Ok(())
}

/// Who a connection's requests are made by: the token it presented, when the hub took it or issued
/// it and it expired, where it comes from, and whether it is anonymous (it presented no token the
/// hub takes, and is held to the bounds of one).
struct Caller {
    token: Option<String>,
    source: Option<IpAddr>,
    anonymous: bool,
}

/// Complete the upgrade of `stream`, from `peer` behind the `proxies` the server trusts, when it
/// asks for the relay's path with a token the hub takes, or with none (or one the hub issued that
/// expired) when the hub takes an anonymous connection from its source: the connection, and who its
/// requests are made by.
async fn upgrade<S, H>(
    stream: S,
    hub: &H,
    clock: &Clock,
    max_message: usize,
    peer: Option<SocketAddr>,
    proxies: &[IpAddr],
) -> Result<(WebSocketStream<S>, Caller), ServeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
    let mut caller: Option<Caller> = None;
    let config = WebSocketConfig::default()
        .max_message_size(Some(max_message))
        .max_frame_size(Some(max_message));
    // The callback's error is `ErrorResponse` because tungstenite's `Callback` contract says so; its
    // size is the library's, not this crate's to box.
    #[allow(clippy::result_large_err)]
    let verdict = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
        if request.uri().path() != PATH {
            return Err(refusal(http::StatusCode::NOT_FOUND, "no such relay path"));
        }
        let headers = request.headers();
        let forwarded: Vec<&str> = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();
        let source = peer.map(|peer| login::source_behind(peer.ip(), &forwarded, proxies));
        let presented = headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        let now = clock();
        match presented.map(|token| hub.authenticate(token, now)) {
            Some(Ok(_)) => {
                caller = Some(Caller {
                    token: presented.map(str::to_string),
                    source,
                    anonymous: false,
                });
                return Ok(response);
            }
            // A token the relay issued that expired, or none: an anonymous connection, when the
            // relay offers login.
            Some(Err(TokenError::Expired)) | None => {}
            Some(Err(_)) => {
                return Err(refusal(
                    http::StatusCode::UNAUTHORIZED,
                    "not a token of this relay",
                ))
            }
        }
        match hub.open_anonymous(source, now) {
            Opened::Taken => {
                caller = Some(Caller {
                    token: presented.map(str::to_string),
                    source,
                    anonymous: true,
                });
                Ok(response)
            }
            Opened::NotOffered => Err(refusal(
                http::StatusCode::UNAUTHORIZED,
                if presented.is_some() {
                    "not a token of this relay"
                } else {
                    "no token"
                },
            )),
            Opened::RateLimited { retry_after_ms } => {
                let mut refused = refusal(
                    http::StatusCode::TOO_MANY_REQUESTS,
                    "this address opened as many connections with no token as it may",
                );
                if let Ok(value) =
                    http::HeaderValue::from_str(&retry_after_ms.div_ceil(1000).to_string())
                {
                    refused
                        .headers_mut()
                        .insert(http::header::RETRY_AFTER, value);
                }
                Err(refused)
            }
        }
    };
    let ws = tokio_tungstenite::accept_hdr_async_with_config(stream, verdict, Some(config))
        .await
        .map_err(|error| ServeError::Handshake(error.to_string()))?;
    let caller = caller.expect("an upgrade completes only after the callback said who calls");
    Ok((ws, caller))
}

/// What an open connection is held to beyond its messages: how long it may wait for a request (and
/// for its answer to be taken), how many requests it may make, the stop that closes it, and where
/// its requests are counted.
#[derive(Default)]
struct Watch<'a> {
    idle: Option<Duration>,
    requests: Option<u32>,
    stop: Option<&'a Stop>,
    tally: Option<&'a Tally>,
}

/// Answer each text message of `ws` with the hub's response, until the connection closes, a
/// message that is not a request comes, or `watch` closes it. A request that was read is always
/// answered: the stop and the idle bound are looked at only while the connection waits for one,
/// and a connection that made as many requests as it may is closed once the last is answered.
async fn exchange<S, H>(
    mut ws: WebSocketStream<S>,
    caller: &Caller,
    hub: &Arc<H>,
    clock: &Clock,
    watch: Watch<'_>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
    let mut answered: u32 = 0;
    loop {
        // The stop first: a server that is stopping answers no request it has not begun.
        let next = tokio::select! {
            biased;
            () = raised(watch.stop) => break,
            next = ws.next() => next,
            () = after(watch.idle) => break,
        };
        let Some(Ok(message)) = next else { break };
        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => break,
        };
        let answer = {
            let (hub, clock, token, source) = (
                hub.clone(),
                clock.clone(),
                caller.token.clone(),
                caller.source,
            );
            tokio::task::spawn_blocking(move || {
                hub.answer(token.as_deref(), source, &text, clock())
            })
            .await
        };
        let Ok(answer) = answer else { break };
        if let Some(tally) = watch.tally {
            tally.requests.fetch_add(1, Ordering::Relaxed);
        }
        let sent = within(watch.idle, ws.send(Message::Text(answer.into()))).await;
        if !matches!(sent, Some(Ok(()))) {
            break;
        }
        answered = answered.saturating_add(1);
        if watch.requests.is_some_and(|most| answered >= most) {
            break;
        }
    }
    let _ = within(watch.idle, ws.close(None)).await;
}

/// The end of `bound`, or never when there is none.
async fn after(bound: Option<Duration>) {
    match bound {
        Some(bound) => tokio::time::sleep(bound).await,
        None => std::future::pending().await,
    }
}

/// When `stop` is raised, or never when there is none.
async fn raised(stop: Option<&Stop>) {
    match stop {
        Some(stop) => stop.wait().await,
        None => std::future::pending().await,
    }
}

/// `work`, given at most `bound` when there is one: `None` when the bound passed first.
async fn within<T>(bound: Option<Duration>, work: impl Future<Output = T>) -> Option<T> {
    match bound {
        Some(bound) => tokio::time::timeout(bound, work).await.ok(),
        None => Some(work.await),
    }
}

/// Accept connections on `listener` and run each as a task, for as long as the listener lives.
pub async fn serve<H>(
    listener: TcpListener,
    hub: Arc<H>,
    clock: Clock,
    max_message: usize,
) -> std::io::Result<()>
where
    H: Hub,
{
    accept_each(listener, hub, clock, max_message, |stream| {
        std::future::ready(Ok::<_, std::io::Error>(stream))
    })
    .await
}

/// [`serve`] over TLS: each connection's TLS handshake is completed with `tls` (made by
/// [`crate::tls::server_config`] from the deployer's certificate and key) before its upgrade, and a
/// connection whose handshake fails is closed and not run.
pub async fn serve_tls<H>(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    hub: Arc<H>,
    clock: Clock,
    max_message: usize,
) -> std::io::Result<()>
where
    H: Hub,
{
    let acceptor = TlsAcceptor::from(tls);
    accept_each(listener, hub, clock, max_message, move |stream| {
        acceptor.accept(stream)
    })
    .await
}

/// The loop of [`serve`] and [`serve_tls`]: accept, `open` the stream as the binding's (a TLS
/// handshake, or nothing), and run the connection as a task.
async fn accept_each<H, S, O, Opening>(
    listener: TcpListener,
    hub: Arc<H>,
    clock: Clock,
    max_message: usize,
    open: O,
) -> std::io::Result<()>
where
    H: Hub,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    O: Fn(TcpStream) -> Opening,
    Opening: Future<Output = std::io::Result<S>> + Send + 'static,
{
    loop {
        let (stream, peer): (TcpStream, SocketAddr) = listener.accept().await?;
        let opening = open(stream);
        let (hub, clock) = (hub.clone(), clock.clone());
        tokio::spawn(async move {
            let Ok(stream) = opening.await else {
                return;
            };
            let _ = run_connection(stream, Some(peer), hub, clock, max_message).await;
        });
    }
}

/// What a [`Server`] holds each connection to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The longest message a connection takes, in bytes: the longest request the relay takes
    /// (`Limits::max_request_bytes`). A longer one closes the connection.
    pub max_message: usize,
    /// The most connections open at once, at most [`Bounds::MOST_CONNECTIONS`]. One accepted past
    /// it is closed at once, before its TLS handshake or its upgrade costs anything.
    pub max_connections: usize,
    /// How long a connection has to finish its TLS handshake and its upgrade.
    pub handshake: Duration,
    /// How long an open connection may wait for its next request, and for its answer to be taken,
    /// before it is closed: a device connects again when it next has something to ask.
    pub idle: Duration,
    /// The most requests a connection that presents no token the relay takes may make: a login is
    /// a challenge and the login, and a device dials again with its token.
    pub anonymous_requests: u32,
    /// How long such a connection may wait for its next request, when that is shorter than `idle`.
    pub anonymous_idle: Duration,
}

impl Bounds {
    /// The most connections a server can be bounded to.
    pub const MOST_CONNECTIONS: usize = 1 << 24;
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            max_message: Limits::default().max_request_bytes,
            max_connections: 512,
            handshake: Duration::from_secs(10),
            idle: Duration::from_secs(300),
            anonymous_requests: 8,
            anonymous_idle: Duration::from_secs(30),
        }
    }
}

/// What tells a [`Server`] to stop: its listeners stop accepting, a connection still in its
/// handshake is closed, and an open one is closed once it has answered the request it is answering.
/// Any thread may raise it; raising it again does nothing.
#[derive(Clone)]
pub struct Stop(Arc<watch::Sender<bool>>);

impl Stop {
    pub fn new() -> Self {
        Self(Arc::new(watch::channel(false).0))
    }

    /// Stop the server, from any thread.
    pub fn raise(&self) {
        self.0.send_replace(true);
    }

    /// Whether it was raised.
    pub fn is_raised(&self) -> bool {
        *self.0.borrow()
    }

    /// Wait until it is raised.
    pub async fn wait(&self) {
        let mut watching = self.0.subscribe();
        let raised = watching.wait_for(|raised| *raised).await.is_ok();
        if !raised {
            // The sender is this stop's own and lives as long as it does: it cannot close first.
            std::future::pending::<()>().await;
        }
    }
}

impl Default for Stop {
    fn default() -> Self {
        Self::new()
    }
}

/// What a [`Server`] counted since it began: how many, never who, from where or what.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Connections taken and run.
    pub accepted: u64,
    /// Connections closed at once because as many as the bound were open.
    pub refused: u64,
    /// Connections that did not complete their TLS handshake or their upgrade, in time or at all
    /// (a token the relay did not take among them).
    pub unopened: u64,
    /// Requests answered.
    pub requests: u64,
    /// Times the listener failed to accept, which the server waits out and goes on from.
    pub accept_failures: u64,
    /// Connections open now, in their handshake or exchanging.
    pub open: u64,
}

#[derive(Default)]
struct Tally {
    accepted: AtomicU64,
    refused: AtomicU64,
    unopened: AtomicU64,
    requests: AtomicU64,
    accept_failures: AtomicU64,
}

/// The binding's server as a relay is deployed with: bounded by [`Bounds`] and stopped by a
/// [`Stop`]. One server may listen on several sockets (`ws://` on one, `wss://` on another), and
/// the bound on connections is over all of them.
pub struct Server<H: Hub> {
    shared: Arc<Shared<H>>,
}

struct Shared<H> {
    hub: Arc<H>,
    clock: Clock,
    bounds: Bounds,
    stop: Stop,
    permits: Arc<Semaphore>,
    capacity: u32,
    tally: Tally,
    /// The proxies whose `X-Forwarded-For` names a connection's source.
    proxies: Vec<IpAddr>,
}

impl<H: Hub> Clone for Server<H> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

/// How long a listener waits after it failed to accept (the process out of descriptors, say)
/// before it accepts again, so that a failure that lasts does not spin.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

impl<H: Hub> Server<H> {
    /// A server that answers with `hub` at the time `clock` gives (milliseconds since the Unix
    /// epoch, the clock tokens expire by).
    pub fn new(hub: Arc<H>, clock: Clock, bounds: Bounds) -> Self {
        Self::trusting(hub, clock, bounds, Vec::new())
    }

    /// [`Server::new`] behind the proxies at the addresses `proxies`: a connection from one of
    /// them comes from the address its `X-Forwarded-For` names (`login::source_behind`), and one
    /// from any other address comes from that address whatever it says. What the source is for is
    /// the allowance of the connections that present no token.
    pub fn trusting(hub: Arc<H>, clock: Clock, bounds: Bounds, proxies: Vec<IpAddr>) -> Self {
        let capacity = bounds.max_connections.clamp(1, Bounds::MOST_CONNECTIONS);
        Self {
            shared: Arc::new(Shared {
                hub,
                clock,
                bounds,
                stop: Stop::new(),
                permits: Arc::new(Semaphore::new(capacity)),
                capacity: u32::try_from(capacity).unwrap_or(u32::MAX),
                tally: Tally::default(),
                proxies,
            }),
        }
    }

    /// What stops this server.
    pub fn stop(&self) -> &Stop {
        &self.shared.stop
    }

    /// What it counted so far.
    pub fn counts(&self) -> Counts {
        let tally = &self.shared.tally;
        let free = self.shared.permits.available_permits() as u64;
        Counts {
            accepted: tally.accepted.load(Ordering::Relaxed),
            refused: tally.refused.load(Ordering::Relaxed),
            unopened: tally.unopened.load(Ordering::Relaxed),
            requests: tally.requests.load(Ordering::Relaxed),
            accept_failures: tally.accept_failures.load(Ordering::Relaxed),
            open: u64::from(self.shared.capacity).saturating_sub(free),
        }
    }

    /// Accept connections on `listener` until the stop is raised, completing each one's TLS
    /// handshake with `tls` when it is given (made by [`crate::tls::server_config`]), and run each
    /// as a task. The listener is closed when this returns; the connections it took go on until
    /// the stop closes them.
    pub async fn listen(&self, listener: TcpListener, tls: Option<Arc<ServerConfig>>) {
        let acceptor = tls.map(TlsAcceptor::from);
        let shared = &self.shared;
        loop {
            let accepted = tokio::select! {
                biased;
                () = shared.stop.wait() => return,
                accepted = listener.accept() => accepted,
            };
            let (stream, peer) = match accepted {
                Ok(accepted) => accepted,
                Err(_) => {
                    shared.tally.accept_failures.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(ACCEPT_PAUSE).await;
                    continue;
                }
            };
            let Ok(permit) = shared.permits.clone().try_acquire_owned() else {
                // As many as the bound are open: this one is closed as it is dropped.
                shared.tally.refused.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            shared.tally.accepted.fetch_add(1, Ordering::Relaxed);
            // A request and its answer are small and wait on each other: no delay for coalescing.
            let _ = stream.set_nodelay(true);
            let server = self.clone();
            match &acceptor {
                None => {
                    let opening = std::future::ready(Ok::<_, std::io::Error>(stream));
                    tokio::spawn(server.connection(opening, peer, permit));
                }
                Some(acceptor) => {
                    tokio::spawn(server.connection(acceptor.accept(stream), peer, permit));
                }
            }
        }
    }

    /// Wait, after the stop was raised, until every connection has closed or `grace` has passed:
    /// the number still open when it returns, `0` when every one closed.
    pub async fn drain(&self, grace: Duration) -> u64 {
        let all = self.shared.permits.acquire_many(self.shared.capacity);
        match tokio::time::timeout(grace, all).await {
            Ok(Ok(_every_permit)) => 0,
            _ => self.counts().open,
        }
    }

    /// One connection: its TLS handshake and its upgrade within the bound, then its exchange. The
    /// permit is the connection's place under the bound, given back when it closes.
    async fn connection<S, Opening>(
        self,
        opening: Opening,
        peer: SocketAddr,
        _permit: OwnedSemaphorePermit,
    ) where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        Opening: Future<Output = std::io::Result<S>> + Send + 'static,
    {
        let shared = &self.shared;
        let handshake = async {
            let stream = opening.await.ok()?;
            upgrade(
                stream,
                shared.hub.as_ref(),
                &shared.clock,
                shared.bounds.max_message,
                Some(peer),
                &shared.proxies,
            )
            .await
            .ok()
        };
        let upgraded = tokio::select! {
            upgraded = tokio::time::timeout(shared.bounds.handshake, handshake) => {
                upgraded.ok().flatten()
            }
            () = shared.stop.wait() => None,
        };
        let Some((ws, caller)) = upgraded else {
            shared.tally.unopened.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let bounds = &shared.bounds;
        let watch = if caller.anonymous {
            Watch {
                idle: Some(bounds.idle.min(bounds.anonymous_idle)),
                requests: Some(bounds.anonymous_requests),
                stop: Some(&shared.stop),
                tally: Some(&shared.tally),
            }
        } else {
            Watch {
                idle: Some(bounds.idle),
                requests: None,
                stop: Some(&shared.stop),
                tally: Some(&shared.tally),
            }
        };
        exchange(ws, &caller, &shared.hub, &shared.clock, watch).await;
    }
}
