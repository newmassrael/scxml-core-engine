// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use sce_build::forge::drift::{compute_source_hash, DriftHeader, HEADER_BANNER};
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

fn header_for_fixture(fix: &VerifyFixture) -> DriftHeader {
    DriftHeader {
        source_hash: compute_source_hash(&fix.input_root, None).unwrap(),
    }
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

/// A tree generated before the header dropped `template-hash` and
/// `generated-at` still carries them. `verify` judges the `source-hash`
/// such a file carries like any other; the retired lines are for a
/// regeneration to remove, not for `verify` to refuse.
#[test]
fn verify_reads_a_header_of_the_older_shape() {
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
    assert_eq!(
        code, 0,
        "a legacy header with a current source-hash must pass. stderr: {stderr}"
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

#[test]
fn verify_passes_when_input_set_is_empty_scxml_directory() {
    // Edge case: no `.scxml` files under input_root. compute_source_hash
    // walks an empty tree but should produce a stable hash (just the
    // BTreeMap-with-zero-entries digest). The synthetic GeneratedOutput
    // sees that hash; verify confirms no drift.
    let root = TempDir::new().unwrap();
    let input_root = root.path().join("empty-input");
    let out_dir = root.path().join("out");
    fs::create_dir_all(&input_root).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    let header = DriftHeader {
        source_hash: compute_source_hash(&input_root, None).unwrap(),
    };
    let mut output = GeneratedOutput {
        files: vec![("foo_sm.rs".into(), "pub struct Foo;\n".into())],
        ..Default::default()
    };
    sce_build::apply_drift_headers_to_output(&mut output, &header);
    for (name, content) in output.files {
        fs::write(out_dir.join(name), content).unwrap();
    }
    let bin = env_bin();
    let result = Command::new(bin)
        .arg("verify")
        .arg(out_dir.to_str().unwrap())
        .arg("--input-root")
        .arg(input_root.to_str().unwrap())
        .output()
        .unwrap();
    assert_eq!(
        result.status.code().unwrap_or(-1),
        0,
        "empty-input verify must pass. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn input_root_override_pins_hash_to_canonical_location() {
    // Round B donedata symmetry follow-up: when an authoring script
    // stages its tracked input into a tmp dir (so split-out children
    // don't pollute the source tree), the `--input-root` override
    // lets the cmd_generate hash root point back at the canonical
    // location. The embedded source-hash must therefore match
    // `compute_source_hash(canonical)` — not the staging dir — so a
    // stranger running `sce-codegen verify <out> --input-root <canonical>`
    // reproduces the hash directly from the repo.
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
    // Add a second SCXML to canonical only so the two dirs produce
    // different recursive hashes — proves `--input-root` actually
    // routes through compute_source_hash and is not silently
    // discarded.
    fs::write(
        canonical.join("bar.scxml"),
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

    let canonical_header = DriftHeader {
        source_hash: compute_source_hash(&canonical, None).unwrap(),
    };
    let stage_header = DriftHeader {
        source_hash: compute_source_hash(&stage, None).unwrap(),
    };
    assert_ne!(
        canonical_header.source_hex(),
        stage_header.source_hex(),
        "test setup: canonical and stage dirs must produce different hashes",
    );

    let foo_sm = out_dir.join("foo_sm.rs");
    assert!(
        foo_sm.exists(),
        "generate must emit foo_sm.rs at {}",
        foo_sm.display()
    );
    let content = fs::read_to_string(&foo_sm).unwrap();
    let expected_line = format!("source-hash: {}", canonical_header.source_hex());
    assert!(
        content.contains(&expected_line),
        "generated file must embed canonical source-hash. expected line: `{expected_line}`\nfirst 5 lines of {}:\n{}",
        foo_sm.display(),
        content.lines().take(5).collect::<Vec<_>>().join("\n"),
    );

    // Verify against canonical should pass.
    let result = Command::new(bin)
        .arg("verify")
        .arg(out_dir.to_str().unwrap())
        .arg("--input-root")
        .arg(canonical.to_str().unwrap())
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "verify against canonical must pass. stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );

    // Verify against stage should fail — different file set than
    // the embedded hash was computed against.
    let result = Command::new(bin)
        .arg("verify")
        .arg(out_dir.to_str().unwrap())
        .arg("--input-root")
        .arg(stage.to_str().unwrap())
        .output()
        .unwrap();
    assert_ne!(
        result.status.code(),
        Some(0),
        "verify against stage must fail (different file set than canonical). stderr: {}",
        String::from_utf8_lossy(&result.stderr),
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
//   Kotlin: W3C SM and donedata SM are siblings under
//           `backends/kotlin/tests/src/main/kotlin/com/sce/generated/`
//           ⇒ verify the W3C *harness* tree
//           (`backends/kotlin/tests/src/test/kotlin/com/sce/w3c/`) which is
//           donedata-free and shares the W3C drift context with the
//           SM tree (one `generate-w3c -l kotlin` invocation emits
//           both atomically, so harness-fresh ⇒ SM-fresh).

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

#[test]
fn verify_passes_on_real_committed_kotlin_w3c_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("backends/kotlin/tests")
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
// distinct top-level dirs. Regen via
// `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`.
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
         Run `scripts/regen_donedata_local_invoke.sh` and commit the \
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
         `scripts/regen_donedata_local_invoke_kotlin.sh` and commit the \
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
         Run `scripts/regen_donedata_local_invoke_go.sh` and commit the \
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

/// An input root that resolves to nothing must be refused rather than
/// embedding the empty-input digest.
#[test]
fn generate_refuses_when_the_source_set_is_empty() {
    let root = TempDir::new().unwrap();
    let doc = root.path().join("covered.scxml");
    fs::write(&doc, COVERAGE_FIXTURE).unwrap();
    let empty = root.path().join("empty");
    let out = root.path().join("out");
    fs::create_dir_all(&empty).unwrap();
    fs::create_dir_all(&out).unwrap();

    let (code, stderr) = run_generate(
        &doc,
        &out,
        &[
            "--input-root",
            empty.to_str().unwrap(),
            "--error-format=json",
        ],
    );
    assert_ne!(code, Some(0), "empty source set must not emit");
    assert!(
        stderr.contains("forge/source-hash-input-uncovered"),
        "refusal must carry the contract code, got: {stderr}"
    );
    assert!(
        !out.join("covered_sm.rs").exists(),
        "nothing may be written when the header cannot be trusted"
    );
}

/// A declared `--input-root` that does collect sources is an assertion
/// about where the sources live, so generating from a staged derivative of
/// them is allowed — this is what the fixture regen scripts do, and the
/// empty-set floor above still applies to them.
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
        "{} of {headered} committed generated files carry a retired header key. \
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
            (key == "source-hash" || RETIRED_HEADER_KEYS.contains(&key)).then_some(key)
        })
        .collect();
    Some(keys)
}

/// Every document in a batch must contribute to the drift hash, even
/// when its directory differs from the first document's directory.
#[test]
fn orchestrate_requires_a_root_covering_the_entire_document_set() {
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
    let run = |explicit: bool| {
        let mut cmd = Command::new(env_bin());
        cmd.arg("orchestrate")
            .arg("--scxml")
            .arg(&first)
            .arg("--scxml")
            .arg(&second)
            .arg("-o")
            .arg(&out)
            .args(["-l", "rust", "--error-format=json"]);
        if explicit {
            cmd.arg("--input-root").arg(&inputs);
        }
        cmd.output().unwrap()
    };
    let refused = run(false);
    assert!(
        !refused.status.success(),
        "an inferred root must not omit the second input"
    );
    let diagnostic = String::from_utf8_lossy(&refused.stderr);
    assert!(
        diagnostic.contains("forge/source-hash-input-uncovered"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("second.scxml"), "{diagnostic}");
    let generated = run(true);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
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

/// Coverage is a question about the file, not its bytes. The second input
/// sits outside the inferred root but matches, byte for byte, a document
/// that does sit inside it. Judged by content it passed, and every later
/// edit to it would have left the `source-hash` unmoved.
#[test]
fn orchestrate_refuses_an_outside_input_that_duplicates_a_covered_file() {
    let root = TempDir::new().unwrap();
    let left = root.path().join("left");
    let right = root.path().join("right");
    let out = root.path().join("out");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    let first = left.join("first.scxml");
    let second = right.join("second.scxml");
    let twin = COVERAGE_FIXTURE.replace("covered", "second");
    fs::write(&first, COVERAGE_FIXTURE.replace("covered", "first")).unwrap();
    fs::write(&second, &twin).unwrap();
    // Inside the inferred root, not an input: the same bytes as `second`.
    fs::write(left.join("twin.scxml"), &twin).unwrap();

    let refused = Command::new(env_bin())
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
        !refused.status.success(),
        "a byte-identical file inside the root does not make the outside input covered"
    );
    let diagnostic = String::from_utf8_lossy(&refused.stderr);
    assert!(
        diagnostic.contains("forge/source-hash-input-uncovered"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("second.scxml"), "{diagnostic}");
}
