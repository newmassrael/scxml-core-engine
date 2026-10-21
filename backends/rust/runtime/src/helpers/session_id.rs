// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The id of a root session, the value of the system variable `_sessionid`
//! (docs/adr/0045, "Questions the owner answered" 2).
//!
//! `_sessionid` is bound when a session is loaded and kept until it ends, and
//! it is an XML Nmtoken: another session addresses this one as
//! `#_scxml_<id>`, and every child's id is its parent's and the id the child
//! was started under, `<parent>.<invokeid>`. So a root's id is never empty.
//! The host may give it when it creates the machine ([`given`]), and is told
//! when what it gives is not an Nmtoken rather than having it replaced; where
//! it gives none the runtime draws one ([`drawn`]), a UUID version 4 in
//! lowercase hexadecimal with hyphens, the one spelling every backend draws,
//! so no reader can tell which backend drew it. A restore binds the saved id
//! instead of either, since the session carries on.
//!
//! A target with no source of randomness the runtime can read
//! (`wasm32-unknown-unknown`, a bare-metal one) draws nothing: its host gives
//! the id, as it gives such a target the wall clock.

use core::fmt;

/// Why an id a host gave for a session was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionIdRefusal(String);

impl SessionIdRefusal {
    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionIdRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SessionIdRefusal {}

/// Whether `id` is an XML 1.0 Nmtoken: one name character or more
/// (production `Nmtoken`, fifth edition).
pub fn is_nmtoken(id: &str) -> bool {
    !id.is_empty() && id.chars().all(is_name_char)
}

/// XML 1.0 `NameStartChar`.
fn is_name_start_char(c: char) -> bool {
    matches!(
        c,
        ':' | 'A'..='Z'
            | '_'
            | 'a'..='z'
            | '\u{C0}'..='\u{D6}'
            | '\u{D8}'..='\u{F6}'
            | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}'
            | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
    )
}

/// XML 1.0 `NameChar`.
fn is_name_char(c: char) -> bool {
    is_name_start_char(c)
        || matches!(
            c,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}

/// `id`, given by a host for a session it creates, if it is an Nmtoken; the
/// refusal that says why it is not one otherwise. Nothing replaces an id
/// refused: the host that gave it is told.
pub fn given(id: &str) -> Result<String, SessionIdRefusal> {
    if id.is_empty() {
        return Err(SessionIdRefusal(
            "the session id is empty, and a session id is one name character or more \
             (an XML Nmtoken)"
                .to_string(),
        ));
    }
    match id.chars().find(|c| !is_name_char(*c)) {
        None => Ok(id.to_string()),
        Some(c) => Err(SessionIdRefusal(format!(
            "the session id '{id}' holds {c:?}, which is not a name character: a session id \
             is an XML Nmtoken (letters, digits, '.', '-', '_', ':')"
        ))),
    }
}

/// A session id drawn from the operating system's randomness, a UUID version
/// 4, for a session whose host gave none. `None` on a target with no source
/// of randomness the runtime reads, whose host gives the id.
///
/// # Panics
///
/// When the operating system gives no randomness at all, which a target that
/// has a source does not do: an id made up in its place could be one another
/// session already holds.
pub fn drawn() -> Option<String> {
    #[cfg(not(any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown"))))]
    {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes)
            .expect("the operating system gave no randomness to draw a session id from");
        Some(uuid_v4(bytes))
    }
    #[cfg(any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown")))]
    {
        None
    }
}

/// `bytes` as a UUID version 4 (RFC 9562, section 5.4): the version in the
/// high half of the seventh byte, the variant in the top two bits of the
/// ninth, written in lowercase hexadecimal in groups of 8, 4, 4, 4 and 12.
#[cfg_attr(
    any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown")),
    allow(dead_code)
)]
fn uuid_v4(mut bytes: [u8; 16]) -> String {
    use fmt::Write as _;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut text = String::with_capacity(36);
    for (i, byte) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_nmtoken_is_name_characters_and_nothing_else() {
        for id in [
            "planner-1",
            "dev:7",
            "a.b_c-d",
            "0leading-digit",
            "\u{e9}tat",
            "s.working.worker.2",
        ] {
            assert!(is_nmtoken(id), "{id:?}");
            assert_eq!(given(id).as_deref(), Ok(id));
        }
        for id in ["", "a b", "a#b", "a/b", "<a>", "a\u{d7}b", "tab\t"] {
            assert!(!is_nmtoken(id), "{id:?}");
            assert!(given(id).is_err(), "{id:?} was given");
        }
    }

    #[test]
    fn a_refusal_says_what_is_wrong_with_the_id() {
        let empty = given("").expect_err("the empty id");
        assert!(empty.reason().contains("empty"), "{empty}");
        let spaced = given("planner 1").expect_err("an id with a space");
        assert!(spaced.reason().contains("' '"), "{spaced}");
    }

    #[test]
    fn a_uuid_v4_is_its_bytes_with_the_version_and_variant_set() {
        let bytes: [u8; 16] = core::array::from_fn(|i| i as u8);
        assert_eq!(uuid_v4(bytes), "00010203-0405-4607-8809-0a0b0c0d0e0f");
        assert_eq!(uuid_v4([0xff; 16]), "ffffffff-ffff-4fff-bfff-ffffffffffff");
    }

    #[cfg(not(any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown"))))]
    #[test]
    fn a_drawn_id_is_a_uuid_v4_and_two_draws_differ() {
        let one = drawn().expect("this target has a source of randomness");
        let two = drawn().expect("this target has a source of randomness");
        assert_ne!(one, two);
        for id in [&one, &two] {
            assert_eq!(id.len(), 36, "{id}");
            assert!(is_nmtoken(id), "{id}");
            assert_eq!(&id[14..15], "4", "the version: {id}");
            assert!(
                matches!(&id[19..20], "8" | "9" | "a" | "b"),
                "the variant: {id}"
            );
            assert!(
                id.chars()
                    .all(|c| c == '-' || c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "lowercase hexadecimal: {id}"
            );
        }
    }
}
