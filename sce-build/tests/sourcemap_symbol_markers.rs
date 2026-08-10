// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Per-symbol SCE-MAP marker attribution.
//
// Spec lines 3117 / 3153 / 3158 fix the comment-form marker as
// `SCE-MAP: <scxml_file>:<line> :: <state> :: <artifact>` for the three
// backends that carry attribution as a comment (Rust `#[doc]` + `//`,
// Kotlin `//`, Python `#`). C / Cpp / Go carry it as a `#line` /
// `//line` directive whose syntax is fixed by the language, so their
// only comment-form marker is the module banner.
//
// The presence gates that already exist cannot see whether a marker
// names the symbol it sits on:
//
//   - `forge::sourcemap::validate_emitted_files_have_markers` tests for
//     the literal `SCE-MAP:` substring.
//   - `sourcemap_module_markers` / `sourcemap_function_markers` count
//     occurrences.
//
// A marker naming the wrong symbol, or naming none at all, passes all
// three. This gate joins each marker back to the sidecar symbol table:
// the `(state, artifact)` a marker claims MUST resolve to a real symbol
// whose recorded line range contains the marker's line and whose source
// file matches. That is the property a JTAG hard-fault or panic
// backtrace actually depends on when only the source listing is
// available.
//
// The artifact vocabulary (`_machine`, `_state_body`, `_transition_<i>`,
// `_forge_body`, `_on_entry_<b>_<a>`, `_on_exit_<b>_<a>`) is owned by
// `forge::symbol_mangling`; this gate decodes it with `demangle` rather
// than restating it [[feedback-declared-coverage-is-not-coverage]].

use sce_build::forge::sourcemap::Sourcemap;
use sce_build::forge::symbol_mangling::demangle;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Every sidecar git tracks, as repo-relative paths.
fn tracked_sidecars() -> Vec<PathBuf> {
    let out = Command::new("git")
        .args(["ls-files", "*sce_sourcemap.json"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8(out.stdout)
        .expect("git output is utf-8")
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Source extensions whose marker form is a comment and therefore
/// carries the `:: <state> :: <artifact>` suffix.
const COMMENT_FORM_EXTENSIONS: &[&str] = &["rs", "kt", "py"];

/// One decoded marker occurrence.
#[derive(Debug)]
struct Marker {
    /// Source file the marker was read from, for failure messages.
    origin: String,
    /// 1-based line of the marker within `origin`.
    origin_line: usize,
    /// The SCXML file the marker attributes to.
    scxml_file: String,
    /// The SCXML line the marker attributes to.
    scxml_line: u32,
    /// State path segment — empty for document-root symbols.
    state: String,
    /// Artifact segment. Always the last ` :: ` field.
    artifact: String,
}

/// Parse every `SCE-MAP:` marker in `text`.
///
/// Returns `Err` listing the markers that carry no ` :: ` attribution
/// suffix, because that is the failure this gate exists to name.
fn parse_markers(origin: &str, text: &str) -> Result<Vec<Marker>, Vec<String>> {
    let mut found = Vec::new();
    let mut unattributed = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        let Some(rest) = line.split_once("SCE-MAP:").map(|(_, r)| r) else {
            continue;
        };
        // The Rust `#[doc = "..."]` form closes with `"]`; the plain
        // comment forms run to end-of-line. Trim both.
        let payload = rest
            .trim()
            .trim_end_matches("\"]")
            .trim_end_matches('"')
            .trim();

        // `<file>:<line>` then optional ` :: state :: artifact`.
        let mut segments = payload.split(" :: ");
        let locator = segments.next().unwrap_or_default().trim();
        let attribution: Vec<&str> = segments.map(str::trim).collect();

        let Some((file, line_part)) = locator.rsplit_once(':') else {
            continue; // not a locator-shaped marker (prose mentions)
        };
        let Ok(scxml_line) = line_part.trim().parse::<u32>() else {
            continue;
        };

        // Artifact is always the last field; the state segment is
        // present only for state-scoped symbols.
        let (state, artifact) = match attribution.as_slice() {
            [] => {
                unattributed.push(format!("{origin}:{}: {}", idx + 1, line.trim()));
                continue;
            }
            [artifact] => (String::new(), (*artifact).to_string()),
            [state, artifact] => ((*state).to_string(), (*artifact).to_string()),
            more => {
                unattributed.push(format!(
                    "{origin}:{}: marker carries {} attribution fields, expected 1 or 2",
                    idx + 1,
                    more.len()
                ));
                continue;
            }
        };

        found.push(Marker {
            origin: origin.to_string(),
            origin_line: idx + 1,
            scxml_file: file.to_string(),
            scxml_line,
            state,
            artifact,
        });
    }

    if unattributed.is_empty() {
        Ok(found)
    } else {
        Err(unattributed)
    }
}

/// Sidecar rows reachable from the `(state, artifact)` pair a marker
/// names: the source file each row traces to and the line range it
/// spans. A list because one directory's sidecar accumulates every
/// machine emitted into it — see [`index_by_attribution`].
type AttributionIndex = BTreeMap<(String, String), Vec<(String, [u32; 2])>>;

/// Index a sidecar by the `(state_path, artifact)` pair a marker names.
///
/// Keys come from `demangle`, so the vocabulary stays owned by
/// `symbol_mangling`. The state path is the flattened form the mangler
/// produces (hierarchy `/` already collapsed to `_`), which is what a
/// marker must name for the join to be reconstructible.
///
/// The value is a LIST, not a single row: one sidecar accumulates every
/// machine emitted into its directory, so a parent and its `<invoke>`
/// child both contribute a `_machine` symbol under the same
/// `(state, artifact)` key. The machine segment is what separates them
/// in the mangled key, and a marker does not carry it — the source file
/// it cites does. Collapsing the list would make the join resolve
/// against whichever machine sorted last.
fn index_by_attribution(map: &Sourcemap) -> AttributionIndex {
    let mut index: AttributionIndex = BTreeMap::new();
    for (mangled, symbol) in &map.symbols {
        let Some((_machine, state_path, artifact)) = demangle(mangled) else {
            continue;
        };
        index
            .entry((state_path, artifact))
            .or_default()
            .push((symbol.scxml_file.clone(), symbol.line_range));
    }
    index
}

/// Join one file's markers against a sidecar index, returning violations.
fn join_violations(markers: &[Marker], index: &AttributionIndex) -> Vec<String> {
    let mut out = Vec::new();
    for m in markers {
        let key = (m.state.clone(), m.artifact.clone());
        let Some(candidates) = index.get(&key) else {
            out.push(format!(
                "{}:{}: names symbol (state={:?}, artifact={:?}) that the sidecar does not contain",
                m.origin, m.origin_line, m.state, m.artifact
            ));
            continue;
        };
        // Sidecars record the SCXML path the emit was handed; a marker
        // records what the parser put on the node. Match on the
        // basename so neither side has to be the canonical spelling.
        let file_matches = |candidate: &String| {
            let basename = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
            basename(candidate) == basename(&m.scxml_file)
        };
        let resolved = candidates.iter().any(|(file, range)| {
            file_matches(file) && m.scxml_line >= range[0] && m.scxml_line <= range[1]
        });
        if !resolved {
            out.push(format!(
                "{}:{}: cites {}:{} but (state={:?}, artifact={:?}) resolves only to {:?}",
                m.origin,
                m.origin_line,
                m.scxml_file,
                m.scxml_line,
                m.state,
                m.artifact,
                candidates
            ));
        }
    }
    out
}

/// Lower bound on sidecars swept. A discovery bug that silences the
/// sweep reads as a pass without it.
const MIN_SIDECARS: usize = 200;

/// Lower bound on markers joined across the committed trees.
const MIN_COMMITTED_MARKERS: usize = 1000;

#[test]
fn committed_markers_name_a_symbol_the_sidecar_contains() {
    let root = repo_root();
    let sidecars = tracked_sidecars();
    assert!(
        sidecars.len() >= MIN_SIDECARS,
        "swept {} sidecars, expected at least {MIN_SIDECARS} — discovery regressed",
        sidecars.len()
    );

    let mut violations: Vec<String> = Vec::new();
    let mut unattributed: Vec<String> = Vec::new();
    let mut joined = 0usize;

    for sidecar in &sidecars {
        let abs = root.join(sidecar);
        let text = std::fs::read_to_string(&abs).expect("sidecar readable");
        let map: Sourcemap = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not a sourcemap: {e}", sidecar.display()));
        let index = index_by_attribution(&map);

        let dir = abs.parent().expect("sidecar has a directory");
        for entry in std::fs::read_dir(dir).expect("sidecar directory readable") {
            let path = entry.expect("dir entry").path();
            let is_comment_form = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| COMMENT_FORM_EXTENSIONS.contains(&e));
            if !is_comment_form {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("source readable");
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            match parse_markers(&rel, &src) {
                Ok(markers) => {
                    joined += markers.len();
                    violations.extend(join_violations(&markers, &index));
                }
                Err(bare) => unattributed.extend(bare),
            }
        }
    }

    assert!(
        unattributed.is_empty(),
        "{} committed markers carry no `:: <state> :: <artifact>` attribution \
         (spec lines 3117/3153/3158). First 10:\n{}",
        unattributed.len(),
        unattributed
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        violations.is_empty(),
        "{} committed markers name a symbol that does not resolve. First 10:\n{}",
        violations.len(),
        violations
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        joined >= MIN_COMMITTED_MARKERS,
        "joined {joined} markers, expected at least {MIN_COMMITTED_MARKERS} — \
         the sweep stopped reaching the marker sites"
    );
}

/// Statechart with two states, entry + exit actions and a transition
/// action, so every per-symbol marker site renders.
///
/// `s1`'s first transition deliberately carries no executable content.
/// Backends render only the transitions that have actions, so the
/// action-bearing one is the SECOND entry of `s1`'s transition list but
/// the FIRST rendered — its artifact must be `_transition_1`. Building
/// the artifact from the render loop's own counter yields
/// `_transition_0`, which resolves to the skipped transition at another
/// line. Without this gap the two are indistinguishable.
const STATECHART_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log expr="'entering s1'"/>
    </onentry>
    <onexit>
      <log expr="'leaving s1'"/>
    </onexit>
    <transition event="skip" target="s2"/>
    <transition event="go" target="s2">
      <log expr="'taking go'"/>
    </transition>
  </state>
  <final id="s2"/>
</scxml>
"#;

/// Python has no committed sidecar tree, so its marker attribution is
/// only reachable by generating. Rust and Kotlin run here too: the
/// committed sweep proves the trees on disk, this proves the emitter
/// that writes them.
#[test]
fn generated_markers_name_a_symbol_the_sidecar_contains() {
    let tmp = std::env::temp_dir().join(format!(
        "sce_symbol_markers_pid{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let input = tmp.join("symbol_probe.scxml");
    std::fs::write(&input, STATECHART_FIXTURE).expect("fixture written");

    let mut checked_languages = 0usize;
    for lang in ["rust", "kotlin", "python"] {
        let out_dir = tmp.join(lang);
        std::fs::create_dir_all(&out_dir).expect("out dir");
        let status = Command::new(sce_codegen_bin())
            .args(["generate", input.to_str().expect("utf-8 path")])
            .arg("-o")
            .arg(&out_dir)
            .args(["-l", lang])
            .output()
            .expect("sce-codegen runs");
        assert!(
            status.status.success(),
            "generate -l {lang} failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );

        let sidecar = out_dir.join("sce_sourcemap.json");
        let map: Sourcemap = serde_json::from_str(
            &std::fs::read_to_string(&sidecar).expect("sidecar emitted next to the artifacts"),
        )
        .expect("sidecar parses");
        let index = index_by_attribution(&map);

        let mut markers_here = 0usize;
        for entry in std::fs::read_dir(&out_dir).expect("out dir readable") {
            let path = entry.expect("dir entry").path();
            let is_comment_form = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| COMMENT_FORM_EXTENSIONS.contains(&e));
            if !is_comment_form {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("source readable");
            let rel = path.display().to_string();
            let markers = parse_markers(&rel, &src).unwrap_or_else(|bare| {
                panic!("{lang}: unattributed markers:\n{}", bare.join("\n"))
            });
            markers_here += markers.len();
            let violations = join_violations(&markers, &index);
            assert!(
                violations.is_empty(),
                "{lang}: {} markers do not resolve:\n{}",
                violations.len(),
                violations.join("\n")
            );
        }
        assert!(
            markers_here >= 3,
            "{lang}: only {markers_here} markers rendered — the probe stopped \
             reaching the per-symbol sites"
        );
        checked_languages += 1;
    }
    assert_eq!(
        checked_languages, 3,
        "every comment-form backend must be probed"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

/// A child whose parent owns a transition with actions. The Kotlin
/// backend renders that parent transition a second time under the
/// child's dispatch arm (`compute_effective_transitions` = self +
/// ancestors), which is the one case where the render site is not the
/// owner.
const INHERITED_TRANSITION_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="outer" datamodel="ecmascript">
  <state id="outer" initial="inner">
    <transition event="abort" target="done">
      <log expr="'ancestor abort'"/>
    </transition>
    <state id="inner">
      <transition event="go" target="done">
        <log expr="'inner go'"/>
      </transition>
    </state>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The marker on an inherited transition must name the state that OWNS
/// it, not the arm it renders under.
///
/// This is the assertion that forbids rebuilding the attribution from
/// the render site. Deriving it there yields `inner :: _transition_0`,
/// which is not a dangling reference — it resolves to a real symbol,
/// the child's own `go` transition at a different line. A gate that
/// only asked "does the symbol exist" would pass that; only the line
/// join catches it, and only this fixture produces it.
#[test]
fn inherited_transition_marker_names_the_owning_state() {
    let tmp = std::env::temp_dir().join(format!(
        "sce_inherited_marker_pid{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let input = tmp.join("inherited_probe.scxml");
    std::fs::write(&input, INHERITED_TRANSITION_FIXTURE).expect("fixture written");
    let out_dir = tmp.join("kotlin");

    let run = Command::new(sce_codegen_bin())
        .args(["generate", input.to_str().expect("utf-8 path")])
        .arg("-o")
        .arg(&out_dir)
        .args(["-l", "kotlin"])
        .output()
        .expect("sce-codegen runs");
    assert!(
        run.status.success(),
        "generate failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let sources: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .expect("out dir readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("kt"))
        .collect();
    assert!(!sources.is_empty(), "kotlin backend emitted no .kt file");

    let mut in_child_arm = false;
    let mut inherited_markers = 0usize;
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("source readable");
        for line in text.lines() {
            // The child's dispatch arm opens here and the next arm
            // closes it; every marker between the two belongs to a
            // transition rendered under the child.
            if line.contains("is InheritedProbeState.Inner ->") {
                in_child_arm = true;
            } else if line.contains("is InheritedProbeState.Outer ->") {
                in_child_arm = false;
            }
            if !in_child_arm || !line.contains("SCE-MAP:") {
                continue;
            }
            if line.contains("'ancestor abort'") {
                continue;
            }
            // Inside the child's arm, the inherited transition is the
            // one attributed to the parent.
            if line.contains(":: outer :: ") {
                inherited_markers += 1;
            }
        }
    }
    assert_eq!(
        inherited_markers, 1,
        "expected exactly one marker inside the child arm attributed to the \
         owning parent state; found {inherited_markers}. A marker rebuilt from \
         the render site would read `:: inner ::` here."
    );

    // The generic join must also hold for this shape.
    let map: Sourcemap = serde_json::from_str(
        &std::fs::read_to_string(out_dir.join("sce_sourcemap.json")).expect("sidecar emitted"),
    )
    .expect("sidecar parses");
    let index = index_by_attribution(&map);
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("source readable");
        let markers =
            parse_markers(&path.display().to_string(), &text).expect("markers are attributed");
        let violations = join_violations(&markers, &index);
        assert!(violations.is_empty(), "{}", violations.join("\n"));
    }

    std::fs::remove_dir_all(&tmp).ok();
}
