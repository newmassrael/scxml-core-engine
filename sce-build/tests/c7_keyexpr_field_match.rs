//! Item C7 wildcard keyexpr — bounded-string element-field → borrowed
//! `bytes`-view call-site projection, parity across all 6 backends.
//!
//! Locked projection semantics: an algorithm iterating a bounded-collection
//! whose element-type carries a bounded-string field (`keyexpr_entry`'s
//! `pattern`) projects that field to each backend's borrowed byte-view
//! idiom when it flows into a `bytes` parameter — here the inner
//! byte-compare algorithm's `bytes_equal(a: bytes, b: bytes)`. The second argument `target` is the
//! outer's own `bytes` param (already a borrowed view) and passes through
//! unprojected, exercising both argument forms that reach one `bytes`
//! parameter list.
//!
//! Fixtures (in `tests/forge/resources/`):
//! - `keyexpr_entry.scxml`         — element-type codec, `pattern` string.
//! - `local_keyexpr_table.scxml`   — BC over `keyexpr_entry`.
//! - `algorithm_bytes_equal.scxml` — inner byte-compare algorithm.
//! - `algorithm_keyexpr_field_match.scxml` — outer, projects the field.
//!
//! Drives the same orchestrator path (`compile_scxml_with_imports`) as
//! `c7_keyexpr_fixture.rs`, because the element-type field schema that
//! drives the projection is resolved from `element_type_candidates` —
//! only the multi-doc orchestrator carries it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

const GO_PREFIX: &str = "github.com/acme/project/generated";

fn template_dir(lang: Language) -> PathBuf {
    sce_build::find_template_dir_for(lang)
}

fn resource_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("forge")
        .join("resources")
        .join(name)
}

fn copy_resource_into(dir: &Path, name: &str) -> PathBuf {
    let src = resource_path(name);
    let content =
        fs::read_to_string(&src).unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
    let dest = dir.join(name);
    fs::write(&dest, content).expect("write resource");
    dest
}

fn options_for(lang: Language) -> ForgeCompileOptions {
    let mut opts = ForgeCompileOptions::default();
    if matches!(lang, Language::Go) {
        opts.go_module_prefix = Some(GO_PREFIX.to_string());
    }
    opts
}

fn compile_field_match_for(lang: Language) -> String {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "keyexpr_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_keyexpr_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_field_match.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(lang),
        lang,
        &options_for(lang),
        None,
    )
    .expect("orchestrator codegen succeeds");
    extract_field_match(&outputs)
}

fn extract_field_match(outputs: &[(String, GeneratedOutput)]) -> String {
    outputs
        .iter()
        .find(|(name, _)| name == "algorithm_keyexpr_field_match.scxml")
        .expect("field_match output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ── Per-backend projected dispatch — exact call-site form ──────────

#[test]
fn rust_field_match_projects_string_field_to_byte_slice() {
    let code = compile_field_match_for(Language::Rust);
    // Identity SSOT: the imported module is named from the algorithm's
    // `name=` attribute (`bytes_equal`), not its file stem
    // (`algorithm_bytes_equal`) — `generate_forge` writes the module as
    // `bytes_equal.rs` and the cross-doc `use super::bytes_equal;` resolves it.
    assert!(
        code.contains("bytes_equal::bytes_equal(entry.pattern.as_bytes(), target)"),
        "Rust string-field projection missing; got:\n{code}"
    );
}

#[test]
fn c11_field_match_projects_string_field_to_byte_view() {
    let code = compile_field_match_for(Language::C11);
    // C7 §A6: the cross-algorithm call is the imported algorithm's bare
    // canonical symbol (`bytes_equal`), not the file-stem-prefixed
    // `algorithm_bytes_equal_bytes_equal` — C11 has no namespace and the
    // algorithm kind names its emitted symbol by the `name=` attribute, so
    // the bare call resolves against the bare `static inline` definition.
    assert!(
        code.contains(
            "bytes_equal((sce_forge_bytes_view_t){ (const uint8_t *)entry.pattern, entry.pattern_len }, target)"
        ),
        "C11 string-field projection missing; got:\n{code}"
    );
}

#[test]
fn cpp_field_match_projects_string_field_to_span() {
    let code = compile_field_match_for(Language::Cpp);
    // Identity SSOT: the imported namespace is named from the algorithm's
    // `name=` attribute (`SCE::Generated::BytesEqual`), not its file stem
    // (`AlgorithmBytesEqual`) — `generate_forge` emits the definition in
    // `namespace SCE::Generated::BytesEqual` and the cross-doc call qualifies
    // against it.
    assert!(
        code.contains(
            "BytesEqual::bytes_equal(std::span<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>(entry.pattern.data()), entry.pattern.size()), target)"
        ),
        "Cpp string-field projection missing; got:\n{code}"
    );
}

#[test]
fn go_field_match_projects_string_field_to_byte_slice() {
    let code = compile_field_match_for(Language::Go);
    // Go exports struct fields in PascalCase (`codec_field_id` SSOT), so the
    // element-field read binds against `entry.Pattern`, not `entry.pattern`.
    assert!(
        code.contains("[]byte(entry.Pattern)"),
        "Go string-field projection missing; got:\n{code}"
    );
}

#[test]
fn python_field_match_projects_string_field_to_bytes() {
    let code = compile_field_match_for(Language::Python);
    // Identity SSOT: the imported module is named from the algorithm's
    // `name=` attribute (`bytes_equal`), not its file stem
    // (`algorithm_bytes_equal`) — `from . import bytes_equal` resolves it.
    assert!(
        code.contains("bytes_equal.bytes_equal(entry.pattern.encode(\"utf-8\"), target)"),
        "Python string-field projection missing; got:\n{code}"
    );
}

#[test]
fn kotlin_field_match_projects_string_field_to_byte_array() {
    let code = compile_field_match_for(Language::Kotlin);
    assert!(
        code.contains("entry.pattern.toByteArray(Charsets.UTF_8)"),
        "Kotlin string-field projection missing; got:\n{code}"
    );
}

// ── Drift guard — every backend projects on the field, passes the
//    view-typed `target` through unprojected ─────────────────────────

#[test]
fn field_match_projects_on_all_six_backends() {
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let code = compile_field_match_for(lang);
        // The bounded-string element field is projected to a byte view.
        let projection_marker = match lang {
            Language::Rust => "entry.pattern.as_bytes()",
            Language::C11 => "(const uint8_t *)entry.pattern, entry.pattern_len",
            Language::Cpp => "reinterpret_cast<const std::uint8_t*>(entry.pattern.data())",
            Language::Go => "[]byte(entry.Pattern)",
            Language::Python => "entry.pattern.encode(\"utf-8\")",
            Language::Kotlin => "entry.pattern.toByteArray(Charsets.UTF_8)",
        };
        assert!(
            code.contains(projection_marker),
            "{lang:?}: string-field projection `{projection_marker}` missing; got:\n{code}"
        );
    }
}

// ── §A6 multi-file C11 compile gate ───────────────────────────────
//
// The substring tests above pin the projected call-site form; this gate
// proves the *whole* generated multi-file C11 program compiles clean.
// It exercises the three cross-doc codegen fixes the keyexpr matcher
// depends on (RFC c7-wildcard §A6), each of which produced non-compiling
// C before:
//   (a) one `#include` per line — the import loop previously concatenated
//       `#include "a.h"#include "b.h"static inline…` onto a single line;
//   (b) the bare cross-algorithm call `bytes_equal(...)` resolving against
//       the bare `static inline` definition — was `<file_stem>_<fn>`, a
//       dangling symbol;
//   (c) the codec string-field encode casting `char*` → `const uint8_t*`
//       — was a `-Werror=pointer-sign` failure;
// plus the name-based `#include "bytes_equal.h"` matching the header
// filename `generate_forge` actually emits (named by the algorithm's
// `name=` attribute, not its file stem).

fn resolve_tool(name: &str) -> Option<PathBuf> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

#[test]
fn c11_field_match_multi_file_compiles_werror() {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "keyexpr_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_keyexpr_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_field_match.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(Language::C11),
        Language::C11,
        &options_for(Language::C11),
        None,
    )
    .expect("orchestrator codegen succeeds");

    // Lay every generated header into one directory so the name-based
    // `#include`s resolve against the files actually emitted.
    let out_dir = dir.path().join("c11_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(out_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let Some(cc) = resolve_tool("gcc").or_else(|| resolve_tool("clang")) else {
        eprintln!("SKIP c11_field_match_multi_file_compiles_werror: gcc/clang not on PATH");
        return;
    };

    let driver = out_dir.join("drive_field_match.c");
    fs::write(
        &driver,
        "#include \"keyexpr_field_match.h\"\nint main(void) { return 0; }\n",
    )
    .expect("write driver");

    let runtime_inc = repo_root().join("backends/c/forge-runtime/include");
    // Tier 1 header tree. Generated forge C headers reach into it for
    // `<sce/portability.h>` (`SCE_STATIC_ASSERT`, the one spelling of a
    // compile-time assertion valid in both C and C++) and, for pool
    // headers, `<sce/sample.h>`. `sce_forge_runtime_c` names the same
    // path in its INTERFACE include directories; this test drives the
    // compiler directly, so it states the dependency itself.
    let core_runtime_inc = repo_root().join("backends/c/runtime/include");
    let output = Command::new(&cc)
        .args(["-std=c11", "-c", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&runtime_inc)
        .arg("-I")
        .arg(&core_runtime_inc)
        .arg("-I")
        .arg(&out_dir)
        .arg("-o")
        .arg(out_dir.join("drive_field_match.o"))
        .arg(&driver)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");
    assert!(
        output.status.success(),
        "multi-file C11 compile of the keyexpr field-match program failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── Cross-doc identity-SSOT real-compile gates (g++ + rustc) ──────
//
// The field_match fixture above drags in the bounded-collection + codec
// emit, whose C++ output carries pre-existing, identity-unrelated defects
// (a missing static `capacity()`, a designated-initializer in a static
// `decode`, unused-parameter stubs) that are out of scope for the
// cross-doc identity fix and tracked separately. To verify the identity
// SSOT itself on a real compiler — that the name-based `#include` / `use`
// resolves and the qualified call binds against the imported algorithm's
// emitted symbol — these gates compile the smallest cross-doc shape:
// `algorithm_xdoc_compile` (`name="xdoc_eq_caller"`) importing
// `algorithm_bytes_equal` (`name="bytes_equal"`). Both documents carry a
// `name=` that diverges from their file stem, so a stem-keyed reference
// would dangle. The program is std-only (free functions, no BC/codec), so
// the gates need only g++ (C++20) / rustc.

#[test]
fn cpp_cross_doc_algorithm_compiles_werror() {
    let dir = tempdir().expect("tempdir");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_xdoc_compile.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[inner.as_path(), outer.as_path()],
        &template_dir(Language::Cpp),
        Language::Cpp,
        &options_for(Language::Cpp),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let out_dir = dir.path().join("cpp_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(out_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let Some(cxx) = resolve_tool("g++").or_else(|| resolve_tool("clang++")) else {
        eprintln!("SKIP cpp_cross_doc_algorithm_compiles_werror: g++/clang++ not on PATH");
        return;
    };

    let driver = out_dir.join("drive_xdoc.cpp");
    fs::write(
        &driver,
        "#include \"xdoc_eq_caller.h\"\nint main() { return 0; }\n",
    )
    .expect("write driver");

    let output = Command::new(&cxx)
        .args(["-std=c++20", "-c", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&out_dir)
        .arg("-o")
        .arg(out_dir.join("drive_xdoc.o"))
        .arg(&driver)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c++ compiler");
    assert!(
        output.status.success(),
        "C++ compile of the cross-doc algorithm program failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn rust_cross_doc_algorithm_compiles() {
    let dir = tempdir().expect("tempdir");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_xdoc_compile.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[inner.as_path(), outer.as_path()],
        &template_dir(Language::Rust),
        Language::Rust,
        &options_for(Language::Rust),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let Some(rustc) = resolve_tool("rustc") else {
        eprintln!("SKIP rust_cross_doc_algorithm_compiles: rustc not on PATH");
        return;
    };

    let out_dir = dir.path().join("rust_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let mut modules = Vec::new();
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(out_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
            // The primary module files are `<name>.rs`; skip the
            // `<name>_test.rs` test-vector sidecars (they carry `#[test]`
            // harness fns the lib crate does not need to resolve identity).
            if let Some(stem) = filename.strip_suffix(".rs") {
                if !stem.ends_with("_test") {
                    modules.push(stem.to_string());
                }
            }
        }
    }
    // Assemble a crate root declaring every emitted module so the
    // cross-doc `use super::<module>;` resolves against the crate root.
    let lib: String = modules.iter().map(|m| format!("pub mod {m};\n")).collect();
    let lib_rs = out_dir.join("lib.rs");
    fs::write(&lib_rs, lib).expect("write lib.rs");

    let output = Command::new(&rustc)
        .args(["--edition", "2021", "--crate-type=lib"])
        .arg("-o")
        .arg(out_dir.join("libxdoc.rlib"))
        .arg(&lib_rs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run rustc");
    assert!(
        output.status.success(),
        "rustc compile of the cross-doc algorithm crate failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── Cross-doc identity-SSOT real-compile gates (Go / Kotlin / Python) ─
//
// W1-supplement: the C11/Cpp/Rust cross-doc gates above prove the name-based
// import resolves and the qualified call binds on a real compiler. Go, Kotlin
// and Python were previously substring-only (the `*_projects_*` assertions),
// the one place a symbol-name drift could pass SCE-green yet break a consumer
// build. These gates close that gap with the same minimal std-only
// `algorithm_xdoc_compile` (`name="xdoc_eq_caller"`) → `algorithm_bytes_equal`
// (`name="bytes_equal"`) shape, both names diverging from their file stem.

#[test]
fn go_cross_doc_algorithm_compiles() {
    let dir = tempdir().expect("tempdir");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_xdoc_compile.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[inner.as_path(), outer.as_path()],
        &template_dir(Language::Go),
        Language::Go,
        &options_for(Language::Go),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let Some(go) = resolve_tool("go") else {
        eprintln!("SKIP go_cross_doc_algorithm_compiles: go not on PATH");
        return;
    };

    let out_dir = dir.path().join("go_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    fs::write(
        out_dir.join("go.mod"),
        format!("module {GO_PREFIX}\n\ngo 1.22\n"),
    )
    .expect("write go.mod");
    // Go packages live one-per-directory matching their import path
    // (`<prefix>/<package>`); the generated file is the flat `<package>.go`,
    // so place each into `<out>/<package>/`. A `<package>_test.go` sidecar
    // (none here — the fixture carries no test vectors) would share its
    // primary's directory.
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            let base = filename.strip_suffix(".go").expect("go file");
            let pkg = base.strip_suffix("_test").unwrap_or(base);
            let pkg_dir = out_dir.join(pkg);
            fs::create_dir_all(&pkg_dir).expect("create pkg dir");
            fs::write(pkg_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let output = Command::new(&go)
        .args(["build", "./..."])
        .current_dir(&out_dir)
        .env("GOFLAGS", "-mod=mod")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run go build");
    assert!(
        output.status.success(),
        "go build of the cross-doc algorithm module failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn kotlin_cross_doc_algorithm_compiles() {
    let dir = tempdir().expect("tempdir");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_xdoc_compile.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[inner.as_path(), outer.as_path()],
        &template_dir(Language::Kotlin),
        Language::Kotlin,
        &options_for(Language::Kotlin),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let Some(kotlinc) = resolve_tool("kotlinc") else {
        eprintln!("SKIP kotlin_cross_doc_algorithm_compiles: kotlinc not on PATH");
        return;
    };

    let out_dir = dir.path().join("kt_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    // Kotlin resolves the cross-package wildcard import within a single
    // compilation, so all primary `.kt` files compile together. Skip any
    // `*TestVectors.kt` sidecar (none here) — it needs the kotlin-test dep.
    let mut sources = Vec::new();
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            if filename.ends_with("TestVectors.kt") {
                continue;
            }
            let path = out_dir.join(filename);
            fs::write(&path, content).unwrap_or_else(|e| panic!("write {filename}: {e}"));
            sources.push(path);
        }
    }

    let output = Command::new(&kotlinc)
        .args(&sources)
        .arg("-d")
        .arg(out_dir.join("xdoc.jar"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc compile of the cross-doc algorithm sources failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn python_cross_doc_algorithm_imports() {
    let dir = tempdir().expect("tempdir");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_xdoc_compile.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[inner.as_path(), outer.as_path()],
        &template_dir(Language::Python),
        Language::Python,
        &options_for(Language::Python),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let Some(python) = resolve_tool("python3").or_else(|| resolve_tool("python")) else {
        eprintln!("SKIP python_cross_doc_algorithm_imports: python3 not on PATH");
        return;
    };

    // Python is interpreted; the cross-doc `from . import bytes_equal` only
    // resolves inside a package, so lay both modules into one package dir and
    // *import + call* the caller. Importing runs the relative import (binds
    // the sibling module); calling resolves `bytes_equal.bytes_equal` — a
    // bad cross-doc symbol would surface as an AttributeError at the call.
    let out_dir = dir.path().join("py_out");
    let pkg_dir = out_dir.join("xdoc_pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("__init__.py"), "").expect("write __init__.py");
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            if filename.ends_with("_test.py") {
                continue;
            }
            fs::write(pkg_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let output = Command::new(&python)
        .arg("-c")
        .arg("from xdoc_pkg import xdoc_eq_caller as m; m.xdoc_eq_caller(b'a', b'a')")
        .current_dir(&out_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run python3");
    assert!(
        output.status.success(),
        "python import+call of the cross-doc algorithm package failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── Full C++ field-match compile gate (BC + codec + algorithm) ────
//
// The cross-doc gate above isolates identity on a std-only shape; this
// gate compiles the *whole* field-match program — the bounded-collection
// (`LocalKeyexprTable`), the element codec (`keyexpr_entry`), and both
// algorithm headers — so the C++ BC/codec emit is held to the same
// real-compiler bar the C11 program already meets. It exercises the W2
// emit fixes the matcher depends on: the BC `capacity()` static accessor,
// the codec `decode` reading fields off `out` rather than free locals in
// a designated initializer, and the `cursor`/writer parameters being used
// (no `-Werror=unused-parameter`).
#[test]
fn cpp_field_match_multi_file_compiles_werror() {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "keyexpr_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_keyexpr_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_field_match.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(Language::Cpp),
        Language::Cpp,
        &options_for(Language::Cpp),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let out_dir = dir.path().join("cpp_field_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(out_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let Some(cxx) = resolve_tool("g++").or_else(|| resolve_tool("clang++")) else {
        eprintln!("SKIP cpp_field_match_multi_file_compiles_werror: g++/clang++ not on PATH");
        return;
    };

    let driver = out_dir.join("drive_field_match.cpp");
    fs::write(
        &driver,
        "#include \"keyexpr_field_match.h\"\nint main() { return 0; }\n",
    )
    .expect("write driver");

    let runtime_inc = repo_root().join("backends/cpp/forge-runtime/include");
    let output = Command::new(&cxx)
        .args(["-std=c++20", "-c", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&runtime_inc)
        .arg("-I")
        .arg(&out_dir)
        .arg("-o")
        .arg(out_dir.join("drive_field_match.o"))
        .arg(&driver)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c++ compiler");
    assert!(
        output.status.success(),
        "multi-file C++ compile of the keyexpr field-match program failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── Full Rust field-match compile gate (no-alloc, real cargo build) ──
//
// RFC c7-wildcard: a bounded-collection is an owned, self-contained,
// no-alloc container, so it stores the element codec's owned mirror —
// which for a borrowed element (a `String` / `Bytes` field) is now the
// no-alloc bounded-inline `{Element}Owned` (`heapless::String<N>`), not
// the borrowed `{Element}<'a>` view (whose lifetime would otherwise infect
// the whole collection). This gate compiles the *whole* field-match
// program — element codec, bounded-collection, and both algorithms —
// against the real `sce-forge-runtime` with DEFAULT features (no `alloc`),
// proving the collection over a bounded-string element is genuinely
// no-alloc and self-contained, at C11 parity. The orchestrator path feeds
// the element-type field schema that drives the borrowed-element →
// owned-mirror storage decision (single-file `generate` cannot).
#[test]
fn rust_field_match_multi_file_compiles() {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "keyexpr_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_keyexpr_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_field_match.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(Language::Rust),
        Language::Rust,
        &options_for(Language::Rust),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let Some(cargo) = resolve_tool("cargo") else {
        eprintln!("SKIP rust_field_match_multi_file_compiles: cargo not on PATH");
        return;
    };

    // A throwaway crate that depends on the real runtime by path and
    // declares every emitted primary module at the crate root, so the
    // cross-doc `use super::<module>;` references resolve. Detached from
    // the parent workspace via an empty `[workspace]` table.
    let crate_dir = dir.path().join("kxfm");
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let mut modules = Vec::new();
    for (_doc, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(src_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
            if let Some(stem) = filename.strip_suffix(".rs") {
                if !stem.ends_with("_test") {
                    modules.push(stem.to_string());
                }
            }
        }
    }
    let lib_rs: String = modules.iter().map(|m| format!("pub mod {m};\n")).collect();
    fs::write(src_dir.join("lib.rs"), lib_rs).expect("write lib.rs");

    let runtime_path = repo_root().join("backends/rust/forge-runtime");
    let cargo_toml = format!(
        "[package]\n\
         name = \"kxfm_compile_gate\"\n\
         version = \"0.0.0\"\n\
         edition = \"2021\"\n\
         \n\
         [lib]\n\
         path = \"src/lib.rs\"\n\
         \n\
         [dependencies]\n\
         sce-forge-runtime = {{ path = {runtime:?} }}\n\
         \n\
         [workspace]\n",
        runtime = runtime_path.to_string_lossy(),
    );
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).expect("write Cargo.toml");

    // Isolated target dir so this build never contends with the parent
    // `cargo test` invocation's target lock. DEFAULT features → no `alloc`.
    let output = Command::new(&cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", dir.path().join("target"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run cargo build");
    assert!(
        output.status.success(),
        "no-alloc cargo build of the keyexpr field-match crate failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── Full field-match compile gates (BC + codec + algorithm) for
//    Go / Kotlin / Python ────────────────────────────────────────────
//
// W1-supplement #1: the C11/Cpp/Rust gates above already compile the
// *whole* field-match program (bounded-collection + element codec + both
// algorithms) on a real compiler. Go, Kotlin and Python had only the
// std-only cross-doc gate, which carries no BC/codec emit — so their
// bounded-collection + codec code generators had never been held to a
// real compiler at all. These three gates close that gap at full parity.
// (The Kotlin gate first surfaced a `const val` mask initialised with a
// runtime `shl` — rejected by kotlinc, root-fixed to the C11/Go literal
// form in the BC template.)

fn orchestrate_field_match(lang: Language) -> (tempfile::TempDir, Vec<(String, GeneratedOutput)>) {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "keyexpr_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_keyexpr_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_bytes_equal.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_field_match.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(lang),
        lang,
        &options_for(lang),
        None,
    )
    .expect("orchestrator codegen succeeds");
    (dir, outputs)
}

#[test]
fn go_field_match_multi_file_compiles() {
    let (dir, outputs) = orchestrate_field_match(Language::Go);

    let Some(go) = resolve_tool("go") else {
        eprintln!("SKIP go_field_match_multi_file_compiles: go not on PATH");
        return;
    };

    // The element codec imports the Go runtime
    // (`github.com/newmassrael/sce-forge-runtime/codec`), so the throwaway
    // module pulls it in by a `replace` directive onto the in-repo runtime.
    let out_dir = dir.path().join("go_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let runtime_go = repo_root().join("backends/go/forge-runtime");
    fs::write(
        out_dir.join("go.mod"),
        format!(
            "module {GO_PREFIX}\n\n\
             go 1.22\n\n\
             require github.com/newmassrael/sce-forge-runtime v0.0.0\n\n\
             replace github.com/newmassrael/sce-forge-runtime => {}\n",
            runtime_go.to_string_lossy(),
        ),
    )
    .expect("write go.mod");
    // One package per directory matching the import path `<prefix>/<package>`;
    // a `<package>_test.go` sidecar shares its primary's directory.
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            let base = filename.strip_suffix(".go").expect("go file");
            let pkg = base.strip_suffix("_test").unwrap_or(base);
            let pkg_dir = out_dir.join(pkg);
            fs::create_dir_all(&pkg_dir).expect("create pkg dir");
            fs::write(pkg_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let output = Command::new(&go)
        .args(["build", "./..."])
        .current_dir(&out_dir)
        .env("GOFLAGS", "-mod=mod")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run go build");
    assert!(
        output.status.success(),
        "go build of the full keyexpr field-match module failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn kotlin_field_match_multi_file_compiles() {
    let (dir, outputs) = orchestrate_field_match(Language::Kotlin);

    let Some(kotlinc) = resolve_tool("kotlinc") else {
        eprintln!("SKIP kotlin_field_match_multi_file_compiles: kotlinc not on PATH");
        return;
    };

    // The element codec imports `com.sce.forge.runtime.{SceCursor, SceSink,
    // CodecError, MutableListSink}`, all defined in the runtime's single
    // `Codec.kt`; compile it alongside the generated sources. Skip any
    // `*TestVectors.kt` sidecar (it needs the kotlin-test dependency).
    let out_dir = dir.path().join("kt_out");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let mut sources = Vec::new();
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            if filename.ends_with("TestVectors.kt") {
                continue;
            }
            let path = out_dir.join(filename);
            fs::write(&path, content).unwrap_or_else(|e| panic!("write {filename}: {e}"));
            sources.push(path);
        }
    }
    sources.push(repo_root().join(
        "backends/kotlin/forge-runtime/src/commonMain/kotlin/com/sce/forge/runtime/Codec.kt",
    ));

    let output = Command::new(&kotlinc)
        .args(&sources)
        .arg("-d")
        .arg(out_dir.join("field_match.jar"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc compile of the full keyexpr field-match sources failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn python_field_match_multi_file_imports_and_calls() {
    let (dir, outputs) = orchestrate_field_match(Language::Python);

    let Some(python) = resolve_tool("python3").or_else(|| resolve_tool("python")) else {
        eprintln!("SKIP python_field_match_multi_file_imports_and_calls: python3 not on PATH");
        return;
    };

    // Lay every module into one package; the element codec imports the
    // `sce_forge_runtime` package, supplied on PYTHONPATH. Python is
    // interpreted, so importing the field_match module only binds its
    // top-level relative imports — the projected call site
    // (`bytes_equal.bytes_equal(entry.pattern.encode("utf-8"), target)`)
    // lives inside the function body. To exercise it, insert one entry and
    // *call* `keyexpr_field_match`: a bad cross-doc symbol or a miscompiled
    // BC/codec surfaces as an AttributeError / assertion at the call.
    let out_dir = dir.path().join("py_out");
    let pkg_dir = out_dir.join("fmpkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("__init__.py"), "").expect("write __init__.py");
    for (_src, generated) in &outputs {
        for (filename, content) in &generated.files {
            if filename.ends_with("_test.py") {
                continue;
            }
            fs::write(pkg_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        }
    }

    let runtime_py = repo_root().join("backends/python/forge-runtime");
    let driver = "\
from fmpkg import keyexpr_field_match as m\n\
from fmpkg.local_keyexpr_table import LocalKeyexprTable\n\
from fmpkg.keyexpr_entry import KeyexprEntry\n\
t = LocalKeyexprTable()\n\
t.insert(KeyexprEntry(pattern_len=1, pattern='x'))\n\
assert m.keyexpr_field_match(b'x', t) == 0, 'expected match hit'\n\
assert m.keyexpr_field_match(b'zz', t) == 0xFFFF, 'expected miss sentinel'\n";

    let output = Command::new(&python)
        .arg("-c")
        .arg(driver)
        .current_dir(&out_dir)
        .env("PYTHONPATH", &runtime_py)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run python3");
    assert!(
        output.status.success(),
        "python import+call of the full keyexpr field-match package failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
