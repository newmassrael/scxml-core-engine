// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What binds `ecmascript_acceptance` to the filters it describes.
//
// `sce_build::ecmascript_acceptance::refusals` walks the model and
// reports every expression the ECMAScript frontend refuses;
// `sce_build::filters` lowers those same expressions during rendering
// and emits a refusal as Lua that raises. Two walks over one model can
// drift, and a checker that has drifted from the rewriter it describes
// is worse than none — it reports a clean document that is not, or
// accuses a clean one that is.
//
// The binding is measured rather than argued. Every refusal the filters
// make leaves an observable in the artifact: the raise carries the
// message verbatim. So the two sets can be compared directly over the
// whole corpus, in both directions:
//
//   * **Nothing escapes reporting.** For every document and every
//     backend that lowers through the frontend, each raise in the
//     generated source is one the walker also reported. A site the
//     walker forgets reds this.
//   * **Nothing is invented.** Every refusal the walker reports is one
//     some backend actually emitted. A site the walker checks that no
//     template lowers — a `cond` emitted as native code, a `<data>` body
//     that §scxml-B-2 reads as a string — reds this.
//
// The second direction is why the corpus is swept rather than a handful
// of fixtures: a phantom is only visible on a document that carries the
// construct, and which documents those are is precisely what nobody
// knew before this pass existed.
//
// C++ is absent from the backend list on purpose. It does not lower
// `<assign expr>` through the frontend — it hands the authored ECMAScript
// to its engine and lets it decide — so it emits no raise to compare
// against. That gap is a real one and it is not this test's to assert;
// `cpp_does_not_lower_authored_ecmascript_here` pins which backends are
// in scope so the list cannot quietly shrink.
//
// ⚠ The list is DERIVED, so "absent" is a measurement rather than an
// omission: `lowering_backends()` asks each backend for its default. That
// is what moved Kotlin INTO the population on 2026-08-30 without anyone
// editing an array.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sce_build::ecmascript_acceptance::refusals;
use sce_build::generator::{Language, ScriptEngineTarget};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// The backends whose templates lower authored ECMAScript through
/// `sce_build::ecmascript`, i.e. the ones whose artifacts carry a raise
/// to compare against.
/// ⚠ DERIVED, not listed. This was a hand-written array of four, and it was a
/// second roster beside the one the generator already keeps: a backend whose
/// default moved would have left the array behind, and the array's silence
/// reads exactly like "that backend does not lower".
///
/// Measured 2026-08-30, when `Language::Kotlin`'s default became Lua: the
/// array said four, the generator said five, and the only test that noticed
/// was the one asserting Kotlin was NOT here. Deriving it means moving a
/// backend across the seam is one edit in `generator.rs`, and this list
/// follows.
fn lowering_backends() -> Vec<Language> {
    Language::ALL
        .iter()
        .copied()
        .filter(|lang| lang.default_script_engine_target() == ScriptEngineTarget::Lua)
        .collect()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The marker the filters' raise carries, and the join key for this
/// comparison.
const MARKER: &str = "is not valid ECMAScript: ";

/// Undo the source-language escaping the artifact applied, so a message
/// containing a quote still matches. Rust, Python and C11 all spell the
/// raise inside a double-quoted literal; Go uses a raw literal and is
/// unaffected by this pass.
fn unescape(rendered: &str) -> String {
    rendered.replace("\\\"", "\"").replace("\\\\", "\\")
}

/// Every distinct refusal message embedded in `rendered`.
///
/// A message runs from the marker to the closing quote of the literal
/// that holds it. Templates repeat a guard's raise once per
/// transition-processing branch, so the result is a set: the claim is
/// about which refusals exist, not how many times a template mentions
/// one.
fn refusals_in_artifact(rendered: &str) -> BTreeSet<String> {
    let text = unescape(rendered);
    let mut found = BTreeSet::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find(MARKER) {
        // Back up to the word before the marker (`expr` / `cond` /
        // `script`), which the role decides and the comparison needs.
        let head = &rest[..at];
        let word_start = head.rfind("SCXML ").map(|i| i + "SCXML ".len());
        let after = &rest[at + MARKER.len()..];
        let end = after.find('"').unwrap_or(after.len());
        if let Some(ws) = word_start {
            found.insert(format!("{}{}", &head[ws..], &after[..end]));
        }
        rest = &after[end.min(after.len())..];
    }
    found
}

/// The same key, computed from a walker refusal.
///
/// The filter writes `SCXML {word} is not valid ECMAScript: {source}:
/// {err}`; [`refusals_in_artifact`] strips the fixed prefix and keeps
/// `{word} {source}: {err}`, which is what this rebuilds from the
/// walker's side. `wire_word` is the shared spelling of `{word}`, so the
/// two are one definition rather than two literals that happen to agree.
fn refusal_key(r: &sce_build::ecmascript_acceptance::RefusedExpression) -> String {
    let key = format!("{} {}: {}", r.role.wire_word(), r.source, r.error);
    // [`refusals_in_artifact`] ends a message at the closing quote of
    // the literal holding it, so a message carrying a double quote of
    // its own is read as a shorter message on the artifact side and
    // never joins — which is a silent pass in one direction and an
    // unexplainable mismatch in the other. `ExprError::InvalidLvalue`
    // used to spell its subject with `{:?}` and cost exactly that. The
    // assumption is asserted rather than documented so a message that
    // reintroduces one says so.
    assert!(
        !key.contains('"'),
        "a refusal carrying a double quote cannot be joined across the \
         escaping the artifact applies — quote it the way every other \
         `ExprError` variant does: {key}"
    );
    key
}

/// Every `.scxml` this repository tracks.
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

/// Parse one document, or `None` when it does not parse — a corpus of
/// this size carries deliberate negative fixtures, and the pipeline
/// stage that judges them is not this one.
fn parse(path: &Path) -> Option<SCXMLModel> {
    let mut parser = SCXMLParser::new();
    let mut model = parser.parse_file(path.to_str()?).ok()?;
    sce_build::analyzer::analyze(&mut model, path.to_str()?);
    Some(model)
}

fn render(model: &SCXMLModel, lang: Language, stem: &str) -> Option<String> {
    let dir = sce_build::find_template_dir_for(lang);
    match lang {
        Language::Rust => sce_build::generator::generate(model, &dir, false).ok(),
        Language::Go => sce_build::generator::generate_go(model, &dir).ok(),
        Language::Python => sce_build::generator::generate_python(model, &dir).ok(),
        Language::C11 => sce_build::generator::generate_c11(model, &dir, stem, None)
            .ok()
            .map(|out| {
                out.files
                    .iter()
                    .map(|(_, body)| body.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
        Language::Kotlin => sce_build::generator::generate_kotlin(model, &dir, None).ok(),
        // ⚠ NOT `_ => None`, and the difference is the whole point of the
        // derived population above. Two of the three callers read this as
        // `let Some(x) = render(..) else { continue }`, so a backend this arm
        // cannot render is SKIPPED rather than reported — the population grows
        // and the coverage silently does not. Measured 2026-08-30: Kotlin
        // joined the population when its default moved, `_ => None` dropped it
        // from two tests without a word, and only the third — which unwraps —
        // said anything at all.
        //
        // An unrenderable backend is RED now. A backend that reaches here is
        // one the derivation says lowers, so being unable to render it means
        // this file cannot check the very claim it exists for.
        other => panic!(
            "{other:?} is in the lowering population and this test cannot \
             render it. Add its arm — skipping it would leave the population \
             larger than the coverage, which is the shape a `_ => None` hides."
        ),
    }
}

#[test]
fn every_refusal_a_backend_emits_is_one_the_walker_reports() {
    let mut checked = 0usize;
    let mut with_refusals = 0usize;
    let mut failures = Vec::new();

    for path in corpus() {
        let Some(model) = parse(&path) else { continue };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let reported: BTreeSet<String> = refusals(&model).iter().map(refusal_key).collect();
        if !reported.is_empty() {
            with_refusals += 1;
        }
        for lang in &lowering_backends() {
            let Some(rendered) = render(&model, *lang, stem) else {
                continue;
            };
            checked += 1;
            for emitted in refusals_in_artifact(&rendered) {
                if !reported.contains(&emitted) {
                    failures.push(format!(
                        "{} [{:?}]: artifact raises a refusal the walker did not report:\n    {emitted}",
                        path.display(),
                        lang
                    ));
                }
            }
        }
    }

    // A discovery bug that rendered nothing would otherwise read as a
    // pass. The corpus was 708 documents when this bound was set, and
    // the four backends refuse different subsets of them.
    assert!(
        checked > 1500,
        "swept only {checked} document/backend pair(s); expected well over 1500"
    );
    assert!(
        with_refusals > 0,
        "no document in the corpus carries a refusal — the comparison proved nothing"
    );
    assert!(
        failures.is_empty(),
        "{} unreported refusal(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_refusal_the_walker_reports_is_one_a_backend_emits() {
    let mut phantoms = Vec::new();
    let mut confirmed = 0usize;

    for path in corpus() {
        let Some(model) = parse(&path) else { continue };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let reported: BTreeSet<String> = refusals(&model).iter().map(refusal_key).collect();
        if reported.is_empty() {
            continue;
        }
        let mut emitted = BTreeSet::new();
        for lang in &lowering_backends() {
            if let Some(rendered) = render(&model, *lang, stem) {
                emitted.extend(refusals_in_artifact(&rendered));
            }
        }
        for claim in &reported {
            if emitted.contains(claim) {
                confirmed += 1;
            } else {
                phantoms.push(format!("{}: {claim}", path.display()));
            }
        }
    }

    assert!(
        confirmed > 0,
        "no reported refusal was confirmed in any artifact — the comparison proved nothing"
    );
    assert!(
        phantoms.is_empty(),
        "{} reported refusal(s) no backend emits:\n{}",
        phantoms.len(),
        phantoms.join("\n")
    );
}

/// The corpus carries no document reaching for a standard method this
/// datamodel lacks, so the sweeps above prove nothing about that class.
///
/// That absence is the reason the class went unnoticed: `words.map(...)`
/// was emitted as the Lua field call `words.map(...)` on every backend,
/// and no fixture ever asked. The document is written here rather than
/// added to the corpus because a tracked fixture would have to be
/// lint-clean — `every_authored_document_is_free_of_refused_expressions`
/// sweeps the authored corpus for exactly this — so the one document
/// that must carry a refusal belongs to the test that needs it.
#[test]
fn a_refused_builtin_reaches_the_artifact_of_every_lowering_backend() {
    const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="n" expr="words.map(function(w) { return w; }).length"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("builtin-parity");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join("reaching.scxml");
    std::fs::write(&path, DOCUMENT).expect("write document");

    let model = parse(&path).expect("the document parses");
    let reported: BTreeSet<String> = refusals(&model).iter().map(refusal_key).collect();
    assert_eq!(
        reported.len(),
        1,
        "the walker should report exactly the one reach: {reported:?}"
    );

    for lang in &lowering_backends() {
        let rendered = render(&model, *lang, "reaching")
            .unwrap_or_else(|| panic!("{lang:?} renders the document"));
        assert_eq!(
            refusals_in_artifact(&rendered),
            reported,
            "{lang:?} embeds a refusal the walker does not report, or vice versa"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The backend that still hands its engine the authored ECMAScript.
///
/// C++ emits no raise here because its DEFAULT artifact carries the author's
/// text for a run-time engine to deal with. That is a gap in that backend,
/// not a property of the accepted subset — the frontend's verdict is about
/// the document. This test states the split rather than leaving it to be
/// inferred from an absence above.
///
/// ⚠ It said "C++ AND KOTLIN" until 2026-08-30, and Kotlin left on the day
/// `Language::Kotlin.default_script_engine_target()` moved to Lua. The two
/// were never here for the same reason: C++'s guard site does not carry the
/// pair filter at all, while Kotlin's did — what kept Kotlin on this side was
/// the DEFAULT, which is a host policy, and the day the hosts moved the
/// default moved with them. The population above is derived from exactly that
/// answer, so this test now asserts the ONE backend left rather than a pair
/// whose members were held here by different facts.
///
/// The membership assertion below is what keeps this honest in the other
/// direction: the day C++'s default moves too, this case is red rather than
/// quietly checking a claim the tree has stopped making.
#[test]
fn cpp_does_not_lower_authored_ecmascript_here() {
    let path = repo_root().join("resources/344/test344.scxml");
    let model = parse(&path).expect("test344 parses");
    assert!(
        !refusals(&model).is_empty(),
        "test344 writes cond=\"return\" on purpose"
    );

    let lang = Language::Cpp;
    assert!(
        !lowering_backends().contains(&lang),
        "`{lang:?}` now defaults to Lua, so it belongs to the derived \
         population above and this test asserts the opposite of the tree. \
         Drop it from here rather than pinning a split that has closed."
    );
    let dir = sce_build::find_template_dir_for(lang);
    let rendered = sce_build::generator::generate_cpp(&model, &dir, "test344", None)
        .expect("cpp renders")
        .files
        .iter()
        .map(|(_, body)| body.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        refusals_in_artifact(&rendered).is_empty(),
        "{lang:?} now embeds a refusal — its default has moved, so it \
         belongs to the derived population above"
    );
    assert!(
        rendered.contains("return"),
        "{lang:?} should carry the authored cond verbatim"
    );
}
