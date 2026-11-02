// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The client side of the relay's WebSocket binding: the reference [`Transport`] (ADR 0079 section
//! 5), and the one a deployment with no other uses.
//!
//! A device connects to [`PATH`] with the token its login earned as `Authorization: Bearer
//! <token>`, sends each request text as a text message and reads the response text as the next one,
//! one connection for one device (the server is `crate::serve`). The link owns a small runtime of
//! its own and blocks on each exchange, so it is called from a thread of the host's (a background
//! worker), never from inside another runtime.
//!
//! An exchange connects when the link has no connection, and gives the connection up whenever the
//! exchange does not come to its response: the connection failed or closed, the deadline passed,
//! the exchange was cancelled, or the network changed while it waited. A response that comes late on
//! a connection given up is never read as the answer to a later request; the next exchange dials
//! again with the same token. When the network's signal was raised since the connection was made,
//! the next exchange dials again before it asks. A refusal of the token at the upgrade (401) is
//! [`TransportError::Unauthorized`].
//!
//! A `wss://` link runs TLS as `crate::tls` configures it: `ring`'s cryptography, named to rustls
//! rather than taken from the process, and the public certificate authorities ([`WssLink::new`]) or
//! the ones a deployment names ([`WssLink::believing`]). A certificate they did not issue fails the
//! exchange as the link's failure.

use std::io;
use std::sync::Arc;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use sce_rust_relay_transport::{Notice, Notices, Signal, Transport, TransportError};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{http, Error as WsError, Message};
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

use crate::tls::{self, ClientConfig, Roots};

/// The path a device connects to.
pub const PATH: &str = "/sce-relay/1";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A link to a relay over a WebSocket, with the token that names the device of one list.
pub struct WssLink {
    runtime: tokio::runtime::Runtime,
    url: String,
    token: String,
    /// What a `wss://` dial runs TLS on; a `ws://` one passes it unused.
    tls: Arc<ClientConfig>,
    socket: Option<Socket>,
    /// How many times the network's signal was raised when the connection was made.
    dialled_at: u64,
    network: Signal,
    notices: Notices,
}

/// Why one exchange on the socket did not come to its response.
enum Broke {
    Unauthorized(String),
    Link(String),
}

/// How the wait for one exchange ended.
enum Waited {
    Answered(Result<String, Broke>),
    /// The cancellation or the network's signal was raised.
    Woken,
    Expired,
}

impl WssLink {
    /// A link to the relay at `base` (`ws://host:port` or `wss://host:port`) with `token`, which
    /// believes the public certificate authorities. Nothing is dialled until the first exchange; an
    /// address that is not a WebSocket's is refused here.
    pub fn new(base: &str, token: &str) -> io::Result<Self> {
        Self::believing(base, token, &Roots::Public)
    }

    /// [`WssLink::new`] believing `roots` for a `wss://` relay's certificate: a relay under a
    /// private authority is reached with [`Roots::Only`] and that authority's certificate, and a
    /// deployment whose relays are under either a private authority or a public one with
    /// [`Roots::PublicAnd`]. An authority that is not a certificate is refused here.
    pub fn believing(base: &str, token: &str, roots: &Roots) -> io::Result<Self> {
        let base = base.trim_end_matches('/');
        if !(base.starts_with("ws://") || base.starts_with("wss://")) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("`{base}` is not a ws:// or wss:// address"),
            ));
        }
        let tls = tls::client_config(roots).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("the authorities to believe are not usable: {error}"),
            )
        })?;
        Ok(Self {
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
            url: format!("{base}{PATH}"),
            token: token.to_string(),
            tls,
            socket: None,
            dialled_at: 0,
            network: Signal::new(),
            notices: Notices::default(),
        })
    }

    /// Give the connection up, when there is one, and tell the host why.
    fn give_up(&mut self, why: &str) {
        if self.socket.take().is_some() {
            self.notices.push(Notice::Lost(why.to_string()));
        }
    }
}

/// Dial the relay at `url`, over TLS as `tls` says when it is a `wss://` one, and present `token`.
async fn dial(url: &str, token: &str, tls: &Arc<ClientConfig>) -> Result<Socket, Broke> {
    let mut upgrade = url
        .into_client_request()
        .map_err(|error| Broke::Link(error.to_string()))?;
    let value = http::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| Broke::Link("the token is not a header value".to_string()))?;
    upgrade
        .headers_mut()
        .insert(http::header::AUTHORIZATION, value);
    let connector = Connector::Rustls(Arc::clone(tls));
    match tokio_tungstenite::connect_async_tls_with_config(upgrade, None, false, Some(connector))
        .await
    {
        Ok((socket, _)) => Ok(socket),
        Err(WsError::Http(response)) if response.status() == http::StatusCode::UNAUTHORIZED => {
            let why = response
                .body()
                .as_deref()
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            Err(Broke::Unauthorized(why.into_owned()))
        }
        Err(error) => Err(Broke::Link(error.to_string())),
    }
}

/// Carry `request` on the connection, dialled first when there is none.
async fn carry(
    socket: &mut Option<Socket>,
    url: &str,
    token: &str,
    tls: &Arc<ClientConfig>,
    request: &str,
) -> Result<String, Broke> {
    if socket.is_none() {
        *socket = Some(dial(url, token, tls).await?);
    }
    match socket.as_mut() {
        Some(open) => ask(open, request).await,
        None => Err(Broke::Link("the link is not open".to_string())),
    }
}

/// Send `request` on `socket` and read the text that answers it.
async fn ask(socket: &mut Socket, request: &str) -> Result<String, Broke> {
    socket
        .send(Message::Text(request.into()))
        .await
        .map_err(|error| Broke::Link(error.to_string()))?;
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => return Ok(text.to_string()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(_)) | Some(Err(_)) | None => {
                return Err(Broke::Link("the relay closed the link".to_string()))
            }
        }
    }
}

impl Transport for WssLink {
    fn exchange(
        &mut self,
        request: &str,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError> {
        if self.socket.is_some() && self.network.raised() != self.dialled_at {
            self.give_up("the network changed");
        }
        // One wake for the two signals, made for this exchange alone, so that a raise of an earlier
        // exchange's is never taken for one of this.
        let woken = Arc::new(Notify::new());
        let _cancelled = {
            let woken = Arc::clone(&woken);
            cancel.watch(move || woken.notify_one())
        };
        let _moved = {
            let woken = Arc::clone(&woken);
            self.network.watch(move || woken.notify_one())
        };
        // Read after the watches began, so that a raise before them is not missed.
        if cancel.is_raised() {
            return Err(TransportError::Cancelled);
        }
        let network_at = self.network.raised();

        let dialling = self.socket.is_none();
        let Self {
            runtime,
            url,
            token,
            tls,
            socket,
            ..
        } = &mut *self;
        let work = carry(socket, url.as_str(), token.as_str(), tls, request);
        let expiry = async move {
            match deadline {
                Some(deadline) => {
                    tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await
                }
                None => std::future::pending::<()>().await,
            }
        };
        let waited = runtime.block_on(async {
            // An answer that came is taken before a wake or the deadline that came with it.
            tokio::select! {
                biased;
                answered = work => Waited::Answered(answered),
                () = woken.notified() => Waited::Woken,
                () = expiry => Waited::Expired,
            }
        });
        if dialling && self.socket.is_some() {
            self.dialled_at = network_at;
            self.notices.push(Notice::Connected);
        }

        let error = match waited {
            Waited::Answered(Ok(text)) => return Ok(text),
            Waited::Answered(Err(Broke::Unauthorized(why))) => TransportError::Unauthorized(why),
            Waited::Answered(Err(Broke::Link(why))) => TransportError::Link(why),
            Waited::Woken if cancel.is_raised() => TransportError::Cancelled,
            Waited::Woken => TransportError::Link("the network changed".to_string()),
            Waited::Expired => {
                // The runtime's timer counts in its own ticks: the deadline is passed by the clock
                // the caller reads before the exchange says it is.
                if let Some(left) =
                    deadline.and_then(|deadline| deadline.checked_duration_since(Instant::now()))
                {
                    std::thread::sleep(left);
                }
                TransportError::Deadline
            }
        };
        self.give_up(&error.to_string());
        Err(error)
    }

    fn take_notices(&mut self) -> Vec<Notice> {
        self.notices.take()
    }

    fn network(&self) -> Signal {
        self.network.clone()
    }
}
