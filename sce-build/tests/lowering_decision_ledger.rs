// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The D1 decision ledger is held to the tree, row by row.
//!
//! `docs/SCE_LUA_TRANSLATION_SEAM.md` has to price four things before a
//! person can choose whether `sce-build` grows a C-callable lowering
//! surface. Three are closed on numbers and one is open on a judgement,
//! and that summary is the document's most quotable paragraph — which is
//! exactly the kind of sentence this repository has watched rot. The
//! same file records two instances already: a per-call figure quoted
//! from a `/tmp` probe that was deleted when the round ended and was
//! wrong by 2x, and "159 of 382" lifted out of a neighbouring
//! measurement and reused as a lane's size. Both read as measured. What
//! separated them from a measurement was that nothing asked the tree.
//!
//! So the summary is written as a table between two markers, and this
//! gate parses that table and asks the tree what each row claims:
//!
//!   * `derive:expression-alphabet=<n>` re-counts the alphabet the row
//!     reports, from the two enums the document names. A row saying 15
//!     stops being true the day a sixteenth `ExprError` variant lands,
//!     and this is what says so.
//!   * `census:<Token>` requires the row's evidence to exist, to be
//!     tracked, and requires some tracked file to actually PRINT
//!     `<Token> census:`. A closure resting on a probe nobody can run
//!     again is the `/tmp` defect, and this is the shape that catches
//!     it.
//!   * `precondition:rust-is-not-linked` is the one an OPEN row carries.
//!     It holds the premise that makes the row open: nothing in the tree
//!     links a Rust artifact, so the cost of linking one cannot be
//!     measured in advance and the item is a judgement rather than an
//!     unrun experiment. The day that stops being true, this turns red
//!     and the row has to be re-adjudicated.
//!
//! Two structural rules matter as much as the checks.
//!
//! An unrecognised status, kind or check is RED, never skipped. A ledger
//! whose unclassified rows pass is not a ledger; it is a list with an
//! exemption in it.
//!
//! And the table may not become a second copy of the prose list above
//! it. The bullets under "What the probe did NOT price" are the
//! population, so the row count is taken from them rather than restated
//! here — a bullet added without a row, or a row added without a
//! bullet, fails. `MIN_ROWS` is a floor beneath both, because two lists
//! that agree on being empty would otherwise agree.
//!
//! ## What this does NOT check
//!
//! It does not verify the *values* of the two census rows. `301 of
//! 1120` and `577ns` are produced by sweeps and by a timing probe on a
//! shared machine, and re-running either here would either duplicate
//! `scope_obligation` or assert a bound this repository has already
//! decided not to assert. What it holds instead is that the command
//! behind each number still exists and still emits the census the
//! document quotes, which is the half that rots silently.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The document that carries the ledger.
const LEDGER_DOC: &str = "docs/SCE_LUA_TRANSLATION_SEAM.md";

/// Markers delimiting the machine-readable block.
const OPEN_MARKER: &str = "<!-- D1-LEDGER";
const CLOSE_MARKER: &str = "<!-- /D1-LEDGER -->";

/// The prose list the row count is derived from.
const POPULATION_HEADING: &str = "What the probe did NOT price";

/// A floor beneath both lists.
///
/// The section was written against four items. Lowering this is a claim
/// that one of them never needed pricing, which is a claim to argue in
/// the document rather than to make by deleting a row — and without a
/// floor, an empty table and an empty bullet list agree with each other.
const MIN_ROWS: usize = 4;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Tracked paths, as `git` reports them.
///
/// `git ls-files` rather than `Path::exists`, for the reason every
/// tree-wide gate in this crate uses it: an untracked scratch file must
/// not be able to satisfy a row, and a configured build directory is
/// full of files that would.
fn tracked_files() -> BTreeSet<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

#[derive(Debug)]
struct Row {
    id: String,
    status: String,
    kind: String,
    check: String,
    evidence: String,
}

/// Strip the markdown emphasis a cell may carry around its payload.
fn cell(raw: &str) -> String {
    raw.trim().trim_matches('`').trim().to_string()
}

/// The rows between the two markers.
///
/// Everything that is not a table row — the marker comment's own lines,
/// the header, the alignment row — is dropped by shape rather than by
/// position, so a column added to the table's prose header cannot shift
/// what this reads.
fn ledger_rows(doc: &str) -> Vec<Row> {
    let lines: Vec<&str> = doc.lines().collect();
    let open = lines
        .iter()
        .position(|l| l.trim_start().starts_with(OPEN_MARKER))
        .unwrap_or_else(|| panic!("{LEDGER_DOC} carries no `{OPEN_MARKER}` marker"));
    let close = lines
        .iter()
        .position(|l| l.trim() == CLOSE_MARKER)
        .unwrap_or_else(|| panic!("{LEDGER_DOC} carries no `{CLOSE_MARKER}` marker"));
    assert!(
        close > open,
        "the D1-LEDGER close marker precedes its open marker in {LEDGER_DOC}"
    );

    let mut rows = Vec::new();
    for line in &lines[open + 1..close] {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(cell)
            .collect::<Vec<_>>();
        if cells.len() != 6 {
            panic!(
                "a D1-LEDGER line has {} cells, not the 6 the block declares \
                 (id | status | kind | number | check | evidence): {line}",
                cells.len()
            );
        }
        // The header and the alignment row.
        if cells[0] == "id" || cells[0].chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        rows.push(Row {
            id: cells[0].clone(),
            status: cells[1].clone(),
            kind: cells[2].clone(),
            check: cells[4].clone(),
            evidence: cells[5].clone(),
        });
    }
    rows
}

/// The bullets the table is a view of.
fn population_bullets(doc: &str) -> usize {
    let lines: Vec<&str> = doc.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains(POPULATION_HEADING))
        .unwrap_or_else(|| panic!("{LEDGER_DOC} no longer contains `{POPULATION_HEADING}`"));
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("### "))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());
    lines[start + 1..end]
        .iter()
        .filter(|l| l.starts_with("- "))
        .count()
}

/// Variants of the `ExprError` enum, counted the way the document says.
fn expr_error_variants(src: &str) -> usize {
    enum_variants(src, "pub enum ExprError {", |l| {
        l.starts_with("    ") && l.as_bytes().get(4).is_some_and(u8::is_ascii_uppercase)
    })
}

/// `DiagnosticCode`s in the `Expression*` family.
fn expression_codes(src: &str) -> usize {
    enum_variants(src, "pub enum DiagnosticCode {", |l| {
        l.starts_with("    Expression") && l.trim_end().ends_with(',')
    })
}

/// The variant names of a fieldless enum, in declaration order.
///
/// Order is the payload here, not a convenience: `ScopeStage` is a
/// ladder whose whole meaning is which name arrives before which.
fn enum_variant_names(src: &str, opener: &str) -> Vec<String> {
    let mut inside = false;
    let mut names = Vec::new();
    for line in src.lines() {
        if !inside {
            if line.trim_start().starts_with(opener) {
                inside = true;
            }
            continue;
        }
        if line == "}" {
            return names;
        }
        let trimmed = line.trim();
        if line.starts_with("    ")
            && trimmed
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        {
            names.push(trimmed.trim_end_matches(',').to_string());
        }
    }
    panic!("`{opener}` is not closed by a `}}` at column 0");
}

/// The lines of a shell script or workflow with commentary removed and
/// backslash continuations joined, so a token can be read as an
/// ARGUMENT rather than as text that happens to appear in the file.
///
/// Both bash and YAML comment with `#`, and no cargo argument this is
/// used to read contains one, so cutting each line at its first `#`
/// removes prose and commented-out invocations alike.
fn command_lines(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for line in src.lines() {
        let code = line.split('#').next().unwrap_or("").trim_end();
        match code.strip_suffix('\\') {
            Some(head) => {
                current.push_str(head);
                current.push(' ');
            }
            None => {
                current.push_str(code);
                out.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Whether `src` PASSES `feature` to cargo — as a member of a
/// `--features` / `-F` value on a line that is not commented out.
///
/// ⚠ The distinction is the whole of this function. The first form of
/// the check that uses it asked whether the text `ffi-probe` appeared
/// anywhere in a gate file, and the paragraph explaining why the
/// feature is named there satisfied that on its own: a mutation that
/// deleted the feature from the cargo invocation and left the prose
/// standing kept the gate GREEN while nothing compiled the probe. That
/// is the exact failure the check exists to prevent, so it was blind in
/// precisely the case it was written for. Naming a feature is not
/// building it, and a scanner that reads comments cannot tell.
fn passes_cargo_feature(src: &str, feature: &str) -> bool {
    command_lines(src).iter().any(|line| {
        let mut tokens = line.split_whitespace();
        loop {
            let Some(token) = tokens.next() else {
                return false;
            };
            let value = match token.strip_prefix("--features=") {
                Some(value) => Some(value),
                None if token == "--features" || token == "-F" => tokens.next(),
                None => None,
            };
            let Some(value) = value else { continue };
            // cargo accepts a comma- OR space-separated list, and the
            // shell may have quoted the whole of it — in which case the
            // list spans several whitespace tokens and has to be put
            // back together before it can be read as a list.
            let mut value = value.to_string();
            if let Some(quote) = value.chars().next().filter(|c| *c == '"' || *c == '\'') {
                while value.len() < 2 || !value.ends_with(quote) {
                    let Some(rest) = tokens.next() else { break };
                    value.push(' ');
                    value.push_str(rest);
                }
            }
            if value
                .trim_matches(|c| c == '"' || c == '\'')
                .split([',', ' '])
                .any(|named| named == feature)
            {
                return true;
            }
        }
    })
}

/// The boundary `passes_cargo_feature` exists to hold, pinned so that a
/// later simplification back to a substring search fails here first.
///
/// The negative cases are not hypothetical. The middle one is the
/// mutation that survived the check this replaced: `tree-hygiene.sh`
/// lost `ffi-probe` from its cargo invocation, kept the paragraph
/// explaining why the feature is named there, and the ledger stayed
/// green while no lane compiled the probe.
#[test]
fn a_named_feature_is_not_a_passed_feature() {
    for passed in [
        "cargo test -p sce-build --features cli,ffi-probe \\\n    --test roadmap_marker_gate\n",
        "cargo build --features=cli,ffi-probe\n",
        "cargo build -F ffi-probe\n",
        "  cargo test --features \"cli ffi-probe\"\n",
    ] {
        assert!(
            passes_cargo_feature(passed, "ffi-probe"),
            "should read a passed feature out of: {passed:?}"
        );
    }

    let named_only = "# `ffi-probe` is here so that SOMETHING compiles it.\n\
                      cargo test -p sce-build --features cli\n";
    assert!(
        named_only.contains("ffi-probe"),
        "the mutation kept the prose"
    );
    for not_passed in [
        named_only,
        // A commented-out invocation does not build anything either.
        "# cargo test -p sce-build --features cli,ffi-probe\n",
        // A longer name that merely starts the same way is not a member.
        "cargo test --features ffi-probe-extra\n",
        // `--features` belonging to some other command on another line.
        "cargo test --features\ncli,ffi-probe\n",
    ] {
        assert!(
            !passes_cargo_feature(not_passed, "ffi-probe"),
            "should NOT count as building the probe: {not_passed:?}"
        );
    }
}

fn enum_variants(src: &str, opener: &str, is_variant: impl Fn(&str) -> bool) -> usize {
    let mut inside = false;
    let mut count = 0usize;
    for line in src.lines() {
        if !inside {
            if line.trim_start().starts_with(opener) {
                inside = true;
            }
            continue;
        }
        if line == "}" {
            return count;
        }
        // A variant line, not the fields of a struct-shaped one: those
        // are indented further, which is what the four-space test says.
        if is_variant(line) {
            count += 1;
        }
    }
    panic!("`{opener}` is not closed by a `}}` at column 0");
}

/// Does any tracked file link a Rust artifact into a CMake target?
///
/// Three spellings, because there are three ways it could arrive: the
/// standard CMake/Rust bridge, a hand-written link against the built
/// library, and the library named as a file. `cargo` on its own is NOT
/// one of them — the tree already invokes cargo to obtain the
/// `sce-codegen` binary, which is a build tool and not a link.
fn cmake_links_rust(tracked: &BTreeSet<String>) -> Vec<String> {
    let mut hits = Vec::new();
    for path in tracked {
        if !(path.ends_with("CMakeLists.txt") || path.ends_with(".cmake")) {
            continue;
        }
        let src = read(path);
        for (n, line) in src.lines().enumerate() {
            let code = line.split('#').next().unwrap_or("");
            if code.contains("corrosion_")
                || code.contains("libsce_build")
                || code.contains("sce_build.so")
                || code.contains("sce_build.a")
            {
                hits.push(format!("{path}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    hits
}

#[test]
fn the_d1_ledger_is_classified_and_holds_to_the_tree() {
    let doc = read(LEDGER_DOC);
    let rows = ledger_rows(&doc);
    let tracked = tracked_files();

    assert!(
        rows.len() >= MIN_ROWS,
        "the D1 ledger has {} row(s), below the floor of {MIN_ROWS}. \
         A row removed is a claim that the item never needed pricing; \
         make that claim in the document, not by deleting the row.",
        rows.len()
    );

    let bullets = population_bullets(&doc);
    assert_eq!(
        rows.len(),
        bullets,
        "the D1 ledger has {} row(s) against {bullets} bullet(s) under \
         `{POPULATION_HEADING}`. The table is a VIEW of that list; two \
         lists that can disagree are two lists that will.",
        rows.len()
    );

    let mut seen = BTreeSet::new();
    let mut open = Vec::new();

    for row in &rows {
        assert!(
            seen.insert(row.id.clone()),
            "the D1 ledger names `{}` twice",
            row.id
        );

        match row.status.as_str() {
            "CLOSED" | "OPEN" => {}
            other => panic!(
                "row `{}` carries status `{other}`, which this gate does not \
                 recognise. An unclassified row is RED, not a pass — add the \
                 status here deliberately or spell one that exists \
                 (CLOSED, OPEN).",
                row.id
            ),
        }

        match row.kind.as_str() {
            "measurement" | "counting" | "judgement" => {}
            other => panic!(
                "row `{}` carries kind `{other}`, which this gate does not \
                 recognise (measurement, counting, judgement).",
                row.id
            ),
        }

        assert!(
            tracked.contains(&row.evidence),
            "row `{}` names evidence `{}`, which git does not track. \
             A closure resting on a file nobody can open is the `/tmp` \
             probe defect this document already recorded once.",
            row.id,
            row.evidence
        );

        if row.status == "OPEN" {
            open.push(row.id.clone());
            assert_eq!(
                row.kind, "judgement",
                "row `{}` is OPEN with kind `{}`. An item still open on a \
                 MEASUREMENT is an experiment nobody is running; say which \
                 command produces it and close it, or say why it is a \
                 judgement.",
                row.id, row.kind
            );
        }

        check_row(row, &tracked);
    }

    assert!(
        open.len() <= 1,
        "the D1 ledger has {} OPEN rows ({}). The section's claim is that \
         exactly one item is left and it is a person's; more than one means \
         that sentence is no longer true.",
        open.len(),
        open.join(", ")
    );
}

fn check_row(row: &Row, tracked: &BTreeSet<String>) {
    if let Some(want) = row.check.strip_prefix("derive:expression-alphabet=") {
        let want: usize = want
            .parse()
            .unwrap_or_else(|_| panic!("row `{}` declares a non-numeric alphabet size", row.id));
        let variants = expr_error_variants(&read("sce-build/src/forge/error.rs"));
        let codes = expression_codes(&read("sce-build/src/forge/diagnostic.rs"));
        assert_eq!(
            variants, want,
            "row `{}` reports {want} distinguishable failures; \
             `ExprError` now has {variants} variants. The number in the \
             document is what a decision is being taken on — re-derive it \
             and rewrite the row.",
            row.id
        );
        assert_eq!(
            codes, want,
            "row `{}` reports {want}; the `Expression*` family of \
             `DiagnosticCode` now has {codes} members. The row's claim is \
             that the code a C surface would carry already exists, one per \
             variant, so these two counts moving apart is the claim failing.",
            row.id
        );
        return;
    }

    if let Some(token) = row.check.strip_prefix("census:") {
        let needle = format!("{token} census:");
        let producer = tracked
            .iter()
            .filter(|p| p.ends_with(".rs") || p.ends_with(".sh") || p.ends_with(".py"))
            .find(|p| read(p).contains(&needle));
        assert!(
            producer.is_some(),
            "row `{}` rests on a `{needle}` line, and no tracked file emits \
             one. The number stays quotable while the command behind it is \
             gone — which is exactly how a figure from a deleted probe \
             survived a round in this document.",
            row.id
        );
        assert!(
            read(&row.evidence).contains(token),
            "row `{}` names `{}` as the way to re-derive it, but that file \
             does not mention `{token}`. The evidence must be the command \
             that produces the census, not a neighbouring script.",
            row.id,
            row.evidence
        );
        return;
    }

    if let Some(want) = row.check.strip_prefix("derive:rewriter-footprint=") {
        let want: usize = want
            .parse()
            .unwrap_or_else(|_| panic!("row `{}` declares a non-numeric line count", row.id));
        let cpp = "sce/src/scripting/EcmaScriptToLuaTransformer.cpp";
        let hdr = "sce/include/scripting/EcmaScriptToLuaTransformer.h";
        for path in [cpp, hdr] {
            assert!(
                tracked.contains(path),
                "row `{}` prices the swap on `{path}`, which git no longer tracks. \
                 If the rewriter has retired, the swap has HAPPENED and this row \
                 is a historical note, not a price — rewrite it.",
                row.id
            );
        }
        let lines = read(cpp).lines().count() + read(hdr).lines().count();
        assert_eq!(
            lines, want,
            "row `{}` says {want} tracked line(s) leave with the rewriter; the two \
             files now hold {lines}. The number a decision is taken on has moved.",
            row.id
        );

        // The half that makes the net a SUBTRACTION rather than two
        // unrelated figures: both sides have to be paid by the same
        // population. The rewriter is compiled by every C++ configure
        // because its translation unit is listed unconditionally — put
        // it behind `$<$<BOOL:${SCE_ENABLE_LUA}>:...>` the way
        // `LuaEngine.cpp` is and the OUT half stops matching the IN
        // half's population, which is exactly the defect this row was
        // written to remove.
        let sources = read("sce/sce_base_sources.cmake");
        let listed = sources
            .lines()
            .map(|l| l.split('#').next().unwrap_or(""))
            .find(|l| l.contains("EcmaScriptToLuaTransformer.cpp"))
            .unwrap_or_else(|| {
                panic!(
                    "row `{}`: `sce/sce_base_sources.cmake` no longer lists the \
                     rewriter, so it is not compiled by every C++ configure and \
                     the net's two halves no longer share a population",
                    row.id
                )
            });
        assert!(
            !listed.contains("$<"),
            "row `{}`: the rewriter is now listed behind a generator expression \
             (`{}`), so some C++ configures do not compile it. The OUT half is \
             then paid by a narrower population than the IN half, and the net \
             stops being a subtraction.",
            row.id,
            listed.trim()
        );

        // The IN half is measured by building an off-by-default feature,
        // and `clippy-check.yml` runs `--workspace --all-targets` WITHOUT
        // `--all-features`. So unless a lane PASSES the feature, nothing
        // compiles the probe — and a probe that stops compiling makes
        // this row's number un-re-derivable, which is the precise defect
        // it was committed to remove.
        //
        // ⚠ "Passes", not "mentions", and the difference was measured
        // rather than reasoned. This asked whether the text `ffi-probe`
        // occurred anywhere in a gate file, and the comment in
        // `tree-hygiene.sh` that explains why the feature is there said
        // it too — so deleting the feature from the cargo invocation and
        // leaving that paragraph standing kept this GREEN while nothing
        // built the probe. A gate blind in the one case it exists for is
        // not a gate, and `passes_cargo_feature` reads arguments.
        let gate_files: Vec<&String> = tracked
            .iter()
            .filter(|p| p.starts_with("scripts/gates/") || p.starts_with(".github/workflows/"))
            .collect();
        let compiled_by: Vec<&&String> = gate_files
            .iter()
            .filter(|p| passes_cargo_feature(&read(p), "ffi-probe"))
            .collect();
        let merely_named: Vec<&&String> = gate_files
            .iter()
            .filter(|p| read(p).contains("ffi-probe"))
            .collect();
        assert!(
            !compiled_by.is_empty(),
            "row `{}`: no gate script or workflow PASSES `--features ffi-probe` \
             to cargo, so no lane compiles the probe `{}` builds. The probe \
             would rot unnoticed and this row's number would go back to being \
             a figure nobody can reproduce. Files that merely NAME the feature, \
             which is not the same thing and does not compile it: {:?}",
            row.id,
            row.evidence,
            merely_named,
        );
        return;
    }

    if let Some(stage) = row.check.strip_prefix("derive:scope-ladder=") {
        let src = read("sce-build/src/ecmascript/scope.rs");
        let ladder: Vec<String> = enum_variant_names(&src, "pub enum ScopeStage {");
        let at = |name: &str| {
            ladder.iter().position(|v| v == name).unwrap_or_else(|| {
                panic!(
                    "row `{}`: `ScopeStage` has no `{name}` — the ladder is {ladder:?}",
                    row.id
                )
            })
        };
        let (data, here, writes) = (at("DataModel"), at(stage), at("WriteTargets"));
        assert!(
            data < here && here < writes,
            "row `{}`: `{stage}` sits at {here} in the ladder, not strictly \
             between `DataModel` ({data}) and `WriteTargets` ({writes}). The \
             answer rests on a load-time name arriving BEFORE a name an \
             `<assign>` writes — W3C SCXML 5.8 against a mid-execution write — \
             and a ladder in another order measured three sites where there \
             are none.",
            row.id
        );

        // Where the stage sits is half of it; the other half is what is
        // absorbed there. A `LoadTime` variant that nothing reads would
        // leave the ladder looking right and the census reading three.
        let gate = src
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with("if stage >=") && l.contains("ScopeStage::"))
            .unwrap_or_else(|| panic!("row `{}`: no stage gate in `from_model_upto`", row.id));
        assert!(
            gate.contains(&format!("ScopeStage::{stage}")),
            "row `{}`: `from_model_upto` admits the document-level `<script>`s \
             at `{}` rather than at `{stage}`. The stage exists and nothing \
             reaches it.",
            row.id,
            gate,
        );
        return;
    }

    if row.check == "precondition:rust-is-not-linked" {
        assert_eq!(
            row.status, "OPEN",
            "row `{}` is {} and carries a precondition check. A precondition \
             says why an item CANNOT be measured yet; a closed row must name \
             the measurement or the decision that closed it.",
            row.id, row.status
        );

        let cargo = read("sce-build/Cargo.toml");
        let declared = cargo
            .lines()
            .find(|l| l.trim_start().starts_with("crate-type"))
            .unwrap_or_else(|| panic!("sce-build/Cargo.toml declares no crate-type"));
        assert!(
            declared.contains("rlib") && !declared.contains("cdylib"),
            "row `{}` rests on `sce-build` being rlib-only; Cargo.toml now \
             says `{}`. The surface being decided on now EXISTS, so its cost \
             is measurable and this row is no longer a judgement about an \
             absent thing — re-adjudicate it.",
            row.id,
            declared.trim()
        );

        let hits = cmake_links_rust(tracked);
        assert!(
            hits.is_empty(),
            "row `{}` rests on nothing in the tree linking a Rust artifact, \
             and {} site(s) now do:\n  {}\nThe ON/OFF experiment the document \
             withdrew becomes measurable the moment this is false, and the \
             row has to be rewritten rather than left standing.",
            row.id,
            hits.len(),
            hits.join("\n  ")
        );
        return;
    }

    panic!(
        "row `{}` declares check `{}`, which this gate does not implement. \
         An unrecognised check is RED: a row whose check is skipped is a row \
         holding nothing, and it reads exactly like one that passed.",
        row.id, row.check
    );
}
