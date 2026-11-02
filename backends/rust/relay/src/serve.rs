// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The WebSocket binding of the relay: a request text goes in as a text message and the response
//! text comes back as one, in order, one connection for one device.
//!
//! A device connects to [`PATH`] with the token its login earned as `Authorization: Bearer
//! <token>`. The upgrade is refused (401) when the relay did not issue the token or it has expired,
//! and every request on the connection is judged again by the relay, so a token that expires on a
//! connection that stays open stops being answered and the device reconnects with a new one. A
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
use std::net::SocketAddr;
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
use crate::tls::ServerConfig;
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
    let (ws, token) = upgrade(stream, hub.as_ref(), &clock, max_message).await?;
    exchange(ws, &token, &hub, &clock, Watch::default()).await;
    Ok(())
}

/// Complete the upgrade of `stream` when it asks for the relay's path with a token the hub takes:
/// the connection, and the token every request on it is made with.
async fn upgrade<S, H>(
    stream: S,
    hub: &H,
    clock: &Clock,
    max_message: usize,
) -> Result<(WebSocketStream<S>, String), ServeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
    let mut token: Option<String> = None;
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
        let presented = request
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        let Some(presented) = presented else {
            return Err(refusal(http::StatusCode::UNAUTHORIZED, "no token"));
        };
        let accepted = hub.authenticate(presented, clock()).is_ok();
        if !accepted {
            return Err(refusal(
                http::StatusCode::UNAUTHORIZED,
                "not a token of this relay",
            ));
        }
        token = Some(presented.to_string());
        Ok(response)
    };
    let ws = tokio_tungstenite::accept_hdr_async_with_config(stream, verdict, Some(config))
        .await
        .map_err(|error| ServeError::Handshake(error.to_string()))?;
    let token = token.expect("an upgrade completes only after the callback took the token");
    Ok((ws, token))
}

/// What an open connection is held to beyond its messages: how long it may wait for a request (and
/// for its answer to be taken), the stop that closes it, and where its requests are counted.
#[derive(Default)]
struct Watch<'a> {
    idle: Option<Duration>,
    stop: Option<&'a Stop>,
    tally: Option<&'a Tally>,
}

/// Answer each text message of `ws` with the hub's response, until the connection closes, a
/// message that is not a request comes, or `watch` closes it. A request that was read is always
/// answered: the stop and the idle bound are looked at only while the connection waits for one.
async fn exchange<S, H>(
    mut ws: WebSocketStream<S>,
    token: &str,
    hub: &Arc<H>,
    clock: &Clock,
    watch: Watch<'_>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Hub,
{
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
            let (hub, clock, token) = (hub.clone(), clock.clone(), token.to_string());
            tokio::task::spawn_blocking(move || hub.handle(&token, &text, clock())).await
        };
        let Ok(answer) = answer else { break };
        if let Some(tally) = watch.tally {
            tally.requests.fetch_add(1, Ordering::Relaxed);
        }
        let sent = within(watch.idle, ws.send(Message::Text(answer.into()))).await;
        if !matches!(sent, Some(Ok(()))) {
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
        let (stream, _peer): (TcpStream, SocketAddr) = listener.accept().await?;
        let opening = open(stream);
        let (hub, clock) = (hub.clone(), clock.clone());
        tokio::spawn(async move {
            let Ok(stream) = opening.await else {
                return;
            };
            let _ = serve_connection(stream, hub, clock, max_message).await;
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
            let stream = match accepted {
                Ok((stream, _peer)) => stream,
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
                    tokio::spawn(server.connection(opening, permit));
                }
                Some(acceptor) => {
                    tokio::spawn(server.connection(acceptor.accept(stream), permit));
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
    async fn connection<S, Opening>(self, opening: Opening, _permit: OwnedSemaphorePermit)
    where
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
        let Some((ws, token)) = upgraded else {
            shared.tally.unopened.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let watch = Watch {
            idle: Some(shared.bounds.idle),
            stop: Some(&shared.stop),
            tally: Some(&shared.tally),
        };
        exchange(ws, &token, &shared.hub, &shared.clock, watch).await;
    }
}
