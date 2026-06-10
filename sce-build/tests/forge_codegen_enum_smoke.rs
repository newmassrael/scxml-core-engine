// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Item C1 Path A — per-backend Enum codegen smoke gate.
//
// Generates each of the 3 positive Enum fixtures under
// `tests/fixtures/enum/` on all 6 backends and feeds the output to a
// per-backend syntax checker (g++, syn::parse_file, gofmt -e, kotlinc,
// py_compile, clang -fsyntax-only). The contract is structural — no
// runtime correctness assertion — but every backend's generated header
// MUST compile per `feedback_byte_goldens_not_compile` + the design
// RFC §5.2 acceptance gate ("6/6 Enum backends produce compilable
// code per per-backend harness").
//
// Toolchain-missing behaviour mirrors `codegen_smoke.rs`: detect each
// compiler with `which` and skip with `eprintln!` if absent. CI is
// expected to install every backend's toolchain; local dev machines
// without one toolchain should not be blocked.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Positive Enum fixtures from `tests/fixtures/enum/`. Each varies one
/// axis per `feedback_codegen_fixture_orthogonal_axes`:
///   * `enum_minimal`     — uint8 underlying, 3 variants, contiguous
///   * `enum_wide`        — uint16 underlying, variants requiring 16-bit
///   * `enum_hex_values`  — uint8 underlying, hex-notation values
///
/// Negative fixtures live alongside but are exercised by
/// `tests/enum_kind.rs` for parse-time rejection, not by this smoke
/// gate (they never reach codegen).
const FIXTURES: &[&str] = &["enum_minimal", "enum_wide", "enum_hex_values"];

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/enum")
}

fn scratch_for(lang: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_enum_smoke");
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
/// Go imports; enum fixtures import nothing, but the flag must still be
/// supplied because the CLI validates it up-front).
fn run_generate(lang: &str, out_dir: &Path, fixture: &str) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let fx_path = fixtures_dir().join(format!("{fixture}.scxml"));

    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args(["generate", "-l", lang, "-o"]).arg(out_dir);
    if lang == "go" {
        cmd.args(["--go-module-prefix", "example.com/sce-forge/enum-smoke"]);
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
    // syn::parse_file runs in-process — already a sce-build dev dep
    // via the existing codegen_smoke.rs harness.
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
    // Enum templates ship `@OptIn(ExperimentalUnsignedTypes::class)`
    // so a runtime jar with the unsigned-type opt-in available is
    // required for kotlinc to accept the source. The existing
    // sce-kotlin-runtime jar carries the right Kotlin stdlib version.
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
            // `python -m py_compile <file>` syntax-checks + bytecode-
            // compiles without executing the module. Exits non-zero on
            // SyntaxError.
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
    // Prefer clang for strict-C11 conformance checking; fall back to
    // gcc when clang is unavailable.
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
    // C11 requires at least one declaration in a TU when -Wempty-translation-unit
    // is enabled — emit a dummy extern so the file isn't structurally empty.
    src.push_str("extern int sce_forge_enum_smoke_stub;\n");
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c11",
        "-fsyntax-only",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
    ]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run cc");
    assert!(
        output.status.success(),
        "C11 -fsyntax-only failed\nheaders: {headers:?}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
