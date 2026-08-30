// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Rust source with its prose removed.
//!
//! Several gates in this suite answer "does this file DO X" by looking for a
//! signature in the file's text. A comment is text, so a file that only TALKS
//! about X answers yes — and the answer is then about the explanation rather
//! than about the code.
//!
//! Measured 2026-08-30, on the tree as it stood: `workflow_trigger_coverage`
//! decides which tests read inputs no `paths:` filter can enumerate by looking
//! for the workflow directory's path in a test's source.
//! `gate_selectors_do_not_leak.rs` names `mutation-rounds.yml` under that
//! directory in its opening comment, to say WHICH caller exports the selectors
//! it holds shut, and reads nothing there at all. The gate demanded it be
//! registered as unfilterable, which would have put a target whose inputs are
//! `sce-build/tests/**` into the workflow that runs on every push — the
//! opposite of what the registry is for. Registered gates: 29. Detected with
//! prose stripped: the same 29.
//!
//! The same reading had already been made once, in the other direction:
//! `gate_selectors_do_not_leak`'s own call-site scan put
//! `workflow_trigger_coverage.rs` into its population because that file names
//! the gate script in a comment, and then failed it for lacking a helper it
//! could never need. Two scanners, one mistake, so the answer lives here once.
//!
//! `code_only` blanks comments in place: every line of the input is a line of
//! the output, so a scan that reports line numbers keeps reporting the input's.
//! String and character literals are preserved verbatim — a `//` inside one is
//! data, and a stripper that ate it would remove code and report a file as not
//! doing what it does.

#![allow(dead_code)]

/// Whether a character can continue a Rust identifier.
fn continues_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The source with every comment removed, line count preserved.
///
/// Handles the four ways a `//` can appear without opening a comment: inside a
/// string literal, inside a raw string literal (whose backslashes are inert),
/// inside a character literal, and inside a block comment being closed. Block
/// comments nest, as they do in the language.
pub fn code_only(src: &str) -> String {
    let s: Vec<char> = src.chars().collect();
    let n = s.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;

    while i < n {
        // A line comment runs to the newline, which the next turn of the loop
        // copies — that is what keeps the output's line numbering the input's.
        if s[i] == '/' && i + 1 < n && s[i + 1] == '/' {
            while i < n && s[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // A block comment nests, and its newlines are kept for the same reason.
        if s[i] == '/' && i + 1 < n && s[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if s[i] == '/' && i + 1 < n && s[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && i + 1 < n && s[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if s[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // A raw string ends at a quote followed by as many `#` as opened it,
        // and nothing inside it escapes anything.
        if (s[i] == 'r' || s[i] == 'b') && (i == 0 || !continues_ident(s[i - 1])) {
            if let Some(hashes) = raw_string_hashes_at(&s, i) {
                let body = i + if s[i] == 'b' { 2 } else { 1 } + hashes + 1;
                out.extend(&s[i..body]);
                i = body;
                while i < n {
                    if s[i] == '"' && run_of_hashes(&s, i + 1) >= hashes {
                        out.extend(&s[i..=i + hashes]);
                        i += hashes + 1;
                        break;
                    }
                    out.push(s[i]);
                    i += 1;
                }
                continue;
            }
        }

        if s[i] == '"' {
            out.push('"');
            i += 1;
            while i < n {
                if s[i] == '\\' && i + 1 < n {
                    out.push(s[i]);
                    out.push(s[i + 1]);
                    i += 2;
                    continue;
                }
                out.push(s[i]);
                i += 1;
                if s[i - 1] == '"' {
                    break;
                }
            }
            continue;
        }

        // A quote opens a character literal only when it closes two characters
        // later or opens an escape. Otherwise it is a lifetime, and reading
        // `&'a str` as an unterminated literal would swallow every comment up
        // to the next quote in the file.
        if s[i] == '\'' && char_literal_opens_at(&s, i) {
            out.push('\'');
            i += 1;
            while i < n {
                if s[i] == '\\' && i + 1 < n {
                    out.push(s[i]);
                    out.push(s[i + 1]);
                    i += 2;
                    continue;
                }
                out.push(s[i]);
                i += 1;
                if s[i - 1] == '\'' {
                    break;
                }
            }
            continue;
        }

        out.push(s[i]);
        i += 1;
    }
    out
}

/// How many `#` a raw string opening at `i` carries, if one opens there.
///
/// `i` is at `r`, or at the `b` of `br`; anything else — a plain `b"..."`, an
/// `r` that begins an identifier — is not a raw string and is left to the
/// plain-string arm.
fn raw_string_hashes_at(s: &[char], i: usize) -> Option<usize> {
    let mut j = i;
    if s[j] == 'b' {
        j += 1;
    }
    if j >= s.len() || s[j] != 'r' {
        return None;
    }
    j += 1;
    let hashes = run_of_hashes(s, j);
    let quote = j + hashes;
    (quote < s.len() && s[quote] == '"').then_some(hashes)
}

/// How many `#` run from `i`.
fn run_of_hashes(s: &[char], i: usize) -> usize {
    let mut k = i;
    while k < s.len() && s[k] == '#' {
        k += 1;
    }
    k - i
}

/// Whether the quote at `i` opens a character literal rather than a lifetime.
fn char_literal_opens_at(s: &[char], i: usize) -> bool {
    if i + 1 < s.len() && s[i + 1] == '\\' {
        return true;
    }
    i + 2 < s.len() && s[i + 2] == '\''
}
