// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The half of the HTTPS binding's browser shell that runs in the page's WebAssembly module
//! (docs/adr/0117, D2). The other half is the page's script (`backends/rust/browser-node/js`),
//! which carries an exchange with `fetch`, signs with a key of the Web Cryptography API, keeps
//! what the node accepted in IndexedDB, waits with the page's timers and holds the node's Web
//! Lock. Neither half decides anything of the binding: every decision is the client core's, the
//! one the native shell of the module `client` steps, so the browser is held to the same rules
//! by being the same code.
//!
//! [`BrowserClient`] holds the core. The script gives it what happened through one method per
//! input and carries out the effects each answers, a JSON array of objects, in their order:
//!
//! * `{"count": "<label>"}`: an exchange, by the status its server answered (docs/adr/0109,
//!   D16), as the label of `ExchangeOutcome`.
//! * `{"tell": "ready" | "restarted", "peer"}`, `{"tell": "lost" | "interrupted" |
//!   "unauthenticated", "peer", "words"}`, `{"tell": "arrived" | "unsent", "peer", "envelope"}`
//!   and `{"tell": "carried", "peer", "envelopes"}`: a [`LinkEvent`], each envelope in
//!   standard base64.
//! * `{"ask": "draw", "lane", "bytes"}`: draw that many random bytes and read the moment, then
//!   [`BrowserClient::drawn`].
//! * `{"ask": "sign", "lane", "base"}`: sign the base (standard base64 of its bytes) with the
//!   node's Ed25519 key, then [`BrowserClient::signed`] with the 64 bytes of the signature. The
//!   base is RFC 9421's, made here by the profile's own code (`signature::RequestKey`), so the
//!   key never leaves the platform and nothing of the profile is written in the script.
//! * `{"ask": "carry", "lane", "url", "headers", "body", "timeout_ms"}`: post `body` (standard
//!   base64) to `url` with `headers` (pairs of a name and a value), then
//!   [`BrowserClient::answered`] with what the server answered, or [`BrowserClient::failed`]
//!   when no response came within `timeout_ms`.
//! * `{"ask": "wake", "lane", "ms"}` or `{"ask": "wake", "lane", "until_ms"}`: wait that long, or
//!   until that moment since the Unix epoch, then [`BrowserClient::woken`].
//! * `{"ask": "stop", "lane"}`: the lane is over; a wait it has is not waited out.
//!
//! What the script owes the core is what the native shell owes it (`client_core`): one input at a
//! time, each lane's tasks one at a time in the order asked, and the input each names.
//!
//! A browser node's binding is `receipt: durable`: what the page accepted is kept in its store
//! until its server's `200` says a durable checkpoint holds it (`{"tell": "carried"}`,
//! docs/adr/0109, D10), which is the promise of docs/adr/0117, D6. Its credential is the `key`
//! authenticator's (D3), and its server is of the origin that served the page (D9); settings of
//! any other kind are refused when the client is made.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::time::Duration;

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use sce_rust_crypto::cose::TrustedKey;
use sce_rust_crypto::ed25519::{PUBLIC_KEY_BYTES, SIGNATURE_BYTES};
use sce_rust_crypto::random::RandomSource;

use super::client::{base_is_allowed, ClientSettings};
use super::client_core::{
    ClientCore, CoreSettings, Drawn, Effect, Input, Lane, Received, Task, Wait,
};
use super::signature::{
    draw_nonce, normalize_target_uri, RequestKey, UnsignedRequest, NONCE_BYTES,
};
use super::{is_path_segment, Credential, HttpsFailure, HttpsResponse, LinkEvent, PATH_PREFIX};
use crate::reconnect::Backoff;

/// Why a browser client was not made, or an input was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    /// The settings are not JSON of the shape [`BrowserClient::start`] reads; the words say what.
    Settings(String),
    /// The machine name is not a path segment as it stands (SCE_MESH.md section 22.1).
    NotAPathSegment(String),
    /// The base is not an `https://` URL, or an `http://` one for a loopback host, or carries a
    /// query or a fragment.
    BadBase(String),
    /// The base is not of the origin that served the page (docs/adr/0117, D9).
    OtherOrigin { base: String, origin: String },
    /// The binding is not `receipt: durable`, so what the page accepted could not be kept until
    /// its server holds it (docs/adr/0117, D6).
    NotDurable,
    /// The seed of the spread of a backoff is not 32 bytes.
    Seed,
    /// An input names no lane, or is not of the shape its method reads; the words say what.
    Input(String),
}

impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserError::Settings(words) => write!(f, "the settings are refused: {words}"),
            BrowserError::NotAPathSegment(own) => {
                write!(f, "the machine name {own:?} is not a path segment")
            }
            BrowserError::BadBase(base) => write!(
                f,
                "the base {base:?} is not https://, or http:// of a loopback host, without query or fragment"
            ),
            BrowserError::OtherOrigin { base, origin } => write!(
                f,
                "the base {base:?} is not of the origin {origin:?} that served the page"
            ),
            BrowserError::NotDurable => f.write_str(
                "a browser node keeps what it accepted until its server holds it: its binding is receipt: durable",
            ),
            BrowserError::Seed => f.write_str("the seed is not 32 bytes"),
            BrowserError::Input(words) => write!(f, "the input is refused: {words}"),
        }
    }
}

/// The client core of a browser node, and what of a lane's request waits for the page's key.
pub struct BrowserClient {
    core: ClientCore,
    key: RequestKey,
    /// The request each lane asked the page to sign, by `lane_index`, until it is signed.
    signing: [Option<UnsignedRequest>; 2],
    /// What the spread of a backoff is drawn from: the seed the page drew, and how many draws
    /// were made of it.
    seed: [u8; 32],
    draws: u64,
}

/// Random bytes the page drew, handed to the profile's nonce as a source that gives exactly them.
struct Given<'a>(&'a [u8]);

impl RandomSource for Given<'_> {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        if self.0.len() != out.len() {
            return false;
        }
        out.copy_from_slice(self.0);
        true
    }
}

impl BrowserClient {
    /// A client with `settings`, a JSON object:
    ///
    /// * `origin`: the origin that served the page (`location.origin`), which `base` must be of;
    /// * `base`, `own`, `server`: the server's base, this machine's name and the server
    ///   machine's, as [`ClientSettings`] has them;
    /// * `public_key`: the node's Ed25519 public key, 32 bytes in base64url without padding;
    /// * `durable`: `true`, the binding's `receipt: durable`;
    /// * optionally `hold_ms`, `exchange_timeout_ms`, `max_body_bytes`, `backoff` (an object of
    ///   `initial_ms`, `max_ms`, `factor` and `jitter`) and `verify` (an array of objects of
    ///   `id` and `public_key`, the keys of the trust set `verify:` names), each as
    ///   [`ClientSettings::new`] defaults it when absent.
    ///
    /// `seed` is 32 random bytes the page drew, which the spread of each backoff is drawn from.
    /// The client asks nothing until [`Self::started`].
    pub fn start(settings: &str, seed: &[u8]) -> Result<BrowserClient, BrowserError> {
        let seed: [u8; 32] = seed.try_into().map_err(|_| BrowserError::Seed)?;
        let (core, key) = read_settings(settings)?;
        Ok(BrowserClient {
            core: ClientCore::new(core),
            key,
            signing: [None, None],
            seed,
            draws: 0,
        })
    }

    /// The `keyid` the node's requests are signed under: the thumbprint of its public key.
    pub fn key_id(&self) -> String {
        self.key.key_id().to_string()
    }

    /// The page carries out what it is asked from now on: the first poll is made at once.
    pub fn started(&mut self) -> String {
        self.run(Input::Started)
    }

    /// The node gives the server `envelope`, which its store holds already.
    pub fn transmitted(&mut self, envelope: &[u8]) -> String {
        self.run(Input::Transmitted(envelope.to_vec()))
    }

    /// The page drew `random` (empty when its source gave nothing) and read `moment_ms`, in
    /// milliseconds since the Unix epoch, for `lane`'s request.
    pub fn drawn(
        &mut self,
        lane: &str,
        random: &[u8],
        moment_ms: f64,
    ) -> Result<String, BrowserError> {
        let lane = lane_of(lane)?;
        let moment = if moment_ms.is_finite() && moment_ms > 0.0 {
            Duration::from_millis(moment_ms as u64)
        } else {
            Duration::ZERO
        };
        let drawn = draw_nonce(&mut Given(random)).map(|nonce| Drawn { nonce, moment });
        Ok(self.run(Input::Drawn(lane, drawn)))
    }

    /// The node's key signed the base `lane`'s request asked for: `signature` is its 64 bytes,
    /// or empty when the key could not sign.
    pub fn signed(&mut self, lane: &str, signature: &[u8]) -> Result<String, BrowserError> {
        let lane = lane_of(lane)?;
        let fields = self.signing[lane_index(lane)]
            .take()
            .filter(|_| signature.len() == SIGNATURE_BYTES)
            .map(|unsigned| Vec::from(unsigned.fields(signature)));
        Ok(self.run(Input::Signed(lane, fields)))
    }

    /// `lane`'s exchange was answered `status`, with `headers` (a JSON array of pairs of a name
    /// and a value) and `body`.
    pub fn answered(
        &mut self,
        lane: &str,
        status: u16,
        headers: &str,
        body: &[u8],
    ) -> Result<String, BrowserError> {
        let lane = lane_of(lane)?;
        let headers = read_pairs(headers)?;
        let response = HttpsResponse {
            status,
            headers,
            body: body.to_vec(),
        };
        Ok(self.run(Input::Answered(lane, Ok(response))))
    }

    /// `lane`'s exchange got no response: `kind` is `timeout` when it took longer than its
    /// `timeout_ms`, and anything else is a failure the platform said no more of than `words`
    /// (`fetch` tells a refused connection, a failed TLS handshake and a proxy's refusal apart
    /// from none of the others).
    pub fn failed(&mut self, lane: &str, kind: &str, words: &str) -> Result<String, BrowserError> {
        let lane = lane_of(lane)?;
        let failure = match kind {
            "timeout" => HttpsFailure::Timeout,
            _ => HttpsFailure::Other(words.to_string()),
        };
        Ok(self.run(Input::Answered(lane, Err(failure))))
    }

    /// The moment `lane` waited for has come.
    pub fn woken(&mut self, lane: &str) -> Result<String, BrowserError> {
        let lane = lane_of(lane)?;
        Ok(self.run(Input::Woken(lane)))
    }

    /// Where the node stands in its server's mailbox, as JSON: `{"epoch", "number"}`, the
    /// incarnation (`null` before the first `200`) and the highest number handed to the page.
    pub fn received(&self) -> String {
        let Received { epoch, number } = self.core.received();
        json!({ "epoch": epoch, "number": number }).to_string()
    }

    /// The page's store holds every envelope up to `received`, read with [`Self::received`]
    /// before the page kept what it was handed (docs/adr/0109, D10).
    pub fn held(&mut self, received: &str) -> Result<String, BrowserError> {
        let value: Value = serde_json::from_str(received)
            .map_err(|error| BrowserError::Input(error.to_string()))?;
        let epoch = match value.get("epoch") {
            None | Some(Value::Null) => None,
            Some(Value::String(epoch)) => Some(epoch.clone()),
            Some(_) => return Err(BrowserError::Input("`epoch` is not a text".to_string())),
        };
        let number = value
            .get("number")
            .and_then(Value::as_u64)
            .ok_or_else(|| BrowserError::Input("`number` is not a count".to_string()))?;
        Ok(self.run(Input::Held(Received { epoch, number })))
    }

    /// The node stops: no exchange is begun after this, and the server is lost.
    pub fn stop(&mut self) -> String {
        self.run(Input::Stop)
    }

    /// Whether the node stopped.
    pub fn stopped(&self) -> bool {
        self.core.stopped()
    }

    /// Give the core `input` and every input its effects make here at once, and answer the
    /// effects the page carries out, in their order, as JSON.
    fn run(&mut self, input: Input) -> String {
        let mut effects: Vec<Value> = Vec::new();
        let mut inputs = VecDeque::from([input]);
        while let Some(input) = inputs.pop_front() {
            for effect in self.core.step(input) {
                if let Some(next) = self.effect(effect, &mut effects) {
                    inputs.push_back(next);
                }
            }
        }
        Value::Array(effects).to_string()
    }

    /// Write `effect` for the page, or answer the input it makes at once: a request whose
    /// signature cannot be prepared, since a field it covers is missing, is not signed.
    fn effect(&mut self, effect: Effect, effects: &mut Vec<Value>) -> Option<Input> {
        match effect {
            Effect::Count(outcome) => effects.push(json!({ "count": outcome.value() })),
            Effect::Tell(event) => effects.push(told(event)),
            Effect::Ask(lane, Task::Draw) => effects.push(json!({
                "ask": "draw",
                "lane": lane_name(lane),
                "bytes": NONCE_BYTES,
            })),
            Effect::Ask(
                lane,
                Task::Sign {
                    target_uri,
                    headers,
                    created,
                },
            ) => match self.key.prepare(&target_uri, &headers, created) {
                Some(unsigned) => {
                    effects.push(json!({
                        "ask": "sign",
                        "lane": lane_name(lane),
                        "base": STANDARD.encode(unsigned.base()),
                    }));
                    self.signing[lane_index(lane)] = Some(unsigned);
                }
                None => return Some(Input::Signed(lane, None)),
            },
            Effect::Ask(lane, Task::Carry(request)) => {
                let headers: Vec<Value> = request
                    .headers
                    .iter()
                    .map(|(name, value)| json!([name, value]))
                    .collect();
                effects.push(json!({
                    "ask": "carry",
                    "lane": lane_name(lane),
                    "url": request.url,
                    "headers": headers,
                    "body": STANDARD.encode(&request.body),
                    "timeout_ms": millis(request.timeout),
                }));
            }
            Effect::Ask(lane, Task::Wake(wait)) => {
                let mut asked = json!({ "ask": "wake", "lane": lane_name(lane) });
                match wait {
                    Wait::For(delay) => asked["ms"] = json!(millis(delay)),
                    Wait::Spread(delay) => {
                        let draw = self.draw();
                        asked["ms"] = json!(millis(Backoff::spread(delay, draw)));
                    }
                    Wait::Until(moment) => asked["until_ms"] = json!(millis(moment)),
                }
                effects.push(asked);
            }
            Effect::Ask(lane, Task::Stop) => {
                self.signing[lane_index(lane)] = None;
                effects.push(json!({ "ask": "stop", "lane": lane_name(lane) }));
            }
        }
        None
    }

    /// The next number the spread of a backoff is drawn from: the first eight bytes of the
    /// SHA-256 of the seed and the count of draws before it.
    fn draw(&mut self) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(self.seed);
        hasher.update(self.draws.to_be_bytes());
        self.draws = self.draws.wrapping_add(1);
        let digest = hasher.finalize();
        let mut word = [0u8; 8];
        word.copy_from_slice(&digest[..8]);
        u64::from_be_bytes(word)
    }
}

/// The core's settings and the node's public key, read from `text` as [`BrowserClient::start`]
/// says.
fn read_settings(text: &str) -> Result<(CoreSettings, RequestKey), BrowserError> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| BrowserError::Settings(error.to_string()))?;
    let text_of = |name: &str| -> Result<String, BrowserError> {
        value
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| BrowserError::Settings(alloc::format!("`{name}` is not a text")))
    };
    let count_of = |name: &str| -> Result<Option<u64>, BrowserError> {
        match value.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(count) => count
                .as_u64()
                .map(Some)
                .ok_or_else(|| BrowserError::Settings(alloc::format!("`{name}` is not a count"))),
        }
    };
    let origin = text_of("origin")?;
    let base = text_of("base")?;
    let own = text_of("own")?;
    let server = text_of("server")?;
    let public_key = key_of(&text_of("public_key")?).ok_or_else(|| {
        BrowserError::Settings("`public_key` is not 32 bytes in base64url".to_string())
    })?;
    if value.get("durable").and_then(Value::as_bool) != Some(true) {
        return Err(BrowserError::NotDurable);
    }
    // SCE_MESH.md section 22.1: every exchange posts to `<base>/sce-mesh/1/<client>`, the one
    // rule both shells hold a base and a name to.
    if !is_path_segment(&own) {
        return Err(BrowserError::NotAPathSegment(own));
    }
    let trimmed = base.trim_end_matches('/');
    if !base_is_allowed(trimmed) {
        return Err(BrowserError::BadBase(base));
    }
    // docs/adr/0117, D9: the binding in a browser is to the origin that served the page.
    let of_origin = match (
        normalize_target_uri(&alloc::format!("{trimmed}/")),
        normalize_target_uri(&alloc::format!("{}/", origin.trim_end_matches('/'))),
    ) {
        (Some(base_uri), Some(origin_uri)) => base_uri.starts_with(&origin_uri),
        _ => false,
    };
    if !of_origin {
        return Err(BrowserError::OtherOrigin { base, origin });
    }
    let url = alloc::format!("{trimmed}{PATH_PREFIX}{own}");
    let target_uri =
        normalize_target_uri(&url).ok_or_else(|| BrowserError::BadBase(base.clone()))?;
    let defaults = ClientSettings::new(trimmed, own, server, Credential::Mtls);
    let backoff = match value.get("backoff") {
        None | Some(Value::Null) => defaults.backoff,
        Some(backoff) => read_backoff(backoff, defaults.backoff)?,
    };
    let verify = match value.get("verify") {
        None | Some(Value::Null) => None,
        Some(keys) => Some(read_trust(keys)?),
    };
    let settings = CoreSettings {
        server: defaults.server,
        url,
        target_uri,
        authorization: None,
        signs: true,
        verify,
        hold: count_of("hold_ms")?.map_or(defaults.hold, Duration::from_millis),
        exchange_timeout: count_of("exchange_timeout_ms")?
            .map_or(defaults.exchange_timeout, Duration::from_millis),
        max_body_bytes: match count_of("max_body_bytes")? {
            Some(bytes) => usize::try_from(bytes).unwrap_or(usize::MAX),
            None => defaults.max_body_bytes,
        },
        backoff,
        durable: true,
    };
    Ok((settings, RequestKey::new(&public_key)))
}

/// A backoff object, each field absent taken from `defaults`.
fn read_backoff(value: &Value, defaults: Backoff) -> Result<Backoff, BrowserError> {
    let millis_of = |name: &str, default: Duration| -> Result<Duration, BrowserError> {
        match value.get(name) {
            None | Some(Value::Null) => Ok(default),
            Some(count) => count.as_u64().map(Duration::from_millis).ok_or_else(|| {
                BrowserError::Settings(alloc::format!("`backoff.{name}` is not a count"))
            }),
        }
    };
    let factor = match value.get("factor") {
        None | Some(Value::Null) => defaults.factor,
        Some(factor) => factor
            .as_u64()
            .and_then(|factor| u32::try_from(factor).ok())
            .ok_or_else(|| BrowserError::Settings("`backoff.factor` is not a count".to_string()))?,
    };
    let jitter = match value.get("jitter") {
        None | Some(Value::Null) => defaults.jitter,
        Some(jitter) => jitter.as_bool().ok_or_else(|| {
            BrowserError::Settings("`backoff.jitter` is not true or false".to_string())
        })?,
    };
    Ok(Backoff {
        initial: millis_of("initial_ms", defaults.initial)?,
        max: millis_of("max_ms", defaults.max)?,
        factor,
        jitter,
    })
}

/// The keys of a trust set, an array of objects of `id` and `public_key`.
fn read_trust(value: &Value) -> Result<Vec<TrustedKey>, BrowserError> {
    let keys = value
        .as_array()
        .ok_or_else(|| BrowserError::Settings("`verify` is not an array".to_string()))?;
    keys.iter()
        .map(|key| {
            let id = key.get("id").and_then(Value::as_str);
            let public_key = key
                .get("public_key")
                .and_then(Value::as_str)
                .and_then(key_of);
            match (id, public_key) {
                (Some(id), Some(public_key)) => Ok(TrustedKey {
                    id: id.to_string(),
                    public_key,
                }),
                _ => Err(BrowserError::Settings(
                    "a key of `verify` is not an `id` and a `public_key` of 32 bytes".to_string(),
                )),
            }
        })
        .collect()
}

/// An Ed25519 public key in base64url without padding.
fn key_of(text: &str) -> Option<[u8; PUBLIC_KEY_BYTES]> {
    URL_SAFE_NO_PAD.decode(text.trim()).ok()?.try_into().ok()
}

/// Pairs of a name and a value, from a JSON array of two-element arrays of text.
fn read_pairs(text: &str) -> Result<Vec<(String, String)>, BrowserError> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| BrowserError::Input(error.to_string()))?;
    let pairs = value
        .as_array()
        .ok_or_else(|| BrowserError::Input("the headers are not an array".to_string()))?;
    pairs
        .iter()
        .map(|pair| match pair.as_array().map(Vec::as_slice) {
            Some([Value::String(name), Value::String(field)]) => Ok((name.clone(), field.clone())),
            _ => Err(BrowserError::Input(
                "a header is not a pair of a name and a value".to_string(),
            )),
        })
        .collect()
}

fn lane_of(name: &str) -> Result<Lane, BrowserError> {
    match name {
        "poll" => Ok(Lane::Poll),
        "send" => Ok(Lane::Send),
        other => Err(BrowserError::Input(alloc::format!(
            "{other:?} is not a lane"
        ))),
    }
}

fn lane_name(lane: Lane) -> &'static str {
    match lane {
        Lane::Poll => "poll",
        Lane::Send => "send",
    }
}

fn lane_index(lane: Lane) -> usize {
    match lane {
        Lane::Poll => 0,
        Lane::Send => 1,
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// A [`LinkEvent`] as the page is told it.
fn told(event: LinkEvent) -> Value {
    match event {
        LinkEvent::Ready(peer) => json!({ "tell": "ready", "peer": peer }),
        LinkEvent::Received(peer, envelope) => {
            json!({ "tell": "arrived", "peer": peer, "envelope": STANDARD.encode(envelope) })
        }
        LinkEvent::Unsent(peer, envelope) => {
            json!({ "tell": "unsent", "peer": peer, "envelope": STANDARD.encode(envelope) })
        }
        LinkEvent::Lost(peer, words) => json!({ "tell": "lost", "peer": peer, "words": words }),
        LinkEvent::Interrupted(peer, words) => {
            json!({ "tell": "interrupted", "peer": peer, "words": words })
        }
        LinkEvent::Restarted(peer) => json!({ "tell": "restarted", "peer": peer }),
        LinkEvent::Unauthenticated(peer, words) => {
            json!({ "tell": "unauthenticated", "peer": peer, "words": words })
        }
        LinkEvent::Carried(peer, envelopes) => {
            let envelopes: Vec<String> = envelopes
                .iter()
                .map(|envelope| STANDARD.encode(envelope))
                .collect();
            json!({ "tell": "carried", "peer": peer, "envelopes": envelopes })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::signature::{field_value, RequestSignature, RequestSigner};
    use super::*;
    use sce_rust_crypto::ed25519::KeyPair;

    const ORIGIN: &str = "http://127.0.0.1:8080";
    const TARGET: &str = "http://127.0.0.1:8080/sce-mesh/1/phone";
    const EPOCH: &str = "0123456789abcdef0123456789abcdef";
    /// 2027-01-15, in seconds and in milliseconds since the Unix epoch.
    const NOW_S: i64 = 1_800_000_000;
    const NOW_MS: f64 = 1_800_000_000_000.0;

    fn key() -> KeyPair {
        KeyPair::from_seed(&[7u8; 32])
    }

    /// The settings of `phone`, served by `list` at the page's own origin, with `changes`
    /// written over them.
    fn settings_with(changes: Value) -> String {
        let mut settings = json!({
            "origin": ORIGIN,
            "base": ORIGIN,
            "own": "phone",
            "server": "list",
            "public_key": URL_SAFE_NO_PAD.encode(key().public_key()),
            "durable": true,
            "backoff": { "initial_ms": 1000, "max_ms": 8000, "factor": 2, "jitter": true },
        });
        if let (Some(settings), Some(changes)) = (settings.as_object_mut(), changes.as_object()) {
            for (name, value) in changes {
                settings.insert(name.clone(), value.clone());
            }
        }
        settings.to_string()
    }

    fn started(seed: u8) -> BrowserClient {
        BrowserClient::start(&settings_with(json!({})), &[seed; 32])
            .expect("the settings are taken")
    }

    fn effects(text: String) -> Value {
        serde_json::from_str(&text).expect("the effects are JSON")
    }

    /// The pairs of a `carry` effect's headers.
    fn headers_of(carry: &Value) -> Vec<(String, String)> {
        carry["headers"]
            .as_array()
            .expect("the headers are an array")
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().expect("a name").to_string(),
                    pair[1].as_str().expect("a value").to_string(),
                )
            })
            .collect()
    }

    /// Draw and sign `lane`'s request as the page does, with the node's key, and answer the one
    /// effect it then asks: the exchange to carry.
    fn signed_request(client: &mut BrowserClient, lane: &str) -> Value {
        let drawn = effects(client.drawn(lane, &[3u8; 16], NOW_MS).expect("a lane"));
        assert_eq!(drawn.as_array().map(Vec::len), Some(1), "{drawn}");
        assert_eq!(drawn[0]["ask"], "sign", "{drawn}");
        let base = STANDARD
            .decode(drawn[0]["base"].as_str().expect("a base"))
            .expect("base64");
        let carried = effects(client.signed(lane, &key().sign(&base)).expect("a lane"));
        assert_eq!(carried.as_array().map(Vec::len), Some(1), "{carried}");
        assert_eq!(carried[0]["ask"], "carry", "{carried}");
        carried[0].clone()
    }

    /// The page's key signs the base the module prepares, and the request it then carries is
    /// signed exactly as the native signer signs it, and verifies as the server verifies one.
    /// Red if `RequestKey::prepare` and `RequestSigner::sign` part, or the module carries the
    /// request before it is signed.
    #[test]
    fn the_page_signs_what_the_native_signer_signs_and_the_server_takes_it() {
        let mut client = started(1);
        assert_eq!(
            effects(client.started()),
            json!([{ "ask": "wake", "lane": "poll", "ms": 0 }])
        );
        assert_eq!(
            effects(client.woken("poll").expect("a lane")),
            json!([{ "ask": "draw", "lane": "poll", "bytes": NONCE_BYTES }])
        );
        let carry = signed_request(&mut client, "poll");
        assert_eq!(carry["url"], TARGET);
        let headers = headers_of(&carry);
        let (unsigned, fields) = headers.split_at(headers.len() - 3);
        let native = RequestSigner::new(key())
            .sign(TARGET, unsigned, NOW_S)
            .expect("every field the signature covers");
        assert_eq!(fields, native.as_slice());
        let field = |name: &str| field_value(&headers, name);
        let read = RequestSignature::read(&field).expect("a signature of the profile");
        assert_eq!(read.key_id(), client.key_id());
        assert_eq!(
            read.verify(&key().public_key(), TARGET, &field, NOW_S, 300),
            Ok(())
        );
    }

    /// A failed exchange waits its backoff spread by a number drawn from the page's seed, the
    /// same for the same seed; a `200` makes the server ready and hands the page what it carried;
    /// and the next request acknowledges only what the page reported its store holds. Red if the
    /// module waits the backoff unspread, or `held` reaches no core.
    #[test]
    fn a_backoff_is_spread_from_the_seed_and_a_request_acknowledges_what_is_held() {
        let failed_wait = |seed: u8| -> u64 {
            let mut client = started(seed);
            client.started();
            client.woken("poll").expect("a lane");
            signed_request(&mut client, "poll");
            let after = effects(
                client
                    .failed("poll", "other", "fetch failed")
                    .expect("a lane"),
            );
            assert_eq!(after[0], json!({ "count": "none" }), "{after}");
            assert_eq!(after[1]["ask"], "wake", "{after}");
            after[1]["ms"].as_u64().expect("a wait in milliseconds")
        };
        // The first draw of a seed of 32 bytes `s` is the first eight bytes of SHA-256(s, 0), read
        // as Backoff::spread reads a draw: 1265 and 1367 ten-thousandths above the base here.
        assert_eq!(
            failed_wait(1),
            1126,
            "1000 spread by the first draw of the seed"
        );
        assert_eq!(failed_wait(1), 1126, "the same seed draws the same spread");
        assert_eq!(failed_wait(2), 1136, "another seed draws another spread");

        let mut client = started(1);
        client.started();
        client.woken("poll").expect("a lane");
        signed_request(&mut client, "poll");
        let headers = json!([["Sce-Mesh-Epoch", EPOCH], ["Sce-Mesh-First", "1"]]).to_string();
        assert_eq!(
            effects(
                client
                    .answered("poll", 200, &headers, &[0x01])
                    .expect("a lane")
            ),
            json!([
                { "count": "200" },
                { "tell": "ready", "peer": "list" },
                { "tell": "arrived", "peer": "list", "envelope": "AQ==" },
                { "ask": "draw", "lane": "poll", "bytes": NONCE_BYTES },
            ])
        );
        let received = client.received();
        assert_eq!(
            effects(received.clone()),
            json!({ "epoch": EPOCH, "number": 1 })
        );
        let ack = |carry: &Value| field_value(&headers_of(carry), "Sce-Mesh-Ack");
        assert_eq!(
            ack(&signed_request(&mut client, "poll")),
            Some("0".to_string()),
            "a durable binding acknowledges nothing its page has not reported held"
        );
        // A request's acknowledgement is read when the lane begins it, which is when the
        // exchange before it is answered (docs/adr/0109, D10): the page reports what it holds
        // first, and the request begun after carries it.
        assert_eq!(effects(client.held(&received).expect("a mark")), json!([]));
        client
            .answered("poll", 200, &headers, &[0x01])
            .expect("a lane");
        assert_eq!(
            ack(&signed_request(&mut client, "poll")),
            Some("1".to_string())
        );
    }

    /// A stop loses the server and ends each lane that waits; the client takes it as stopped.
    #[test]
    fn a_stop_loses_the_server_and_ends_each_waiting_lane() {
        let mut client = started(1);
        client.started();
        assert!(!client.stopped());
        assert_eq!(
            effects(client.stop()),
            json!([
                { "tell": "lost", "peer": "list", "words": "the HTTPS client stopped" },
                { "ask": "stop", "lane": "poll" },
                { "ask": "stop", "lane": "send" },
            ])
        );
        assert!(client.stopped());
    }

    /// Settings of another kind than a browser node's are refused when the client is made: a
    /// server of another origin (D9), a binding that is not durable (D6), a name or a base the
    /// binding cannot post under, and a seed or a key of the wrong length. Red if `read_settings`
    /// takes one of them.
    #[test]
    fn settings_of_another_kind_than_a_browser_nodes_are_refused() {
        let refused =
            |changes: Value| BrowserClient::start(&settings_with(changes), &[1u8; 32]).err();
        assert_eq!(refused(json!({})), None);
        assert_eq!(
            refused(json!({ "base": "http://127.0.0.1:8080/api/" })),
            None
        );
        assert_eq!(
            refused(
                json!({ "origin": "https://cal.example", "base": "https://Cal.Example:443/api" })
            ),
            None,
            "an origin is compared as the binding normalizes a target"
        );
        assert_eq!(
            refused(json!({ "base": "http://127.0.0.1:8081" })),
            Some(BrowserError::OtherOrigin {
                base: "http://127.0.0.1:8081".to_string(),
                origin: ORIGIN.to_string(),
            })
        );
        assert!(matches!(
            refused(
                json!({ "origin": "https://cal.example", "base": "https://cal.example.org/api" })
            ),
            Some(BrowserError::OtherOrigin { .. })
        ));
        assert_eq!(
            refused(json!({ "durable": false })),
            Some(BrowserError::NotDurable)
        );
        assert_eq!(
            refused(json!({ "durable": null })),
            Some(BrowserError::NotDurable)
        );
        assert_eq!(
            refused(json!({ "own": "a b" })),
            Some(BrowserError::NotAPathSegment("a b".to_string()))
        );
        assert_eq!(
            refused(json!({ "origin": "http://example.org", "base": "http://example.org" })),
            Some(BrowserError::BadBase("http://example.org".to_string()))
        );
        assert!(matches!(
            refused(json!({ "public_key": "AAAA" })),
            Some(BrowserError::Settings(_))
        ));
        assert_eq!(
            BrowserClient::start(&settings_with(json!({})), &[1u8; 31]).err(),
            Some(BrowserError::Seed)
        );
    }

    /// An input of no lane, or headers that are not pairs, are refused, and the core is not
    /// stepped.
    #[test]
    fn an_input_the_module_cannot_read_is_refused() {
        let mut client = started(1);
        assert!(matches!(client.woken("push"), Err(BrowserError::Input(_))));
        assert!(matches!(
            client.answered("poll", 200, "[[\"a\"]]", &[]),
            Err(BrowserError::Input(_))
        ));
        assert!(matches!(client.held("{}"), Err(BrowserError::Input(_))));
    }
}
