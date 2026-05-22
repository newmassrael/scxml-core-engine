// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! B9 §6.2.6 generated-source drift detection — end-to-end fixture.
//!
//! Pairs the library helper [`sce_build::apply_drift_headers_to_output`]
//! with the `sce-codegen verify` subcommand:
//!
//! 1. Compute hashes over a synthetic SCXML root + template tree.
//! 2. Apply headers to a hand-built [`generator::GeneratedOutput`].
//! 3. Write the headered output to a temp dir.
//! 4. Invoke `sce-codegen verify <out-dir>` as a subprocess.
//! 5. Assert: clean state passes; tampered source fails with
//!    `forge/source-hash-mismatch`.
//!
//! Why the helper-based fixture rather than full codegen: the helper
//! exposes the contract surface (per Q-§6.2.6-4 (b) "6-backend header
//! emit") without coupling to any one backend's generator pipeline. The
//! follow-up atomic wires `apply_drift_headers_to_output` into every
//! `cmd_*` codegen entry; that integration is out of scope for B9 per
//! the RFC's `[[feedback-design-preflight]]` one-atomic-one-scope
//! discipline.

use sce_build::forge::drift::{compute_source_hash, compute_template_hash, DriftHashes};
use sce_build::generator::GeneratedOutput;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct VerifyFixture {
    _root: TempDir,
    input_root: PathBuf,
    out_dir: PathBuf,
    template_root: PathBuf,
    cargo_lock: PathBuf,
}

impl VerifyFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let input_root = root.path().join("input");
        let out_dir = root.path().join("out");
        let template_root = root.path().join("templates");
        let cargo_lock = root.path().join("Cargo.lock");
        fs::create_dir_all(&input_root).unwrap();
        fs::create_dir_all(&out_dir).unwrap();
        fs::create_dir_all(&template_root).unwrap();
        fs::write(&cargo_lock, b"# synthetic lock file\n").unwrap();
        fs::write(input_root.join("foo.scxml"), b"<scxml/>").unwrap();
        fs::write(
            template_root.join("state_machine.jinja2"),
            b"sample template",
        )
        .unwrap();
        VerifyFixture {
            _root: root,
            input_root,
            out_dir,
            template_root,
            cargo_lock,
        }
    }

    /// Synthetic codegen — emits a one-file `GeneratedOutput` then
    /// invokes the library helper to prepend the §6.2.6 header. Body
    /// content is intentionally simple Rust so a future reader can
    /// recognise the test is purely about the header/verify contract.
    fn generate_headered_rust(&self, hashes: &DriftHashes, generated_at: u64) {
        let mut output = GeneratedOutput {
            files: vec![(
                "foo_sm.rs".to_string(),
                "pub struct Foo;\n\nimpl Foo {\n    pub fn new() -> Self { Foo }\n}\n".to_string(),
            )],
            ..Default::default()
        };
        sce_build::apply_drift_headers_to_output(&mut output, hashes, generated_at);
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

    /// Variant that pins the spawned binary's working directory.
    /// Avoids the parallel-test race where one test's
    /// `set_current_dir` leaks into another test's
    /// `locate_workspace_root` by mutating the shared process cwd.
    fn run_verify_from(&self, cwd: Option<&Path>) -> (i32, String, String) {
        let bin = env_bin();
        let mut cmd = Command::new(bin);
        cmd.arg("verify")
            .arg(self.out_dir.to_str().unwrap())
            .arg("--input-root")
            .arg(self.input_root.to_str().unwrap())
            .arg("--template-root")
            .arg(self.template_root.to_str().unwrap())
            .arg("--cargo-lock")
            .arg(self.cargo_lock.to_str().unwrap());
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

fn compute_hashes_for_fixture(fix: &VerifyFixture) -> DriftHashes {
    let source_hash = compute_source_hash(&fix.input_root, None).unwrap();
    let template_hash = compute_template_hash(&fix.template_root, &fix.cargo_lock).unwrap();
    DriftHashes {
        source_hash,
        template_hash,
    }
}

#[test]
fn verify_passes_on_clean_round_trip() {
    let fix = VerifyFixture::new();
    let hashes = compute_hashes_for_fixture(&fix);
    fix.generate_headered_rust(&hashes, 1715731200);
    let (code, _stdout, stderr) = fix.run_verify();
    assert_eq!(code, 0, "verify must pass on clean state. stderr: {stderr}");
}

#[test]
fn verify_fails_when_source_drifts() {
    let fix = VerifyFixture::new();
    let hashes = compute_hashes_for_fixture(&fix);
    fix.generate_headered_rust(&hashes, 1715731200);

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

#[test]
fn verify_fails_when_template_drifts() {
    let fix = VerifyFixture::new();
    let hashes = compute_hashes_for_fixture(&fix);
    fix.generate_headered_rust(&hashes, 1715731200);

    // Drift the template tree after generation.
    fs::write(
        fix.template_root.join("state_machine.jinja2"),
        b"sample template - drifted",
    )
    .unwrap();

    let (code, _stdout, stderr) = fix.run_verify();
    assert_ne!(
        code, 0,
        "verify must fail when template drifted. stderr: {stderr}"
    );
    assert!(
        stderr.contains("template-hash") || stderr.contains("forge/source-hash-mismatch"),
        "diagnostic must name the drift axis. stderr: {stderr}"
    );
}

#[test]
fn helper_emits_python_header_with_hash_prefix() {
    // 6-backend coverage check: helper picks `#` for `.py` and `//`
    // for everything else. This is the cross-backend invariant from
    // Q-§6.2.6-4 — single helper, prefix derived from file extension.
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
    let hashes = DriftHashes {
        source_hash: [0xaa; 32],
        template_hash: [0xbb; 32],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 0);

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
    let hashes = DriftHashes {
        source_hash: [0x11; 32],
        template_hash: [0x22; 32],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 100);
    let once = output.files[0].1.clone();
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 100);
    let twice = output.files[0].1.clone();
    assert_eq!(
        once, twice,
        "helper must be idempotent under repeat application"
    );
}

#[test]
fn verify_passes_when_run_from_different_working_directory() {
    // The verify CLI walks upward for workspace root only as a
    // *default*; we override template_root + cargo_lock explicitly, so
    // changing the working directory should not affect outcome. Pin
    // the spawned binary's cwd via `Command::current_dir` instead of
    // mutating the process-global cwd — `std::env::set_current_dir`
    // would race with other parallel tests that rely on the workspace
    // cwd to resolve `locate_workspace_root()`.
    let fix = VerifyFixture::new();
    let hashes = compute_hashes_for_fixture(&fix);
    fix.generate_headered_rust(&hashes, 0);
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
    let template_root = root.path().join("templates");
    let cargo_lock = root.path().join("Cargo.lock");
    fs::create_dir_all(&input_root).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&template_root).unwrap();
    fs::write(&cargo_lock, b"# lock\n").unwrap();
    fs::write(template_root.join("t.jinja2"), b"t").unwrap();

    let hashes = DriftHashes {
        source_hash: compute_source_hash(&input_root, None).unwrap(),
        template_hash: compute_template_hash(&template_root, &cargo_lock).unwrap(),
    };
    let mut output = GeneratedOutput {
        files: vec![("foo_sm.rs".into(), "pub struct Foo;\n".into())],
        ..Default::default()
    };
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 0);
    for (name, content) in output.files {
        fs::write(out_dir.join(name), content).unwrap();
    }
    let bin = env_bin();
    let result = Command::new(bin)
        .arg("verify")
        .arg(out_dir.to_str().unwrap())
        .arg("--input-root")
        .arg(input_root.to_str().unwrap())
        .arg("--template-root")
        .arg(template_root.to_str().unwrap())
        .arg("--cargo-lock")
        .arg(cargo_lock.to_str().unwrap())
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

    let canonical_hashes = DriftHashes {
        source_hash: compute_source_hash(&canonical, None).unwrap(),
        template_hash: [0u8; 32], // unused below; we only compare source axis
    };
    let stage_hashes = DriftHashes {
        source_hash: compute_source_hash(&stage, None).unwrap(),
        template_hash: [0u8; 32],
    };
    assert_ne!(
        canonical_hashes.source_hex(),
        stage_hashes.source_hex(),
        "test setup: canonical and stage dirs must produce different hashes",
    );

    let foo_sm = out_dir.join("foo_sm.rs");
    assert!(
        foo_sm.exists(),
        "generate must emit foo_sm.rs at {}",
        foo_sm.display()
    );
    let content = fs::read_to_string(&foo_sm).unwrap();
    let expected_line = format!("source-hash: {}", canonical_hashes.source_hex());
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
    let hashes = compute_hashes_for_fixture(&fix);
    assert_ne!(
        hashes.source_hash, [0u8; 32],
        "synthetic fixture must produce non-zero source-hash"
    );
    assert_ne!(
        hashes.template_hash, [0u8; 32],
        "synthetic fixture must produce non-zero template-hash"
    );
    assert!(fix.input_root.is_dir());
    assert!(fix.template_root.is_dir());
    assert!(fix.cargo_lock.is_file());
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

// Q-§6.2.6-3 lock-in: `template-hash` covers the entire
// `tools/codegen/templates/**` tree plus `Cargo.lock`. That means a
// template edit in any backend invalidates *every* committed
// generated tree's embedded hash — even backends whose own emit is
// byte-identical. The synthetic fixtures above (`verify_passes_…`,
// `verify_fails_when_…`) exercise the verify contract on hand-built
// inputs; they cannot catch the cross-backend invalidation that
// actually bit the donedata + clippy_two_allow chains
// (a3aae599 / df2fb95d / acd10fe6 / ae139888,
//  c2dfe502 / 1a5efb07 / dc228ab4 / 64f93cf4), because every CI lane
// (W3C `generate-w3c -l <lang>` fresh regen, Kotlin gradle's
// in-place `generateScxml`) bypasses the committed tree entirely.
//
// The W3C invariants below run the same `sce-codegen verify` the
// synthetic tests exercise, but against the actual repo state. They
// are the textbook enforcement of "any commit touching
// `tools/codegen/templates/**` or `Cargo.lock` must refresh both
// committed trees" — once they pass, the chain of per-backend
// template atomics cannot land while leaving any other backend's
// tree stale.
//
// Scope is W3C-only on purpose. The hand-authored
// `donedata_local_invoke` fixture (`sce-{rust,kotlin}-tests/.../fixtures/`)
// has a *distinct* drift context — its `source-hash` is computed
// against the fixture dir, not `resources/`. Mixing the two under
// one verify call would mis-attribute the donedata source-hash
// (95d74…) as a W3C drift; the regen-scripts that emit donedata
// (`scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`) are the
// existing gate for that context. Layout per backend:
//
//   Rust:   W3C SM under `sce-rust-tests/src/generated/`
//           donedata SM under `sce-rust-tests/src/integration/donedata_local_invoke/`
//           ⇒ verify `src/generated/` is donedata-free.
//
//   Kotlin: W3C SM and donedata SM are siblings under
//           `sce-kotlin-tests/src/main/kotlin/com/sce/generated/`
//           ⇒ verify the W3C *harness* tree
//           (`sce-kotlin-tests/src/test/kotlin/com/sce/w3c/`) which is
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
    let target = workspace.join("sce-rust-tests").join("src").join("generated");
    let input_root = workspace.join("resources");
    let (code, stderr) = run_verify_real_tree(&target, &input_root);
    assert_eq!(
        code, 0,
        "verify must pass on the committed Rust W3C generated tree. \
         A failure here means tools/codegen/templates/** or Cargo.lock \
         changed without refreshing sce-rust-tests/src/generated/. \
         Run `cargo build --bin sce-codegen --features cli --release \
         -p sce-build` then `target/release/sce-codegen generate-w3c \
         -l rust` and commit the result. stderr:\n{stderr}"
    );
}

#[test]
fn verify_passes_on_real_committed_kotlin_w3c_tree() {
    let workspace = workspace_root();
    let target = workspace
        .join("sce-kotlin-tests")
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
         A failure here means tools/codegen/templates/** or Cargo.lock \
         changed without refreshing the committed Kotlin generated \
         tree. Run `cargo build --bin sce-codegen --features cli \
         --release -p sce-build` then `target/release/sce-codegen \
         generate-w3c -l kotlin` and commit the result. stderr:\n{stderr}"
    );
}
