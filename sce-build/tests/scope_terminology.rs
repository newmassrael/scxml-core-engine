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

// Every tracked file is scanned. There is no extension list.
//
// There used to be one, and it was wrong twice for the same reason. It
// began as four extensions and missed twelve occurrences in the C++
// headers and test suite — the files where the diagnostic families are
// actually declared. Widening it to fifteen still missed `.c` (360
// tracked files: the C11 backend, a producer of its own), `.scxml` and
// `.txml` (the documents this project ships), and the web tree.
//
// A curated list of what to read cannot be checked against the thing
// it is meant to cover, so each omission is invisible until someone
// counts by hand. Reading the whole tree removes the question: a file
// git tracks is a file this gate reads, and anything genuinely outside
// SCE's own voice is registered below with its reason.
//
// Non-UTF8 blobs are skipped where they are read — there is no prose
// to scan in them, and that is a property of the bytes rather than of
// a list someone maintains.

/// Preceding words that make `agent` a different noun — a field name
/// or a protocol role — rather than SCE naming who reads its output.
///
/// A path exemption would be wrong for these. `User-Agent` sits in the
/// middle of `sce/src/events/`, beside code this gate should keep
/// reading, and `on-target agent` sits in one line of a spec whose
/// other 3000 lines this gate should keep reading too. Exempting the
/// directory would hand out permission far past the sentence that
/// needs it.
///
/// `User-Agent` is a field name from RFC 9110; renaming it would break
/// the wire it names. `on-target agent` is the synthesis protocol's
/// device-side role — the stub SCE codegen emits to run ON the target
/// and speak to the host fuzz driver over RTT/UART, in the synthesis
/// RFC's fuzz-coverage-transport section — so it names a thing SCE
/// generates, not a thing that reads SCE.
///
/// Written without a `§` token deliberately: the first draft cited
/// `§synth-F4`, which is an ARCHITECTURE tier label and not a section
/// of that ledger. The citation gate rejected it as
/// hallucination-class, which is what it is for.
///
/// `userAgent` needs no entry: the matcher is whole-word, so a
/// camelCase identifier never matches. `User-Agent` and `user_agent`
/// do, because `-` and `_` are boundaries — deliberately, since a
/// snake_case symbol naming a reader is the same claim as the prose.
/// The spaced form is here for the prose that describes the field
/// (`// User agent string`), which is the header by another spelling,
/// and for W3C's own `a user agent that can parse`.
const NON_READER_PREFIXES: &[&str] = &["user-", "user_", "user ", "on-target "];

/// Path prefixes where the term is not SCE describing itself, each with
/// the reason.
///
/// An exemption is a claim, so it is registered rather than assumed, and
/// the test checks it in both directions: a prefix that no longer
/// contains the term is dropped from this list, so the list cannot decay
/// into permission for files that have since been cleaned.
const EXEMPT_PREFIXES: &[(&str, &str)] = &[
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
    (
        "examples/ai_loop/",
        "the agent here is the subject the state machine supervises, not \
         the audience of anything SCE emits — the same distinction that \
         puts `on-target agent` in NON_READER_PREFIXES. The example's \
         whole point is that SCE stays a deterministic supervisor of a \
         process it does not model, so renaming the supervised party \
         would remove the one thing the example demonstrates. What the \
         rule forbids is SCE claiming who reads its wire surfaces, and \
         this example claims nothing about that.",
    ),
];

/// Lower bound on files the scan must read. Measured, not guessed:
/// 6328 of 6343 tracked files were read as text when the extension list was
/// dropped, and the floor sits well under that so ordinary tree growth
/// or pruning does not move it.
const MIN_SCANNED_FILES: usize = 5000;

/// Every tracked file, asked of git so untracked scratch files and
/// ignored build output cannot affect the verdict.
fn tracked_sources() -> Vec<PathBuf> {
    let root = repo_root();
    let out = Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files"])
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
            // `get` rather than an index: the left edge is `start` minus a
            // prefix length in BYTES, and a multi-byte char straddling it
            // makes indexing panic. A gate that panics reports nothing —
            // this one crashed on `— is the agent's session still there?`
            // in `examples/ai_loop/`, and the violation it was scanning
            // for went unreported behind its own backtrace for two
            // commits. `get` yields `None` there, which is the right
            // answer anyway: a byte sequence that is not this prefix.
            let other_noun = NON_READER_PREFIXES
                .iter()
                .any(|p| start >= p.len() && lower.get(start - p.len()..start) == Some(*p));
            if before_ok && after_ok && !other_noun {
                hits.push(i + 1);
                break;
            }
            from = end;
        }
    }
    hits
}

/// Does one `EXEMPT_PREFIXES` entry cover this repo-relative path?
///
/// A trailing `/` is what separates the two kinds of entry: with one, the
/// entry covers a directory and everything beneath it; without one, it
/// names a single file and covers only that file.
///
/// Extracted rather than written inline for the same reason `term_lines`
/// is: the tree cannot demonstrate it. Every entry in the table today is a
/// directory, so the file branch is unreachable from the sweep and would
/// ship unexercised — and an exemption rule that is never exercised is one
/// nobody can tell from a rule that exempts everything.
fn exemption_covers(prefix: &str, rel: &str) -> bool {
    if prefix.ends_with('/') {
        rel.starts_with(prefix)
    } else {
        rel == prefix
    }
}

/// A file entry covers itself and stops there; a directory entry reaches
/// its descendants.
///
/// The pair matters in one specific direction: under the previous rule the
/// only expressible entry was a directory, so exempting one file meant
/// exempting its siblings too. `first.rs` below is the case that could not
/// be said before — and `second.rs` is the sibling that must stay covered
/// by the sweep, which is the whole point.
#[test]
fn a_file_exemption_does_not_reach_its_siblings() {
    assert!(exemption_covers("a/b/first.rs", "a/b/first.rs"));
    assert!(!exemption_covers("a/b/first.rs", "a/b/second.rs"));
    // Not a prefix match in disguise: a longer path that merely starts
    // with the entry's bytes is a different file.
    assert!(!exemption_covers("a/b/first.rs", "a/b/first.rs.bak"));
    // And a file entry never opens a directory of the same stem.
    assert!(!exemption_covers("a/b/first", "a/b/first/inner.rs"));

    assert!(exemption_covers("a/b/", "a/b/first.rs"));
    assert!(exemption_covers("a/b/", "a/b/deeper/third.rs"));
    assert!(!exemption_covers("a/b/", "a/c/fourth.rs"));
    // The trailing slash is load-bearing: without it this would match
    // `a/bc/` as well, which is the accidental sibling coverage the
    // trailing-`/` requirement was originally written to prevent and
    // which exact matching now prevents properly.
    assert!(!exemption_covers("a/b/", "a/bc/fifth.rs"));
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
            .find(|(p, _)| exemption_covers(p, &rel))
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
/// The table is the one place a reader learns why a path is allowed to
/// differ. `feedback_capability_lists_in_prose_drift` is the failure this
/// avoids: a claim nothing checks decays into a label.
///
/// An entry names either a directory (trailing `/`) or one file (no
/// trailing `/`), and both are checked to exist. The rule used to demand
/// the trailing `/` outright, so that an entry "cannot match a sibling
/// file by accident" — but exact path equality serves that purpose
/// strictly better, and demanding the slash inverted the intent: the only
/// way to excuse a single file was to excuse its whole directory, which
/// is precisely the accidental sibling coverage the rule was written to
/// prevent. One file under `tests/` would have exempted every driver
/// beside it.
///
/// Existence is asserted because a mistyped entry is silent otherwise: it
/// exempts nothing, and the sweep's own staleness check would then report
/// it as an exemption covering nothing — a message about a dead entry,
/// for what is really a typo.
#[test]
fn every_exemption_states_why() {
    assert!(
        !EXEMPT_PREFIXES.is_empty(),
        "the exemption table is empty — either the reverse check above \
         is now unreachable or the table was dropped by accident"
    );
    let root = repo_root();
    for (prefix, reason) in EXEMPT_PREFIXES {
        assert!(
            reason.len() > 60,
            "exemption for {prefix:?} has no real reason: {reason:?}"
        );
        assert_eq!(
            exemption_resolves(&root, prefix),
            Ok(()),
            "exemption {prefix:?} does not resolve"
        );
    }
}

/// Does the entry name something that is actually there?
///
/// Split out so it can be asked about a path that is deliberately wrong,
/// which the table itself can never supply: a const cannot be given a
/// typo at test time, so the check would otherwise be asserted only
/// against entries that already pass and would keep passing if it were
/// deleted.
fn exemption_resolves(root: &Path, prefix: &str) -> Result<(), String> {
    let target = root.join(prefix.trim_end_matches('/'));
    if prefix.ends_with('/') {
        if target.is_dir() {
            Ok(())
        } else {
            Err(format!(
                "{prefix:?} ends in `/` so it names a directory, but \
                 {target:?} is not one — an entry that resolves to nothing \
                 exempts nothing, silently"
            ))
        }
    } else if target.is_file() {
        Ok(())
    } else {
        Err(format!(
            "{prefix:?} has no trailing `/` so it names one file, but \
             {target:?} is not one — add the `/` to cover a directory, or \
             correct the path"
        ))
    }
}

/// A mistyped entry is refused rather than quietly exempting nothing.
///
/// Without this the existence check is unfalsifiable: every entry in the
/// table resolves, so deleting the check changes no verdict. The two
/// wrong-kind cases are here because they are the likely mistakes — a
/// directory written without its slash, and a file written with one.
#[test]
fn an_exemption_that_resolves_to_nothing_is_refused() {
    let root = repo_root();

    assert!(exemption_resolves(&root, "docs/adr/").is_ok());
    assert!(exemption_resolves(&root, "sce-build/tests/scope_terminology.rs").is_ok());

    // Neither kind of typo passes.
    assert!(exemption_resolves(&root, "docs/adr-does-not-exist/").is_err());
    assert!(exemption_resolves(&root, "sce-build/tests/not_a_real_test.rs").is_err());

    // A directory named as a file, and a file named as a directory: both
    // resolve to something on disk, so only the kind check rejects them.
    assert!(exemption_resolves(&root, "docs/adr").is_err());
    assert!(exemption_resolves(&root, "sce-build/tests/scope_terminology.rs/").is_err());
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
        // A multi-byte char inside the prefix window. The matcher steps
        // back a prefix length in BYTES to test for `user-` and friends,
        // and indexing there used to panic when an em-dash straddled the
        // edge — so the scan died mid-tree and the 29 real violations it
        // was walking toward went unreported behind the backtrace for
        // two commits. A gate that crashes is indistinguishable from a
        // gate that found nothing, and this line is the one that did it.
        "watch    — is the agent's session still there?",
        // The same edge one byte over, so the assertion does not rest on
        // a single alignment.
        "the loop — an agent — is supervised",
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
