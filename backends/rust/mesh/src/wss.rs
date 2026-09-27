// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The WebSocket binding (SCE_MESH.md §mesh-18): the socket plumbing between
//! a WebSocket and the router core, and nothing else.
//!
//! A link — one connection to one peer — runs as a task. It hands the host
//! [`LinkEvent`]s: the peer became ready, arrived bytes, was lost, and each
//! envelope still queued when it was. The host feeds them to its
//! [`Endpoint`] with [`deliver`], on the thread that steps the engine, so
//! every decision stays in the core. The core's sends reach a link through
//! [`WssTransport`], which only queues bytes for that link's writer.
//!
//! The server accepts on any stream the host hands it — a TCP stream, or one
//! its TLS acceptor already wrapped — and the client dials a `ws://` or
//! `wss://` URL, with rustls for the latter.

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::endpoint::{Endpoint, Environment, Transport, TransportFailure};

/// The path a client opens, before its own machine name (§mesh-18.1).
pub const PATH_PREFIX: &str = "/sce-mesh/1/";

/// The keepalive interval when the deployment sets none (§mesh-18.3).
pub const DEFAULT_KEEPALIVE: Duration = Duration::from_secs(30);

/// What a link tells its host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
    /// The upgrade completed: `peer`'s binding is ready.
    Ready(String),
    /// A binary message from `peer`: one envelope's bytes.
    Received(String, Vec<u8>),
    /// An envelope queued for `peer` that the link closed before writing.
    Unsent(String, Vec<u8>),
    /// The link to `peer` closed, failed, or stopped answering pings; the
    /// words say which.
    Lost(String, String),
}

/// Hand a link's event to the endpoint it concerns.
pub fn deliver<T: Transport, E: Environment>(endpoint: &mut Endpoint<T, E>, event: LinkEvent) {
    match event {
        LinkEvent::Ready(peer) => endpoint.peer_ready(&peer),
        LinkEvent::Received(peer, bytes) => endpoint.receive(&peer, bytes),
        LinkEvent::Unsent(peer, bytes) => endpoint.transmit_failed(
            &peer,
            bytes,
            TransportFailure {
                retryable: true,
                message: Some("the link closed before the envelope was written".to_string()),
            },
        ),
        LinkEvent::Lost(peer, _) => endpoint.peer_not_ready(&peer),
    }
}

/// Why a link could not be opened.
#[derive(Debug)]
pub enum WssError {
    /// A machine name is not a path segment as it stands (§mesh-18.1).
    NotAPathSegment(String),
    /// The WebSocket handshake failed.
    Handshake(tokio_tungstenite::tungstenite::Error),
}

/// Whether `name` is a path segment as it stands: RFC 3986's unreserved
/// characters, so no encoding can make two names one.
pub fn is_path_segment(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}

type LinkId = u64;

/// The core's side of every link: bytes queued for the peer they name.
#[derive(Clone, Default)]
pub struct WssTransport {
    links: Arc<Mutex<Links>>,
}

#[derive(Default)]
struct Links {
    next_id: LinkId,
    writers: HashMap<String, (LinkId, mpsc::UnboundedSender<Vec<u8>>)>,
}

impl WssTransport {
    pub fn new() -> Self {
        Self::default()
    }

    fn open(&self, peer: &str) -> (LinkId, mpsc::UnboundedReceiver<Vec<u8>>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut links = self
            .links
            .lock()
            .expect("the link table is never left mid-update");
        links.next_id += 1;
        let id = links.next_id;
        // A newer link to the same peer replaces the older one: the older
        // one's writer stops receiving, and it is about to close.
        links.writers.insert(peer.to_string(), (id, tx));
        (id, rx)
    }

    fn close(&self, peer: &str, id: LinkId) {
        let mut links = self
            .links
            .lock()
            .expect("the link table is never left mid-update");
        if links
            .writers
            .get(peer)
            .is_some_and(|(current, _)| *current == id)
        {
            links.writers.remove(peer);
        }
    }
}

impl Transport for WssTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        let links = self
            .links
            .lock()
            .expect("the link table is never left mid-update");
        let Some((_, writer)) = links.writers.get(peer) else {
            return Err(TransportFailure {
                retryable: true,
                message: Some(alloc::format!("no link to {peer}")),
            });
        };
        writer.send(bytes.to_vec()).map_err(|_| TransportFailure {
            retryable: true,
            message: Some(alloc::format!("the link to {peer} closed")),
        })
    }
}

/// Serve one connection the host accepted: read the client's name from the
/// upgrade path, answer 404 for a version this does not speak or a name
/// `is_bound` refuses (§mesh-18.1), and run the link until it ends.
pub async fn accept<S>(
    stream: S,
    is_bound: impl Fn(&str) -> bool,
    transport: WssTransport,
    events: mpsc::UnboundedSender<LinkEvent>,
    keepalive: Duration,
) -> Result<(), WssError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut peer = None;
    let ws = tokio_tungstenite::accept_hdr_async(stream, route_upgrade(&is_bound, &mut peer))
        .await
        .map_err(WssError::Handshake)?;
    let peer = peer.expect("an upgrade completes only after the callback named the peer");
    run(peer, ws, transport, events, keepalive).await;
    Ok(())
}

/// The upgrade's verdict (§mesh-18.1): the client named in the path, when it
/// is one `is_bound` accepts, is written to `peer`; anything else is 404.
///
/// The callback's error is `ErrorResponse` because tungstenite's `Callback`
/// contract says so; its size is the library's, not this crate's to box.
#[allow(clippy::result_large_err)]
fn route_upgrade<'a>(
    is_bound: &'a impl Fn(&str) -> bool,
    peer: &'a mut Option<String>,
) -> impl FnOnce(&Request, Response) -> Result<Response, ErrorResponse> + 'a {
    move |request: &Request, response: Response| match request
        .uri()
        .path()
        .strip_prefix(PATH_PREFIX)
    {
        Some(name) if is_path_segment(name) && is_bound(name) => {
            *peer = Some(name.to_string());
            Ok(response)
        }
        _ => Err(not_found()),
    }
}

/// Dial the server machine `server` at `base` (a `ws://` or `wss://` URL,
/// before the binding's path) as `own`, and run the link until it ends. A
/// client reconnects by calling this again, on its own backoff (§mesh-18.3).
pub async fn connect(
    base: &str,
    own: &str,
    server: &str,
    transport: WssTransport,
    events: mpsc::UnboundedSender<LinkEvent>,
    keepalive: Duration,
) -> Result<(), WssError> {
    if !is_path_segment(own) {
        return Err(WssError::NotAPathSegment(own.to_string()));
    }
    let url = alloc::format!("{}{PATH_PREFIX}{own}", base.trim_end_matches('/'));
    let (ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(WssError::Handshake)?;
    run(server.to_string(), ws, transport, events, keepalive).await;
    Ok(())
}

fn not_found() -> ErrorResponse {
    let mut response = http::Response::new(Some("no such Mesh binding".to_string()));
    *response.status_mut() = http::StatusCode::NOT_FOUND;
    response
}

/// One ready link, until it closes, fails, or stops answering pings.
async fn run<S>(
    peer: String,
    ws: WebSocketStream<S>,
    transport: WssTransport,
    events: mpsc::UnboundedSender<LinkEvent>,
    keepalive: Duration,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (id, mut outgoing) = transport.open(&peer);
    let (mut sink, mut stream) = ws.split();
    let _ = events.send(LinkEvent::Ready(peer.clone()));

    let mut ticker = tokio::time::interval(keepalive);
    ticker.reset();
    let mut awaiting_pong: Option<Instant> = None;
    let reason = loop {
        tokio::select! {
            bytes = outgoing.recv() => {
                let Some(bytes) = bytes else {
                    break "the link was replaced by a newer one".to_string();
                };
                if let Err(error) = sink.send(Message::Binary(bytes.clone().into())).await {
                    let _ = events.send(LinkEvent::Unsent(peer.clone(), bytes));
                    break alloc::format!("write failed: {error}");
                }
            }
            message = stream.next() => match message {
                Some(Ok(Message::Binary(bytes))) => {
                    let _ = events.send(LinkEvent::Received(peer.clone(), bytes.to_vec()));
                }
                // §mesh-18.2: text is not an envelope.
                Some(Ok(Message::Text(_))) => {
                    let _ = sink
                        .send(Message::Close(Some(CloseFrame {
                            code: CloseCode::Unsupported,
                            reason: "an envelope is a binary message".into(),
                        })))
                        .await;
                    break "the peer sent a text message".to_string();
                }
                Some(Ok(Message::Pong(_))) => awaiting_pong = None,
                // A ping is answered by the WebSocket layer itself.
                Some(Ok(Message::Ping(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Close(_))) | None => break "the peer closed the link".to_string(),
                Some(Err(error)) => break alloc::format!("read failed: {error}"),
            },
            _ = ticker.tick() => {
                if awaiting_pong.is_some_and(|since| since.elapsed() >= keepalive) {
                    break "no pong within the keepalive interval".to_string();
                }
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break "a ping could not be written".to_string();
                }
                awaiting_pong.get_or_insert_with(Instant::now);
            }
        }
    };

    transport.close(&peer, id);
    outgoing.close();
    while let Ok(bytes) = outgoing.try_recv() {
        let _ = events.send(LinkEvent::Unsent(peer.clone(), bytes));
    }
    let _ = events.send(LinkEvent::Lost(peer, reason));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::Delivery;
    use crate::router::{PeerConfig, Router};
    use sce_rust_runtime::HostSendRequest;
    use tokio::net::TcpListener;

    /// A clock that counts, and ids that count.
    #[derive(Default)]
    struct Counting {
        now: i64,
        ids: u8,
    }

    impl Environment for Counting {
        fn now_ms(&mut self) -> i64 {
            self.now += 1;
            self.now
        }
        fn envelope_id(&mut self) -> [u8; 16] {
            self.ids += 1;
            [self.ids; 16]
        }
        fn jitter_draw(&mut self) -> i64 {
            0
        }
    }

    fn endpoint(
        machine: &str,
        peer: &str,
        transport: WssTransport,
    ) -> Endpoint<WssTransport, Counting> {
        let mut router = Router::new(machine, 8, 50).unwrap();
        router.add_peer(
            peer,
            PeerConfig {
                transport: "wss",
                max_pending: 8,
                max_age_ms: 0,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
            },
        );
        Endpoint::new(router, transport, Counting::default())
    }

    fn send(event: &str) -> HostSendRequest {
        HostSendRequest {
            processor_type: crate::endpoint::MESH_PROCESSOR_TYPE.to_string(),
            event_name: event.to_string(),
            target: "#server".to_string(),
            send_id: "s-1".to_string(),
            event_data: r#"{"v":1}"#.to_string(),
            ..HostSendRequest::default()
        }
    }

    async fn next(events: &mut mpsc::UnboundedReceiver<LinkEvent>) -> LinkEvent {
        tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("a link event within five seconds")
            .expect("the link task holds the channel open")
    }

    #[tokio::test]
    async fn a_send_crosses_a_real_socket_to_the_peers_engine() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = alloc::format!("ws://{}", listener.local_addr().unwrap());

        let server_transport = WssTransport::new();
        let (server_tx, mut server_events) = mpsc::unbounded_channel();
        let accepting = server_transport.clone();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = accept(
                stream,
                |name| name == "client",
                accepting,
                server_tx,
                DEFAULT_KEEPALIVE,
            )
            .await;
        });

        let client_transport = WssTransport::new();
        let (client_tx, mut client_events) = mpsc::unbounded_channel();
        let dialing = client_transport.clone();
        let dial_base = base.clone();
        tokio::spawn(async move {
            let _ = connect(
                &dial_base,
                "client",
                "server",
                dialing,
                client_tx,
                DEFAULT_KEEPALIVE,
            )
            .await;
        });

        let mut client = endpoint("client", "server", client_transport);
        let mut server = endpoint("server", "client", server_transport);
        assert_eq!(
            next(&mut client_events).await,
            LinkEvent::Ready("server".to_string())
        );
        assert_eq!(
            next(&mut server_events).await,
            LinkEvent::Ready("client".to_string())
        );
        deliver(&mut client, LinkEvent::Ready("server".to_string()));
        deliver(&mut server, LinkEvent::Ready("client".to_string()));

        client.send(&send("go"));
        let received = next(&mut server_events).await;
        assert!(matches!(&received, LinkEvent::Received(peer, _) if peer == "client"));
        deliver(&mut server, received);

        let events = server.take_events();
        let [event] = events.as_slice() else {
            panic!("expected one event, got {events:?}");
        };
        assert_eq!(event.name, "go");
        assert_eq!(event.metadata.data, r#"{"v":1}"#);
        assert_eq!(event.metadata.origin, "mesh://client");
        assert_eq!(event.metadata.send_id, "s-1");
        assert!(client.take_host_errors().is_empty());
    }

    #[tokio::test]
    async fn a_name_the_server_has_no_binding_for_is_refused_at_the_upgrade() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = alloc::format!("ws://{}", listener.local_addr().unwrap());
        let (server_tx, _server_events) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = accept(
                stream,
                |name| name == "client",
                WssTransport::new(),
                server_tx,
                DEFAULT_KEEPALIVE,
            )
            .await;
        });
        let (client_tx, _client_events) = mpsc::unbounded_channel();
        let refused = connect(
            &base,
            "stranger",
            "server",
            WssTransport::new(),
            client_tx,
            DEFAULT_KEEPALIVE,
        )
        .await;
        let Err(WssError::Handshake(tokio_tungstenite::tungstenite::Error::Http(response))) =
            refused
        else {
            panic!("expected an HTTP refusal, got {refused:?}");
        };
        assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_closed_link_ends_readiness_and_the_core_raises_row_one() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = alloc::format!("ws://{}", listener.local_addr().unwrap());
        let (server_tx, mut server_events) = mpsc::unbounded_channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = accept(
                stream,
                |_| true,
                WssTransport::new(),
                server_tx,
                DEFAULT_KEEPALIVE,
            )
            .await;
        });
        let client_transport = WssTransport::new();
        let (client_tx, mut client_events) = mpsc::unbounded_channel();
        let dialing = client_transport.clone();
        tokio::spawn(async move {
            let _ = connect(
                &base,
                "client",
                "server",
                dialing,
                client_tx,
                DEFAULT_KEEPALIVE,
            )
            .await;
        });
        let mut client = endpoint("client", "server", client_transport);
        deliver(&mut client, next(&mut client_events).await);
        assert_eq!(
            next(&mut server_events).await,
            LinkEvent::Ready("client".to_string())
        );

        // The server side goes away; its socket closes with it.
        server.abort();
        let lost = next(&mut client_events).await;
        assert!(
            matches!(&lost, LinkEvent::Lost(peer, _) if peer == "server"),
            "{lost:?}"
        );
        deliver(&mut client, lost);

        let events = client.take_events();
        let [event] = events.as_slice() else {
            panic!("expected one row, got {events:?}");
        };
        assert_eq!(event.name, "error.communication");
        assert_eq!(
            event.metadata.data,
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"server","transport":"wss"}"#
        );
    }

    #[test]
    fn a_machine_name_is_a_path_segment_as_it_stands() {
        for good in ["client", "hmi_2", "a.b", "x-y", "t~"] {
            assert!(is_path_segment(good), "{good}");
        }
        for bad in ["", "a/b", "a b", "é", "a%2F", "a?b"] {
            assert!(!is_path_segment(bad), "{bad}");
        }
    }
}
