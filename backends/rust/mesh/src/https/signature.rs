// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Message signatures of the HTTPS binding (SCE_MESH.md section 22.9, ADR 0109 D9): HTTP Message
//! Signatures (RFC 9421) with Ed25519, over a body bound by its digest (RFC 9530), so that what a
//! server sent is told from what an intermediary that reads the exchange (a proxy that inspects
//! TLS) wrote, and what a client sent is proved by a key no such intermediary holds.
//!
//! Two signatures, each under the label `sce`, with `alg="ed25519"` and `tag="sce-mesh-1"`:
//!
//! * **A response** of status `200`, signed by the server's `sign:` key and verified by a client
//!   under `verify:` against its trust set, the key chosen by `keyid`: `@status`,
//!   `content-digest`, `sce-mesh-epoch`, `sce-mesh-first` when the response carries one, and of
//!   the request it answers (`;req`) `@method`, `@target-uri`, `sce-mesh-ack` and
//!   `sce-mesh-nonce`. The nonce is 16 random bytes the client draws for each exchange, so a
//!   response answers one request and no other.
//! * **A request** of the `key` authenticator, signed by the client's key and verified by the
//!   server against the key its name is pinned to: `@method`, `@target-uri`, `content-digest`,
//!   `sce-mesh-ack`, `sce-mesh-epoch` when the request carries one, `sce-mesh-hold-ms` and
//!   `sce-mesh-nonce`, with `created`; `keyid` is the key's COSE key thumbprint (RFC 9679) in
//!   base64url. The key itself travels in `Sce-Mesh-Key`, as the `COSE_Key` its thumbprint
//!   digests, for a server that has not pinned the name yet.
//!
//! What a signature covers is fixed by this profile and by what the message carries, never read
//! from the message: a verifier builds the list it expects and refuses a signature that covers
//! another. The structured fields (RFC 8941) are read and written here as far as the profile
//! needs, and the signature base is RFC 9421's, so that any implementation of the RFC makes and
//! checks the same bytes (Appendix B.2.6 is held below).

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use sha2::{Digest, Sha256};

use sce_rust_crypto::cose::{
    cose_key_ed25519_required, ed25519_of_cose_key, thumbprint_ed25519, TrustedKey,
};
use sce_rust_crypto::ed25519::{self, KeyPair, PUBLIC_KEY_BYTES};
use sce_rust_crypto::random::RandomSource;

use super::{ACK_HEADER, EPOCH_HEADER, FIRST_HEADER, HOLD_HEADER};

/// The field that names what a signature covers and how (RFC 9421 section 4.1).
pub const SIGNATURE_INPUT_HEADER: &str = "Signature-Input";
/// The field that carries a signature (RFC 9421 section 4.2).
pub const SIGNATURE_HEADER: &str = "Signature";
/// The digest of a message's content (RFC 9530 section 2).
pub const CONTENT_DIGEST_HEADER: &str = "Content-Digest";
/// The request header of the client's nonce: 16 random bytes in base64url without padding.
pub const NONCE_HEADER: &str = "Sce-Mesh-Nonce";
/// The request header that offers the client's public key under the `key` authenticator: the
/// `COSE_Key` its thumbprint digests, in base64url without padding.
pub const KEY_HEADER: &str = "Sce-Mesh-Key";
/// The label both signatures are kept under in the two fields.
pub const LABEL: &str = "sce";
/// The algorithm of RFC 9421 section 3.3.6.
pub const ALGORITHM: &str = "ed25519";
/// The application tag of RFC 9421 section 2.3, which keeps a signature this profile makes from
/// being taken for one another application of the same key makes.
pub const TAG: &str = "sce-mesh-1";
/// The random bytes of a nonce.
pub const NONCE_BYTES: usize = 16;
/// The authentication scheme a `401` of the `key` authenticator names.
pub const KEY_SCHEME: &str = "SCE-Signature";
/// deploy.yaml's `max_skew_ms` when the binding sets none: how far a request's `created` may lie
/// from the server's clock.
pub const DEFAULT_MAX_SKEW_MS: u32 = 300_000;

// ── Structured field values (RFC 8941), as far as the profile reads and writes them ──

/// A bare item (RFC 8941 section 3.3). A decimal is not one the profile reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BareItem {
    Integer(i64),
    String(String),
    Token(String),
    Bytes(Vec<u8>),
    Boolean(bool),
}

/// The parameters of an item or an inner list, in order (RFC 8941 section 3.1.2).
pub type Parameters = Vec<(String, BareItem)>;

/// An item and its parameters.
pub type Item = (BareItem, Parameters);

/// A member of a dictionary (RFC 8941 section 3.2): an item, or an inner list with parameters
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    Item(Item),
    InnerList(Vec<Item>, Parameters),
}

/// A cursor over a structured field's text.
struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(b' ') {
            self.at += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.at += 1;
        }
    }

    fn key(&mut self) -> Option<String> {
        let first = self.peek()?;
        if !(first.is_ascii_lowercase() || first == b'*') {
            return None;
        }
        let start = self.at;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b'*')
            {
                self.at += 1;
            } else {
                break;
            }
        }
        Some(String::from_utf8_lossy(&self.text[start..self.at]).into_owned())
    }

    fn bare_item(&mut self) -> Option<BareItem> {
        match self.peek()? {
            b'-' | b'0'..=b'9' => self.integer(),
            b'"' => self.string(),
            b':' => self.bytes(),
            b'?' => self.boolean(),
            byte if byte.is_ascii_alphabetic() || byte == b'*' => self.token(),
            _ => None,
        }
    }

    fn integer(&mut self) -> Option<BareItem> {
        let negative = self.peek() == Some(b'-');
        if negative {
            self.at += 1;
        }
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        let digits = self.at - start;
        // RFC 8941 section 3.3.1: at most fifteen digits; a `.` would make it a decimal.
        if digits == 0 || digits > 15 || self.peek() == Some(b'.') {
            return None;
        }
        let magnitude: i64 = core::str::from_utf8(&self.text[start..self.at])
            .ok()?
            .parse()
            .ok()?;
        Some(BareItem::Integer(if negative {
            -magnitude
        } else {
            magnitude
        }))
    }

    fn string(&mut self) -> Option<BareItem> {
        self.take()?;
        let mut out = String::new();
        loop {
            match self.take()? {
                b'\\' => match self.take()? {
                    escaped @ (b'"' | b'\\') => out.push(char::from(escaped)),
                    _ => return None,
                },
                b'"' => return Some(BareItem::String(out)),
                byte @ 0x20..=0x7e => out.push(char::from(byte)),
                _ => return None,
            }
        }
    }

    fn token(&mut self) -> Option<BareItem> {
        let start = self.at;
        self.at += 1;
        while let Some(byte) = self.peek() {
            let tchar = byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                        | b':'
                        | b'/'
                );
            if !tchar {
                break;
            }
            self.at += 1;
        }
        Some(BareItem::Token(
            String::from_utf8_lossy(&self.text[start..self.at]).into_owned(),
        ))
    }

    fn bytes(&mut self) -> Option<BareItem> {
        self.take()?;
        let start = self.at;
        while let Some(byte) = self.peek() {
            if byte == b':' {
                break;
            }
            if !(byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')) {
                return None;
            }
            self.at += 1;
        }
        let encoded = &self.text[start..self.at];
        self.take()?;
        // RFC 8941 section 4.2.7: a parser does not fail on absent `=` padding.
        let unpadded: &[u8] = {
            let mut end = encoded.len();
            while end > 0 && encoded[end - 1] == b'=' {
                end -= 1;
            }
            &encoded[..end]
        };
        STANDARD_NO_PAD.decode(unpadded).ok().map(BareItem::Bytes)
    }

    fn boolean(&mut self) -> Option<BareItem> {
        self.take()?;
        match self.take()? {
            b'1' => Some(BareItem::Boolean(true)),
            b'0' => Some(BareItem::Boolean(false)),
            _ => None,
        }
    }

    fn parameters(&mut self) -> Option<Parameters> {
        let mut parameters: Parameters = Vec::new();
        while self.peek() == Some(b';') {
            self.at += 1;
            self.skip_spaces();
            let key = self.key()?;
            let value = if self.peek() == Some(b'=') {
                self.at += 1;
                self.bare_item()?
            } else {
                BareItem::Boolean(true)
            };
            // RFC 8941 section 4.2.3.2: a repeated key keeps its first place and its last value.
            match parameters.iter_mut().find(|(held, _)| *held == key) {
                Some(slot) => slot.1 = value,
                None => parameters.push((key, value)),
            }
        }
        Some(parameters)
    }

    fn item(&mut self) -> Option<Item> {
        let bare = self.bare_item()?;
        Some((bare, self.parameters()?))
    }

    fn inner_list(&mut self) -> Option<Member> {
        self.take()?;
        let mut items = Vec::new();
        loop {
            self.skip_spaces();
            if self.peek()? == b')' {
                self.at += 1;
                return Some(Member::InnerList(items, self.parameters()?));
            }
            items.push(self.item()?);
            if !matches!(self.peek()?, b' ' | b')') {
                return None;
            }
        }
    }
}

/// The members of a dictionary field (RFC 8941 section 4.2.2), in order; `None` for a text that
/// is not one. Several field lines are one field, joined by `", "` first.
pub fn parse_dictionary(text: &str) -> Option<Vec<(String, Member)>> {
    let mut reader = Reader {
        text: text.as_bytes(),
        at: 0,
    };
    reader.skip_spaces();
    let mut members: Vec<(String, Member)> = Vec::new();
    while reader.peek().is_some() {
        let key = reader.key()?;
        let member = if reader.peek() == Some(b'=') {
            reader.at += 1;
            if reader.peek()? == b'(' {
                reader.inner_list()?
            } else {
                Member::Item(reader.item()?)
            }
        } else {
            Member::Item((BareItem::Boolean(true), reader.parameters()?))
        };
        match members.iter_mut().find(|(held, _)| *held == key) {
            Some(slot) => slot.1 = member,
            None => members.push((key, member)),
        }
        reader.skip_whitespace();
        if reader.peek().is_none() {
            break;
        }
        if reader.take()? != b',' {
            return None;
        }
        reader.skip_whitespace();
        // A comma is followed by a member: a trailing comma, which ends the text, is refused.
        reader.peek()?;
    }
    Some(members)
}

fn serialize_bare_item(item: &BareItem, out: &mut String) {
    match item {
        BareItem::Integer(value) => out.push_str(&value.to_string()),
        BareItem::String(text) => {
            out.push('"');
            for c in text.chars() {
                if matches!(c, '"' | '\\') {
                    out.push('\\');
                }
                out.push(c);
            }
            out.push('"');
        }
        BareItem::Token(token) => out.push_str(token),
        BareItem::Bytes(bytes) => {
            out.push(':');
            out.push_str(&STANDARD.encode(bytes));
            out.push(':');
        }
        BareItem::Boolean(value) => out.push_str(if *value { "?1" } else { "?0" }),
    }
}

fn serialize_parameters(parameters: &Parameters, out: &mut String) {
    for (key, value) in parameters {
        out.push(';');
        out.push_str(key);
        if *value != BareItem::Boolean(true) {
            out.push('=');
            serialize_bare_item(value, out);
        }
    }
}

/// The serialization of an inner list (RFC 8941 section 4.1.1.1): what the `@signature-params`
/// line of a signature base holds (RFC 9421 section 2.3).
pub fn serialize_inner_list(items: &[Item], parameters: &Parameters) -> String {
    let mut out = String::from("(");
    for (at, (bare, item_parameters)) in items.iter().enumerate() {
        if at > 0 {
            out.push(' ');
        }
        serialize_bare_item(bare, &mut out);
        serialize_parameters(item_parameters, &mut out);
    }
    out.push(')');
    serialize_parameters(parameters, &mut out);
    out
}

// ── The signature base (RFC 9421 section 2.5) ──

/// A component a signature covers (RFC 9421 section 2): a derived component (`@method`) or a
/// field by its name in lower case, and whether it is read from the request a response answers
/// (`;req`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub req: bool,
}

impl Component {
    /// The component `name` of the message itself.
    pub fn of(name: &str) -> Self {
        Component {
            name: name.to_ascii_lowercase(),
            req: false,
        }
    }

    /// The component `name` of the request a response answers.
    pub fn of_request(name: &str) -> Self {
        Component {
            name: name.to_ascii_lowercase(),
            req: true,
        }
    }

    /// Its component identifier, as a signature's inner list names it.
    pub fn identifier(&self) -> Item {
        let parameters = if self.req {
            alloc::vec![("req".to_string(), BareItem::Boolean(true))]
        } else {
            Vec::new()
        };
        (BareItem::String(self.name.clone()), parameters)
    }
}

/// The signature base of RFC 9421 section 2.5: a line of each covered component and its value,
/// in the order covered, then the `@signature-params` line, `signature_params` being the
/// serialization of the inner list of the covered components with the signature's parameters.
pub fn signature_base(covered: &[(Component, String)], signature_params: &str) -> String {
    let mut base = String::new();
    for (component, value) in covered {
        let (bare, parameters) = component.identifier();
        serialize_bare_item(&bare, &mut base);
        serialize_parameters(&parameters, &mut base);
        base.push_str(": ");
        base.push_str(value);
        base.push('\n');
    }
    base.push_str("\"@signature-params\": ");
    base.push_str(signature_params);
    base
}

/// The value of the field `name` among `headers`, compared without regard to case: every line of
/// it, each trimmed, joined by `", "` (RFC 9421 section 2.1). `None` when there is none.
pub fn field_value(headers: &[(String, String)], name: &str) -> Option<String> {
    let lines: Vec<&str> = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
        .collect();
    (!lines.is_empty()).then(|| lines.join(", "))
}

// ── What the profile signs ──

/// The `Content-Digest` of a body (RFC 9530): its SHA-256, as the dictionary member `sha-256`.
pub fn content_digest(body: &[u8]) -> String {
    let mut value = String::from("sha-256=");
    serialize_bare_item(&BareItem::Bytes(Sha256::digest(body).to_vec()), &mut value);
    value
}

/// Whether `field`, a `Content-Digest` value, holds the SHA-256 of `body`. Another algorithm
/// beside it is passed over; a value without `sha-256` is not one the profile takes.
pub fn digest_matches(field: &str, body: &[u8]) -> bool {
    let Some(members) = parse_dictionary(field) else {
        return false;
    };
    members.iter().any(|(key, member)| {
        key == "sha-256"
            && matches!(member, Member::Item((BareItem::Bytes(digest), _))
                if digest.as_slice() == Sha256::digest(body).as_slice())
    })
}

/// A fresh nonce of [`NONCE_BYTES`] from `random`, in base64url without padding; `None` when the
/// source gave no bytes.
pub fn draw_nonce(random: &mut dyn RandomSource) -> Option<String> {
    let mut bytes = [0u8; NONCE_BYTES];
    random
        .fill(&mut bytes)
        .then(|| URL_SAFE_NO_PAD.encode(bytes))
}

/// The `keyid` of a request signature: the COSE key thumbprint (RFC 9679) of the client's public
/// key, in base64url without padding.
pub fn key_id_of(public_key: &[u8; PUBLIC_KEY_BYTES]) -> String {
    URL_SAFE_NO_PAD.encode(thumbprint_ed25519(public_key))
}

/// The value of `Sce-Mesh-Key` that offers `public_key`: the `COSE_Key` its thumbprint digests,
/// in base64url without padding.
pub fn offered_key(public_key: &[u8; PUBLIC_KEY_BYTES]) -> String {
    URL_SAFE_NO_PAD.encode(cose_key_ed25519_required(public_key))
}

/// The public key a `Sce-Mesh-Key` value offers, when it is exactly what [`offered_key`] writes.
pub fn key_of_offer(text: &str) -> Option<[u8; PUBLIC_KEY_BYTES]> {
    let bytes = URL_SAFE_NO_PAD.decode(text.trim()).ok()?;
    ed25519_of_cose_key(&bytes)
}

/// A target URI (RFC 9110 section 7.1) as both ends of the binding read it for `@target-uri`:
/// the scheme and the host in lower case, the port left out when it is the scheme's own, and an
/// empty path written `/`. `None` for a text that is no absolute `http` or `https` URI.
pub fn normalize_target_uri(uri: &str) -> Option<String> {
    let (scheme, rest) = uri.split_once("://")?;
    let split = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, path_and_query) = rest.split_at(split);
    target_uri(scheme, authority, path_and_query)
}

/// [`normalize_target_uri`] of a URI given by its parts, as a server reads them from a request:
/// its scheme, its authority (`Host`, or the `:authority` of HTTP/2) and its path and query.
pub fn target_uri(scheme: &str, authority: &str, path_and_query: &str) -> Option<String> {
    let scheme = scheme.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "https" => "443",
        "http" => "80",
        _ => return None,
    };
    let authority = authority.rsplit('@').next().unwrap_or_default();
    let (host, port) = if let Some(literal) = authority.strip_prefix('[') {
        let (host, after) = literal.split_once(']')?;
        let port = match after {
            "" => None,
            colon => Some(colon.strip_prefix(':')?),
        };
        (alloc::format!("[{}]", host.to_ascii_lowercase()), port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_ascii_lowercase(), Some(port)),
            None => (authority.to_ascii_lowercase(), None),
        }
    };
    if host.is_empty() || port.is_some_and(|port| !port.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let port = port.filter(|port| !port.is_empty() && *port != default_port);
    let path = if path_and_query.is_empty() {
        "/"
    } else {
        path_and_query
    };
    Some(match port {
        Some(port) => alloc::format!("{scheme}://{host}:{port}{path}"),
        None => alloc::format!("{scheme}://{host}{path}"),
    })
}

/// Why a signed message was not taken: the words a refusal is told with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No `Content-Digest`, or one that is not the body's.
    Digest,
    /// No signature under the profile's label, or fields that do not read as RFC 8941 says.
    Unsigned,
    /// A signature that covers another list of components, names other parameters, or covers a
    /// field the message lacks.
    Profile,
    /// A `keyid` of no key the verifier holds for the signer.
    UnknownKey,
    /// A `created` further from the verifier's clock than it allows.
    Skew,
    /// The signature does not verify.
    Signature,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refusal::Digest => "the Content-Digest is absent or not the body's",
            Refusal::Unsigned => "no signature of the binding's profile",
            Refusal::Profile => "a signature that covers other than the profile names",
            Refusal::UnknownKey => "a signature by no key the verifier holds",
            Refusal::Skew => "a signature created too far from the verifier's clock",
            Refusal::Signature => "a signature that does not verify",
        })
    }
}

/// The parts of the request a response answers that its signature covers (`;req`), as the
/// request carried them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    /// The request's target URI, normalized ([`normalize_target_uri`]).
    pub target_uri: String,
    /// The request's `Sce-Mesh-Ack`.
    pub ack: String,
    /// The request's `Sce-Mesh-Nonce`.
    pub nonce: String,
}

/// The components a response signature covers, given whether the response carries
/// `Sce-Mesh-First`: what a signer signs and a verifier expects.
fn response_components(has_first: bool) -> Vec<Component> {
    let mut covered = alloc::vec![
        Component::of("@status"),
        Component::of(CONTENT_DIGEST_HEADER),
        Component::of(EPOCH_HEADER),
    ];
    if has_first {
        covered.push(Component::of(FIRST_HEADER));
    }
    covered.extend([
        Component::of_request("@method"),
        Component::of_request("@target-uri"),
        Component::of_request(ACK_HEADER),
        Component::of_request(NONCE_HEADER),
    ]);
    covered
}

/// The components a request signature covers, given whether the request carries
/// `Sce-Mesh-Epoch`.
fn request_components(has_epoch: bool) -> Vec<Component> {
    let mut covered = alloc::vec![
        Component::of("@method"),
        Component::of("@target-uri"),
        Component::of(CONTENT_DIGEST_HEADER),
        Component::of(ACK_HEADER),
    ];
    if has_epoch {
        covered.push(Component::of(EPOCH_HEADER));
    }
    covered.extend([Component::of(HOLD_HEADER), Component::of(NONCE_HEADER)]);
    covered
}

/// The two fields that carry a signature of `base` by `key` under [`LABEL`]: `Signature-Input`
/// with `signature_params`, and `Signature`.
fn signature_fields(
    key: &KeyPair,
    covered: &[(Component, String)],
    signature_params: &str,
) -> [(String, String); 2] {
    let signature = key.sign(signature_base(covered, signature_params).as_bytes());
    let mut value = alloc::format!("{LABEL}=");
    serialize_bare_item(&BareItem::Bytes(signature.to_vec()), &mut value);
    [
        (
            SIGNATURE_INPUT_HEADER.to_string(),
            alloc::format!("{LABEL}={signature_params}"),
        ),
        (SIGNATURE_HEADER.to_string(), value),
    ]
}

/// The values of `components` read from a message: a derived component by `derived`, a field of
/// the message by `field`, one of the request by `request_field`. `None` when one is absent.
fn values_of(
    components: Vec<Component>,
    derived: &dyn Fn(&Component) -> Option<String>,
    field: &dyn Fn(&str) -> Option<String>,
) -> Option<Vec<(Component, String)>> {
    components
        .into_iter()
        .map(|component| {
            let value = if component.name.starts_with('@') {
                derived(&component)
            } else {
                field(component.name.as_str())
            }?;
            Some((component, value))
        })
        .collect()
}

/// A signature as a message carries it under [`LABEL`]: what its inner list covers, its
/// parameters, and its bytes.
struct Carried {
    items: Vec<Item>,
    parameters: Parameters,
    signature: Vec<u8>,
}

impl Carried {
    /// The signature `field` finds under [`LABEL`], or why there is none.
    fn read(field: &dyn Fn(&str) -> Option<String>) -> Result<Carried, Refusal> {
        let input = field(SIGNATURE_INPUT_HEADER)
            .and_then(|text| parse_dictionary(&text))
            .ok_or(Refusal::Unsigned)?;
        let signatures = field(SIGNATURE_HEADER)
            .and_then(|text| parse_dictionary(&text))
            .ok_or(Refusal::Unsigned)?;
        let Some((_, Member::InnerList(items, parameters))) =
            input.into_iter().find(|(key, _)| key == LABEL)
        else {
            return Err(Refusal::Unsigned);
        };
        let Some((_, Member::Item((BareItem::Bytes(signature), _)))) =
            signatures.into_iter().find(|(key, _)| key == LABEL)
        else {
            return Err(Refusal::Unsigned);
        };
        Ok(Carried {
            items,
            parameters,
            signature,
        })
    }

    /// Whether it covers exactly `expected`, in that order.
    fn covers(&self, expected: &[Component]) -> bool {
        self.items.len() == expected.len()
            && self
                .items
                .iter()
                .zip(expected)
                .all(|(item, component)| *item == component.identifier())
    }

    /// The parameter `name`, which must be there once.
    fn parameter(&self, name: &str) -> Option<&BareItem> {
        self.parameters
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// Whether its parameters are exactly `names`, in any order, with `alg` and `tag` the
    /// profile's.
    fn parameters_are(&self, names: &[&str]) -> bool {
        self.parameters.len() == names.len()
            && names.iter().all(|name| self.parameter(name).is_some())
            && self.parameter("alg") == Some(&BareItem::String(ALGORITHM.to_string()))
            && self.parameter("tag") == Some(&BareItem::String(TAG.to_string()))
    }

    /// The `keyid` parameter's text.
    fn key_id(&self) -> Result<&str, Refusal> {
        match self.parameter("keyid") {
            Some(BareItem::String(id)) => Ok(id.as_str()),
            _ => Err(Refusal::Profile),
        }
    }

    /// Whether `public_key` made it over `covered`.
    fn verifies(
        &self,
        public_key: &[u8; PUBLIC_KEY_BYTES],
        covered: &[(Component, String)],
    ) -> bool {
        let params = serialize_inner_list(&self.items, &self.parameters);
        ed25519::verify(
            public_key,
            signature_base(covered, &params).as_bytes(),
            &self.signature,
        )
    }
}

/// What a server's response signature is made of: the key a client's trust set knows by
/// `key_id`, a `sign` key of the server's host (docs/SCE_HOST_FEATURES.md section 13).
#[derive(Clone, PartialEq, Eq)]
pub struct ResponseSigner {
    key_id: String,
    key: KeyPair,
}

impl fmt::Debug for ResponseSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseSigner")
            .field("key_id", &self.key_id)
            .field("key", &self.key)
            .finish()
    }
}

impl ResponseSigner {
    /// The signer of the key `key`, known to clients as `key_id`.
    pub fn new(key_id: impl Into<String>, key: KeyPair) -> Self {
        ResponseSigner {
            key_id: key_id.into(),
            key,
        }
    }

    /// The id clients know the key by.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// The `Signature-Input` and `Signature` of a `200` whose `Content-Digest`, `Sce-Mesh-Epoch`
    /// and `Sce-Mesh-First` (when it carries one) are given, answering `answered`.
    pub fn sign(
        &self,
        content_digest: &str,
        epoch: &str,
        first: Option<&str>,
        answered: &Answered,
    ) -> [(String, String); 2] {
        // §mesh-22.9: a `200` is signed over its status, its body's digest and its numbering, and
        // over the request it answers, so that it answers that request alone.
        let mut covered = alloc::vec![
            (Component::of("@status"), "200".to_string()),
            (
                Component::of(CONTENT_DIGEST_HEADER),
                content_digest.to_string()
            ),
            (Component::of(EPOCH_HEADER), epoch.to_string()),
        ];
        if let Some(first) = first {
            covered.push((Component::of(FIRST_HEADER), first.to_string()));
        }
        covered.extend([
            (Component::of_request("@method"), "POST".to_string()),
            (
                Component::of_request("@target-uri"),
                answered.target_uri.clone(),
            ),
            (Component::of_request(ACK_HEADER), answered.ack.clone()),
            (Component::of_request(NONCE_HEADER), answered.nonce.clone()),
        ]);
        let items: Vec<Item> = covered
            .iter()
            .map(|(component, _)| component.identifier())
            .collect();
        let parameters: Parameters = alloc::vec![
            ("keyid".to_string(), BareItem::String(self.key_id.clone())),
            ("alg".to_string(), BareItem::String(ALGORITHM.to_string())),
            ("tag".to_string(), BareItem::String(TAG.to_string())),
        ];
        signature_fields(
            &self.key,
            &covered,
            &serialize_inner_list(&items, &parameters),
        )
    }
}

/// Whether a response of `status` with the fields `field` reads and the body `body` is a `200`
/// that a key of `trusted` signed as this profile signs one, answering `answered`; the
/// signer is chosen by `keyid` alone (a trust set rotates keys by id).
pub fn verify_response(
    trusted: &[TrustedKey],
    status: u16,
    field: &dyn Fn(&str) -> Option<String>,
    body: &[u8],
    answered: &Answered,
) -> Result<(), Refusal> {
    // §mesh-22.9: the list a response's signature covers is the profile's, built here from what
    // the response carries and never read from it; a signature of any other list is refused.
    if status != 200 {
        return Err(Refusal::Profile);
    }
    let digest = field(CONTENT_DIGEST_HEADER).ok_or(Refusal::Digest)?;
    if !digest_matches(&digest, body) {
        return Err(Refusal::Digest);
    }
    let carried = Carried::read(field)?;
    let expected = response_components(field(FIRST_HEADER).is_some());
    if !carried.covers(&expected) || !carried.parameters_are(&["keyid", "alg", "tag"]) {
        return Err(Refusal::Profile);
    }
    let key_id = carried.key_id()?;
    let signer = trusted
        .iter()
        .find(|key| key.id == key_id)
        .ok_or(Refusal::UnknownKey)?;
    let derived = |component: &Component| -> Option<String> {
        match (component.name.as_str(), component.req) {
            ("@status", false) => Some(status.to_string()),
            ("@method", true) => Some("POST".to_string()),
            ("@target-uri", true) => Some(answered.target_uri.clone()),
            _ => None,
        }
    };
    let request_field = |name: &str| -> Option<String> {
        if name.eq_ignore_ascii_case(ACK_HEADER) {
            Some(answered.ack.clone())
        } else if name.eq_ignore_ascii_case(NONCE_HEADER) {
            Some(answered.nonce.clone())
        } else {
            None
        }
    };
    let covered: Option<Vec<(Component, String)>> = expected
        .into_iter()
        .map(|component| {
            let value = if component.name.starts_with('@') {
                derived(&component)
            } else if component.req {
                request_field(component.name.as_str())
            } else {
                field(component.name.as_str())
            }?;
            Some((component, value))
        })
        .collect();
    let covered = covered.ok_or(Refusal::Profile)?;
    if carried.verifies(&signer.public_key, &covered) {
        Ok(())
    } else {
        Err(Refusal::Signature)
    }
}

/// What a client signs its requests with under the `key` authenticator: its key pair, generated
/// once by its host (docs/SCE_HOST_FEATURES.md section 13), and the id and the offer derived from
/// the public key.
#[derive(Clone, PartialEq, Eq)]
pub struct RequestSigner {
    key: KeyPair,
    key_id: String,
    offer: String,
}

impl fmt::Debug for RequestSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestSigner")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

impl RequestSigner {
    /// The signer of `key`.
    pub fn new(key: KeyPair) -> Self {
        let public = key.public_key();
        RequestSigner {
            key_id: key_id_of(&public),
            offer: offered_key(&public),
            key,
        }
    }

    /// The `keyid` its signatures name: the thumbprint of its public key.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// The fields that sign a `POST` to `target_uri` (normalized) whose other fields are
    /// `headers`, made at `created` (seconds since the Unix epoch): `Sce-Mesh-Key`,
    /// `Signature-Input` and `Signature`. `headers` holds `Content-Digest`, `Sce-Mesh-Ack`,
    /// `Sce-Mesh-Hold-Ms`, `Sce-Mesh-Nonce` and, when the request names one, `Sce-Mesh-Epoch`;
    /// `None` when one of the others is missing.
    pub fn sign(
        &self,
        target_uri: &str,
        headers: &[(String, String)],
        created: i64,
    ) -> Option<[(String, String); 3]> {
        // §mesh-22.9: the `key` authenticator's request signature, over the request's method,
        // target, body digest, numbering, hold and nonce, made at `created`.
        let field = |name: &str| field_value(headers, name);
        let expected = request_components(field(EPOCH_HEADER).is_some());
        let derived = |component: &Component| -> Option<String> {
            match component.name.as_str() {
                "@method" => Some("POST".to_string()),
                "@target-uri" => Some(target_uri.to_string()),
                _ => None,
            }
        };
        let covered = values_of(expected, &derived, &field)?;
        let items: Vec<Item> = covered
            .iter()
            .map(|(component, _)| component.identifier())
            .collect();
        let parameters: Parameters = alloc::vec![
            ("created".to_string(), BareItem::Integer(created)),
            ("keyid".to_string(), BareItem::String(self.key_id.clone())),
            ("alg".to_string(), BareItem::String(ALGORITHM.to_string())),
            ("tag".to_string(), BareItem::String(TAG.to_string())),
        ];
        let [input, signature] = signature_fields(
            &self.key,
            &covered,
            &serialize_inner_list(&items, &parameters),
        );
        Some([
            (KEY_HEADER.to_string(), self.offer.clone()),
            input,
            signature,
        ])
    }
}

/// A request signature as a server reads it before the request's body: the signature itself, and
/// the `keyid` that says which key made it.
pub struct RequestSignature {
    carried: Carried,
    key_id: String,
    created: i64,
}

impl RequestSignature {
    /// The signature the fields `field` reads carry, held to the profile's list of components
    /// and parameters.
    pub fn read(field: &dyn Fn(&str) -> Option<String>) -> Result<Self, Refusal> {
        let carried = Carried::read(field)?;
        let expected = request_components(field(EPOCH_HEADER).is_some());
        if !carried.covers(&expected)
            || !carried.parameters_are(&["created", "keyid", "alg", "tag"])
        {
            return Err(Refusal::Profile);
        }
        let key_id = carried.key_id()?.to_string();
        let Some(BareItem::Integer(created)) = carried.parameter("created").cloned() else {
            return Err(Refusal::Profile);
        };
        Ok(RequestSignature {
            carried,
            key_id,
            created,
        })
    }

    /// The `keyid` it names.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Whether `public_key` made it over a `POST` to `target_uri` whose fields `field` reads, at
    /// a `created` no further than `max_skew_s` seconds from `now_s`. The body is held to its
    /// `Content-Digest` apart ([`digest_matches`]), once it has arrived.
    pub fn verify(
        &self,
        public_key: &[u8; PUBLIC_KEY_BYTES],
        target_uri: &str,
        field: &dyn Fn(&str) -> Option<String>,
        now_s: i64,
        max_skew_s: u64,
    ) -> Result<(), Refusal> {
        // §mesh-22.9: the key the name is held to, and a `created` within the server's skew.
        if self.key_id != key_id_of(public_key) {
            return Err(Refusal::UnknownKey);
        }
        if now_s.abs_diff(self.created) > max_skew_s {
            return Err(Refusal::Skew);
        }
        let expected = request_components(field(EPOCH_HEADER).is_some());
        let derived = |component: &Component| -> Option<String> {
            match component.name.as_str() {
                "@method" => Some("POST".to_string()),
                "@target-uri" => Some(target_uri.to_string()),
                _ => None,
            }
        };
        let covered = values_of(expected, &derived, field).ok_or(Refusal::Profile)?;
        if self.carried.verifies(public_key, &covered) {
            Ok(())
        } else {
            Err(Refusal::Signature)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(text: &str) -> [u8; 32] {
        sce_rust_crypto::bytes_of_hex(text)
            .expect("hex")
            .try_into()
            .expect("32 bytes")
    }

    /// RFC 9421 Appendix B.1.4's `test-key-ed25519`: the seed its PKCS#8 document carries.
    const B14_SEED: &str = "9f8362f87a484a954e6e740c5b4c0e84229139a20aa8ab56ff66586f6a7d29c5";
    const B14_PUBLIC: &str = "26b40b8f93fff3d897112f7ebc582b232dbd72517d082fe83cfb30ddce43d1bb";

    /// RFC 9421 Appendix B.2.6: the request of B.2 signed with `test-key-ed25519` over `date`,
    /// `@method`, `@path`, `@authority`, `content-type` and `content-length`. Ed25519 is
    /// deterministic, so the signature base this module writes, signed with the key, is the
    /// signature the RFC prints; and the RFC's `Signature-Input` reads back as the inner list it
    /// writes. Red if a line of the base, the identifier serialization or the parameters move.
    #[test]
    fn rfc_9421_appendix_b_2_6_is_signed_byte_for_byte() {
        let key = KeyPair::from_seed(&hex32(B14_SEED));
        assert_eq!(key.public_key(), hex32(B14_PUBLIC));
        let covered = [
            (Component::of("date"), "Tue, 20 Apr 2021 02:07:55 GMT"),
            (Component::of("@method"), "POST"),
            (Component::of("@path"), "/foo"),
            (Component::of("@authority"), "example.com"),
            (Component::of("content-type"), "application/json"),
            (Component::of("content-length"), "18"),
        ]
        .map(|(component, value)| (component, value.to_string()));
        let items: Vec<Item> = covered.iter().map(|(c, _)| c.identifier()).collect();
        let parameters: Parameters = alloc::vec![
            ("created".to_string(), BareItem::Integer(1_618_884_473)),
            (
                "keyid".to_string(),
                BareItem::String("test-key-ed25519".to_string())
            ),
        ];
        let params = serialize_inner_list(&items, &parameters);
        assert_eq!(
            params,
            r#"("date" "@method" "@path" "@authority" "content-type" "content-length");created=1618884473;keyid="test-key-ed25519""#
        );
        let base = signature_base(&covered, &params);
        assert_eq!(
            base,
            "\"date\": Tue, 20 Apr 2021 02:07:55 GMT\n\
             \"@method\": POST\n\
             \"@path\": /foo\n\
             \"@authority\": example.com\n\
             \"content-type\": application/json\n\
             \"content-length\": 18\n\
             \"@signature-params\": (\"date\" \"@method\" \"@path\" \"@authority\" \"content-type\" \"content-length\");created=1618884473;keyid=\"test-key-ed25519\""
        );
        let signature = key.sign(base.as_bytes());
        assert_eq!(
            STANDARD.encode(signature),
            "wqcAqbmYJ2ji2glfAMaRy4gruYYnx2nEFN2HN6jrnDnQCK1u02Gb04v9EDgwUPiu4A0w6vuQv5lIp5WPpBKRCw=="
        );
        assert!(ed25519::verify(
            &key.public_key(),
            base.as_bytes(),
            &signature
        ));

        let read = parse_dictionary(&alloc::format!("sig-b26={params}")).expect("a dictionary");
        let [(label, Member::InnerList(read_items, read_parameters))] = read.as_slice() else {
            panic!("one inner list, got {read:?}");
        };
        assert_eq!(label, "sig-b26");
        assert_eq!(serialize_inner_list(read_items, read_parameters), params);
        let signatures =
            parse_dictionary(&alloc::format!("sig-b26=:{}:", STANDARD.encode(signature)))
                .expect("a dictionary");
        assert_eq!(
            signatures,
            alloc::vec![(
                "sig-b26".to_string(),
                Member::Item((BareItem::Bytes(signature.to_vec()), Vec::new()))
            )]
        );
    }

    /// RFC 9530 section 2's example: the SHA-256 of `{"hello": "world"}`.
    #[test]
    fn a_content_digest_is_the_sha_256_of_the_body() {
        let body = br#"{"hello": "world"}"#;
        let digest = content_digest(body);
        assert_eq!(
            digest,
            "sha-256=:X48E9qOokqqrvdts8nOJRJN3OWDUoyWxBf7kbu9DBPE=:"
        );
        assert!(digest_matches(&digest, body));
        assert!(digest_matches(
            &alloc::format!("sha-512=:AAAA:, {digest}"),
            body
        ));
        assert!(!digest_matches(&digest, b"{}"));
        assert!(!digest_matches("sha-512=:AAAA:", body), "no sha-256");
        assert!(!digest_matches("not a dictionary(", body));
    }

    #[test]
    fn a_dictionary_reads_what_rfc_8941_writes_and_refuses_the_rest() {
        assert_eq!(
            parse_dictionary(r#"a=1, b=?0;x, c="q\"t", d=tok/en, e=(1 "two");p"#),
            Some(alloc::vec![
                (
                    "a".to_string(),
                    Member::Item((BareItem::Integer(1), Vec::new()))
                ),
                (
                    "b".to_string(),
                    Member::Item((
                        BareItem::Boolean(false),
                        alloc::vec![("x".to_string(), BareItem::Boolean(true))]
                    ))
                ),
                (
                    "c".to_string(),
                    Member::Item((BareItem::String("q\"t".to_string()), Vec::new()))
                ),
                (
                    "d".to_string(),
                    Member::Item((BareItem::Token("tok/en".to_string()), Vec::new()))
                ),
                (
                    "e".to_string(),
                    Member::InnerList(
                        alloc::vec![
                            (BareItem::Integer(1), Vec::new()),
                            (BareItem::String("two".to_string()), Vec::new())
                        ],
                        alloc::vec![("p".to_string(), BareItem::Boolean(true))]
                    )
                ),
            ])
        );
        for bad in ["a=", "a=1,", "A=1", "a=1.5", "a=(1", r#"a="\n""#, "a=1 b=2"] {
            assert_eq!(parse_dictionary(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_target_uri_is_read_one_way_at_both_ends() {
        for (given, normal) in [
            (
                "https://Cal.Example:443/api/sce-mesh/1/phone",
                "https://cal.example/api/sce-mesh/1/phone",
            ),
            (
                "HTTPS://cal.example:9443/sce-mesh/1/phone",
                "https://cal.example:9443/sce-mesh/1/phone",
            ),
            ("http://127.0.0.1:80", "http://127.0.0.1/"),
            ("http://[::1]:9443/x?y", "http://[::1]:9443/x?y"),
        ] {
            assert_eq!(
                normalize_target_uri(given).as_deref(),
                Some(normal),
                "{given}"
            );
        }
        assert_eq!(
            target_uri("https", "cal.example:443", "/sce-mesh/1/phone").as_deref(),
            Some("https://cal.example/sce-mesh/1/phone")
        );
        for bad in [
            "cal.example/x",
            "wss://cal.example/x",
            "https://",
            "https://h:x/",
        ] {
            assert_eq!(normalize_target_uri(bad), None, "{bad}");
        }
    }

    const EPOCH: &str = "0123456789abcdef0123456789abcdef";

    fn answered() -> Answered {
        Answered {
            target_uri: "https://cal.example/sce-mesh/1/phone".to_string(),
            ack: "4".to_string(),
            nonce: "AAECAwQFBgcICQoLDA0ODw".to_string(),
        }
    }

    fn server() -> (ResponseSigner, Vec<TrustedKey>) {
        let key = KeyPair::from_seed(&[3u8; 32]);
        let trusted = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: key.public_key(),
        }];
        (ResponseSigner::new("server-2026", key), trusted)
    }

    /// A `200` the server signed, with `body` numbered from `first`.
    fn signed_response(
        signer: &ResponseSigner,
        first: Option<&str>,
        body: &[u8],
        answered: &Answered,
    ) -> Vec<(String, String)> {
        let digest = content_digest(body);
        let mut headers = alloc::vec![
            (EPOCH_HEADER.to_string(), EPOCH.to_string()),
            (CONTENT_DIGEST_HEADER.to_string(), digest.clone()),
        ];
        if let Some(first) = first {
            headers.push((FIRST_HEADER.to_string(), first.to_string()));
        }
        headers.extend(signer.sign(&digest, EPOCH, first, answered));
        headers
    }

    fn verify(
        trusted: &[TrustedKey],
        headers: &[(String, String)],
        body: &[u8],
        answered: &Answered,
    ) -> Result<(), Refusal> {
        let field = |name: &str| field_value(headers, name);
        verify_response(trusted, 200, &field, body, answered)
    }

    /// A response verifies under the key its `keyid` names, and each thing an intermediary could
    /// change is refused: the first number, the body, the request it answers (a response
    /// replayed to another nonce), the signer, and the signature's absence. Red if any covered
    /// component leaves the base, or `Content-Digest` is not held to the body.
    #[test]
    fn a_response_verifies_and_every_change_is_refused() {
        let (signer, trusted) = server();
        let body = [0x01, 0x02];
        let headers = signed_response(&signer, Some("5"), &body, &answered());
        assert_eq!(verify(&trusted, &headers, &body, &answered()), Ok(()));

        let mut first = headers.clone();
        first
            .iter_mut()
            .find(|(name, _)| name == FIRST_HEADER)
            .expect("a first number")
            .1 = "1".to_string();
        assert_eq!(
            verify(&trusted, &first, &body, &answered()),
            Err(Refusal::Signature)
        );

        let mut dropped_first = headers.clone();
        dropped_first.retain(|(name, _)| name != FIRST_HEADER);
        assert_eq!(
            verify(&trusted, &dropped_first, &body, &answered()),
            Err(Refusal::Profile),
            "the signature covers a first number the response no longer carries"
        );

        assert_eq!(
            verify(&trusted, &headers, &[0x01, 0x03], &answered()),
            Err(Refusal::Digest)
        );

        let replayed = Answered {
            nonce: "BBECAwQFBgcICQoLDA0ODw".to_string(),
            ..answered()
        };
        assert_eq!(
            verify(&trusted, &headers, &body, &replayed),
            Err(Refusal::Signature),
            "an earlier response answers another nonce"
        );

        let stranger = alloc::vec![TrustedKey {
            id: "someone-else".to_string(),
            public_key: KeyPair::from_seed(&[4u8; 32]).public_key(),
        }];
        assert_eq!(
            verify(&stranger, &headers, &body, &answered()),
            Err(Refusal::UnknownKey)
        );
        let impostor = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: KeyPair::from_seed(&[4u8; 32]).public_key(),
        }];
        assert_eq!(
            verify(&impostor, &headers, &body, &answered()),
            Err(Refusal::Signature)
        );

        let mut unsigned = headers.clone();
        unsigned.retain(|(name, _)| name != SIGNATURE_HEADER);
        assert_eq!(
            verify(&trusted, &unsigned, &body, &answered()),
            Err(Refusal::Unsigned)
        );

        let empty = signed_response(&signer, None, &[], &answered());
        assert_eq!(verify(&trusted, &empty, &[], &answered()), Ok(()));
    }

    fn client_key() -> KeyPair {
        KeyPair::from_seed(&[9u8; 32])
    }

    const TARGET: &str = "https://cal.example/sce-mesh/1/phone";

    fn request_headers(body: &[u8], epoch: Option<&str>) -> Vec<(String, String)> {
        let mut headers = alloc::vec![
            (CONTENT_DIGEST_HEADER.to_string(), content_digest(body)),
            (ACK_HEADER.to_string(), "0".to_string()),
            (HOLD_HEADER.to_string(), "0".to_string()),
            (NONCE_HEADER.to_string(), answered().nonce),
        ];
        if let Some(epoch) = epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.to_string()));
        }
        headers
    }

    /// A request signed by the client's key verifies under that key within the skew, and is
    /// refused outside it, under another key, and with a covered field changed. Red if `created`
    /// is not held to the skew or a covered field leaves the base.
    #[test]
    fn a_request_verifies_under_its_key_and_its_clock() {
        let signer = RequestSigner::new(client_key());
        let mut headers = request_headers(&[0x05], Some(EPOCH));
        headers.extend(signer.sign(TARGET, &headers, 1_000_000).expect("signed"));
        let field = |name: &str| field_value(&headers, name);
        let read = RequestSignature::read(&field).expect("the profile's signature");
        assert_eq!(read.key_id(), signer.key_id());
        let offered = key_of_offer(&field(KEY_HEADER).expect("an offer")).expect("a key");
        assert_eq!(offered, client_key().public_key());
        assert_eq!(
            read.verify(&offered, TARGET, &field, 1_000_000 + 300, 300),
            Ok(())
        );
        assert_eq!(
            read.verify(&offered, TARGET, &field, 1_000_000 - 301, 300),
            Err(Refusal::Skew)
        );
        assert_eq!(
            read.verify(
                &KeyPair::from_seed(&[8u8; 32]).public_key(),
                TARGET,
                &field,
                1_000_000,
                300
            ),
            Err(Refusal::UnknownKey),
            "a second key is not the one the signature names"
        );
        assert_eq!(
            read.verify(
                &offered,
                "https://cal.example/sce-mesh/1/tablet",
                &field,
                1_000_000,
                300
            ),
            Err(Refusal::Signature),
            "a signature for one name is no signature for another"
        );
        let mut acked = headers.clone();
        acked
            .iter_mut()
            .find(|(name, _)| name == ACK_HEADER)
            .expect("an acknowledgement")
            .1 = "9".to_string();
        let acked_field = |name: &str| field_value(&acked, name);
        assert_eq!(
            read.verify(&offered, TARGET, &acked_field, 1_000_000, 300),
            Err(Refusal::Signature)
        );

        let mut first = request_headers(&[], None);
        first.extend(signer.sign(TARGET, &first, 7).expect("signed"));
        let first_field = |name: &str| field_value(&first, name);
        let read = RequestSignature::read(&first_field).expect("without an incarnation");
        assert_eq!(read.verify(&offered, TARGET, &first_field, 7, 0), Ok(()));
    }
}
