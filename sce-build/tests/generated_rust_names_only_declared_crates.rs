// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Generated Rust may only name crates the consumer was told to declare.
//
// A generated machine is compiled inside *someone else's* crate, so every
// crate name it emits becomes a line that consumer's `Cargo.toml` must
// carry. The runtime therefore re-exports what emitted code needs
// (`sce_rust_runtime::log`, `sce_forge_runtime::heapless`) and templates
// reach through the runtime instead of naming the crate directly — one
// dependency edge, one pinned version, and no contract that lives only in
// the templates.
//
// The Rust datamodel error paths broke that rule: every `<assign>`,
// `<data>` init, guard evaluation and `<log>` emitted a bare `log::error!`.
// `log` resolves in the *calling* crate, so a consumer compiling a machine
// with any non-`null` datamodel got a wall of `E0433: unresolved module or
// unlinked crate `log`` from code they never wrote — reported from the
// field as PINION-PR86 (42 errors on one 14-state machine).
//
// Nothing upstream failed, because the two crates that compile generated
// Rust in this repo — `sce-rust-tests` and the `c6` bounded-collection
// probe — both declared the crate the templates leaked. A consumer's
// manifest is the only place the gap is visible, so the second test here
// builds a crate that declares `sce-rust-runtime` and nothing else.
//
// The field report attributed its own long innocence to the `datamodel`
// attribute — its other two machines are `datamodel="null"` and never hit
// this. Measured against this pin, that is not the mechanism: the
// attribute does not reach the decision at all. `null`, `ecmascript`,
// `xpath` and an invented string all produce the same
// `needs_script_engine: true` and the same emitted sites. What decides is
// whether the document carries expressions — a `<data expr=…>`, a
// transition `cond`, an `<assign>`. A `datamodel="null"` document with any
// of those breaks a consumer exactly as an `ecmascript` one does, so the
// fixture below is chosen for its expressions, not for its datamodel
// string.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// Crate roots a generated machine may name.
///
/// `sce_rust_runtime` is the machine's declared runtime. `core` / `std` /
/// `alloc` ship with the toolchain. `sce_link_runtime` is admitted on a
/// different ground than a re-export would give: it appears in the
/// `…LinkRx` trait's *public* signatures (`Sample<'_, M>`), which the
/// consumer instantiates from their own link driver — they must name the
/// crate to make the call at all, so the edge is one they already have.
/// That is the discriminator for this list: a crate the consumer writes
/// themselves is a dependency; a crate that appears only inside an
/// expansion they never see is a leak.
const ALLOWED_CRATE_ROOTS: &[&str] = &[
    "sce_rust_runtime",
    "sce_link_runtime",
    "core",
    "std",
    "alloc",
];

/// Path roots that are not crates: local items and keywords.
const NON_CRATE_ROOTS: &[&str] = &["crate", "self", "super", "Self"];

/// Registered tool namespaces. `clippy::style` inside `#![allow(…)]` is a
/// lint path — rustc resolves it against the tool list, never against the
/// extern prelude, so it links nothing and costs the consumer no manifest
/// line.
const TOOL_NAMESPACES: &[&str] = &["clippy", "rustdoc", "rustfmt"];

/// Exercises every Rust template that carries a diagnostic log site:
/// `<data>` init, `<assign>`, guard `cond`, `<log>`, `<script>`,
/// `<foreach>`, and `<send>` with a param. Each of those is an expression
/// the script-engine tier has to evaluate, and that tier is what emits the
/// sites — the `datamodel` string names the language a consumer believes
/// they are writing, and reaches no decision here.
const LOGGING_FIXTURE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript" name="depgate">
  <datamodel>
    <data id="budget" expr="40"/>
    <data id="seen" expr="0"/>
    <data id="items" expr="[1,2,3]"/>
  </datamodel>
  <state id="s1">
    <onentry>
      <log label="entering" expr="budget"/>
      <script>seen = 0</script>
      <assign location="seen" expr="seen + 1"/>
      <foreach array="items" item="it" index="ix">
        <assign location="seen" expr="seen + it"/>
      </foreach>
      <send event="ping" target="#_internal">
        <param name="count" expr="seen"/>
      </send>
    </onentry>
    <transition event="ping" cond="seen &lt; budget" target="s2"/>
    <transition event="ping" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"##;

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Generate the fixture and return the machine's Rust source.
fn generate_machine(dir: &Path) -> String {
    let scxml = dir.join("depgate.scxml");
    fs::write(&scxml, LOGGING_FIXTURE).expect("write fixture");

    let outputs = compile_scxml_with_imports(
        &[scxml.as_path()],
        &[],
        &template_dir(),
        Language::Rust,
        &ForgeCompileOptions::default(),
        None,
    )
    .expect("codegen succeeds");

    outputs
        .iter()
        .flat_map(|(_doc, generated)| generated.files.iter())
        .find(|(name, _)| name.ends_with("_sm.rs"))
        .map(|(_, content)| content.clone())
        .expect("a machine source was emitted")
}

/// Strip `//` line comments and `/* */` blocks.
///
/// Generated Rust carries prose that names crates it does not link —
/// `heapless::Vec<S, N>` in the `SceString` doc comment, `log::debug!` in
/// the facade's own description. Scanning raw source reports those as
/// dependencies and the gate becomes noise, so comments come out before
/// anything is counted. String literals stay in: their content is emitted
/// diagnostic text, and a `::` inside one has never been a path.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut block_depth = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        let next = bytes.get(i + 1).copied();
        if block_depth > 0 {
            if c == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
                continue;
            }
            if c == '*' && next == Some('/') {
                block_depth -= 1;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_string {
            if c == '\\' {
                out.push(c);
                if let Some(n) = next {
                    out.push(n);
                }
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && next == Some('/') {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            block_depth = 1;
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Collect the root segment of every `a::b` path in `src`, paired with the
/// line it sits on.
///
/// A root is an identifier followed by `::` that is not itself preceded by
/// `::` or by an identifier character — i.e. the leftmost segment. Both
/// `::log::error!` and `log::error!` yield `log`.
fn crate_roots(src: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (lineno, line) in src.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if !(chars[i].is_ascii_alphabetic() || chars[i] == '_') {
                i += 1;
                continue;
            }
            // Reject a segment that is not leftmost: preceded by an
            // identifier char, by `::`, or by `.` (method chain).
            let prev = if i == 0 { None } else { Some(chars[i - 1]) };
            let prev_is_ident = prev.is_some_and(|p| p.is_alphanumeric() || p == '_');
            let prev_is_colon = prev == Some(':');
            let prev_is_dot = prev == Some('.');
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            if prev_is_ident || prev_is_colon || prev_is_dot {
                continue;
            }
            if chars.get(i) == Some(&':') && chars.get(i + 1) == Some(&':') {
                found.push((lineno + 1, chars[start..i].iter().collect::<String>()));
            }
        }
    }
    found
}

#[test]
fn generated_rust_names_no_undeclared_crate() {
    let dir = tempdir().expect("tempdir");
    let source = generate_machine(dir.path());
    let code = strip_comments(&source);

    // Lower bound. A template edit that simply stops emitting diagnostics
    // would satisfy an "no bad crate names" assertion vacuously, so pin
    // that this fixture really did reach the sites under test. Seven
    // constructs above carry a log site; the count is well under what the
    // fixture emits today and only trips if emission collapses.
    let facade_calls = code.matches("sce_log_").count();
    assert!(
        facade_calls >= 7,
        "fixture reached only {facade_calls} logging sites — it no longer \
         exercises the datamodel error paths, so this gate proves nothing",
    );

    let mut violations: Vec<String> = Vec::new();
    for (lineno, root) in crate_roots(&code) {
        if ALLOWED_CRATE_ROOTS.contains(&root.as_str())
            || NON_CRATE_ROOTS.contains(&root.as_str())
            || TOOL_NAMESPACES.contains(&root.as_str())
        {
            continue;
        }
        // A root that opens with an uppercase letter is a type, trait or
        // enum — `String::new()`, `Vec::new()`, `Duration::from_millis`.
        // Cargo crate names are lowercase, so an uppercase root can never
        // be the extern-prelude name this gate is looking for.
        if root.starts_with(|c: char| c.is_uppercase()) {
            continue;
        }
        // Anything the generated file defines itself (modules, types,
        // enums) is a local path, not a crate.
        if code.contains(&format!("mod {root}"))
            || code.contains(&format!("struct {root}"))
            || code.contains(&format!("enum {root}"))
        {
            continue;
        }
        let line = code.lines().nth(lineno - 1).unwrap_or("").trim();
        violations.push(format!("  line {lineno}: `{root}::` in `{line}`"));
    }

    assert!(
        violations.is_empty(),
        "generated Rust names {} crate(s) a consumer's manifest was never \
         told to declare. Emitted code must reach these through the \
         runtime's re-export (`::sce_rust_runtime::…`), which every \
         consumer already depends on:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

#[test]
fn a_consumer_declaring_only_the_runtime_compiles_a_datamodel_machine() {
    let dir = tempdir().expect("tempdir");
    let source = generate_machine(dir.path());

    let crate_dir = dir.path().join("lone_dep_consumer");
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    // The generated machine opens with inner attributes (`#![allow(…)]`),
    // so it is a crate root as emitted — the same shape the `nostd-build`
    // probe compiles, and the shape a consumer gets from `include!` at the
    // top of their own module.
    fs::write(src_dir.join("lib.rs"), &source).expect("write lib.rs");

    let runtime_path = repo_root().join("backends/rust/runtime");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\n\
             name = \"lone_dep_consumer\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             publish = false\n\
             \n\
             [lib]\n\
             path = \"src/lib.rs\"\n\
             \n\
             # Exactly one entry, on purpose. Every crate the generated\n\
             # machine reaches has to arrive through this edge; adding a\n\
             # second line here to make the build pass would delete the\n\
             # only thing this crate proves.\n\
             [dependencies]\n\
             sce-rust-runtime = {{ path = {runtime:?} }}\n\
             \n\
             [workspace]\n",
            runtime = runtime_path.to_string_lossy(),
        ),
    )
    .expect("write Cargo.toml");

    // Shared across runs so the runtime's dependency tree is built once
    // rather than per invocation; separate from the outer build's target
    // dir so the two cargo processes never contend for the same lock.
    let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lone-dep-consumer-target");

    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("run cargo build");

    assert!(
        output.status.success(),
        "a crate declaring only `sce-rust-runtime` must compile a\n\
         `datamodel=\"ecmascript\"` machine. It does not, which means the\n\
         generated code reaches a crate the consumer was never told to\n\
         declare:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
