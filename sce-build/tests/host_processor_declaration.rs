// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! W3C SCXML 6.2.5 — the build half of a host-served Event I/O Processor,
//! measured through the CLI a consumer actually runs.
//!
//! Three claims, and the third is the one that keeps the other two honest:
//!
//! 1. **Without a declaration, the build says so.** `needs_host_processor`
//!    is true and `host_processor_causes` names the site. This is the
//!    silence being repaid — a `<send type>` nobody implements used to
//!    compile with no word at all.
//! 2. **With a declaration, the build stops saying so and emits a
//!    dispatch.** The cause disappears because there is no longer anything
//!    to report, and `host_processor_types` echoes the build's half of the
//!    contract so a consumer can check it against its registrations.
//! 3. **A backend with no registry refuses the declaration.** Honouring it
//!    there would emit a dispatch nothing can service — the build would
//!    then actively promise a delivery that never happens, which is worse
//!    than the silence this whole round removes.
//!
//! The manifest is read as JSON rather than grepped: a field that moved
//! from present-and-false to absent would pass a substring check and change
//! what a consumer reads.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The binary under test, resolved the way every other CLI test here does.
///
/// `CARGO_BIN_EXE_*` rather than a walk up from `current_exe()`: cargo
/// treats the named binary as a build dependency of this test, so it is
/// rebuilt before the test runs. A hand-rolled path finds whatever happens
/// to be on disk — which passes against a stale binary, and makes a change
/// to the CLI invisible to a mutation round measuring this suite.
fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn fixture() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml")
}

struct Run {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Run {
    let out = Command::new(codegen_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    Run {
        exit: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn manifest(run: &Run) -> serde_json::Value {
    let line = run
        .stdout
        .lines()
        .last()
        .unwrap_or_else(|| panic!("no manifest on stdout; stderr was:\n{}", run.stderr));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("manifest is not JSON ({e}): {line}"))
}

/// Scratch output directory, per test, so two tests cannot read each
/// other's artifacts.
fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sce-host-processor-{tag}"));
    std::fs::create_dir_all(&dir).expect("scratch output directory");
    dir
}

#[test]
fn without_a_declaration_the_build_names_the_unserved_send() {
    let out = out_dir("undeclared");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);

    let m = manifest(&r);
    assert_eq!(
        m["needs_host_processor"], true,
        "an unserved <send type> was not reported: {m}"
    );
    let causes = m["host_processor_causes"]
        .as_array()
        .unwrap_or_else(|| panic!("no host_processor_causes: {m}"));
    let send: Vec<&serde_json::Value> =
        causes.iter().filter(|c| c["kind"] == "send-type").collect();
    assert_eq!(send.len(), 1, "expected one send cause: {causes:?}");
    assert_eq!(send[0]["processor_type"], "x-sce-host");
    // The line is the point: the report exists so a repair can open the
    // file, not so a consumer knows something somewhere is wrong.
    assert!(
        send[0]["location"]["line"].is_number(),
        "the cause carries no line: {}",
        send[0]
    );
    // Not a rejection. The document is valid SCXML with defined meaning.
    assert!(
        m.get("rejected").is_none(),
        "the document was rejected: {m}"
    );
}

#[test]
fn a_declaration_removes_the_cause_and_echoes_itself() {
    let out = out_dir("declared");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);

    let m = manifest(&r);
    assert_eq!(
        m["needs_host_processor"], false,
        "a declared type is still reported as unserved: {m}"
    );
    assert!(
        m.get("host_processor_causes").is_none(),
        "a declared type left a cause behind: {m}"
    );
    assert_eq!(
        m["host_processor_types"],
        serde_json::json!(["x-sce-host"]),
        "the build did not echo its own declaration: {m}"
    );

    // The emitted code is the other half of the claim: a manifest saying
    // "served" over code that still refuses would be the same silence one
    // layer up.
    let emitted = std::fs::read_to_string(out.join("statechart_host_processor_sm.rs"))
        .expect("the generator wrote the machine");
    assert!(
        emitted.contains("perform_host_send"),
        "no dispatch was emitted for a declared type",
    );
    assert!(
        !emitted.contains("names a processor this platform does not support"),
        "a declared type still emitted the unsupported-type refusal",
    );
}

/// Declaring a type SCE already implements is not an error — it names
/// something already true. Refusing it would make a host's declaration
/// list order-sensitive against SCE's own version.
#[test]
fn declaring_a_standard_processor_is_a_no_op() {
    let out = out_dir("standard");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);
    let m = manifest(&r);
    // The fixture's own unserved send is untouched: declaring the standard
    // processor claims nothing else.
    assert_eq!(m["needs_host_processor"], true, "{m}");
}

fn invoker_fixture() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml")
}

/// The invoke half of the same claim: undeclared, the build names the site;
/// declared, it stops naming it and emits a start instead.
#[test]
fn the_invoke_half_is_reported_and_then_claimed() {
    let undeclared = out_dir("invoke-undeclared");
    let r = run(&[
        "generate",
        invoker_fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        undeclared.to_str().unwrap(),
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);
    let m = manifest(&r);
    assert_eq!(m["needs_host_processor"], true, "{m}");
    let causes = m["host_processor_causes"]
        .as_array()
        .unwrap_or_else(|| panic!("no host_processor_causes: {m}"));
    assert_eq!(causes.len(), 1, "{causes:?}");
    assert_eq!(causes[0]["kind"], "invoke-type");
    assert_eq!(causes[0]["invoke"], "probe");

    let declared = out_dir("invoke-declared");
    let r = run(&[
        "generate",
        invoker_fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        declared.to_str().unwrap(),
        "--host-invoker",
        "x-sce-host",
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);
    let m = manifest(&r);
    assert_eq!(m["needs_host_processor"], false, "{m}");
    assert!(m.get("host_processor_causes").is_none(), "{m}");
    assert_eq!(
        m["host_invoker_types"],
        serde_json::json!(["x-sce-host"]),
        "the build did not echo its own invoker declaration: {m}"
    );

    let emitted = std::fs::read_to_string(declared.join("statechart_host_invoker_sm.rs"))
        .expect("the generator wrote the machine");
    assert!(
        emitted.contains("perform_host_invoke"),
        "no start was emitted for a declared invoker",
    );
    // The teardown half. A machine that starts a host invocation and never
    // cancels it leaves the host running work nobody will stop, and no
    // configuration assertion can see that.
    assert!(
        emitted.contains("cancel_host_invoke"),
        "no cancel was emitted for a declared invoker",
    );
    assert!(
        !emitted.contains("<invoke> declares a type this processor does not support"),
        "a declared invoker still emitted the unsupported-type refusal",
    );
}

/// The two declarations are separate contracts, and the flags must not
/// leak into each other: declaring a send processor cannot claim an
/// `<invoke>` of the same name, or the build would promise a lifecycle
/// nothing implements.
#[test]
fn a_send_declaration_does_not_claim_the_invoke_of_the_same_name() {
    let out = out_dir("invoke-crossed");
    let r = run(&[
        "generate",
        invoker_fixture().to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);
    let m = manifest(&r);
    assert_eq!(
        m["needs_host_processor"], true,
        "a send declaration claimed an <invoke>: {m}"
    );
    let emitted = std::fs::read_to_string(out.join("statechart_host_invoker_sm.rs"))
        .expect("the generator wrote the machine");
    assert!(
        !emitted.contains("perform_host_invoke"),
        "a send declaration emitted an invoke start",
    );
}

/// Every backend without a host-processor registry refuses the
/// declaration, by name.
///
/// Iterated rather than spot-checked: a seventh backend that gained a
/// dispatch path and forgot its registry would otherwise be the one nobody
/// asked.
#[test]
fn a_backend_without_a_registry_refuses_the_declaration() {
    for lang in ["cpp", "c11", "kotlin", "go", "python"] {
        let out = out_dir(&format!("refuse-{lang}"));
        let r = run(&[
            "generate",
            fixture().to_str().unwrap(),
            "-l",
            lang,
            "-o",
            out.to_str().unwrap(),
            "--host-processor",
            "x-sce-host",
            "--error-format=json",
        ]);
        assert_ne!(
            r.exit,
            Some(0),
            "{lang} accepted a declaration it cannot service",
        );
        assert!(
            r.stderr.contains("generate/unsupported-feature"),
            "{lang} refused with the wrong diagnostic: {}",
            r.stderr,
        );
        // The message must name the backend AND the type, or an author
        // reading it cannot tell which of several declarations to drop.
        assert!(
            r.stderr.contains("x-sce-host"),
            "{lang}'s refusal does not name the declared type: {}",
            r.stderr,
        );
    }
}

/// The same document, same backends, no declaration: accepted. Without
/// this the test above would also pass on a build that refused the fixture
/// outright for some unrelated reason.
#[test]
fn those_backends_accept_the_same_document_undeclared() {
    for lang in ["cpp", "c11", "kotlin", "go", "python"] {
        let out = out_dir(&format!("accept-{lang}"));
        let r = run(&[
            "generate",
            fixture().to_str().unwrap(),
            "-l",
            lang,
            "-o",
            out.to_str().unwrap(),
        ]);
        assert_eq!(
            r.exit,
            Some(0),
            "{lang} refused the undeclared document: {}",
            r.stderr,
        );
    }
}
