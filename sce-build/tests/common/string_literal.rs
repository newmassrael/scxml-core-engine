// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The text a generated string literal's body compiles to, read through the
//! escapes the generator's escaper for that language writes
//! (`sce_build::filters`).
//!
//! A test that reads a value back out of generated source, rather than by
//! running the program, has to undo those escapes as the language's compiler
//! does, and the languages do not agree: C and C++ write a character past ASCII
//! as the octal escapes of its UTF-8 bytes, where the other backends write it
//! as itself. One reader for every backend reads none of them right, so each
//! grammar here is the set of escapes one escaper writes, and an escape outside
//! it panics: a reader that guessed would hand the comparison a value no
//! compiler builds.

#![allow(dead_code)]

/// The escapes one backend's string-literal escaper writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escapes {
    /// `escape_c`, which `escape_cpp` forwards to: `\\`, `\"`, `\n`, `\r`,
    /// `\t`, `\?`, and every other byte as a three-digit octal escape. An
    /// octal escape is a byte, not a character, so the bytes of a character
    /// past ASCII are read back into it as UTF-8.
    CFamily,
    /// `escape_rust`: `\\`, `\"`, `\n`, `\r`, `\t`, `\0` and `\u{...}`.
    Rust,
    /// `escape_go`: `\\`, `\"`, `\n`, `\r`, `\t` and `\uXXXX`.
    Go,
    /// `escape_kotlin`: Go's escapes and `\$`.
    Kotlin,
    /// `escape_swift`: `\\`, `\"`, `\n`, `\r`, `\t`, `\0` and `\u{...}`.
    Swift,
    /// `escape_python`: `\\`, `\"`, `\n`, `\r`, `\t` and `\xHH`, which in a
    /// Python text is the character of that code point.
    Python,
}

impl Escapes {
    /// The escapes of the backend `generate -l` names `lang`.
    pub fn of(lang: &str) -> Escapes {
        match lang {
            "cpp" | "c11" => Escapes::CFamily,
            "rust" => Escapes::Rust,
            "go" => Escapes::Go,
            "kotlin" => Escapes::Kotlin,
            "swift" => Escapes::Swift,
            "python" => Escapes::Python,
            other => panic!("no string-literal escapes are known for {other}"),
        }
    }
}

/// The text the body of a string literal (the characters between its quotes)
/// compiles to, read through `escapes`.
pub fn literal_value(escapes: Escapes, body: &str) -> String {
    let mut bytes = Vec::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            push_char(&mut bytes, ch);
            continue;
        }
        let escape = chars
            .next()
            .unwrap_or_else(|| panic!("a literal ends inside an escape: `{body}`"));
        match (escapes, escape) {
            (_, '\\') => bytes.push(b'\\'),
            (_, '"') => bytes.push(b'"'),
            (_, 'n') => bytes.push(b'\n'),
            (_, 'r') => bytes.push(b'\r'),
            (_, 't') => bytes.push(b'\t'),
            (Escapes::CFamily, '?') => bytes.push(b'?'),
            (Escapes::CFamily, first @ '0'..='7') => {
                let digits: String = std::iter::once(first)
                    .chain(chars.by_ref().take(2))
                    .collect();
                let byte = u8::from_str_radix(&digits, 8).unwrap_or_else(|_| {
                    panic!("an octal escape of three digits naming a byte, not `\\{digits}` in `{body}`")
                });
                bytes.push(byte);
            }
            (Escapes::Rust | Escapes::Swift, '0') => bytes.push(0),
            (Escapes::Rust | Escapes::Swift, 'u') => {
                assert_eq!(
                    chars.next(),
                    Some('{'),
                    "a `\\u` escape opens a brace in `{body}`"
                );
                let hex: String = chars.by_ref().take_while(|c| *c != '}').collect();
                push_char(&mut bytes, code_point(&hex, body));
            }
            (Escapes::Go | Escapes::Kotlin, 'u') => {
                let hex: String = chars.by_ref().take(4).collect();
                push_char(&mut bytes, code_point(&hex, body));
            }
            (Escapes::Kotlin, '$') => bytes.push(b'$'),
            (Escapes::Python, 'x') => {
                let hex: String = chars.by_ref().take(2).collect();
                push_char(&mut bytes, code_point(&hex, body));
            }
            (_, other) => {
                panic!("an escape the {escapes:?} escaper does not write: `\\{other}` in `{body}`")
            }
        }
    }
    String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("a literal whose bytes are UTF-8 ({error}): `{body}`"))
}

fn push_char(bytes: &mut Vec<u8>, ch: char) {
    let mut utf8 = [0u8; 4];
    bytes.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
}

/// The character `hex` names by its code point.
fn code_point(hex: &str, body: &str) -> char {
    u32::from_str_radix(hex, 16)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or_else(|| panic!("`{hex}` names no character in `{body}`"))
}
