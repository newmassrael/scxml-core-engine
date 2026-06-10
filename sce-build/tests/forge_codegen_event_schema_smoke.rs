// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Item C1 Path A — per-backend EventSchema codegen
// smoke gate.
//
// Generates each of the 2 standalone EventSchema fixtures (no enum
// imports — those are exercised at the integration-test layer via
// `positive_cross_doc_enum_import_compiles`) on all 6 backends and
// feeds the output to a per-backend syntax checker (g++, syn::parse_file,
// gofmt -e, kotlinc, py_compile, clang -fsyntax-only). The contract
// is structural — no runtime correctness assertion — but every
// backend's generated payload-struct header MUST compile per
// `feedback_byte_goldens_not_compile` + the design RFC §5.2 acceptance
// gate ("6/6 EventSchema backends produce compilable code per
// per-backend harness").
//
// Toolchain-missing behaviour mirrors `forge_codegen_enum_smoke.rs`
// and `codegen_smoke.rs`: detect each compiler with `which` and skip
// with `eprintln!` if absent. CI is expected to install every
// backend's toolchain; local dev machines without one toolchain
// should not be blocked.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Standalone EventSchema fixtures from `tests/fixtures/event_schema/`.
/// Each varies one axis per `feedback_codegen_fixture_orthogonal_axes`:
///   * `schema_job_completed_minimal` — one uint32 field
///   * `schema_job_completed_multi`   — uint32 + bool + string axes
///
/// Cross-doc enum-import fixtures (`schema_job_completed_with_enum.scxml`)
/// require multi-file orchestrator invocation — exercised by the
/// integration test `positive_cross_doc_enum_import_compiles` in
/// `event_schema_kind.rs`, not by this single-file smoke gate.
const FIXTURES: &[&str] = &["schema_job_completed_minimal", "schema_job_completed_multi"];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event_schema")
}

fn scratch_for(lang: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_event_schema_smoke");
    base.join(lang)
}

fn resolve_tool(name: &str) -> Option<PathBuf> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

/// Run `sce-codegen generate -l <lang> -o <out> <fixture>.scxml`. Adds
/// `--go-module-prefix` when targeting Go (resolve_imports rejects bare
/// Go imports; standalone EventSchema fixtures import nothing, but the
/// flag must still be supplied because the CLI validates it up-front).
fn run_generate(lang: &str, out_dir: &Path, fixture: &str) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let fx_path = fixtures_dir().join(format!("{fixture}.scxml"));

    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args(["generate", "-l", lang, "-o"]).arg(out_dir);
    if lang == "go" {
        cmd.args([
            "--go-module-prefix",
            "example.com/sce-forge/event-schema-smoke",
        ]);
    }
    cmd.arg(&fx_path);

    let output = cmd.output().expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l {lang} {fixture} failed (exit {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn find_by_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn reset_scratch(lang: &str) -> PathBuf {
    let scratch = scratch_for(lang);
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");
    scratch
}

#[test]
fn smoke_cpp() {
    let Some(gpp) = resolve_tool("g++") else {
        eprintln!("SKIP smoke_cpp: g++ not on PATH");
        return;
    };
    let scratch = reset_scratch("cpp");

    let mut headers: Vec<String> = Vec::new();
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("cpp", &out_dir, fixture);
        include_dirs.push(out_dir.clone());
        for hdr in find_by_ext(&out_dir, "h") {
            if let Some(name) = hdr.file_name().and_then(|s| s.to_str()) {
                headers.push(name.to_string());
            }
        }
    }
    assert!(!headers.is_empty(), "no .h artefacts emitted");

    let stub = scratch.join("stub.cpp");
    let mut src = String::new();
    for h in &headers {
        src.push_str(&format!("#include \"{h}\"\n"));
    }
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&gpp);
    cmd.args(["-std=c++20", "-fsyntax-only", "-Wall", "-Wextra", "-Werror"]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run g++");

    assert!(
        output.status.success(),
        "g++ -fsyntax-only failed\nheaders: {headers:?}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn smoke_rust() {
    let scratch = reset_scratch("rust");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("rust", &out_dir, fixture);
        let rs_files = find_by_ext(&out_dir, "rs");
        assert!(!rs_files.is_empty(), "no .rs emitted for {fixture}");
        for rs in rs_files {
            let src = std::fs::read_to_string(&rs).expect("read rs");
            if let Err(e) = syn::parse_file(&src) {
                panic!(
                    "syn::parse_file failed for {fixture} at {}: {e}",
                    rs.display(),
                );
            }
        }
    }
}

// NL→IR Item C1 Path A (EventSchema MCU native lowering, step 2) — the
// importing-statechart side of the contract. `statechart_minimal`
// imports the minimal schema and reads `_event.data.elapsed_ms` in a
// transition guard. The Rust backend must lower it to a script-engine-
// free native typed-payload comparison: `needs_script_engine = false`,
// a `<Machine>Payload` sum, and a `matches!(&self.pending_payload, …)`
// guard — never the raw ECMAScript cond (which would not even parse:
// `===` is not Rust) and never the `safe_evaluate_guard` script path.
// `syn::parse_file` is the syntactic gate; the substring assertions pin
// the lowering shape so a regression to the raw-cond branch fails loudly.
#[test]
fn statechart_native_lowering_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("rust_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("rust", &out_dir, "statechart_minimal");

    let rs_files = find_by_ext(&out_dir, "rs");
    assert_eq!(rs_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&rs_files[0]).expect("read rs");

    if let Err(e) = syn::parse_file(&src) {
        panic!("generated native-lowered SM does not parse as Rust: {e}");
    }

    assert!(
        src.contains("const NEEDS_SCRIPT_ENGINE: bool = false;"),
        "typed-guard statechart must not require a script engine"
    );
    assert!(
        src.contains("pub enum StatechartMinimalPayload"),
        "expected the per-document typed payload sum"
    );
    assert!(
        src.contains(
            "matches!(&self.pending_payload, StatechartMinimalPayload::JobCompleted(ev) if ev.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: an EXTENSION TRAIT (orphan-rule-clean —
    // `Engine` is foreign, so an inherent impl would be E0116) whose impl
    // binds the event name + payload variant in one call so the pairing
    // cannot be constructed inconsistently (the generic
    // `raise_external_typed` is the underlying primitive).
    assert!(
        src.contains("pub trait StatechartMinimalInject")
            && src.contains(
                "impl StatechartMinimalInject for ::sce_rust_runtime::Engine<StatechartMinimalPolicy>"
            )
            && src.contains(
                "fn raise_job_completed(&mut self, payload: StatechartMinimalJobCompletedPayload)"
            )
            && src.contains(
                "self.raise_external_typed(StatechartMinimalEvent::JobCompleted, \
                 StatechartMinimalPayload::JobCompleted(payload))"
            ),
        "expected the per-event typed inject extension trait (orphan-rule-clean); got:\n{src}"
    );
    assert!(
        !src.contains("safe_evaluate_guard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The raw cond fragment `elapsed_ms ===` would only appear if the
    // pre-lowering ECMAScript guard leaked into the emitted code (`===`
    // is not valid Rust). A precise sentinel — decorative `// ====`
    // separators and `_event.data` prose comments do not match it.
    assert!(
        !src.contains("elapsed_ms ==="),
        "the raw ECMAScript guard must be fully lowered away"
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Go parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Go backend
// must lower the typed `_event.data.elapsed_ms` guard to a script-engine-free
// native comparison: a `<Machine>PayloadTag` enum, a `pendingPayloadTag` +
// `pending<Event>Payload` policy channel, a per-event `Raise<Event>` inject
// seam, and a `p.pendingPayloadTag == … && (…)` guard — never the raw
// ECMAScript cond (`===` is not Go) and never the `p.evaluateGuard(...)`
// script path. `gofmt -e` is the syntactic gate; the substring assertions pin
// the lowering shape. The real compile+run is the committed integration gate
// `sce-go-tests/integration/event_schema_native` (regen:
// scripts/regen_event_schema_native_go.sh).
#[test]
fn statechart_native_lowering_go_emits_engine_free_typed_guard() {
    let Some(gofmt) = resolve_tool("gofmt") else {
        eprintln!("SKIP statechart_native_lowering_go: gofmt not on PATH");
        return;
    };
    let scratch = reset_scratch("go_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("go", &out_dir, "statechart_minimal");

    let go_files = find_by_ext(&out_dir, "go");
    assert_eq!(go_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&go_files[0]).expect("read go");

    let gofmt_ok = Command::new(&gofmt)
        .arg("-e")
        .arg(&go_files[0])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("run gofmt");
    assert!(
        gofmt_ok.status.success(),
        "gofmt -e failed for the native-lowered Go SM\nstderr: {}",
        String::from_utf8_lossy(&gofmt_ok.stderr),
    );

    assert!(
        src.contains("type StatechartMinimalPayloadTag int"),
        "expected the per-document typed payload tag enum; got:\n{src}"
    );
    assert!(
        src.contains(
            "p.pendingPayloadTag == StatechartMinimalPayloadTagJobCompleted && (p.pendingJobCompletedPayload.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a free function (the Go twin of Rust's
    // extension trait — `Engine` is foreign, so no method can be added) that
    // binds the event name + payload field values in one call, packing them
    // into the type-erased `TypedPayload` carrier.
    assert!(
        src.contains(
            "func RaiseJobCompleted(e *sce.Engine[StatechartMinimalState, StatechartMinimalEvent], elapsed_ms uint32)"
        ) && src.contains(
            "Metadata: sce.EventMetadata{EventType: sce.EventTypeExternal, TypedPayload: StatechartMinimalJobCompletedPayload{"
        ),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    assert!(
        !src.contains("evaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The `// "…" lowered to …` traceability comment echoes the raw cond
    // (`elapsed_ms === 0`), so a bare `elapsed_ms ===` sentinel would false-
    // positive on it (the same choice the C11 smoke makes). A precise sentinel
    // instead: the raw ECMAScript cond would only reach emitted code via the
    // `{% else %} if {{ trans.cond }}` arm, i.e. `if _event.data.elapsed_ms === 0 {`.
    assert!(
        !src.contains("if _event.data.elapsed_ms === 0 {"),
        "the raw ECMAScript guard must be fully lowered away (not leaked into the else arm)"
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Kotlin parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Kotlin
// backend lowers the typed `_event.data.elapsed_ms` guard to a script-engine-
// free native comparison over the type-erased `EventMetadata.typedPayload`
// carrier: a per-event payload data class, a nullable `pending<Event>Payload`
// field, a `populateTypedPayload` override that `is`-checks the carrier, a
// per-event `raise<Event>` inject seam, and a `pending<Event>Payload != null &&
// (…)` guard — never the raw ECMAScript cond (`===` is not Kotlin) and never
// the `safeEvaluateGuard(...)` script path. The substring assertions pin the
// lowering shape; a kotlinc compile against the runtime jar is the syntactic
// gate (skipped when kotlinc or the jar is absent — the same toolchain-missing
// behaviour as `smoke_kotlin`). The real compile+run is the committed
// integration gate `sce-kotlin-tests` `EventSchemaNativeTest` (regen:
// scripts/regen_event_schema_native_kotlin.sh).
#[test]
fn statechart_native_lowering_kotlin_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("kotlin_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("kotlin", &out_dir, "statechart_minimal");

    let kt_files = find_by_ext(&out_dir, "kt");
    assert_eq!(kt_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&kt_files[0]).expect("read kt");

    assert!(
        src.contains("data class StatechartMinimalJobCompletedPayload(val elapsed_ms: UInt)"),
        "expected the per-event typed payload data class; got:\n{src}"
    );
    // Nullable carrier field replaces Go's tag enum — `null` is the "no payload"
    // sentinel, and the guard's `… != null` prefix is the tag check.
    assert!(
        src.contains(
            "private var pendingJobCompletedPayload: StatechartMinimalJobCompletedPayload? = null"
        ),
        "expected the nullable typed-payload policy field; got:\n{src}"
    );
    assert!(
        src.contains("override fun populateTypedPayload(metadata: EventMetadata)")
            && src.contains(
                "is StatechartMinimalJobCompletedPayload -> pendingJobCompletedPayload = tp"
            ),
        "expected the populateTypedPayload carrier lift; got:\n{src}"
    );
    assert!(
        src.contains(
            "pendingJobCompletedPayload != null && (pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt())"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a member of the generated machine (which IS the
    // engine subclass — no orphan-rule barrier as in Rust/Go) binding the event
    // name + payload field values in one call.
    assert!(
        src.contains("fun raiseJobCompleted(elapsed_ms: UInt)")
            && src.contains(
                "EventMetadata(type = \"external\", typedPayload = StatechartMinimalJobCompletedPayload(elapsed_ms))"
            ),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    assert!(
        !src.contains("safeEvaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The raw ECMAScript cond would only reach emitted code via the
    // `{% else %}` arm (`… && _event.data.elapsed_ms === 0 ->`); a bare
    // `elapsed_ms ===` sentinel would false-positive on the traceability
    // comment, so pin the leaked-guard shape precisely.
    assert!(
        !src.contains("&& _event.data.elapsed_ms === 0 ->"),
        "the raw ECMAScript guard must be fully lowered away (not leaked into the else arm)"
    );

    // Syntactic gate: compile the native-lowered SM against the runtime jar.
    let Some(kotlinc) = resolve_tool("kotlinc") else {
        eprintln!("SKIP statechart_native_lowering_kotlin compile: kotlinc not on PATH");
        return;
    };
    let Some(jar) = find_kotlin_runtime_jar() else {
        eprintln!(
            "SKIP statechart_native_lowering_kotlin compile: runtime jar missing under \
             sce-kotlin-runtime/build/libs (run `./gradlew :sce-kotlin-runtime:assemble`)",
        );
        return;
    };
    let classes = scratch.join("classes");
    std::fs::create_dir_all(&classes).expect("create classes dir");
    let output = Command::new(&kotlinc)
        .args(["-cp"])
        .arg(&jar)
        .args(["-d"])
        .arg(&classes)
        .arg("-nowarn")
        .arg(&kt_files[0])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc failed for the native-lowered Kotlin SM\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Python parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Python
// backend lowers the typed `_event.data.elapsed_ms` guard to a script-engine-
// free native comparison over the type-erased `EventMetadata.typed_payload`
// carrier: a per-event payload dataclass, a `_pending_<event>_payload` instance
// field, a `set_current_event` lift that `isinstance`-checks the carrier, a
// per-event module-level `raise_<event>` inject seam, and a
// `self._pending_<event>_payload is not None and (…)` guard — never the
// `self._guard(engine, …)` script path. `py_compile` is the syntactic gate; the
// substring assertions pin the lowering shape. The real run is the committed
// integration gate `sce-python-tests/integration/event_schema_native`
// (regen: scripts/regen_event_schema_native_python.sh).
#[test]
fn statechart_native_lowering_python_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("python_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("python", &out_dir, "statechart_minimal");

    let py_files = find_by_ext(&out_dir, "py");
    assert_eq!(py_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&py_files[0]).expect("read py");

    assert!(
        src.contains("class StatechartMinimalJobCompletedPayload:")
            && src.contains("    elapsed_ms: int"),
        "expected the per-event typed payload dataclass; got:\n{src}"
    );
    assert!(
        src.contains("self._pending_job_completed_payload = None"),
        "expected the instance-field reset (init + set_current_event); got:\n{src}"
    );
    assert!(
        src.contains("if isinstance(_tp, StatechartMinimalJobCompletedPayload):"),
        "expected the populate isinstance lift in set_current_event; got:\n{src}"
    );
    assert!(
        src.contains(
            "self._pending_job_completed_payload is not None and \
             (self._pending_job_completed_payload.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a module-level free function (the Python twin of
    // the Go `Raise<Event>` func — the runtime `Engine` is foreign) binding the
    // event name + payload field values in one call.
    assert!(
        src.contains("def raise_job_completed(engine, elapsed_ms: int) -> None:")
            && src.contains("typed_payload=StatechartMinimalJobCompletedPayload(elapsed_ms)"),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    assert!(
        !src.contains("self._guard(engine,"),
        "native lowering must not fall back to the script-engine guard"
    );

    // Syntactic gate: py_compile the native-lowered SM.
    let Some(python) = resolve_tool("python3").or_else(|| resolve_tool("python")) else {
        eprintln!("SKIP statechart_native_lowering_python compile: python3/python not on PATH");
        return;
    };
    let output = Command::new(&python)
        .args(["-m", "py_compile"])
        .arg(&py_files[0])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run python -m py_compile");
    assert!(
        output.status.success(),
        "py_compile failed for the native-lowered Python SM\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — C++ parity +
// real compile+run gate. The C++ backend lowers the typed
// `_event.data.elapsed_ms` guard to a script-engine-free native comparison
// over an `std::any`-carried typed payload: a `<Machine>PayloadTag` enum, a
// `pendingPayloadTag_` + `pending<Event>Payload_` policy channel, a
// `populateTypedPayload` any_cast hook, a per-event `raise<Event>` inject seam,
// and a `pendingPayloadTag_ == … && (…)` guard. Beyond the shape assertions
// this generates the SM and really compiles + runs it (header-only, NO runtime
// lib — the no-script-engine value path links zero, the C++ analogue of the
// C11 freestanding gate) to prove the typed inject → native guard → transition
// actually fires.
#[test]
fn statechart_native_lowering_cpp_compiles_and_runs() {
    let scratch = reset_scratch("cpp_native");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("cpp", &out_dir, "statechart_minimal");

    let h_files = find_by_ext(&out_dir, "h");
    assert_eq!(h_files.len(), 1, "expected exactly one generated header");
    let header = std::fs::read_to_string(&h_files[0]).expect("read h");
    let inl = std::fs::read_to_string(&find_by_ext(&out_dir, "inl")[0]).expect("read inl");

    assert!(
        header.contains("enum class StatechartMinimalPayloadTag"),
        "expected the per-document typed payload tag enum; got:\n{header}"
    );
    assert!(
        header.contains("void populateTypedPayload(const ::std::any &tp)")
            && header.contains("::std::any_cast<StatechartMinimalJobCompletedPayload>(&tp)"),
        "expected the any_cast populate hook; got:\n{header}"
    );
    assert!(
        header.contains("void raiseJobCompleted(uint32_t elapsed_ms)")
            && header.contains("m.typedPayload = StatechartMinimalJobCompletedPayload{"),
        "expected the per-event typed inject seam; got:\n{header}"
    );
    assert!(
        inl.contains("pendingPayloadTag_ == StatechartMinimalPayloadTag::JobCompleted &&")
            && inl.contains("(pendingJobCompletedPayload_.elapsed_ms == 0)"),
        "expected the native typed-payload guard; got:\n{inl}"
    );
    assert!(
        !inl.contains("safeEvaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The `// "…" lowered to …` comment echoes the raw cond, so use a precise
    // sentinel: the raw cond would only reach emitted code via the `{% else %}`
    // arm, i.e. `if (_event.data.elapsed_ms === 0) SCE_LIKELY {`.
    assert!(
        !inl.contains("if (_event.data.elapsed_ms === 0)"),
        "the raw ECMAScript guard must be fully lowered away"
    );

    // Real compile + run. The generated SM is header-only template code; a
    // typed-guard machine has needs_script_engine == false, so it links no
    // runtime (the MCU-relevant property, proven here for C++ too).
    let Some(gpp) = resolve_tool("g++") else {
        eprintln!("SKIP statechart_native_lowering_cpp compile: g++ not on PATH");
        return;
    };
    let driver = out_dir.join("driver.cpp");
    std::fs::write(
        &driver,
        r#"#include "statechart_minimal_sm.h"
#include <cstdio>
using SM = ::SCE::Generated::statechart_minimal::statechart_minimal;
int main() {
    SM sm; sm.initialize();
    if (sm.getCurrentState() != SM::State::Waiting) { std::puts("FAIL initial"); return 1; }
    sm.raiseJobCompleted(0);   // elapsed_ms == 0 satisfies the native typed guard
    sm.step();
    if (sm.getCurrentState() != SM::State::Done) { std::puts("FAIL fire"); return 2; }
    SM sm2; sm2.initialize();
    sm2.raiseJobCompleted(5);  // guard rejects -> stays put
    sm2.step();
    if (sm2.getCurrentState() != SM::State::Waiting) { std::puts("FAIL miss"); return 3; }
    std::puts("OK"); return 0;
}
"#,
    )
    .expect("write driver");
    let bin = out_dir.join("driver");
    let inc = repo_root().join("sce/include");
    let compile = Command::new(&gpp)
        .arg("-std=c++17")
        .arg("-I")
        .arg(&inc)
        .arg("-I")
        .arg(&out_dir)
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run g++");
    assert!(
        compile.status.success(),
        "header-only compile of the native-lowered C++ SM failed\nstderr: {}",
        String::from_utf8_lossy(&compile.stderr),
    );
    let run = Command::new(&bin).output().expect("run driver");
    assert!(
        run.status.success(),
        "native-lowered C++ SM run failed (exit {:?})\nstdout: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
    );
}

// NL→IR Item C1 Path A (EventSchema MCU native lowering, RFC §10.4 step
// 5) — C11 parity for `statechart_native_lowering_emits_engine_free_typed_guard`.
// The C11 backend must lower the typed `_event.data.elapsed_ms` guard to a
// native tagged-union comparison: a `<name>_payload_t` union, a
// `pending_payload` copy in the pop loop, a `raise_external_typed` inject
// seam, and a `tag ==` + field-comparison guard — never the Lua
// `lua_eval_guard` path (there is no script engine on an MCU). Beyond the
// shape assertions the generated translation unit must COMPILE under the
// strict MCU profile (`-std=c11 -Wall -Wextra -Wpedantic -Werror`) and,
// critically, under `-ffreestanding` — the value path's whole point is that
// it carries no hosted-libc / Lua dependency, so it builds for a bare-metal
// target. That freestanding compile is the A5 MCU acceptance gate.
#[test]
fn statechart_native_lowering_c11_compiles_freestanding() {
    let Some(cc) = resolve_tool("clang").or_else(|| resolve_tool("gcc")) else {
        eprintln!("SKIP statechart_native_lowering_c11: clang/gcc not on PATH");
        return;
    };
    let scratch = reset_scratch("c11_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("c11", &out_dir, "statechart_minimal");

    let c_files = find_by_ext(&out_dir, "c");
    assert_eq!(c_files.len(), 1, "expected exactly one generated C11 TU");
    let src = std::fs::read_to_string(&c_files[0]).expect("read .c");

    assert!(
        src.contains(
            "sm->pending_payload.tag == STATECHART_MINIMAL_PAYLOAD_JOB_COMPLETED \
             && (sm->pending_payload.as.job_completed.elapsed_ms == 0)"
        ),
        "expected the native tagged-union typed-payload guard; got:\n{src}"
    );
    assert!(
        src.contains(
            "statechart_minimal_raise_job_completed_typed(statechart_minimal_t *sm, \
             const statechart_minimal_job_completed_payload_t *payload)"
        ),
        "expected the per-event typed-payload inject seam (binds event+tag+member)"
    );
    assert!(
        !src.contains("lua_eval_guard"),
        "native lowering must not fall back to the Lua script-engine guard"
    );
    // `elapsed_ms ===` would only survive if the raw ECMAScript cond leaked
    // (`===` is not valid C). A precise sentinel — the `cond="…"` echo
    // comment uses `escape_c` and reads `elapsed_ms === 0` inside a quoted
    // string, so match the bare operator form the guard would emit.
    assert!(
        !src.contains("if (sm->pending_payload.as.job_completed.elapsed_ms === 0)")
            && !src.contains("lua_eval"),
        "the raw ECMAScript guard must be fully lowered away"
    );

    let runtime_inc = repo_root().join("sce-c-runtime/include");
    // Strict MCU profile + freestanding: the no-script-engine value path
    // must build with no hosted-libc assumption (A5 MCU gate).
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c11",
        "-ffreestanding",
        "-c",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror",
    ]);
    cmd.arg("-I").arg(&runtime_inc);
    cmd.arg("-I").arg(&out_dir);
    cmd.arg("-o").arg(out_dir.join("sm.o"));
    cmd.arg(&c_files[0]);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");
    assert!(
        output.status.success(),
        "freestanding -std=c11 compile of the native-lowered TU failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    // A5 north-star gate: a REAL bare-metal ARM cross-compile (Cortex-M4,
    // Thumb). The freestanding host compile above proves "no hosted-libc
    // calls"; this proves the value path actually builds for the MCU ISA
    // the watching-zenoh consumer targets. Skipped (not failed) when the
    // cross toolchain is absent, mirroring the host-compiler skips above —
    // CI installs `gcc-arm-none-eabi`.
    let Some(arm_cc) = resolve_tool("arm-none-eabi-gcc") else {
        eprintln!("SKIP arm-none-eabi cross-compile: arm-none-eabi-gcc not on PATH");
        return;
    };
    let mut arm = Command::new(&arm_cc);
    arm.args([
        "-mcpu=cortex-m4",
        "-mthumb",
        "-std=c11",
        "-ffreestanding",
        "-c",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror",
    ]);
    arm.arg("-I").arg(&runtime_inc);
    arm.arg("-I").arg(&out_dir);
    arm.arg("-o").arg(out_dir.join("sm.arm.o"));
    arm.arg(&c_files[0]);
    let arm_out = arm
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run arm-none-eabi-gcc");
    assert!(
        arm_out.status.success(),
        "arm-none-eabi (cortex-m4) cross-compile of the native-lowered TU failed\nstderr: {}",
        String::from_utf8_lossy(&arm_out.stderr),
    );
}

#[test]
fn smoke_go() {
    let Some(gofmt) = resolve_tool("gofmt") else {
        eprintln!("SKIP smoke_go: gofmt not on PATH");
        return;
    };
    let scratch = reset_scratch("go");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("go", &out_dir, fixture);
        let go_files = find_by_ext(&out_dir, "go");
        assert!(!go_files.is_empty(), "no .go emitted for {fixture}");
        for go in go_files {
            let output = Command::new(&gofmt)
                .arg("-e")
                .arg(&go)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .expect("run gofmt");
            assert!(
                output.status.success(),
                "gofmt -e failed for {fixture} at {}\nstderr: {}",
                go.display(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}

fn find_kotlin_runtime_jar() -> Option<PathBuf> {
    let libs = repo_root().join("sce-kotlin-runtime/build/libs");
    let entries = std::fs::read_dir(&libs).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("sce-kotlin-runtime-jvm-")
                && name.ends_with(".jar")
                && !name.ends_with("-sources.jar")
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

#[test]
fn smoke_kotlin() {
    let Some(kotlinc) = resolve_tool("kotlinc") else {
        eprintln!("SKIP smoke_kotlin: kotlinc not on PATH");
        return;
    };
    let Some(jar) = find_kotlin_runtime_jar() else {
        eprintln!(
            "SKIP smoke_kotlin: runtime jar missing under sce-kotlin-runtime/build/libs \
             (run `./gradlew :sce-kotlin-runtime:assemble`)",
        );
        return;
    };
    let scratch = reset_scratch("kotlin");

    let mut kt_paths: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("kotlin", &out_dir, fixture);
        kt_paths.extend(find_by_ext(&out_dir, "kt"));
    }
    assert!(!kt_paths.is_empty(), "no .kt files emitted");

    let classes = scratch.join("classes");
    std::fs::create_dir_all(&classes).expect("create classes dir");

    let mut cmd = Command::new(&kotlinc);
    cmd.args(["-cp"]).arg(&jar);
    cmd.args(["-d"]).arg(&classes);
    cmd.arg("-nowarn");
    for p in &kt_paths {
        cmd.arg(p);
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn smoke_python() {
    let Some(python) = resolve_tool("python3").or_else(|| resolve_tool("python")) else {
        eprintln!("SKIP smoke_python: python3/python not on PATH");
        return;
    };
    let scratch = reset_scratch("python");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("python", &out_dir, fixture);
        let py_files = find_by_ext(&out_dir, "py");
        assert!(!py_files.is_empty(), "no .py emitted for {fixture}");
        for py in py_files {
            let output = Command::new(&python)
                .args(["-m", "py_compile"])
                .arg(&py)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run python -m py_compile");
            assert!(
                output.status.success(),
                "py_compile failed for {fixture} at {}\nstdout: {}\nstderr: {}",
                py.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}

#[test]
fn smoke_c11() {
    let Some(cc) = resolve_tool("clang").or_else(|| resolve_tool("gcc")) else {
        eprintln!("SKIP smoke_c11: clang/gcc not on PATH");
        return;
    };
    let scratch = reset_scratch("c11");

    let mut headers: Vec<String> = Vec::new();
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("c11", &out_dir, fixture);
        include_dirs.push(out_dir.clone());
        for hdr in find_by_ext(&out_dir, "h") {
            if let Some(name) = hdr.file_name().and_then(|s| s.to_str()) {
                headers.push(name.to_string());
            }
        }
    }
    assert!(!headers.is_empty(), "no .h artefacts emitted");

    let stub = scratch.join("stub.c");
    let mut src = String::new();
    for h in &headers {
        src.push_str(&format!("#include \"{h}\"\n"));
    }
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&cc);
    cmd.args(["-std=c11", "-fsyntax-only", "-Wall", "-Wextra", "-Werror"]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");

    assert!(
        output.status.success(),
        "c11 compiler -fsyntax-only failed\nheaders: {headers:?}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
