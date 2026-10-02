// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The sockets.
//!
//! Every wait on a client has an end here, because the first server this shell
//! used had none: a request that announced a large body and never sent it kept
//! its thread reading after the answer had gone out, and a handful of those
//! stopped the server for everyone, before any token was looked at. Now:
//!
//! - the request line and headers must arrive within [`Limits::header_read`];
//! - the body must arrive within [`Limits::body_read`], and a body announced
//!   larger than [`MAX_BODY_BYTES`] is refused on its announcement, unread;
//! - a call without the token is refused before its body is read at all;
//! - only [`Limits::connections`] connections are served at once; the next is
//!   closed unanswered, which is what a server out of room should say;
//! - a connection serves one request and closes, so nothing waits for a second.
//!
//! The handler itself ([`Shell::handle`]) is synchronous and may wait on the
//! works folder's file lock, so it runs on the blocking pool and never on the
//! threads that read sockets.

use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Body, Incoming};
use hyper::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::{Reply, Shell, MAX_BODY_BYTES};

/// How long a client may take, and how many may be served at once.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub header_read: Duration,
    pub body_read: Duration,
    pub connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            header_read: Duration::from_secs(10),
            body_read: Duration::from_secs(30),
            connections: 64,
        }
    }
}

/// Serve `listener` until `stop` completes.
pub async fn serve(
    shell: Arc<Shell>,
    listener: TcpListener,
    limits: Limits,
    stop: impl Future<Output = ()>,
) {
    let room = Arc::new(Semaphore::new(limits.connections));
    tokio::pin!(stop);
    loop {
        let stream = tokio::select! {
            () = &mut stop => return,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(_) => {
                    // Out of descriptors or a connection reset before it was
                    // accepted: neither ends the server, and neither should spin it.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            },
        };
        let Ok(place) = Arc::clone(&room).try_acquire_owned() else {
            continue; // dropped here: the connection is closed unanswered
        };
        let shell = Arc::clone(&shell);
        tokio::spawn(async move {
            let service = service_fn(move |request| answer(Arc::clone(&shell), limits, request));
            let connection = http1::Builder::new()
                .timer(TokioTimer::new())
                .header_read_timeout(limits.header_read)
                .keep_alive(false)
                .serve_connection(TokioIo::new(stream), service);
            // A client that hung up or broke the protocol has nobody to tell.
            let _ = connection.await;
            drop(place);
        });
    }
}

async fn answer(
    shell: Arc<Shell>,
    limits: Limits,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, body) = request.into_parts();
    let method = parts.method.as_str().to_string();
    let url = parts
        .uri
        .path_and_query()
        .map_or("/", |p| p.as_str())
        .to_string();
    let authorization = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let reply = if shell.refuses_before_reading(&method, &url, authorization.as_deref()) {
        // The same handler gives the same refusal; it is only asked before the
        // body, so a caller without the token cannot make the server read one.
        run(shell, method, url, authorization, Vec::new()).await
    } else {
        match read(body, limits).await {
            Ok(bytes) => run(shell, method, url, authorization, bytes).await,
            Err(reply) => reply,
        }
    };
    Ok(into_response(reply))
}

async fn read(body: Incoming, limits: Limits) -> Result<Vec<u8>, Reply> {
    // `Limited` stops a body that exceeds the limit as it arrives; an announced
    // length is known at once, and is refused without waiting for a byte.
    if body.size_hint().lower() > MAX_BODY_BYTES as u64 {
        return Err(Reply::error(
            413,
            "too-large",
            "the request is larger than any source",
        ));
    }
    let collecting = Limited::new(body, MAX_BODY_BYTES).collect();
    match tokio::time::timeout(limits.body_read, collecting).await {
        Ok(Ok(collected)) => Ok(collected.to_bytes().to_vec()),
        Ok(Err(error)) if error.is::<http_body_util::LengthLimitError>() => Err(Reply::error(
            413,
            "too-large",
            "the request is larger than any source",
        )),
        Ok(Err(error)) => Err(Reply::error(
            400,
            "bad-request",
            format!("the body could not be read: {error}"),
        )),
        Err(_) => Err(Reply::error(
            408,
            "bad-request",
            "the body did not arrive in time",
        )),
    }
}

async fn run(
    shell: Arc<Shell>,
    method: String,
    url: String,
    authorization: Option<String>,
    body: Vec<u8>,
) -> Reply {
    tokio::task::spawn_blocking(move || {
        shell.handle(&method, &url, authorization.as_deref(), &body)
    })
    .await
    .unwrap_or_else(|_| Reply::error(500, "io", "the handler stopped unexpectedly"))
}

fn into_response(reply: Reply) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from(reply.body)));
    *response.status_mut() =
        StatusCode::from_u16(reply.status).expect("the handler answers with statuses it chose");
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(reply.content_type));
    for (name, value) in [
        // Nothing here is worth keeping: the works change under the screen.
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        (
            "content-security-policy",
            "default-src 'self'; connect-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:",
        ),
    ] {
        headers.insert(HeaderName::from_static(name), HeaderValue::from_static(value));
    }
    response
}
