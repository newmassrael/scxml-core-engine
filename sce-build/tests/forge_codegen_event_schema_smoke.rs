// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Item C1 Path A Atomic 4 — per-backend EventSchema codegen
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
