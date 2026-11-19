// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! HTTP (docs/SCE_HOST_FEATURES.md, section 5): the semantics over a transport
//! the embedding supplies.

use crate::{failure, record, Outcome, ReferenceHost};
use serde_json::json;

/// The most bytes of response headers and of body a result record holds.
const HEADERS_BOUND: usize = 4096;
/// The most bytes of a response's body the result record holds (`body` of
/// `http_fetch_result`), the bound a network transport reads a body to.
pub const HTTP_BODY_MAX_BYTES: usize = 16384;

const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];

/// What the host hands its transport: the request, its header lines already
/// split into name and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// What a transport answers: the response as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Why a transport answered no response, each the failure code of section 5
/// it is reported as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailure {
    /// No response was received: name resolution, connection, TLS, a timeout,
    /// or the connection ended (`network`).
    Network,
    /// The transport's destination policy refused the target before anything
    /// was sent: it is not on the allow-list, or every address its name
    /// resolved to is refused (`destination_refused`; docs/adr/0109, D4).
    DestinationRefused,
    /// The response passed a bound of the transport before it was read whole
    /// (`response_too_large`); nothing of it is kept.
    ResponseTooLarge,
}

impl TransportFailure {
    /// The failure code of section 5 this is reported as.
    pub fn code(self) -> &'static str {
        match self {
            TransportFailure::Network => "network",
            TransportFailure::DestinationRefused => "destination_refused",
            TransportFailure::ResponseTooLarge => "response_too_large",
        }
    }
}

/// Carries one exchange. `Ok(None)` is a transport that has not answered: the
/// host holds the exchange until it is cancelled.
pub trait Transport {
    fn exchange(&mut self, request: &HttpRequest)
        -> Result<Option<HttpResponse>, TransportFailure>;
}

/// An RFC 9110 token: what a header name is made of.
fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

/// The header lines of a request, split into name and value, or `None` for a
/// line that is not one. Empty lines are ignored, and the value is without the
/// spaces and tabs around it.
fn header_lines(text: &str) -> Option<Vec<(String, String)>> {
    let mut headers = Vec::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':')?;
        if !is_token(name) {
            return None;
        }
        headers.push((
            name.to_string(),
            value.trim_matches(|c| c == ' ' || c == '\t').to_string(),
        ));
    }
    Some(headers)
}

/// An absolute `http` or `https` URL with a host, and no ASCII space or control
/// character (U+0000 to U+0020 and U+007F) in it.
fn is_http_url(url: &str) -> bool {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"));
    let Some(rest) = rest else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty()
        && !authority.starts_with(':')
        && !url.chars().any(|c| c <= ' ' || c == '\u{7f}')
}

/// Whether the arguments of an exchange are ones the operation accepts.
pub(crate) fn acceptable(method: &str, url: &str, headers: &str) -> bool {
    METHODS.contains(&method) && is_http_url(url) && header_lines(headers).is_some()
}

pub(crate) fn fetch<T: Transport>(
    host: &mut ReferenceHost<T>,
    id: &str,
    method: &str,
    url: &str,
    headers: &str,
    body: &str,
) -> Outcome {
    let request = HttpRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: header_lines(headers).expect("the arguments were accepted"),
        body: body.to_string(),
    };
    let response = match host.transport.exchange(&request) {
        Ok(Some(response)) => response,
        Ok(None) => {
            host.held.push(id.to_string());
            return Outcome::Hold;
        }
        Err(why) => return failure("sce-host-http-fetch", why.code()),
    };
    let lines: Vec<String> = response
        .headers
        .iter()
        .map(|(name, value)| format!("{}: {}", name.to_ascii_lowercase(), value.trim()))
        .collect();
    let headers = lines.join("\n");
    // A response to HEAD has no body, whatever the transport carried.
    let body = if method == "HEAD" {
        Vec::new()
    } else {
        response.body
    };
    if headers.len() > HEADERS_BOUND || body.len() > HTTP_BODY_MAX_BYTES {
        return failure("sce-host-http-fetch", "response_too_large");
    }
    let Ok(body) = String::from_utf8(body) else {
        return failure("sce-host-http-fetch", "invalid_body");
    };
    record(json!({ "status": response.status, "headers": headers, "body": body }))
}
