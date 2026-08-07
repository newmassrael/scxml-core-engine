// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A spec citation names a section, never a line number.
//
// `§mesh-16.5` is checked: mnemosyne resolves the anchor and fails the
// push when it does not exist. `L3500` is checked by nobody, and drifts
// the moment anyone inserts a paragraph above it. The two look equally
// authoritative in a comment, which is the problem — a reader who
// follows the line number lands somewhere unrelated and has no way to
// tell that the citation was ever right.
//
// Measured before this gate: every one of the 32 line citations in the
// tree was stale. Not most — all of them. `§16.5 L3500` pointed at a
// Zenoh SHM table 262 lines above the section it named; `§9.6 L1393`
// was 99 lines off. They were correct when written and the spec grew.
//
// So the fix is not to correct the numbers, which would decay again on
// the next edit, but to stop carrying them: the anchor already locates
// the section, and it is the half that stays true.
//
// Prose that happens to contain `L` followed by digits is not the
// target; the pattern is anchored on a spec citation, so only a line
// number attached to one is rejected.

use std::path::{Path, PathBuf};

/// Directories scanned for citations. Walked rather than listed —
/// a gate that reads a hand-kept file list reports full coverage while
/// checking whatever the author had open (measured, in this repo, at 8%).
const SCAN_ROOTS: &[&str] = &["sce-build/src", "sce/include", "tools", "tests", "backends"];

const EXTENSIONS: &[&str] = &[
    "rs", "h", "hpp", "cpp", "inl", "c", "jinja2", "kt", "go", "py",
];

/// Spec documents whose citations this gate governs.
///
/// `SCE_MESH.md` only, and the exclusion is a decision rather than an
/// oversight. `rfc-sce-protocol-synthesis.md` is held at a fixed 4053
/// lines — corrections are rewritten to the same line count on purpose,
/// and roughly 1264 citations across the tree depend on that. Its line
/// numbers are a maintained contract, not drift waiting to happen, so
/// forbidding them there would break a working convention to enforce a
/// rule it does not need.
///
/// A document that later stops pinning its line count belongs here.
const SPEC_DOCS: &[&str] = &["SCE_MESH.md"];

/// The synth RFC's pinned length. See [`SYNTH_RFC`].
const SYNTH_RFC_LINES: usize = 4053;

/// The document the exclusion above is granted to.
const SYNTH_RFC: &str = "docs/spec/synth/rfc-sce-protocol-synthesis.md";

/// Measured floor on citations into the synth RFC, so a scan that stops
/// matching cannot report a clean tree. Measured at 1316 when this gate
/// landed.
const MIN_SYNTH_RFC_CITATIONS: usize = 1200;

/// Measured floor on citations seen, so a scan that stops matching
/// cannot pass as a clean tree.
const MIN_CITATIONS_SCANNED: usize = 800;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                // `build` / `target` hold generated copies of the same
                // comments; scanning them double-reports one defect.
                if !matches!(name.as_ref(), "build" | "target" | "node_modules")
                    && !name.starts_with('.')
                {
                    walk(&path, out);
                }
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.contains(&e))
            {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    for r in SCAN_ROOTS {
        walk(&root.join(r), &mut found);
    }
    found.sort();
    found
}

/// Does a line number follow this citation closely enough to be read as
/// part of it?
///
/// "Closely enough" is a short window rather than the rest of the line:
/// a citation is often followed by prose, and that prose may legitimately
/// contain a capital L before digits. The window covers the shapes the
/// tree actually used — `§x L123`, `§x, L123`, `§x (L123)`, `§x rule 12,
/// L123`, `§x L123-456`.
fn line_number_follows(rest: &str) -> Option<String> {
    const WINDOW: usize = 48;
    let window: String = rest.chars().take(WINDOW).collect();
    let bytes: Vec<char> = window.chars().collect();
    for i in 0..bytes.len() {
        if bytes[i] != 'L' {
            continue;
        }
        let digits: String = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.len() < 3 {
            continue;
        }
        // A preceding alphanumeric means this is part of a word
        // (`HTML5000`), not a line reference.
        if i > 0 && (bytes[i - 1].is_alphanumeric() || bytes[i - 1] == '_') {
            continue;
        }
        return Some(format!("L{digits}"));
    }
    None
}

#[test]
fn spec_citations_name_a_section_not_a_line() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    for path in source_files(&root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // binary or unreadable; nothing to cite
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();

        for (lineno, line) in text.lines().enumerate() {
            for doc in SPEC_DOCS {
                let mut from = 0usize;
                while let Some(at) = line[from..].find(doc) {
                    let start = from + at + doc.len();
                    scanned += 1;
                    if let Some(found) = line_number_follows(&line[start..]) {
                        violations.push(format!(
                            "{rel}:{}: cites {doc} with {found}\n  {}\n  \
                             Line numbers in a spec citation are checked by nothing and go stale \
                             on the next edit — every one in this tree was wrong when the rule \
                             landed. Cite the section anchor alone; it resolves and is verified.",
                            lineno + 1,
                            line.trim(),
                        ));
                    }
                    from = start;
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "spec citations carry line numbers:\n\n{}",
        violations.join("\n\n"),
    );

    assert!(
        scanned >= MIN_CITATIONS_SCANNED,
        "scanned only {scanned} spec citations, floor {MIN_CITATIONS_SCANNED} — the scan \
         stopped matching, so a green result means nothing was checked",
    );
}

/// The synth RFC's exclusion above rests on one claim: its line count is
/// pinned, so a citation written today still resolves tomorrow. Nothing
/// enforced that claim — the convention lived in a comment, and an author
/// who inserted a paragraph would silently move every citation below it.
/// Measured at the time this gate landed: 1316 citations across the tree
/// name a line in that file, and 985 of them point below line 682.
///
/// So the pin is asserted here. Adding content means re-wrapping nearby
/// prose to give the line back — which is what "corrections are rewritten
/// to the same line count" already meant, now said out loud.
///
/// This does not check that a given line still says what its citation
/// claims; content can be rewritten in place under a stable count. It
/// catches the failure that rots every downstream citation at once, which
/// is insertion and deletion.
#[test]
fn synth_rfc_holds_its_pinned_line_count() {
    let path = repo_root().join(SYNTH_RFC);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    // Trailing-newline-insensitive: a file ending in "\n" splits to a final
    // empty element that is not a line.
    let lines = text.lines().count();
    assert_eq!(
        lines, SYNTH_RFC_LINES,
        "{SYNTH_RFC} is {lines} lines, pinned at {SYNTH_RFC_LINES}. Citations across the \
         tree name absolute line numbers in this file, so an insertion or deletion moves \
         every one below it and none of them would fail. To add content, re-wrap prose \
         nearby to return the line; to remove it, spend the freed line the same way. If \
         the pin is being retired on purpose, add this document to SPEC_DOCS above so the \
         citations are forbidden instead, and update both in the same commit.",
    );
}

/// The pin above is only worth having while citations depend on it. If that
/// number collapses, the pin has become ceremony and should be retired
/// deliberately rather than kept out of habit — this floor is what forces
/// that decision to be made rather than drifted into.
#[test]
fn the_synth_rfc_pin_still_has_dependents() {
    // `source_files` walks every SCAN_ROOTS entry itself, so it takes the
    // repo root — joining a root name onto it again would walk
    // `sce-build/src/sce-build/src` and count nothing.
    let mut found = 0usize;
    for file in source_files(&repo_root()) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        found += count_synth_rfc_line_citations(&text);
    }
    assert!(
        found >= MIN_SYNTH_RFC_CITATIONS,
        "only {found} citations name a line in {SYNTH_RFC}, floor {MIN_SYNTH_RFC_CITATIONS}. \
         Either the scan stopped matching, or the tree has moved to section anchors — in \
         which case retire the pin and move this document into SPEC_DOCS.",
    );
}

/// Count `§synth-5-B line 488` shaped citations, including the `L488` and
/// `lines 2232-2540` forms.
/// Anchored on the `§` so prose containing a bare "line 12" is not counted.
fn count_synth_rfc_line_citations(text: &str) -> usize {
    let mut count = 0;
    for (idx, _) in text.match_indices('§') {
        let rest = &text[idx + '§'.len_utf8()..];
        // Section token: digits, letters, dots and dashes (`synth-5-J-2`).
        let sec_len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .unwrap_or(rest.len());
        if sec_len == 0 {
            continue;
        }
        let after = rest[sec_len..].trim_start();
        let after = match after.strip_prefix("lines") {
            Some(r) => r,
            None => match after.strip_prefix("line") {
                Some(r) => r,
                None => match after.strip_prefix('L') {
                    Some(r) => r,
                    None => continue,
                },
            },
        };
        if after.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
            count += 1;
        }
    }
    count
}
