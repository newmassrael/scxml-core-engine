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

/// Who in the consumer crate names the machine.
///
/// The distinction is not cosmetic: a `pub use` inside a private module is
/// unreachable from outside the crate, so whether `unused_imports` fires on
/// the shim's re-export depends entirely on this. A consumer whose library
/// only *hosts* the machine and drives it from tests — which is the shape a
/// plugin crate has — is the one that found the gap.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReachedFrom {
    /// The library names an item through the re-export.
    LibraryCode,
    /// Only a `#[cfg(test)]` module does.
    TestsOnly,
}

/// Write the probe crate and return its manifest path.
///
/// One dependency, on purpose — the same discipline
/// `generated_rust_names_only_declared_crates` applies. `build.rs`
/// carries no dependency at all: it copies the artifacts this test
/// already produced through the real `compile_scxml` into `OUT_DIR`, so
/// the consumer's source can use the documented spelling without this
/// test paying to compile the generator a second time.
fn write_probe_crate(
    crate_dir: &Path,
    artifacts: &Path,
    deny_warnings: bool,
    pkg: &str,
) -> PathBuf {
    write_probe_crate_reached_from(
        crate_dir,
        artifacts,
        deny_warnings,
        ReachedFrom::LibraryCode,
        pkg,
    )
}

/// `pkg` is the probe crate's package name, and it must differ per caller.
///
/// Cargo derives a build script's `OUT_DIR` from the package identity, and
/// every probe here shares one `CARGO_TARGET_DIR` (see `cargo_build_run`, which
/// shares it deliberately so the runtime is built once). Two probes named the
/// same package therefore land in ONE `OUT_DIR` — and each probe's `build.rs`
/// copies its own `_sm.include.rs` there, whose `#[path]` names the tempdir
/// that probe was generated into. Whichever ran last wins, and the other then
/// compiles against a shim pointing into a tempdir that has already been
/// dropped:
///
/// ```text
/// Compiling sce_include_route_probe v0.0.0 (/tmp/.tmpqskBGS/probe)
/// error: couldn't read `/tmp/.tmp6Ete3k/out/ai_loop_sm.rs`: No such file
///  --> .../build/sce_include_route_probe-9e306a704447e8c2/out/ai_loop_sm.include.rs:3
/// ```
///
/// That is a push rejection measured 2026-08-21, and it is a race: the same
/// test passes alone and passes most of the time in the file. Naming the
/// package after the test that owns it separates the `OUT_DIR`s without giving
/// up the shared target dir.
fn write_probe_crate_reached_from(
    crate_dir: &Path,
    artifacts: &Path,
    deny_warnings: bool,
    reached_from: ReachedFrom,
    pkg: &str,
) -> PathBuf {
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
    let body = match reached_from {
        ReachedFrom::LibraryCode => format!(
            "pub mod machine {{\n\
             \x20   include!(concat!(env!(\"OUT_DIR\"), \"/{STEM}_sm.include.rs\"));\n\
             }}\n\
             \n\
             /// Names an item through the re-export, so the route is\n\
             /// asserted to place the machine where the consumer looks.\n\
             pub fn policy_name() -> &'static str {{\n\
             \x20   std::any::type_name::<machine::AiLoopPolicy>()\n\
             }}\n"
        ),
        ReachedFrom::TestsOnly => format!(
            "mod machine {{\n\
             \x20   include!(concat!(env!(\"OUT_DIR\"), \"/{STEM}_sm.include.rs\"));\n\
             }}\n\
             \n\
             #[cfg(test)]\n\
             mod tests {{\n\
             \x20   #[test]\n\
             \x20   fn the_machine_is_reachable_from_a_test() {{\n\
             \x20       let _ = std::any::type_name::<crate::machine::AiLoopPolicy>();\n\
             \x20   }}\n\
             }}\n"
        ),
    };

    fs::write(
        src.join("lib.rs"),
        format!(
            "{deny}\n{body}",
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
             name = \"{pkg}\"\n\
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
    let manifest = write_probe_crate(&crate_dir, &artifacts, true, "sce_include_route_probe_lib");
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

/// The shim's own two lines carry a budget too.
///
/// The machine's twelve allows are audited and reach the consumer intact —
/// that is what the route above is for. The shim SCE writes beside it had
/// none, and it is generated code the consumer may not edit either. Measured
/// 2026-08-20 by a downstream plugin crate: a library that only *hosts* the
/// machine and drives it from tests leaves the re-export unreachable from
/// outside, `unused_imports` fires on `pub use {stem}_sm::*;`, and under the
/// consumer's own `deny(warnings)` that is a hard error on a line they did not
/// write and cannot fix.
///
/// One named allow rather than a blanket, for the reason the machine's twelve
/// are named: a blanket on two lines would also swallow a real warning in
/// them.
#[test]
fn the_shim_compiles_when_only_a_test_reaches_the_machine() {
    let dir = tempdir().expect("tempdir");
    let artifacts = dir.path().join("out");
    fs::create_dir_all(&artifacts).expect("create out dir");
    compile_into(&artifacts);

    let crate_dir = dir.path().join("probe");
    let manifest = write_probe_crate_reached_from(
        &crate_dir,
        &artifacts,
        true,
        ReachedFrom::TestsOnly,
        "sce_include_route_probe_tests_only",
    );
    let built = cargo_build(&manifest);
    assert!(
        built.status.success(),
        "a consumer that reaches the machine only from its tests must still \
         compile under #![deny(warnings)]: the shim is SCE's line, so the \
         allow that keeps it quiet has to be SCE's too\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    // And the allow is exactly one, named. A blanket here would pass the
    // assertion above while giving up the thing the route exists to keep.
    let shim =
        fs::read_to_string(artifacts.join(format!("{STEM}_sm.include.rs"))).expect("read shim");
    assert!(
        shim.contains("#[allow(unused_imports)]"),
        "the shim compiles clean for some other reason than the named \
         allow, so nothing pins it:\n{shim}"
    );
    assert!(
        !shim.contains("allow(warnings"),
        "the shim reaches for a blanket, which is the consumer-side habit \
         this whole route replaced:\n{shim}"
    );
}
