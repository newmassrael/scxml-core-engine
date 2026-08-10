// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE describes its surfaces, not who reads them.
//
// The project's scope boundary puts natural language, AI integration and
// prompt engineering upstream, in other projects: SCE defines an IR and
// generates deterministic code from it. A diagnostic record is read by
// CI, by an IDE, by a person at a terminal, and by a repair loop driving
// an LLM — SCE cannot tell which, and nothing in its behaviour depends
// on the answer.
//
// Calling the wire surfaces "agent-facing" contradicted that. It named
// one consumer as though it were the audience, which made the boundary
// read as porous in the one place a reader looks to find it: the
// contract documents and the schemas themselves. Two XSD headers and a
// field description on the diagnostic schema carried the word onto the
// wire, where an external consumer reads it.
//
// This gate keeps the vocabulary honest. It is a source scan, so it
// carries a floor: a scan that stopped reading the tree would report no
// violations and pass, which is the failure mode that lets a term creep
// back one commit at a time.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// Extensions carrying SCE's own prose: source, contracts, schemas,
/// build tooling and templates.
///
/// The first four were the whole list, and the omission was not
/// harmless: twelve occurrences sat in C++ headers, the C++ test suite
/// and a template expander, which is where the diagnostic families are
/// actually declared. A gate that reads only the Rust half of a
/// two-producer project measures the vocabulary of one producer and
/// reports it as the project's.
const SCANNED_EXTENSIONS: &[&str] = &[
    "md", "rs", "json", "xsd", "h", "hpp", "cpp", "py", "sh", "yml", "yaml", "cmake", "kt", "go",
    "jinja2",
];

/// Extensionless files that still carry prose worth scanning.
const SCANNED_FILENAMES: &[&str] = &["CMakeLists.txt"];

/// HTTP's own header name, which is not SCE's vocabulary.
///
/// A path exemption would be wrong here — these sit in the middle of
/// `sce/src/events/`, beside code this gate should keep reading — so the
/// exemption is on the token instead. `User-Agent` is a field name from
/// RFC 9110; renaming it would break the wire it names.
///
/// `userAgent` needs no entry: the matcher is whole-word, so a camelCase
/// identifier never matches. `User-Agent` and `user_agent` do, because
/// `-` and `_` are boundaries — deliberately, since a snake_case symbol
/// naming a reader is the same claim as the prose. The spaced form is
/// here for the prose that describes the field (`// User agent string`),
/// which is the header by another spelling; W3C's own `a user agent that
/// can parse` is covered by the `docs/spec/` path exemption instead.
const HTTP_HEADER_PREFIXES: &[&str] = &["user-", "user_", "user "];

/// Path prefixes where the term is not SCE describing itself, each with
/// the reason.
///
/// An exemption is a claim, so it is registered rather than assumed, and
/// the test checks it in both directions: a prefix that no longer
/// contains the term is dropped from this list, so the list cannot decay
/// into permission for files that have since been cleaned.
const EXEMPT_PREFIXES: &[(&str, &str)] = &[
    (
        "docs/spec/",
        "W3C SCXML text quoted verbatim — `a user agent that can parse` \
         is the standard's own wording, and the spec store is pinned \
         byte-for-byte. The synthesis RFC's `on-target agent stub` is a \
         protocol role (the thing running on the device), not a reader \
         of SCE's output.",
    ),
    (
        "docs/sce-ledger/",
        "ledger RFCs record decisions as they were written. Editing the \
         prose rewrites the record and trips `validate-content-drift`, \
         which exists to catch exactly that.",
    ),
    (
        "docs/adr/",
        "an ADR is the decision as taken, including the framing it was \
         taken under. The vocabulary moved afterwards; the record of \
         why it was chosen did not.",
    ),
];

/// Lower bound on files the scan must read. Measured, not guessed:
/// 4685 tracked files matched when this landed, and the floor sits well
/// under that so ordinary tree growth or pruning does not move it.
const MIN_SCANNED_FILES: usize = 3000;

/// Tracked files with a scanned extension, asked of git so untracked
/// scratch files and ignored build output cannot affect the verdict.
fn tracked_sources() -> Vec<PathBuf> {
    let root = repo_root();
    let mut args = vec![
        "-C".to_string(),
        root.display().to_string(),
        "ls-files".to_string(),
    ];
    for ext in SCANNED_EXTENSIONS {
        args.push(format!("*.{ext}"));
    }
    for name in SCANNED_FILENAMES {
        args.push((*name).to_string());
        args.push(format!("*/{name}"));
    }
    let out = Command::new("git")
        .args(&args)
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| root.join(l))
        .collect()
}

/// Case-insensitive whole-word search for the term, returning 1-based
/// line numbers.
///
/// Whole-word matching matters in both directions: `agent` inside
/// `management` is not this term, and a bare `Agents` at the start of a
/// sentence is.
fn term_lines(body: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    for (i, line) in body.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find("agent") {
            let start = from + rel;
            let end = start + "agent".len();
            let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
            // `agents` is the same term; anything else attached is not.
            let after = &bytes[end..];
            let after_ok = match after.first() {
                None => true,
                Some(b's') => after
                    .get(1)
                    .map(|c| !c.is_ascii_alphanumeric())
                    .unwrap_or(true),
                Some(c) => !c.is_ascii_alphanumeric(),
            };
            let http_header = HTTP_HEADER_PREFIXES
                .iter()
                .any(|p| start >= p.len() && &lower[start - p.len()..start] == *p);
            if before_ok && after_ok && !http_header {
                hits.push(i + 1);
                break;
            }
            from = end;
        }
    }
    hits
}

#[test]
fn sce_does_not_name_its_readers() {
    let root = repo_root();
    let mut scanned = 0usize;
    let mut violations: Vec<String> = Vec::new();
    let mut exempt_prefixes_seen: BTreeSet<&str> = BTreeSet::new();

    for path in tracked_sources() {
        let rel = path
            .strip_prefix(&root)
            .expect("tracked path sits under the repo root")
            .display()
            .to_string();
        // This file names the term to forbid it; exempting it by path
        // would also exempt any future test dropped beside it, so it is
        // matched exactly.
        if rel == "sce-build/tests/scope_terminology.rs" {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue; // non-UTF8 tracked blob: nothing to read prose in
        };
        scanned += 1;
        let hits = term_lines(&body);
        if hits.is_empty() {
            continue;
        }
        if let Some(prefix) = EXEMPT_PREFIXES
            .iter()
            .find(|(p, _)| rel.starts_with(p))
            .map(|(p, _)| *p)
        {
            exempt_prefixes_seen.insert(prefix);
            continue;
        }
        violations.push(format!("  {rel}: line(s) {hits:?}"));
    }

    assert!(
        scanned >= MIN_SCANNED_FILES,
        "scanned only {scanned} file(s), below the {MIN_SCANNED_FILES} on \
         record — the enumeration is broken, not the vocabulary. A scan \
         that reads nothing reports no violations and passes."
    );

    assert!(
        violations.is_empty(),
        "SCE names one of its readers instead of describing the surface \
         ({} file(s)):\n{}\n\nSCE provides wire surfaces; which consumer \
         reads them (CI, an IDE, a person, a repair loop) is outside its \
         scope, and saying otherwise on a schema or contract document \
         puts the claim where an external consumer reads it. Say \
         `consumer` — or drop the qualifier, since `wire surface` \
         already carries the meaning. If the word is genuinely something \
         else (a protocol role, quoted spec text), register the path \
         prefix in EXEMPT_PREFIXES with the reason.",
        violations.len(),
        violations.join("\n"),
    );

    // Reverse direction: an exemption whose files no longer carry the
    // term is permission nobody needs, and it would silently cover a
    // future reintroduction under that prefix.
    let registered: BTreeSet<&str> = EXEMPT_PREFIXES.iter().map(|(p, _)| *p).collect();
    let stale: Vec<&&str> = registered.difference(&exempt_prefixes_seen).collect();
    assert!(
        stale.is_empty(),
        "EXEMPT_PREFIXES names prefix(es) whose files no longer use the \
         term: {stale:?}\nDrop the entry — an exemption that covers \
         nothing would quietly cover the next reintroduction there.",
    );
}

/// Every exemption states a reason, and the reason is a sentence rather
/// than a placeholder.
///
/// The prefix list is the one place a reader learns why a directory is
/// allowed to differ. `feedback_capability_lists_in_prose_drift` is the
/// failure this avoids: a claim nothing checks decays into a label.
#[test]
fn every_exemption_states_why() {
    assert!(
        !EXEMPT_PREFIXES.is_empty(),
        "the exemption table is empty — either the reverse check above \
         is now unreachable or the table was dropped by accident"
    );
    for (prefix, reason) in EXEMPT_PREFIXES {
        assert!(
            reason.len() > 60,
            "exemption for {prefix:?} has no real reason: {reason:?}"
        );
        assert!(
            prefix.ends_with('/'),
            "exemption {prefix:?} must be a directory prefix ending in \
             `/`, so it cannot match a sibling file by accident"
        );
    }
}

/// The matcher recognises the term and nothing that merely contains it.
///
/// Needed because the tree cannot demonstrate this. Deleting the
/// word-boundary condition entirely leaves the sweep above green: no
/// tracked file happens to contain `agent` inside a longer word, so
/// widening the match to a bare substring changes no verdict and the
/// logic sits unproven. Synthetic input is the only thing that reaches
/// the branch.
#[test]
fn the_matcher_reads_words_not_substrings() {
    // The term, in the forms SCE's prose actually used.
    for line in [
        "agent",
        "Agents dispatch on `code`",
        "so an agent can apply the fix",
        "the agent-facing wire surface",
        "(CI, an IDE, a repair agent).",
        // HTTP's `User-Agent` is exempt, but a bare `agent` is not, and
        // the exemption keys on the `user-` / `user_` prefix rather than
        // on the whole line.
        "a repair agent for user-agent parsing",
        // A snake_case identifier is SCE naming a reader too, and `_`
        // is a boundary here for exactly that reason: a helper called
        // `agent_facing_surfaces` is the same claim as the prose.
        "fn agents_dispatch_on_code()",
    ] {
        assert_eq!(
            term_lines(line),
            vec![1],
            "should have matched the term in {line:?}"
        );
    }

    // Longer words that merely contain the letters. A substring matcher
    // reports every one of these, which is what makes the boundary
    // condition load-bearing rather than decorative.
    for line in [
        "management of the event queue",
        "reagent",
        "agentic",
        "agenda",
        // RFC 9110's field name, in the three spellings the tree uses.
        "request.headers[\"User-Agent\"] = \"SCE/1.0\"",
        "metadata[\"user_agent\"] = header",
        "std::string userAgent;",
    ] {
        assert!(
            term_lines(line).is_empty(),
            "should not have matched anything in {line:?}"
        );
    }

    // Line numbers are 1-based and every hit is reported once per line,
    // so a file's report points at lines a reader can open.
    assert_eq!(
        term_lines("clean\nan agent here\nclean\nagents\n"),
        vec![2, 4]
    );
}
