// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The script-engine cause census, held to the ceilings in its ledger.
//!
//! ADR 0003 decided that SCE compiles one portable document into each
//! backend's own language, and that a runtime script engine is the fallback
//! for what cannot be decided at build time. A decision like that disappears
//! unless something counts it, so this counts it: every tracked `.scxml` is
//! asked why it would need an engine, and the population per cause is held to
//! `docs/SCE_SCRIPT_ENGINE_CENSUS.md`.
//!
//! The direction is deliberate. A count may FALL — that is the programme
//! working. A RISE fails, which does not forbid it: it requires the same
//! commit to edit the ledger, so growth is a decision somebody wrote down
//! rather than drift nobody saw.
//!
//! ## Why this is a test and not a shell gate
//!
//! It was a shell gate first, and the shell gate was the wrong instrument
//! twice over. Measured 2026-09-16: one `sce-codegen check` per document took
//! 2610s over 736 documents. Neither half of that cost was the measurement.
//!
//!   * `check`'s contract is that it "reaches the verdict `generate` would --
//!     the same parse, the same validators, the same backend codegen", and
//!     with no `--language` it sweeps EVERY backend. The census reads
//!     `needs_script_engine` and `script_engine_causes`, which the binary
//!     itself obtains from a parse plus an analyze and nothing more.
//!   * One process per document paid a 95 MB debug binary's start-up 736
//!     times. Putting the whole corpus through one process changed nothing
//!     (50 documents, 148s), which is how the cost was identified as compute
//!     rather than spawn.
//!
//! Parsing and analyzing the whole corpus in one process is already paid for
//! several times over in this same lane -- `scope_obligation` does exactly
//! `parse_file` then `analyzer::analyze` per tracked document, and
//! `tree-hygiene` measured 193s for its whole target list. So the census is
//! cheap and only ever looked expensive.
//!
//! ## Walked is not judged
//!
//! The shell gate reported one number where there are two, and that is the
//! defect this file exists to not repeat. A document that does not parse
//! contributes no cause while still being swept. Counting it in the
//! denominator makes a clean table mean "never asked" rather than "nothing
//! found" -- and the corpus carries deliberate negative fixtures, forge
//! documents under `sce:kind`, and `sce:template` roots, so the two numbers
//! genuinely differ (736 walked, 475 judged on 2026-09-16). Both are reported
//! and both carry a floor.
//!
//! The retired gate reported 650, which was neither number: it counted a
//! document as judged whenever `check` emitted a manifest, while the facts
//! function behind that manifest parses the document itself and returns
//! nothing when it cannot. A document could therefore sit in that denominator
//! having never been asked -- the same false-clean, one level below where the
//! gate was looking for it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// The ledger this test holds the tree to.
const LEDGER: &str = "docs/SCE_SCRIPT_ENGINE_CENSUS.md";

/// The fence opening the block every enforced number lives in.
///
/// One block rather than prose: a number parsed out of a sentence stops being
/// enforced the moment somebody rewords the sentence, and nothing says so.
const OPEN_MARKER: &str = "```census";

/// The fence closing it.
const CLOSE_MARKER: &str = "```";

/// Bookkeeping keys, as opposed to cause kinds.
///
/// Named here so the cause-vocabulary check below can tell a ledger row that
/// should match a `ScriptEngineCauseKind` from one that should not.
const BOOKKEEPING: &[&str] = &[
    "documents-floor",
    "documents-judged-floor",
    "engine-documents",
    "native-prefix-documents",
];

/// Fewest entries the block can hold and still be the table this enforces.
///
/// An empty or truncated block would make every comparison below vacuously
/// true, which is the shape this repository keeps paying for.
const MIN_LEDGER_ENTRIES: usize = 10;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every `.scxml` this repository tracks.
///
/// `git ls-files` rather than a directory walk, for the reason
/// `scope_obligation` gives: a corpus narrowed to authored examples would
/// exclude the W3C documents, and those are most of the population.
fn corpus() -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "*.scxml"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| repo_root().join(line))
        .filter(|p| p.is_file())
        .collect()
}

/// Parse and analyze one document, or `None` when it does not parse.
///
/// This is the binary's `scxml_host_requirement_facts` without the manifest
/// wrapper: the same `parse_file` and the same `analyzer::analyze`, which is
/// what makes this test and `sce-codegen check` unable to disagree about a
/// document's causes.
///
/// A document that does not parse is NOT judged. The corpus carries
/// deliberate negative fixtures and forge documents the statechart pipeline
/// correctly refuses; the stage that judges those is not this one.
fn judge(path: &Path) -> Option<SCXMLModel> {
    let mut parser = SCXMLParser::new();
    let mut model = parser.parse_file(path.to_str()?).ok()?;
    sce_build::analyzer::analyze(&mut model, path.to_str()?);
    Some(model)
}

/// Every number the ledger enforces, read from its one block.
///
/// Fails loudly on a malformed block rather than reading it as empty: a
/// missing marker, an unreadable row and a table nobody wrote all have to be
/// distinguishable from a table that passes.
fn ceilings() -> BTreeMap<String, usize> {
    let path = repo_root().join(LEDGER);
    let doc =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lines: Vec<&str> = doc.lines().collect();

    let open = lines
        .iter()
        .position(|l| l.trim_start().starts_with(OPEN_MARKER))
        .unwrap_or_else(|| panic!("{LEDGER} carries no `{OPEN_MARKER}` block"));
    let close = lines
        .iter()
        .enumerate()
        .skip(open + 1)
        .find(|(_, l)| l.trim() == CLOSE_MARKER)
        .map(|(n, _)| n)
        .unwrap_or_else(|| panic!("{LEDGER}'s `{OPEN_MARKER}` block is never closed"));

    let mut out = BTreeMap::new();
    for line in &lines[open + 1..close] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert!(
            fields.len() == 2,
            "a census row has {} field(s), not the 2 the block declares \
             (`key value`): {line}",
            fields.len()
        );
        let value: usize = fields[1]
            .parse()
            .unwrap_or_else(|_| panic!("a census row's value is not a number: {line}"));
        assert!(
            out.insert(fields[0].to_string(), value).is_none(),
            "the census block names `{}` twice, so one of the two is not \
             enforcing anything",
            fields[0]
        );
    }

    assert!(
        out.len() >= MIN_LEDGER_ENTRIES,
        "the census block in {LEDGER} holds {} entries -- too few to be the \
         table this test enforces",
        out.len()
    );
    for required in BOOKKEEPING {
        assert!(
            out.contains_key(*required),
            "the census block names no `{required}`, so that axis would be \
             unmeasured while reading as clean"
        );
    }
    out
}

/// What one corpus walk produces.
#[derive(Default)]
struct Census {
    /// Documents swept.
    walked: usize,
    /// Documents that parsed, and so could contribute a cause.
    judged: usize,
    /// Judged documents whose build links a script engine.
    engine_documents: usize,
    /// Judged documents that reach for a native guard prefix.
    native_documents: usize,
    /// Causes by wire kind, over judged documents.
    causes: BTreeMap<&'static str, usize>,
}

fn walk() -> Census {
    let mut census = Census::default();
    for path in corpus() {
        census.walked += 1;
        let Some(model) = judge(&path) else {
            continue;
        };
        census.judged += 1;

        if model.needs_script_engine {
            census.engine_documents += 1;
        }
        for cause in &model.script_engine_causes {
            *census.causes.entry(cause.to_wire().kind).or_insert(0) += 1;
        }

        // A document that reaches for a native prefix has spent its
        // portability, and is counted in its own column rather than driven to
        // zero. Asked of the model rather than grepped out of the text: the
        // parser is what decides whether `cpp:` in a `cond` is a native guard
        // or a string that happens to start that way, and a text scan cannot
        // tell those apart.
        let native = model.states.values().any(|state| {
            state
                .transitions
                .iter()
                .any(|t| t.is_cpp_condition || t.is_kt_condition)
        });
        if native {
            census.native_documents += 1;
        }
    }
    census
}

/// The whole observed table, for a failure message.
///
/// Printed on every failure so that correcting the ledger -- or bootstrapping
/// it after the instrument changes -- is one run rather than a bisection.
fn observed(census: &Census) -> String {
    let mut out = String::new();
    out.push_str(&format!("  documents-floor {}\n", census.walked));
    out.push_str(&format!("  documents-judged-floor {}\n", census.judged));
    out.push_str(&format!("  engine-documents {}\n", census.engine_documents));
    out.push_str(&format!(
        "  native-prefix-documents {}\n",
        census.native_documents
    ));
    let mut by_count: Vec<(&&str, &usize)> = census.causes.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (kind, count) in by_count {
        out.push_str(&format!("  {kind} {count}\n"));
    }
    out
}

/// The census ledger still describes the tree, and nothing has grown.
#[test]
fn the_census_ledger_holds_the_tree_to_its_ceilings() {
    let limit = ceilings();
    let census = walk();

    // Printed on EVERY run, not only on a failure.
    //
    // A ratchet nobody can read is one nobody lowers, and a census whose
    // numbers appear only when it is red cannot be bootstrapped without first
    // breaking it. The block below is spelled exactly as the ledger carries
    // it, so moving a ceiling is a copy rather than a transcription.
    println!("script-engine census, as the ledger block spells it:");
    print!("{}", observed(&census));

    let mut faults: Vec<String> = Vec::new();

    // Floors first. A sweep that shrank makes every ceiling below vacuously
    // satisfied, so a clean table would mean nothing.
    let walked_floor = limit["documents-floor"];
    if census.walked < walked_floor {
        faults.push(format!(
            "swept {} document(s), below the floor of {walked_floor} -- the \
             sweep shrank",
            census.walked
        ));
    }
    let judged_floor = limit["documents-judged-floor"];
    if census.judged < judged_floor {
        faults.push(format!(
            "judged {} document(s), below the floor of {judged_floor} -- \
             documents stopped parsing, and a cause they can no longer \
             contribute reads as a cause they do not have",
            census.judged
        ));
    }

    let engine_ceiling = limit["engine-documents"];
    if census.engine_documents > engine_ceiling {
        faults.push(format!(
            "documents needing an engine: {} > {engine_ceiling}",
            census.engine_documents
        ));
    }
    let native_ceiling = limit["native-prefix-documents"];
    if census.native_documents > native_ceiling {
        faults.push(format!(
            "documents using a native prefix: {} > {native_ceiling}",
            census.native_documents
        ));
    }

    for (kind, count) in &census.causes {
        match limit.get(*kind) {
            None => faults.push(format!(
                "cause `{kind}` appears {count} time(s) and the ledger does \
                 not name it"
            )),
            Some(ceiling) if count > ceiling => {
                faults.push(format!("cause `{kind}`: {count} > {ceiling}"))
            }
            Some(_) => {}
        }
    }

    // A ledger row for a cause the corpus no longer carries is a ceiling
    // nothing can breach, and it is indistinguishable from a misspelled key
    // -- which is a ceiling that was never enforcing anything. Either way the
    // row is removed in the commit that empties it, the same discipline a
    // rise takes.
    for key in limit.keys() {
        if BOOKKEEPING.contains(&key.as_str()) {
            continue;
        }
        if !census.causes.contains_key(key.as_str()) {
            faults.push(format!(
                "the ledger names cause `{key}`, which no document carries -- \
                 remove the row in the commit that emptied it, or correct the \
                 spelling if it never named a real kind"
            ));
        }
    }

    assert!(
        faults.is_empty(),
        "the script-engine census disagrees with {LEDGER}:\n{}\n\n\
         observed, which is the block {LEDGER} should carry:\n{}",
        faults
            .iter()
            .map(|f| format!("  {f}"))
            .collect::<Vec<_>>()
            .join("\n"),
        observed(&census)
    );
}
