// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The id of a root session, the value of the system variable `_sessionid`
//! (docs/adr/0045, "Questions the owner answered" 2).
//!
//! `_sessionid` is bound when a session is loaded and kept until it ends, and
//! it is an XML Nmtoken: another session addresses this one as
//! `#_scxml_<id>`, and every child's id is its parent's and the id the child
//! was started under, `<parent>.<invokeid>`. So a root's id is never empty.
//! The host may give it when it creates the machine ([`checked`], and
//! `given` where there is an allocator), and is told when what it gives is
//! not an Nmtoken rather than having it replaced; where it gives none the
//! runtime draws one (`drawn`), a UUID version 4 in lowercase hexadecimal
//! with hyphens, the one spelling every backend draws, so no reader can tell
//! which backend drew it. A restore binds the saved id instead of either,
//! since the session carries on.
//!
//! A target with no source of randomness the runtime can read
//! (`wasm32-unknown-unknown`, a bare-metal one, any `no_std` build) draws
//! nothing: the generated machine there has no constructor that does not take
//! the id, so its host gives it, as it gives such a target the wall clock.

use core::fmt;

/// Why an id a host gave for a session was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionIdRefusal {
    /// The empty text: a session id is one name character or more.
    Empty,
    /// The id holds this character, which is not an XML name character.
    NotANameCharacter(char),
}

impl fmt::Display for SessionIdRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str(
                "the session id is empty, and a session id is one name character or more \
                 (an XML Nmtoken)",
            ),
            Self::NotANameCharacter(c) => write!(
                f,
                "the session id holds {c:?}, which is not a name character: a session id is \
                 an XML Nmtoken (letters, digits, '.', '-', '_', ':')"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SessionIdRefusal {}

/// Whether `id` is an XML 1.0 Nmtoken: one name character or more
/// (production `Nmtoken`, fifth edition).
pub fn is_nmtoken(id: &str) -> bool {
    checked(id).is_ok()
}

/// XML 1.0 `NameStartChar`.
const fn is_name_start_char(c: char) -> bool {
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
const fn is_name_char(c: char) -> bool {
    is_name_start_char(c)
        || matches!(
            c,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}

/// The character `bytes` holds at `at`, and how many bytes it takes.
///
/// `bytes` is the text of a `&str`, so it is well-formed UTF-8 and `at` is the
/// start of a character: the lead byte says how many bytes follow, each
/// carrying six bits of the scalar value. A function of its own because
/// `str::chars` is not a constant expression, and an id a program holds in a
/// `static` is checked while it is compiled ([`checked`]).
const fn character_at(bytes: &[u8], at: usize) -> (char, usize) {
    let lead = bytes[at];
    let (len, mut scalar) = if lead < 0x80 {
        (1, lead as u32)
    } else if lead < 0xE0 {
        (2, (lead & 0x1F) as u32)
    } else if lead < 0xF0 {
        (3, (lead & 0x0F) as u32)
    } else {
        (4, (lead & 0x07) as u32)
    };
    let mut i = 1;
    while i < len {
        scalar = (scalar << 6) | (bytes[at + i] & 0x3F) as u32;
        i += 1;
    }
    match char::from_u32(scalar) {
        Some(c) => (c, len),
        // The text of a `&str` holds scalar values only. (A message here would
        // be a formatting call, which a constant function cannot make.)
        None => unreachable!(),
    }
}

/// `id`, given by a host for a session it creates, if it is an Nmtoken; the
/// refusal that says why it is not one otherwise. Nothing replaces an id
/// refused: the host that gave it is told. Allocates nothing, so a `no_std`
/// machine holds the very text its host gave.
///
/// A constant expression: a machine built in a `static`
/// (`Engine::new_const` and the generated `new_const`) has its id checked
/// while the program is compiled, and an id that is not an Nmtoken is a
/// compile error and not a machine that starts without one.
pub const fn checked(id: &str) -> Result<&str, SessionIdRefusal> {
    let bytes = id.as_bytes();
    if bytes.is_empty() {
        return Err(SessionIdRefusal::Empty);
    }
    let mut at = 0;
    while at < bytes.len() {
        let (c, len) = character_at(bytes, at);
        if !is_name_char(c) {
            return Err(SessionIdRefusal::NotANameCharacter(c));
        }
        at += len;
    }
    Ok(id)
}

/// [`checked`], owned: the id a machine with an allocator holds.
#[cfg(feature = "std")]
pub fn given(id: &str) -> Result<String, SessionIdRefusal> {
    checked(id).map(str::to_string)
}

/// A session id drawn from the operating system's randomness, a UUID version
/// 4, for a session whose host gave none. Compiled only where there is a
/// source of randomness the runtime reads; the generated `new` that calls it
/// is compiled under the same predicate.
///
/// # Panics
///
/// When the operating system gives no randomness at all, which a target that
/// has a source does not do: an id made up in its place could be one another
/// session already holds.
#[cfg(all(
    feature = "std",
    not(any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown")))
))]
pub fn drawn() -> String {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes)
        .expect("the operating system gave no randomness to draw a session id from");
    uuid_v4(bytes)
}

/// `bytes` as a UUID version 4 (RFC 9562, section 5.4): the version in the
/// high half of the seventh byte, the variant in the top two bits of the
/// ninth, written in lowercase hexadecimal in groups of 8, 4, 4, 4 and 12.
#[cfg(feature = "std")]
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

#[cfg(all(test, feature = "std"))]
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
            assert_eq!(checked(id), Ok(id));
            assert_eq!(given(id).as_deref(), Ok(id));
        }
        for id in ["", "a b", "a#b", "a/b", "<a>", "a\u{d7}b", "tab\t"] {
            assert!(!is_nmtoken(id), "{id:?}");
            assert!(checked(id).is_err(), "{id:?} was taken");
            assert!(given(id).is_err(), "{id:?} was given");
        }
    }

    #[test]
    fn a_refusal_says_what_is_wrong_with_the_id() {
        assert_eq!(checked(""), Err(SessionIdRefusal::Empty));
        assert!(SessionIdRefusal::Empty.to_string().contains("empty"));
        let spaced = checked("planner 1").expect_err("an id with a space");
        assert_eq!(spaced, SessionIdRefusal::NotANameCharacter(' '));
        assert!(spaced.to_string().contains("' '"), "{spaced}");
        assert!(spaced.to_string().contains("Nmtoken"), "{spaced}");
    }

    #[test]
    fn a_character_of_every_length_is_judged_as_the_character_it_is() {
        // One name character and one that is not, for each length UTF-8 gives
        // a scalar value, read against `str::chars`, which the constant
        // expression replaces.
        for (name_char, other) in [
            ('a', ' '),
            ('\u{e9}', '\u{d7}'),
            ('\u{3b1}', '\u{37e}'),
            ('\u{4e2d}', '\u{2190}'),
            ('\u{10000}', '\u{f0000}'),
        ] {
            assert!(is_name_char(name_char) && !is_name_char(other));
            let alone = name_char.to_string();
            assert_eq!(checked(&alone), Ok(alone.as_str()), "{name_char:?}");
            let tail = format!("a{name_char}b{name_char}");
            assert_eq!(checked(&tail), Ok(tail.as_str()), "{tail:?}");
            let broken = format!("a{name_char}{other}b");
            let expected = broken.chars().find(|c| !is_name_char(*c));
            assert_eq!(
                checked(&broken),
                expected.map_or(Ok(broken.as_str()), |c| Err(
                    SessionIdRefusal::NotANameCharacter(c)
                )),
                "{broken:?}"
            );
        }
    }

    #[test]
    fn a_checked_id_is_the_text_given_not_a_copy() {
        let id: &'static str = "device-7";
        let held = checked(id).expect("an Nmtoken");
        assert!(core::ptr::eq(held, id));
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
        let one = drawn();
        let two = drawn();
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
