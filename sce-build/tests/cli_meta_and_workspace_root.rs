// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//      override → SCE_WORKSPACE_ROOT → the tree the run works in`, with
//      explicit warnings on validation failures along the way. It decides
//      which checkout `verify-generator` judges this binary against when
//      no `--root` is named. The last layer climbs from the working
//      directory, from the package Cargo runs the binary for, and from the
//      binary's own directory, and never reads where the binary was
//      compiled: a compiled-in path is the build machine's and made two
//      builds of one source differ by their checkout. Vendored binaries
//      (consumer cwd is the consumer workspace; SCE source lives in
//      `vendor/sce/`) are why the package Cargo runs is one of its starts.

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
    // still find the real workspace through the package Cargo runs the
    // binary for (named here as `cargo test` names it), and the bogus
    // explicit override must emit its warning along the way.
    let package = ws_real.join("sce-build");
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &[
            "--workspace-root",
            bogus.to_str().unwrap(),
            "verify-generator",
        ],
        &[
            ("SCE_WORKSPACE_ROOT", None),
            ("CARGO_MANIFEST_DIR", Some(package.to_str().unwrap())),
        ],
    );
    assert!(
        stderr.contains("--workspace-root") && stderr.contains("does not contain"),
        "invalid --workspace-root must emit a validation warning; stderr:\n{stderr}",
    );
    assert_eq!(
        code,
        Some(0),
        "the fall-through (the search for the tree the run works in) must still resolve \
         the real workspace; stderr:\n{stderr}",
    );
}

#[test]
fn workspace_root_is_found_from_the_package_cargo_runs() {
    // The vendored-binary scenario: consumer cwd is somewhere entirely
    // unrelated to SCE, no explicit --workspace-root / SCE_WORKSPACE_ROOT
    // is provided, and Cargo runs the binary for the SCE package
    // (`cargo run -p sce-build` from the consumer workspace). The package
    // Cargo names is what must save the day; the legacy "walk up from cwd"
    // resolver missed vendor-pinned consumers.
    let package = sce_workspace_root().join("sce-build");
    let (code, stderr) = run_codegen(
        &std::env::temp_dir(),
        &["verify-generator"],
        &[
            ("SCE_WORKSPACE_ROOT", None),
            ("CARGO_MANIFEST_DIR", Some(package.to_str().unwrap())),
        ],
    );
    assert_eq!(
        code,
        Some(0),
        "the package Cargo runs the binary for must resolve the real workspace; \
         stderr:\n{stderr}",
    );
}

// ── #4: the batch commands' project root, as a generation names it ──
//
// A cached generation (ADR 0097) is run from the checkout's root with
// `SCE_WORKSPACE_ROOT=.`, so that its key names no checkout's location. The
// batch command (`generate-w3c`) must read that
// value against the directory it runs in: falling through to the checkout
// that built the binary reads that checkout's registry, which is outside
// the generation's checkout and outside its key.

/// A checkout holding one fixture no other checkout registers, `9999`: the
/// conformance registry the batch commands find their root by, and the
/// fixture's document.
fn checkout_of_one_fixture(root: &std::path::Path) {
    let registry = root.join("tests/w3c/conformance/fixtures.json");
    std::fs::create_dir_all(registry.parent().unwrap()).expect("the registry's directory");
    std::fs::write(
        &registry,
        r#"{
  "version": 1,
  "harnesses": { "simple": "Run the machine to completion." },
  "fixtures": [
    { "id": "9999", "summary": "a fixture only this checkout registers" }
  ]
}
"#,
    )
    .expect("write the registry");
    let document = root.join("resources/9999/test9999.scxml");
    std::fs::create_dir_all(document.parent().unwrap()).expect("the fixture's directory");
    std::fs::write(
        &document,
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"pass\">\n\
         <final id=\"pass\"/>\n\
         </scxml>\n",
    )
    .expect("write the fixture");
}

/// `generate-w3c -l rust --list` run by `binary` from `cwd`, with `env`
/// applied over an environment that names no root (`None` removes a
/// variable); the templates are named, so that the run reads the one tree
/// this test asks about and no other. Returns (succeeded, stdout, stderr).
fn list_fixtures(
    binary: &std::path::Path,
    cwd: &std::path::Path,
    env: &[(&str, Option<&str>)],
) -> (bool, String, String) {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd)
        .args(["generate-w3c", "-l", "rust", "--list"])
        .env(
            "SCE_TEMPLATE_DIR",
            sce_workspace_root().join("tools/codegen/templates"),
        )
        .env_remove("SCE_WORKSPACE_ROOT")
        .env_remove("CARGO_MANIFEST_DIR")
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
    let out = cmd.output().expect("spawn sce-codegen generate-w3c --list");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A working directory no checkout is above: the root of the file system.
/// Not a temporary directory, which a machine may keep inside a checkout,
/// where the climb from the working directory would find that checkout first.
fn unrelated_directory() -> &'static std::path::Path {
    std::path::Path::new("/")
}

/// Whether a listing is the one of [`checkout_of_one_fixture`].
fn lists_the_one_fixture(stdout: &str) -> bool {
    stdout.contains("W3C conformance registry: 1 fixtures") && stdout.contains("9999")
}

#[test]
fn a_relative_workspace_root_is_the_directory_the_run_starts_in() {
    let checkout = tempfile::tempdir().expect("a checkout");
    checkout_of_one_fixture(checkout.path());
    let (ok, stdout, stderr) = list_fixtures(
        &sce_codegen_bin(),
        checkout.path(),
        &[("SCE_WORKSPACE_ROOT", Some("."))],
    );
    assert!(
        ok,
        "generate-w3c --list failed in a checkout of one fixture; stderr:\n{stderr}",
    );
    assert!(
        lists_the_one_fixture(&stdout),
        "SCE_WORKSPACE_ROOT=. must name the checkout the run starts in, not the one that \
         built this binary; stdout:\n{stdout}\nstderr:\n{stderr}",
    );
}

// ── #5: with no root named, the tree the run works in ──────────────
//
// The search climbs from the working directory, then from the package Cargo
// runs the binary for, then from the binary's own directory, and never reads
// where the binary was compiled. Each case below puts the checkout of one
// fixture where only one start reaches it, while this binary's own checkout
// stays reachable from its directory: a resolver that took the checkout the
// binary was built in, or took the starts in another order, lists that
// checkout's whole registry instead.

#[test]
fn with_no_root_named_a_run_inside_a_checkout_works_in_it() {
    let checkout = tempfile::tempdir().expect("a checkout");
    checkout_of_one_fixture(checkout.path());
    let inside = checkout.path().join("resources/9999");
    let (ok, stdout, stderr) = list_fixtures(&sce_codegen_bin(), &inside, &[]);
    assert!(
        ok && lists_the_one_fixture(&stdout),
        "a run started inside a checkout must work in that checkout, found by climbing from \
         the working directory; stdout:\n{stdout}\nstderr:\n{stderr}",
    );
}

#[test]
fn with_no_root_named_a_run_works_in_the_checkout_of_the_package_cargo_runs() {
    let consumer = tempfile::tempdir().expect("a consumer workspace");
    let vendored = consumer.path().join("vendor/sce");
    checkout_of_one_fixture(&vendored);
    let package = vendored.join("sce-build");
    std::fs::create_dir_all(&package).expect("the vendored package");
    let (ok, stdout, stderr) = list_fixtures(
        &sce_codegen_bin(),
        unrelated_directory(),
        &[("CARGO_MANIFEST_DIR", Some(package.to_str().unwrap()))],
    );
    assert!(
        ok && lists_the_one_fixture(&stdout),
        "a run Cargo starts for a vendored SCE package must work in the vendored checkout; \
         stdout:\n{stdout}\nstderr:\n{stderr}",
    );
}

#[test]
fn with_no_root_named_a_binary_placed_in_a_checkout_works_in_it() {
    // In the target directory, so that the binary can be linked rather than
    // copied: one file system, and the checkout the binary sits in is this
    // one, not the one it was built in.
    let checkout = tempfile::Builder::new()
        .prefix("placed-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("a checkout");
    checkout_of_one_fixture(checkout.path());
    let placed = checkout.path().join(".bx/artifacts/codegen");
    std::fs::create_dir_all(&placed).expect("the place of the output");
    let binary = placed.join("sce-codegen");
    if std::fs::hard_link(sce_codegen_bin(), &binary).is_err() {
        std::fs::copy(sce_codegen_bin(), &binary).expect("place the binary");
    }
    let (ok, stdout, stderr) = list_fixtures(&binary, unrelated_directory(), &[]);
    assert!(
        ok && lists_the_one_fixture(&stdout),
        "a binary placed inside a checkout, run from elsewhere with nothing named, must work \
         in the checkout it sits in; stdout:\n{stdout}\nstderr:\n{stderr}",
    );
}
