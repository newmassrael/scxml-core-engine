// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What `compile_scxml` promises a `build.rs` consumer.
//
// The function writes to `OUT_DIR`, and the Cargo convention for an
// `OUT_DIR` artifact is `include!`. The machine it writes is a module
// *file*: it opens with an audited suppression budget spelled as inner
// attributes and an inner doc comment, both of which rustc refuses in
// `include!`'s expansion position. Nothing in this repository consumed
// the function, so nothing here ever met that — the only consumers are
// downstream, and two independent ones (`pinion-core/build.rs`,
// `sprag-plugin/build.rs`) reached the same workaround byte-for-byte:
// filter out every line starting `#![` or `//!`, then blanket the
// module with `#![allow(warnings, clippy::all, …)]` of their own.
//
// What that costs is the thing SCE built on purpose. The budget's own
// header says each allow "corresponds to a lint that actually fires on
// generated code today" and that adding one "requires reproducing the
// warning on a real fixture". A consumer's blanket replaces that with
// one word, swallows real warnings in the generated code alongside the
// audited ones, and does not narrow when SCE narrows. The strip also
// deletes the `#![doc = "SCE-MAP: …"]` provenance marker.
//
// `{stem}_sm.include.rs` is the answer, and these are its promises:
//
//   1. A consumer compiles it having deleted nothing.
//   2. With no blanket of their own — SCE's audited allows do the work,
//      under `#![deny(warnings)]`.
//   3. When SCE drops one allow from the budget, the warning reappears
//      on the consumer's side with no edit to the consumer.
//
// (3) is what makes (2) a measurement rather than a coincidence: a
// consumer that compiles clean because nothing warns proves nothing
// about whose suppression is doing the work.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A document with expressions, so the generated machine is the shape
/// that carries the budget: a datamodel, guards and assignments. The
/// worked example is used rather than a minimal fixture because it is
/// also what a downstream consumer actually compiles.
const FIXTURE: &str = "examples/ai_loop/ai_loop.scxml";
const STEM: &str = "ai_loop";

/// Write the probe crate and return its manifest path.
///
/// One dependency, on purpose — the same discipline
/// `generated_rust_names_only_declared_crates` applies. `build.rs`
/// carries no dependency at all: it copies the artifacts this test
/// already produced through the real `compile_scxml` into `OUT_DIR`, so
/// the consumer's source can use the documented spelling without this
/// test paying to compile the generator a second time.
fn write_probe_crate(crate_dir: &Path, artifacts: &Path, deny_warnings: bool) -> PathBuf {
    let src = crate_dir.join("src");
    fs::create_dir_all(&src).expect("create src");

    fs::write(
        crate_dir.join("build.rs"),
        format!(
            "use std::path::Path;\n\
             fn main() {{\n\
             \x20   let out = std::env::var(\"OUT_DIR\").unwrap();\n\
             \x20   for name in [\"{STEM}_sm.rs\", \"{STEM}_sm.include.rs\"] {{\n\
             \x20       std::fs::copy(Path::new({artifacts:?}).join(name),\n\
             \x20                     Path::new(&out).join(name)).unwrap();\n\
             \x20   }}\n\
             \x20   println!(\"cargo::rerun-if-changed=build.rs\");\n\
             }}\n",
            artifacts = artifacts.to_string_lossy(),
        ),
    )
    .expect("write build.rs");

    // Exactly the spelling `compile_scxml`'s documentation gives, and no
    // blanket allow anywhere: whatever suppresses the generated code's
    // lints has to have arrived from SCE.
    fs::write(
        src.join("lib.rs"),
        format!(
            "{deny}\n\
             pub mod machine {{\n\
             \x20   include!(concat!(env!(\"OUT_DIR\"), \"/{STEM}_sm.include.rs\"));\n\
             }}\n\
             \n\
             /// Names an item through the re-export, so the route is\n\
             /// asserted to place the machine where the consumer looks.\n\
             pub fn policy_name() -> &'static str {{\n\
             \x20   std::any::type_name::<machine::AiLoopPolicy>()\n\
             }}\n",
            deny = if deny_warnings {
                "#![deny(warnings)]"
            } else {
                ""
            },
        ),
    )
    .expect("write lib.rs");

    let manifest = crate_dir.join("Cargo.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\n\
             name = \"sce_include_route_probe\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             publish = false\n\
             build = \"build.rs\"\n\
             \n\
             [lib]\n\
             path = \"src/lib.rs\"\n\
             \n\
             [dependencies]\n\
             sce-rust-runtime = {{ path = {runtime:?} }}\n\
             \n\
             [workspace]\n",
            runtime = repo_root().join("backends/rust/runtime").to_string_lossy(),
        ),
    )
    .expect("write Cargo.toml");
    manifest
}

fn cargo_build(manifest: &Path) -> std::process::Output {
    cargo_build_run(manifest).output
}

/// The same build, with the route it took — see
/// `common::run_cargo_offline_first`. This probe crate has no lock file, so
/// cargo would reach the registry index on every run even when the local
/// cache already answers, which is how a brief outage refused a push.
fn cargo_build_run(manifest: &Path) -> common::ProbeRun {
    // Shared across runs so the runtime's tree is built once, and
    // separate from the outer build's target dir so the two cargo
    // processes never contend for the same lock.
    let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("include-route-probe-target");
    common::run_cargo_offline_first(|| {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("build")
            .arg("--manifest-path")
            .arg(manifest)
            .env("CARGO_TARGET_DIR", &target_dir);
        cmd
    })
}

/// Serialises the `OUT_DIR` write below. The variable is process-wide
/// and cargo runs a file's tests on parallel threads, so two callers
/// setting it would hand one of them the other's directory.
static OUT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run the real writer exactly as a `build.rs` does: `OUT_DIR` in the
/// environment, one call, artifacts on disk.
///
/// Reaching for the real entry point rather than the generator beneath
/// it is the point — the gap this file closes was in the facade, not in
/// the emitter, and a test that called `generate` directly would have
/// been green throughout.
fn compile_into(out_dir: &Path) {
    let _guard = OUT_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("OUT_DIR", out_dir);
    let fixture = repo_root().join(FIXTURE);
    sce_build::compile_scxml(&[fixture.to_str().expect("fixture path is UTF-8")]);
}

#[test]
fn the_documented_route_compiles_with_no_edit_and_no_blanket_allow() {
    let dir = tempdir().expect("tempdir");
    let artifacts = dir.path().join("out");
    fs::create_dir_all(&artifacts).expect("create out dir");
    compile_into(&artifacts);

    let machine = artifacts.join(format!("{STEM}_sm.rs"));
    let shim = artifacts.join(format!("{STEM}_sm.include.rs"));
    assert!(machine.is_file(), "compile_scxml wrote no machine");
    assert!(
        shim.is_file(),
        "compile_scxml wrote no include shim — the consumption route it \
         documents does not exist"
    );

    // The premise: the machine really is a module file, not an
    // include-able fragment. If this stops holding the shim is no longer
    // needed and this whole route should be reconsidered rather than
    // quietly kept.
    let machine_text = fs::read_to_string(&machine).expect("read machine");
    assert!(
        machine_text.lines().any(|l| l.starts_with("#![allow(")),
        "the machine carries no inner-attribute budget"
    );
    assert!(
        machine_text.lines().any(|l| l.starts_with("//!")),
        "the machine carries no inner doc comment"
    );

    // Promise 1 and 2, together: nothing deleted, nothing blanketed.
    let crate_dir = dir.path().join("probe");
    let manifest = write_probe_crate(&crate_dir, &artifacts, true);
    let built = cargo_build(&manifest);
    assert!(
        built.status.success(),
        "a consumer following `compile_scxml`'s documented route must \
         compile under #![deny(warnings)] having deleted nothing:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    // Promise 3: the budget is load-bearing, and it is SCE's copy that
    // bears it. Drop one audited allow from the artifact and the warning
    // has to reach the consumer, with the consumer untouched.
    let without_one = machine_text
        .lines()
        .filter(|l| *l != "#![allow(dead_code)]")
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(
        without_one, machine_text,
        "`#![allow(dead_code)]` is no longer in the budget — pick another \
         allow this fixture actually needs, do not delete the assertion"
    );
    fs::write(&machine, format!("{without_one}\n")).expect("rewrite machine");
    // The copy in the probe's OUT_DIR is stale, but the shim's `#[path]`
    // names the artifact directory absolutely, so the rebuild reads the
    // file just rewritten. Touching the consumer's own source is what
    // makes cargo recompile it, and it is deliberately the ONLY thing
    // touched on the consumer side.
    fs::write(
        crate_dir.join("src/lib.rs"),
        format!(
            "{}\n// force a recompile\n",
            fs::read_to_string(crate_dir.join("src/lib.rs")).expect("read lib.rs")
        ),
    )
    .expect("touch lib.rs");

    let rebuilt = cargo_build(&manifest);
    assert!(
        !rebuilt.status.success(),
        "removing an audited allow left the consumer clean, so the budget \
         reaching them is not the one doing the suppressing"
    );
    let stderr = String::from_utf8_lossy(&rebuilt.stderr);
    assert!(
        stderr.contains("never used") || stderr.contains("never read"),
        "the consumer failed for some reason other than the dropped \
         allow:\n{stderr}"
    );
}

/// The shape the strip destroys, named so the cost is asserted rather
/// than described.
///
/// A consumer filtering `#![` also deletes the machine's `#![doc =
/// "SCE-MAP: …"]` — the §synth-5-O provenance marker that lets a panic
/// backtrace be mapped back to the SCXML line. The documented route
/// keeps it, and this is what says so.
#[test]
fn the_route_preserves_the_provenance_marker_a_strip_would_delete() {
    let dir = tempdir().expect("tempdir");
    let artifacts = dir.path().join("out");
    fs::create_dir_all(&artifacts).expect("create out dir");
    compile_into(&artifacts);

    let machine =
        fs::read_to_string(artifacts.join(format!("{STEM}_sm.rs"))).expect("read machine");
    let marker = machine
        .lines()
        .find(|l| l.starts_with("#![doc = \"SCE-MAP:"))
        .expect("the machine carries a module-level SCE-MAP marker");
    assert!(
        marker.contains("ai_loop.scxml:"),
        "the marker names no source line: {marker}"
    );

    // The shim carries no marker of its own, and must not: an inner
    // attribute there is exactly what `include!` refuses.
    let shim =
        fs::read_to_string(artifacts.join(format!("{STEM}_sm.include.rs"))).expect("read shim");
    assert!(
        !shim.contains("#!["),
        "the shim carries an inner attribute, which is the thing it \
         exists to avoid:\n{shim}"
    );
    assert!(
        !shim.lines().any(|l| l.trim_start().starts_with("//!")),
        "the shim carries an inner doc comment:\n{shim}"
    );
}
