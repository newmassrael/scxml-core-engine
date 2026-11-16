// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! HTTP send request data structure (§scxml-C-2 BasicHTTPEventProcessor).
//!
//! Ports the C++ `HttpSendRequest` struct from
//! `sce/include/static/StaticExecutionEngine.h:47`. Transport-agnostic — the
//! engine delegates actual HTTP dispatch to a user-supplied callback via
//! [`Engine::set_http_send_callback`](crate::Engine::set_http_send_callback). The test harness
//! injects a blocking HTTP client; production users inject their own dispatcher.
//!
//! This separation matches the C++ design: `StaticExecutionEngine` never links
//! against an HTTP library. `HttpEventTarget` (in `sce_runtime`) and
//! `W3CHttpTestServer` (in tests) live in separate translation units.

use std::collections::HashMap;

/// §scxml-C-2: HTTP send request payload passed to the `on_http_send` callback.
///
/// Matches C++ `SCE::Static::HttpSendRequest` 1:1.
#[derive(Debug, Clone, Default)]
pub struct HttpSendRequest {
    /// Absolute HTTP URL (e.g., `http://localhost:8080/test`).
    pub target: String,
    /// Event name to encode in the HTTP POST payload.
    pub event_name: String,
    /// Raw content body (§scxml-C-2: sent as multipart form data).
    pub content: String,
    /// Form parameters (§scxml-6.2 `<param>` elements). Multiple values per key allowed.
    pub params: HashMap<String, Vec<String>>,
    /// Send ID for correlation and cancellation (§scxml-6.2.5).
    pub send_id: String,
}

/// §scxml-C-2: HTTP send response returned by the `on_http_send` callback.
///
/// When the callback returns `Ok(Some(HttpSendResponse))`, the engine injects
/// the response event into the external queue. This enables real HTTP
/// round-trips against the shared W3C HTTP test server
/// (`standalone_http_server.js`).
#[derive(Debug, Clone, Default)]
pub struct HttpSendResponse {
    /// Event name extracted from the HTTP response (e.g., `event1`, `HTTP.POST`).
    pub event_name: String,
    /// Event data from the response (§scxml-5.10.1: `_event.data`).
    pub event_data: String,
}

/// §scxml-C-2: the `on_http_send` callback's report that the request was not
/// delivered: the server could not be reached, or it refused the request with
/// a status outside 2xx.
///
/// The engine answers it as §scxml-6.2.4 answers a send the processor is unable
/// to dispatch: `error.communication`, carrying the send id, on the internal
/// queue. [`reason`](Self::reason) is the error's `_event.data`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpSendFailure {
    /// Why the request was not delivered, in the transport's words.
    pub reason: String,
}

impl HttpSendFailure {
    /// A failure that says why in `reason`.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// What the `on_http_send` callback answers for one request: delivered, with
/// the event the receiver replied with or none (`Ok`), or not delivered
/// (`Err`).
pub type HttpSendOutcome = Result<Option<HttpSendResponse>, HttpSendFailure>;

/// The host's BasicHTTP transport as the engine holds it. `+ Send` so the
/// engine stays host-movable (see `Engine::completion_callback`).
pub(crate) type HttpSendTransport = Box<dyn FnMut(HttpSendRequest) -> HttpSendOutcome + Send>;
