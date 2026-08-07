// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A depfile must name every input the render actually read.
//
// `codegen_depfile_coverage.rs` asserts the *wiring*: that each CMake
// codegen step passes `--write-deps` and declares a `DEPFILE`. Wiring is
// not the property that keeps a build correct — the depfile also has to
// list the files whose edits change the output. A depfile that is wired
// but under-populated fails exactly the way an absent one does: the
// build reuses a stale artefact, reports success, and the template fix
// appears not to work. The wiring gate certifies it as correct.
//
// Three under-declarations were measured against the binary before this
// gate existed, each one a silent stale-artefact hazard:
//
//   1. forge documents (`sce:kind="codec"` and friends) got a depfile
//      naming the input `.scxml` and nothing else — zero templates, on
//      all six backends. Editing `forge/cpp/codec.h.jinja2` did not
//      regenerate the codec header.
//   2. the same depfile omitted `<sce:import>` targets, though editing
//      an imported document demonstrably changes the importing
//      document's output (its `source-hash` covers the import).
//   3. rust / kotlin / go / python statechart depfiles omitted the
//      shared `_macros/` family, which `state_machine.<lang>.jinja2`
//      includes on line 5. Deleting the undeclared set and re-rendering
//      failed outright: `template not found: _macros/sce_map_marker.jinja2`.
//   4. once (2) was fixed by listing `parsed.imports`, the depfile named
//      the *direct* imports and stopped: in an `algorithm → codec →
//      codec` chain, widening the leaf changed `route_msg.h` and nothing
//      declared the leaf. Found because this gate's fixture grew a second
//      level — a one-level chain cannot tell "direct" from "reachable".
//   5. the synth-6.2.6 source set went undeclared entirely. The
//      `source-hash` every emitted file carries folds *every*
//      `**/*.scxml` under the input root, so editing a document the
//      compile never reads still changes the bytes written — measured on
//      all six backends and both pipelines. Ninja reused the artefact
//      and its embedded hash stopped describing the tree it claims to
//      come from, which is the state `sce-codegen verify` refuses.
//   6. the conformance harness had no depfile at all: of the 156 codegen
//      steps under `cmake/`, `tests/` and `backends/`, its two were the
//      only ones declaring inputs by hand — a `file(GLOB)` over the
//      per-kind fragments (missing whatever they include) and no fixture
//      document at all, though the harness folds all 165 into its hash.
//
// The first three have one cause: the depfile computed its template set
// with a second walk of its own instead of asking the loader what it
// loads. The fourth is the same shape one layer over, in the import
// graph: the CLI re-derived the import paths instead of taking what the
// compile actually read. The fifth is the shape's limit case — nobody
// re-derived the source set because nobody declared it, though the code
// that computed the hash had it in hand. The sixth is what the axis
// looks like when a route has no depfile mechanism to get wrong.
//
// The probe here is behavioural rather than structural. For each backend
// and pipeline it renders once, prunes every template the depfile did
// *not* declare, and renders again: if the depfile is complete, the
// pruned tree still produces byte-identical output, because nothing that
// was removed could have contributed. Anything the render needs and the
// depfile omits shows up as a render failure or a byte difference. That
// direction is the one that matters — over-declaring costs a spurious
// rebuild, under-declaring ships a stale artefact.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// Every backend, so a defect present on one is not hidden by five
/// passing. Defect 3 above was invisible on `cpp` / `c11`, whose loader
/// scope is the whole tree, and live on the other four.
const LANGUAGES: &[&str] = &["cpp", "rust", "kotlin", "go", "python", "c11"];

/// Lower bound on templates pruned by a single probe, read off a run
/// rather than guessed (the smallest measured was 176, on `c11`
/// statechart). A probe that prunes nothing renders an untouched tree
/// and passes without proving anything — the same "scanned zero, reported
/// green" failure the coverage gate carries a floor against.
const MIN_PRUNED_PER_PROBE: usize = 100;

/// One probe per (backend, pipeline). Pinned exactly: a scenario that
/// stops being generated must fail here rather than quietly shrink the
/// matrix.
const EXPECTED_PROBES: usize = LANGUAGES.len() * 2;

fn codegen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sce-codegen")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn template_root() -> PathBuf {
    repo_root().join("tools").join("codegen").join("templates")
}

/// A rich statechart: every action template the backend carries, plus an
/// `<invoke>` so the synthesised child document is emitted too. A
/// degenerate two-state document would leave most of the tree unrendered
/// and let an omission pass unobserved.
const STATECHART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="depfile_probe" initial="work" datamodel="ecmascript">
  <datamodel>
    <data id="counter" expr="0"/>
    <data id="items" expr="[1,2,3]"/>
  </datamodel>
  <state id="work" initial="inner">
    <onentry>
      <log label="enter" expr="'work'"/>
      <assign location="counter" expr="counter + 1"/>
      <foreach array="items" item="it">
        <log expr="it"/>
      </foreach>
      <if cond="counter &gt; 0">
        <raise event="positive"/>
      <elseif cond="counter == 0"/>
        <raise event="zero"/>
      <else/>
        <raise event="negative"/>
      </if>
      <send event="ping" id="pingId" delay="10ms">
        <param name="n" expr="counter"/>
      </send>
      <cancel sendid="pingId"/>
      <script>counter = counter;</script>
    </onentry>
    <onexit>
      <log label="exit" expr="'work'"/>
    </onexit>
    <invoke type="scxml" id="child">
      <content>
        <scxml version="1.0" initial="c0">
          <final id="c0"/>
        </scxml>
      </content>
    </invoke>
    <state id="inner">
      <transition event="positive" target="done"/>
      <transition event="zero" target="done"/>
      <transition event="negative" target="done"/>
    </state>
    <history id="hist" type="deep">
      <transition target="inner"/>
    </history>
  </state>
  <final id="done">
    <donedata>
      <param name="total" expr="counter"/>
    </donedata>
  </final>
</scxml>
"#;

/// A forge codec carrying one field, optionally importing another codec.
///
/// Built rather than spelled out per variant: the "mutated" form of a
/// document differs from the original in one number, and holding the two
/// as separate literals is how they drift into no longer being the same
/// document with one change.
fn forge_codec(name: &str, field: &str, bits: u32, imports: Option<&str>) -> String {
    let import_line = imports
        .map(|src| format!("  <sce:import src=\"{src}\" kind=\"codec\" as=\"inner\"/>\n"))
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
{import_line}  <datamodel>
    <sce:field id="{field}" sce:type="uint{bits}" sce:byte="0" sce:bit-size="{bits}"/>
  </datamodel>
</scxml>
"#
    )
}

/// The leaf of the import chain — imported by `frame_codec`, and so
/// reachable from the compiled document only transitively.
fn forge_inner_codec(bits: u32) -> String {
    forge_codec("inner_codec", "inner_id", bits, None)
}

/// The middle of the chain: imported directly by the compiled algorithm,
/// and itself importing the leaf.
fn forge_frame_codec(bits: u32) -> String {
    forge_codec("frame_codec", "msg_id", bits, Some("inner_codec.scxml"))
}

/// The compiled document. Its `sce:kind` routes away from the statechart
/// arm, and the two-level import chain below it is what gives the
/// `<sce:import>` half of the contract something to be measured against.
const FORGE_ALGORITHM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="route_msg" version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="frame" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="frame.msg_id === 1"/>
  </sce:body>
</scxml>
"#;

/// The oracle catalog's float tolerance, and a value to move it to.
///
/// The C11 harness `#define`s this once, so changing it is an edit whose
/// effect reaches the emitted source — which is what the probe needs and
/// what a whitespace edit is not.
const ORACLE_TOLERANCE: &str = "\"float_tolerance\": 1e-12";
const ORACLE_TOLERANCE_MUTATED: &str = "\"float_tolerance\": 1e-9";

/// A decode-rejection vector's `why`, and a value to move it to.
///
/// The tolerance above reaches only the C11 harness, so on its own it
/// probes one route and reports the other five as not reading a file they
/// do read. Reject vectors are inlined by all six codec fragments — the
/// `why` string lands in each backend's assertion message — so moving one
/// is the edit that reaches every reading route. Both mutations are
/// applied together and each is asserted to have matched, because a probe
/// that silently degrades to one surface would fail the five routes with a
/// message blaming the producer.
const ORACLE_REJECT_WHY: &str = "chain saturates max-depth=8";
const ORACLE_REJECT_WHY_MUTATED: &str = "chain saturates the declared depth";

/// The field width every chain document starts at, and the width a
/// mutation widens it to. Any pair of distinct widths works; naming them
/// keeps the "before" and "after" of each probe from drifting apart.
const CHAIN_BITS: u32 = 8;
const CHAIN_BITS_WIDENED: u32 = 16;

/// A document in the import chain, by how far it sits from the compiled
/// one. Both distances are probed: the depfile named the direct import
/// and stopped there, so a chain of length one could not tell the two
/// apart.
struct ChainDoc {
    filename: &'static str,
    distance: &'static str,
    widened: fn() -> String,
}

const CHAIN_DOCS: &[ChainDoc] = &[
    ChainDoc {
        filename: "frame_codec.scxml",
        distance: "directly imported",
        widened: || forge_frame_codec(CHAIN_BITS_WIDENED),
    },
    ChainDoc {
        filename: "inner_codec.scxml",
        distance: "transitively imported",
        widened: || forge_inner_codec(CHAIN_BITS_WIDENED),
    },
];

/// Which pipeline a scenario routes through — the two arms compute their
/// template scope differently, so both need a probe.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pipeline {
    Statechart,
    Forge,
}

impl Pipeline {
    fn label(self) -> &'static str {
        match self {
            Pipeline::Statechart => "statechart",
            Pipeline::Forge => "forge",
        }
    }
}

/// Run `sce-codegen generate` against `templates`, returning stdout on
/// success and the full diagnostic on failure.
///
/// `SOURCE_DATE_EPOCH` is pinned so the `generated-at` stamp does not
/// make every comparison differ; `--go-module-prefix` is required by the
/// Go crossfile route and ignored elsewhere.
fn generate(
    doc: &Path,
    out: &Path,
    lang: &str,
    templates: &Path,
    depfile: Option<&Path>,
) -> Result<(), String> {
    std::fs::create_dir_all(out).expect("output directory is creatable");
    let mut cmd = Command::new(codegen_bin());
    cmd.arg("generate")
        .arg(doc)
        .arg("-o")
        .arg(out)
        .arg("-l")
        .arg(lang)
        .arg("--go-module-prefix")
        .arg("example.com/depfile_probe")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("SCE_TEMPLATE_DIR", templates);
    if let Some(d) = depfile {
        cmd.arg("--write-deps").arg(d);
    }
    let output = cmd.output().expect("sce-codegen is runnable");
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "exit {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

/// Render the numerical conformance harness for `lang`, as the two
/// CMake conformance steps do.
///
/// A third entry point into `write_depfile`, and one that had no depfile
/// at all until `generate-conformance` learned `--write-deps`: of the
/// 156 codegen invocations under `cmake/`, `tests/` and `backends/`,
/// these two were the only ones declaring their inputs by hand.
fn generate_conformance(
    manifest: &Path,
    out: &Path,
    lang: &str,
    templates: &Path,
    depfile: Option<&Path>,
) -> Result<(), String> {
    std::fs::create_dir_all(out).expect("output directory is creatable");
    let mut cmd = Command::new(codegen_bin());
    cmd.arg("generate-conformance")
        .arg("--language")
        .arg(lang)
        .arg("--manifest")
        .arg(manifest)
        .arg("--output-dir")
        .arg(out)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("SCE_TEMPLATE_DIR", templates);
    if let Some(d) = depfile {
        cmd.arg("--write-deps").arg(d);
    }
    let output = cmd.output().expect("sce-codegen is runnable");
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "exit {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

/// Copy the tracked conformance catalog into `dst`, preserving the
/// `<root>/conformance/fixtures.json` + `<root>/resources/*.scxml`
/// layout the manifest path resolution depends on, and return the copied
/// manifest.
///
/// Copied rather than used in place because the probe below edits a
/// fixture, and a gate that mutates the tracked tree is a gate that
/// fails differently depending on whether the last run cleaned up.
fn stage_conformance_catalog(dst: &Path) -> PathBuf {
    let src_root = repo_root().join("tests").join("forge");
    let manifest = dst.join("conformance").join("fixtures.json");
    let conformance = manifest.parent().expect("manifest has a parent").to_owned();
    std::fs::create_dir_all(&conformance).expect("staging directory is creatable");
    // The whole directory, not just the manifest: the C11 route also
    // reads `numerical_reference.json` from here to bake the oracle into
    // the harness source.
    for entry in std::fs::read_dir(src_root.join("conformance")).expect("catalog is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_file() {
            std::fs::copy(
                &path,
                conformance.join(path.file_name().expect("file has a name")),
            )
            .expect("catalog file is copyable");
        }
    }

    let resources = dst.join("resources");
    std::fs::create_dir_all(&resources).expect("resource directory is creatable");
    let mut copied = 0usize;
    for entry in std::fs::read_dir(src_root.join("resources")).expect("resources are readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_file() {
            std::fs::copy(
                &path,
                resources.join(path.file_name().expect("file has a name")),
            )
            .expect("resource is copyable");
            copied += 1;
        }
    }
    assert!(
        copied > 0,
        "staged no conformance resources from {} — the probe would render against an \
         empty source set and prove nothing",
        src_root.display(),
    );
    manifest
}

/// Prerequisites listed in a Make-style depfile.
///
/// The format is `targets: dep \<newline> dep ...`; everything after the
/// first `:` is whitespace- and backslash-separated. Both the template
/// paths and the source prerequisites are returned, since the two halves
/// of the contract are asserted separately.
fn depfile_prerequisites(depfile: &Path) -> BTreeSet<PathBuf> {
    let text = std::fs::read_to_string(depfile)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", depfile.display()));
    // Colon-space, not a bare colon: the targets are absolute paths, and
    // splitting on `':'` would cut the first one short on any platform
    // whose paths carry one.
    let (_, rhs) = text
        .split_once(": ")
        .unwrap_or_else(|| panic!("{} has a `target: prereqs` shape", depfile.display()));
    rhs.split(|c: char| c.is_whitespace() || c == '\\')
        .filter(|t| !t.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Copy `src` into `dst`, keeping only the `.jinja2` files whose path
/// appears in `keep`. Returns how many were pruned.
fn copy_pruned(src: &Path, dst: &Path, keep: &BTreeSet<PathBuf>) -> usize {
    fn walk(src_root: &Path, dir: &Path, dst_root: &Path, keep: &BTreeSet<PathBuf>) -> usize {
        let mut pruned = 0;
        for entry in std::fs::read_dir(dir).expect("template directory is readable") {
            let entry = entry.expect("directory entry is readable");
            let path = entry.path();
            if path.is_dir() {
                pruned += walk(src_root, &path, dst_root, keep);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("jinja2") {
                continue;
            }
            if !keep.contains(&path) {
                pruned += 1;
                continue;
            }
            let rel = path.strip_prefix(src_root).expect("path is under the root");
            let target = dst_root.join(rel);
            std::fs::create_dir_all(target.parent().expect("target has a parent"))
                .expect("output directory is creatable");
            std::fs::copy(&path, &target).expect("template is copyable");
        }
        pruned
    }
    walk(src, src, dst, keep)
}

/// Every emitted file in `dir` paired with its bytes, excluding the
/// depfile — the only artefact that legitimately differs between two
/// runs, because it names the output directory it was written into.
fn emitted_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("output directory is readable") {
        let entry = entry.expect("directory entry is readable");
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) == Some("d") {
            continue;
        }
        let name = path
            .file_name()
            .expect("file has a name")
            .to_string_lossy()
            .to_string();
        out.push((name, std::fs::read(&path).expect("artefact is readable")));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// A document that shares the input root with the compiled one but is
/// unreachable from it — no import, no include, no reference of any kind.
///
/// It still feeds the artefact's `source-hash`: the synth-6.2.6 source
/// set is every `**/*.scxml` under the input root, folded whole. So
/// editing this document changes the compiled document's output, which
/// makes it a prerequisite whether or not the compile ever read it.
const SOURCE_SET_SIBLING: &str = "unrelated_sibling.scxml";

/// The sibling at two widths, so the mutation has something observable
/// to change. Held as one builder for the same reason `forge_codec` is:
/// two literals drift into no longer being the same document with one
/// change.
fn source_set_sibling(bits: u32) -> String {
    forge_codec("unrelated_sibling", "sibling_field", bits, None)
}

/// Materialise a scenario's source documents into `dir`, returning the
/// document to compile.
fn write_sources(dir: &Path, pipeline: Pipeline) -> PathBuf {
    match pipeline {
        Pipeline::Statechart => {
            let doc = dir.join("depfile_probe.scxml");
            std::fs::write(&doc, STATECHART).expect("fixture is writable");
            doc
        }
        Pipeline::Forge => {
            std::fs::write(dir.join("inner_codec.scxml"), forge_inner_codec(CHAIN_BITS))
                .expect("fixture is writable");
            std::fs::write(dir.join("frame_codec.scxml"), forge_frame_codec(CHAIN_BITS))
                .expect("fixture is writable");
            let doc = dir.join("route_msg.scxml");
            std::fs::write(&doc, FORGE_ALGORITHM).expect("fixture is writable");
            doc
        }
    }
}

/// Render, prune everything undeclared, render again, compare.
///
/// Returns the depfile prerequisites so the caller can make the
/// scenario-specific assertions (the `<sce:import>` half) without
/// generating a third time.
fn probe(lang: &str, pipeline: Pipeline, violations: &mut Vec<String>) -> BTreeSet<PathBuf> {
    let case = format!("{lang}/{}", pipeline.label());
    let work = TempDir::new().expect("tempdir");
    let src = work.path().join("src");
    std::fs::create_dir_all(&src).expect("source directory is creatable");
    let doc = write_sources(&src, pipeline);

    let base_out = work.path().join("base");
    let depfile = work.path().join("base.d");
    let root = template_root();
    if let Err(e) = generate(&doc, &base_out, lang, &root, Some(&depfile)) {
        violations.push(format!("{case}: baseline generation failed\n{e}"));
        return BTreeSet::new();
    }

    let prerequisites = depfile_prerequisites(&depfile);
    let declared: BTreeSet<PathBuf> = prerequisites
        .iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jinja2"))
        .cloned()
        .collect();

    let baseline = emitted_files(&base_out);
    if baseline.is_empty() {
        violations.push(format!(
            "{case}: baseline emitted nothing, so the probe below would compare two empty sets"
        ));
        return prerequisites;
    }

    if declared.is_empty() {
        violations.push(format!(
            "{case}: the depfile names no template at all, so every template edit \
             leaves these {} artefact(s) stale and the build reports success. \
             A depfile that is wired but empty fails exactly as an absent one does.",
            baseline.len()
        ));
        return prerequisites;
    }

    let pruned_root = work.path().join("templates");
    let pruned = copy_pruned(&root, &pruned_root, &declared);
    if pruned < MIN_PRUNED_PER_PROBE {
        violations.push(format!(
            "{case}: pruned only {pruned} templates, floor {MIN_PRUNED_PER_PROBE} — \
             a probe that removes nothing re-renders an untouched tree and passes \
             without having tested anything",
        ));
        return prerequisites;
    }

    let probe_out = work.path().join("probe");
    if let Err(e) = generate(&doc, &probe_out, lang, &pruned_root, None) {
        violations.push(format!(
            "{case}: rendering against only the {} declared template(s) failed, so the \
             depfile omits a template the render needs. Editing that template will \
             leave the output stale.\n{e}",
            declared.len()
        ));
        return prerequisites;
    }

    let after = emitted_files(&probe_out);
    if after != baseline {
        let names_before: Vec<&str> = baseline.iter().map(|(n, _)| n.as_str()).collect();
        let names_after: Vec<&str> = after.iter().map(|(n, _)| n.as_str()).collect();
        let differing: Vec<&str> = baseline
            .iter()
            .filter(|(n, b)| after.iter().any(|(m, a)| m == n && a != b))
            .map(|(n, _)| n.as_str())
            .collect();
        violations.push(format!(
            "{case}: pruning the undeclared templates changed the output, so one of \
             them contributes to it while going undeclared.\n  \
             before: {names_before:?}\n  after:  {names_after:?}\n  \
             differing contents: {differing:?}",
        ));
    }

    prerequisites
}

#[test]
fn every_declared_depfile_names_the_templates_its_render_reads() {
    let mut violations = Vec::new();
    let mut probes = 0usize;

    for lang in LANGUAGES {
        for pipeline in [Pipeline::Statechart, Pipeline::Forge] {
            probe(lang, pipeline, &mut violations);
            probes += 1;
        }
    }

    assert_eq!(
        probes, EXPECTED_PROBES,
        "ran {probes} probes, expected {EXPECTED_PROBES} — the matrix shrank, so a green \
         result covers less than it claims",
    );
    assert!(
        violations.is_empty(),
        "depfiles do not name every input their render reads:\n\n{}",
        violations.join("\n\n"),
    );
}

/// Every forge document the compile reads is a prerequisite of its
/// output — at any depth in the import chain, not just the first.
///
/// Asserted in two steps per document, because the first is what makes
/// the second worth asserting: widen a field of that document, re-render,
/// and require the output to change. Only then is omitting it from the
/// depfile a stale-artefact hazard rather than a cosmetic gap.
///
/// The chain is two levels deep for a reason. The first version of this
/// gate used a one-level chain, and the fix it drove satisfied it by
/// listing `parsed.imports` — the *direct* imports. That passed while a
/// grandchild edit still shipped a stale artefact: widening
/// `inner_codec.scxml` changes `route_msg.h`'s `source-hash`, and
/// nothing declared it. A fixture that cannot distinguish "direct" from
/// "reachable" cannot tell whether the fix closed the axis or only the
/// case the fixture happened to show.
#[test]
fn forge_imports_are_declared_prerequisites() {
    let mut violations = Vec::new();
    let mut probes = 0usize;

    for lang in LANGUAGES {
        for target in CHAIN_DOCS {
            probes += 1;
            let case = format!("{lang}/{}", target.filename);
            let work = TempDir::new().expect("tempdir");
            let src = work.path().join("src");
            std::fs::create_dir_all(&src).expect("source directory is creatable");
            let doc = write_sources(&src, Pipeline::Forge);
            let root = template_root();

            let before_out = work.path().join("before");
            let depfile = work.path().join("before.d");
            if let Err(e) = generate(&doc, &before_out, lang, &root, Some(&depfile)) {
                violations.push(format!("{case}: baseline generation failed\n{e}"));
                continue;
            }
            let before = emitted_files(&before_out);

            std::fs::write(src.join(target.filename), (target.widened)())
                .expect("fixture is writable");
            let after_out = work.path().join("after");
            if let Err(e) = generate(&doc, &after_out, lang, &root, None) {
                violations.push(format!(
                    "{case}: generation after the import edit failed\n{e}"
                ));
                continue;
            }
            let after = emitted_files(&after_out);

            if before == after {
                violations.push(format!(
                    "{case}: widening a field of this {} codec left the compiled \
                     document's output byte-identical, so the probe cannot tell whether \
                     the import is declared. Give the mutation something observable to \
                     change before asserting on the depfile.",
                    target.distance,
                ));
                continue;
            }

            let prerequisites = depfile_prerequisites(&depfile);
            let declared = prerequisites
                .iter()
                .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(target.filename));
            if !declared {
                violations.push(format!(
                    "{case}: editing this {} document changes the output (just \
                     demonstrated) but it is not a prerequisite in the depfile, so the \
                     build will reuse the stale artefact. Prerequisites were: {:?}",
                    target.distance,
                    prerequisites
                        .iter()
                        .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
                        .collect::<Vec<_>>(),
                ));
            }
        }
    }

    let expected = LANGUAGES.len() * CHAIN_DOCS.len();
    assert_eq!(
        probes, expected,
        "ran {probes} probes, expected {expected} — the chain shrank, and a chain of one \
         cannot distinguish a depfile that names every reachable import from one that \
         names only the direct ones",
    );
    assert!(
        violations.is_empty(),
        "forge imports are not declared as prerequisites:\n\n{}",
        violations.join("\n\n"),
    );
}

/// A depfile never names a file the same invocation wrote.
///
/// The counterpart to the source-set gate below, and the reason that one
/// cannot simply declare everything it finds. The synth-6.2.6 source
/// set is *every* `**/*.scxml` under the input root, and a codegen run
/// writes `.scxml` files of its own into `-o`: the §9.6.6 synth children
/// for inline `<content>` invokes, and the hybrid-invoke stubs. Whenever
/// `-o` is the directory the input was staged into — which the mesh
/// §9.6.6 step does deliberately, so stage 3 can read the synth document
/// stage 2 emitted — those writes land inside the invocation's own
/// source set.
///
/// Declaring them made the build edge depend on its own output. Ninja
/// does not tolerate that: `dependency cycle:
/// parent_synth_inline__sce_synth_invoke__remote_inv.scxml -> itself`,
/// with 131 of 378 tests reported `Not Run` behind the fixture that
/// failed. Caught by running ctest and not by any unit assertion, which
/// is why the property is pinned here.
///
/// The probe renders into a directory holding nothing but the input, so
/// every other file present afterwards is one the tool wrote — no
/// inference about which writes are side effects, and a future side
/// effect is covered without the gate being told about it.
///
/// It renders **twice**, and that is not redundancy. The source set is
/// collected before the synth children are written, so a first run into
/// a clean directory cannot see them and its depfile is clean no matter
/// what the writer does — a single-render probe stays green with the
/// filter deleted, which is how this one was first written and how the
/// mutation check caught it. The build hit the cycle on the *second*
/// ninja run, reading the depfile of a run whose input directory already
/// held the previous run's output.
#[test]
fn a_depfile_never_names_a_file_the_run_wrote() {
    let mut violations = Vec::new();
    let mut probes = 0usize;
    let mut total_written = 0usize;

    for lang in LANGUAGES {
        probes += 1;
        let work = TempDir::new().expect("tempdir");
        // Input and output share a directory, as the mesh §9.6.6 stages
        // arrange deliberately. A probe that kept them apart could not
        // reach the defect at all.
        let dir = work.path().join("staged");
        std::fs::create_dir_all(&dir).expect("staging directory is creatable");
        let doc = write_sources(&dir, Pipeline::Statechart);

        let before: BTreeSet<PathBuf> = std::fs::read_dir(&dir)
            .expect("staged directory is readable")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();

        // First render: seeds the directory with whatever the tool
        // writes beside its input.
        if let Err(e) = generate(&doc, &dir, lang, &template_root(), None) {
            violations.push(format!(
                "{lang}: first generation into the input directory failed\n{e}"
            ));
            continue;
        }

        let written: BTreeSet<PathBuf> = std::fs::read_dir(&dir)
            .expect("staged directory is readable")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| !before.contains(p))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("scxml"))
            .collect();
        total_written += written.len();

        // Second render: the source set now finds the first render's
        // output, which is the state ninja read when it refused the
        // build.
        let depfile = work.path().join("staged.d");
        if let Err(e) = generate(&doc, &dir, lang, &template_root(), Some(&depfile)) {
            violations.push(format!(
                "{lang}: second generation into the input directory failed\n{e}"
            ));
            continue;
        }

        let prerequisites = depfile_prerequisites(&depfile);
        let cycles: Vec<String> = written
            .iter()
            .filter(|w| {
                prerequisites.iter().any(|p| {
                    p == *w
                        || (p.file_name().is_some() && p.file_name() == w.file_name())
                            && p.parent() == w.parent()
                })
            })
            .map(|w| {
                w.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string()
            })
            .collect();

        if !cycles.is_empty() {
            violations.push(format!(
                "{lang}: the depfile names {} file(s) this run wrote — {cycles:?}. CMake \
                 declares these as OUTPUTs of the same command, so ninja reads the depfile \
                 and refuses the build with a dependency cycle. A file the run produced is \
                 not a reason to re-run it.",
                cycles.len(),
            ));
        }
    }

    assert_eq!(
        probes,
        LANGUAGES.len(),
        "ran {probes} probes, expected {} — a backend stopped being covered",
        LANGUAGES.len(),
    );
    // Without a write, the disjointness above holds vacuously on every
    // backend and the gate reports green having compared nothing.
    assert!(
        total_written > 0,
        "no backend wrote a file into the input directory, so the probe compared the \
         prerequisites against an empty set — the fixture stopped producing the synth or \
         hybrid children the cycle came from",
    );
    assert!(
        violations.is_empty(),
        "depfiles name files their own run produced:\n\n{}",
        violations.join("\n\n"),
    );
}

/// Every document in the synth-6.2.6 source set is a declared
/// prerequisite — including the ones the compile never reads.
///
/// The two prerequisite families above are both *reachability* families:
/// templates the render loads, documents the compile imports. The source
/// set is neither. `DriftContext::compute` folds every `**/*.scxml` under
/// the input root into the `source-hash` the artefact carries, so a
/// document that is never parsed, never imported and never mentioned
/// still changes the bytes written. Measured against the binary before
/// this gate existed, on both pipelines:
///
///   edit an unrelated sibling  →  `source-hash:` line changes
///   declared in the depfile    →  no
///
/// Ninja therefore reuses the artefact, and its embedded hash no longer
/// describes the tree it claims to come from — which is exactly what
/// `sce-codegen verify` refuses. The freshness contract the spec puts on
/// the header is only as good as the prerequisite that maintains it.
///
/// The probe asserts the change first and the declaration second: an
/// omission is a staleness hazard only once editing the omitted file
/// demonstrably moves the output.
#[test]
fn source_set_members_are_declared_prerequisites() {
    let mut violations = Vec::new();
    let mut probes = 0usize;

    for lang in LANGUAGES {
        for pipeline in [Pipeline::Statechart, Pipeline::Forge] {
            probes += 1;
            let case = format!("{lang}/{}", pipeline.label());
            let work = TempDir::new().expect("tempdir");
            let src = work.path().join("src");
            std::fs::create_dir_all(&src).expect("source directory is creatable");
            let doc = write_sources(&src, pipeline);
            std::fs::write(src.join(SOURCE_SET_SIBLING), source_set_sibling(CHAIN_BITS))
                .expect("fixture is writable");
            let root = template_root();

            let before_out = work.path().join("before");
            let depfile = work.path().join("before.d");
            if let Err(e) = generate(&doc, &before_out, lang, &root, Some(&depfile)) {
                violations.push(format!("{case}: baseline generation failed\n{e}"));
                continue;
            }
            let before = emitted_files(&before_out);

            std::fs::write(
                src.join(SOURCE_SET_SIBLING),
                source_set_sibling(CHAIN_BITS_WIDENED),
            )
            .expect("fixture is writable");
            let after_out = work.path().join("after");
            if let Err(e) = generate(&doc, &after_out, lang, &root, None) {
                violations.push(format!(
                    "{case}: generation after the sibling edit failed\n{e}"
                ));
                continue;
            }
            let after = emitted_files(&after_out);

            if before == after {
                violations.push(format!(
                    "{case}: editing a sibling document under the input root left the \
                     compiled document's output byte-identical, so this probe cannot tell \
                     whether the source set is declared. Give the source-hash something \
                     observable to cover before asserting on the depfile.",
                ));
                continue;
            }

            let prerequisites = depfile_prerequisites(&depfile);
            let declared = prerequisites
                .iter()
                .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(SOURCE_SET_SIBLING));
            if !declared {
                violations.push(format!(
                    "{case}: editing this source-set member changes the output (just \
                     demonstrated) but it is not a prerequisite in the depfile, so the \
                     build reuses an artefact whose embedded source-hash no longer \
                     describes its inputs — the state `sce-codegen verify` refuses. \
                     Prerequisites were: {:?}",
                    prerequisites
                        .iter()
                        .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
                        .collect::<Vec<_>>(),
                ));
            }
        }
    }

    assert_eq!(
        probes, EXPECTED_PROBES,
        "ran {probes} probes, expected {EXPECTED_PROBES} — the matrix shrank, so a green \
         result covers less than it claims",
    );
    assert!(
        violations.is_empty(),
        "source-set members are not declared as prerequisites:\n\n{}",
        violations.join("\n\n"),
    );
}
/// The oracle is declared only by a render that baked something out of it.
///
/// The probe above uses the real catalog, where every backend folds reject
/// vectors and so every backend legitimately declares the file. That leaves
/// the other half of the rule — a route that reads nothing from the oracle
/// must not name it — resting on a catalog shape the probe never produces.
/// This constructs that shape: strip the `rejects` arrays, and the five
/// non-C11 routes have nothing left to bake, while C11 still bakes the
/// tolerance and the per-fixture cases.
#[test]
fn the_oracle_is_declared_only_where_it_is_read() {
    let mut violations = Vec::new();
    let mut stripped_total = 0usize;

    for lang in LANGUAGES {
        let work = TempDir::new().expect("tempdir");
        let manifest = stage_conformance_catalog(&work.path().join("forge"));
        let oracle = manifest
            .parent()
            .expect("manifest has a parent")
            .join("numerical_reference.json");

        let text = std::fs::read_to_string(&oracle).expect("oracle catalog is readable");
        let mut doc: serde_json::Value =
            serde_json::from_str(&text).expect("oracle catalog is JSON");
        let mut stripped = 0usize;
        if let Some(codecs) = doc.get_mut("codecs").and_then(|c| c.as_object_mut()) {
            for (_, entry) in codecs.iter_mut() {
                if let Some(obj) = entry.as_object_mut() {
                    if obj.remove("rejects").is_some() {
                        stripped += 1;
                    }
                }
            }
        }
        // A strip that removed nothing would leave the two arms
        // indistinguishable and pass without testing anything.
        assert!(
            stripped > 0,
            "{lang}: the catalog carries no reject vectors to strip, so this probe cannot \
             tell a declaring route from a reading one",
        );
        stripped_total += stripped;
        std::fs::write(
            &oracle,
            serde_json::to_string_pretty(&doc).expect("re-serializes"),
        )
        .expect("oracle catalog is writable");

        let out = work.path().join("out");
        let depfile = work.path().join("out.d");
        if let Err(e) =
            generate_conformance(&manifest, &out, lang, &template_root(), Some(&depfile))
        {
            violations.push(format!("{lang}: render without reject vectors failed\n{e}"));
            continue;
        }
        let named = depfile_prerequisites(&depfile)
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("numerical_reference.json"));
        // C11 bakes the tolerance and the per-fixture cases, so it reads the
        // oracle whatever the reject vectors do; the other five had nothing
        // else to take from it.
        let expected = *lang == "c11";
        if named != expected {
            violations.push(format!(
                "{lang}: with no reject vectors in the catalog the oracle is {} as a \
                 prerequisite, expected {}. Declaration has to track what the render \
                 actually baked — naming a file this route no longer reads only forces \
                 rebuilds, and not naming one it does read reuses a stale harness",
                if named { "named" } else { "absent" },
                if expected { "named" } else { "absent" },
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "the oracle prerequisite does not track what the render baked:\n\n{}",
        violations.join("\n\n"),
    );
    assert!(
        stripped_total >= LANGUAGES.len(),
        "stripped {stripped_total} reject arrays across {} probes — under one per probe \
         means a probe ran against an unmodified catalog",
        LANGUAGES.len(),
    );
}

/// The conformance harness declares its inputs on the same terms.
///
/// Its two CMake steps were the only codegen invocations in the tree
/// with no `DEPFILE`, because `generate-conformance` took no
/// `--write-deps`. They declared inputs by hand instead: the harness
/// template, a `file(GLOB ... CONFIGURE_DEPENDS)` over the per-kind
/// fragments, and the manifest. Both ways that is weaker than a depfile
/// are live here — the glob names what the scaffold includes directly
/// and not what those fragments pull in, and no fixture document was
/// named at all although the harness asserts against every one of them
/// and folds every one into its `source-hash`.
///
/// Both halves are probed the way the gates above probe them: prune
/// every undeclared template and re-render for the template half, edit a
/// fixture and require the change to be declared for the source half.
#[test]
fn the_conformance_harness_declares_the_inputs_it_reads() {
    let mut violations = Vec::new();
    let mut probes = 0usize;

    for lang in LANGUAGES {
        probes += 1;
        let work = TempDir::new().expect("tempdir");
        let manifest = stage_conformance_catalog(&work.path().join("forge"));
        let root = template_root();

        let base_out = work.path().join("base");
        let depfile = work.path().join("base.d");
        if let Err(e) = generate_conformance(&manifest, &base_out, lang, &root, Some(&depfile)) {
            violations.push(format!("{lang}: baseline harness render failed\n{e}"));
            continue;
        }
        let baseline = emitted_files(&base_out);
        let prerequisites = depfile_prerequisites(&depfile);
        let declared: BTreeSet<PathBuf> = prerequisites
            .iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jinja2"))
            .cloned()
            .collect();

        if declared.is_empty() {
            violations.push(format!(
                "{lang}: the harness depfile names no template at all, so every template \
                 edit leaves the harness stale and the build reports success"
            ));
            continue;
        }

        let pruned_root = work.path().join("templates");
        let pruned = copy_pruned(&root, &pruned_root, &declared);
        if pruned < MIN_PRUNED_PER_PROBE {
            violations.push(format!(
                "{lang}: pruned only {pruned} templates, floor {MIN_PRUNED_PER_PROBE} — a \
                 probe that removes nothing re-renders an untouched tree and passes \
                 without having tested anything",
            ));
            continue;
        }
        let probe_out = work.path().join("probe");
        match generate_conformance(&manifest, &probe_out, lang, &pruned_root, None) {
            Ok(()) => {
                if emitted_files(&probe_out) != baseline {
                    violations.push(format!(
                        "{lang}: pruning the undeclared templates changed the harness, so \
                         one of them contributes to it while going undeclared",
                    ));
                }
            }
            Err(e) => violations.push(format!(
                "{lang}: rendering against only the {} declared template(s) failed, so the \
                 depfile omits a template the harness render needs\n{e}",
                declared.len(),
            )),
        }

        // Source half: a fixture document the harness asserts against.
        // Chosen from the staged directory rather than named, so a
        // renamed fixture fails the floor below instead of silently
        // skipping the assertion.
        let resources = manifest
            .parent()
            .and_then(|p| p.parent())
            .expect("manifest sits under <root>/conformance")
            .join("resources");
        let mut fixtures: Vec<PathBuf> = std::fs::read_dir(&resources)
            .expect("staged resources are readable")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("scxml"))
            .collect();
        fixtures.sort();
        let Some(target) = fixtures.first().cloned() else {
            violations.push(format!(
                "{lang}: staged no fixture documents, so the source-set half of this probe \
                 asserts nothing"
            ));
            continue;
        };

        let original = std::fs::read(&target).expect("fixture is readable");
        let mut mutated = original.clone();
        // Appending an XML comment changes the bytes the source-hash
        // folds without changing what the document means, so the render
        // cannot fail for an unrelated reason.
        mutated.extend_from_slice(b"<!-- source-set probe -->\n");
        std::fs::write(&target, &mutated).expect("fixture is writable");

        let after_out = work.path().join("after");
        match generate_conformance(&manifest, &after_out, lang, &root, None) {
            Ok(()) => {
                if emitted_files(&after_out) == baseline {
                    violations.push(format!(
                        "{lang}: editing a fixture document left the harness byte-identical, \
                         so this probe cannot tell whether the source set is declared",
                    ));
                } else {
                    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                    let named = prerequisites
                        .iter()
                        .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(name));
                    if !named {
                        violations.push(format!(
                            "{lang}: editing fixture {name} changes the harness (just \
                             demonstrated) but it is not a prerequisite, so the build \
                             reuses a harness whose embedded source-hash no longer \
                             describes its inputs",
                        ));
                    }
                }
            }
            Err(e) => violations.push(format!(
                "{lang}: harness render after the fixture edit failed\n{e}"
            )),
        }
        std::fs::write(&target, &original).expect("fixture is restorable");

        // The oracle catalog, which only the C11 route reads and which
        // sits outside `resource_dir` — so neither the template scope
        // nor the source set covers it. It reaches the depfile through
        // `RenderedHarness::extra_inputs`; without an assertion here
        // that field would be written and never read, which is how a
        // reporting path rots into decoration.
        let oracle = manifest
            .parent()
            .expect("manifest has a parent")
            .join("numerical_reference.json");
        let oracle_original = std::fs::read_to_string(&oracle).expect("oracle catalog is readable");
        // Values the harness bakes in, not whitespace. Appending a
        // newline is semantic-preserving — the JSON parses to the same
        // tree, the harness renders the same bytes, and the probe
        // concludes the file is unread. The mutation has to move
        // something the render actually emits, and the two surfaces the
        // oracle reaches differ by route: the tolerance is C11-only, the
        // reject vectors are all six.
        let tolerance_moved = oracle_original.replace(ORACLE_TOLERANCE, ORACLE_TOLERANCE_MUTATED);
        assert_ne!(
            tolerance_moved,
            oracle_original,
            "{lang}: the oracle tolerance mutation matched nothing in {} — a probe whose \
             edit does not apply reports `unread` for a file the render does read",
            oracle.display(),
        );
        let oracle_mutated = tolerance_moved.replace(ORACLE_REJECT_WHY, ORACLE_REJECT_WHY_MUTATED);
        assert_ne!(
            oracle_mutated,
            tolerance_moved,
            "{lang}: the oracle reject-vector mutation matched nothing in {} — the probe \
             would degrade to the C11-only tolerance and report the other five routes as \
             not reading a file they do read",
            oracle.display(),
        );
        std::fs::write(&oracle, &oracle_mutated).expect("oracle catalog is writable");
        let oracle_out = work.path().join("oracle");
        match generate_conformance(&manifest, &oracle_out, lang, &root, None) {
            Ok(()) => {
                let changed = emitted_files(&oracle_out) != baseline;
                let named = prerequisites.iter().any(|p| {
                    p.file_name().and_then(|n| n.to_str()) == Some("numerical_reference.json")
                });
                // Only the route that reads it must declare it. Asserting
                // "declared" unconditionally would demand a prerequisite
                // on five backends that never open the file, and asserting
                // "changed" unconditionally would fail on those same five
                // for the same reason — so the two are tied together:
                // whichever routes read it are exactly the routes that
                // have to name it.
                if changed && !named {
                    violations.push(format!(
                        "{lang}: editing numerical_reference.json changes the harness (just \
                         demonstrated) but it is not a prerequisite — it lies outside both \
                         the template scope and the source set, so nothing else can declare \
                         it and the build reuses a stale harness",
                    ));
                }
                if named && !changed {
                    violations.push(format!(
                        "{lang}: numerical_reference.json is declared as a prerequisite but \
                         editing it leaves the harness byte-identical, so this route does \
                         not read it and the declaration only forces spurious rebuilds",
                    ));
                }
            }
            Err(e) => violations.push(format!(
                "{lang}: harness render after the oracle edit failed\n{e}"
            )),
        }
        std::fs::write(&oracle, oracle_original.as_bytes()).expect("oracle catalog is restorable");
    }

    assert_eq!(
        probes,
        LANGUAGES.len(),
        "ran {probes} probes, expected {} — a backend stopped being covered",
        LANGUAGES.len(),
    );
    assert!(
        violations.is_empty(),
        "the conformance harness does not declare the inputs it reads:\n\n{}",
        violations.join("\n\n"),
    );
}
