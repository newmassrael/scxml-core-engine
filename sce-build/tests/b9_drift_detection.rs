// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! B9 §synth-6.2.6 generated-source drift detection — end-to-end fixture.
//!
//! Pairs the library helper [`sce_build::apply_drift_headers_to_output`]
//! with the `sce-codegen verify` subcommand:
//!
//! 1. Compute the `source-hash` over a synthetic SCXML root.
//! 2. Apply headers to a hand-built [`generator::GeneratedOutput`].
//! 3. Write the headered output to a temp dir.
//! 4. Invoke `sce-codegen verify <out-dir>` as a subprocess.
//! 5. Assert: clean state passes; tampered source fails with
//!    `forge/source-hash-mismatch`.
//!
//! Why the helper-based fixture rather than full codegen: the helper
//! exposes the contract surface (the "6-backend header
//! emit") without coupling to any one backend's generator pipeline. The
//! follow-up atomic wires `apply_drift_headers_to_output` into every
//! `cmd_*` codegen entry; that integration is out of scope for B9 per
//! the RFC's `[[feedback-design-preflight]]` one-atomic-one-scope
//! discipline.
//!
//! `verify` answers whether generated files were generated from the
//! current inputs. Whether they hold what the current generator produces
//! is judged on content by `--assert-unchanged`
//! (`generation_can_assert_its_output_unchanged.rs`).

mod common;

use sce_build::forge::drift::{ClosureOptions, DriftHeader, InputClosure, HEADER_BANNER};
use sce_build::generator::GeneratedOutput;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct VerifyFixture {
    _root: TempDir,
    input_root: PathBuf,
    out_dir: PathBuf,
}

impl VerifyFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let input_root = root.path().join("input");
        let out_dir = root.path().join("out");
        fs::create_dir_all(&input_root).unwrap();
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(input_root.join("foo.scxml"), b"<scxml/>").unwrap();
        VerifyFixture {
            _root: root,
            input_root,
            out_dir,
        }
    }

    /// Synthetic codegen — emits a one-file `GeneratedOutput` then
    /// invokes the library helper to prepend the §synth-6.2.6 header. Body
    /// content is intentionally simple Rust so a future reader can
    /// recognise the test is purely about the header/verify contract.
    fn generate_headered_rust(&self, header: &DriftHeader) {
        let mut output = GeneratedOutput {
            files: vec![(
                "foo_sm.rs".to_string(),
                "pub struct Foo;\n\nimpl Foo {\n    pub fn new() -> Self { Foo }\n}\n".to_string(),
            )],
            ..Default::default()
        };
        sce_build::apply_drift_headers_to_output(&mut output, header);
        for (filename, content) in output.files {
            let dest = self.out_dir.join(filename);
            fs::write(dest, content).unwrap();
        }
    }

    /// Invokes the `sce-codegen verify` binary as a subprocess and
    /// returns (exit_code, stdout, stderr).
    fn run_verify(&self) -> (i32, String, String) {
        self.run_verify_from(None)
    }

    /// Variant that pins the spawned binary's working directory, so a
    /// test about the working directory changes the child's and not the
    /// shared process cwd other tests run in.
    fn run_verify_from(&self, cwd: Option<&Path>) -> (i32, String, String) {
        let bin = env_bin();
        let mut cmd = Command::new(bin);
        cmd.arg("verify")
            .arg(self.out_dir.to_str().unwrap())
            .arg("--input-root")
            .arg(self.input_root.to_str().unwrap());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let output = cmd.output().expect("spawn sce-codegen");
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (code, stdout, stderr)
    }
}

fn env_bin() -> &'static str {
    // CARGO_BIN_EXE_sce-codegen is populated by Cargo when the test
    // target declares `required-features = ["cli"]`. See Cargo.toml.
    env!("CARGO_BIN_EXE_sce-codegen")
}

/// The header of a file generated from `document`: its input closure.
fn header_of(document: &Path) -> DriftHeader {
    DriftHeader::of(
        &InputClosure::of(&[document], ClosureOptions::default())
            .unwrap_or_else(|e| panic!("the closure of {} is read: {e}", document.display())),
    )
}

fn header_for_fixture(fix: &VerifyFixture) -> DriftHeader {
    header_of(&fix.input_root.join("foo.scxml"))
}

#[test]
fn verify_passes_on_clean_round_trip() {
    let fix = VerifyFixture::new();
    fix.generate_headered_rust(&header_for_fixture(&fix));
    let (code, _stdout, stderr) = fix.run_verify();
    assert_eq!(code, 0, "verify must pass on clean state. stderr: {stderr}");
}

#[test]
fn verify_fails_when_source_drifts() {
    let fix = VerifyFixture::new();
    fix.generate_headered_rust(&header_for_fixture(&fix));

    // Drift the input SCXML after generation. The embedded header
    // still carries the pre-drift hash; recompute will not match.
    fs::write(fix.input_root.join("foo.scxml"), b"<scxml version='1.0'/>").unwrap();

    let (code, _stdout, stderr) = fix.run_verify();
    assert_ne!(
        code, 0,
        "verify must fail when source drifted. stderr: {stderr}"
    );
    assert!(
        stderr.contains("source-hash") || stderr.contains("forge/source-hash-mismatch"),
        "diagnostic must name the drift axis. stderr: {stderr}"
    );
}

/// A header of a shape before ADR 0094 names no document, so `verify` has
/// no closure to recompute for it: a tree that missed the migration is
/// refused, whatever its `source-hash` says. It is still read as a header,
/// not skipped as headerless.
#[test]
fn verify_refuses_a_header_that_names_no_document() {
    let fix = VerifyFixture::new();
    let header = header_for_fixture(&fix);
    fs::write(
        fix.out_dir.join("foo_sm.rs"),
        format!(
            "// {HEADER_BANNER}\n// source-hash: {}\n// template-hash: {}\n\
             // generated-at: 0\npub struct Foo;\n",
            header.source_hex(),
            "ab".repeat(32),
        ),
    )
    .unwrap();

    let (code, _stdout, stderr) = fix.run_verify();
    assert_ne!(
        code, 0,
        "a header with no `source:` line must not pass. stderr: {stderr}"
    );
    assert!(
        stderr.contains("source:"),
        "the refusal names what the header lacks. stderr: {stderr}"
    );
    assert!(
        !stderr.contains("headerless"),
        "a legacy header was not recognised as a header. stderr: {stderr}"
    );
}

#[test]
fn helper_emits_python_header_with_hash_prefix() {
    // 6-backend coverage check: helper picks `#` for `.py` and `//`
    // for everything else. This is the cross-backend invariant from
    // Single helper, prefix derived from file extension.
    let mut output = GeneratedOutput {
        files: vec![
            ("foo.py".into(), "def main():\n    pass\n".into()),
            ("foo.rs".into(), "pub fn main() {}\n".into()),
            ("foo.cpp".into(), "int main() { return 0; }\n".into()),
            ("foo.h".into(), "#pragma once\n".into()),
            ("foo.kt".into(), "fun main() {}\n".into()),
            ("foo.go".into(), "package main\n\nfunc main() {}\n".into()),
            ("foo.c".into(), "int main() { return 0; }\n".into()),
        ],
        ..Default::default()
    };
    let header = DriftHeader {
        source_hash: [0xaa; 32],
        seeds: vec!["foo.scxml".to_string()],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &header);

    for (filename, content) in &output.files {
        let prefix_expected = if filename.ends_with(".py") {
            "# "
        } else {
            "// "
        };
        let first_line = content.lines().next().unwrap();
        assert!(
            first_line.starts_with(prefix_expected),
            "{filename}: first line `{first_line}` lacks expected prefix `{prefix_expected}`"
        );
        assert!(
            first_line.contains("SCE-GENERATED"),
            "{filename}: first line must include the SCE-GENERATED banner"
        );
    }
}

#[test]
fn helper_is_idempotent_across_two_invocations() {
    // The wrapper must be re-runnable on already-headered output
    // without duplicating the block — important for the future
    // `apply_drift_headers + reformat + apply_drift_headers` flow
    // some build systems impose.
    let mut output = GeneratedOutput {
        files: vec![("foo.rs".into(), "pub fn x() {}\n".into())],
        ..Default::default()
    };
    let header = DriftHeader {
        source_hash: [0x11; 32],
        seeds: vec!["foo.scxml".to_string()],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &header);
    let once = output.files[0].1.clone();
    sce_build::apply_drift_headers_to_output(&mut output, &header);
    let twice = output.files[0].1.clone();
    assert_eq!(
        once, twice,
        "helper must be idempotent under repeat application"
    );
}

#[test]
fn verify_passes_when_run_from_different_working_directory() {
    // Given absolute paths, `verify` reads nothing relative to where it
    // runs. Pin the spawned binary's cwd via `Command::current_dir`
    // instead of mutating the process-global cwd, which would race with
    // other parallel tests.
    let fix = VerifyFixture::new();
    fix.generate_headered_rust(&header_for_fixture(&fix));
    let alt_cwd = fix._root.path().join("alt-cwd");
    fs::create_dir_all(&alt_cwd).unwrap();
    let (code, _stdout, stderr) = fix.run_verify_from(Some(&alt_cwd));
    assert_eq!(
        code, 0,
        "verify with absolute paths must be cwd-independent. stderr: {stderr}"
    );
}

/// A file whose document is under none of the roots `verify` is given has
/// no closure to recompute, and is refused rather than passed.
#[test]
fn verify_refuses_a_file_whose_document_is_under_no_root() {
    let fix = VerifyFixture::new();
    fix.generate_headered_rust(&header_for_fixture(&fix));
    fs::remove_file(fix.input_root.join("foo.scxml")).unwrap();

    let (code, _stdout, stderr) = fix.run_verify();
    assert_ne!(
        code, 0,
        "a file whose document is gone must not pass. stderr: {stderr}"
    );
    assert!(
        stderr.contains("foo.scxml"),
        "the refusal names the document it could not find. stderr: {stderr}"
    );
}

/// ADR 0094 test 5 through the command: a staging script copies its tracked
/// fixture into a directory of its own and names the tracked directory as
/// `--input-root`. The copy's files carry the header a generation of the
/// tracked document gives, so `verify` against the tracked directory passes;
/// and a sibling of the tracked document, which it never reads, moves
/// nothing.
#[test]
fn a_staged_copy_carries_the_header_of_the_document_it_stands_for() {
    let root = TempDir::new().unwrap();
    let canonical = root.path().join("canonical");
    let stage = root.path().join("stage");
    let out_dir = root.path().join("out");
    fs::create_dir_all(&canonical).unwrap();
    fs::create_dir_all(&stage).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    // A minimal SCXML that survives the parser + Rust codegen path.
    let scxml = "<?xml version='1.0'?>\n\
                 <scxml xmlns='http://www.w3.org/2005/07/scxml' \
                 initial='s0' version='1.0' datamodel='ecmascript'>\
                 <state id='s0'/></scxml>";
    fs::write(canonical.join("foo.scxml"), scxml).unwrap();
    fs::write(stage.join("foo.scxml"), scxml).unwrap();
    // A sibling of the tracked document, which it does not read: under the
    // directory rule it moved the header, under the closure it does not.
    let sibling = canonical.join("bar.scxml");
    fs::write(
        &sibling,
        "<?xml version='1.0'?>\n\
         <scxml xmlns='http://www.w3.org/2005/07/scxml' \
         initial='b0' version='1.0' datamodel='ecmascript'>\
         <state id='b0'/></scxml>",
    )
    .unwrap();

    let bin = env_bin();
    let result = Command::new(bin)
        .arg("generate")
        .arg(stage.join("foo.scxml").to_str().unwrap())
        .arg("--input-root")
        .arg(canonical.to_str().unwrap())
        .arg("-l")
        .arg("rust")
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("spawn sce-codegen generate");
    assert!(
        result.status.success(),
        "generate failed. stdout: {} stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );

    let tracked_header = header_of(&canonical.join("foo.scxml"));
    let foo_sm = out_dir.join("foo_sm.rs");
    assert!(
        foo_sm.exists(),
        "generate must emit foo_sm.rs at {}",
        foo_sm.display()
    );
    let content = fs::read_to_string(&foo_sm).unwrap();
    let expected_line = format!("source-hash: {}", tracked_header.source_hex());
    assert!(
        content.contains(&expected_line) && content.contains("source: foo.scxml\n"),
        "the copy's file must carry the tracked document's header. expected line: \
         `{expected_line}`\nfirst 5 lines of {}:\n{}",
        foo_sm.display(),
        content.lines().take(5).collect::<Vec<_>>().join("\n"),
    );

    let verify = || {
        Command::new(bin)
            .arg("verify")
            .arg(out_dir.to_str().unwrap())
            .arg("--input-root")
            .arg(canonical.to_str().unwrap())
            .output()
            .unwrap()
    };
    let result = verify();
    assert_eq!(
        result.status.code(),
        Some(0),
        "verify against the tracked directory must pass. stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );

    fs::write(
        &sibling,
        "<?xml version='1.0'?>\n\
         <scxml xmlns='http://www.w3.org/2005/07/scxml' \
         initial='b1' version='1.0' datamodel='ecmascript'>\
         <state id='b1'/></scxml>",
    )
    .unwrap();
    let result = verify();
    assert_eq!(
        result.status.code(),
        Some(0),
        "an edit to a sibling the document never reads moves nothing. stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );
}

/// A statechart that starts the child `child.scxml` by `src`.
const PARENT_OF_A_FILE_CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="parent" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="done.invoke" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
"#;

/// The child `PARENT_OF_A_FILE_CHILD` starts, its final state named `mark`.
fn file_child(mark: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<scxml \
         xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" name=\"child\" \
         initial=\"{mark}\">\n  <final id=\"{mark}\"/>\n</scxml>\n"
    )
}

/// A directory holding `parent.scxml`, the child it starts, and `lone.scxml`,
/// which reads neither.
fn three_documents(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("parent.scxml"), PARENT_OF_A_FILE_CHILD).unwrap();
    fs::write(dir.join("child.scxml"), file_child("done")).unwrap();
    fs::write(
        dir.join("lone.scxml"),
        COVERAGE_FIXTURE.replace("covered", "lone"),
    )
    .unwrap();
}

/// Every file under `dir`, by name, with its bytes.
fn tree(dir: &Path) -> std::collections::BTreeMap<String, String> {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(entry.path()).unwrap_or_default(),
            )
        })
        .collect()
}

fn source_hash_line(text: &str) -> Option<&str> {
    text.lines()
        .find(|line| line.starts_with("// source-hash: "))
}

/// ADR 0094 test 1: editing one document moves no header of a file whose
/// closure excludes it. `lone`'s files are byte for byte what they were;
/// `parent`, which reads the child for its interface, has a new header.
#[test]
fn editing_one_document_moves_no_header_whose_closure_excludes_it() {
    let root = TempDir::new().unwrap();
    let input = root.path().join("input");
    three_documents(&input);
    let lone_out = root.path().join("lone");
    let parent_out = root.path().join("parent");
    fs::create_dir_all(&lone_out).unwrap();
    fs::create_dir_all(&parent_out).unwrap();
    let generate_both = || {
        for (doc, out) in [("lone.scxml", &lone_out), ("parent.scxml", &parent_out)] {
            let (code, stderr) = run_generate(&input.join(doc), out, &[]);
            assert_eq!(code, Some(0), "generate {doc}: {stderr}");
        }
    };
    generate_both();
    let lone_before = tree(&lone_out);
    let parent_before = fs::read_to_string(parent_out.join("parent_sm.rs")).unwrap();
    assert!(!lone_before.is_empty(), "lone generated nothing");

    fs::write(input.join("child.scxml"), file_child("finished")).unwrap();
    generate_both();

    assert_eq!(
        tree(&lone_out),
        lone_before,
        "a document that does not read the child keeps every byte, header included"
    );
    let parent_after = fs::read_to_string(parent_out.join("parent_sm.rs")).unwrap();
    assert_ne!(
        source_hash_line(&parent_after),
        source_hash_line(&parent_before),
        "the parent reads the child, so its header moves"
    );
}

/// ADR 0094 test 8: after an edit, `verify` refuses exactly the files whose
/// closure moved, with the digest of that file's own closure as `expected`,
/// and passes once those alone are generated again.
#[test]
fn verify_refuses_exactly_the_files_whose_closure_moved() {
    let root = TempDir::new().unwrap();
    let input = root.path().join("input");
    three_documents(&input);
    let out = root.path().join("out");
    fs::create_dir_all(&out).unwrap();
    for doc in ["lone.scxml", "parent.scxml"] {
        let (code, stderr) = run_generate(&input.join(doc), &out, &[]);
        assert_eq!(code, Some(0), "generate {doc}: {stderr}");
    }
    let (code, stderr) = run_verify_real_tree(&out, &input);
    assert_eq!(code, 0, "a fresh tree passes: {stderr}");

    fs::write(input.join("child.scxml"), file_child("finished")).unwrap();
    let (code, stderr) = run_verify_real_tree(&out, &input);
    assert_ne!(code, 0, "the parent's closure moved: {stderr}");
    assert!(stderr.contains("parent_sm.rs"), "{stderr}");
    let expected = header_of(&input.join("parent.scxml")).source_hex();
    assert!(
        stderr.contains(&format!("recomputed={expected}")),
        "expected is the parent's own closure: {stderr}"
    );

    let (code, stderr) = run_generate(&input.join("parent.scxml"), &out, &[]);
    assert_eq!(code, Some(0), "{stderr}");
    let (code, stderr) = run_verify_real_tree(&out, &input);
    assert_eq!(
        code, 0,
        "lone's files were never stale, so the parent's alone fix the tree: {stderr}"
    );
}

/// A statechart holding one machine written in-line, its `<invoke id="inner">`.
const HOLDER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="holder" initial="s0">
  <state id="s0">
    <invoke id="inner" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
"#;

/// The `source` lines of a generated file's header.
fn header_lines(text: &str) -> Vec<String> {
    text.lines()
        .take_while(|line| line.starts_with("// "))
        .filter(|line| line.contains("source"))
        .map(str::to_string)
        .collect()
}

/// `input/holder.scxml` generated into `out`, both under `root`; the
/// document's path.
fn generated_holder(root: &Path) -> PathBuf {
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&out).unwrap();
    let doc = input.join("holder.scxml");
    fs::write(&doc, HOLDER).unwrap();
    let (code, stderr) = run_generate(&doc, &out, &[]);
    assert_eq!(code, Some(0), "{stderr}");
    doc
}

/// ADR 0094 test 7: a machine written in-line in an `<invoke>` is generated
/// from the document that holds its text, so its files carry that
/// document's header.
#[test]
fn an_inline_child_carries_its_parents_header() {
    let root = TempDir::new().unwrap();
    generated_holder(root.path());
    let out = root.path().join("out");

    let files = tree(&out);
    let parent = header_lines(files.get("holder_sm.rs").expect("the holder's machine"));
    let children: Vec<&String> = files
        .iter()
        .filter(|(name, _)| name.contains("__sce_synth_invoke__") && name.ends_with(".rs"))
        .map(|(_, text)| text)
        .collect();
    assert!(
        !children.is_empty(),
        "no in-line child was generated: {:?}",
        files.keys()
    );
    for child in children {
        assert_eq!(
            header_lines(child),
            parent,
            "the child carries the holder's header"
        );
    }
    assert!(
        parent.iter().any(|line| line == "// source: holder.scxml"),
        "{parent:?}"
    );
}

/// ADR 0094, owner's answer 2: the document of an in-line child that a run
/// writes records the document it came from, and a run that generates from
/// that written document, as the integration scripts and a Mesh partition's
/// run do, is stamped with that document's closure and names it, so
/// `verify` finds it under the root that holds the authored document.
#[test]
fn a_written_child_carries_the_header_of_the_document_it_came_from() {
    let root = TempDir::new().unwrap();
    let doc = generated_holder(root.path());
    let input = root.path().join("input");
    let written = root
        .path()
        .join("out")
        .join("holder__sce_synth_invoke__inner.scxml");
    let text = fs::read_to_string(&written).expect("the run writes the child's document");
    assert_eq!(
        sce_build::forge::drift::recorded_origin(&text).as_deref(),
        Some("holder.scxml"),
        "the written child records its origin: {text}"
    );

    let child_out = root.path().join("child_out");
    fs::create_dir_all(&child_out).unwrap();
    let (code, stderr) = run_generate(
        &written,
        &child_out,
        &[
            "--as-child",
            "--parent-stem",
            "holder",
            "--input-root",
            input.to_str().unwrap(),
        ],
    );
    assert_eq!(code, Some(0), "{stderr}");
    let expected = vec![
        format!("// source-hash: {}", header_of(&doc).source_hex()),
        "// source: holder.scxml".to_string(),
    ];
    let headed: Vec<String> = tree(&child_out)
        .into_values()
        .filter(|text| text.starts_with("// SCE-GENERATED"))
        .collect();
    assert!(
        !headed.is_empty(),
        "the child's run wrote no drift-headed file"
    );
    for file in &headed {
        assert_eq!(
            header_lines(file),
            expected,
            "the child carries its origin's header"
        );
    }
    let (code, stderr) = run_verify_real_tree(&child_out, &input);
    assert_eq!(code, 0, "verify finds the origin under the root: {stderr}");
}

/// ADR 0094 test 13: a depfile declares the closure, so a sibling the
/// document never reads is not a prerequisite and its edit re-runs nothing.
#[test]
fn the_depfile_names_the_closure_and_no_sibling() {
    let root = TempDir::new().unwrap();
    let input = root.path().join("input");
    three_documents(&input);
    let out = root.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let depfile = root.path().join("parent.d");
    let (code, stderr) = run_generate(
        &input.join("parent.scxml"),
        &out,
        &["--write-deps", depfile.to_str().unwrap()],
    );
    assert_eq!(code, Some(0), "{stderr}");
    let deps = fs::read_to_string(&depfile).unwrap();
    assert!(deps.contains("parent.scxml"), "{deps}");
    assert!(
        deps.contains("child.scxml"),
        "the child the parent reads: {deps}"
    );
    assert!(
        !deps.contains("lone.scxml"),
        "a sibling it never reads: {deps}"
    );
}

#[test]
fn fixture_helper_invariants() {
    // Sanity: fixture setup itself produces parseable hash output, so
    // a test failure later isolates the actual verify logic rather
    // than fixture wiring.
    let fix = VerifyFixture::new();
    let header = header_for_fixture(&fix);
    assert_ne!(
        header.source_hash, [0u8; 32],
        "synthetic fixture must produce non-zero source-hash"
    );
    assert!(fix.input_root.is_dir());
    let _: &Path = fix.out_dir.as_path();
}

// Resolves the SCE workspace root from this crate's compile-time
// `CARGO_MANIFEST_DIR` (= `<workspace>/sce-build`). The real-tree
// invariants below use it to point `sce-codegen verify` at the
// canonical committed generated trees and the W3C `resources/` input
// set the original `generate-w3c` invocation hashed against.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build crate dir must have a parent (the workspace root)")
        .to_path_buf()
}

// The synthetic fixtures above exercise the verify contract on hand-built
// inputs. The invariants below run the same `sce-codegen verify` against
// the committed trees: each tree's embedded `source-hash` must still
// describe its input root, so an input edited without regenerating the
// tree it feeds is caught here, on every run, without regenerating
// anything. A template or generator change is a question about content
// instead — the `regen-reproduces` lane and `--assert-unchanged` answer it.
//
// Each tree is verified against its own input root, because each is a
// distinct drift context. The hand-authored `donedata_local_invoke`
// fixture hashes against its fixture dir, not `resources/`; mixing the
// two under one verify call would mis-attribute the donedata
// source-hash as a W3C drift. Layout per backend:
//
//   Rust:   W3C SM under `backends/rust/tests/src/generated/`
//           donedata SM under `backends/rust/tests/src/integration/donedata_local_invoke/`
//           ⇒ verify `src/generated/` is donedata-free.
//
//   Kotlin: W3C SM under `backends/kotlin/w3c/src/main/kotlin/com/sce/generated/`,
//           a module of its own; donedata SM under
//           `backends/kotlin/tests/src/main/kotlin/com/sce/integration/donedata_local_invoke/`
//           ⇒ verify the W3C test-class tree
//           (`backends/kotlin/w3c/src/test/kotlin/com/sce/w3c/`), which
//           shares the W3C drift context with the SM tree (one
//           `generate-w3c -l kotlin` invocation emits both atomically, so
//           test-class-fresh ⇒ SM-fresh).

fn run_verify_real_tree(target: &Path, input_root: &Path) -> (i32, String) {
    let bin = env_bin();
    let result = Command::new(bin)
        .arg("verify")
        .arg(target.to_str().unwrap())
        .arg("--input-root")
        .arg(input_root.to_str().unwrap())
        .output()
        .expect("spawn sce-codegen");
    let code = result.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    (code, stderr)
}

#[test]
fn verify_passes_on_real_committed_rust_w3c_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/rust/tests")
        .join("src")
        .join("generated");
    let input_root = workspace.join("resources");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust W3C generated tree. \
         A failure here means resources/ changed without refreshing \
         backends/rust/tests/src/generated/. Run \
         `scripts/regen_all_committed_trees.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

/// ADR 0094 test 16 for the tree the defect was measured on: every file of
/// the committed Rust `static_datamodel` tree states the closure of the
/// documents it was generated from, found under the three fixture
/// directories its regeneration script reads (standard documents need no
/// root). Before the ADR one digest of the whole fixture directory stood in
/// all of them.
#[test]
fn verify_passes_on_real_committed_rust_static_datamodel_tree() {
    let workspace = workspace_root();
    let target = workspace.join("backends/rust/tests/src/integration/static_datamodel");
    let fixtures = workspace.join("sce-build/tests/fixtures");
    let result = Command::new(env_bin())
        .arg("verify")
        .arg(&target)
        .arg("--input-root")
        .arg(fixtures.join("static_datamodel"))
        .arg("--input-root")
        .arg(fixtures.join("test_profile"))
        .arg("--input-root")
        .arg(fixtures.join("union"))
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        result.status.code(),
        Some(0),
        "verify must pass on the committed Rust static_datamodel tree. A failure \
         here means a fixture changed without refreshing it. Run \
         `scripts/materialize.sh rust-static-datamodel` and commit the result. stderr:\n{}",
        String::from_utf8_lossy(&result.stderr),
    );
}

#[test]
fn verify_passes_on_real_committed_kotlin_w3c_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/kotlin/w3c")
        .join("src")
        .join("test")
        .join("kotlin")
        .join("com")
        .join("sce")
        .join("w3c");
    let input_root = workspace.join("resources");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Kotlin W3C generated tree. \
         A failure here means resources/ changed without refreshing the \
         committed Kotlin generated tree. Run \
         `scripts/regen_all_committed_trees.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

// Donedata drift context. The canonical
// `donedata_local_invoke.scxml` fixture lives at
// `integration_resources/donedata_local_invoke/`;
// all three committed-tree backends share that input root. The new
// top-level dir is intentionally outside `resources/` — the W3C
// `resources/<N>/` tree is a *separate* §synth-6.2.6 input root, and
// `compute_source_hash` recurses through the input root, so nesting
// integration under `resources/` would fold the integration fixture
// into the W3C source-hash domain. Distinct drift contexts demand
// distinct top-level dirs. Regen via `scripts/materialize.sh rust-integration`,
// `scripts/materialize.sh kotlin-integration` and
// `scripts/materialize.sh go-integration`.
//
// Python is intentionally skipped: `backends/python/bindings/tests/` runs the
// pybind11 → C++ Interpreter channel, so no donedata SM is codegen'd
// for Python — there is no committed §synth-6.2.6 header to verify.

#[test]
fn verify_passes_on_real_committed_rust_donedata_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/rust/tests")
        .join("src")
        .join("integration")
        .join("donedata_local_invoke");
    let input_root = workspace
        .join("integration_resources")
        .join("donedata_local_invoke");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust donedata tree. A \
         failure here means \
         integration_resources/donedata_local_invoke/donedata_local_invoke.scxml \
         changed without refreshing backends/rust/tests/src/integration/donedata_local_invoke/. \
         Run `scripts/materialize.sh rust-integration` and commit the \
         result. stderr:\n{stderr}"
    );
}

#[test]
fn verify_passes_on_real_committed_kotlin_donedata_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/kotlin/tests")
        .join("src")
        .join("main")
        .join("kotlin")
        .join("com")
        .join("sce")
        .join("integration")
        .join("donedata_local_invoke");
    let input_root = workspace
        .join("integration_resources")
        .join("donedata_local_invoke");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Kotlin donedata tree. A \
         failure here means \
         integration_resources/donedata_local_invoke/donedata_local_invoke.scxml \
         changed without refreshing the donedata generated dir. Run \
         `scripts/materialize.sh kotlin-integration` and commit the \
         result. stderr:\n{stderr}"
    );
}

#[test]
fn verify_passes_on_real_committed_go_donedata_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/go/tests")
        .join("integration")
        .join("donedata_local_invoke");
    let input_root = workspace
        .join("integration_resources")
        .join("donedata_local_invoke");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Go donedata tree. A \
         failure here means \
         integration_resources/donedata_local_invoke/donedata_local_invoke.scxml \
         changed without refreshing backends/go/tests/donedata_local_invoke/. \
         Run `scripts/materialize.sh go-integration` and commit the \
         result. stderr:\n{stderr}"
    );
}

// Forge variant-default round-trip Go drift context. The 3 codec
// fixtures under `tests/forge/resources/codec_{default,variant_default}_marker*`
// are emitted via `backends/go/forge-runtime/round_trip/generate.sh` to
// `backends/go/forge-runtime/round_trip/generated/`. The Go runtime test
// `default_round_trip_test.go` includes them at build time. This
// context was discovered after the §synth-6.2.6 sweep audit found the
// previous run of generate.sh had been on the pre-writer-encode template
// tree — committed Encode() returned `[]byte` while the current
// template emits `Encode(SceSink) error` + a `EncodeToBytes() []byte`
// legacy facade. No CI lane was running `go test ./round_trip/`, so
// the API mismatch surfaced only via `verify`.
#[test]
fn verify_passes_on_real_committed_forge_default_round_trip_go_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/go/forge-runtime")
        .join("round_trip")
        .join("generated");
    let input_root = workspace.join("tests").join("forge").join("resources");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Go forge round-trip tree. \
         A failure here means a tests/forge/resources/*.scxml changed \
         without refreshing backends/go/forge-runtime/round_trip/generated/. \
         Run `backends/go/forge-runtime/round_trip/generate.sh` and commit \
         the result. stderr:\n{stderr}"
    );
}

/// The Mesh envelope codec the C++ runtime is built on is generated from the
/// standard library and committed (sce/src/mesh/generated/), so that a C++
/// consumer needs no Rust toolchain — which makes it one more tree that goes
/// stale when a standard document or a cpp template changes.
#[test]
fn verify_passes_on_real_committed_mesh_envelope_cpp_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("sce")
        .join("src")
        .join("mesh")
        .join("generated");
    let input_root = workspace.join("stdlib").join("mesh");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed C++ Mesh envelope tree. \
         A failure here means a stdlib/mesh/*.scxml or a cpp codec / enum \
         template changed without refreshing sce/src/mesh/generated/. \
         Run `sce/src/mesh/generate_envelope.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

/// The two peer tables the WebSocket loopback builds its routers from
/// (SCE_MESH.md §mesh-19) are generated from `tests/mesh/wss_loopback/`,
/// documents AND `deploy.yaml`: a table is a function of the deployment as
/// much as of the document, so the recompute is handed the deploy file too,
/// and an edit to either one without a regeneration is a red here.
#[test]
fn verify_passes_on_real_committed_mesh_peer_tables() {
    let workspace = workspace_root();
    let input_root = workspace.join("tests/mesh/wss_loopback");
    let deploy = input_root.join("deploy.yaml");
    for (target, script) in [
        (
            workspace.join("backends/rust/mesh/examples/generated"),
            "backends/rust/mesh/generate.sh",
        ),
        (
            workspace.join("backends/kotlin/mesh/src/jvmTest/kotlin/com/sce/generated/client"),
            "backends/kotlin/mesh/generate.sh",
        ),
        (
            workspace.join("backends/swift/mesh-wss/Tests/SceMeshWssTests/Generated"),
            "backends/swift/mesh-wss/generate.sh",
        ),
    ] {
        let result = Command::new(env_bin())
            .arg("verify")
            .arg(&target)
            .arg("--input-root")
            .arg(&input_root)
            .arg("--deploy")
            .arg(&deploy)
            .output()
            .expect("spawn sce-codegen");
        assert_eq!(
            result.status.code(),
            Some(0),
            "verify must pass on {}. A failure here means \
             tests/mesh/wss_loopback/ or a mesh_router template changed \
             without a regeneration. Run `{script}` and commit the result. \
             stderr:\n{}",
            target.display(),
            String::from_utf8_lossy(&result.stderr),
        );
    }
}

#[test]
fn verify_passes_on_real_committed_mesh_rust_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends")
        .join("rust")
        .join("mesh")
        .join("src")
        .join("generated");
    let input_root = workspace.join("stdlib").join("mesh");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust Mesh tree. A failure here \
         means a stdlib/mesh/*.scxml or a Rust forge template changed \
         without refreshing backends/rust/mesh/src/generated/. Run \
         `backends/rust/mesh/generate.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

/// The standard rate limiter (`sce:std/rate`) the Rust Mesh crate's HTTPS
/// edge judges its limits with (docs/adr/0109, D2), committed beside the Mesh
/// tree. Turns red when a `stdlib/rate/*.scxml` changes and the tree is not
/// regenerated.
#[test]
fn verify_passes_on_real_committed_mesh_rate_rust_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends")
        .join("rust")
        .join("mesh")
        .join("src")
        .join("generated_rate");
    let input_root = workspace.join("stdlib").join("rate");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust rate tree. A failure here \
         means a stdlib/rate/*.scxml or a Rust forge template changed \
         without refreshing backends/rust/mesh/src/generated_rate/. Run \
         `backends/rust/mesh/generate.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

/// The Rust runtime's standard engine documents, the one committed tree the
/// generator is itself compiled from (ADR 0097, amendment of 2026-11-17). Turns
/// red when a `stdlib/engine/*.scxml` changes and the tree is not regenerated.
#[test]
fn verify_passes_on_real_committed_runtime_rust_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends")
        .join("rust")
        .join("runtime")
        .join("generated");
    let input_root = workspace.join("stdlib").join("engine");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust runtime engine tree. A failure \
         here means a stdlib/engine/*.scxml changed without refreshing \
         backends/rust/runtime/generated/. Run \
         `scripts/regen_all_committed_trees.sh runtime-rust` and commit the \
         result. stderr:\n{stderr}"
    );
}

/// The other runtimes' standard engine documents, each a committed tree of
/// its runtime (ADR 0097, amendment of 2026-11-17). Turns red when a
/// `stdlib/engine/*.scxml` changes and one of them is not regenerated.
#[test]
fn verify_passes_on_real_committed_runtime_engine_trees() {
    let workspace = workspace_root();
    let input_root = workspace.join("stdlib").join("engine");
    for (step, tree) in [
        (
            "runtime-kotlin",
            "backends/kotlin/runtime/src/commonMain/kotlin/generated",
        ),
        ("runtime-go", "backends/go/runtime/generated"),
        (
            "runtime-python",
            "backends/python/runtime/sce_runtime/generated",
        ),
        (
            "runtime-swift",
            "backends/swift/runtime/Sources/SceRuntime/Generated",
        ),
        ("runtime-c", "backends/c/runtime/include/sce/generated"),
        // The C++ engines' tree is written by the step that emits the embed
        // manifest, which reads it.
        ("embed-manifest", "sce/include/generated"),
    ] {
        let (code, stderr) = run_verify_real_tree(&workspace.join(tree), &input_root);
        assert_eq!(
            code, 0,
            "verify must pass on the committed engine tree {tree}. A failure \
             here means a stdlib/engine/*.scxml changed without refreshing it. \
             Run `scripts/regen_all_committed_trees.sh {step}` and commit the \
             result. stderr:\n{stderr}"
        );
    }
}

#[test]
fn verify_passes_on_real_committed_mesh_kotlin_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends")
        .join("kotlin")
        .join("mesh")
        .join("src")
        .join("commonMain")
        .join("kotlin")
        .join("generated");
    let input_root = workspace.join("stdlib").join("mesh");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Kotlin Mesh tree. A failure here \
         means a stdlib/mesh/*.scxml or a Kotlin forge template changed \
         without refreshing backends/kotlin/mesh/src/commonMain/kotlin/generated/. \
         Run `backends/kotlin/mesh/generate.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

#[test]
fn verify_passes_on_real_committed_mesh_swift_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends")
        .join("swift")
        .join("mesh")
        .join("Sources")
        .join("SceMesh")
        .join("Generated");
    let input_root = workspace.join("stdlib").join("mesh");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Swift Mesh tree. A failure here \
         means a stdlib/mesh/*.scxml or a Swift forge template changed \
         without refreshing backends/swift/mesh/Sources/SceMesh/Generated/. \
         Run `backends/swift/mesh/generate.sh` and commit the result. \
         stderr:\n{stderr}"
    );
}

// ── §synth-6.2.6 source-set coverage guard ──────────────────────────
//
// The `source-hash` fold is total over whatever the walk collected, so a
// walk that collected nothing still yields a well-formed 64-hex digest —
// sha256 of the empty input. On the wire that is indistinguishable from a
// successful hash, which makes the header unauditable rather than merely
// wrong. These assert the emit-time refusal, and the one case that is
// deliberately allowed to pass through it.

/// Minimal statechart the `generate` subcommand will actually emit for.
const COVERAGE_FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="covered" initial="s0">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
"#;

fn run_generate(doc: &Path, out: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let result = Command::new(env_bin())
        .arg("generate")
        .arg(doc)
        .arg("-o")
        .arg(out)
        .arg("-l")
        .arg("rust")
        .args(extra)
        .output()
        .expect("sce-codegen must be runnable");
    (
        result.status.code(),
        String::from_utf8_lossy(&result.stderr).into_owned(),
    )
}

/// The header covers the document's closure, never the directory the run
/// names: an `--input-root` that holds nothing leaves the header the one a
/// run without it writes, and never the empty-input digest.
#[test]
fn generate_does_not_hash_the_input_root_it_is_given() {
    let root = TempDir::new().unwrap();
    let doc = root.path().join("covered.scxml");
    fs::write(&doc, COVERAGE_FIXTURE).unwrap();
    let empty = root.path().join("empty");
    let out = root.path().join("out");
    fs::create_dir_all(&empty).unwrap();
    fs::create_dir_all(&out).unwrap();

    let (code, stderr) = run_generate(&doc, &out, &["--input-root", empty.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{stderr}");
    let written = fs::read_to_string(out.join("covered_sm.rs")).unwrap();
    let expected = format!("// source-hash: {}", header_of(&doc).source_hex());
    assert_eq!(source_hash_line(&written), Some(expected.as_str()));
    assert!(
        !written.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        "the empty-input digest must never reach a header"
    );
}

/// Generating from a staged derivative under a declared `--input-root` is
/// allowed — this is what the fixture regen scripts do — even when the
/// derivative's bytes differ from the tracked document's: its header then
/// describes the bytes it was generated from.
#[test]
fn generate_allows_a_staged_derivative_under_a_declared_root() {
    let root = TempDir::new().unwrap();
    let tracked = root.path().join("tracked");
    let stage = root.path().join("stage");
    let out = root.path().join("out");
    fs::create_dir_all(&tracked).unwrap();
    fs::create_dir_all(&stage).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(tracked.join("covered.scxml"), COVERAGE_FIXTURE).unwrap();
    // The staged document differs from the tracked one — a synthesized
    // child, in the real workflow — so a content match would not save it.
    let staged = stage.join("covered.scxml");
    fs::write(&staged, COVERAGE_FIXTURE.replace("covered", "derived")).unwrap();

    let (code, stderr) = run_generate(&staged, &out, &["--input-root", tracked.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(0),
        "a declared root with a non-empty set must not be second-guessed: {stderr}"
    );
}

/// With the root inferred from the input's own location, the input has to
/// be in the set — if it is not, the walk lost it, which is exactly the
/// symlinked-sandbox failure this guard exists for.
#[test]
fn generate_hashes_a_symlinked_input_rather_than_the_empty_digest() {
    let real = TempDir::new().unwrap();
    let sandbox = TempDir::new().unwrap();
    let out = sandbox.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let target = real.path().join("covered.scxml");
    fs::write(&target, COVERAGE_FIXTURE).unwrap();
    // Build sandboxes materialise declared inputs as links into the real
    // tree; the emitted hash must match generating from the real file.
    let link = sandbox.path().join("covered.scxml");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let (code, stderr) = run_generate(&link, &out, &[]);
    assert_eq!(code, Some(0), "symlinked input must generate: {stderr}");

    let direct_out = real.path().join("out");
    fs::create_dir_all(&direct_out).unwrap();
    let (code, stderr) = run_generate(&target, &direct_out, &[]);
    assert_eq!(code, Some(0), "{stderr}");

    let via_link = fs::read_to_string(out.join("covered_sm.rs")).unwrap();
    let direct = fs::read_to_string(direct_out.join("covered_sm.rs")).unwrap();
    let hash_of = |s: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix("// source-hash: ").map(str::to_string))
            .expect("header carries a source-hash")
    };
    assert_eq!(
        hash_of(&via_link),
        hash_of(&direct),
        "a symlinked input must hash as the file it points at"
    );
    assert_ne!(
        hash_of(&via_link),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "the empty-input digest must never reach a header"
    );
}

/// Header keys a committed generated file must no longer carry — the
/// ones `forge::drift` retired (it says why).
const RETIRED_HEADER_KEYS: [&str; 2] = ["template-hash", "generated-at"];

/// Every committed generated file carries the header in its current shape.
///
/// A tree regenerated by a binary built before `template-hash` and
/// `generated-at` were retired — or a tree left out of the regeneration
/// that retired them — brings them back, and nothing else local notices:
/// `verify` reads only the `source-hash`, which such a file still carries
/// correctly. The `regen-reproduces` lane sees the bytes differ, in CI;
/// this sees it on every local run, without regenerating anything.
///
/// Every tracked file that opens with the banner is read, rather than a
/// list of roots. The list this replaced (it scanned for a pinned
/// `generated-at`) was short two of its five roots for months, and each
/// root nobody listed was output nobody watched.
#[test]
fn committed_generated_files_carry_the_current_header_shape() {
    let mut headered = 0usize;
    let mut retired: Vec<String> = Vec::new();
    for path in common::repository::files_git_tracks(&[]) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(keys) = header_keys(&content) else {
            continue;
        };
        headered += 1;
        if let Some(key) = keys.iter().find(|key| RETIRED_HEADER_KEYS.contains(key)) {
            retired.push(format!("{} carries `{key}`", path.display()));
        }
        // ADR 0094: a header names the documents its digest is a closure
        // of, or `verify` has nothing to recompute.
        if !keys.contains(&"source") {
            retired.push(format!("{} names no `source:` document", path.display()));
        }
    }

    // Every committed sourcemap sits beside the generated files it maps,
    // each of which carries a header, so there are at least as many
    // headered files as sourcemaps. Fewer means the banner stopped being
    // recognised and the scan above read nothing.
    let sourcemaps = common::repository::paths_git_tracks(&["*sce_sourcemap.json"]).len();
    assert!(
        sourcemaps > 0 && headered >= sourcemaps,
        "{headered} committed file(s) carry the §synth-6.2.6 banner, fewer than the \
         {sourcemaps} committed sourcemaps that map them — the scan is not reading headers"
    );
    assert!(
        retired.is_empty(),
        "{} of {headered} committed generated files carry a retired header key \
         or name no `source:` document. \
         Re-run `scripts/regen_all_committed_trees.sh` with a current sce-codegen \
         and commit the result. First few:\n{}",
        retired.len(),
        retired
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// The keys of `content`'s leading §synth-6.2.6 header, or `None` when it
/// does not open with the banner. The header is the banner line and the
/// `key: value` comment lines directly under it.
fn header_keys(content: &str) -> Option<Vec<&str>> {
    let mut lines = content.lines();
    if !lines.next()?.contains(HEADER_BANNER) {
        return None;
    }
    let keys = lines
        .map_while(|line| {
            let trimmed = line.trim_start();
            let body = trimmed
                .strip_prefix("// ")
                .or_else(|| trimmed.strip_prefix("# "))?;
            let (key, _) = body.split_once(": ")?;
            (key == "source-hash" || key == "source" || RETIRED_HEADER_KEYS.contains(&key))
                .then_some(key)
        })
        .collect();
    Some(keys)
}

/// Every document of a set contributes to the header of every file the set
/// writes (its compile is joint, ADR 0094 section 4), even when its
/// directory differs from the first document's, and with no `--input-root`.
#[test]
fn orchestrate_covers_every_document_of_its_set() {
    let root = TempDir::new().unwrap();
    let inputs = root.path().join("inputs");
    let left = inputs.join("left");
    let right = inputs.join("right");
    let out = root.path().join("out");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    let first = left.join("first.scxml");
    let second = right.join("second.scxml");
    fs::write(&first, COVERAGE_FIXTURE.replace("covered", "first")).unwrap();
    fs::write(&second, COVERAGE_FIXTURE.replace("covered", "second")).unwrap();
    let generated = Command::new(env_bin())
        .arg("orchestrate")
        .arg("--scxml")
        .arg(&first)
        .arg("--scxml")
        .arg(&second)
        .arg("-o")
        .arg(&out)
        .args(["-l", "rust", "--error-format=json"])
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let headed: Vec<String> = tree(&out)
        .into_iter()
        .filter(|(name, text)| name.ends_with(".rs") && text.starts_with("// SCE-GENERATED"))
        .map(|(_, text)| text)
        .collect();
    assert!(
        !headed.is_empty(),
        "orchestrate wrote no drift-headed Rust file"
    );
    for written in &headed {
        assert!(
            written.contains("// source: left/first.scxml\n// source: right/second.scxml\n"),
            "each document of the set is named, from the directory that holds both:\n{}",
            written.lines().take(5).collect::<Vec<_>>().join("\n")
        );
    }
    let (code, stderr) = run_verify_real_tree(&out, &inputs);
    assert_eq!(code, 0, "{stderr}");
    fs::write(
        &second,
        COVERAGE_FIXTURE.replace("covered", "second_changed"),
    )
    .unwrap();
    let (code, stderr) = run_verify_real_tree(&out, &inputs);
    assert_ne!(
        code, 0,
        "a change outside the first input's directory must invalidate output"
    );
    assert!(stderr.contains("source-hash mismatch"), "{stderr}");
}

/// The committed Mesh trees are generated from `stdlib/mesh`, whose
/// documents import each other by relative name and nothing from the rest
/// of the library by its `sce:std/...` name — so their source set is the
/// documents under that directory and no standard document more, and a
/// document added anywhere else in the library leaves their headers alone.
///
/// ⚠ Every Mesh document's header comment names it as `sce:std/mesh/…`, and
/// until 2026-09-28 that text alone folded the WHOLE library into the set:
/// each new standard document re-stamped all three Mesh trees.
#[test]
fn the_mesh_trees_hash_their_own_documents_and_no_other_part_of_the_library() {
    let root = workspace_root().join("stdlib/mesh");
    let on_disk = fs::read_dir(&root)
        .expect("stdlib/mesh")
        .filter(|entry| {
            entry
                .as_ref()
                .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "scxml"))
        })
        .count();
    assert!(on_disk > 0, "stdlib/mesh holds the Mesh documents");
    let set = sce_build::forge::drift::SourceSet::collect(&root, None).expect("collects");
    assert_eq!(
        set.len(),
        on_disk,
        "stdlib/mesh folds in a standard document it does not import"
    );
}
