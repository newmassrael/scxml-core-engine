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

/// The C++ half of the same claim.
///
/// Dropping a backend from `a_backend_without_a_registry_refuses_the_
/// declaration` is a claim that it now HAS a path, and a list is the one
/// shape in which that claim can be made by deleting something. So the
/// deletion has to be paid for here: the machine C++ emits for a declared
/// type dispatches, and carries no refusal for it.
///
/// Asserted on the emitted text rather than by running the machine because
/// this is the build's half. `tests/integration/HostProcessorAotTest.cpp`
/// compiles and runs that same machine, with a handler and without one.
#[test]
fn a_declared_type_emits_a_dispatch_for_cpp() {
    let out = out_dir("cpp-dispatch");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "cpp",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(
        r.exit,
        Some(0),
        "cpp refused a declaration it can service: {}",
        r.stderr
    );

    let emitted = std::fs::read_to_string(out.join("statechart_host_processor_sm.inl"))
        .expect("the generator wrote the machine");
    assert!(
        emitted.contains("performHostSend"),
        "no dispatch was emitted for a declared type",
    );
    assert!(
        !emitted.contains("names a processor this platform does not support"),
        "a declared type still emitted the unsupported-type refusal",
    );
    // The request has to carry what the author wrote, or the document can
    // name an act but not parameterise it. The fixture's `<param>` is the
    // one field that proves the crossing rather than the call.
    assert!(
        emitted.contains(r#"hostRequest.params["within"]"#),
        "the emitted dispatch dropped the <param> the fixture declares",
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
///
/// C++ and C11 are absent from the `--host-processor` half and present in
/// the `--host-invoker` half, and that asymmetry is the point rather than an
/// omission: each grew a `<send>` dispatch registry —
/// `StaticExecutionEngine::registerEventProcessor` and
/// `sce_host_processor_registry_t` respectively — and neither has an invoker
/// registry. While the two flags were one refusal a backend either had both
/// or neither, and this loop could ask them together; a single answer now
/// would either accept an invoker declaration they cannot service or refuse a
/// processor declaration they can.
/// `a_declared_type_emits_a_dispatch_for_cpp` and its C11 sibling are the
/// other side of the same claim — without them, dropping a backend from a
/// refusal list would read as a pass.
#[test]
fn a_backend_without_a_registry_refuses_the_declaration() {
    // BOTH flags, because they are two lists feeding one check. Sweeping
    // only `--host-processor` left the invoke half untested: a mutation
    // that dropped `host_invoker_types` from the check reddened nothing,
    // so a build declaring only an invoker would have compiled on a
    // backend that cannot service it.
    let mut cases: Vec<(&str, &str)> = Vec::new();
    for lang in ["kotlin", "go", "python"] {
        cases.push((lang, "--host-processor"));
        cases.push((lang, "--host-invoker"));
    }
    cases.push(("cpp", "--host-invoker"));
    cases.push(("c11", "--host-invoker"));

    for (lang, flag) in cases {
        {
            let doc = if flag == "--host-processor" {
                fixture()
            } else {
                invoker_fixture()
            };
            let tag = flag.trim_start_matches("--");
            let out = out_dir(&format!("refuse-{lang}-{tag}"));
            let r = run(&[
                "generate",
                doc.to_str().unwrap(),
                "-l",
                lang,
                "-o",
                out.to_str().unwrap(),
                flag,
                "x-sce-host",
                "--error-format=json",
            ]);
            assert_ne!(
                r.exit,
                Some(0),
                "{lang} accepted a {flag} declaration it cannot service",
            );
            assert!(
                r.stderr.contains("generate/unsupported-feature"),
                "{lang} refused {flag} with the wrong diagnostic: {}",
                r.stderr,
            );
            // The message must name the backend AND the type, or an author
            // reading it cannot tell which of several declarations to drop.
            assert!(
                r.stderr.contains("x-sce-host"),
                "{lang}'s {flag} refusal does not name the declared type: {}",
                r.stderr,
            );
        }
    }
}

/// The C11 half of the same claim.
///
/// Same shape as the C++ one above and for the same reason: dropping a
/// backend from the refusal list is a claim made by DELETING something, so
/// the deletion is paid for by asking what the backend now emits.
///
/// Three assertions rather than the C++ two, because C11's registry lives
/// on the generated struct rather than in a hand-written runtime class:
/// the dispatch, the storage it dispatches into, and the entry point a
/// host wires through. A machine emitting the call without the field, or
/// the field without the init variant, would compile in this test's
/// absence and leave the host with no way to register before the first
/// `<onentry>` act.
///
/// Asserted on the emitted text rather than by running the machine because
/// this is the build's half. `backends/c/tests/integration/test_host_processor.c`
/// compiles and runs that same machine, with a handler and without one.
#[test]
fn a_declared_type_emits_a_dispatch_for_c11() {
    let out = out_dir("c11-dispatch");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "c11",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(
        r.exit,
        Some(0),
        "c11 refused a declaration it can service: {}",
        r.stderr
    );

    let source = std::fs::read_to_string(out.join("statechart_host_processor_sm.c"))
        .expect("the generator wrote the machine");
    assert!(
        source.contains("statechart_host_processor_perform_host_send(sm, &_host_req)"),
        "no dispatch was emitted for a declared type",
    );
    assert!(
        !source.contains("names a processor this platform does not support"),
        "a declared type still emitted the unsupported-type refusal",
    );
    // The request has to carry what the author wrote, or the document can
    // name an act but not parameterise it. The fixture's `<param>` is the
    // one field that proves the crossing rather than the call.
    assert!(
        source.contains(r#"_host_params[_host_param_count].name = "within""#),
        "the emitted dispatch dropped the <param> the fixture declares",
    );

    let header = std::fs::read_to_string(out.join("statechart_host_processor_sm.h"))
        .expect("the generator wrote the header");
    assert!(
        header.contains("sce_host_processor_registry_t host_processors;"),
        "the machine dispatches into a registry it does not carry",
    );
    assert!(
        header.contains("statechart_host_processor_init_with_host_processors"),
        "no entry point for wiring a host before the first <onentry> act",
    );
}

/// A machine that declares nothing carries no registry.
///
/// The footprint half of the gate, and the reason the emission is keyed on
/// the declaration at all: this backend exists for deployments where a
/// per-struct array of handler slots is a real cost. Without this, the
/// cheapest way to make the test above pass would be to emit the registry
/// unconditionally and every machine in the corpus would grow.
#[test]
fn an_undeclared_machine_carries_no_registry_for_c11() {
    let out = out_dir("c11-no-registry");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "c11",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);

    let header = std::fs::read_to_string(out.join("statechart_host_processor_sm.h"))
        .expect("the generator wrote the header");
    assert!(
        !header.contains("sce_host_processor_registry_t"),
        "a machine that declared no host processor still carries the registry",
    );
    assert!(
        !header.contains("host_processor.h"),
        "a machine that declared no host processor still includes the runtime header",
    );
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
