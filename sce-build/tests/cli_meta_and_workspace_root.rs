// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Regression tests for sce-codegen CLI meta surface + workspace-root
// resolution. Pins three contract points downstream consumers
// depend on:
//
//   1. `--version` returns `sce-codegen <CARGO_PKG_VERSION>`. Standard
//      CLI convention; lets vendor pinners verify the binary matches
//      their pinned source.
//   2. `generate --help` does not parrot the stale
//      "C11 is RFC §synth-5-J-1 foundation only — emitter lands in M2+"
//      sentence. The c11 emitter is fully landed; the stale text
//      mis-led one consumer into a `hand-author header path` workaround
//      (vendor pin R30 → R53), so a regression here would re-open the
//      same mis-judgment for the next contributor.
//   3. `--workspace-root <PATH>` resolution chain: `--workspace-root
//      override → SCE_WORKSPACE_ROOT → CARGO_MANIFEST_DIR/.. → cwd-walk`,
//      with explicit warnings on validation failures along the way. It
//      decides which checkout `verify-generator` judges this binary
//      against when no `--root` is named, and vendored binaries (consumer
//      cwd is the consumer workspace; SCE source lives in `vendor/sce/`)
//      are why it does not start at the cwd: the legacy resolver only
//      walked upward from there and never found SCE.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
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
//
// Observed through `verify-generator` with no `--root`: it judges this
// binary against whichever checkout the chain resolves. This test binary
// was built from the sources of the checkout `sce_workspace_root()` names,
// so resolving THAT checkout is the one outcome that passes. Falling
// through to the unrelated working directory reports the binary
// unverifiable, and resolving some other checkout reports it stale — both
// non-zero, both named on stderr.

/// Run `sce-codegen <args>` from `cwd` with `env` applied (`None`
/// removes a variable), returning `(exit code, stderr)`.
fn run_codegen(
    cwd: &std::path::Path,
    args: &[&str],
    env: &[(&str, Option<&str>)],
) -> (Option<i32>, String) {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.current_dir(cwd)
        .args(args)
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
    let out = cmd.output().expect("spawn sce-codegen");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn workspace_root_explicit_flag_resolves_the_workspace() {
    let ws = sce_workspace_root();
    // cwd is unrelated to SCE; --workspace-root pins the real root.
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &["--workspace-root", ws.to_str().unwrap(), "verify-generator"],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert_eq!(
        code,
        Some(0),
        "an explicit --workspace-root must resolve the checkout this binary was built \
         from; stderr:\n{stderr}",
    );
}

#[test]
fn workspace_root_env_var_resolves_the_workspace() {
    let ws = sce_workspace_root();
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &["verify-generator"],
        &[("SCE_WORKSPACE_ROOT", Some(ws.to_str().unwrap()))],
    );
    assert_eq!(
        code,
        Some(0),
        "SCE_WORKSPACE_ROOT must resolve the checkout this binary was built from; \
         stderr:\n{stderr}",
    );
}

#[test]
fn workspace_root_invalid_explicit_flag_warns_and_falls_through() {
    let bogus = std::env::temp_dir().join("definitely-not-a-workspace-root-XYZ");
    // Don't actually create the dir — its absence is the point.
    let ws_real = sce_workspace_root();
    assert!(
        ws_real.exists(),
        "sce_workspace_root() must resolve to an existing dir for this test to be meaningful",
    );

    // We deliberately keep SCE_WORKSPACE_ROOT unset; resolution must
    // still find the real workspace via CARGO_MANIFEST_DIR/.. (this
    // test binary was built against the SCE workspace, so that layer
    // resolves), and the bogus explicit override must emit its warning
    // along the way.
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &[
            "--workspace-root",
            bogus.to_str().unwrap(),
            "verify-generator",
        ],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert!(
        stderr.contains("--workspace-root") && stderr.contains("does not contain"),
        "invalid --workspace-root must emit a validation warning; stderr:\n{stderr}",
    );
    assert_eq!(
        code,
        Some(0),
        "the fall-through (CARGO_MANIFEST_DIR/..) must still resolve the real workspace; \
         stderr:\n{stderr}",
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
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &["verify-generator"],
        &[("SCE_WORKSPACE_ROOT", None)],
    );
    assert_eq!(
        code,
        Some(0),
        "the CARGO_MANIFEST_DIR/.. fallback must resolve the real workspace; \
         stderr:\n{stderr}",
    );
}
