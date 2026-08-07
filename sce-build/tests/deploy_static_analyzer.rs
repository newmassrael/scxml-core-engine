// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `build.static_analyzer` is descriptive, and that is exactly why it
// needs a reader.
//
// SCE Protocol-Synthesis RFC, synth-5-E lines 1409-1429 lists three accepted
// release configurations for the ownership contract. The second is "the
// defensive layer off plus a recognized commercial analyzer in CI,
// declared via `build.static_analyzer` so deploy review can see which
// one", and the spec is explicit that SCE "does not verify or gate on
// the claim". Nothing about the generated code changes with this key.
//
// A key that changes nothing and is read by nothing is not descriptive,
// it is inert: the author writes it, believes the deployment is
// documented, and no artefact of the build says otherwise. That is the
// shape of the `instances:` defect — a deploy key parsed into a struct
// nobody consulted — and of the silently-inert hooks synth-2.4 forbids. The
// deploy layer already has the antidote elsewhere: permissive
// distributability prints its merge notices precisely so an author sees
// that the analyzer collapsed their partition plan.
//
// So the contract asserted here is not "SCE acts on the declaration" —
// it must not — but "SCE carries it somewhere a reviewer can read it".
// The manifest is that place, and these tests drive the real binary to
// prove the value reaches it.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn codegen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sce-codegen")
}

const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="m" initial="s0">
  <state id="s0"><transition event="go" target="s1"/></state>
  <final id="s1"/>
</scxml>
"#;

/// A single-machine deploy, optionally declaring an analyzer.
fn deploy_yaml(build_block: &str) -> String {
    format!(
        r#"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      m:
        source: m.scxml
{build_block}"#
    )
}

struct Fixture {
    _root: TempDir,
    machine: PathBuf,
    deploy: PathBuf,
    out: PathBuf,
}

impl Fixture {
    fn new(build_block: &str) -> Self {
        let root = TempDir::new().expect("tempdir");
        let machine = root.path().join("m.scxml");
        std::fs::write(&machine, MACHINE).expect("fixture is writable");
        let deploy = root.path().join("deploy.yaml");
        std::fs::write(&deploy, deploy_yaml(build_block)).expect("fixture is writable");
        let out = root.path().join("out");
        std::fs::create_dir_all(&out).expect("output directory is creatable");
        Fixture {
            _root: root,
            machine,
            deploy,
            out,
        }
    }

    /// Rewrite this fixture's deploy and emit into a fresh output
    /// directory, returning it.
    ///
    /// Reusing one fixture rather than building a second is not a
    /// convenience: the generated header records `// From: <path>`, so
    /// two fixtures in two temporary directories differ in a line that
    /// has nothing to do with what is being compared. Any probe that
    /// compares emitted code across deploy variants has to hold the
    /// source path fixed.
    fn with_deploy(&self, build_block: &str, tag: &str) -> PathBuf {
        std::fs::write(&self.deploy, deploy_yaml(build_block)).expect("fixture is writable");
        let out = self.out.join(tag);
        std::fs::create_dir_all(&out).expect("output directory is creatable");
        let output = Command::new(codegen_bin())
            .arg("generate")
            .arg(&self.machine)
            .arg("--deploy")
            .arg(&self.deploy)
            .arg("-o")
            .arg(&out)
            .arg("-l")
            .arg("cpp")
            .env("SOURCE_DATE_EPOCH", "0")
            .output()
            .expect("sce-codegen is runnable");
        assert!(
            output.status.success(),
            "generation failed for {tag}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        out
    }
}

/// Run the generator against `fixture`, returning `(stdout, stderr)` and
/// whether it succeeded.
fn generate(fixture: &Fixture) -> (bool, String, String) {
    let output = Command::new(codegen_bin())
        .arg("generate")
        .arg(&fixture.machine)
        .arg("--deploy")
        .arg(&fixture.deploy)
        .arg("-o")
        .arg(&fixture.out)
        .arg("-l")
        .arg("cpp")
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("sce-codegen is runnable");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// The manifest's `deploy` object, as raw JSON text, or `None` when the
/// manifest omits the key.
///
/// Read by locating the key rather than by deserialising into a mirror
/// struct: a mirror would be a second statement of the wire shape, and
/// the point of the assertion is what the bytes on stdout say.
fn deploy_object(stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?;
    let start = line.find("\"deploy\":")?;
    let rest = &line[start + "\"deploy\":".len()..];
    let open = rest.find('{')?;
    let close = rest[open..].find('}')? + open;
    Some(rest[open..=close].to_string())
}

/// Absent declaration ⇒ the manifest carries no `deploy` key at all.
///
/// Omission rather than `{}` or `null`, so every manifest producible
/// before this field existed stays byte-identical — the same discipline
/// `script_engine_causes` follows (SCE_ERROR_CONTRACT.md §10.1).
#[test]
fn an_undeclared_analyzer_leaves_the_manifest_unchanged() {
    let fixture = Fixture::new("");
    let (ok, stdout, stderr) = generate(&fixture);
    assert!(ok, "generation failed\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !stdout.contains("\"deploy\""),
        "a deploy declaring nothing must not add a manifest key; got: {stdout}",
    );
}

/// A forge document takes a pipeline the deploy never reaches, and the
/// manifest says nothing rather than something untrue.
///
/// `--deploy` is accepted on this route but no deploy-derived model
/// mutation applies to a `sce:kind="codec"` document, so SCE consulted
/// no deployment. Echoing the declaration anyway would attribute it to a
/// build that did not read it — the opposite failure from the silent one
/// the rest of this file guards, and just as misleading to a reviewer.
///
/// Pinned rather than left implicit because the boundary is surprising:
/// the same deploy that produces a `deploy` object for a state machine
/// produces none here. Extending the reader to this route is a
/// deliberate change, and it starts by editing this test.
#[test]
fn a_forge_document_reports_no_deploy_it_never_applied() {
    let root = TempDir::new().expect("tempdir");
    let doc = root.path().join("codec.scxml");
    std::fs::write(
        &doc,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="c1" version="1.0">
  <datamodel>
    <sce:field id="a" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("fixture is writable");
    let deploy = root.path().join("deploy.yaml");
    std::fs::write(
        &deploy,
        r#"version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      c1:
        source: codec.scxml
build:
  static_analyzer: coverity
"#,
    )
    .expect("fixture is writable");
    let out = root.path().join("out");
    std::fs::create_dir_all(&out).expect("output directory is creatable");

    let output = Command::new(codegen_bin())
        .arg("generate")
        .arg(&doc)
        .arg("--deploy")
        .arg(&deploy)
        .arg("-o")
        .arg(&out)
        .arg("-l")
        .arg("cpp")
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("sce-codegen is runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "forge generation failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        stdout.contains("\"artifacts\""),
        "the probe needs a real forge run to be meaningful; got: {stdout}",
    );
    assert!(
        !stdout.contains("\"deploy\""),
        "the forge pipeline applies no deploy, so reporting one claims a deployment \
         SCE never consulted; got: {stdout}",
    );
}

/// Each recognized analyzer reaches the manifest as the spelling the
/// author wrote.
///
/// Both are asserted, and asserted to *differ*: a reader that hardcoded
/// one value would satisfy a single-value test while carrying nothing of
/// the author's declaration.
#[test]
fn every_recognized_analyzer_reaches_the_manifest_as_written() {
    let mut seen: Vec<String> = Vec::new();

    for spelling in ["pc-lint-plus", "coverity"] {
        let fixture = Fixture::new(&format!("build:\n  static_analyzer: {spelling}\n"));
        let (ok, stdout, stderr) = generate(&fixture);
        assert!(
            ok,
            "generation failed for {spelling}\nstdout: {stdout}\nstderr: {stderr}",
        );
        let object = deploy_object(&stdout).unwrap_or_else(|| {
            panic!(
                "the manifest carries no `deploy` object for `{spelling}`, so the \
                 declaration is parsed and then dropped — the author documents a \
                 deployment the build says nothing about. Manifest: {stdout}"
            )
        });
        assert!(
            object.contains(&format!("\"static_analyzer\":\"{spelling}\"")),
            "manifest `deploy` object must echo `{spelling}` verbatim; got {object}",
        );
        seen.push(object);
    }

    assert_eq!(seen.len(), 2, "both recognized analyzers must be probed");
    assert_ne!(
        seen[0], seen[1],
        "both analyzers produced the same manifest object, so the field reports a \
         constant rather than what the deploy declared",
    );
}

/// An analyzer the spec rules out is refused, and the diagnostic names
/// the ones that are not.
///
/// This is vocabulary validation, not adjudication of the claim: SCE
/// still never checks whether CI runs the analyzer. Polyspace is the
/// case worth pinning because synth-5-E rejects it for a reason a user
/// cannot guess — its in-source comments justify findings rather than
/// describe function behaviour, so the annotations SCE emits say nothing
/// to it. Accepting the string would let a deployment read as covered by
/// a tool that cannot see the contract.
#[test]
fn an_analyzer_the_spec_excludes_is_refused_by_name() {
    for spelling in ["polyspace", "clang-tidy"] {
        let fixture = Fixture::new(&format!("build:\n  static_analyzer: {spelling}\n"));
        let (ok, stdout, stderr) = generate(&fixture);
        assert!(
            !ok,
            "`{spelling}` is excluded by synth-5-E but was accepted; stdout: {stdout}",
        );
        assert!(
            stderr.contains("pc-lint-plus") && stderr.contains("coverity"),
            "refusing `{spelling}` must name the recognized analyzers so the author can \
             correct it; got: {stderr}",
        );
    }
}

/// A misspelled `build:` key is refused rather than ignored.
///
/// `deny_unknown_fields` is what makes the block honest: without it
/// `static_analyser:` (or any other near miss) parses to an empty
/// `BuildConfig` and the deployment silently declares nothing, which is
/// the failure this whole file exists to prevent.
#[test]
fn a_misspelled_build_key_is_refused_rather_than_ignored() {
    let fixture = Fixture::new("build:\n  static_analyser: coverity\n");
    let (ok, stdout, stderr) = generate(&fixture);
    assert!(
        !ok,
        "a misspelled key under `build:` must not parse to an empty block; stdout: {stdout}",
    );
    assert!(
        stderr.contains("static_analyser") || stderr.contains("unknown field"),
        "the diagnostic must point at the unknown key; got: {stderr}",
    );
}

/// Every emitted file paired with its text, minus the provenance lines.
///
/// `source-hash` is excluded because synth-6.2.6 hashes every input the
/// run read — deploy.yaml included — so *any* deploy edit moves it, a
/// YAML comment as much as a declaration. Comparing it would assert that
/// the provenance header works, not that the key is inert.
fn emitted_bodies(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("output directory is readable") {
        let entry = entry.expect("directory entry is readable");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("artefact is readable text");
        let body: String = text
            .lines()
            .filter(|l| !l.contains("source-hash") && !l.contains("source_hash"))
            .collect::<Vec<_>>()
            .join("\n");
        out.push((
            path.file_name()
                .expect("file has a name")
                .to_string_lossy()
                .to_string(),
            body,
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Declaring an analyzer is no more load-bearing than a YAML comment.
///
/// The spec calls the key descriptive rather than load-bearing, which is
/// a claim about the emitted code. Stating it as "changes no byte" would
/// be false for a reason that has nothing to do with this key: the
/// synth-6.2.6 `source-hash` covers deploy.yaml, so editing any
/// character of it — including a comment — moves that line.
///
/// The comment is therefore the control. A run whose deploy gained only
/// a comment and a run whose deploy gained the declaration must emit the
/// same code, which is the strongest available statement of "this key
/// selects no emission". The provenance line is asserted to have moved
/// in both, so the comparison is known to be running on two genuinely
/// different inputs rather than on one input twice.
#[test]
fn declaring_an_analyzer_is_no_more_load_bearing_than_a_comment() {
    // One fixture, three deploys: the emitted header records the source
    // path, so three fixtures would differ on `// From:` alone.
    let fixture = Fixture::new("");
    let base = fixture.with_deploy("", "base");
    let commented = fixture.with_deploy("# a comment that declares nothing\n", "commented");
    let declared = fixture.with_deploy("build:\n  static_analyzer: coverity\n", "declared");

    let base_text = std::fs::read_to_string(base.join("m_sm.h")).expect("header exists");
    let commented_text = std::fs::read_to_string(commented.join("m_sm.h")).expect("header exists");
    let declared_text = std::fs::read_to_string(declared.join("m_sm.h")).expect("header exists");
    let source_hash = |t: &str| -> String {
        t.lines()
            .find(|l| l.contains("source-hash"))
            .unwrap_or("<none>")
            .to_string()
    };
    assert_ne!(
        source_hash(&base_text),
        source_hash(&commented_text),
        "a deploy comment did not move `source-hash`, so the provenance header does not \
         cover deploy.yaml and this probe's control is not a control",
    );
    assert_ne!(
        source_hash(&base_text),
        source_hash(&declared_text),
        "the declaration did not move `source-hash`, so deploy.yaml is not covered by \
         provenance on this path",
    );

    let a = emitted_bodies(&commented);
    let b = emitted_bodies(&declared);
    assert!(!a.is_empty(), "the probe compared two empty output sets");
    assert_eq!(
        a, b,
        "declaring an analyzer emitted different code than an inert deploy comment, but \
         synth-5-E calls the key descriptive rather than load-bearing — no emission may \
         depend on it",
    );
}

/// The recognized set matches the analyzer families the ownership
/// contract actually renders.
///
/// `StaticAnalyzer` names the analyzers SCE's annotations speak to, and
/// `OwnershipContract::rendered_lines` is what emits them — one PC-lint
/// `-sem` line plus the Coverity model primitives. Held by this equality
/// rather than derived, because the renderers are methods rather than an
/// enumerable registry: adding a third family to `rendered_lines`
/// without a matching variant makes the counts disagree, so the deploy
/// vocabulary cannot silently fall behind what codegen emits.
#[test]
fn the_recognized_set_matches_what_the_ownership_contract_renders() {
    use sce_build::forge::ownership_contract::RUNTIME_CONTRACTS;

    let mut contributing = 0usize;
    for contract in RUNTIME_CONTRACTS.iter() {
        let pc_lint = usize::from(contract.pc_lint_annotation().is_some());
        let coverity = contract.coverity_annotations().len();
        contributing += pc_lint + coverity;
        assert_eq!(
            contract.rendered_lines().len(),
            pc_lint + coverity,
            "`{}` renders lines that belong to neither analyzer family named by \
             `StaticAnalyzer`. Either add the variant to \
             `mesh::deploy::StaticAnalyzer` so a deployment can declare it, or the \
             deploy vocabulary now claims less than codegen emits.",
            contract.name,
        );
    }
    assert!(
        contributing > 0,
        "no contract rendered an annotation, so the equality above held vacuously",
    );
}
