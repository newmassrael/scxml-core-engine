// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-relay probe`: one real exchange with a relay, as a device would make it, timed stage by
//! stage, and the one thing it came to.
//!
//! A relay that is running and holds its port can still fail every device: its certificate expired,
//! its authority's key changed, its store stopped taking records. A probe goes the whole way a
//! device goes: it resolves the relay's name, connects, completes TLS believing the authorities it
//! is told to (a `wss://` relay), asks for the upgrade presenting a token, which the relay judges,
//! and makes requests of the protocol. Each stage is taken on its own, so that a failure says which
//! stage failed ([`Outcome`]), and the exit status says it to a monitor (`crate::exit`).
//!
//! What it asks depends on the list its token is for. A token for the probe list ([`PROBE_LIST`],
//! which no application names) lets it write: it gives the relay the list's first entry, the same
//! entry on every run ([`probe_genesis`]), which the first probe makes the relay keep and every later
//! one finds held, so the store is written once and the list never grows; it then reads the log
//! back and checks that its first entry is that one. A token for any other list is used only to
//! read that list's log: a probe never writes to a list an application holds.
//!
//! The token is one the relay's authority issued: handed to the probe in a file, or issued by the
//! probe itself for the probe list from the authority's key ([`own_token`]). The probe never
//! prints a token.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use sce_rust_relay::tls::{self, ClientConfig, Roots};
use sce_rust_relay::wss;
use sce_rust_relay::{Authority, Claims, Code, Request, Response};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::Instant as Deadline;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::{http, Error as WsError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::exit;
use crate::logline::word;

/// The list a probe writes to: its own, a name no application gives a list.
pub const PROBE_LIST: &str = "sce-relay-probe";

/// The node the token a probe issues itself names in the probe list.
pub const PROBE_NODE: u64 = 1;

/// How long the token a probe issues itself is good for, in seconds: the probe's own time and room
/// for the probe's clock and the relay's to differ.
pub const PROBE_TOKEN_SECONDS: u64 = 300;

/// The seed of the probe list's administrator. Anyone can make this key from the bytes here, and it
/// guards nothing: the list holds no member and nothing that is anyone's, and only a token for it,
/// which the holder of the relay's authority issues, reaches it.
const PROBE_ADMINISTRATOR_SEED: [u8; 32] = *b"sce-relay probe administrator 1\0";

/// The first entry of the probe list: the same on every run and every host, so that a relay that
/// holds it already takes it again as held and keeps nothing more.
pub fn probe_genesis() -> Entry {
    let administrator = Identity::from_seed(PROBE_ADMINISTRATOR_SEED);
    Log::genesis_draft(
        PROBE_LIST,
        Body::Genesis {
            administrators: vec![administrator.public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator)
    .finish()
}

/// A token for the probe's node of the probe list from `authority`, good for
/// [`PROBE_TOKEN_SECONDS`] from `now_ms`.
pub fn own_token(authority: &Authority, now_ms: u64) -> String {
    authority.issue(
        PROBE_LIST,
        PROBE_NODE,
        now_ms.saturating_add(PROBE_TOKEN_SECONDS * 1000),
    )
}

/// What a probe came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The relay answered every request as the protocol says, and gave back what was written.
    Ok,
    /// The relay was not reached: its name did not resolve, or nothing took the connection at its
    /// address in time.
    Unreachable,
    /// The TLS handshake failed: a certificate under an authority the probe does not believe, for
    /// another name or expired, or no TLS at the address.
    TlsRefused,
    /// The relay refused the token: at the upgrade, or as `unauthorized`.
    TokenRefused,
    /// The relay answered `unavailable`: its store did not take a record, and it takes nothing
    /// until its operator has looked.
    StoreFailed,
    /// The relay took the connection and did not finish a stage in time, closed the connection
    /// before it answered, or answered `rate_limited`.
    Busy,
    /// The relay answered, but not as the protocol says.
    ProtocolError,
}

impl Outcome {
    /// The word a report line names it by.
    pub fn word(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Unreachable => "unreachable",
            Outcome::TlsRefused => "tls_refused",
            Outcome::TokenRefused => "token_refused",
            Outcome::StoreFailed => "store_failed",
            Outcome::Busy => "busy",
            Outcome::ProtocolError => "protocol_error",
        }
    }

    /// The status `sce-relay probe` exits with.
    pub fn status(self) -> u8 {
        match self {
            Outcome::Ok => exit::OK,
            Outcome::Unreachable => exit::UNAVAILABLE,
            Outcome::TlsRefused => exit::TLS_REFUSED,
            Outcome::TokenRefused => exit::NO_PERMISSION,
            Outcome::StoreFailed => exit::IO_ERROR,
            Outcome::Busy => exit::TEMPORARY,
            Outcome::ProtocolError => exit::PROTOCOL,
        }
    }
}

/// A stage of a probe, in the order they are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The relay's name to its addresses.
    Resolve,
    /// A connection to one of them.
    Connect,
    /// The TLS handshake, for a `wss://` relay.
    Tls,
    /// The WebSocket upgrade, with the token.
    Upgrade,
    /// The requests and their responses.
    Exchange,
}

impl Stage {
    /// The word a report line names it by.
    pub fn word(self) -> &'static str {
        match self {
            Stage::Resolve => "resolve",
            Stage::Connect => "connect",
            Stage::Tls => "tls",
            Stage::Upgrade => "upgrade",
            Stage::Exchange => "exchange",
        }
    }

    /// What a stage that did not end in time comes to: before a connection, the relay was not
    /// reached; after it, the relay is there and did not answer.
    fn late(self) -> Outcome {
        match self {
            Stage::Resolve | Stage::Connect => Outcome::Unreachable,
            Stage::Tls | Stage::Upgrade | Stage::Exchange => Outcome::Busy,
        }
    }
}

/// What a probe asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The log of the token's list, read: a token for a list that is not the probe list.
    Read,
    /// The probe list's first entry, given and read back.
    WriteRead,
}

impl Asked {
    pub fn word(self) -> &'static str {
        match self {
            Asked::Read => "read",
            Asked::WriteRead => "write_read",
        }
    }
}

/// What a probe saw: its outcome, the stage it ended at, and how long each stage took.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The address the probe dialled: the relay's base and its path.
    pub relay: String,
    pub outcome: Outcome,
    /// The stage the probe was at when it ended; the last one when it came to [`Outcome::Ok`].
    pub stage: Stage,
    pub asked: Asked,
    /// Whether the probe's write made the relay keep the probe list's first entry (`Some(true)`),
    /// found it held (`Some(false)`), or was not made. A relay that keeps it again after the first
    /// probe has lost its store.
    pub created: Option<bool>,
    /// How long each stage that ended took, in order.
    pub timings: Vec<(Stage, Duration)>,
    /// How long the whole probe took.
    pub total: Duration,
    /// Why, when the outcome is not [`Outcome::Ok`]: never the token.
    pub reason: Option<String>,
}

impl Report {
    /// The report as one line of `key=value` words: the outcome, the relay, what was asked, the
    /// time of each stage and of the whole in milliseconds, and, when it failed, the stage and why.
    pub fn line(&self) -> String {
        let mut line = format!(
            "outcome={} relay={} asked={}",
            self.outcome.word(),
            word(&self.relay),
            self.asked.word()
        );
        if let Some(created) = self.created {
            line.push_str(if created {
                " list=created"
            } else {
                " list=held"
            });
        }
        if self.outcome != Outcome::Ok {
            line.push_str(" stage=");
            line.push_str(self.stage.word());
        }
        for (stage, took) in &self.timings {
            line.push_str(&format!(" {}_ms={}", stage.word(), milliseconds(*took)));
        }
        line.push_str(&format!(" total_ms={}", milliseconds(self.total)));
        if let Some(reason) = &self.reason {
            line.push_str(" reason=");
            line.push_str(&word(reason));
        }
        line
    }

    /// Take `stage`, given until `deadline`, and note how long it took when it ended.
    async fn take<T, F>(&mut self, stage: Stage, deadline: Deadline, work: F) -> Result<T, Failure>
    where
        F: std::future::Future<Output = Result<T, Failure>>,
    {
        self.stage = stage;
        let began = Instant::now();
        let done = match tokio::time::timeout_at(deadline, work).await {
            Ok(done) => done?,
            Err(_) => {
                return Err((
                    stage.late(),
                    format!(
                        "the {} stage did not end within the time given",
                        stage.word()
                    ),
                ))
            }
        };
        self.timings.push((stage, began.elapsed()));
        Ok(done)
    }
}

/// A duration in milliseconds, to a tenth.
fn milliseconds(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64() * 1000.0)
}

/// Why a stage failed, and what that comes to.
type Failure = (Outcome, String);

/// Why a probe was not begun: what it was given is wrong, before anything is dialled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    /// The address is not a relay's (`ws://` or `wss://`, with a host).
    Address(String),
    /// The authorities to believe are not certificates, or the token is not a header's value.
    Unusable(String),
}

/// The relay a probe dials, what it believes of its certificate, and the token it presents.
pub struct Target {
    url: String,
    host: String,
    port: u16,
    /// The name the relay's certificate must be for: a `wss://` relay's host.
    server_name: Option<ServerName<'static>>,
    tls: Arc<ClientConfig>,
    token: String,
    writes: bool,
}

impl Target {
    /// The relay at `base` (`ws://host[:port]` or `wss://host[:port]`), believing `roots` for its
    /// certificate, with `token`.
    pub fn new(base: &str, roots: &Roots, token: &str) -> Result<Self, TargetError> {
        let url = wss::endpoint(base).map_err(|why| TargetError::Address(why.to_string()))?;
        let uri: http::Uri = url
            .parse()
            .map_err(|why| TargetError::Address(format!("`{base}` is not an address: {why}")))?;
        let secure = uri.scheme_str() == Some("wss");
        let named = uri
            .host()
            .filter(|host| !host.is_empty())
            .ok_or_else(|| TargetError::Address(format!("`{base}` names no host")))?;
        // An IPv6 address is written in brackets, which are not part of it.
        let host = named
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .unwrap_or(named)
            .to_string();
        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
        let server_name = if secure {
            Some(ServerName::try_from(host.clone()).map_err(|_| {
                TargetError::Address(format!("`{host}` is not a name a certificate is for"))
            })?)
        } else {
            None
        };
        let tls = tls::client_config(roots).map_err(|why| {
            TargetError::Unusable(format!("the authorities to believe are not usable: {why}"))
        })?;
        wss::upgrade_request(&url, token).map_err(TargetError::Unusable)?;
        let writes = Claims::of(token).is_ok_and(|claims| claims.list == PROBE_LIST);
        Ok(Self {
            url,
            host,
            port,
            server_name,
            tls,
            token: token.to_string(),
            writes,
        })
    }

    /// The address the probe dials: the relay's base and its path.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Whether the probe writes: whether its token is for the probe list.
    pub fn writes(&self) -> bool {
        self.writes
    }
}

/// Probe the relay `target` names, all of it within `within`.
pub async fn probe(target: &Target, within: Duration) -> Report {
    let began = Instant::now();
    let deadline = Deadline::now() + within;
    let mut report = Report {
        relay: target.url.clone(),
        outcome: Outcome::Ok,
        stage: Stage::Resolve,
        asked: if target.writes {
            Asked::WriteRead
        } else {
            Asked::Read
        },
        created: None,
        timings: Vec::new(),
        total: Duration::ZERO,
        reason: None,
    };
    match run(target, deadline, &mut report).await {
        Ok(created) => report.created = created,
        Err((outcome, reason)) => {
            report.outcome = outcome;
            report.reason = Some(reason);
        }
    }
    report.total = began.elapsed();
    report
}

/// The stages, one after the other: what the write found, when the probe wrote.
async fn run(
    target: &Target,
    deadline: Deadline,
    report: &mut Report,
) -> Result<Option<bool>, Failure> {
    let host = target.host.as_str();
    let port = target.port;
    let addresses = report
        .take(Stage::Resolve, deadline, async move {
            let found: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host, port))
                .await
                .map_err(|why| {
                    (
                        Outcome::Unreachable,
                        format!("`{host}` does not resolve: {why}"),
                    )
                })?
                .collect();
            if found.is_empty() {
                return Err((
                    Outcome::Unreachable,
                    format!("`{host}` resolves to no address"),
                ));
            }
            Ok(found)
        })
        .await?;

    let connection = report
        .take(Stage::Connect, deadline, async move {
            let mut failures = Vec::new();
            for address in addresses {
                match TcpStream::connect(address).await {
                    Ok(stream) => return Ok(stream),
                    Err(why) => failures.push(format!("{address}: {why}")),
                }
            }
            Err((
                Outcome::Unreachable,
                format!("no connection was taken ({})", failures.join("; ")),
            ))
        })
        .await?;

    let stream = match &target.server_name {
        Some(name) => {
            let connector = TlsConnector::from(target.tls.clone());
            let name = name.clone();
            let secured = report
                .take(Stage::Tls, deadline, async move {
                    connector.connect(name, connection).await.map_err(|why| {
                        (
                            Outcome::TlsRefused,
                            format!("the TLS handshake failed: {why}"),
                        )
                    })
                })
                .await?;
            MaybeTlsStream::Rustls(secured)
        }
        None => MaybeTlsStream::Plain(connection),
    };

    let upgrade = wss::upgrade_request(&target.url, &target.token)
        .map_err(|why| (Outcome::ProtocolError, why))?;
    let (mut socket, _) = report
        .take(Stage::Upgrade, deadline, async move {
            tokio_tungstenite::client_async_with_config(upgrade, stream, None)
                .await
                .map_err(upgrade_failure)
        })
        .await?;

    let writes = target.writes;
    report
        .take(Stage::Exchange, deadline, async move {
            let created = exchange(&mut socket, writes).await;
            // The probe is done with the connection whatever it came to; a close the relay does
            // not take changes nothing of what it answered.
            let _ = socket.close(None).await;
            created
        })
        .await
}

/// What a refused upgrade comes to.
fn upgrade_failure(error: WsError) -> Failure {
    match error {
        WsError::Http(response) if response.status() == http::StatusCode::UNAUTHORIZED => {
            let why = response
                .body()
                .as_deref()
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            (
                Outcome::TokenRefused,
                format!("the relay refused the token at the upgrade: {why}"),
            )
        }
        WsError::Http(response) => (
            Outcome::ProtocolError,
            format!(
                "the relay answered the upgrade with {} and not 101",
                response.status()
            ),
        ),
        closed @ (WsError::Io(_)
        | WsError::ConnectionClosed
        | WsError::AlreadyClosed
        | WsError::Protocol(ProtocolError::HandshakeIncomplete)) => (
            Outcome::Busy,
            format!("the relay closed the connection before it answered the upgrade: {closed}"),
        ),
        other => (
            Outcome::ProtocolError,
            format!("the upgrade was not a WebSocket's: {other}"),
        ),
    }
}

/// The requests of a probe on an open connection: the probe list's first entry given and the log
/// read back when it writes, the log read when it does not.
async fn exchange<S>(socket: &mut WebSocketStream<S>, writes: bool) -> Result<Option<bool>, Failure>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let genesis = probe_genesis();
    let created = if writes {
        let given = Request::Append {
            entry: genesis.to_json(),
        };
        match ask(socket, &given).await? {
            Response::Appended { new, .. } => Some(new),
            other => return Err(unexpected("append", other)),
        }
    } else {
        None
    };
    match ask(socket, &Request::Entries { from: 0 }).await? {
        Response::Entries { entries, .. } => {
            if writes {
                let first = entries.first().and_then(|text| Entry::from_json(text));
                if first.as_ref() != Some(&genesis) {
                    return Err((
                        Outcome::ProtocolError,
                        "the relay gave back a first entry of the probe list that is not the one \
                         given"
                            .to_string(),
                    ));
                }
            }
            Ok(created)
        }
        other => Err(unexpected("entries", other)),
    }
}

/// Send `request` and read the response that answers it.
async fn ask<S>(socket: &mut WebSocketStream<S>, request: &Request) -> Result<Response, Failure>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let closed = |why: String| {
        (
            Outcome::Busy,
            format!("the relay closed the connection before it answered: {why}"),
        )
    };
    socket
        .send(Message::Text(request.to_json().into()))
        .await
        .map_err(|why| closed(why.to_string()))?;
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                return Response::from_json(text.as_str()).map_err(|why| {
                    (
                        Outcome::ProtocolError,
                        format!("the relay's answer is not a response: {why}"),
                    )
                })
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(other)) => {
                return Err((
                    Outcome::ProtocolError,
                    format!("the relay answered with a message that is not text: {other:?}"),
                ))
            }
            Some(Err(why)) => return Err(closed(why.to_string())),
            None => return Err(closed("the connection ended".to_string())),
        }
    }
}

/// What an answer that is not the one a request earns comes to: a refusal by its code, anything
/// else a protocol error.
fn unexpected(asked: &str, response: Response) -> Failure {
    match response {
        Response::Refused { code, reason } => {
            let outcome = match code {
                Code::Unauthorized => Outcome::TokenRefused,
                Code::Unavailable => Outcome::StoreFailed,
                Code::RateLimited { .. } => Outcome::Busy,
                Code::Forbidden
                | Code::TooLarge
                | Code::Invalid
                | Code::Behind
                | Code::Conflict { .. } => Outcome::ProtocolError,
            };
            (
                outcome,
                format!("the relay refused `{asked}` ({code:?}): {reason}"),
            )
        }
        other => (
            Outcome::ProtocolError,
            format!("the relay answered `{asked}` with another response: {other:?}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outcome_has_a_word_and_a_status_of_its_own() {
        let outcomes = [
            Outcome::Ok,
            Outcome::Unreachable,
            Outcome::TlsRefused,
            Outcome::TokenRefused,
            Outcome::StoreFailed,
            Outcome::Busy,
            Outcome::ProtocolError,
        ];
        let words: std::collections::BTreeSet<&str> = outcomes.iter().map(|o| o.word()).collect();
        let statuses: std::collections::BTreeSet<u8> =
            outcomes.iter().map(|o| o.status()).collect();
        assert_eq!(words.len(), outcomes.len());
        assert_eq!(statuses.len(), outcomes.len());
        assert_eq!(Outcome::Ok.status(), 0);
        assert_eq!(Outcome::Unreachable.status(), 69);
        assert_eq!(Outcome::StoreFailed.status(), 74);
        assert_eq!(Outcome::Busy.status(), 75);
        assert_eq!(Outcome::ProtocolError.status(), 76);
        assert_eq!(Outcome::TokenRefused.status(), 77);
        assert_eq!(Outcome::TlsRefused.status(), 79);
    }

    #[test]
    fn a_refusal_comes_to_the_outcome_its_code_says() {
        let refused = |code: Code| unexpected("append", Response::refused(code, "why")).0;
        assert_eq!(refused(Code::Unauthorized), Outcome::TokenRefused);
        assert_eq!(refused(Code::Unavailable), Outcome::StoreFailed);
        assert_eq!(
            refused(Code::RateLimited { retry_after_ms: 5 }),
            Outcome::Busy
        );
        for code in [Code::Forbidden, Code::TooLarge, Code::Invalid, Code::Behind] {
            assert_eq!(refused(code), Outcome::ProtocolError);
        }
        let (outcome, reason) = unexpected("entries", Response::Appended { new: true, len: 1 });
        assert_eq!(outcome, Outcome::ProtocolError);
        assert!(reason.contains("`entries`"), "{reason}");
    }

    #[test]
    fn the_probe_lists_first_entry_is_the_same_on_every_run() {
        let genesis = probe_genesis();
        assert_eq!(genesis, probe_genesis());
        assert_eq!(genesis.list, PROBE_LIST);
        assert_eq!(Entry::from_json(&genesis.to_json()), Some(genesis));
    }

    #[test]
    fn a_probe_writes_only_with_a_token_for_the_probe_list() {
        let authority = Authority::new([3; 32], "relay.test");
        let own = Target::new(
            "ws://127.0.0.1:1",
            &Roots::Public,
            &own_token(&authority, 0),
        )
        .unwrap();
        assert!(own.writes());
        let other = authority.issue("home", PROBE_NODE, u64::MAX);
        let read = Target::new("ws://127.0.0.1:1", &Roots::Public, &other).unwrap();
        assert!(!read.writes());
        let opaque = Target::new("ws://127.0.0.1:1", &Roots::Public, "not-a-token").unwrap();
        assert!(
            !opaque.writes(),
            "a text that is not a token is only read with"
        );
    }

    #[test]
    fn a_token_the_probe_issues_itself_names_its_node_of_the_probe_list_for_a_while() {
        let authority = Authority::new([3; 32], "relay.test");
        let token = own_token(&authority, 1_000);
        let session = authority.verify(&token, 1_000).unwrap();
        assert_eq!(session.list, PROBE_LIST);
        assert_eq!(session.node, PROBE_NODE);
        assert!(authority
            .verify(&token, 1_000 + PROBE_TOKEN_SECONDS * 1000 - 1)
            .is_ok());
        assert!(authority
            .verify(&token, 1_000 + PROBE_TOKEN_SECONDS * 1000)
            .is_err());
    }

    #[test]
    fn a_target_that_is_not_a_relays_address_is_refused_before_anything_is_dialled() {
        for base in ["https://relay.example", "relay.example", "ws://", "wss://"] {
            assert!(
                matches!(
                    Target::new(base, &Roots::Public, "t"),
                    Err(TargetError::Address(_))
                ),
                "{base}"
            );
        }
        assert!(matches!(
            Target::new(
                "wss://relay.example",
                &Roots::Only(vec![b"not a certificate".to_vec()]),
                "t"
            ),
            Err(TargetError::Unusable(_))
        ));
        assert!(matches!(
            Target::new("ws://relay.example", &Roots::Public, "line\nbreak"),
            Err(TargetError::Unusable(_))
        ));
        let target = Target::new("wss://[::1]:8443/", &Roots::Public, "t").unwrap();
        assert_eq!(target.url(), "wss://[::1]:8443/sce-relay/1");
        assert_eq!(target.host, "::1");
        assert_eq!(target.port, 8443);
        let plain = Target::new("ws://relay.example", &Roots::Public, "t").unwrap();
        assert_eq!(plain.port, 80);
        assert!(plain.server_name.is_none());
    }

    #[test]
    fn a_report_is_one_line_that_says_where_it_failed_and_never_the_token() {
        let report = Report {
            relay: "wss://relay.example/sce-relay/1".to_string(),
            outcome: Outcome::TlsRefused,
            stage: Stage::Tls,
            asked: Asked::WriteRead,
            created: None,
            timings: vec![
                (Stage::Resolve, Duration::from_micros(1_240)),
                (Stage::Connect, Duration::from_millis(3)),
            ],
            total: Duration::from_millis(9),
            reason: Some("invalid peer certificate: UnknownIssuer".to_string()),
        };
        assert_eq!(
            report.line(),
            "outcome=tls_refused relay=wss://relay.example/sce-relay/1 asked=write_read \
             stage=tls resolve_ms=1.2 connect_ms=3.0 total_ms=9.0 \
             reason=\"invalid peer certificate: UnknownIssuer\""
        );
        let ok = Report {
            outcome: Outcome::Ok,
            stage: Stage::Exchange,
            created: Some(false),
            reason: None,
            ..report
        };
        assert!(ok.line().starts_with(
            "outcome=ok relay=wss://relay.example/sce-relay/1 asked=write_read list=held \
             resolve_ms="
        ));
        assert!(!ok.line().contains("stage="));
    }
}
