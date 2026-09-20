// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A file carrying non-Latin prose says why it must.
//!
//! This tree is written in English, and every rule it states is stated so
//! that a reader who did not write it can act on it. A sentence in another
//! script breaks that in a way nothing else in the tree measures: it is not
//! wrong, it is *unreadable to part of the audience*, and unreadable prose
//! makes a rule that cannot be checked.
//!
//! Measured 2026-09-18, before this gate existed, over the tracked tree:
//!
//! * a private shorthand — an option named by a Greek letter, written in
//!   Korean — appeared in six files across the C AOT suite, **with no legend
//!   anywhere in the tree**. Nobody outside the session that coined it could
//!   resolve it, and the sessions that copied it forward did not try;
//! * four build- and visualizer-configuration files carried their comments
//!   in Korean only;
//! * seven files quoted an instruction in Korean where the English sentence
//!   beside it already carried the rule.
//!
//! None of those was caught by anything, because nothing was looking.
//!
//! # What the registry buys, and what it does not
//!
//! Some files must carry another script — the script is the *subject*, not
//! the prose. A column-arithmetic test needs a character that is three bytes
//! and one scalar; a guard against committing somebody's copyrighted
//! sentence needs the endings that sentence is written with. Deleting those
//! would delete what they measure.
//!
//! So this gate does not ban a script. It requires that a file carrying one
//! **argue for it, in the registry below, in one line a reader can check**.
//! [`a_declaration_whose_file_no_longer_carries_it_is_refused`] is what
//! stops the registry becoming a list nobody prunes: an entry whose file no
//! longer carries the script fails, so the list can only ever describe the
//! tree as it is.
//!
//! ⚠ **The honest reading of a green run.** This gate looks for the scripts
//! listed in [`scripts_this_gate_reads`] and no others, so green means "no
//! *undeclared* Korean, Han, kana or Cyrillic prose", never "the tree is all
//! English". A script absent from that list is not detected at all.
//!
//! ⚠⚠ **Greek is deliberately absent, and the measurement is the reason.**
//! `α`, `β`, `γ` and `η` appear in 74 tracked files as mathematical names —
//! a complexity bound, a coefficient, an angle. They are notation, not
//! prose, and a gate that refused them would refuse 74 files for writing
//! mathematics the way mathematics is written. The discriminator this gate
//! can actually apply is *which script*, not *which use*, so the honest
//! move is to leave a script out rather than to add it and then paper over
//! the result with 74 registry entries that argue nothing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A script this gate reads, and the ranges that spell it.
///
/// Han, the kana and Cyrillic are here although the tree holds no
/// undeclared instance of any of them. That is the point: a gate added on
/// the day a script arrives is a gate written by whoever was surprised, and
/// the cost of carrying an empty range is one array row.
fn scripts_this_gate_reads() -> &'static [(&'static str, &'static [(u32, u32)])] {
    &[
        (
            "Hangul",
            &[
                (0xAC00, 0xD7A3), // syllables
                (0x1100, 0x11FF), // jamo
                (0x3130, 0x318F), // compatibility jamo
                (0xA960, 0xA97F), // jamo extended-A
                (0xD7B0, 0xD7FF), // jamo extended-B
            ],
        ),
        ("Han", &[(0x4E00, 0x9FFF), (0x3400, 0x4DBF)]),
        ("Hiragana", &[(0x3040, 0x309F)]),
        ("Katakana", &[(0x30A0, 0x30FF)]),
        ("Cyrillic", &[(0x0400, 0x04FF)]),
    ]
}

/// The script `c` belongs to, when this gate reads that script.
fn script_of(c: char) -> Option<&'static str> {
    let n = c as u32;
    scripts_this_gate_reads()
        .iter()
        .find(|(_, ranges)| ranges.iter().any(|(lo, hi)| n >= *lo && n <= *hi))
        .map(|(name, _)| *name)
}

/// Files that carry another script because the script is what they measure.
///
/// The second field is the argument, and it is the whole value of the
/// entry: a path alone would record that somebody once added a path.
///
/// ⚠ Adding a row is a claim that deleting the characters would delete
/// something the file measures. "It reads better that way" is not one — the
/// six files this gate was written for all read fine in English once the
/// shorthand was gone, and the rules they state got *more* checkable.
/// ⚠ WHAT IS *NOT* HERE, AND WHY THE LIST IS THIS SHORT.
///
/// Seven files were on it when this gate landed, and six came off the same
/// day. Each needed a value that is **not ASCII and several UTF-8 bytes
/// wide** — a column-arithmetic fixture, a Lua round trip, a `\uXXXX`
/// decode, a refused parameter name, a shell's message under another
/// locale. None of them needed a particular *script*: the characters were
/// arbitrary, so they are now written as `\u{...}` escapes, which keeps
/// those sources ASCII and states the width instead of showing it to
/// whoever happens to read the script that was picked. Two of them grew an
/// assert on the width while they were being edited, because a fixture
/// nobody checks can be swapped for an ASCII one and leave the test green.
///
/// That is the question to ask of any proposed row: does this file need a
/// value that is not ASCII, or does it need THIS script? Only the second
/// belongs here.
const THE_SCRIPT_IS_THE_SUBJECT: &[(&str, &str)] = &[
    (
        "sce-build/src/template.rs",
        "a negative fixture for the parameter-name rule: a non-ASCII name \
         must be refused, and only a non-ASCII name can show that",
    ),
    (
        "sce-build/src/requirement_manifest.rs",
        "the normative-modality vocabulary of a Korean-normative convention \
         IS the data; the markers are grammatical endings, so an English \
         paraphrase would not match any sentence",
    ),
    (
        "sce-build/tests/the_modality_vocabulary_is_the_documents.rs",
        "drives that vocabulary over sentences written with those endings, \
         which is the only way to show the table matches prose and not a \
         list of verbs",
    ),
    (
        "sce-build/src/forge/page.rs",
        "holds the Korean lexicon, whose spellings ARE the answer it is \
         asked for: what each grammar word is called. An escape would hide \
         the one thing a reader of a lexicon has to check, and an English \
         paraphrase would not be that lexicon but a different one",
    ),
];

// ⚠ This gate is NOT in the list above, and that is deliberate. Its own
// positive control writes every probe as `\u{...}` escapes, so the file
// carries no character in any script it reads and needs no exemption. A
// gate that has to exempt itself is one an author can widen by widening the
// exemption; this one cannot be.

/// Vendored trees. Not ours to translate, and rewriting a dependency's
/// comments is how a vendor bump turns into a merge conflict.
const NOT_OURS: &[&str] = &["third_party/"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every path `git` tracks.
///
/// `git ls-files` rather than a directory walk, for the reason the
/// neighbouring tree-wide gates give: the claim is about the tree as
/// committed, and that also makes this gate's inputs wider than any
/// `paths:` filter — which is why it is registered in `UNFILTERABLE_GATES`.
fn tracked_files(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files"])
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// One offending line, for a message the reader can act on without
/// re-running anything.
struct Hit {
    path: String,
    line: usize,
    script: &'static str,
}

/// Every hit in one file's text.
///
/// Split out so the positive control can drive it over text the tree does
/// not hold. A sweep can only ever come back green, and a green from a
/// scan is this repository's most-repeated shape of measuring nothing.
fn hits_in(path: &str, text: &str) -> Vec<Hit> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if let Some(script) = line.chars().find_map(script_of) {
            out.push(Hit {
                path: path.to_string(),
                line: n + 1,
                script,
            });
        }
    }
    out
}

/// The tracked files this gate judges, with their text.
///
/// A file that does not decode as UTF-8 is skipped rather than reported.
/// That is not a hole being waved through: a PNG, a jar and an epub all
/// matched a byte-level Korean search during the measurement above, purely
/// because some of their compressed bytes fall in the range. Decoding first
/// removes every one of those without a suffix list to maintain — and a
/// Shift-JIS document under a vendored tree drops out by the same rule.
fn judged_files(root: &Path) -> Vec<(String, String)> {
    tracked_files(root)
        .into_iter()
        .filter(|p| !NOT_OURS.iter().any(|d| p.starts_with(d)))
        .filter_map(|p| {
            let bytes = std::fs::read(root.join(&p)).ok()?;
            let text = String::from_utf8(bytes).ok()?;
            Some((p, text))
        })
        .collect()
}

#[test]
fn every_file_carrying_non_latin_prose_is_declared() {
    let root = repo_root();
    let declared: BTreeSet<&str> = THE_SCRIPT_IS_THE_SUBJECT.iter().map(|(p, _)| *p).collect();

    let mut undeclared: Vec<Hit> = Vec::new();
    for (path, text) in judged_files(&root) {
        if declared.contains(path.as_str()) {
            continue;
        }
        undeclared.extend(hits_in(&path, &text));
    }

    assert!(
        undeclared.is_empty(),
        "{} line(s) carry a script this tree does not write prose in.\n\
         Either say it in English, or add the file to \
         THE_SCRIPT_IS_THE_SUBJECT with the argument for why the characters \
         are what the file measures:\n{}",
        undeclared.len(),
        undeclared
            .iter()
            .take(40)
            .map(|h| format!("  {}:{} ({})", h.path, h.line, h.script))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn a_declaration_whose_file_no_longer_carries_it_is_refused() {
    // Without this the registry is write-only. A file gets translated, its
    // row stays, and the row goes on granting an exemption to whatever is
    // written there next — which is the "declared but unread" shape this
    // tree refuses elsewhere, arriving through the door marked "allowed".
    let root = repo_root();
    let mut stale: Vec<&str> = Vec::new();
    for (path, _) in THE_SCRIPT_IS_THE_SUBJECT {
        let full = root.join(path);
        assert!(
            full.is_file(),
            "{path} is declared here but is not a file in the tree"
        );
        let text = std::fs::read_to_string(&full)
            .unwrap_or_else(|e| panic!("{path} is declared here and cannot be read: {e}"));
        if hits_in(path, &text).is_empty() {
            stale.push(path);
        }
    }
    assert!(
        stale.is_empty(),
        "{} declaration(s) no longer describe their file — delete the \
         row(s):\n{}",
        stale.len(),
        stale
            .iter()
            .map(|p| format!("  {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_detector_is_exercised_on_text_the_tree_does_not_hold() {
    // The sweep above passes against a detector that finds nothing, and a
    // detector that finds nothing is exactly what a translated tree looks
    // like. So the decision is driven here, in both directions, over text
    // written down rather than found.
    //
    // ⚠ The negative half is the half that matters. A gate that refused the
    // punctuation and mathematics this tree writes everywhere — the warning
    // signs, the em dashes, the guillemets, `α`/`β`/`γ` — would be turned
    // off within a day, and a gate that is off is worse than no gate,
    // because the tree still says it has one.
    let must_fire = [
        ("Hangul", "let label = \"\u{d55c}\u{ae00}\";"),
        ("Hangul", "// \u{1100}\u{1161} jamo, spelled out"),
        ("Han", "// \u{6f22}\u{5b57} in a comment"),
        ("Hiragana", "// \u{3072}\u{3089}\u{304c}\u{306a}"),
        ("Katakana", "(\"\u{30dd}\u{30fc}\u{30c8}\", false),"),
        ("Cyrillic", "// \u{043a}\u{0438}\u{0440}\u{0438}\u{043b}"),
    ];
    for (script, line) in must_fire {
        let hits = hits_in("probe.rs", line);
        assert_eq!(
            hits.len(),
            1,
            "the detector did not fire on {script}: {line:?}"
        );
        assert_eq!(hits[0].script, script, "wrong script named for {line:?}");
        assert_eq!(hits[0].line, 1);
    }

    let must_not_fire = [
        "// \u{26a0}\u{26a0}\u{26a0} a warning sign is not prose",
        "// an em dash \u{2014} and \u{ab}guillemets\u{bb} and a middot \u{b7}",
        "// O(\u{3b1}(n)) with coefficients \u{3b2} and \u{3b3}, angle \u{3b7}",
        "// accented Latin: caf\u{e9}, na\u{ef}ve, \u{fc}ber",
        "let x = 1; // plain ASCII",
        "",
    ];
    for line in must_not_fire {
        assert!(
            hits_in("probe.rs", line).is_empty(),
            "the detector fired on text this tree writes: {line:?}"
        );
    }

    // A multi-line input reports the line the reader has to open, not the
    // first line of the file.
    let text = "fn main() {}\n// ok\nlet s = \"\u{d55c}\";\n";
    let hits = hits_in("probe.rs", text);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 3, "the hit does not name the offending line");
}
