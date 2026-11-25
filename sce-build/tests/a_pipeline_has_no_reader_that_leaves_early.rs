// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// No tracked shell script hands a pipe to a reader that leaves before its
// writer is done: `head`, or a `grep` that stops at a match (`-q`, `-m`, `-l`, `-L`,
// their long spellings, and the same flags of `rg`).
//
// Such a reader closes the pipe when it has what it wants, and whether the
// writer has finished writing by then is a matter of timing: a writer that
// has not dies of SIGPIPE. Under `set -o pipefail`, which every gate runs
// with (scripts/gates/lib.sh), that death is the pipeline's status, so the
// same input gives a verdict on one run and another on the next. Measured
// where it was met: `cpp-suite` went red with no test failing
// (`sort | head -1`, 2026-08-26); the consumer-guide hook read a waiver as
// absent in 420 of 2880 runs under load (`git log | grep -q`); the endpoint
// gate missed the one violation in a file of 8000 lines (`printf | grep -q`);
// and the regeneration gate ended with `printf: write error: Broken pipe` before
// it said which file differed (jobs 135410162303, 135456764853, 135426659426,
// 135477004902). Each was mended where it was found, and the next one was
// written somewhere else, so the rule is held here over every script.
//
// What is written instead reads all of what it is handed or reads a file of
// its own: a herestring (`grep -q pat <<<"$text"`), a line range in `sed`
// (`sed -n '1,10p'`), a count the reader of a file stops at itself
// (`grep -m 20 pat file`, `find ... -print -quit`), or the first line taken in
// the shell (`${text%%$'\n'*}`).

mod common;

/// The fewest pipes the sweep must find, and the fewest scripts it must
/// read: a scan that finds nothing certifies nothing. The tree held 375
/// pipes in 172 scripts when this was written.
const MIN_PIPES: usize = 200;
const MIN_SCRIPTS: usize = 100;

/// Whether the tracked file `path`, whose text is `text`, is a shell script: a
/// `.sh` or `.bash` file, or a file of no extension that starts with a shell's
/// `#!` line (`scripts/mutate`, the git hooks, `gradlew`).
fn is_shell_script(path: &str, text: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    if name.ends_with(".sh") || name.ends_with(".bash") {
        return true;
    }
    if name.contains('.') {
        return false;
    }
    let first = text.lines().next().unwrap_or("");
    first.starts_with("#!")
        && (first.contains("bash") || first.ends_with("/sh") || first.ends_with(" sh"))
}

/// What a byte of a script is to the scan.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The script's own words.
    Code,
    /// Inside `'...'`.
    Single,
    /// Inside `"..."`.
    Double,
    /// Inside `$(...)`, which is code wherever it stands.
    Substitution,
    /// A `(` opened inside a substitution, closed by its `)`.
    Group,
    /// Inside `((...))`, where `|` is an or and `<<` a shift.
    Arithmetic,
}

/// `text` with every byte that is not the script's own code made a
/// placeholder of one byte, so that offsets are the text's: the contents of
/// quotes (but a `$(...)` in double quotes, which is code), comments, the
/// bodies of here-documents, arithmetic, and an escaped character. A
/// backslash before a newline joins the two lines, as the shell does.
fn code_of(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let at_byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut stack = vec![Mode::Code];
    let mut arithmetic_depth = 0usize;
    // The delimiters of the here-documents a line opened, and whether each
    // may be indented with tabs (`<<-`); their bodies start at the next line.
    let mut pending: Vec<(String, bool)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let mode = *stack.last().unwrap_or(&Mode::Code);
        if b == b'\n' && mode != Mode::Single && mode != Mode::Double && !pending.is_empty() {
            out.push(b'\n');
            i += 1;
            for (word, tabs) in std::mem::take(&mut pending) {
                // Each body line becomes blank, to and with its delimiter.
                while i < bytes.len() {
                    let end = bytes[i..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map_or(bytes.len(), |n| i + n);
                    let line = &text[i..end];
                    let line = if tabs {
                        line.trim_start_matches('\t')
                    } else {
                        line
                    };
                    out.resize(out.len() + (end - i), b' ');
                    i = end;
                    if i < bytes.len() {
                        out.push(b'\n');
                        i += 1;
                    }
                    if line == word {
                        break;
                    }
                }
            }
            continue;
        }
        match mode {
            Mode::Single => {
                if b == b'\'' {
                    stack.pop();
                }
                out.push(if b == b'\n' { b'\n' } else { b'Q' });
            }
            Mode::Double => {
                if b == b'\\' && i + 1 < bytes.len() {
                    out.extend_from_slice(b"QQ");
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    stack.pop();
                    out.push(b'Q');
                } else if b == b'$' && at_byte(i + 1) == b'(' && at_byte(i + 2) == b'(' {
                    stack.push(Mode::Arithmetic);
                    arithmetic_depth = 0;
                    out.extend_from_slice(b"$((");
                    i += 3;
                    continue;
                } else if b == b'$' && at_byte(i + 1) == b'(' {
                    stack.push(Mode::Substitution);
                    out.extend_from_slice(b"$(");
                    i += 2;
                    continue;
                } else {
                    out.push(if b == b'\n' { b'\n' } else { b'Q' });
                }
            }
            Mode::Arithmetic => {
                if b == b'(' {
                    arithmetic_depth += 1;
                } else if b == b')' && arithmetic_depth > 0 {
                    arithmetic_depth -= 1;
                } else if b == b')' && at_byte(i + 1) == b')' {
                    stack.pop();
                    out.extend_from_slice(b"))");
                    i += 2;
                    continue;
                }
                out.push(if b == b'\n' { b'\n' } else { b'A' });
            }
            Mode::Code | Mode::Substitution | Mode::Group => match b {
                b'\'' => {
                    stack.push(Mode::Single);
                    out.push(b'Q');
                }
                b'"' => {
                    stack.push(Mode::Double);
                    out.push(b'Q');
                }
                b'\\' => {
                    out.push(b'_');
                    if i + 1 < bytes.len() {
                        out.push(if bytes[i + 1] == b'\n' { b' ' } else { b'_' });
                    }
                    i += 2;
                    continue;
                }
                b'#' if i == 0 || matches!(bytes[i - 1], b' ' | b'\t' | b'\n' | b';' | b'(') => {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        out.push(b' ');
                        i += 1;
                    }
                    continue;
                }
                b'(' if at_byte(i + 1) == b'(' => {
                    stack.push(Mode::Arithmetic);
                    arithmetic_depth = 0;
                    out.extend_from_slice(b"((");
                    i += 2;
                    continue;
                }
                b'$' if at_byte(i + 1) == b'(' && at_byte(i + 2) == b'(' => {
                    stack.push(Mode::Arithmetic);
                    arithmetic_depth = 0;
                    out.extend_from_slice(b"$((");
                    i += 3;
                    continue;
                }
                b'$' if at_byte(i + 1) == b'(' => {
                    stack.push(Mode::Substitution);
                    out.extend_from_slice(b"$(");
                    i += 2;
                    continue;
                }
                b'(' if mode != Mode::Code => {
                    stack.push(Mode::Group);
                    out.push(b);
                }
                b')' if mode != Mode::Code => {
                    stack.pop();
                    out.push(b);
                }
                b'<' if at_byte(i + 1) == b'<'
                    && at_byte(i + 2) != b'<'
                    && (i == 0 || bytes[i - 1] != b'<') =>
                {
                    let mut j = i + 2;
                    let tabs = at_byte(j) == b'-';
                    if tabs {
                        j += 1;
                    }
                    while matches!(at_byte(j), b' ' | b'\t' | b'\'' | b'"' | b'\\') {
                        j += 1;
                    }
                    let start = j;
                    while at_byte(j).is_ascii_alphanumeric() || at_byte(j) == b'_' {
                        j += 1;
                    }
                    if j > start {
                        pending.push((text[start..j].to_string(), tabs));
                    }
                    out.extend_from_slice(b"<<");
                    i += 2;
                    continue;
                }
                _ => out.push(b),
            },
        }
        i += 1;
    }
    out
}

/// The flags of a `grep` or an `rg` that stop it reading its input before the
/// end: at the first match (`-q`, `-l`, `-L`) or at a count (`-m`).
fn stops_early(argument: &str) -> bool {
    if let Some(long) = argument.strip_prefix("--") {
        return [
            "quiet",
            "silent",
            "max-count",
            "files-with-matches",
            "files-without-match",
        ]
        .iter()
        .any(|flag| long == *flag || long.starts_with(&format!("{flag}=")));
    }
    argument.strip_prefix('-').is_some_and(|short| {
        short
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .any(|flag| matches!(flag, 'q' | 'm' | 'l' | 'L'))
    })
}

/// A reader of a pipe that leaves before its writer is done.
#[derive(Debug, PartialEq, Eq)]
struct Early {
    /// The 1-based line of the `|` that hands it the pipe.
    line: usize,
    /// The reader as written: its name and its flags.
    reader: String,
}

/// How many pipes `text` has, and each whose reader leaves early.
fn early_readers(text: &str) -> (usize, Vec<Early>) {
    let code = code_of(text);
    let at = |i: usize| code.get(i).copied().unwrap_or(0);
    let mut pipes = 0;
    let mut found = Vec::new();
    for (i, &b) in code.iter().enumerate() {
        if b != b'|' || at(i + 1) == b'|' || (i > 0 && code[i - 1] == b'|') {
            continue;
        }
        pipes += 1;
        // The reader's words, to the end of its command.
        let mut j = i + 1;
        if at(j) == b'&' {
            j += 1;
        }
        let end = code[j..]
            .iter()
            .position(|c| matches!(c, b'|' | b';' | b'&' | b')' | b'\n' | b'>' | b'<'))
            .map_or(code.len(), |n| j + n);
        let command = String::from_utf8_lossy(&code[j..end]);
        let mut words = command
            .split_whitespace()
            .skip_while(|word| *word == "command" || is_assignment(word));
        let Some(name) = words.next() else {
            continue;
        };
        let name = name.rsplit('/').next().unwrap_or(name);
        let flags: Vec<&str> = words.filter(|word| word.starts_with('-')).collect();
        let leaves = match name {
            "head" => true,
            "grep" | "egrep" | "fgrep" | "rg" => flags.iter().any(|flag| stops_early(flag)),
            _ => false,
        };
        if leaves {
            let line = text.as_bytes()[..i].iter().filter(|&&c| c == b'\n').count() + 1;
            found.push(Early {
                line,
                reader: std::iter::once(name)
                    .chain(flags)
                    .collect::<Vec<_>>()
                    .join(" "),
            });
        }
    }
    (pipes, found)
}

/// `NAME=value`, an assignment a command is prefixed with.
fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

#[test]
fn no_tracked_script_hands_a_pipe_to_a_reader_that_leaves_early() {
    let root = common::repository::root();
    let mut scripts = 0usize;
    let mut pipes = 0usize;
    let mut offenders = Vec::new();
    for path in common::repository::paths_git_tracks(&[]) {
        // A name with another extension is no script; only the rest is read.
        let name = path.rsplit('/').next().unwrap_or(&path);
        if name.contains('.') && !name.ends_with(".sh") && !name.ends_with(".bash") {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if !is_shell_script(&path, &text) {
            continue;
        }
        scripts += 1;
        let (count, early) = early_readers(&text);
        pipes += count;
        offenders.extend(
            early
                .into_iter()
                .map(|early| format!("{path}:{}: | {}", early.line, early.reader)),
        );
    }
    assert!(
        offenders.is_empty(),
        "{} pipe(s) are handed to a reader that leaves before its writer is done, so the \
         writer may die of SIGPIPE and, under pipefail, the pipeline's status is a matter of \
         timing. Read the whole input instead (a herestring, `sed -n '1,Np'`, the first line \
         taken in the shell), or let the reader of a file stop itself (`grep -m N pat file`, \
         `find ... -print -quit`):\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
    assert!(
        scripts >= MIN_SCRIPTS && pipes >= MIN_PIPES,
        "{scripts} script(s) and {pipes} pipe(s) read; expected at least {MIN_SCRIPTS} and \
         {MIN_PIPES}. A scan that finds nothing certifies nothing."
    );
}

/// Each reader that leaves early is found, with the line its pipe is on.
#[test]
fn a_reader_that_leaves_early_is_found_where_its_pipe_is() {
    let script = "#!/usr/bin/env bash\n\
                  set -euo pipefail\n\
                  first=\"$(printf '%s\\n' \"$x\" | head -1)\"\n\
                  if git log | grep -Eq \"$waiver\"; then :; fi\n\
                  sort -rn \\\n    | head -n 1\n\
                  out | grep -m1 x; out | LC_ALL=C rg --quiet y; out |& grep -l z\n";

    let (pipes, early) = early_readers(script);

    let found: Vec<(usize, &str)> = early.iter().map(|e| (e.line, e.reader.as_str())).collect();
    assert_eq!(
        found,
        vec![
            (3, "head -1"),
            (4, "grep -Eq"),
            (6, "head -n"),
            (7, "grep -m1"),
            (7, "rg --quiet"),
            (7, "grep -l"),
        ]
    );
    assert_eq!(pipes, 6);
}

/// What reads to the end of its input, and what is no pipe at all, is not
/// found: an or, a quoted bar, a comment, a here-document's body, arithmetic,
/// a herestring, and a reader of a file.
#[test]
fn what_reads_all_of_its_input_or_is_no_pipe_is_not_found() {
    let script = "#!/usr/bin/env bash\n\
                  a || head -1 file\n\
                  grep -E 'x|head' file | sed -n '1,10p'\n\
                  # a | head -1\n\
                  python3 - <<'PY'\n\
                  print(a | head)\n\
                  PY\n\
                  n=$(( 1 << 2 | 3 ))\n\
                  grep -q pat <<<\"$text\"\n\
                  grep -m 20 pat file | cut -d: -f1\n\
                  echo \"a | head -1\" | wc -l\n\
                  printf '%s' \"$(git diff | sed -n '1,40p')\"\n";

    let (pipes, early) = early_readers(script);

    assert_eq!(early, Vec::new());
    assert_eq!(pipes, 4, "the four pipes that are pipes are counted");
}

/// A shell script is a `.sh` file or one of no extension with a shell's
/// `#!` line; anything else is not read.
#[test]
fn a_shell_script_is_known_by_its_name_or_its_first_line() {
    assert!(is_shell_script("scripts/gates/lib.sh", ""));
    assert!(is_shell_script("scripts/mutate", "#!/usr/bin/env bash\n"));
    assert!(is_shell_script("gradlew", "#!/bin/sh\n"));
    assert!(!is_shell_script(
        "scripts/lib/repro.py",
        "#!/usr/bin/env python3\n"
    ));
    assert!(!is_shell_script(
        "LICENSE",
        "GNU AFFERO GENERAL PUBLIC LICENSE\n"
    ));
}
