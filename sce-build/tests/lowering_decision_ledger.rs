// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The D1 decision ledger is held to the tree, row by row.
//!
//! `docs/SCE_LUA_TRANSLATION_SEAM.md` had to price four things before a
//! person could choose whether `sce-build` grows a C-callable lowering
//! surface. All four were priced, two more numbers arrived while they
//! were being checked, and on 2026-08-29 the owner CHOSE: link it, and
//! retire the rewriter. Six rows now, five closed on numbers and one on
//! that decision. Each sentence in that summary is the document's most
//! quotable kind — which is
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
//!   * `decision:linked-beside-lua` is what a CLOSED-by-decision row
//!     carries. There is no measurement to re-run, so what stands in
//!     for one is the tree still doing what was decided: the surface
//!     exists behind its feature, a CMake file links it, the link sits
//!     inside `if(SCE_ENABLE_LUA)` rather than behind the engine
//!     selection, and `LuaEngine` actually calls it. The last is not
//!     decoration — a linked library nothing reaches is discarded by
//!     the linker, and without it this row could not fail.
//!   * `precondition:rust-is-not-linked` is what the OPEN row carried
//!     before that: nothing in the tree links a Rust artifact, so the
//!     cost could not be measured and the item was a judgement rather
//!     than an unrun experiment. **No row carries it now**, and it is
//!     kept because the shape is right for the next premise that has
//!     to hold a row open.
//!     ⚠ It is also kept as a warning. When the link finally arrived it
//!     did NOT fire, because `cmake_links_rust` matched spellings
//!     (`libsce_build`, `corrosion_`) and the real link asks cargo for
//!     the path at configure time and exposes `SCE::Lowering`. A
//!     tripwire is not tested by the thing it is watching for; this one
//!     was only found because the row it guarded was being replaced.
//!
//! Three structural rules matter as much as the checks.
//!
//! An unrecognised status, kind or check is RED, never skipped. A ledger
//! whose unclassified rows pass is not a ledger; it is a list with an
//! exemption in it.
//!
//! A check name the document SPELLS must be one a row declares
//! (`a_check_named_in_prose_is_a_check_a_row_declares`). Naming a check
//! promises that some gate re-derives the sentence beside it, and the
//! promise outlived its row once already: the section under the table went
//! on citing the precondition above for a day after the decision row
//! replaced it, saying a Rust link would turn the gate red — and the link
//! landed with no row left to turn.
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

/// Run `git` at the repository root; report whether it succeeded and what it
/// printed.
///
/// ⚠ Two questions need two commands here, and the reason is a defect this
/// repository walked into: `git show <sha>:<path>` answers "there is no such
/// commit" and "that commit has no such path" with the SAME sentence, and one
/// was read for the other. `git cat-file -t` is asked about the COMMIT and
/// `git ls-tree` about the PATH, so a failure says which of the two it is.
fn git_at_root(args: &[&str]) -> (bool, String) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(args)
        .output()
        .expect("git runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
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
/// the check that uses it asked whether the text `ffi` appeared
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
/// lost `ffi` from its cargo invocation, kept the paragraph
/// explaining why the feature is named there, and the ledger stayed
/// green while no lane compiled the probe.
#[test]
fn a_named_feature_is_not_a_passed_feature() {
    for passed in [
        "cargo test -p sce-build --features cli,ffi \\\n    --test roadmap_marker_gate\n",
        "cargo build --features=cli,ffi\n",
        "cargo build -F ffi\n",
        "  cargo test --features \"cli ffi\"\n",
    ] {
        assert!(
            passes_cargo_feature(passed, "ffi"),
            "should read a passed feature out of: {passed:?}"
        );
    }

    let named_only = "# `ffi` is here so that SOMETHING compiles it.\n\
                      cargo test -p sce-build --features cli\n";
    assert!(named_only.contains("ffi"), "the mutation kept the prose");
    for not_passed in [
        named_only,
        // A commented-out invocation does not build anything either.
        "# cargo test -p sce-build --features cli,ffi\n",
        // A longer name that merely starts the same way is not a member.
        "cargo test --features ffi-extra\n",
        // `--features` belonging to some other command on another line.
        "cargo test --features\ncli,ffi\n",
    ] {
        assert!(
            !passes_cargo_feature(not_passed, "ffi"),
            "should NOT count as building the probe: {not_passed:?}"
        );
    }
}

/// `src` with C and C++ commentary removed and the CONTENTS of string
/// and character literals emptied, so a symbol can be read as a CALL
/// rather than as text that happens to appear in the file.
///
/// The same distinction `passes_cargo_feature` draws, one language over.
/// Literals are emptied as well as comments because a symbol named in a
/// diagnostic message is not called either, and both are how a deleted
/// call leaves its name behind.
///
/// It reads C++ as a lexer does and not as a parser does: `a / *p` would
/// be taken for a block comment. No such division exists in the file this
/// is pointed at, and widening it to a parse would be a second front end
/// for a check that only has to tell code from prose.
fn cpp_code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for inner in chars.by_ref() {
                    if inner == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for inner in chars.by_ref() {
                    // A block comment spans lines, and the lines are kept
                    // so a later line-based reading still lines up.
                    if inner == '\n' {
                        out.push('\n');
                    }
                    if prev == '*' && inner == '/' {
                        break;
                    }
                    prev = inner;
                }
            }
            '"' | '\'' => {
                out.push(c);
                let mut escaped = false;
                for inner in chars.by_ref() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if inner == '\\' {
                        escaped = true;
                        continue;
                    }
                    if inner == c {
                        out.push(c);
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// The boundary `cpp_code_only` exists to hold, pinned so that a later
/// simplification back to a substring search over the whole file fails
/// here first.
///
/// The first negative case is not hypothetical. It is the mutation run on
/// 2026-08-29 against `decision:linked-beside-lua`: the call was deleted
/// from `LuaEngine::loweredTextOf` and the paragraph explaining it was
/// left standing, exactly as a careless revert would leave it. The C++
/// suite went red on eleven expressions and this ledger stayed GREEN —
/// blind in precisely the case the check was written for, which is the
/// same defect `a_named_feature_is_not_a_passed_feature` records one
/// language over.
#[test]
fn a_mentioned_call_is_not_a_made_call() {
    for called in [
        "if (char *lowered = sce_lower_value(text.c_str(), scope())) {\n",
        "// offered to the frontend first\nreturn sce_lower_value(t, s);\n",
        "/* block */ x = sce_lower_value(t, s);\n",
    ] {
        assert!(
            cpp_code_only(called).contains("sce_lower_value("),
            "should read a made call out of: {called:?}"
        );
    }

    let mentioned_only = "// MUTATION: the call to sce_lower_value(...) was removed here.\n\
                          return transformer_.transform(expression.text());\n";
    assert!(
        mentioned_only.contains("sce_lower_value("),
        "the mutation kept the prose"
    );
    for not_called in [
        mentioned_only,
        // A block comment hides it just as well as a line comment.
        "/*\n * sce_lower_value(t, s) used to be called here.\n */\n",
        // A diagnostic naming the symbol does not call it.
        "throw std::runtime_error(\"sce_lower_value( is unavailable\");\n",
    ] {
        assert!(
            !cpp_code_only(not_called).contains("sce_lower_value("),
            "should NOT count as calling the surface: {not_called:?}"
        );
    }

    // An apostrophe inside a comment must not be read as opening a
    // character literal and swallowing the code after it: comments are
    // cut before literals are, and this pins that order.
    let after_an_apostrophe = "// the frontend doesn't refuse this one\nsce_lower_value(t, s);\n";
    assert!(
        cpp_code_only(after_an_apostrophe).contains("sce_lower_value("),
        "an apostrophe in a comment must not hide the code below it"
    );
}

/// The boundary `reaches_rewriter` holds, pinned so that the retirement
/// check cannot be quietly widened or quietly blinded.
///
/// Each negative case is a shape the tree actually contains. The engine
/// still explains, in prose, what the rewriter was and why it went, in
/// both a `//` comment and a `/** */` block, and a check that read those
/// as callers would force the history to be deleted in order to go
/// green — the opposite of what this row is for. How MANY such comments
/// there are is not written here on purpose: it is a number nothing
/// re-derives, and this file records what a restated count costs.
///
/// Each positive case is what a revert looks like. The member declaration
/// is how the caller stood until the day this row was written; the
/// include is what would survive a caller being removed and the header
/// left behind, and the unit would then still be compiled into the engine
/// with nothing to show for it.
#[test]
fn a_remembered_rewriter_is_not_a_reached_one() {
    for reaching in [
        // The member, as `LuaEngine.h` carried it.
        "class LuaEngine {\n    EcmaScriptToLuaTransformer transformer_;\n};\n",
        "std::string s = SCE::EcmaScriptToLuaTransformer{}.transformScript(t);\n",
        // The include alone: no call, but the engine still compiles it in.
        "#include \"scripting/EcmaScriptToLuaTransformer.h\"\n#include <string>\n",
        // A comment ABOVE a real use must not hide the use.
        "// the rewriter is gone\nEcmaScriptToLuaTransformer t;\n",
    ] {
        assert!(
            reaches_rewriter(reaching),
            "should count as reaching the rewriter: {reaching:?}"
        );
    }

    // ⚠ The predicate is per FILE and the claim is per POPULATION, and this
    // is the case that separates them. `return transformer_.transform(…);`
    // is exactly how the call stood in `LuaEngine.cpp` until this row was
    // written, and read on its own it names nothing — the type appears only
    // where the member is DECLARED, one file over.
    //
    // That is not a hole, and this pins why: C++ has no way to call a
    // class's method without a declaration of that class being visible, and
    // `sce/include/` is swept together with `sce/src/`. So the pair is
    // caught even though one half of it is invisible. Keying the predicate
    // on the member's NAME instead would make it catch that half — and go
    // blind the day the member is renamed, which is a rename of a detail
    // rather than of the subject.
    let caller_cpp = "return transformer_.transform(expression.text());\n";
    let its_header = "class LuaEngine {\n    EcmaScriptToLuaTransformer transformer_;\n};\n";
    assert!(
        !reaches_rewriter(caller_cpp),
        "a call through a member names no type, and pretending otherwise \
         would hide which half of the pair this check actually sees"
    );
    assert!(
        [caller_cpp, its_header].iter().any(|f| reaches_rewriter(f)),
        "the population is swept as a whole — a source and the header that \
         declares what it calls — and neither half being seen would mean a \
         restored caller passes"
    );

    for remembering in [
        // The five shapes the engine's own prose actually uses.
        "// `EcmaScriptToLuaTransformer` rewrote the text without a parse.\n",
        "/*\n * EcmaScriptToLuaTransformer::transformScript replaced text.\n */\n",
        "    /// must adapt it (`EcmaScriptToLuaTransformer`) or refuse.\n",
        "// #include \"scripting/EcmaScriptToLuaTransformer.h\"\n",
        "/* #include \"scripting/EcmaScriptToLuaTransformer.h\" */\n",
        // A diagnostic naming it is not a use of it, for the same reason
        // one naming `sce_lower_value(` is not a call to it.
        "throw std::runtime_error(\"EcmaScriptToLuaTransformer is retired\");\n",
        // An include of something else entirely.
        "#include \"scripting/LoweringScope.h\"\n",
    ] {
        assert!(
            !reaches_rewriter(remembering),
            "should NOT count as reaching the rewriter: {remembering:?}"
        );
    }

    // The two readings line up by index, which is the whole mechanism of
    // the include arm: a block comment spanning three lines must not shift
    // the include below it onto a different raw line.
    let after_a_block =
        "/* one\n * two\n */\n#include \"scripting/EcmaScriptToLuaTransformer.h\"\n";
    assert!(
        reaches_rewriter(after_a_block),
        "a block comment above an include must not shift the line the include is read from"
    );
}

/// The prefixes a check identifier can open with.
///
/// One per shape `check_row` implements. A kind added there without a
/// line here would let the document spell it and promise nothing, which
/// is the failure this list exists inside.
const CHECK_KINDS: &[&str] = &[
    "census:",
    "derive:",
    "decision:",
    "precondition:",
    "retirement:",
    "retired-measurement:",
];

/// The rewriter the `retire-rewriter` row is about.
///
/// One constant, used as the type name, as the stem of the two files that
/// ARE the retired unit, and as the header a caller would include. That is
/// not brevity: a sweep whose subject is spelled three times can be half
/// renamed, and the half left behind reports zero because it is looking for
/// something nothing carries.
const REWRITER: &str = "EcmaScriptToLuaTransformer";

/// Every check identifier the document SPELLS, with the line it is on.
///
/// Backticked spans only, and fenced code blocks are skipped: a shell
/// transcript quoting a check name is showing a command, not making a
/// promise about this table.
fn check_names_spelled(doc: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut fenced = false;
    for (n, line) in doc.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        // Odd segments of a backtick split are the spans INSIDE backticks.
        for span in line.split('`').skip(1).step_by(2) {
            let span = span.trim();
            if CHECK_KINDS
                .iter()
                .any(|k| span.starts_with(k) && span.len() > k.len())
            {
                out.push((n + 1, span.to_string()));
            }
        }
    }
    out
}

/// A check name in prose is a promise that some gate re-derives the
/// sentence beside it, so a name no row declares is a promise nothing
/// keeps.
///
/// Measured rather than reasoned, like everything else here. The section
/// under the table went on naming `precondition:rust-is-not-linked` for a
/// day after the row declaring it was replaced by the decision row. What
/// the sentence promised was specific — the day anything in the tree links
/// a Rust artifact, the gate turns red and the row has to be
/// re-adjudicated — and the tree linked one that same afternoon, with no
/// row left to turn. Nothing was red, because a retired check reads
/// exactly like a live one.
///
/// This is the third instance of one defect in this file, and it is worth
/// naming as such: `swap-net-footprint` was satisfied by the paragraph
/// explaining its feature, `decision:linked-beside-lua` by the paragraph
/// explaining its call, and now the document's own prose by a check that
/// had gone. Each time the repair was to read the thing that decides
/// instead of the thing that describes it.
#[test]
fn a_check_named_in_prose_is_a_check_a_row_declares() {
    let doc = read(LEDGER_DOC);
    let declared: BTreeSet<String> = ledger_rows(&doc).into_iter().map(|r| r.check).collect();
    let spelled = check_names_spelled(&doc);

    // Arity floor. Every row spells its own check inside the block, so a
    // sweep that reads fewer names than there are rows has stopped
    // reading the document rather than found it clean — and an empty
    // sweep passes every assertion below it.
    assert!(
        spelled.len() >= declared.len(),
        "{LEDGER_DOC} declares {} check(s) in its rows but this sweep found {} \
         spelled anywhere in the file. The rows themselves spell theirs, so \
         fewer means the sweep is no longer reading what it checks.",
        declared.len(),
        spelled.len()
    );

    let stale: Vec<String> = spelled
        .iter()
        .filter(|(_, name)| !declared.contains(name))
        .map(|(n, name)| format!("  {LEDGER_DOC}:{n}: `{name}`"))
        .collect();
    assert!(
        stale.is_empty(),
        "{} check name(s) are spelled in {LEDGER_DOC} that no ledger row \
         declares:\n{}\nA check name is a promise that some gate re-derives \
         the sentence beside it. A retired one reads exactly like a live one, \
         so the sentence goes on being trusted while nothing can fault it — \
         which is what happened to the precondition the OPEN row carried. \
         Either restore the row, or say what happened without spelling the \
         name as an identifier.",
        stale.len(),
        stale.join("\n")
    );
}

/// The boundary the sweep above rests on, pinned so a later
/// simplification cannot widen or narrow it silently.
#[test]
fn a_name_in_a_fence_is_not_a_promise() {
    let spelled = check_names_spelled(
        "prose naming `census:Live` once\n\
         ```sh\n\
         echo `census:InAFence`\n\
         ```\n\
         and `derive:thing=3` after it\n",
    );
    let names: Vec<&str> = spelled.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["census:Live", "derive:thing=3"],
        "a fenced block shows a command; only prose makes the promise"
    );

    // A bare kind is not an identifier: the document may say what shape a
    // check has without claiming a particular one exists.
    assert!(
        check_names_spelled("the `precondition:` shape is kept\n").is_empty(),
        "a kind with no token names no check"
    );
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
            // ⚠ The first four spellings were the whole list, and they
            // did not see the link when it arrived. `SCEBuildLowering.cmake`
            // asks cargo for the artifact path at configure time and
            // exposes it as `SCE::Lowering`, so no tracked CMake line ever
            // contains `libsce_build` — the detector would have reported
            // "nothing links Rust" with the engine linking it. Found on
            // 2026-08-29 while replacing the row that detector guarded,
            // which is the only reason it was found at all: a tripwire is
            // not tested by the thing it is watching for.
            if code.contains("corrosion_")
                || code.contains("libsce_build")
                || code.contains("sce_build.so")
                || code.contains("sce_build.a")
                || code.contains("SCE::Lowering")
                || code.contains("--crate-type")
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

        // `decision` is the fourth, added 2026-08-29 when the owner closed
        // the one item that was never waiting on a number. It is the kind
        // that needs the most care: a measurement row rots when the number
        // moves and something re-measures, but a decision row has nothing
        // to re-run, so what its check must hold is the TREE still doing
        // what the decision said.
        match row.kind.as_str() {
            "measurement" | "counting" | "judgement" | "decision" => {}
            other => panic!(
                "row `{}` carries kind `{other}`, which this gate does not \
                 recognise (measurement, counting, judgement, decision).",
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

    if row.check == "decision:linked-beside-lua" {
        assert_eq!(
            row.status, "CLOSED",
            "row `{}` carries a decision check. A decision is taken or it is \
             not; an OPEN row holding one is a decision nobody made.",
            row.id
        );

        // A decision row is the one shape that could become a sentence
        // nothing holds — there is no measurement to re-run, so what
        // stands in for one is the tree doing what the decision says.
        // Four things, and each is a way the row could quietly stop
        // being true.

        // 1. The surface exists and is reachable by a feature name.
        let cargo = read("sce-build/Cargo.toml");
        assert!(
            cargo.lines().any(|l| l.trim_start().starts_with("ffi =")),
            "row `{}`: `sce-build/Cargo.toml` declares no `ffi` feature, so \
             there is no surface for the decision to have linked.",
            row.id
        );

        // 2. Some tracked CMake file links it. This is the exact inverse
        //    of the tripwire this row replaced — that one failed if any
        //    site appeared, this one fails if none does.
        let hits = cmake_links_rust(tracked);
        assert!(
            !hits.is_empty(),
            "row `{}` says the frontend is linked and no tracked CMake file \
             links a Rust artifact. The decision was recorded and then \
             reverted, or the link moved somewhere this cannot see — which \
             has happened once already, to the detector itself.",
            row.id
        );

        // 3. It is linked BESIDE the capability, not at top level and not
        //    behind the engine SELECTION. That is the half of the decision
        //    that was argued rather than measured, so it is the half most
        //    likely to be undone by someone narrowing a guard to save a
        //    build.
        let site = read("sce/CMakeLists.txt");
        let mut depth_in_lua_guard = 0usize;
        let mut nesting = 0usize;
        let mut linked_under_lua = false;
        for line in site.lines() {
            let code = line.split('#').next().unwrap_or("").trim();
            if code.starts_with("if(") {
                nesting += 1;
                if code.contains("SCE_ENABLE_LUA") && depth_in_lua_guard == 0 {
                    depth_in_lua_guard = nesting;
                }
            }
            if depth_in_lua_guard > 0 && code.contains("SCE::Lowering") {
                linked_under_lua = true;
            }
            if code.starts_with("endif(") {
                if depth_in_lua_guard == nesting {
                    depth_in_lua_guard = 0;
                }
                nesting = nesting.saturating_sub(1);
            }
        }
        assert!(
            linked_under_lua,
            "row `{}`: `sce/CMakeLists.txt` does not link `SCE::Lowering` \
             inside an `if(SCE_ENABLE_LUA)` guard. The decision was for the \
             CAPABILITY, not the selection: scoping it to \
             `SCE_SCRIPT_ENGINE=lua` takes `EcmaScriptSemanticsOnLuaEngine` \
             its subject in every default build, and that suite is what \
             measures the divergences this whole decision is about.",
            row.id
        );

        // 4. It is BUILT with the feature that makes the C surface exist.
        //    `sce-build`'s `ffi` is off by default, so a link that forgets
        //    it produces a staticlib holding no `sce_lower_value` at all and
        //    the failure arrives as an undefined symbol at the far end of a
        //    C++ build.
        //
        //    ⚠ "Passes", not "mentions", and the difference was measured
        //    rather than reasoned. This assertion lived on the footprint row
        //    and asked whether the text `ffi` occurred anywhere under
        //    `scripts/gates/`; the comment in `tree-hygiene.sh` explaining
        //    why the feature is named there satisfied it on its own, so
        //    deleting the feature from the cargo invocation and leaving that
        //    paragraph standing kept the row GREEN while nothing built the
        //    surface. That row has since retired with its subject, and the
        //    guarantee moved HERE — onto the CMake that performs the build
        //    rather than a lane that mentions it, which is where it should
        //    have been: every C++ configure runs this, and no lane has to
        //    remember to.
        let cmake_files: Vec<&String> = tracked
            .iter()
            .filter(|p| p.starts_with("cmake/") && p.ends_with(".cmake"))
            .collect();
        let builds_with: Vec<&&String> = cmake_files
            .iter()
            .filter(|p| passes_cargo_feature(&read(p), "ffi"))
            .collect();
        let merely_named: Vec<&&String> = cmake_files
            .iter()
            .filter(|p| read(p).contains("ffi"))
            .collect();
        assert!(
            !builds_with.is_empty(),
            "row `{}`: no file under `cmake/` PASSES `--features ffi` to \
             cargo, so nothing builds the C surface this row says the engine \
             links against. Files that merely NAME the feature, which is not \
             the same thing and does not compile it: {:?}",
            row.id,
            merely_named
        );

        // 5. Something CALLS it. A linked library nothing reaches is
        //    discarded by the linker, so a row resting on the link alone
        //    would be a row that cannot fail — the population would have
        //    a role in it that can never be zero.
        //
        //    ⚠ In CODE, not in the file. This read the whole file until
        //    2026-08-29, when the mutation it exists to catch was run
        //    against it: the call was deleted and the paragraph above it
        //    left standing, and the ledger stayed green while the C++
        //    suite went red on eleven expressions. That is the same
        //    defect `passes_cargo_feature` was written for, and
        //    `a_mentioned_call_is_not_a_made_call` pins the boundary so a
        //    simplification back to a whole-file search fails there first.
        //
        //    ⚠⚠ And in the engine's scripting SOURCES, not in one named
        //    file. This read `LuaEngine.cpp` alone until the scope stopped
        //    being a process-wide constant and became a session's: the
        //    call moved into `LoweringScope.cpp`, the one translation unit
        //    that names the C surface, and a check anchored on a file name
        //    would have gone red for a move that changed nothing it is
        //    about. Staying green for a deletion and going red for a
        //    refactor are the same defect from opposite sides — the
        //    subject is that SOMETHING in the engine calls the surface.
        let sources: Vec<&String> = tracked
            .iter()
            .filter(|p| p.starts_with("sce/src/scripting/") && p.ends_with(".cpp"))
            .collect();
        assert!(
            sources.len() >= 2,
            "row `{}`: this sweep found {} tracked source(s) under \
             `sce/src/scripting/`. The engine and the surface it calls both \
             live there, so fewer than two means the sweep stopped reading \
             rather than found the call gone — and an empty sweep would \
             report the deletion below as the tree's answer.",
            row.id,
            sources.len()
        );
        let callers: Vec<&str> = sources
            .iter()
            .filter(|p| cpp_code_only(&read(p)).contains("sce_lower_value("))
            .map(|p| p.as_str())
            .collect();
        assert!(
            !callers.is_empty(),
            "row `{}`: nothing under `sce/src/scripting/` calls the surface. \
             The link would then be dead weight the linker discards, and this \
             row would describe a build-system fact with no behaviour behind \
             it.",
            row.id
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

    if let Some(sha) = row.check.strip_prefix("retired-measurement:") {
        // A retired measurement is STILL a measurement, and rewriting its
        // kind would hide that the number came from a probe that ran. What
        // changed is that its subject left the working tree, so the probe
        // cannot be re-run HERE — and this check is what keeps that from
        // being a sentence nobody re-derives.
        assert_eq!(
            row.kind, "measurement",
            "row `{}` carries a retired-measurement check with kind `{}`. The \
             number was measured; a retired measurement is a measurement whose \
             subject has gone, not a decision.",
            row.id, row.kind
        );
        assert_eq!(
            row.status, "CLOSED",
            "row `{}` is {} and carries a retired-measurement check. A subject \
             that has left the tree cannot be measured again here, so the row \
             cannot be open on the promise of a re-run.",
            row.id, row.status
        );

        // 1. The pin is a real commit.
        //
        //    ⚠ `git cat-file -t` and not `git show`. `git show <sha>:<path>`
        //    answers "no such commit" and "that commit has no such path" with
        //    the SAME sentence, and this repository has already read one for
        //    the other once and reverted a correct diagnosis over it.
        let (found, kind) = git_at_root(&["cat-file", "-t", sha]);
        assert!(
            found && kind == "commit",
            "row `{}` pins its number to `{sha}`, which this repository does \
             not carry as a commit (`git cat-file -t` said {:?}). A pin that \
             resolves to nothing makes the number un-re-derivable, which is \
             the exact defect the ledger exists to refuse.",
            row.id,
            if found { kind.as_str() } else { "failure" }
        );

        // 2. That commit still holds everything the measurement rested on, so
        //    a reader can check it out and run the probe again.
        for path in DEPARTED_WITH_REWRITER {
            let (ok, listed) = git_at_root(&["ls-tree", "--name-only", sha, "--", path]);
            assert!(
                ok && listed == *path,
                "row `{}`: `{path}` is not in the tree of `{sha}`, so the pin \
                 does not hold the subject the number was measured on. Pin a \
                 commit that does — the number cannot be re-derived from one \
                 that never had it.",
                row.id
            );
        }

        // 3. And this tree no longer holds them. Without this the row could
        //    sit as a historical note over a subject that quietly came back,
        //    which is a live price left unmeasured — the shape this ledger
        //    was built to catch.
        for path in DEPARTED_WITH_REWRITER {
            assert!(
                !tracked.contains(*path),
                "row `{}`: `{path}` is tracked again, so its subject is back \
                 in the working tree and this row is no longer historical. \
                 Re-measure it and give it a live check, or say why it cannot \
                 be measured — a retired-measurement row over a present \
                 subject is a number nobody is re-deriving.",
                row.id
            );
        }

        // 4. The probe itself is KEPT, and says so where a person meets it.
        //    Evidence that is gone leaves the row citing nothing; evidence
        //    that is present and silent invites a run that cannot succeed in
        //    this tree, and costs whoever tries it the time to find out.
        assert!(
            tracked.contains(&row.evidence),
            "row `{}` cites `{}` as the probe behind its number, and git does \
             not track it. The measurement is retired, not erased — the script \
             is how the number can be re-derived at the pin above.",
            row.id,
            row.evidence
        );
        let probe = read(&row.evidence);
        assert!(
            probe.contains("retired-measurement:"),
            "row `{}`: `{}` does not name `retired-measurement:` anywhere, so \
             nothing in the script tells a reader that its subject left the \
             tree at a named commit. It will be run, it will fail, and the \
             reason will be somewhere else.",
            row.id,
            row.evidence
        );
        return;
    }

    if row.check == "retirement:rewriter-deleted" {
        assert_eq!(
            row.kind, "decision",
            "row `{}` carries a retirement check with kind `{}`. A retirement \
             has no measurement to re-run — what stands in for one is the tree \
             still doing what was decided, which is what `decision` means here.",
            row.id, row.kind
        );

        // 1. The unit is GONE, and nothing compiles it.
        //
        //    ⚠ This inverts what stood here while the row said `uncalled`.
        //    Then the unit had to still exist, because "no file names X" is
        //    trivially true of an X nothing carries and a deletion would have
        //    reported the retirement complete for a reason with nothing to do
        //    with callers. Deletion is now the claim, so the requirement flips
        //    — and the control that requirement was buying has to be bought
        //    somewhere else instead, which is what step 3 is.
        let unit: Vec<&str> = tracked
            .iter()
            .map(|p| p.as_str())
            .filter(|p| is_cpp_source(p) && file_stem_of(p).starts_with(REWRITER))
            .collect();
        assert!(
            unit.is_empty(),
            "row `{}`: {} tracked C++ file(s) are still named after \
             `{REWRITER}`:\n  {}\nThe row claims the unit was DELETED, not \
             merely left uncalled. If it came back, this row is the one that \
             has to be rewritten — the `retirement:rewriter-uncalled` shape it \
             replaced is what a returned-but-uncalled unit needs.",
            row.id,
            unit.len(),
            unit.join("\n  ")
        );
        let sources = read("sce/sce_base_sources.cmake");
        assert!(
            !sources
                .lines()
                .map(|l| l.split('#').next().unwrap_or(""))
                .any(|l| l.contains(REWRITER)),
            "row `{}`: `sce/sce_base_sources.cmake` still lists `{REWRITER}`, \
             so every C++ configure is still told to compile a translation \
             unit this tree no longer has. A build that cannot run is not a \
             completed deletion.",
            row.id
        );

        // 2. The population: EVERY tracked C++ file.
        //
        //    ⚠ It was `sce/src` and `sce/include` for a day, and that was a
        //    boundary nothing re-derived. One file outside those directories
        //    really does construct the rewriter — the benchmark that PRICED
        //    the retirement — and under a directory boundary it was not
        //    exempted, it was INVISIBLE: the sweep never opened it, so
        //    nothing in the tree said whether it was an instrument or a
        //    caller, and a second file joining it would have been as quiet.
        //
        //    That boundary was replaced by a classification, and the deletion
        //    then collapsed the classification too: with the unit and its one
        //    instrument both gone, no tracked C++ file may reach the rewriter
        //    for any reason, and there is no exemption of any shape left to
        //    hide behind.
        let population: Vec<&str> = tracked
            .iter()
            .map(|p| p.as_str())
            .filter(|p| is_cpp_source(p))
            .collect();
        assert!(
            population.len() >= TRACKED_CPP_FLOOR,
            "row `{}`: this sweep found {} tracked C++ file(s), below the \
             floor of {TRACKED_CPP_FLOOR}. A sweep that stopped reading \
             reports an empty result, and an empty result reads exactly like \
             a completed retirement.",
            row.id,
            population.len()
        );
        //    The engine's own sources remain the SUBJECT even though the
        //    sweep is wider, and they carry their own floor: a population
        //    held up by the committed generated trees while the engine's
        //    share of it went to zero would pass the line above having
        //    stopped reading the only files this row is really about.
        let engine: Vec<&str> = population
            .iter()
            .copied()
            .filter(|p| p.starts_with("sce/src/") || p.starts_with("sce/include/"))
            .collect();
        assert!(
            engine.len() >= ENGINE_SOURCE_FLOOR,
            "row `{}`: this sweep found {} engine source(s) under `sce/src/` \
             and `sce/include/`, below the floor of {ENGINE_SOURCE_FLOOR}. A \
             sweep that stopped reading reports an empty result, and an empty \
             result reads exactly like a completed retirement.",
            row.id,
            engine.len()
        );

        // 3. THE CONTROL, and the whole reason this row is not vacuous.
        //
        //    ⚠⚠⚠ While the unit existed, the control was its own files: the
        //    predicate that reports zero below had to report THEM, or it had
        //    stopped being able to see the name at all. Deleting the unit
        //    deleted that control, and a check that lost its control while
        //    keeping its claim is a check that now passes for the same reason
        //    an unread tree does.
        //
        //    What buys it back is the PROSE. The engine, its suites and this
        //    repository's documents still explain what the rewriter was and
        //    why it went, so a sweep that is really opening files finds the
        //    name in RAW text many times over while finding it in CODE never.
        //    The raw half is asserted first: it is what turns a silent reader
        //    into a red one.
        //
        //    ⚠ This CAN legitimately reach zero — by deleting every comment
        //    that remembers the rewriter. That is allowed, and it is exactly
        //    when this row has to be rewritten rather than quietly relaxed:
        //    with no mention anywhere, nothing in the tree can show that the
        //    sweep still reads, and the only control left would be the
        //    fixtures in `a_remembered_rewriter_is_not_a_reached_one`.
        let mentions: Vec<&str> = population
            .iter()
            .copied()
            .filter(|p| read(p).contains(REWRITER))
            .collect();
        assert!(
            mentions.len() >= REWRITER_MENTION_FLOOR,
            "row `{}`: only {} tracked C++ file(s) mention `{REWRITER}` in raw \
             text, below the floor of {REWRITER_MENTION_FLOOR}. Every one of \
             them is prose — the history of a deleted unit — and they are this \
             sweep's only proof that it is opening files at all. Without them \
             the zero below is indistinguishable from a sweep that read \
             nothing, so this row must be rewritten rather than left standing.",
            row.id,
            mentions.len()
        );

        // 4. The claim itself. No classification is left: with the unit gone
        //    there is nothing a file could legitimately reach, so there is no
        //    instrument list here any more and no exemption of any shape.
        let callers: Vec<&str> = population
            .iter()
            .copied()
            .filter(|p| reaches_rewriter(&read(p)))
            .collect();
        assert!(
            callers.is_empty(),
            "row `{}`: {} tracked C++ file(s) reach `{REWRITER}` in code:\n  \
             {}\nThe unit is deleted, so a file that names the type or \
             includes its header cannot compile — this is a build break as \
             well as a false row. The {} file(s) that merely MENTION it in \
             comments are the control above and are not this.",
            row.id,
            callers.len(),
            callers.join("\n  "),
            mentions.len()
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

/// What left the working tree when the rewriter was deleted.
///
/// ⚠ This is the population two RETIRED measurement rows rest on, and every
/// entry is asserted from both sides: the pinned commit's tree must still
/// hold it (so the number the row carries can be re-derived by checking that
/// commit out) and this tree must NOT (so a row cannot sit as a historical
/// note over a subject that quietly came back).
///
/// The benchmark is here for the reason the ledger keeps saying out loud: it
/// was the INSTRUMENT that priced the retirement, it constructed the rewriter
/// to do so, and an instrument outlives its subject only as rot.
const DEPARTED_WITH_REWRITER: &[&str] = &[
    "sce/include/scripting/EcmaScriptToLuaTransformer.h",
    "sce/src/scripting/EcmaScriptToLuaTransformer.cpp",
    "tests/benchmarks/EcmaLoweringPerCallBenchmark.cpp",
];

/// A floor for the engine sweep, well under the count on the day it was
/// written (349) and well over anything a broken filter would return.
const ENGINE_SOURCE_FLOOR: usize = 200;

/// A floor for the whole-tree sweep, well under the count on the day the
/// population was widened from the engine's two directories to every tracked
/// C++ file (1937, the committed generated trees included) and well over
/// anything a broken filter would return.
const TRACKED_CPP_FLOOR: usize = 1000;

/// A floor for the raw-text mentions that are the deletion sweep's only
/// remaining control.
///
/// Ten tracked C++ files still explain the rewriter in comments on the day
/// the unit was deleted — the engine's two scripting headers, `LuaEngine`
/// itself, four engine suites, a benchmark, the C backend's DOM binding, and
/// `SceLowering.h`. The floor is half of that: high enough that a sweep which
/// stopped reading cannot clear it, low enough that trimming prose does not
/// fail a round that changed nothing about callers.
const REWRITER_MENTION_FLOOR: usize = 5;

fn is_cpp_source(path: &str) -> bool {
    matches!(
        path.rsplit_once('.').map(|(_, ext)| ext),
        Some("cpp" | "cc" | "cxx" | "h" | "hpp" | "c")
    )
}

fn file_stem_of(path: &str) -> &str {
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    name.split_once('.').map_or(name, |(stem, _)| stem)
}

/// Whether one C++ file reaches the retired rewriter.
///
/// ⚠ Per FILE, while the row's claim is per POPULATION, and the difference
/// is deliberate. A call made through a MEMBER — `transformer_.transform(…)`,
/// which is how the call stood — names no type in the file that makes it.
/// What makes it compile is a declaration of the type, and C++ has no way to
/// do without one: `sce/include/` is swept together with `sce/src/`, so the
/// declaration is in the population even when the call is invisible. Keying
/// this on the member's name would see the other half and go blind the day
/// the member is renamed. `a_remembered_rewriter_is_not_a_reached_one` pins
/// both halves.
///
/// Two ways, because a caller needs both and either one alone would let the
/// other stand:
///
///   * it NAMES the type in code — a member, a local, a qualified call. A
///     mention in a comment is not a use, and this document deliberately
///     keeps the history of the rewriter in prose, so `cpp_code_only` runs
///     first for the same reason `a_mentioned_call_is_not_a_made_call`
///     pins it one check over.
///   * it INCLUDES the header. An include with no use is not a call, but it
///     is the engine still compiling a second translator into itself, and
///     the deletion round this row is a step towards cannot happen while one
///     stands.
///
/// The include has to be read off the RAW line rather than the stripped one:
/// `cpp_code_only` keeps a string's quotes and drops what is between them, so
/// `#include "…/EcmaScriptToLuaTransformer.h"` survives as `#include ""`. It
/// keeps one newline per newline it consumes — its own comment says so, and
/// `a_mentioned_call_is_not_a_made_call` pins it — so the two readings line up
/// by index, and an include commented out is an include the stripped side
/// never reports.
fn reaches_rewriter(src: &str) -> bool {
    let code = cpp_code_only(src);
    if code.contains(REWRITER) {
        return true;
    }
    let raw: Vec<&str> = src.lines().collect();
    code.lines().enumerate().any(|(n, line)| {
        line.trim_start().starts_with("#include")
            && raw
                .get(n)
                .is_some_and(|r| r.contains(&format!("{REWRITER}.h")))
    })
}
