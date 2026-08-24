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
//! 3. **The declaration is what makes the difference, on every backend.**
//!    The same document generated with the flag and without it emits a
//!    different machine: with, the start site; without, the
//!    unsupported-type refusal and nothing else. Asserting a REFUSAL by a
//!    backend that lacks a registry was the earlier form of this claim, and
//!    it retired itself twice — once when every backend grew a `<send>`
//!    registry and again when every backend grew an `<invoke>` one. A test
//!    whose subject is a coverage gap stops asserting anything the day the
//!    gap closes, while still reading as a pass.
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
///
/// Emptied first. The path is derived from the tag rather than randomised,
/// so a previous run's artifacts are sitting in it — and a test that asserts
/// a file was NOT written reads yesterday's answer if they are left there.
fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sce-host-processor-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
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

/// What every backend emits for a declared `<invoke>` type, per language:
/// the machine file the start site lands in, and the call that hands the
/// invocation to the host.
///
/// Read off the emitted text rather than named in a template, because the
/// question this table answers is what a CONSUMER compiles. A row per
/// backend and the arity asserted below: a row deleted rather than fixed
/// is how a sweep quietly stops covering what it names.
const INVOKER_EMISSION: [(&str, &str, &str); 6] = [
    (
        "rust",
        "statechart_host_invoker_sm.rs",
        "perform_host_invoke(",
    ),
    (
        "cpp",
        "statechart_host_invoker_sm.inl",
        "performHostInvoke(",
    ),
    // C11's registry lookup IS the dispatch — there is no wrapper call to
    // name. `_host_inv_entry` is the start site's own local; the exit site
    // uses `_host_cancel_entry`, so the two cannot be confused.
    (
        "c11",
        "statechart_host_invoker_sm.c",
        "_host_inv_entry->handler(",
    ),
    ("go", "statechart_host_invoker_sm.go", "PerformHostInvoke("),
    (
        "python",
        "statechart_host_invoker_sm.py",
        "perform_host_invoke(",
    ),
    (
        "kotlin",
        "statechart_host_invokerSm.kt",
        "performHostInvoke(",
    ),
];

/// Every backend honours an `<invoke>` declaration, and the declaration is
/// what makes the difference.
///
/// This is what `a_backend_without_a_registry_refuses_the_declaration`
/// became, and the reason it had to change is worth stating: that test
/// asserted a REFUSAL, and a refusal is a coverage gap wearing a test's
/// clothes. It evaporates the day the gap closes. It did so twice — the
/// `--host-processor` half emptied when every backend grew a `<send>`
/// registry, and the `--host-invoker` half emptied when every backend grew
/// an `<invoke>` one. An empty sweep asserts nothing while still reading
/// as a pass.
///
/// So the sweep survives with its subject changed. What is asked now is the
/// DECLARATION's own effect: the same document, generated with the flag and
/// without it, emits a different machine. That observable cannot be retired
/// by coverage — it is the feature, not the absence of one.
///
/// Both directions are asserted per backend, because either alone is
/// satisfiable by a machine that is wrong:
///
/// * declared -> the start site is present and the unsupported-type refusal
///   is gone. Emitting both would start the invocation AND raise
///   `error.execution` for it.
/// * undeclared -> the start site is absent. Without this, a backend that
///   dispatched unconditionally would pass, and a host that never declared
///   the type would be handed an invocation it never agreed to run.
///
/// Asserted on the emitted text because this is the build's half; the six
/// runtime channels named on the fixture compile and run the same machine.
#[test]
fn a_declared_invoker_changes_the_machine_on_every_backend() {
    // The floor is the language set itself: a backend dropped from the
    // table would otherwise shrink this sweep silently.
    assert_eq!(
        INVOKER_EMISSION.len(),
        6,
        "the emission table no longer covers every backend SCE generates",
    );

    for (lang, file, start_site) in INVOKER_EMISSION {
        let declared_dir = out_dir(&format!("invoker-declared-{lang}"));
        let declared = run(&[
            "generate",
            invoker_fixture().to_str().unwrap(),
            "-l",
            lang,
            "-o",
            declared_dir.to_str().unwrap(),
            "--host-invoker",
            "x-sce-host",
            "--error-format=json",
        ]);
        assert_eq!(
            declared.exit,
            Some(0),
            "{lang} refused an invoker declaration it can service: {}",
            declared.stderr,
        );
        let emitted = std::fs::read_to_string(declared_dir.join(file))
            .unwrap_or_else(|e| panic!("{lang} did not write {file}: {e}"));
        assert!(
            emitted.contains(start_site),
            "{lang} emitted no start site (`{start_site}`) for a declared invoker",
        );
        assert!(
            !emitted.contains(UNSUPPORTED_INVOKE_REFUSAL[lang_index(lang)]),
            "{lang} still emitted the unsupported-type refusal for a declared \
             invoker, so the machine both starts it and raises for it",
        );

        // The control. Same document, same backend, no declaration: the
        // start site must not appear, or the flag is not what put it there.
        let bare_dir = out_dir(&format!("invoker-bare-{lang}"));
        let bare = run(&[
            "generate",
            invoker_fixture().to_str().unwrap(),
            "-l",
            lang,
            "-o",
            bare_dir.to_str().unwrap(),
            "--error-format=json",
        ]);
        assert_eq!(
            bare.exit,
            Some(0),
            "{lang} refused the undeclared document; an `<invoke>` naming an \
             unimplemented type is valid SCXML: {}",
            bare.stderr,
        );
        let undeclared = std::fs::read_to_string(bare_dir.join(file))
            .unwrap_or_else(|e| panic!("{lang} did not write {file}: {e}"));
        assert!(
            !undeclared.contains(start_site),
            "{lang} emitted the start site (`{start_site}`) with no declaration \
             at all — the host would be handed an invocation it never declared",
        );
        assert!(
            undeclared.contains(UNSUPPORTED_INVOKE_REFUSAL[lang_index(lang)]),
            "{lang} emitted neither a start nor the unsupported-type refusal, \
             so an undeclared `<invoke>` is silently doing nothing",
        );
    }
}

/// The `error.execution` text an undeclared `<invoke type>` raises, indexed
/// by [`INVOKER_EMISSION`]'s row order.
///
/// ⚠ C++ is the odd row and its wording is NOT a typo here. It matches the
/// C++ Interpreter (`sce/src/runtime/InvokeExecutor.cpp`), which predates
/// the five newer backends; the five agreed on the other wording among
/// themselves. Two spellings of one W3C fact is a parity defect on this
/// seam, not a per-backend contract — repaying it edits a template, which
/// re-pins every committed tree, so it is done in the round that owns the
/// regeneration rather than smuggled into this one. When it lands, these
/// six entries collapse to one constant.
const UNSUPPORTED_INVOKE_REFUSAL: [&str; 6] = [
    "<invoke> declares a type this processor does not support",
    "Unsupported <invoke> type: x-sce-host",
    "<invoke> declares a type this processor does not support",
    "<invoke> declares a type this processor does not support",
    "<invoke> declares a type this processor does not support",
    "<invoke> declares a type this processor does not support",
];

fn lang_index(lang: &str) -> usize {
    INVOKER_EMISSION
        .iter()
        .position(|(l, _, _)| *l == lang)
        .expect("the language came from the table")
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

/// The Go half of the same claim.
///
/// Same shape and same reason as the C++ and C11 ones: dropping a backend
/// from the refusal list is a claim made by DELETING something, so the
/// deletion is paid for by asking what the backend now emits.
///
/// Asserted on the emitted text rather than by running the machine because
/// this is the build's half.
/// `backends/go/tests/integration/statechart_host_processor/` compiles and
/// runs that same machine, with a handler and without one.
#[test]
fn a_declared_type_emits_a_dispatch_for_go() {
    let out = out_dir("go-dispatch");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "go",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(
        r.exit,
        Some(0),
        "go refused a declaration it can service: {}",
        r.stderr
    );

    let emitted = std::fs::read_to_string(out.join("statechart_host_processor_sm.go"))
        .expect("the generator wrote the machine");
    // Both halves, because they stopped being one expression. The request
    // was an inline literal until a host-served `<send delay>` had to be
    // QUEUED rather than performed, which needs the request as a value the
    // emitted code can name. Asserting only the literal reads a machine
    // that builds a request and never hands it over as a pass.
    assert!(
        emitted.contains("hostRequest := sce.HostSendRequest{"),
        "no dispatch was emitted for a declared type",
    );
    assert!(
        emitted.contains("engine.PerformHostSend(hostRequest)"),
        "the request was built and never handed to the engine",
    );
    assert!(
        !emitted.contains("names a processor this platform does not support"),
        "a declared type still emitted the unsupported-type refusal",
    );
    // The request has to carry what the author wrote, or the document can
    // name an act but not parameterise it. The fixture's `<param>` is the
    // one field that proves the crossing rather than the call.
    assert!(
        emitted.contains(r#"{Name: "within", Value: "2500"}"#),
        "the emitted dispatch dropped the <param> the fixture declares",
    );
}

/// The Python half of the same claim.
///
/// Same shape and same reason as its siblings: dropping a backend from the
/// refusal list is a claim made by DELETING something, so the deletion is
/// paid for by asking what the backend now emits.
///
/// Two assertions beyond the dispatch, because Python's dependency on the
/// runtime type is an import: a machine emitting the call without the import
/// raises `NameError` at the first act, which no build-time check would see.
///
/// Asserted on the emitted text rather than by running the machine because
/// this is the build's half.
/// `backends/python/tests/integration/host_processor/` imports and runs that
/// same machine, with a handler and without one.
#[test]
fn a_declared_type_emits_a_dispatch_for_python() {
    let out = out_dir("python-dispatch");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "python",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(
        r.exit,
        Some(0),
        "python refused a declaration it can service: {}",
        r.stderr
    );

    let emitted = std::fs::read_to_string(out.join("statechart_host_processor_sm.py"))
        .expect("the generator wrote the machine");
    // Both halves, for the same reason as the Go sibling: the request
    // became a named value when a host-served `<send delay>` had to be
    // queued instead of performed.
    assert!(
        emitted.contains("_host_request = _HostSendRequest("),
        "no dispatch was emitted for a declared type",
    );
    assert!(
        emitted.contains("engine.perform_host_send(_host_request)"),
        "the request was built and never handed to the engine",
    );
    assert!(
        emitted.contains("from sce_runtime import HostSendRequest as _HostSendRequest"),
        "the dispatch was emitted without the import it needs",
    );
    assert!(
        !emitted.contains("names a processor this platform does not support"),
        "a declared type still emitted the unsupported-type refusal",
    );
    // The request has to carry what the author wrote, or the document can
    // name an act but not parameterise it. The fixture's `<param>` is the
    // one field that proves the crossing rather than the call.
    assert!(
        emitted.contains(r#"("within", "\"2500\""),"#),
        "the emitted dispatch dropped the <param> the fixture declares",
    );
}

/// …and a machine that declares nothing carries no import for it.
///
/// The counterpart of `an_undeclared_machine_carries_no_registry_for_c11`:
/// the cheapest way to make the test above pass is to import
/// unconditionally, and an unused import in a generated file a consumer
/// reads is noise this backend does not have to carry.
#[test]
fn an_undeclared_machine_carries_no_host_import_for_python() {
    let out = out_dir("python-no-import");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "python",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(r.exit, Some(0), "generation must succeed: {}", r.stderr);

    let emitted = std::fs::read_to_string(out.join("statechart_host_processor_sm.py"))
        .expect("the generator wrote the machine");
    assert!(
        !emitted.contains("HostSendRequest"),
        "a machine that declared no host processor still imports the request type",
    );
}

/// The Kotlin half of the same claim, and the one that emptied the refusal
/// list.
///
/// Same shape and same reason as its four siblings: dropping a backend from
/// the refusal list is a claim made by DELETING something, so the deletion is
/// paid for by asking what the backend now emits. With this one there is
/// nothing left to drop, which is why
/// `a_backend_without_a_registry_refuses_the_declaration` no longer has a
/// processor half — these five tests are what stands in its place.
///
/// Asserted on the emitted text rather than by running the machine because
/// this is the build's half.
/// `backends/kotlin/tests/.../HostProcessorTest.kt` compiles and runs that
/// same machine, with a handler and without one.
#[test]
fn a_declared_type_emits_a_dispatch_for_kotlin() {
    let out = out_dir("kotlin-dispatch");
    let r = run(&[
        "generate",
        fixture().to_str().unwrap(),
        "-l",
        "kotlin",
        "-o",
        out.to_str().unwrap(),
        "--host-processor",
        "x-sce-host",
    ]);
    assert_eq!(
        r.exit,
        Some(0),
        "kotlin refused a declaration it can service: {}",
        r.stderr
    );

    let emitted = std::fs::read_to_string(out.join("statechart_host_processorSm.kt"))
        .expect("the generator wrote the machine");
    assert!(
        emitted.contains("performHostSend("),
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
        emitted.contains(r#"hostParams["within"] = listOf("2500")"#),
        "the emitted dispatch dropped the <param> the fixture declares",
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
