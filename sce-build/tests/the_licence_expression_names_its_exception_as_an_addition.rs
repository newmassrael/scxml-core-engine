// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every SPDX licence expression the tree carries is one SPDX can read, and
//! every one that names SCE's Linking Exception is the expression
//! `LICENSE-EXCEPTION.md` gives.
//!
//! # What was wrong
//!
//! The expression every SCE file carried put a `LicenseRef-` after `WITH`.
//! In the SPDX licence expression grammar the operand of `WITH` is an
//! exception, not a licence: an identifier from the SPDX exception list, or,
//! for an exception the list does not hold, an `AdditionRef-` (SPDX 3.0
//! Annex B, which added `addition-ref` for exactly this). A `LicenseRef-`
//! names a licence, so a reader that follows the grammar refuses the
//! expression or reads the Linking Exception as a second licence, and the
//! exception the expression exists to state is lost. `LicenseRef-` stays
//! right where a licence stands: `LicenseRef-SCE-Commercial` is one.
//!
//! Nothing read the expression before this gate (measured 2026-11-16): the
//! spelling was copied into some six thousand headers, fourteen Cargo
//! manifests and three Python projects, and the only check of licence text
//! (`scripts/verify_licenses.sh`) reads the third-party notices.
//!
//! # What is held
//!
//! * The expression `LICENSE-EXCEPTION.md` gives under "SPDX IDENTIFIER" is
//!   the one source. The exception's own text says it applies to every file
//!   carrying that expression, so a file that spells it otherwise is a file
//!   the exception's text does not name.
//! * Every `SPDX-License-Identifier:` line in a tracked file, and every
//!   `license` field of a tracked `Cargo.toml` or `pyproject.toml`, is a
//!   well-formed expression in that sense, and names the Linking Exception
//!   only as the whole expression the exception file gives.
//!
//! Third-party files keep their own identifiers (`MIT`, `EPL-2.0`); they are
//! read by the grammar rule only. The `PRIOR VERSIONS` paragraph of
//! `LICENSE` and ADR 0004 quote the expression an earlier release carried,
//! as history, on lines that are not identifier lines, and are not read.

mod common;

use std::path::Path;

/// The marker an identifier line carries.
const MARKER: &str = "SPDX-License-Identifier:";

/// What ends an expression written inside a comment, a string or prose: a
/// template or markup comment's closer, a C comment's, a quote, an escaped
/// newline inside a literal, a code span's backtick.
const ENDS: &[&str] = &["-#}", "#}", "-->", "*/", "\"", "\\n", "`"];

/// The expression `LICENSE-EXCEPTION.md` gives under "SPDX IDENTIFIER".
fn canonical() -> String {
    let path = common::repository::root().join("LICENSE-EXCEPTION.md");
    let text = std::fs::read_to_string(&path).expect("LICENSE-EXCEPTION.md reads");
    let section = text
        .split("## SPDX IDENTIFIER")
        .nth(1)
        .expect("LICENSE-EXCEPTION.md has an SPDX IDENTIFIER section");
    section
        .lines()
        .find_map(|line| line.trim().strip_prefix(MARKER))
        .map(|expression| expression.trim().to_string())
        .expect("the SPDX IDENTIFIER section gives the expression")
}

/// The expression written after `MARKER` on `line`, cut where the comment,
/// string or prose around it resumes. `None` when the line holds no marker
/// or nothing follows it.
fn expression_on(line: &str) -> Option<String> {
    let after = &line[line.find(MARKER)? + MARKER.len()..];
    let end = ENDS
        .iter()
        .filter_map(|end| after.find(end))
        .min()
        .unwrap_or(after.len());
    let expression = after[..end].trim();
    (!expression.is_empty()).then(|| expression.to_string())
}

/// What is wrong with `expression` as an SPDX licence expression, by the
/// rules this gate holds: the operand of `WITH` is an exception (an SPDX
/// exception identifier or an `AdditionRef-`), never a `LicenseRef-`; an
/// `AdditionRef-` stands only after `WITH`; and `WITH` stands between a
/// licence and an exception. Empty when nothing is.
fn faults(expression: &str) -> Vec<String> {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let operand =
        |t: Option<&str>| t.is_some_and(|t| !matches!(t, "AND" | "OR" | "WITH" | "(" | ")"));
    let mut out = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let after_with = i > 0 && tokens[i - 1] == "WITH";
        if *token == "WITH" {
            let licence = i.checked_sub(1).map(|j| tokens[j]);
            let exception = tokens.get(i + 1).copied();
            if !operand(licence) || !operand(exception) {
                out.push(format!(
                    "`WITH` at token {i} does not stand between a licence and an exception"
                ));
            }
        } else if after_with && token.starts_with("LicenseRef-") {
            out.push(format!(
                "`{token}` follows `WITH`, where an exception stands: an exception the SPDX \
                 list does not hold is an `AdditionRef-`, and a `LicenseRef-` names a licence"
            ));
        } else if !after_with && token.starts_with("AdditionRef-") {
            out.push(format!(
                "`{token}` stands where a licence stands: an addition is the operand of `WITH`"
            ));
        }
    }
    out
}

/// The `license` value of one manifest line: `license = "…"` in a Cargo
/// manifest, `license = { text = "…" }` in a Python project.
fn manifest_licence(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("license")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let rest = rest
        .strip_prefix('{')
        .and_then(|inner| inner.trim().strip_prefix("text"))
        .and_then(|inner| inner.trim().strip_prefix('='))
        .map_or(rest, str::trim);
    let quoted = rest.strip_prefix('"')?;
    Some(quoted[..quoted.find('"')?].to_string())
}

/// One expression the tree carries, and where.
struct Found {
    path: String,
    line: usize,
    expression: String,
    /// Read from a manifest's `license` field rather than an identifier
    /// line.
    field: bool,
}

/// Every expression in one file's text: its identifier lines, and its
/// `license` fields when it is a Cargo manifest or a Python project.
fn expressions_in(path: &str, text: &str) -> Vec<Found> {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let manifest = matches!(name, "Cargo.toml" | "pyproject.toml");
    text.lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let (expression, field) = match expression_on(line) {
                Some(expression) => (expression, false),
                None if manifest => (manifest_licence(line)?, true),
                None => return None,
            };
            Some(Found {
                path: path.to_string(),
                line: n + 1,
                expression,
                field,
            })
        })
        .collect()
}

/// What is wrong with one expression the tree carries, given the canonical
/// one.
fn judged(found: &Found, canonical: &str) -> Vec<String> {
    let mut out: Vec<String> = faults(&found.expression)
        .into_iter()
        .map(|fault| format!("{}:{}: {fault}", found.path, found.line))
        .collect();
    if found.expression.contains("SCE-Linking-Exception") && found.expression != canonical {
        out.push(format!(
            "{}:{}: `{}` names the Linking Exception, and the expression LICENSE-EXCEPTION.md \
             gives is `{canonical}`",
            found.path, found.line, found.expression
        ));
    }
    out
}

#[test]
fn the_expression_the_exception_gives_is_one_spdx_reads() {
    let canonical = canonical();
    assert!(
        canonical.contains("SCE-Linking-Exception"),
        "the expression LICENSE-EXCEPTION.md gives does not name the exception: {canonical}"
    );
    assert_eq!(faults(&canonical), Vec::<String>::new(), "{canonical}");
}

#[test]
fn every_expression_in_the_tree_is_well_formed_and_names_the_exception_one_way() {
    let canonical = canonical();
    let root = common::repository::root();
    let mut read = 0usize;
    let mut naming_the_exception = 0usize;
    let mut manifests = 0usize;
    let mut wrong = Vec::new();
    for path in common::repository::paths_git_tracks(&[]) {
        // A tracked file the working tree deleted is not read; a file that
        // is not UTF-8 is read lossily, since the marker is ASCII.
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        for found in expressions_in(&path, &text) {
            read += 1;
            if found.expression.contains("SCE-Linking-Exception") {
                naming_the_exception += 1;
                manifests += usize::from(found.field);
            }
            wrong.extend(judged(&found, &canonical));
        }
    }

    // Floors: a sweep that read nothing judged nothing. Every SCE source
    // file carries the expression, and the workspace and the Python
    // projects state it as their licence.
    assert!(
        naming_the_exception >= 1000,
        "only {naming_the_exception} expression(s) naming the exception were read of {read}"
    );
    assert!(
        manifests >= 4,
        "only {manifests} manifest licence field(s) were read"
    );
    assert!(
        wrong.is_empty(),
        "{} expression(s) are not as LICENSE-EXCEPTION.md gives them or not SPDX:\n  {}",
        wrong.len(),
        wrong
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// The rules refuse what they exist for, so a green sweep is not a sweep
/// that reads nothing.
#[test]
fn the_rules_refuse_a_licence_reference_after_with() {
    let licence_ref = ["License", "Ref-"].concat();
    let earlier = format!(
        "AGPL-3.0-only WITH {licence_ref}SCE-Linking-Exception OR {licence_ref}SCE-Commercial"
    );
    assert_eq!(faults(&earlier).len(), 1, "{:?}", faults(&earlier));
    assert_eq!(
        faults("AdditionRef-SCE-Linking-Exception OR MIT").len(),
        1,
        "an addition standing alone"
    );
    assert_eq!(faults("MIT WITH").len(), 1, "a WITH with no exception");
    assert!(faults("Apache-2.0 WITH LLVM-exception").is_empty());
    assert!(faults("(MIT OR Apache-2.0) AND BSD-3-Clause").is_empty());

    let canonical = canonical();
    let found = Found {
        path: "probe.rs".to_string(),
        line: 1,
        expression: earlier,
        field: false,
    };
    assert_eq!(
        judged(&found, &canonical).len(),
        2,
        "the earlier spelling is malformed and not the canonical expression"
    );
}

/// The expression is read out of every shape a file writes it in, and a
/// manifest's licence field out of both manifest shapes.
#[test]
fn the_expression_is_read_out_of_every_shape_it_is_written_in() {
    let e = "AGPL-3.0-only WITH AdditionRef-X OR LicenseRef-Y";
    for line in [
        format!("// {MARKER} {e}"),
        format!("# {MARKER} {e}"),
        format!("{{# {MARKER} {e} #}}"),
        format!("{{# {MARKER} {e} -#}}"),
        format!("<!-- {MARKER} {e} -->"),
        format!("/* {MARKER} {e} */"),
        format!("    echo \"// {MARKER} {e}\""),
    ] {
        assert_eq!(expression_on(&line).as_deref(), Some(e), "{line}");
    }
    assert_eq!(
        expression_on(&format!("\"// {MARKER} MIT\\n{{body}}\")")).as_deref(),
        Some("MIT")
    );
    assert_eq!(expression_on("no marker here"), None);

    assert_eq!(
        manifest_licence(&format!("license = \"{e}\"")).as_deref(),
        Some(e)
    );
    assert_eq!(
        manifest_licence(&format!("license = {{text = \"{e}\"}}")).as_deref(),
        Some(e)
    );
    assert_eq!(
        manifest_licence(&format!("license = {{ text = \"{e}\" }}")).as_deref(),
        Some(e)
    );
    assert_eq!(manifest_licence("license.workspace = true"), None);
    assert_eq!(manifest_licence("license-file = \"LICENSE\""), None);
}
