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
//
// All three have one cause: the depfile computed its template set with
// a second walk of its own instead of asking the loader what it loads.
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

/// A forge algorithm importing a forge codec — the pipeline `sce:kind`
/// routes away from the statechart arm, and the import gives the
/// `<sce:import>` half of the contract something to be measured against.
const FORGE_CODEC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame_codec" version="1.0">
  <datamodel>
    <sce:field id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#;

/// The same codec with a widened field. Used to demonstrate that the
/// imported document is load-bearing before demanding it be declared —
/// an assertion nobody can break is not worth making.
const FORGE_CODEC_MUTATED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame_codec" version="1.0">
  <datamodel>
    <sce:field id="msg_id" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>
"#;

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
            std::fs::write(dir.join("frame_codec.scxml"), FORGE_CODEC)
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

/// The imported forge document is a prerequisite of the importing
/// document's output.
///
/// Asserted in two steps, because the first is what makes the second
/// worth asserting: mutate the imported codec, re-render, and require
/// the output to change — only then is omitting it from the depfile a
/// stale-artefact hazard rather than a cosmetic gap.
#[test]
fn forge_imports_are_declared_prerequisites() {
    let mut violations = Vec::new();

    for lang in LANGUAGES {
        let work = TempDir::new().expect("tempdir");
        let src = work.path().join("src");
        std::fs::create_dir_all(&src).expect("source directory is creatable");
        let doc = write_sources(&src, Pipeline::Forge);
        let import = src.join("frame_codec.scxml");
        let root = template_root();

        let before_out = work.path().join("before");
        let depfile = work.path().join("before.d");
        if let Err(e) = generate(&doc, &before_out, lang, &root, Some(&depfile)) {
            violations.push(format!("{lang}: baseline generation failed\n{e}"));
            continue;
        }
        let before = emitted_files(&before_out);

        std::fs::write(&import, FORGE_CODEC_MUTATED).expect("fixture is writable");
        let after_out = work.path().join("after");
        if let Err(e) = generate(&doc, &after_out, lang, &root, None) {
            violations.push(format!(
                "{lang}: generation after the import edit failed\n{e}"
            ));
            continue;
        }
        let after = emitted_files(&after_out);

        if before == after {
            violations.push(format!(
                "{lang}: widening a field of the imported codec left the importing \
                 document's output byte-identical, so this probe cannot tell whether \
                 the import is declared. Give the mutation something observable to \
                 change before asserting on the depfile.",
            ));
            continue;
        }

        let prerequisites = depfile_prerequisites(&depfile);
        let declared = prerequisites
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("frame_codec.scxml"));
        if !declared {
            violations.push(format!(
                "{lang}: editing the imported `frame_codec.scxml` changes the output \
                 (just demonstrated) but it is not a prerequisite in the depfile, so \
                 the build will reuse the stale artefact. Prerequisites were: {:?}",
                prerequisites
                    .iter()
                    .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
                    .collect::<Vec<_>>(),
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "forge imports are not declared as prerequisites:\n\n{}",
        violations.join("\n\n"),
    );
}
