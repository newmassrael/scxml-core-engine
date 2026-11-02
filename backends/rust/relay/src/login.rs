// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A member logs in by proving the key its list's log gave its node, and the relay issues its
//! token (ADR 0080 sections 1, 2 and 5).
//!
//! **The relay's side, the [`Doorway`].** A connection that presents no token the relay takes is
//! anonymous: it asks for a challenge for one node of one list, and logs in against it. A challenge
//! is stateless: its nonce is 16 bytes drawn from a key the relay process holds and a 16-byte tag,
//! HMAC-SHA256 under that key over the list, the node, the instant it expires at (the relay's clock
//! and 60 seconds) and the 16 bytes, so that a nonce made for one node or list is no use for
//! another and the relay keeps nothing per challenge. A login is taken when the tag is the relay's,
//! the instant has not passed, the nonce was not used, the latest grant of the node in the log the
//! relay holds gives it a role of at least 1 and a key, and the signature of the login
//! (`sce_rust_trust::login`) holds under that key for the relay's own audience. Entries brought with
//! the login are appended first, as `append` would, so that a device whose grant the relay was not
//! given brings it. The answer is an ordinary token of the relay's [`Authority`]. A used nonce is
//! kept until its instant, in memory and to a bound; a relay that starts again draws a new key, and
//! every nonce it gave before is void.
//!
//! The 16 bytes of a nonce are not drawn from the operating system at each challenge: they are
//! HMAC-SHA256 under the process's key, which is, of a counter, so that a relay draws once and a
//! test that fixes the key ([`Doorway::with_key`]) records the same nonces at every run. Whoever
//! does not hold the key cannot tell them from random bytes.
//!
//! **What an anonymous connection is held to.** Per source (an IPv4 address, an IPv6 /64, and one
//! source for a transport that names none): connections opened, challenges and logins, each so many a
//! minute; per node of a list, logins an hour, charged when the signature held, so that a stranger
//! spends no member's allowance. The tables forget a key a window after it last spent, and hold a
//! bounded number of keys ([`crate::limits`]). What a connection may ask besides is refused:
//! `create` and `join` (ADR 0080 sections 3 and 4) as requests this relay does not offer
//! (`invalid`), and every request of a session as `unauthorized`.
//!
//! **The device's side, the [`LoginMachine`].** It asks for a challenge for its person's node, stops
//! when the relay names itself by another audience than the one the device was configured with,
//! signs the login with that configured audience, brings the entry of its own latest grant, and ends
//! with the [`Token`]. It does no I/O, as a round of the courier does not.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use sce_rust_trust::{encoding, PublicKey, Trust};
use serde_json::Value;

use crate::client::{CourierError, LinkError};
use crate::driver::Exchange;
use crate::limits::Windows;
use crate::protocol::{Code, Login, Request, Response};
use crate::token::{equal, hmac_sha256, Authority};

/// How long a challenge holds, by the relay's clock.
pub const CHALLENGE_MS: u64 = 60_000;

const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 3_600_000;

/// The domains the relay's key is used under: the tag of a challenge, and the bytes of a nonce.
const TAG_DOMAIN: &str = "sce-relay/challenge-tag/1";
const DRAW_DOMAIN: &str = "sce-relay/challenge-draw/1";

/// What a relay that offers login holds an anonymous connection and a login to. The defaults are
/// the decision's: a token of an hour, and allowances a device that logs in once an hour never
/// meets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginPolicy {
    /// How long a token a login earned is good for, in seconds.
    pub token_seconds: u64,
    /// Connections with no token a source opens, each minute.
    pub connections_per_minute: u32,
    /// Challenges a source asks for, each minute.
    pub challenges_per_minute: u32,
    /// Logins a source makes, each minute.
    pub logins_per_minute: u32,
    /// Logins of one node of one list whose signature held, each hour.
    pub logins_per_node_per_hour: u32,
    /// The most nonces kept as used at once; past it, a login waits until the oldest expires.
    pub used_nonces: usize,
    /// The most sources, and nodes, each table of allowances holds at once.
    pub sources: usize,
}

impl Default for LoginPolicy {
    fn default() -> Self {
        Self {
            token_seconds: 3600,
            connections_per_minute: 30,
            challenges_per_minute: 30,
            logins_per_minute: 30,
            logins_per_node_per_hour: 60,
            used_nonces: 100_000,
            sources: 65_536,
        }
    }
}

/// Whether a connection that presents no token the relay takes is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    Taken,
    /// The relay offers no login: a connection needs a token.
    NotOffered,
    /// Its source opened as many as it may this minute.
    RateLimited {
        retry_after_ms: u64,
    },
}

/// The source an allowance is kept for: an IPv4 address whole, an IPv6 address by its /64 (one
/// network hands a device a /64), an IPv4 address carried as IPv6 as the IPv4 one, and none for a
/// transport that names no address, which is one source.
pub fn source_of(address: Option<IpAddr>) -> Option<IpAddr> {
    address.map(|address| match address {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let [a, b, c, d, ..] = v6.segments();
                IpAddr::V6(Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0))
            }
        },
    })
}

/// The address a connection from `peer` comes from, when `forwarded` are the values of its
/// `X-Forwarded-For` headers in order: `peer` itself unless it is one of the `trusted` proxies, and
/// then the hop nearest to the relay that is not a trusted proxy, read from the right, since each
/// proxy appends the address it saw. A hop that is not an address ends the reading at the last one
/// that was, so that no text a client writes moves its source past a proxy that is trusted.
pub fn source_behind(peer: IpAddr, forwarded: &[&str], trusted: &[IpAddr]) -> IpAddr {
    if !trusted.contains(&peer) {
        return peer;
    }
    let mut source = peer;
    for hop in forwarded
        .iter()
        .rev()
        .copied()
        .flat_map(|value| value.rsplit(','))
    {
        let hop = hop.trim();
        let Some(address) = hop
            .parse::<IpAddr>()
            .ok()
            .or_else(|| hop.parse::<SocketAddr>().ok().map(|at| at.ip()))
        else {
            return source;
        };
        source = address;
        if !trusted.contains(&address) {
            return address;
        }
    }
    source
}

/// Whether the request `text` is one a device makes to log in, or to join or create a list, which a
/// relay answers whether or not the connection presents a token.
pub fn is_anonymous(text: &str) -> bool {
    matches!(
        op_of(text).as_deref(),
        Some("challenge" | "login" | "create" | "join")
    )
}

fn op_of(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    Some(value.get("op")?.as_str()?.to_string())
}

/// What a relay that offers no login says to `request` on a connection with no token.
pub(crate) fn not_offered(request: &str) -> Response {
    if is_anonymous(request) {
        Response::refused(Code::Invalid, "this relay does not offer login")
    } else {
        Response::refused(Code::Unauthorized, "no token")
    }
}

/// What an anonymous connection asked, once the doorway let it through.
pub(crate) enum Asked {
    Challenge { list: String, node: u64 },
    Login(Checked),
}

/// A login whose challenge is the relay's, has not expired and was not used; what it signed is
/// checked by the list's side ([`Checked::signed_by`]).
pub(crate) struct Checked {
    login: Login,
    nonce: [u8; 32],
    signature: [u8; 64],
}

impl Checked {
    pub(crate) fn login(&self) -> &Login {
        &self.login
    }

    /// Whether `key` signed this login for the relay named `audience`.
    pub(crate) fn signed_by(&self, key: &PublicKey, audience: &str) -> bool {
        let bytes = sce_rust_trust::login::signing_bytes(
            audience,
            &self.login.list,
            self.login.node,
            &self.nonce,
            self.login.expires_at_ms,
        );
        key.verifies(&bytes, &self.signature)
    }
}

/// The relay's side of login: the key its challenges are made under, the nonces used, and the
/// allowances of those that log in. One for a relay, whatever the number of its slots.
pub struct Doorway {
    policy: LoginPolicy,
    key: [u8; 32],
    /// How many nonces were drawn.
    drawn: u64,
    used: BTreeSet<[u8; 32]>,
    /// The used nonces by the instant they expire at, so that the expired are forgotten in order.
    expiring: BTreeSet<(u64, [u8; 32])>,
    connections: Windows<Option<IpAddr>>,
    challenges: Windows<Option<IpAddr>>,
    logins: Windows<Option<IpAddr>>,
    nodes: Windows<(String, u64)>,
}

impl Doorway {
    /// A doorway under a key drawn from the operating system, the only source it is drawn from.
    pub fn new(policy: LoginPolicy) -> Result<Self, getrandom::Error> {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key)?;
        Ok(Self::with_key(policy, key))
    }

    /// A doorway under `key`: a test's, or a recording's, whose challenges are the same at every
    /// run. A relay that is deployed draws its key ([`Self::new`]).
    pub fn with_key(policy: LoginPolicy, key: [u8; 32]) -> Self {
        Self {
            policy,
            key,
            drawn: 0,
            used: BTreeSet::new(),
            expiring: BTreeSet::new(),
            connections: Windows::new(policy.sources),
            challenges: Windows::new(policy.sources),
            logins: Windows::new(policy.sources),
            nodes: Windows::new(policy.sources),
        }
    }

    pub fn policy(&self) -> &LoginPolicy {
        &self.policy
    }

    /// Whether a connection that presents no token the relay takes is taken from `source` at
    /// `now_ms`.
    pub(crate) fn open(&mut self, source: Option<IpAddr>, now_ms: u64) -> Opened {
        match self.connections.take(
            source_of(source),
            self.policy.connections_per_minute,
            MINUTE_MS,
            now_ms,
        ) {
            Ok(()) => Opened::Taken,
            Err(retry_after_ms) => Opened::RateLimited { retry_after_ms },
        }
    }

    /// What an anonymous request from `source` asks, when the doorway lets it through: its size,
    /// its form, its source's allowance, and for a login the challenge it names.
    pub(crate) fn ask(
        &mut self,
        source: Option<IpAddr>,
        text: &str,
        max_request_bytes: usize,
        now_ms: u64,
    ) -> Result<Asked, Response> {
        if text.len() > max_request_bytes {
            return Err(Response::refused(
                Code::TooLarge,
                format!("a request is at most {max_request_bytes} bytes"),
            ));
        }
        let request = Request::from_json(text).map_err(|reason| {
            let reason = match op_of(text).as_deref() {
                Some("create") => {
                    "this relay does not offer `create`: a list is created with a token of its \
                     authority"
                        .to_string()
                }
                Some("join") => "this relay does not offer `join`".to_string(),
                _ => reason,
            };
            Response::refused(Code::Invalid, reason)
        })?;
        let source = source_of(source);
        let limited = |retry_after_ms| {
            Response::refused(
                Code::RateLimited { retry_after_ms },
                "this source has spent its allowance",
            )
        };
        match request {
            Request::Challenge { list, node } => {
                self.challenges
                    .take(source, self.policy.challenges_per_minute, MINUTE_MS, now_ms)
                    .map_err(limited)?;
                if list.is_empty() {
                    return Err(Response::refused(Code::Invalid, "a list has a name"));
                }
                Ok(Asked::Challenge { list, node })
            }
            Request::Login(login) => {
                self.logins
                    .take(source, self.policy.logins_per_minute, MINUTE_MS, now_ms)
                    .map_err(limited)?;
                self.check(login, now_ms).map(Asked::Login)
            }
            _ => Err(Response::refused(
                Code::Unauthorized,
                "a connection with no token the relay takes asks for a challenge and logs in, \
                 and nothing else",
            )),
        }
    }

    /// The challenge for the device of `node` of `list` at the relay named `audience`, which holds
    /// the list when `held`.
    pub(crate) fn challenge(
        &mut self,
        list: &str,
        node: u64,
        held: bool,
        audience: &str,
        now_ms: u64,
    ) -> Response {
        let expires_at_ms = now_ms.saturating_add(CHALLENGE_MS);
        self.drawn += 1;
        let mut drawn = Vec::new();
        frame(&mut drawn, DRAW_DOMAIN.as_bytes());
        frame(&mut drawn, &self.drawn.to_be_bytes());
        let random = hmac_sha256(&self.key, &drawn);
        let mut nonce = random[..16].to_vec();
        nonce.extend_from_slice(&self.tag(list, node, expires_at_ms, &random[..16]));
        Response::Challenge {
            audience: audience.to_string(),
            nonce: encoding::encode(&nonce),
            expires_at_ms,
            held,
            work: None,
        }
    }

    /// The tag of a challenge for `node` of `list` that expires at `expires_at_ms`, over `random`.
    fn tag(&self, list: &str, node: u64, expires_at_ms: u64, random: &[u8]) -> [u8; 16] {
        let mut message = Vec::new();
        frame(&mut message, TAG_DOMAIN.as_bytes());
        frame(&mut message, list.as_bytes());
        frame(&mut message, node.to_string().as_bytes());
        frame(&mut message, expires_at_ms.to_string().as_bytes());
        frame(&mut message, random);
        let mac = hmac_sha256(&self.key, &message);
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&mac[..16]);
        tag
    }

    /// A login whose challenge this relay made for its list and node, that has not expired and was
    /// not used.
    fn check(&mut self, login: Login, now_ms: u64) -> Result<Checked, Response> {
        let unauthorized = |why: &str| Response::refused(Code::Unauthorized, why);
        let nonce: [u8; 32] = encoding::decode(&login.nonce)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| unauthorized("the nonce is not one this relay gives"))?;
        let tag = self.tag(&login.list, login.node, login.expires_at_ms, &nonce[..16]);
        if !equal(&tag, &nonce[16..]) {
            return Err(unauthorized(
                "the challenge is not one this relay made for this list and node",
            ));
        }
        if now_ms >= login.expires_at_ms {
            return Err(unauthorized("the challenge has expired"));
        }
        self.forget_used(now_ms);
        if self.used.contains(&nonce) {
            return Err(unauthorized("the challenge was used already"));
        }
        let signature: [u8; 64] = encoding::decode(&login.signature)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| unauthorized("the signature is not one"))?;
        Ok(Checked {
            login,
            nonce,
            signature,
        })
    }

    fn forget_used(&mut self, now_ms: u64) {
        while let Some(&(expires_at_ms, nonce)) = self.expiring.first() {
            if expires_at_ms > now_ms {
                break;
            }
            self.expiring.pop_first();
            self.used.remove(&nonce);
        }
    }

    /// The token of a login the list's side admitted: its nonce is used from now on, and the node
    /// spends one of its logins of the hour.
    pub(crate) fn redeem(
        &mut self,
        checked: &Checked,
        authority: &Authority,
        now_ms: u64,
    ) -> Response {
        let login = &checked.login;
        self.forget_used(now_ms);
        if self.used.contains(&checked.nonce) {
            return Response::refused(Code::Unauthorized, "the challenge was used already");
        }
        if self.used.len() >= self.policy.used_nonces {
            let retry_after_ms = self
                .expiring
                .first()
                .map_or(CHALLENGE_MS, |(expires_at_ms, _)| {
                    expires_at_ms.saturating_sub(now_ms)
                });
            return Response::refused(
                Code::RateLimited { retry_after_ms },
                "the relay holds as many used challenges as it keeps",
            );
        }
        if let Err(retry_after_ms) = self.nodes.take(
            (login.list.clone(), login.node),
            self.policy.logins_per_node_per_hour,
            HOUR_MS,
            now_ms,
        ) {
            return Response::refused(
                Code::RateLimited { retry_after_ms },
                "this node has logged in as often as it may this hour",
            );
        }
        self.used.insert(checked.nonce);
        self.expiring.insert((login.expires_at_ms, checked.nonce));
        let expires_at_ms = now_ms.saturating_add(self.policy.token_seconds.saturating_mul(1000));
        Response::Token {
            token: authority.issue(&login.list, login.node, expires_at_ms),
            expires_at_ms,
        }
    }
}

/// `bytes` framed as the trust layer frames a part: its length as eight bytes big-endian, then it.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}

/// The token a login earned, and when it expires by the relay's clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token: String,
    pub expires_at_ms: u64,
}

/// What a [`LoginMachine`] wants next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStep {
    /// Put this request to the relay, and give the machine the answer.
    Request(Request),
    /// The login is over: the token, or why there is none.
    Done(Result<Token, CourierError>),
}

enum Phase {
    Challenge,
    Login(Request),
    Over,
}

/// The device's side of a login, with no link: it asks for a challenge for its person's node, signs
/// the login against it with the audience the device was configured with, and ends with the token.
/// It reads no clock and opens nothing, as a round does not.
pub struct LoginMachine {
    trust: Trust,
    audience: String,
    phase: Phase,
    /// The request put to the relay and not answered yet.
    asked: Option<Request>,
    outcome: Option<Result<Token, CourierError>>,
}

impl LoginMachine {
    /// The login of `trust`'s person at the relay named `audience`: the name the device was
    /// configured with (shipped with the relay's address, or carried in an invitation), never the
    /// one a challenge says. A trust with no person logs in as no one and ends at once.
    pub fn start(trust: Trust, audience: &str) -> Self {
        let outcome = trust.person().is_none().then(|| {
            Err(CourierError::Unexpected(
                "the device has no person to log in as".to_string(),
            ))
        });
        Self {
            trust,
            audience: audience.to_string(),
            phase: Phase::Challenge,
            asked: None,
            outcome,
        }
    }

    /// The request to put to the relay next, or the login's end. Asked again before it is
    /// answered, it is the same request.
    pub fn poll(&mut self) -> LoginStep {
        if let Some(outcome) = &self.outcome {
            return LoginStep::Done(outcome.clone());
        }
        if let Some(request) = &self.asked {
            return LoginStep::Request(request.clone());
        }
        let request = match &self.phase {
            Phase::Challenge => Request::Challenge {
                list: self.trust.list(),
                node: self.trust.person().map_or(0, |(node, _)| node),
            },
            Phase::Login(request) => request.clone(),
            Phase::Over => {
                return LoginStep::Done(Err(CourierError::Unexpected(
                    "the login ended with no outcome".to_string(),
                )))
            }
        };
        self.asked = Some(request.clone());
        LoginStep::Request(request)
    }

    /// Whether a request was given and waits for its answer.
    pub fn is_asking(&self) -> bool {
        self.asked.is_some() && self.outcome.is_none()
    }

    /// The answer to the request [`Self::poll`] gave, or why the link did not carry it. An answer
    /// when no request waits for one is not taken.
    pub fn respond(&mut self, answer: Result<Response, LinkError>) {
        if self.outcome.is_some() || self.asked.take().is_none() {
            return;
        }
        let phase = std::mem::replace(&mut self.phase, Phase::Over);
        let outcome = match (phase, answer) {
            (_, Err(error)) => Err(error.into()),
            (_, Ok(Response::Refused { code, reason })) => {
                Err(CourierError::Relay { code, reason })
            }
            (
                Phase::Challenge,
                Ok(Response::Challenge {
                    audience,
                    nonce,
                    expires_at_ms,
                    ..
                }),
            ) => match self.login(&audience, &nonce, expires_at_ms) {
                Ok(request) => {
                    self.phase = Phase::Login(request);
                    return;
                }
                Err(error) => Err(error),
            },
            (
                Phase::Login(_),
                Ok(Response::Token {
                    token,
                    expires_at_ms,
                }),
            ) => Ok(Token {
                token,
                expires_at_ms,
            }),
            (_, Ok(other)) => Err(CourierError::Unexpected(format!("{other:?}"))),
        };
        self.outcome = Some(outcome);
    }

    /// The login request for a challenge the relay named `audience` gave.
    fn login(
        &self,
        audience: &str,
        nonce: &str,
        expires_at_ms: u64,
    ) -> Result<Request, CourierError> {
        if audience != self.audience {
            return Err(CourierError::Unexpected(format!(
                "the relay says it is `{audience}`, and this device logs in at `{}` alone",
                self.audience
            )));
        }
        let bytes = encoding::decode(nonce).ok_or_else(|| {
            CourierError::Unexpected("the challenge's nonce is not in the URL alphabet".to_string())
        })?;
        let (node, signature) = self
            .trust
            .sign_login(&self.audience, &bytes, expires_at_ms)
            .ok_or_else(|| {
                CourierError::Unexpected("the device has no person to log in as".to_string())
            })?;
        Ok(Request::Login(Login {
            list: self.trust.list(),
            node,
            nonce: nonce.to_string(),
            expires_at_ms,
            signature: encoding::encode(&signature),
            entries: self
                .trust
                .latest_grant(node)
                .map(|entry| vec![entry.to_json()])
                .unwrap_or_default(),
        }))
    }

    /// What the login came to, once it is over.
    pub fn outcome(&self) -> Option<&Result<Token, CourierError>> {
        self.outcome.as_ref()
    }
}

impl Exchange for LoginMachine {
    fn request(&mut self) -> Option<Request> {
        match self.poll() {
            LoginStep::Request(request) => Some(request),
            LoginStep::Done(_) => None,
        }
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) {
        self.respond(answer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn a_source_is_an_ipv4_address_or_an_ipv6_network_of_64_bits() {
        assert_eq!(source_of(None), None);
        assert_eq!(source_of(Some(at("192.0.2.7"))), Some(at("192.0.2.7")));
        assert_eq!(
            source_of(Some(at("2001:db8:1:2:3:4:5:6"))),
            Some(at("2001:db8:1:2::"))
        );
        assert_eq!(
            source_of(Some(at("::ffff:192.0.2.7"))),
            Some(at("192.0.2.7")),
            "an IPv4 address carried as IPv6"
        );
    }

    #[test]
    fn a_forwarded_address_is_believed_from_a_trusted_proxy_alone() {
        let proxy = at("10.0.0.1");
        let trusted = [proxy, at("10.0.0.2")];
        // A client that is no proxy says what it likes, and is its own address.
        assert_eq!(
            source_behind(at("198.51.100.9"), &["203.0.113.5"], &trusted),
            at("198.51.100.9")
        );
        // Behind the proxy: the hop it appended, read from the right.
        assert_eq!(
            source_behind(proxy, &["203.0.113.5, 198.51.100.9"], &trusted),
            at("198.51.100.9")
        );
        // Through two trusted proxies, and over two headers.
        assert_eq!(
            source_behind(proxy, &["198.51.100.9", "10.0.0.2"], &trusted),
            at("198.51.100.9")
        );
        assert_eq!(
            source_behind(proxy, &["198.51.100.9:443"], &trusted),
            at("198.51.100.9"),
            "an address with its port"
        );
        // A hop that is no address stops the reading at the proxy, whatever is left of it.
        assert_eq!(
            source_behind(proxy, &["203.0.113.5, nonsense"], &trusted),
            proxy
        );
        assert_eq!(source_behind(proxy, &[], &trusted), proxy);
    }

    #[test]
    fn a_request_to_log_in_is_told_from_a_sessions_by_its_name() {
        for anonymous in ["challenge", "login", "create", "join"] {
            assert!(is_anonymous(&format!(r#"{{"v":1,"op":"{anonymous}"}}"#)));
        }
        for session in [r#"{"v":1,"op":"heads"}"#, "not json", "{}"] {
            assert!(!is_anonymous(session), "{session}");
        }
    }

    #[test]
    fn a_challenge_is_the_same_for_the_same_key_and_another_for_another() {
        let made = |key: [u8; 32]| {
            let mut doorway = Doorway::with_key(LoginPolicy::default(), key);
            (
                doorway.challenge("home", 2, true, "relay", 0),
                doorway.challenge("home", 2, true, "relay", 0),
            )
        };
        let (first, second) = made([1; 32]);
        assert_eq!(made([1; 32]), (first.clone(), second.clone()));
        assert_ne!(first, second, "a nonce is drawn afresh for each challenge");
        assert_ne!(made([2; 32]).0, first);
        let Response::Challenge {
            nonce,
            expires_at_ms,
            ..
        } = first
        else {
            panic!("a challenge")
        };
        assert_eq!(encoding::decode(&nonce).map(|bytes| bytes.len()), Some(32));
        assert_eq!(expires_at_ms, CHALLENGE_MS);
    }
}
