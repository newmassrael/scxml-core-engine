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
    "sce_annotations",
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
/// test log alone. Inline `<invoke><content><scxml>` children are kept
/// in-memory by the parser and auto-emitted by codegen into `out_dir`
/// alongside the parent's `_sm.*` artefacts — no separate `--as-child`
/// pass per child is needed.
fn run_generate(lang: &str, out_dir: &Path, fixture: &str) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let fx_path = fixtures_dir().join(format!("{fixture}.scxml"));

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

/// Regression guard for `sce-codegen generate -l cpp
/// --cpp-namespace-prefix <NAME>`.
///
/// The flag nests the emitted machine namespace under
/// `SCE::Generated::<prefix>::<machine>`. The child-machine *declaration*
/// is rendered into the `.h` (state_machine.jinja2) while the matching
/// *definitions* — `make_shared` (invoke_methods.jinja2) and the
/// `using ChildEvent` send alias (actions/send.jinja2) — are rendered into
/// the `.inl`. The two render contexts are separate; an earlier revision
/// threaded the prefix into the `.h` context only, so a prefixed
/// invoke-bearing machine declared `shared_ptr<...prefix::child...>` but
/// defined `make_shared<...child...>` and failed to compile.
///
/// This test compiles a fixture that has both an inline `<invoke>` and a
/// targeted child `<send>`, once with the prefix and once without, and
/// asserts:
///   - prefixed output compiles (the core guard), and the prefixed
///     namespace reaches the `.inl` (not just the `.h`);
///   - unset output compiles and contains neither the prefix token nor a
///     stray empty `::::` segment (byte-shape preserved when unset).
#[test]
fn smoke_cpp_namespace_prefix() {
    const FIXTURE: &str = "namespace_prefix_invoke";
    const PREFIX: &str = "ScePrefixTest";

    let Some(gpp) = resolve_tool("g++") else {
        eprintln!("SKIP smoke_cpp_namespace_prefix: g++ not on PATH");
        return;
    };
    let scratch = reset_scratch("cpp_ns_prefix");
    let fx_path = fixtures_dir().join(format!("{FIXTURE}.scxml"));

    let include_roots: Vec<PathBuf> = vec![
        repo_root().join("sce/include"),
        repo_root().join("third_party/spdlog/include"),
        repo_root().join("third_party/nlohmann_json/include"),
    ];

    // Generate `FIXTURE` into a fresh sub-dir, optionally with the prefix
    // flag. Returns the out-dir so the caller can collect headers.
    let generate = |out_dir: &Path, prefix: Option<&str>| {
        std::fs::create_dir_all(out_dir).expect("create out dir");
        let mut cmd = Command::new(sce_codegen_bin());
        cmd.args(["generate", "-l", "cpp", "-o"]).arg(out_dir);
        if let Some(p) = prefix {
            cmd.args(["--cpp-namespace-prefix", p]);
        }
        cmd.arg(&fx_path);
        let output = cmd.output().expect("spawn sce-codegen");
        assert!(
            output.status.success(),
            "sce-codegen generate (prefix={prefix:?}) failed (exit {:?})\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        );
    };

    // Compile every `_sm.h` in `out_dir` as one translation unit. The
    // inline-content child is co-emitted as a template sibling, so the
    // whole suite is self-contained. Returns the g++ stderr on failure.
    let compile = |out_dir: &Path| -> Result<(), String> {
        let headers: Vec<String> = find_by_ext(out_dir, "h")
            .into_iter()
            .filter_map(|h| h.file_name().and_then(|s| s.to_str()).map(String::from))
            .filter(|n| n.ends_with("_sm.h"))
            .collect();
        assert!(
            !headers.is_empty(),
            "no _sm.h artefacts under {}",
            out_dir.display(),
        );
        let stub = out_dir.join("stub.cpp");
        let src: String = headers
            .iter()
            .map(|h| format!("#include \"{h}\"\n"))
            .collect();
        std::fs::write(&stub, &src).expect("write stub");

        let mut cmd = Command::new(&gpp);
        cmd.args(["-std=c++20", "-fsyntax-only"]);
        for dir in &include_roots {
            cmd.arg("-I").arg(dir);
        }
        cmd.arg("-I").arg(out_dir);
        cmd.arg(&stub).stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = cmd.output().expect("run g++");
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    };

    // Concatenate every generated body of one extension under a dir.
    // Kept per-extension so the `.inl` (definitions) can be asserted on
    // independently of the `.h` (declarations) — the bug was the prefix
    // reaching the latter but not the former.
    let read_ext = |out_dir: &Path, ext: &str| -> String {
        find_by_ext(out_dir, ext)
            .iter()
            .map(|f| std::fs::read_to_string(f).expect("read source"))
            .collect()
    };

    // --- prefixed: the core regression guard -------------------------
    let pref_dir = scratch.join("prefixed");
    generate(&pref_dir, Some(PREFIX));
    if let Err(stderr) = compile(&pref_dir) {
        panic!("prefixed g++ -fsyntax-only failed (the decl/def namespace bug):\n{stderr}");
    }
    // The prefixed namespace must reach the `.inl` definitions, not only
    // the `.h` declarations. The qualified name may be wrapped across
    // lines by the formatter, so assert on the namespace token alone
    // rather than a contiguous `make_shared<...>` span. `make_shared` and
    // `ChildEvent` confirm the two distinct definition sites are present
    // in this fixture; the `PREFIX::` token confirms both are nested.
    let pref_inl = read_ext(&pref_dir, "inl");
    let ns_token = format!("SCE::Generated::{PREFIX}::");
    assert!(
        pref_inl.contains("make_shared") && pref_inl.contains(&ns_token),
        "prefixed make_shared (.inl) does not carry the namespace prefix {PREFIX:?}",
    );
    assert!(
        pref_inl.contains("using ChildEvent") && pref_inl.contains(&ns_token),
        "prefixed ChildEvent send alias (.inl) does not carry the namespace prefix {PREFIX:?}",
    );

    // --- unset: byte-shape preserved ---------------------------------
    let unset_dir = scratch.join("unset");
    generate(&unset_dir, None);
    if let Err(stderr) = compile(&unset_dir) {
        panic!("unset g++ -fsyntax-only failed:\n{stderr}");
    }
    let unset_src: String = [read_ext(&unset_dir, "h"), read_ext(&unset_dir, "inl")].concat();
    assert!(
        !unset_src.contains(PREFIX),
        "unset output unexpectedly contains the prefix token {PREFIX:?}",
    );
    assert!(
        !unset_src.contains("::::"),
        "unset output has a stray empty `::::` namespace segment",
    );
}

/// Regression guard for `sce-codegen generate -l c11 --c-symbol-prefix
/// <NAME>` — the C11 peer of `smoke_cpp_namespace_prefix`.
///
/// C has no namespace, so every emitted symbol is `<machine>_…`. The suite
/// prefix nests each symbol (struct tag, enum/macro names, and child refs)
/// across the `.h`/`.c` split. The two C templates render from separate
/// contexts, so a prefix that reaches one but not the other diverges the
/// declaration from the definition and fails to link/compile. The output
/// FILENAMES and SCE-MAP markers must keep the logical (un-prefixed) name —
/// the prefix nests symbols, not files.
///
/// Generates the shared invoke+child-send fixture with and without the
/// prefix, compiles each translation unit, and asserts:
///   - prefixed `.c` carries the prefixed child symbol (the bug site), and
///     the child `#include` filename is NOT prefixed;
///   - unset output compiles and contains neither the prefix token nor a
///     stray double-underscore artefact at the symbol root.
#[test]
fn smoke_c_symbol_prefix() {
    const FIXTURE: &str = "namespace_prefix_invoke";
    const PREFIX: &str = "ScePrefixTest";

    let Some(gcc) = resolve_tool("gcc").or_else(|| resolve_tool("cc")) else {
        eprintln!("SKIP smoke_c_symbol_prefix: gcc/cc not on PATH");
        return;
    };
    let scratch = reset_scratch("c_sym_prefix");
    let fx_path = fixtures_dir().join(format!("{FIXTURE}.scxml"));
    let runtime_inc = repo_root().join("backends/c/runtime/include");

    let generate = |out_dir: &Path, prefix: Option<&str>| {
        std::fs::create_dir_all(out_dir).expect("create out dir");
        let mut cmd = Command::new(sce_codegen_bin());
        cmd.args(["generate", "-l", "c11", "-o"]).arg(out_dir);
        if let Some(p) = prefix {
            cmd.args(["--c-symbol-prefix", p]);
        }
        cmd.arg(&fx_path);
        let output = cmd.output().expect("spawn sce-codegen");
        assert!(
            output.status.success(),
            "sce-codegen generate -l c11 (prefix={prefix:?}) failed (exit {:?})\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        );
    };

    // Compile every generated `.c` translation unit with `-fsyntax-only`.
    // The inline-content child is co-emitted, so the suite is self-contained.
    let compile = |out_dir: &Path| -> Result<(), String> {
        let units = find_by_ext(out_dir, "c");
        assert!(
            !units.is_empty(),
            "no .c emitted under {}",
            out_dir.display()
        );
        for c in &units {
            let mut cmd = Command::new(&gcc);
            cmd.args(["-std=c11", "-fsyntax-only"]);
            cmd.arg("-I").arg(&runtime_inc);
            cmd.arg("-I").arg(out_dir);
            cmd.arg(c).stdout(Stdio::piped()).stderr(Stdio::piped());
            let output = cmd.output().expect("run gcc");
            if !output.status.success() {
                return Err(format!(
                    "{}: {}",
                    c.display(),
                    String::from_utf8_lossy(&output.stderr),
                ));
            }
        }
        Ok(())
    };

    let read_ext = |out_dir: &Path, ext: &str| -> String {
        find_by_ext(out_dir, ext)
            .iter()
            .map(|f| std::fs::read_to_string(f).expect("read source"))
            .collect()
    };

    // --- prefixed: the core regression guard -------------------------
    let pref_dir = scratch.join("prefixed");
    generate(&pref_dir, Some(PREFIX));
    if let Err(stderr) = compile(&pref_dir) {
        panic!("prefixed gcc -fsyntax-only failed (the decl/def symbol bug):\n{stderr}");
    }
    let pref_src = read_ext(&pref_dir, "c") + &read_ext(&pref_dir, "h");
    assert!(
        pref_src.contains(&format!("{PREFIX}_")),
        "prefixed C output carries no `{PREFIX}_` symbol prefix",
    );
    // The child `#include` references a stem-derived filename, never the
    // symbol prefix — guard the exact bug fixed in state_machine.c.jinja2.
    assert!(
        !pref_src.contains(&format!("#include \"{PREFIX}_")),
        "an `#include` filename was wrongly symbol-prefixed",
    );

    // --- unset: byte-shape preserved ---------------------------------
    let unset_dir = scratch.join("unset");
    generate(&unset_dir, None);
    if let Err(stderr) = compile(&unset_dir) {
        panic!("unset gcc -fsyntax-only failed:\n{stderr}");
    }
    let unset_src = read_ext(&unset_dir, "c") + &read_ext(&unset_dir, "h");
    assert!(
        !unset_src.contains(PREFIX),
        "unset C output unexpectedly contains the prefix token {PREFIX:?}",
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
    let libs = repo_root().join("backends/kotlin/runtime/build/libs");
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
            "SKIP smoke_kotlin: runtime jar missing under backends/kotlin/runtime/build/libs \
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
