// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The HTTPS binding (SCE_MESH.md §mesh-22): a client reaches a server by HTTP request and
//! response, so that a client on a network it does not know (a proxy it must tunnel through, a
//! TLS inspection it must accept) still takes part in Mesh.
//!
//! Three layers, and the dependency points from each to the one before it:
//!
//! * The semantics, which are this module and [`client`] and [`server`]: the path, the framing
//!   (a CBOR sequence of envelopes), the mailbox and its acknowledgement, the hold, readiness, the
//!   status codes, the authenticators and the message signatures ([`signature`]). They are the
//!   same in every language, and so every adapter gets them.
//! * The port, [`HttpsTransport`]: one exchange at the level of HTTP semantics, a POST with its
//!   headers and body and a bound on the wait, answered with a response or a failure of a named
//!   kind. The client reaches the network through it and through nothing else.
//! * The adapters, which implement the port. [`adapter::HyperTransport`] is this crate's, on hyper
//!   and rustls with the operating system's verifier and proxy settings; a program with an HTTP
//!   stack of its own implements the port on it instead, and the conformance kit
//!   (the module `conformance`, under the feature `https-conformance`) holds either to the same
//!   behaviour.
//!
//! What the binding hands its host is the [`LinkEvent`]s every binding hands it: a peer became
//! ready, bytes arrived from it, an envelope was not written, it was lost. The client hands two
//! more, since its server keeps a mailbox: the link was interrupted with nothing lost (an
//! exchange failed at the transport), and the server started again without the mailbox (a
//! response of a new incarnation).

pub mod adapter;
pub mod client;
#[cfg(feature = "https-conformance")]
pub mod conformance;
pub mod rate;
pub mod server;
pub mod signature;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use sce_forge_runtime::cbor;
use sce_forge_runtime::codec::SceCursor;
use sha2::{Digest, Sha256};

pub use crate::link::{deliver, is_path_segment, LinkEvent};

/// The path segment before the version and the client's name (section 22.1).
pub const PATH_ROOT: &str = "/sce-mesh/";

/// The version of the binding this crate speaks, as the path spells it.
pub const VERSION: &str = "1";

/// The path a client posts to, before its own machine name: `/sce-mesh/1/`.
pub const PATH_PREFIX: &str = "/sce-mesh/1/";

/// The media type of a request's and a response's body: a CBOR sequence (RFC 8742), the framing
/// of §mesh-22.2.
pub const CONTENT_TYPE: &str = "application/cbor-seq";

/// The request header naming the highest mailbox number the client has received, 0 for none.
pub const ACK_HEADER: &str = "Sce-Mesh-Ack";

/// The request header naming how long, in milliseconds, the server may hold the exchange while
/// the mailbox is empty; 0 or absent answers at once.
pub const HOLD_HEADER: &str = "Sce-Mesh-Hold-Ms";

/// The response header naming the mailbox number of the first envelope of the body; absent when
/// the body is empty.
pub const FIRST_HEADER: &str = "Sce-Mesh-First";

/// The header naming a mailbox's incarnation (section 22.3), 32 lowercase hexadecimal digits: on
/// every `200` response, the incarnation the response's numbers count in; on a request, the one
/// its `Sce-Mesh-Ack` counts in, absent while the client has seen none.
pub const EPOCH_HEADER: &str = "Sce-Mesh-Epoch";

/// The authentication scheme of a bearer token (`Authorization: Bearer <token>`).
pub const BEARER_SCHEME: &str = "Bearer";

/// The authentication scheme of an installation secret
/// (`Authorization: SCE-Install <base64url of the 32-byte secret>`).
pub const INSTALL_SCHEME: &str = "SCE-Install";

/// deploy.yaml's `hold_ms` on the dialing side when the binding sets none.
pub const DEFAULT_HOLD_MS: u32 = 25_000;
/// deploy.yaml's `max_hold_ms` on the accepting side when the binding sets none.
pub const DEFAULT_MAX_HOLD_MS: u32 = 30_000;
/// deploy.yaml's `max_body_bytes` on the accepting side when the binding sets none.
pub const DEFAULT_MAX_BODY_BYTES: u64 = 1_048_576;
/// deploy.yaml's `absence_ms` on the accepting side when the binding sets none.
pub const DEFAULT_ABSENCE_MS: u32 = 120_000;
/// deploy.yaml's `mailbox` on the accepting side when the binding sets none.
pub const DEFAULT_MAILBOX: u32 = 1024;

/// One exchange, as the binding hands it to the port (§mesh-22.6): a POST of `body` to `url`
/// with `headers`, answered within `timeout` or failed as [`HttpsFailure::Timeout`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsRequest {
    /// The full URL: the deployment's base and the binding's path.
    pub url: String,
    /// Every header of the request but those the HTTP version itself owns (`Host`,
    /// `Content-Length`), in order, each name as the binding spells it.
    pub headers: Vec<(String, String)>,
    /// The request body: a CBOR sequence of envelopes, possibly empty.
    pub body: Vec<u8>,
    /// How long the adapter may take for the whole exchange, from opening a connection (or taking
    /// one it keeps) to the last byte of the response body.
    pub timeout: Duration,
}

/// What a server answered, whatever the status: the port's response (§mesh-22.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsResponse {
    /// The status code.
    pub status: u16,
    /// The response headers, in order, each name as the server sent it or in lower case.
    pub headers: Vec<(String, String)>,
    /// The response body.
    pub body: Vec<u8>,
}

impl HttpsResponse {
    /// The first header named `name`, which is compared without regard to case (RFC 9110,
    /// section 5.1).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Why an exchange got no response: the port's failure of a named kind (§mesh-22.6). Each is a
/// failure at the transport, which ends the client's readiness (section 22.4); a response of any
/// status is not a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpsFailure {
    /// No connection could be opened to the server, or to the proxy that reaches it.
    Connect,
    /// The TLS handshake failed: the server's certificate was not one the client believes, or the
    /// two ends share no protocol.
    Tls,
    /// The proxy refused the tunnel or answered something that is not a tunnel.
    Proxy,
    /// The exchange took longer than its request's `timeout`.
    Timeout,
    /// Anything else: the connection closed before the response was whole, a response that is
    /// not HTTP, an error of the adapter's own. The words say which.
    Other(String),
}

impl fmt::Display for HttpsFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpsFailure::Connect => f.write_str("no connection could be opened"),
            HttpsFailure::Tls => f.write_str("the TLS handshake failed"),
            HttpsFailure::Proxy => f.write_str("the proxy refused the tunnel"),
            HttpsFailure::Timeout => f.write_str("the exchange timed out"),
            HttpsFailure::Other(words) => f.write_str(words),
        }
    }
}

/// The port (§mesh-22.6): carries one exchange and nothing else. Every decision of the binding
/// (what to send, what to make of the answer, when to try again) is the binding's; an adapter
/// moves a request to the server and its response back.
///
/// `exchange` blocks the calling thread until the exchange ends. The binding's client calls it
/// from threads of its own, two at once at most (one held exchange that waits for the server's
/// envelopes and one that carries the client's), so an adapter is shared and must take
/// concurrent calls.
///
/// An adapter is held to the platform policy of §mesh-22.6: it believes the authorities the
/// operating system believes, and it reaches a server through the proxy the platform's settings
/// name, tunnelling with `CONNECT`.
pub trait HttpsTransport: Send + Sync {
    /// Carry `request` and return what the server answered.
    fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure>;
}

impl<T: HttpsTransport + ?Sized> HttpsTransport for alloc::sync::Arc<T> {
    fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
        (**self).exchange(request)
    }
}

/// How a client proves the name it posts under (section 22.5).
#[derive(Clone, PartialEq, Eq)]
pub enum Credential {
    /// A token the server's host issued and verifies: `Authorization: Bearer <token>`.
    Bearer(String),
    /// The installation's 256-bit secret, drawn once at the first run ([`draw_install_secret`])
    /// and kept by the host: `Authorization: SCE-Install <base64url of the secret>`. The server
    /// pins the name to the secret's SHA-256 at the first exchange it answers `200`.
    Install([u8; 32]),
    /// A TLS client certificate, which the adapter presents; no header is sent.
    Mtls,
}

impl fmt::Debug for Credential {
    // A credential is a secret; a log that prints one hands it out.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credential::Bearer(_) => f.write_str("Bearer(..)"),
            Credential::Install(_) => f.write_str("Install(..)"),
            Credential::Mtls => f.write_str("Mtls"),
        }
    }
}

impl Credential {
    /// The `Authorization` header's value, or `None` for a certificate.
    pub fn authorization(&self) -> Option<String> {
        match self {
            Credential::Bearer(token) => Some(alloc::format!("{BEARER_SCHEME} {token}")),
            Credential::Install(secret) => Some(alloc::format!(
                "{INSTALL_SCHEME} {}",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret)
            )),
            Credential::Mtls => None,
        }
    }
}

/// A fresh installation secret: 32 bytes from the operating system's random source, through the
/// cryptography every TLS connection of the bindings runs on ([`crate::tls::provider`]).
pub fn draw_install_secret() -> Result<[u8; 32], crate::tls::Error> {
    let mut secret = [0u8; 32];
    crate::tls::provider()
        .secure_random
        .fill(&mut secret)
        .map_err(|_| {
            crate::tls::Error::General("the random source could not be read".to_string())
        })?;
    Ok(secret)
}

/// A fresh mailbox incarnation (section 22.3): 16 bytes from the operating system's random
/// source, through the same cryptography as [`draw_install_secret`], written as 32 lowercase
/// hexadecimal digits.
pub fn draw_epoch() -> Result<String, crate::tls::Error> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut bytes = [0u8; 16];
    crate::tls::provider()
        .secure_random
        .fill(&mut bytes)
        .map_err(|_| {
            crate::tls::Error::General("the random source could not be read".to_string())
        })?;
    let mut epoch = String::with_capacity(32);
    for byte in bytes {
        epoch.push(char::from(DIGITS[usize::from(byte >> 4)]));
        epoch.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    Ok(epoch)
}

/// Whether `text` names a mailbox incarnation: exactly 32 lowercase hexadecimal digits.
pub fn is_epoch(text: &str) -> bool {
    text.len() == 32 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The digest a server pins an installation secret by: its SHA-256.
pub fn install_digest(secret: &[u8]) -> [u8; 32] {
    Sha256::digest(secret).into()
}

/// The secret an `SCE-Install` credential carries (the text after the scheme): base64url, with
/// or without its padding, of exactly 32 bytes. Anything else is `None`.
pub fn decode_install_secret(text: &str) -> Option<[u8; 32]> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text.trim_end_matches('='))
        .ok()?;
    bytes.try_into().ok()
}

/// The envelopes a body carries, in order (§mesh-22.2). Each is one CBOR data item, found by
/// skipping it whole; bytes from which no item can be read (a truncated or malformed item) are
/// handed on as one last piece, so that the core reports them as an envelope that does not
/// decode (SCE_MESH.md section 16.7 row 4) and nothing is dropped unseen.
pub fn split_sequence(body: &[u8]) -> Vec<Vec<u8>> {
    let mut items = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        let mut cursor = SceCursor::new(rest);
        if cbor::skip(&mut cursor).is_err() {
            items.push(rest.to_vec());
            break;
        }
        let used = rest.len() - cursor.remaining();
        items.push(rest[..used].to_vec());
        rest = &rest[used..];
    }
    items
}

/// A body carrying `envelopes`, each its CBOR bytes, one after another (RFC 8742): the request
/// and response body of §mesh-22.2.
pub fn join_sequence<'a>(envelopes: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut body = Vec::new();
    for envelope in envelopes {
        body.extend_from_slice(envelope);
    }
    body
}

/// How long a `Retry-After` value asks to wait, read at `now`: delay-seconds, or an HTTP date in
/// the IMF-fixdate form (RFC 9110, sections 10.2.3 and 5.6.7). A date already past asks for no
/// wait. A value of neither form is `None`.
pub fn retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse::<u64>().ok().map(Duration::from_secs);
    }
    let at = UNIX_EPOCH + Duration::from_secs(imf_fixdate(value)?);
    Some(at.duration_since(now).unwrap_or(Duration::ZERO))
}

/// Seconds since the Unix epoch of an IMF-fixdate, `Sun, 06 Nov 1994 08:49:37 GMT`.
fn imf_fixdate(value: &str) -> Option<u64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut words = value.split(' ');
    let weekday = words.next()?;
    let day = words.next()?;
    let month = words.next()?;
    let year = words.next()?;
    let time = words.next()?;
    if words.next()? != "GMT" || words.next().is_some() {
        return None;
    }
    if weekday.len() != 4 || !weekday.ends_with(',') || day.len() != 2 || year.len() != 4 {
        return None;
    }
    let number = |text: &str| -> Option<u64> {
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    };
    let day = number(day)?;
    let month = MONTHS.iter().position(|m| *m == month)? as u64 + 1;
    let year = number(year)?;
    let mut clock = time.split(':');
    let (hour, minute, second) = (
        number(clock.next()?)?,
        number(clock.next()?)?,
        number(clock.next()?)?,
    );
    if clock.next().is_some()
        || year < 1970
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Days from the civil date, counted from 1970-01-01 (the proleptic Gregorian calendar).
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y / 400;
    let year_of_era = y - era * 400;
    let day_of_year = (153 * m + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = (era * 146_097 + day_of_era).checked_sub(719_468)?;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::envelope::Envelope;

    /// The bytes of an envelope told apart from others by `deadline`.
    fn envelope(deadline: u64) -> Vec<u8> {
        Envelope {
            deadline_unix_ms: Some(deadline),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    #[test]
    fn a_body_is_split_into_the_envelopes_it_joins() {
        let first = envelope(1);
        let second = envelope(2);
        assert_ne!(first, second);
        let body = join_sequence([first.as_slice(), second.as_slice()]);
        assert_eq!(split_sequence(&body), alloc::vec![first, second]);
        assert!(
            split_sequence(&[]).is_empty(),
            "an empty body is no envelope"
        );
    }

    /// What does not read as a CBOR item is handed on whole, as the last piece, so that the core
    /// reports it as row 4 instead of the binding dropping it unseen.
    #[test]
    fn bytes_that_are_no_item_are_handed_on_as_one_last_piece() {
        let first = envelope(1);
        let mut body = first.clone();
        // A map head that promises one entry and ends.
        body.extend_from_slice(&[0xa1, 0x01]);
        assert_eq!(
            split_sequence(&body),
            alloc::vec![first, alloc::vec![0xa1, 0x01]]
        );
    }

    #[test]
    fn an_installation_secret_is_sent_as_base64url_and_read_back() {
        let secret = [7u8; 32];
        let header = Credential::Install(secret).authorization().unwrap();
        let text = header.strip_prefix("SCE-Install ").unwrap();
        assert_eq!(text.len(), 43, "32 bytes are 43 characters without padding");
        assert!(!text.contains('='));
        assert_eq!(decode_install_secret(text), Some(secret));
        let padded = alloc::format!("{text}=");
        assert_eq!(decode_install_secret(&padded), Some(secret));
        assert_eq!(decode_install_secret("c2hvcnQ"), None, "too short");
        assert_eq!(decode_install_secret("!!"), None, "not base64url");
    }

    #[test]
    fn a_bearer_token_is_sent_under_its_scheme_and_a_certificate_sends_nothing() {
        assert_eq!(
            Credential::Bearer("t0k".to_string()).authorization(),
            Some("Bearer t0k".to_string())
        );
        assert_eq!(Credential::Mtls.authorization(), None);
        assert_eq!(
            alloc::format!("{:?}", Credential::Bearer("secret".to_string())),
            "Bearer(..)"
        );
    }

    #[test]
    fn two_installation_secrets_drawn_are_two() {
        let a = draw_install_secret().unwrap();
        let b = draw_install_secret().unwrap();
        assert_ne!(a, b);
        assert_ne!(install_digest(&a), install_digest(&b));
        assert_eq!(install_digest(&a), install_digest(&a));
    }

    #[test]
    fn two_epochs_drawn_are_two_and_each_names_an_incarnation() {
        let a = draw_epoch().unwrap();
        let b = draw_epoch().unwrap();
        assert_ne!(a, b);
        assert!(is_epoch(&a), "{a}");
        assert!(is_epoch(&b), "{b}");
        assert!(is_epoch("0123456789abcdef0123456789abcdef"));
        assert!(
            !is_epoch("0123456789ABCDEF0123456789abcdef"),
            "lowercase only"
        );
        assert!(!is_epoch(&a[1..]), "thirty-one digits");
        assert!(!is_epoch(&alloc::format!("{a}0")), "thirty-three digits");
        assert!(!is_epoch("0123456789abcdef0123456789abcdeg"));
    }

    #[test]
    fn retry_after_is_read_in_seconds_or_as_a_date() {
        let now = UNIX_EPOCH + Duration::from_secs(784_111_777);
        assert_eq!(retry_after("120", now), Some(Duration::from_secs(120)));
        assert_eq!(retry_after(" 0 ", now), Some(Duration::ZERO));
        // RFC 9110's example date is 784111777 seconds after the epoch.
        assert_eq!(
            retry_after("Sun, 06 Nov 1994 08:49:47 GMT", now),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            retry_after("Sun, 06 Nov 1994 08:49:27 GMT", now),
            Some(Duration::ZERO),
            "a date already past asks for no wait"
        );
        assert_eq!(imf_fixdate("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(
            imf_fixdate("Tue, 29 Feb 2000 00:00:00 GMT"),
            Some(951_782_400)
        );
        for bad in [
            "",
            "-1",
            "soon",
            "Sun, 06 Nov 1994 08:49:37 UTC",
            "Sunday, 06-Nov-94 08:49:37 GMT",
            "Sun, 06 Nov 1994 24:00:00 GMT",
        ] {
            assert_eq!(retry_after(bad, now), None, "{bad:?}");
        }
    }

    #[test]
    fn a_response_header_is_found_whatever_its_case() {
        let response = HttpsResponse {
            status: 200,
            headers: alloc::vec![("sce-mesh-first".to_string(), "4".to_string())],
            body: Vec::new(),
        };
        assert_eq!(response.header(FIRST_HEADER), Some("4"));
        assert_eq!(response.header("Retry-After"), None);
    }
}
