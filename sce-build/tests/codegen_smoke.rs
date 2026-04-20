// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Codegen structural smoke validator.
//
// Each fixture under `tests/fixtures/codegen_smoke/` is generated for
// the four backends (C++, Rust, Go, Kotlin) and fed to a language-
// native syntax checker. The goal is not behavioural correctness — the
// W3C harnesses in sce-rust-tests, sce-kotlin-tests, sce-go-tests, and
// `w3c_test_cli` already own that. This harness catches *structural*
// template bugs (brace imbalance, missing delimiters, stray tokens)
// whose cross-products are absent from the W3C corpus.
//
// The motivating case is commit d6ca6b19: a top-level `<final>` with
// a `<donedata>` literal `<content>` compiled to C++ with mismatched
// braces for months, hidden because no W3C test exercised that shape.
// `final_top_literal.scxml` is the regression guard for that bug.
//
// Toolchain-missing behaviour: each test detects its compiler with
// `which` and skips with an `eprintln!` if absent. CI is expected to
// install all four; local dev machines without one toolchain should
// not be blocked. The skip is logged so silence-versus-pass is
// distinguishable when running with `--nocapture`.
//
// See `memory/codegen_template_strategy.md` for the rationale against
// an AST emitter migration — this harness is the chosen gate instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Ordered list of fixture basenames (no `.scxml` extension). Matches
/// the files in `tests/fixtures/codegen_smoke/`. Adding a regression
/// fixture is a single-line change here plus a new SCXML file.
const FIXTURES: &[&str] = &[
    "degenerate_minimal",
    "final_nested_literal",
    "final_top_expr",
    "final_top_literal",
    "final_top_params",
    "history_deep_shallow",
    "invoke_inline_content",
    "parallel_final",
];

/// Path to the `sce-codegen` binary built for this integration test.
///
/// `CARGO_BIN_EXE_<bin>` is populated by cargo when a test target
/// references a binary from the same crate. The `required-features =
/// ["cli"]` entry on this `[[test]]` stanza ensures the binary is
/// built before the test runs.
fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Workspace root — the parent of `sce-build/`. Used to locate C++
/// include directories and the Kotlin runtime jar without hardcoding
/// an absolute path.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codegen_smoke")
}

/// Scratch directory under cargo's per-test tmpdir. Dropping the
/// enclosing test cleans it up when the test process exits.
fn scratch_for(lang: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("codegen_smoke");
    base.join(lang)
}

/// Resolve a tool on `PATH` via `which`. Returns `None` when the tool
/// is missing so the caller can skip rather than fail.
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

/// Run `sce-codegen generate -l <lang> -o <out> <fixture>.scxml`.
/// Panics with both stdout and stderr on non-zero exit so failures in
/// the generator (rather than the checker) are diagnosable from the
/// test log alone. After the parent generates, any `.scxml` files
/// emitted alongside it (inline `<invoke><content>…</content></invoke>`
/// children are extracted as sibling .scxml artefacts by sce-codegen)
/// are regenerated with `--as-child` so the parent's `#include`/`use`/
/// `import` of the child SM type resolves during syntax check.
///
/// The fixture is copied into `out_dir` before generation because
/// sce-codegen's parser writes extracted inline `<content>` children
/// to the source SCXML's *containing directory* (not `-o`). Generating
/// from a scratch-local copy keeps `tests/fixtures/codegen_smoke/`
/// clean of derived artefacts.
fn run_generate(lang: &str, out_dir: &Path, fixture: &str) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let fx_src = fixtures_dir().join(format!("{fixture}.scxml"));
    let fx_path = out_dir.join(format!("{fixture}.scxml"));
    std::fs::copy(&fx_src, &fx_path).expect("copy fixture into scratch");

    let output = Command::new(sce_codegen_bin())
        .args(["generate", "-l", lang, "-o"])
        .arg(out_dir)
        .arg(&fx_path)
        .output()
        .expect("spawn sce-codegen");

    assert!(
        output.status.success(),
        "sce-codegen generate -l {lang} {fixture} failed (exit {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Extracted inline-child SCXMLs need a second generate pass with
    // `--as-child`. Corpus depth is one level (parent → child); if a
    // future fixture nests deeper this loop would miss grandchildren,
    // but the fixture contract (minimal SCXML, one specific template
    // path per fixture) discourages that.
    //
    // `--parent-stem` pins the child's Kotlin/Go package to the
    // parent's, matching the layout `generate-w3c`'s process_child
    // produces. Without it the parent's unqualified reference to the
    // child `StateMachine` class fails to resolve under kotlinc.
    //
    // The parent fixture itself now lives in `out_dir` (copied above)
    // so exclude its stem from the child-regen loop; only truly-
    // extracted children feed `--as-child`.
    for child_scxml in find_by_ext(out_dir, "scxml") {
        if child_scxml.file_stem().and_then(|s| s.to_str()) == Some(fixture) {
            continue;
        }
        let child_out = Command::new(sce_codegen_bin())
            .args(["generate", "--as-child", "--parent-stem", fixture, "-l", lang, "-o"])
            .arg(out_dir)
            .arg(&child_scxml)
            .output()
            .expect("spawn sce-codegen for extracted child");
        assert!(
            child_out.status.success(),
            "sce-codegen generate --as-child -l {lang} for {child_scxml:?} failed (exit {:?})\nstdout: {}\nstderr: {}",
            child_out.status.code(),
            String::from_utf8_lossy(&child_out.stdout),
            String::from_utf8_lossy(&child_out.stderr),
        );
    }
}

/// Return paths in `dir` with the given extension, non-recursive.
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

/// Fresh scratch directory for the given language.
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

    let mut include_dirs: Vec<PathBuf> = vec![
        repo_root().join("sce/include"),
        repo_root().join("third_party/spdlog/include"),
        repo_root().join("third_party/nlohmann_json/include"),
    ];
    let mut headers: Vec<String> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("cpp", &out_dir, fixture);
        include_dirs.push(out_dir.clone());
        for hdr in find_by_ext(&out_dir, "h") {
            if let Some(name) = hdr.file_name().and_then(|s| s.to_str()) {
                if name.ends_with("_sm.h") {
                    headers.push(name.to_string());
                }
            }
        }
    }
    assert!(
        !headers.is_empty(),
        "no _sm.h artefacts discovered under {}",
        scratch.display(),
    );

    // A translation unit including every generated header catches
    // brace/declaration drift across the whole corpus in one g++
    // invocation. Wrapping the headers in a stub .cpp avoids the
    // "pragma once in main file" warning that `-fsyntax-only` on the
    // header directly would emit under GCC.
    let stub = scratch.join("stub.cpp");
    let mut src = String::new();
    for h in &headers {
        src.push_str(&format!("#include \"{h}\"\n"));
    }
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&gpp);
    cmd.args(["-std=c++20", "-fsyntax-only"]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().expect("run g++");

    assert!(
        output.status.success(),
        "g++ -fsyntax-only failed\nheaders: {:?}\nstderr: {}",
        headers,
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn smoke_rust() {
    // `syn::parse_file` runs in-process — no external toolchain, so
    // this test has no skip path. `syn` is a sce-build dev-dependency
    // already relied on by `forge_conformance`.
    let scratch = reset_scratch("rust");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("rust", &out_dir, fixture);
        let rs_files = find_by_ext(&out_dir, "rs");
        assert!(
            !rs_files.is_empty(),
            "no .rs emitted for {fixture} under {}",
            out_dir.display(),
        );
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
        assert!(
            !go_files.is_empty(),
            "no .go emitted for {fixture} under {}",
            out_dir.display(),
        );
        for go in go_files {
            // `gofmt -e` reports every parse error (not just the first)
            // and exits non-zero when any are found. Import resolution
            // is not attempted, so no go.mod plumbing is needed.
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

/// Find the runtime classpath jar produced by `:sce-kotlin-runtime:assemble`.
/// Version is discovered rather than hardcoded so bumping the module
/// version does not silently break this test.
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

    // Batch every fixture into one kotlinc invocation — JVM startup
    // dominates the per-fixture cost, so one invocation with six files
    // runs ~5x faster than six sequential invocations.
    let classes = scratch.join("classes");
    std::fs::create_dir_all(&classes).expect("create classes dir");

    let mut cmd = Command::new(&kotlinc);
    cmd.args(["-cp"]).arg(&jar);
    cmd.args(["-d"]).arg(&classes);
    cmd.arg("-nowarn");
    for p in &kt_paths {
        cmd.arg(p);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd.output().expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
