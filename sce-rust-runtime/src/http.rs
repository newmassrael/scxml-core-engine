// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! HTTP send request data structure (W3C SCXML C.2 BasicHTTPEventProcessor).
//!
//! Ports the C++ `HttpSendRequest` struct from
//! `sce/include/static/StaticExecutionEngine.h:47`. Transport-agnostic — the
//! engine delegates actual HTTP dispatch to a user-supplied callback via
//! [`Engine::set_on_http_send`](crate::Engine::set_on_http_send). The test harness
//! injects a blocking HTTP client; production users inject their own dispatcher.
//!
//! This separation matches the C++ design: `StaticExecutionEngine` never links
//! against an HTTP library. `HttpEventTarget` (in `sce_runtime`) and
//! `W3CHttpTestServer` (in tests) live in separate translation units.

use std::collections::HashMap;

/// W3C SCXML C.2: HTTP send request payload passed to the `on_http_send` callback.
///
/// Matches C++ `SCE::Static::HttpSendRequest` 1:1.
#[derive(Debug, Clone, Default)]
pub struct HttpSendRequest {
    /// Absolute HTTP URL (e.g., `http://localhost:8080/test`).
    pub target: String,
    /// Event name to encode in the HTTP POST payload.
    pub event_name: String,
    /// Raw content body (W3C SCXML C.2: sent as multipart form data).
    pub content: String,
    /// Form parameters (W3C SCXML 6.2 `<param>` elements). Multiple values per key allowed.
    pub params: HashMap<String, Vec<String>>,
    /// Send ID for correlation and cancellation (W3C SCXML 6.2.5).
    pub send_id: String,
}

/// W3C SCXML C.2: HTTP send response returned by the `on_http_send` callback.
///
/// When the callback returns `Some(HttpSendResponse)`, the engine injects the
/// response event into the external queue. This enables real HTTP round-trips
/// against the shared W3C HTTP test server (`standalone_http_server.js`).
#[derive(Debug, Clone, Default)]
pub struct HttpSendResponse {
    /// Event name extracted from the HTTP response (e.g., `event1`, `HTTP.POST`).
    pub event_name: String,
    /// Event data from the response (W3C SCXML 5.10.3: `_event.data`).
    pub event_data: String,
}
