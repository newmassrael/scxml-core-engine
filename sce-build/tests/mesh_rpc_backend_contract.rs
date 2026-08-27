// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! SCE Mesh §9.5 `<invoke type="sce:mesh-rpc">` — the refusal is a contract,
//! and this file is what makes it one.
//!
//! `sce:mesh-rpc` has transport emission only where
//! `tools/codegen/templates/mesh/<dir>/` exists. Every backend's parser
//! accepts the invoke, so a backend without that directory would emit a
//! machine whose `<onentry>` ignores a declaration the author wrote —
//! `reject_mesh_rpc_in_unsupported_lang` turns that silence into a build-time
//! error instead.
//!
//! Two ways to get this wrong, and this file is aimed at both.
//!
//! **Deleting the gate** would look like `<sce:action>`'s retirement and would
//! not be one. That refusal went away because native host dispatch had been an
//! accident of unfinished work — six backends grew the lowering and the gate
//! had nothing left to refuse (`native_action_backend_parity.rs` is what pays
//! for the deletion). Mesh is the other case: `ARCHITECTURE.md` Principle 8
//! makes C++-only mesh a *stated scope rule*, so deleting the gate would not
//! retire a refusal, it would accept a declaration SCE cannot service.
//!
//! **Writing the set down** is the failure the gate already had, one layer up.
//! Which backends are served is derived from the template tree at codegen
//! time, but the DOCUMENTS said it by hand — and had drifted: ARCHITECTURE.md
//! Principle 8 asserted "`tools/codegen/templates/mesh/` contains only
//! `cpp/`", and `SCE_MESH.md` §1 named five non-mesh backends while omitting
//! C11 and counting the interpreter path as a codegen backend. A hand-written
//! roster in a contract document is a claim about the tree written where the
//! tree cannot correct it.
//!
//! So the roster lives in exactly one place — §9.5's table — and this file
//! binds it in both directions:
//!
//! 1. The table names every backend, once.
//! 2. Its `emitted` rows are exactly the tracked template directories. A new
//!    `mesh/<dir>/` that no row claims is a red, and so is a row claiming a
//!    directory that is not there.
//! 3. The CLI agrees with the table on all six. Owning a directory is not the
//!    same as generating, and a row that says `refused` must produce a
//!    refusal that NAMES the construct and the served set — an operator who
//!    picked the wrong `--lang` is the reader of that sentence.
//!
//! Rule 3 is what keeps rules 1-2 from being a table that only agrees with
//! itself: the doc, the template tree and the binary have to say one thing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;

/// The contract document, and the anchor that opens its table. Named rather
/// than searched for by shape: a second table in this file must not be able
/// to answer for this one.
const CONTRACT_DOC: &str = "SCE_MESH.md";
/// The slug, not the whole HTML comment: the refusal site cites it in prose
/// too, and one spelling has to reach both readers.
const ANCHOR: &str = "sce:mesh-rpc-backends";

/// Where a backend's mesh templates live, relative to the repo root.
const MESH_TEMPLATE_ROOT: &str = "tools/codegen/templates/mesh";

/// The §9.5 fixture the refusal is asked about — a real mesh-rpc document
/// this repository already ships and compiles, not a fresh one written to
/// agree with this test.
const FIXTURE: &str = "tests/mesh/brake_invoke.scxml";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// Resolved through `CARGO_BIN_EXE_*` so the emitter under test is a build
/// dependency of this test rather than whatever binary happens to be on disk.
fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// One row of the §9.5 table.
#[derive(Debug)]
struct Row {
    /// The `--lang` spelling, which is `Language::canonical_name`.
    lang: String,
    /// The subdirectory of `MESH_TEMPLATE_ROOT` this backend's templates
    /// would live in. Carried by the row because the mapping is not the
    /// identity — `c11` renders from `mesh/c/` — and a test that guessed it
    /// would bless the wrong spelling.
    dir: String,
    /// `true` for `emitted`, `false` for `refused`.
    emitted: bool,
}

/// Cells of one markdown table row, trimmed, without the outer empties.
fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn unticked(cell: &str) -> String {
    cell.trim()
        .trim_matches('`')
        .trim_end_matches('/')
        .to_string()
}

/// The table as written, read from the anchor to the first line that is not
/// part of it.
fn contract_rows() -> Vec<Row> {
    let path = repo_root().join(CONTRACT_DOC);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{CONTRACT_DOC} is readable: {e}"));

    let anchor_at = text.find(ANCHOR).unwrap_or_else(|| {
        panic!(
            "{CONTRACT_DOC} carries no `{ANCHOR}` anchor. §9.5's backend table is the \
             single place the mesh-rpc roster is written down; without the anchor this \
             gate reads nothing and would pass by reading nothing."
        )
    });

    // The header carries a backticked first cell (`--lang`) just as the data
    // rows do, so it cannot be told apart by shape. It is matched by name and
    // skipped once: a reshaped table is then a red here rather than a row
    // parsed as data.
    const HEADER_FIRST_CELL: &str = "`--lang`";

    let mut rows = Vec::new();
    let mut header_seen = false;
    for line in text[anchor_at..].lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Blank lines separate the anchor from its table; once rows have
            // been seen, a blank ends it.
            if rows.is_empty() {
                continue;
            }
            break;
        }
        if !trimmed.starts_with('|') {
            if rows.is_empty() {
                continue;
            }
            break;
        }
        let c = cells(trimmed);
        if c.len() < 3 {
            continue;
        }
        // The `|---|` separator carries no backticked lang.
        if !c[0].starts_with('`') {
            continue;
        }
        if !header_seen {
            assert_eq!(
                c[0], HEADER_FIRST_CELL,
                "§9.5's backend table opens with an unexpected header. This gate reads \
                 the table positionally — first cell `--lang`, second the template \
                 directory, third the verdict — so a reshaped table has to fail here \
                 rather than be read as one row short."
            );
            header_seen = true;
            continue;
        }
        let verdict = c[2].trim();
        let emitted = match verdict {
            "emitted" => true,
            "refused" => false,
            other => panic!(
                "§9.5 row `{}` says '{other}'. The column has two values — `emitted` \
                 or `refused` — because it is answering whether codegen produces \
                 transport for that backend, and a third word is a claim nothing \
                 checks.",
                c[0]
            ),
        };
        rows.push(Row {
            lang: unticked(&c[0]),
            dir: unticked(&c[1]),
            emitted,
        });
    }
    rows
}

/// The mesh template directories git actually tracks.
///
/// Read from `git ls-files` rather than the filesystem: an untracked scratch
/// directory under the template root is not a backend anyone can generate
/// from, and a directory holding no tracked file renders nothing.
fn tracked_mesh_dirs() -> BTreeSet<String> {
    let out = Command::new("git")
        .args(["ls-files", "--", MESH_TEMPLATE_ROOT])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files failed");

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|p| {
            p.strip_prefix(MESH_TEMPLATE_ROOT)?
                .trim_start_matches('/')
                .split('/')
                .next()
                .filter(|d| !d.is_empty())
                .map(str::to_string)
        })
        .collect()
}

/// Lower bound, asserted wherever the table is looped over. A sweep that lost
/// rows reports "every backend agrees" by asking fewer of them, and an empty
/// loop is indistinguishable from a pass.
fn rows_or_panic() -> Vec<Row> {
    let rows = contract_rows();
    assert_eq!(
        rows.len(),
        Language::ALL.len(),
        "§9.5's backend table has {} rows for {} backends. Every backend needs a row \
         even when the answer is `refused` — an absent row is how a backend ends up \
         with no stated answer at all.\nrows: {rows:?}",
        rows.len(),
        Language::ALL.len()
    );
    rows
}

#[test]
fn the_contract_names_every_backend_exactly_once() {
    let rows = rows_or_panic();

    let named: BTreeSet<&str> = rows.iter().map(|r| r.lang.as_str()).collect();
    assert_eq!(
        named.len(),
        rows.len(),
        "§9.5's backend table names a backend twice. Two rows for one `--lang` can \
         disagree, and then the contract has no answer.\nrows: {rows:?}"
    );

    let expected: BTreeSet<&str> = Language::ALL.iter().map(|l| l.canonical_name()).collect();
    assert_eq!(
        named, expected,
        "§9.5's backend table and `Language::ALL` name different backends. A backend \
         this table forgot is one whose mesh-rpc answer is written nowhere; a spelling \
         it invents is one no `--lang` accepts."
    );
}

#[test]
fn the_emitted_rows_are_exactly_the_tracked_template_directories() {
    let rows = rows_or_panic();
    let tracked = tracked_mesh_dirs();

    assert!(
        !tracked.is_empty(),
        "no tracked file under {MESH_TEMPLATE_ROOT}. Mesh has a C++ arm, so an empty \
         read here is this gate having lost its subject rather than the tree having \
         lost its templates."
    );

    let claimed: BTreeSet<String> = rows
        .iter()
        .filter(|r| r.emitted)
        .map(|r| r.dir.clone())
        .collect();

    assert_eq!(
        claimed, tracked,
        "§9.5's `emitted` rows and {MESH_TEMPLATE_ROOT} disagree. The roster is \
         DERIVED — `mesh_templates_exist_for` reads this tree at codegen time — so a \
         directory the table does not claim is a backend the diagnostic already offers \
         and the contract still denies, and a claim with no directory behind it is a \
         promise codegen will refuse.\nclaimed: {claimed:?}\ntracked: {tracked:?}"
    );

    for row in rows.iter().filter(|r| !r.emitted) {
        assert!(
            !tracked.contains(&row.dir),
            "§9.5 says `{}` is refused, but {MESH_TEMPLATE_ROOT}/{}/ carries tracked \
             templates. Adding that directory is precisely what lifts the refusal, so \
             the row is stale rather than the tree being wrong.",
            row.lang,
            row.dir
        );
    }
}

#[test]
fn every_backend_answers_the_fixture_the_way_the_contract_says() {
    let rows = rows_or_panic();
    let served: Vec<&str> = rows
        .iter()
        .filter(|r| r.emitted)
        .map(|r| r.lang.as_str())
        .collect();

    for row in &rows {
        let out_dir = repo_root()
            .join("target")
            .join("mesh_rpc_backend_contract")
            .join(&row.lang);
        let _ = std::fs::remove_dir_all(&out_dir);
        std::fs::create_dir_all(&out_dir).expect("scratch dir");

        let result = Command::new(codegen_bin())
            .args([
                "generate",
                FIXTURE,
                "-l",
                &row.lang,
                "-o",
                out_dir.to_str().expect("utf-8 path"),
                "--no-format",
            ])
            .current_dir(repo_root())
            .output()
            .expect("sce-codegen runs");
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        if row.emitted {
            assert!(
                result.status.success(),
                "§9.5 says `{}` emits mesh-rpc transport, and `sce-codegen generate -l \
                 {}` refused {FIXTURE}. Owning a template directory is not the same as \
                 generating from it, which is the whole reason this row is checked \
                 against the binary and not only against the tree.\nstderr:\n{stderr}",
                row.lang,
                row.lang
            );
            continue;
        }

        assert!(
            !result.status.success(),
            "§9.5 says `{}` refuses mesh-rpc, and `sce-codegen generate -l {}` accepted \
             {FIXTURE}. A backend with no transport emission that generates anyway has \
             produced a machine whose `<onentry>` ignores the invoke — the silent skip \
             this gate exists to prevent.",
            row.lang,
            row.lang
        );
        assert!(
            stderr.contains("sce:mesh-rpc"),
            "`{}` refused {FIXTURE} without naming the construct. An operator who \
             picked the wrong `--lang` reads this sentence to find out what is \
             unsupported.\nstderr:\n{stderr}",
            row.lang
        );
        for lang in &served {
            assert!(
                stderr.contains(lang),
                "`{}`'s refusal does not name `{lang}`, which §9.5 lists as served. The \
                 message exists to point at a backend that works; a refusal that names \
                 none leaves the operator to guess.\nstderr:\n{stderr}",
                row.lang
            );
        }
    }
}

/// Whitespace collapsed to single spaces, so a line-wrapped quote in prose
/// can be compared with the one-line sentence a terminal shows.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// §9.5 quotes the refusal verbatim, and a verbatim quote rots.
///
/// It already did, inside one commit: the remedy moved from a hard-coded
/// `` `--lang cpp` `` to a derived one, the emitted sentence changed from
/// "generate this machine for" to "generate this machine with", and the
/// document went on showing the old wording. Nothing but a reader would have
/// caught that, and a reader is what the quote exists to serve — an operator
/// searching for the error text they were shown.
#[test]
fn the_documented_diagnostic_is_the_one_the_binary_emits() {
    let doc = std::fs::read_to_string(repo_root().join(CONTRACT_DOC))
        .unwrap_or_else(|e| panic!("{CONTRACT_DOC} is readable: {e}"));

    // The example block is identified by its content, not its position: it is
    // the only fenced block quoting this fixture's refusal.
    let needle = "<invoke type=\"sce:mesh-rpc\"> in 'brake_invoke'";
    let at = doc.find(needle).unwrap_or_else(|| {
        panic!(
            "§9.5 no longer quotes the refusal for '{needle}'. The quote is what an \
             operator searches for after seeing the error, so this gate has to fail \
             rather than stop comparing."
        )
    });
    let open = doc[..at].rfind("```").unwrap_or_else(|| {
        panic!("§9.5's refusal quote is not inside a fenced block; this gate reads the fence")
    });
    let body_start = doc[open..].find('\n').expect("a fence ends its line") + open + 1;
    let close = doc[body_start..]
        .find("```")
        .expect("§9.5's refusal quote has a closing fence")
        + body_start;
    let quoted = one_line(&doc[body_start..close]);

    assert!(
        quoted.len() > 80,
        "§9.5's quoted refusal collapsed to {} chars. A short quote would be a \
         substring of almost any diagnostic, which passes without comparing.\nquoted: \
         {quoted}",
        quoted.len()
    );

    let out_dir = repo_root()
        .join("target")
        .join("mesh_rpc_backend_contract")
        .join("documented_diagnostic");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("scratch dir");

    // The quote is Rust's, because §9.5 shows Rust's. Reading it from the
    // document rather than naming it here would let a re-quoted example move
    // the target it is supposed to be measured against.
    let result = Command::new(codegen_bin())
        .args([
            "generate",
            FIXTURE,
            "-l",
            "rust",
            "-o",
            out_dir.to_str().expect("utf-8 path"),
            "--no-format",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    let stderr = one_line(&String::from_utf8_lossy(&result.stderr));

    assert!(
        stderr.contains(&quoted),
        "§9.5 quotes a refusal the binary no longer emits. The document is showing an \
         operator a sentence they will never see, which is worse than showing none — \
         they will search for it.\n§9.5:  {quoted}\nactual: {stderr}"
    );
}

/// The gate lives here, and the roster it enforces lives in §9.5.
const GATE_SOURCE: &str = "sce-build/src/generator.rs";

/// One `fn <name>` and its body, brace-matched from the signature.
///
/// Brace-matched rather than read to the next `fn`, so a helper landing
/// between these two does not silently extend the region being judged.
/// Format placeholders inside the string literals are balanced pairs, so
/// they do not move the count.
fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).unwrap_or_else(|| {
        panic!(
            "{GATE_SOURCE} no longer declares `{signature}`. The mesh-rpc gate is what \
             this file exists to hold to §9.5's table; if it was renamed, this gate has \
             to be re-aimed rather than left reading nothing."
        )
    });
    let rest = &source[start..];
    let open = rest.find('{').expect("a function has a body");
    let mut depth = 0usize;
    for (i, ch) in rest[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[..open + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces reading `{signature}` out of {GATE_SOURCE}");
}

/// Line comments removed, string literals left alone.
///
/// The rule below is about what the CODE spells, not what a comment may
/// explain: the paragraph recording that this gate once hard-coded `--lang
/// cpp` has to be allowed to say so. A scanner that reads comments as code
/// makes the record of a defect indistinguishable from the defect.
fn strip_line_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let mut in_string = false;
        let mut escaped = false;
        let mut cut = line.len();
        let chars: Vec<char> = line.chars().collect();
        for i in 0..chars.len() {
            let ch = chars[i];
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' if in_string => escaped = true,
                '"' => in_string = !in_string,
                '/' if !in_string && chars.get(i + 1) == Some(&'/') => {
                    cut = chars[..i].iter().map(|c| c.len_utf8()).sum();
                    break;
                }
                _ => {}
            }
        }
        out.push_str(&line[..cut]);
        out.push('\n');
    }
    out
}

/// `needle` as a standalone word — so `go` does not match `algorithm` and
/// `c11` does not match a longer identifier that contains it.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let boundary = |c: Option<char>| !matches!(c, Some(ch) if ch.is_alphanumeric() || ch == '_');
    let bytes: Vec<char> = haystack.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    if pat.is_empty() || bytes.len() < pat.len() {
        return false;
    }
    (0..=bytes.len() - pat.len()).any(|i| {
        bytes[i..i + pat.len()] == pat[..]
            && boundary(i.checked_sub(1).map(|p| bytes[p]))
            && boundary(bytes.get(i + pat.len()).copied())
    })
}

/// The gate's explanatory comment, from its header to the first function.
fn gate_comment(source: &str) -> &str {
    let gate_at = source
        .find("// ── Mesh-rpc backend gate")
        .unwrap_or_else(|| {
            panic!(
                "{GATE_SOURCE} carries no `// ── Mesh-rpc backend gate` header. That \
                 comment is where the refusal explains itself and names §9.5; without \
                 it a reader at the code has no route to the contract."
            )
        });
    let derive_at = source
        .find("fn mesh_templates_exist_for")
        .expect("the gate's derivation helper");
    &source[gate_at..derive_at]
}

fn gate_source() -> String {
    std::fs::read_to_string(repo_root().join(GATE_SOURCE))
        .unwrap_or_else(|e| panic!("{GATE_SOURCE} is readable: {e}"))
}

/// The refusal site must say WHERE its contract is written.
///
/// A gate that derives its own answer is only half a contract. The other half
/// is the roster an author reads before picking `--lang`, and after that
/// roster moved out of `ARCHITECTURE.md` Principle 8 and into §9.5's table,
/// nothing at the code pointed to it — a reader at the refusal would have
/// re-derived the set by reading this function, which is precisely what
/// moving it was meant to stop.
#[test]
fn the_refusal_site_points_at_its_contract() {
    let source = gate_source();
    let comment = gate_comment(&source);

    // Floor. A region that stopped being found would read as a comment
    // satisfying nothing, and an empty `contains` sweep passes.
    assert!(
        comment.len() > 400,
        "the mesh-rpc gate's comment shrank to {} bytes. It carries the reason the \
         refusal is not retired the way `<sce:action>`'s was, and a stub cannot.",
        comment.len()
    );

    for needle in ["SCE_MESH.md §9.5", ANCHOR, "mesh_rpc_backend_contract"] {
        assert!(
            comment.contains(needle),
            "the mesh-rpc gate's comment does not name `{needle}`. The code has to say \
             where its roster lives, or the next reader takes this function for the \
             roster — which is what moving the set out of it was for."
        );
    }
}

/// The refusal site must not spell a backend of its own.
///
/// This is the half that had already been broken. The gate derives WHICH
/// backends it refuses from the template tree, and a comment three lines
/// above the diagnostic promised the message "is pointed at the backends that
/// actually carry a mesh arm today rather than at a spelling this message
/// happened to be written with" — while the next sentence read ``Either
/// generate this machine for `--lang cpp` ``. A second backend gaining a mesh
/// arm would have widened the "exists for" list and still sent every operator
/// to C++.
///
/// Comments are stripped before the scan: the paragraph recording that defect
/// has to be free to name it, or the record becomes indistinguishable from
/// the defect.
#[test]
fn the_refusal_site_spells_no_backend_of_its_own() {
    let source = gate_source();
    let bodies = strip_line_comments(&format!(
        "{}\n{}",
        function_body(&source, "fn mesh_templates_exist_for"),
        function_body(&source, "fn reject_mesh_rpc_in_unsupported_lang"),
    ));
    // Floor. This is an absence check, and an absence found in nothing is
    // the shape that reports "clean" after the scan stopped reading.
    assert!(
        bodies.contains("EMBEDDED_TEMPLATES") && bodies.contains("Language::ALL"),
        "the two gate functions were read but do not contain the derivation they exist \
         for. Either the region extraction lost them or the gate stopped deriving — \
         both make the scan below vacuous.\nread:\n{bodies}"
    );
    for lang in Language::ALL {
        let name = lang.canonical_name();
        assert!(
            !contains_word(&bodies, name),
            "the mesh-rpc gate spells `{name}` in its own body. Which backends are \
             served is DERIVED from the template tree — both the diagnosis and the \
             remedy — so a literal here is a second roster that the tree cannot \
             correct, and it goes stale the moment a backend gains a mesh arm. This \
             exact literal was already shipped once, in the sentence telling the \
             operator which `--lang` to retry with."
        );
    }
}
