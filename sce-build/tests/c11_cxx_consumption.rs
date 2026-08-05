// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! C++ consumption contract for the C11 backend's headers.
//!
//! Every header the C11 backend publishes, and every header it
//! generates, wraps its declarations in `#ifdef __cplusplus extern "C"`.
//! That wrapper states a contract — the same state machine an MCU
//! compiles as C11 is linkable from a C++ application — and nothing
//! checked it until this suite.
//!
//! It was not being met. `_Static_assert` is a C11 keyword with no C++
//! spelling, and it appeared in `sce/sample.h` and in every generated
//! `_sm.h`. A C++ translation unit including any of them failed to
//! parse: GCC rejected them outright, Clang accepted them as an
//! extension and rejected them again under `-Werror`. The headers now
//! spell the assertion through `SCE_STATIC_ASSERT`
//! (`backends/c/runtime/include/sce/portability.h`), which lowers to
//! `static_assert` in C++ and `_Static_assert` in C11.
//!
//! Why this lives here rather than only in CTest: the CTest gate
//! (`c11_headers_are_cxx_consumable`) scans the built tree and covers
//! more headers, but the C11 backend has no CI job, so nothing runs it
//! on push. These tests carry no `required-features`, so they run under
//! plain `cargo test --workspace` — which is what pre-push stage 4 and
//! `rust-workspace-tests.yml` both execute. The two gates overlap on
//! purpose; this one is the copy that runs unattended.
//!
//! Compiling each header as its own translation unit is deliberate. A
//! header that only parses after some other header was included first
//! is not consumable on its own terms, and self-containedness is part
//! of what `extern "C"` promises here.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::compile_scxml_lang;
use sce_build::generator::Language;
use tempfile::tempdir;

/// Locate a C++ driver.
///
/// Deliberately not skip-on-absent. A skip reads as a pass, and this
/// repository cannot be built at all without a C++ compiler — so an
/// absent one is an environment fault worth failing on, not a reason to
/// stop checking the contract.
fn cxx_compiler() -> String {
    for name in ["c++", "g++", "clang++"] {
        if Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
        {
            return name.to_string();
        }
    }
    panic!(
        "no C++ compiler found (tried c++, g++, clang++). This suite pins the \
         `extern \"C\"` contract the C11 headers publish; without a compiler it \
         cannot be checked, and reporting a pass would claim coverage that did \
         not happen."
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn core_include_dir() -> PathBuf {
    repo_root().join("backends/c/runtime/include")
}

fn forge_include_dir() -> PathBuf {
    repo_root().join("backends/c/forge-runtime/include")
}

/// Compile one header as a standalone C++17 translation unit.
///
/// The header is included by absolute path so the check lands on this
/// exact file rather than whichever same-named header the include path
/// happens to resolve first.
fn compile_header_as_cxx17(header: &Path, include_dirs: &[PathBuf]) -> Result<(), String> {
    let dir = tempdir().expect("tempdir");
    let tu = dir.path().join("tu.cpp");
    std::fs::write(
        &tu,
        format!(
            "#include \"{}\"\nint main() {{ return 0; }}\n",
            header.display()
        ),
    )
    .expect("write translation unit");

    let mut cmd = Command::new(cxx_compiler());
    cmd.arg("-std=c++17").arg("-fsyntax-only");
    for inc in include_dirs {
        cmd.arg(format!("-I{}", inc.display()));
    }
    cmd.arg(&tu);

    let out = cmd.output().expect("run C++ compiler");
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

fn headers_in(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "h"))
        .collect();
    found.sort();
    found
}

/// Every hand-written public header compiles standalone as C++17.
///
/// The directory is walked rather than listing files, so a header added
/// later is covered without anyone remembering to register it. An empty
/// directory fails instead of passing over an empty set.
#[test]
fn published_c_headers_compile_as_cxx17() {
    let roots = [
        core_include_dir().join("sce"),
        forge_include_dir().join("sce/forge"),
    ];
    let include_dirs = [core_include_dir(), forge_include_dir()];

    let mut checked = 0usize;
    let mut failures = Vec::new();
    for root in &roots {
        let headers = headers_in(root);
        assert!(
            !headers.is_empty(),
            "no headers under {} — the gate cannot report a pass over an empty set",
            root.display()
        );
        for header in headers {
            checked += 1;
            if let Err(stderr) = compile_header_as_cxx17(&header, &include_dirs) {
                failures.push(format!("{}\n{stderr}", header.display()));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {checked} published header(s) do not compile as C++17, though each \
         opens `extern \"C\"` and so claims to be consumable from C++:\n---\n{}",
        failures.len(),
        failures.join("\n---\n")
    );
}

/// A generated C11 state-machine header compiles standalone as C++17.
///
/// This is the shape that broke: `state_machine.h.jinja2` emitted three
/// `_Static_assert` invariants below an `extern "C"` block, so every one
/// of the generated `_sm.h` headers was unparseable as C++.
#[test]
fn generated_c11_state_machine_header_compiles_as_cxx17() {
    let dir = tempdir().expect("tempdir");
    let scxml_path = dir.path().join("cxx_consumption_probe.scxml");
    std::fs::write(
        &scxml_path,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s0" name="cxx_consumption_probe">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>"##,
    )
    .expect("write probe scxml");

    let template_dir = sce_build::find_template_dir_for(Language::C11);
    let output = compile_scxml_lang(
        scxml_path.to_str().expect("utf-8 path"),
        &template_dir,
        Language::C11,
    )
    .expect("C11 codegen succeeds for the probe machine");

    let (header_name, header_body) = output
        .files
        .iter()
        .find(|(name, _)| name.ends_with(".h"))
        .expect("C11 codegen emits a header");

    // The macro, not the keyword — this is the regression that made
    // every generated header unparseable as C++.
    assert!(
        header_body.contains("SCE_STATIC_ASSERT("),
        "generated header lost its compile-time invariants:\n{header_body}"
    );
    assert!(
        !header_body.contains("_Static_assert("),
        "generated header writes the C11 keyword directly; it opens `extern \"C\"` \
         and so must spell the assertion through SCE_STATIC_ASSERT:\n{header_body}"
    );

    let staged = dir.path().join(header_name);
    if let Some(parent) = staged.parent() {
        std::fs::create_dir_all(parent).expect("create output dir");
    }
    std::fs::write(&staged, header_body).expect("stage generated header");

    let include_dirs = [core_include_dir(), dir.path().to_path_buf()];
    if let Err(stderr) = compile_header_as_cxx17(&staged, &include_dirs) {
        panic!(
            "generated C11 state-machine header does not compile as C++17, though it \
             opens `extern \"C\"`:\n{stderr}"
        );
    }
}

/// `SCE_STATIC_ASSERT` diagnoses a false condition in both languages.
///
/// A macro that expands to nothing would satisfy every "compiles as
/// C++" check above while silently dropping the layout invariants it
/// replaced. Compiling a deliberately false assertion is what
/// distinguishes a working lowering from an inert one.
#[test]
fn static_assert_macro_rejects_a_false_condition_in_both_languages() {
    let dir = tempdir().expect("tempdir");
    let body = "#include <sce/portability.h>\n\
                SCE_STATIC_ASSERT(sizeof(char) == 99, \"deliberately false\");\n\
                int main(void) { return 0; }\n";

    let cases: [(&str, &str, &str); 2] = [
        ("C++17", "probe.cpp", "-std=c++17"),
        ("C11", "probe.c", "-std=c11"),
    ];
    for (label, filename, std_flag) in cases {
        let src = dir.path().join(filename);
        std::fs::write(&src, body).expect("write probe");
        let out = Command::new(cxx_compiler())
            .arg("-x")
            .arg(if filename.ends_with(".c") { "c" } else { "c++" })
            .arg(std_flag)
            .arg("-fsyntax-only")
            .arg(format!("-I{}", core_include_dir().display()))
            .arg(&src)
            .output()
            .expect("run compiler");
        assert!(
            !out.status.success(),
            "SCE_STATIC_ASSERT accepted a false condition under {label} — the macro \
             is inert there, so every invariant written through it is unchecked"
        );
    }
}
