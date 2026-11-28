// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The freshness step of the HTTPS binding (docs/adr/0117, D4; SCE_MESH.md section 22.9): a
//! request a client signed is of use for a bounded time after the client was given the value it
//! signed, however long it was kept before it was sent.
//!
//! A server under `fresh_ms` makes a value for each step of that many milliseconds and each name
//! a client posts under (a machine, or an instance of an open machine as `<machine>/<instance>`),
//! from a key of its own that nobody else holds: `<step>.<tag>`, the step in decimal and the tag
//! the HMAC-SHA-256 (RFC 2104) of the name and the step under the key, in base64url without
//! padding. It gives the present value in `Sce-Mesh-Fresh` with every answer it makes to a
//! request it admitted, and takes a request under the `key` authenticator only when it signs
//! the value of the present step or of the one before ([`super::fresh_step::admitted`]); it
//! keeps nothing for it. A request that signs an older value, or none, is refused `401` before
//! anything of it is taken in, and the refusal carries the present value, signed under `sign:`
//! as a `200` is, so the client signs again (RFC 9449, sections 8 and 11.2, the same form for a
//! nonce a server provides).
//!
//! What this gives is told in that form and no larger: one who can sign with a client's key
//! while they are inside its host, and keeps what they signed, is served in the step after they
//! left and no later. While they are inside they act as the client.

use alloc::string::String;
use core::fmt;
use core::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sce_rust_crypto::equal::same_secret;
use sce_rust_crypto::mac::{hmac_sha256, TAG_BYTES};

use super::fresh_step::admitted;

/// The header that carries a value of the freshness step: on an answer, the present value the
/// server gives; on a request, the value the client signs.
pub const FRESH_HEADER: &str = "Sce-Mesh-Fresh";

/// The `error` of the `WWW-Authenticate` challenge a refusal for freshness names, as RFC 9449,
/// section 9, names the refusal of a request that does not carry the nonce it was given.
pub const USE_FRESH: &str = "use_fresh";

/// What a tag is made over besides the name and the step, so that a value of this profile is
/// never taken for a tag the same key makes for another use.
const DOMAIN: &[u8] = b"sce-mesh-fresh-1";

/// The step a value was made for, read from its text: `None` for a text that is not a value of
/// this profile, a step in decimal of at most twenty digits, a `.`, and the tag's 43 characters.
pub fn step_of(value: &str) -> Option<u64> {
    let (step, tag) = value.trim().split_once('.')?;
    if step.is_empty() || step.len() > 20 || !step.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let tag = URL_SAFE_NO_PAD.decode(tag).ok()?;
    if tag.len() != TAG_BYTES {
        return None;
    }
    step.parse().ok()
}

/// A server's freshness step: the key its values are made of and the length of a step.
#[derive(Clone, PartialEq, Eq)]
pub struct Freshness {
    key: [u8; 32],
    step_ms: u64,
}

impl fmt::Debug for Freshness {
    // The key makes every value; a log that prints it hands out the values of steps to come.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Freshness")
            .field("step_ms", &self.step_ms)
            .finish_non_exhaustive()
    }
}

impl Freshness {
    /// The step of `step` long under `key`; `None` for a step shorter than a millisecond, which
    /// would make every value stale before it arrived.
    pub fn new(key: [u8; 32], step: Duration) -> Option<Self> {
        let step_ms = u64::try_from(step.as_millis()).unwrap_or(u64::MAX);
        (step_ms > 0).then_some(Freshness { key, step_ms })
    }

    /// The step of `step` long under a key drawn from the operating system's random source, through
    /// the cryptography every TLS connection of the bindings runs on ([`crate::tls::provider`]):
    /// a key of the server's own, which it never gives out. A server that starts again draws
    /// another, and each client is refused once and signs the value of the new key.
    #[cfg(feature = "https")]
    pub fn draw(step: Duration) -> Result<Self, crate::tls::Error> {
        let mut key = [0u8; 32];
        crate::tls::provider()
            .secure_random
            .fill(&mut key)
            .map_err(|_| {
                crate::tls::Error::General("the random source could not be read".to_string())
            })?;
        Self::new(key, step).ok_or_else(|| {
            crate::tls::Error::General("a freshness step is a millisecond at least".to_string())
        })
    }

    /// The step the moment `now_ms` (milliseconds since the Unix epoch) is in.
    pub fn step_at(&self, now_ms: u64) -> u64 {
        now_ms / self.step_ms
    }

    /// The value of the step `step` for the name `name`.
    pub fn value_of(&self, name: &str, step: u64) -> String {
        alloc::format!("{step}.{}", URL_SAFE_NO_PAD.encode(self.tag(name, step)))
    }

    /// The value the server gives `name` at the moment `now_ms`: the present step's.
    pub fn value(&self, name: &str, now_ms: u64) -> String {
        self.value_of(name, self.step_at(now_ms))
    }

    /// Whether a request of `name` that signs `carried` (none when it carries no value) is taken
    /// at the moment `now_ms`: a value this server made for that name, of the present step or of
    /// the one before. The tag is compared in constant time.
    pub fn admits(&self, name: &str, carried: Option<&str>, now_ms: u64) -> bool {
        let Some(carried) = carried else {
            return false;
        };
        let Some(step) = step_of(carried) else {
            return false;
        };
        if !admitted(self.step_at(now_ms), step) {
            return false;
        }
        let Some((_, tag)) = carried.trim().split_once('.') else {
            return false;
        };
        URL_SAFE_NO_PAD
            .decode(tag)
            .is_ok_and(|tag| same_secret(&self.tag(name, step), &tag))
    }

    /// The tag of `name` at `step`: the HMAC-SHA-256 under the key of the profile's domain, the
    /// name's length and bytes, and the step, so that no two pairs share the bytes it is made of.
    fn tag(&self, name: &str, step: u64) -> [u8; TAG_BYTES] {
        let mut message = alloc::vec::Vec::with_capacity(DOMAIN.len() + 16 + name.len());
        message.extend_from_slice(DOMAIN);
        message.extend_from_slice(&(name.len() as u64).to_be_bytes());
        message.extend_from_slice(name.as_bytes());
        message.extend_from_slice(&step.to_be_bytes());
        hmac_sha256(&self.key, &message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn freshness() -> Freshness {
        Freshness::new([7u8; 32], Duration::from_secs(60)).expect("a step")
    }

    /// A step's value is taken in that step and the next, and refused from the one after on;
    /// a value of a step to come, of another name, under another key, or none is refused. Red if
    /// `admits` takes a step older than the one before, or reads no tag.
    #[test]
    fn a_value_is_taken_in_its_step_and_the_next_and_no_later() {
        let fresh = freshness();
        let at = |step: u64| step * 60_000 + 59_999;
        let given = fresh.value("phone/a1", at(10));
        assert_eq!(step_of(&given), Some(10));
        assert!(fresh.admits("phone/a1", Some(&given), at(10)), "its step");
        assert!(fresh.admits("phone/a1", Some(&given), at(11)), "the next");
        assert!(!fresh.admits("phone/a1", Some(&given), at(12)), "two on");
        assert!(
            !fresh.admits("phone/a1", Some(&given), at(9)),
            "before it was made"
        );
        assert!(
            !fresh.admits("phone/a2", Some(&given), at(10)),
            "another name"
        );
        let other = Freshness::new([8u8; 32], Duration::from_secs(60)).expect("a step");
        assert!(
            !other.admits("phone/a1", Some(&given), at(10)),
            "another key"
        );
        assert!(!fresh.admits("phone/a1", None, at(10)), "none");
        let forged = alloc::format!("11.{}", given.split_once('.').expect("a tag").1);
        assert!(
            !fresh.admits("phone/a1", Some(&forged), at(11)),
            "a tag of another step"
        );
    }

    /// The text of a value is its step and a tag of 32 bytes, and nothing else reads as one. Red
    /// if `step_of` takes a tag of another length or a step that is no number.
    #[test]
    fn a_value_reads_as_its_step_and_a_whole_tag_and_nothing_else_does() {
        let given = freshness().value("phone", 120_000);
        let (step, tag) = given.split_once('.').expect("a step and a tag");
        assert_eq!(step, "2");
        assert_eq!(tag.len(), 43, "32 bytes are 43 characters without padding");
        assert_eq!(step_of(&given), Some(2));
        for text in ["", "2", "2.", "x.AAAA", ".AAAA", "2.AAAA", "-2.AAAA"] {
            assert_eq!(step_of(text), None, "{text:?}");
        }
        assert_eq!(
            step_of(&alloc::format!("{step}.{tag}AA")),
            None,
            "a longer tag"
        );
    }

    /// A step is a millisecond at least, and the key is never printed. Red if a zero step is
    /// taken or `Debug` shows the key.
    #[test]
    fn a_step_is_a_millisecond_at_least_and_the_key_is_not_printed() {
        assert_eq!(Freshness::new([7u8; 32], Duration::from_micros(999)), None);
        assert_eq!(
            alloc::format!("{:?}", freshness()),
            "Freshness { step_ms: 60000, .. }"
        );
    }

    /// Two steps drawn from the operating system's random source have two keys. Red if `draw`
    /// returns a fixed key. A browser node draws no step, so the check needs the server's build.
    #[cfg(feature = "https")]
    #[test]
    fn two_steps_drawn_have_two_keys() {
        let drawn = Freshness::draw(Duration::from_secs(1)).expect("a key");
        assert_ne!(
            drawn,
            Freshness::draw(Duration::from_secs(1)).expect("a key")
        );
    }
}
