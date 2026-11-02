// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The WebSocket binding of the relay: a request text goes in as a text message and the response
//! text comes back as one, in order, one connection for one device.
//!
//! A device connects to [`PATH`] with the token its login earned as `Authorization: Bearer
//! <token>`. The upgrade is refused (401) when the relay did not issue the token or it has expired,
//! and every request on the connection is judged again by the relay, so a token that expires on a
//! connection that stays open stops being answered and the device reconnects with a new one. A
//! binary message, or a text one longer than the relay takes, closes the connection. TLS is the
//! deployer's: [`serve_connection`] runs on any stream the host hands it, one its TLS acceptor
//! already wrapped included, and the client ([`WssLink`], `crate::wss`) dials `ws://` or `wss://`.
//!
//! The relay's work is short and takes a lock, so it runs on a blocking thread and never holds up
//! the connections that are waiting. What answers is a [`Hub`]: a relay behind one lock, or
//! [`crate::hub::Shards`], whose requests for different lists run at once.

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

use crate::hub::Hub;
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
    let mut ws = tokio_tungstenite::accept_hdr_async_with_config(stream, verdict, Some(config))
        .await
        .map_err(|error| ServeError::Handshake(error.to_string()))?;
    let token = token.expect("an upgrade completes only after the callback took the token");

    while let Some(Ok(message)) = ws.next().await {
        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => break,
        };
        let answer = {
            let (hub, clock, token) = (hub.clone(), clock.clone(), token.clone());
            tokio::task::spawn_blocking(move || hub.handle(&token, &text, clock())).await
        };
        let Ok(answer) = answer else { break };
        if ws.send(Message::Text(answer.into())).await.is_err() {
            break;
        }
    }
    let _ = ws.close(None).await;
    Ok(())
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
    loop {
        let (stream, _peer): (TcpStream, SocketAddr) = listener.accept().await?;
        let (hub, clock) = (hub.clone(), clock.clone());
        tokio::spawn(async move {
            let _ = serve_connection(stream, hub, clock, max_message).await;
        });
    }
}
