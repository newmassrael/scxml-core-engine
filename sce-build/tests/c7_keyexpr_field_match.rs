//! RFC c7-wildcard W-project — bounded-string element-field → borrowed
//! `bytes`-view call-site projection, parity across all 6 backends.
//!
//! Per `claudedocs/rfc-c7-wildcard-keyexpr-expressibility.md` Q-W-5 (a) +
//! Q-W-8 W-project lock: an algorithm iterating a bounded-collection
//! whose element-type carries a bounded-string field (`keyexpr_entry`'s
//! `pattern`) projects that field to each backend's borrowed byte-view
//! idiom when it flows into a `bytes` parameter — here W-index's
//! `bytes_equal(a: bytes, b: bytes)`. The second argument `target` is the
//! outer's own `bytes` param (already a borrowed view) and passes through
//! unprojected, exercising both argument forms that reach one `bytes`
//! parameter list.
//!
//! Fixtures (in `tests/forge/resources/`):
//! - `keyexpr_entry.scxml`         — element-type codec, `pattern` string.
//! - `local_keyexpr_table.scxml`   — BC over `keyexpr_entry`.
//! - `algorithm_bytes_equal.scxml` — W-index inner byte-compare.
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
    assert!(
        code.contains("[]byte(entry.pattern)"),
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
            Language::Go => "[]byte(entry.pattern)",
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

    let runtime_inc = repo_root().join("sce-forge-runtime/c/include");
    let output = Command::new(&cc)
        .args(["-std=c11", "-c", "-Wall", "-Wextra", "-Werror"])
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
