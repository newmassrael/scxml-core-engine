// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A configuration rule cites the clause that states it.
//
// The rules a state configuration must satisfy are written down in one
// place: §scxml-3.11 "Legal State Configurations and Specifications".
// Its normative excerpt lists them verbatim — the configuration contains
// exactly one child of the `<scxml>` element; when it contains a
// non-atomic `<state>` it contains one and only one of that state's
// children; if it contains a `<parallel>` state it contains all of its
// children; it contains one or more atomic states, and every atomic
// state brings its ancestors.
//
// Six backends spelled those rules with a citation to §scxml-3.2, 3.3 or
// 3.4 instead. Those clauses define the ELEMENTS — "The top-level wrapper
// element, which carries version information", "Holds the representation
// of a state", "encapsulates a set of child states which are
// simultaneously active" — and none of them states a configuration rule.
// §scxml-3.2 goes as far as to say "See 3.11 Legal State Configurations
// and Specifications for details", so reading the cited clause sent the
// reader onwards to the right one.
//
// Nothing measured that. `citation_unbound` and `symbol_mismatch` ask
// whether a binding EXISTS, not whether it aims at the clause that says
// the thing — a citation can be perfectly bound and still point at prose
// about something else, and the tree reported `violations: total=0` for
// two weeks while all six backends were wrong. Reading the normative
// excerpt by hand is what found it, and a habit is not a gate.
//
// So this asks the question the citation gates cannot: for each rule
// below, every clause cited on the same line as it must be §scxml-3.11.
//
// What it deliberately does NOT do is forbid §scxml-3.2/3.3/3.4 near
// configuration code. Those citations are correct wherever the claim is
// about the element: `// §scxml-3.4: every region, simultaneously` cites
// the clause that says regions are simultaneously active, which is
// exactly what §scxml-3.4 says. Aiming at the RULE TEXT rather than at
// the file is what keeps this from becoming a ban on a clause number.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The rules, as the tree spells them, lower-cased.
///
/// These are the sentences the engines actually write — the refusal text
/// a host reads and the doc comment above the variant that produces it.
/// Matching on the SENTENCE rather than on a file path is what makes a
/// seventh backend inherit this gate by writing the same rule, and what
/// keeps a legitimate §scxml-3.4 citation about simultaneity from being
/// swept up beside it.
const RULES: &[(&str, &str)] = &[
    (
        "closes on exactly one root",
        "§scxml-3.11: \"The configuration contains exactly one child of the \
         <scxml> element.\"",
    ),
    (
        "holds exactly one active child",
        "§scxml-3.11: \"When the configuration contains a non-atomic <state>, \
         it contains one and only one of the state's children.\"",
    ),
    (
        "holds every region and one is missing",
        "§scxml-3.11: \"If the configuration contains a <parallel> state, it \
         contains all of its children.\"",
    ),
    (
        "holds every region and nothing else",
        "§scxml-3.11: \"If the configuration contains a <parallel> state, it \
         contains all of its children.\"",
    ),
    (
        "makes the current state the atomic",
        "§scxml-3.11: \"The configuration contains one or more atomic states\", \
         and every atomic state brings its ancestors.",
    ),
];

/// The clause every rule above must cite.
const CORRECT_CLAUSE: &str = "3.11";

/// The line as the matcher reads it: lower-cased, with everything that is
/// not a letter or a digit folded to a single space.
///
/// The tree spells one rule two ways. `ConfigurationHelper.h` writes
/// "holds EVERY region, and one is missing" in the doc comment and
/// "holds every region and one is missing" in the refusal string beneath
/// it — the same rule, one comma apart. A matcher reading the raw line
/// sees only the second, so the doc comment above a corrected string
/// would have kept its §scxml-3.4 citation with nothing watching it.
/// Folding punctuation is what makes the pair one subject.
///
/// It stays narrow because the RULES are whole clauses, not keywords: no
/// amount of folding turns "every region, simultaneously" — the §scxml-3.4
/// citation that is correct — into one of them.
fn normalized(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.extend(c.to_lowercase());
        } else {
            pending_space = true;
        }
    }
    out
}

/// Lower bound on files read as text.
///
/// Measured 2026-09-02 at 7152 files decoded, of over seven thousand
/// tracked. The floor sits far below that so ordinary growth or pruning
/// never moves it, and far above zero so a scan that stopped reading
/// cannot report "no violations" and pass — the failure this whole family
/// of gates keeps being bitten by.
///
/// Both numbers in this comment are printed by the test itself; run it
/// with `-- --nocapture` rather than believing this sentence.
const MIN_SCANNED_FILES: usize = 5000;

/// Lower bound on rule sentences the scan must find.
///
/// Without it this gate retires itself. The rules live in six backends
/// plus their tests, and a refactor that reworded them — or a scan that
/// silently matched nothing — would leave a green gate measuring an empty
/// set.
///
/// Measured 2026-09-02 at 51 sentences across 12 files, roughly eight per
/// backend. The floor admits two backends retiring outright and still
/// refuses a tree that has lost the rules — it was 20 while the matcher
/// read raw lines and missed every comma'd variant, which is 30 too low
/// once [`normalized`] folds the two spellings into one subject.
const MIN_RULE_SITES: usize = 30;

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

/// Every W3C clause number cited on one line, in any of the three
/// spellings the tree uses.
///
/// `§scxml-3.11`, `(W3C SCXML 3.11)` and `W3C SCXML 3.11:` all appear,
/// and the first draft of this scan read only the first — which would
/// have passed a tree whose doc comments were fixed and whose refusal
/// STRINGS, the part a user actually reads, were not.
fn clauses_cited(line: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for marker in ["§scxml-", "w3c scxml ", "w3c-scxml-"] {
        let hay = line.to_lowercase();
        let mut from = 0;
        while let Some(at) = hay[from..].find(marker) {
            let start = from + at + marker.len();
            let digits: String = hay[start..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let number = digits.trim_end_matches('.').to_string();
            if !number.is_empty() {
                found.insert(number);
            }
            from = start.max(from + at + 1);
        }
    }
    found
}

#[test]
fn a_configuration_rule_cites_the_clause_that_states_it() {
    let root = repo_root();
    let mut scanned = 0usize;
    let mut sites = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    for path in tracked_sources() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // a blob with no prose in it
        };
        scanned += 1;
        let shown = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        // This file states the rules in order to check them, so it cites
        // §scxml-3.11 beside each one and would otherwise read itself as
        // the tree's thirteenth site. Skipping by the scanner's own path
        // rather than by a pattern keeps that narrow.
        if shown.ends_with("tests/configuration_rules_cite_their_clause.rs") {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            let folded = normalized(line);
            for (rule, correct_text) in RULES {
                if !folded.contains(&normalized(rule)) {
                    continue;
                }
                sites += 1;
                let cited = clauses_cited(line);
                let wrong: Vec<&String> = cited.iter().filter(|c| *c != CORRECT_CLAUSE).collect();
                if wrong.is_empty() {
                    continue;
                }
                offenders.push(format!(
                    "{shown}:{}\n     rule: \"{rule}\"\n    cites: {wrong:?}\n    \
                     should be {correct_text}",
                    number + 1
                ));
            }
        }
    }

    // What the two floors below were set against, printed rather than
    // written down: `cargo test … -- --nocapture` re-derives the numbers
    // the comments on those constants quote, so a later round measures
    // instead of trusting prose nobody re-runs.
    println!(
        "scanned {scanned} file(s); found {sites} configuration-rule sentence(s); \
         {} offender(s)",
        offenders.len()
    );

    assert!(
        scanned >= MIN_SCANNED_FILES,
        "the scan read {scanned} files, below the {MIN_SCANNED_FILES} floor — \
         a scan that stopped reading reports no violations and passes"
    );
    assert!(
        sites >= MIN_RULE_SITES,
        "the scan found {sites} configuration-rule sentence(s), below the \
         {MIN_RULE_SITES} floor — the rules were reworded or the matcher \
         stopped matching, and either way this gate is measuring nothing"
    );
    assert!(
        offenders.is_empty(),
        "a configuration rule cites a clause that does not state it:\n  {}\n\n\
         §scxml-3.2 defines the wrapper element, §scxml-3.3 the <state> \
         element and §scxml-3.4 the <parallel> element. The rules a \
         configuration must satisfy are §scxml-3.11. Citing the element \
         clause for a configuration rule is bound, resolvable, and aimed at \
         prose that does not say it — which is why the citation gates report \
         nothing.",
        offenders.join("\n  ")
    );
}
