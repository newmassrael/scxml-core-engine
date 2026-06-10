// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Regression tests for sce-codegen CLI meta surface + workspace-root
// resolution. Pins three contract points downstream consumers
// (watching-zenoh and beyond) depend on:
//
//   1. `--version` returns `sce-codegen <CARGO_PKG_VERSION>`. Standard
//      CLI convention; lets vendor pinners verify the binary matches
//      their pinned source.
//   2. `generate --help` does not parrot the stale
//      "C11 is RFC §5.J.1 foundation only — emitter lands in M2+"
//      sentence. The c11 emitter is fully landed; the stale text
//      mis-led one consumer into a `hand-author header path` workaround
//      (vendor pin R30 → R53), so a regression here would re-open the
//      same mis-judgment for the next contributor.
//   3. `--workspace-root <PATH>` resolution chain. Vendored binaries
//      (consumer cwd is the consumer workspace; SCE source lives in
//      `vendor/sce/`) used to embed a zero `template-hash` in every
//      generated file because the legacy resolver only walked upward
//      from cwd and never found SCE. The new chain is `--workspace-root
//      override → SCE_WORKSPACE_ROOT → CARGO_MANIFEST_DIR/.. → cwd-walk`
//      with explicit warnings on validation failures along the way.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Minimal well-formed SCXML that drives the codegen pipeline (and the
/// drift-header emission) without depending on forge-kind specifics.
/// Two states with one event so the rust template emits a non-trivial
/// machine — enough to exercise template-hash embedding.
fn write_minimal_scxml(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("m.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="m" version="1.0" initial="s0" datamodel="null">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
"#;
    std::fs::write(&path, body).expect("write scxml");
    path
}

/// Resolve the real SCE workspace root from the test's compile-time
/// context. `CARGO_MANIFEST_DIR` points at `sce-build/`; its parent is
/// the workspace root that carries `tools/codegen/templates/`. Tests
/// reach back to it via this path so they don't bake the cwd-state
/// of `cargo test` into the assertion.
fn sce_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent")
        .to_path_buf()
}

// ── #2: --version surface ──────────────────────────────────────────

#[test]
fn version_flag_prints_package_version() {
    let out = Command::new(sce_codegen_bin())
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen --version");
    assert!(
        out.status.success(),
        "sce-codegen --version must exit 0; got status {} stderr {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let expected_version = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(expected_version),
        "stdout must contain the package version {expected_version}; got {stdout:?}",
    );
    assert!(
        stdout.starts_with("sce-codegen "),
        "stdout must start with the binary name; got {stdout:?}",
    );
}

// ── #1: --language help text no longer parrots the M2+ stale line ──

#[test]
fn generate_help_does_not_carry_m2_plus_stale_text() {
    let out = Command::new(sce_codegen_bin())
        .args(["generate", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen generate --help");
    assert!(
        out.status.success(),
        "sce-codegen generate --help must exit 0; got status {}",
        out.status,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The exact stale phrasing from the R30 vendor pin. A future
    // re-introduction must trip here — not in a downstream consumer's
    // mis-judgment loop.
    assert!(
        !stdout.contains("emitter lands in M2"),
        "generate --help must not parrot the 'emitter lands in M2+' \
         line — c11 is a first-class emit target; \
         got help text:\n{stdout}",
    );
    assert!(
        !stdout.contains("RFC §5.J.1 foundation only"),
        "generate --help must not call c11 a foundation-only stub; \
         got help text:\n{stdout}",
    );
}

#[test]
fn generate_w3c_help_does_not_carry_m2_plus_stale_text() {
    let out = Command::new(sce_codegen_bin())
        .args(["generate-w3c", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen generate-w3c --help");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("emitter lands in M2") && !stdout.contains("RFC §5.J.1 foundation only"),
        "generate-w3c --help still carries the stale c11 status sentence:\n{stdout}",
    );
}

// ── #3: --workspace-root + SCE_WORKSPACE_ROOT resolution chain ─────

/// Helper: run `sce-codegen generate` with a controlled cwd / env /
/// flags, capture stderr, and return the produced `rust_sm.rs` body
/// for hash inspection. Returns `(stderr, body)`.
fn run_generate(
    cwd: &std::path::Path,
    args: &[&str],
    env: &[(&str, Option<&str>)],
) -> (String, String) {
    let scratch = ScratchDir::new("ws-root-out");
    let mut full_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    // Append the standard `-o <out_dir>` so each invocation writes to
    // its own dir.
    full_args.push("-o".into());
    full_args.push(scratch.path().display().to_string());

    let mut cmd = Command::new(sce_codegen_bin());
    cmd.current_dir(cwd)
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        match v {
            Some(val) => {
                cmd.env(k, val);
            }
            None => {
                cmd.env_remove(k);
            }
        }
    }
    let out = cmd.output().expect("spawn sce-codegen generate");
    assert!(
        out.status.success(),
        "sce-codegen generate must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    // The Rust template emits `<basename>_sm.rs`.
    let body =
        std::fs::read_to_string(scratch.path().join("m_sm.rs")).expect("read generated m_sm.rs");
    (stderr, body)
}

const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn workspace_root_explicit_flag_yields_real_template_hash() {
    let scxml_dir = ScratchDir::new("ws-explicit-src");
    let scxml_path = write_minimal_scxml(scxml_dir.path());
    let ws = sce_workspace_root();
    let unrelated_cwd = std::env::temp_dir();

    // cwd is unrelated to SCE; --workspace-root pins the real root.
    let (stderr, body) = run_generate(
        &unrelated_cwd,
        &[
            "--workspace-root",
            ws.to_str().unwrap(),
            "generate",
            scxml_path.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert!(
        !stderr.contains("workspace root not detected"),
        "explicit --workspace-root must suppress the zero-hash warning; stderr:\n{stderr}",
    );
    let hash_line = body
        .lines()
        .find(|l| l.contains("template-hash:"))
        .expect("template-hash line in generated body");
    assert!(
        !hash_line.contains(ZERO_HASH),
        "template-hash must not be zero when --workspace-root pins a valid root; got: {hash_line}",
    );
}

#[test]
fn workspace_root_env_var_yields_real_template_hash() {
    let scxml_dir = ScratchDir::new("ws-env-src");
    let scxml_path = write_minimal_scxml(scxml_dir.path());
    let ws = sce_workspace_root();
    let unrelated_cwd = std::env::temp_dir();

    let (stderr, body) = run_generate(
        &unrelated_cwd,
        &["generate", scxml_path.to_str().unwrap(), "-l", "rust"],
        &[("SCE_WORKSPACE_ROOT", Some(ws.to_str().unwrap()))],
    );
    assert!(
        !stderr.contains("workspace root not detected"),
        "SCE_WORKSPACE_ROOT env must suppress the zero-hash warning; stderr:\n{stderr}",
    );
    let hash_line = body
        .lines()
        .find(|l| l.contains("template-hash:"))
        .expect("template-hash line in generated body");
    assert!(
        !hash_line.contains(ZERO_HASH),
        "template-hash must not be zero with SCE_WORKSPACE_ROOT set; got: {hash_line}",
    );
}

#[test]
fn workspace_root_invalid_explicit_flag_warns_and_falls_through() {
    let scxml_dir = ScratchDir::new("ws-invalid-src");
    let scxml_path = write_minimal_scxml(scxml_dir.path());
    let bogus = std::env::temp_dir().join("definitely-not-a-workspace-root-XYZ");
    // Don't actually create the dir — its absence is the point.
    let unrelated_cwd = std::env::temp_dir();
    let ws_real = sce_workspace_root();

    // We deliberately keep SCE_WORKSPACE_ROOT unset; resolution must
    // still find the real workspace via CARGO_MANIFEST_DIR/.. (this
    // test binary was built against the SCE workspace, so that layer
    // resolves), and the bogus explicit override must emit its warning
    // along the way.
    let (stderr, body) = run_generate(
        &unrelated_cwd,
        &[
            "--workspace-root",
            bogus.to_str().unwrap(),
            "generate",
            scxml_path.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert!(
        stderr.contains("--workspace-root") && stderr.contains("does not contain"),
        "invalid --workspace-root must emit a validation warning; stderr:\n{stderr}",
    );
    // The fall-through (CARGO_MANIFEST_DIR/..) must still resolve to
    // the real workspace, so the embedded hash is real — not zero.
    let hash_line = body
        .lines()
        .find(|l| l.contains("template-hash:"))
        .expect("template-hash line");
    assert!(
        !hash_line.contains(ZERO_HASH),
        "fallback resolution (CARGO_MANIFEST_DIR/..) must still produce a real hash; got: {hash_line}",
    );
    // Sanity: the fallback target is the workspace this test binary
    // was built against.
    assert!(
        ws_real.exists(),
        "sce_workspace_root() must resolve to an existing dir for this test to be meaningful",
    );
}

#[test]
fn workspace_root_compile_time_fallback_resolves_for_vendored_layout() {
    // Simulate the vendored-binary scenario: consumer cwd is somewhere
    // entirely unrelated to SCE, and no explicit --workspace-root /
    // SCE_WORKSPACE_ROOT is provided. The `CARGO_MANIFEST_DIR/..`
    // fallback (baked at build time) is what must save the day —
    // this is the layer the legacy "walk up from cwd" resolver
    // missed for vendor-pinned consumers.
    let scxml_dir = ScratchDir::new("ws-fallback-src");
    let scxml_path = write_minimal_scxml(scxml_dir.path());
    let unrelated_cwd = std::env::temp_dir();

    let (stderr, body) = run_generate(
        &unrelated_cwd,
        &["generate", scxml_path.to_str().unwrap(), "-l", "rust"],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert!(
        !stderr.contains("workspace root not detected"),
        "CARGO_MANIFEST_DIR/.. fallback must keep the zero-hash warning silent; stderr:\n{stderr}",
    );
    let hash_line = body
        .lines()
        .find(|l| l.contains("template-hash:"))
        .expect("template-hash line");
    assert!(
        !hash_line.contains(ZERO_HASH),
        "CARGO_MANIFEST_DIR/.. fallback must produce a real template-hash; got: {hash_line}",
    );
}
