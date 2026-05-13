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
        fs::write(template_root.join("state_machine.jinja2"), b"sample template").unwrap();
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
        let bin = env_bin();
        let output = Command::new(bin)
            .arg("verify")
            .arg(self.out_dir.to_str().unwrap())
            .arg("--input-root")
            .arg(self.input_root.to_str().unwrap())
            .arg("--template-root")
            .arg(self.template_root.to_str().unwrap())
            .arg("--cargo-lock")
            .arg(self.cargo_lock.to_str().unwrap())
            .output()
            .expect("spawn sce-codegen");
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
    assert_ne!(code, 0, "verify must fail when source drifted. stderr: {stderr}");
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
    assert_ne!(code, 0, "verify must fail when template drifted. stderr: {stderr}");
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
    };
    let hashes = DriftHashes {
        source_hash: [0xaa; 32],
        template_hash: [0xbb; 32],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 0);

    for (filename, content) in &output.files {
        let prefix_expected = if filename.ends_with(".py") { "# " } else { "// " };
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
    };
    let hashes = DriftHashes {
        source_hash: [0x11; 32],
        template_hash: [0x22; 32],
    };
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 100);
    let once = output.files[0].1.clone();
    sce_build::apply_drift_headers_to_output(&mut output, &hashes, 100);
    let twice = output.files[0].1.clone();
    assert_eq!(once, twice, "helper must be idempotent under repeat application");
}

#[test]
fn verify_passes_when_run_from_different_working_directory() {
    // The verify CLI walks upward for workspace root only as a
    // *default*; we override template_root + cargo_lock explicitly, so
    // changing the working directory should not affect outcome.
    let fix = VerifyFixture::new();
    let hashes = compute_hashes_for_fixture(&fix);
    fix.generate_headered_rust(&hashes, 0);
    let orig_cwd = std::env::current_dir().unwrap();
    // Switch to a clean directory inside the temp root and re-run.
    let alt_cwd = fix._root.path().join("alt-cwd");
    fs::create_dir_all(&alt_cwd).unwrap();
    std::env::set_current_dir(&alt_cwd).unwrap();
    let (code, _stdout, stderr) = fix.run_verify();
    std::env::set_current_dir(&orig_cwd).unwrap();
    assert_eq!(code, 0, "verify with absolute paths must be cwd-independent. stderr: {stderr}");
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
