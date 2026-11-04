// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a CMake comment ends and CMake code begins.
//!
//! Several suites answer "does this CMake file DO X" by reading its text, and
//! the files document what they do in prose: a comment naming a command is
//! not a call of it. `the_conformance_cmake_route_declares_what_the_generator_plans`
//! wrote the first reading of CMake comments, and `w3c_registry_drives_cmake`
//! needs the same one, so it lives here once rather than as two readings of
//! one grammar that can come to disagree about a bracket comment.

#![allow(dead_code)]

/// The text of a CMake file with its comments removed and its line numbering
/// kept: a line comment runs from an unquoted `#` to the end of the line, a
/// bracket comment from `#[[` (or `#[=[`, with any number of `=`) to the
/// matching close. A `#` inside a quoted argument is text (`"#_parent"`).
pub fn cmake_code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut quoted = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if quoted {
            out.push(c);
            if c == '\\' {
                if let Some(&escaped) = chars.get(i + 1) {
                    out.push(escaped);
                    i += 2;
                    continue;
                }
            } else if c == '"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                out.push(c);
                i += 1;
            }
            '#' => {
                let end = bracket_comment_end(&chars, i + 1).unwrap_or_else(|| {
                    chars[i..]
                        .iter()
                        .position(|&ch| ch == '\n')
                        .map_or(chars.len(), |offset| i + offset)
                });
                out.extend(chars[i..end].iter().filter(|&&ch| ch == '\n'));
                i = end;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// `code` (comments already removed, [`cmake_code`]) with the inside of every
/// quoted argument turned into spaces, the quotes and every newline kept: what
/// is left is the file's structure. Its line numbering is `code`'s and so is
/// its length in characters (a character becomes one space, whatever its width
/// in bytes), so scan this and read a line of it back from `code` by number,
/// not by byte offset. A command named inside
/// a string (`message("if(...)")`) is text, and a scanner that read it would
/// open a block nobody wrote.
pub fn strings_blanked(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut quoted = false;
    let mut escaped = false;
    for c in code.chars() {
        if !quoted {
            if c == '"' {
                quoted = true;
            }
            out.push(c);
            continue;
        }
        if escaped {
            escaped = false;
            out.push(if c == '\n' { '\n' } else { ' ' });
            continue;
        }
        match c {
            '\\' => {
                escaped = true;
                out.push(' ');
            }
            '"' => {
                quoted = false;
                out.push('"');
            }
            '\n' => out.push('\n'),
            _ => out.push(' '),
        }
    }
    out
}

/// Where a bracket comment opened at `at` (just past its `#`) ends, or `None`
/// when what follows the `#` opens no bracket.
fn bracket_comment_end(chars: &[char], at: usize) -> Option<usize> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    let mut j = at + 1;
    let mut level = 0;
    while chars.get(j) == Some(&'=') {
        level += 1;
        j += 1;
    }
    if chars.get(j) != Some(&'[') {
        return None;
    }
    let mut closing = vec![']'; level + 2];
    closing[1..=level].fill('=');
    let body = j + 1;
    chars[body..]
        .windows(closing.len())
        .position(|window| window == closing.as_slice())
        .map(|offset| body + offset + closing.len())
}
