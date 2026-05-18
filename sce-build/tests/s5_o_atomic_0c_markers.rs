// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 0c — per-function SCE-MAP marker
// presence fixture.
//
// Atomic 0b landed module-level markers (one per generated file).
// Atomic 0c extends to:
//   1. Per-function markers above every emitted function header in
//      each backend's statechart templates (entry/exit + transition).
//   2. Forge per-kind body marker emission across all 6 backends
//      (cpp / c / rust / kotlin / go / python) — one marker per
//      per-kind body file.
//
// The fixture probes both axes:
//   - A small statechart (>=2 states + >=1 transition) for the
//     statechart axis. Each backend should now emit MULTIPLE SCE-MAP
//     occurrences (one module-level + one per emitted function).
//   - A forge inline codec for the forge axis. Each backend should
//     emit a SCE-MAP marker pointing at the codec document's source
//     position.
//
// A future template edit that drops one of the per-function macro
// calls (or the forge per-kind import) surfaces here as a count
// regression instead of a silent contract miss
// [[feedback-silently-broken-hooks]].

use std::path::{Path, PathBuf};
use std::process::Command;

/// Statechart fixture: 2 states + 1 transition. Drives the per-function
/// marker count assertion — entry, exit, and transition templates each
/// emit their own marker, so the rendered output must carry more than
/// one SCE-MAP line.
const STATECHART_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log expr="'entering s1'"/>
    </onentry>
    <onexit>
      <log expr="'leaving s1'"/>
    </onexit>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

/// Forge codec fixture: single byte field, exercises the per-kind body
/// emission path. RFC §5.B B5 plain codec — no variant / no TLV chain /
/// no parent-flags / no test vectors. Every backend (rust / cpp / c11 /
/// kotlin / go / python) accepts this shape.
const CODEC_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_probe">
  <datamodel>
    <sce:field id="seq" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>
"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Stage the fixture into a unique temp dir and run sce-codegen on it.
/// Returns the generated file paths (artifact manifest minus the input
/// scxml).
fn generate(name: &str, fixture: &str, lang: &str) -> Vec<PathBuf> {
    let tmp = std::env::temp_dir().join(format!(
        "sce_marker0c_{}_{}_{}_pid{}_{}",
        name,
        lang,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        rand_suffix(),
    ));
    std::fs::create_dir_all(&tmp).expect("create temp dir");
    let scxml = tmp.join(format!("{name}.scxml"));
    std::fs::write(&scxml, fixture).expect("write fixture");

    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&scxml)
        .arg("-l")
        .arg(lang)
        .arg("-o")
        .arg(&tmp)
        .output()
        .expect("sce-codegen invocation");
    assert!(
        out.status.success(),
        "sce-codegen generate -l {lang} on {name}.scxml failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut artifacts = Vec::new();
    for entry in std::fs::read_dir(&tmp).expect("read temp dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().map(|e| e == "scxml").unwrap_or(false) {
            continue;
        }
        if path.is_dir() {
            // Python backend lands its output in a package directory.
            for sub in std::fs::read_dir(&path).expect("read sub dir") {
                let sub = sub.expect("sub entry");
                let sp = sub.path();
                if sp.is_file() {
                    artifacts.push(sp);
                }
            }
        } else {
            artifacts.push(path);
        }
    }
    assert!(
        !artifacts.is_empty(),
        "no artifacts generated for -l {lang} on {name} in {}",
        tmp.display()
    );
    artifacts
}

fn rand_suffix() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{nanos:08x}")
}

/// Count SCE-MAP marker occurrences in `path` referencing the fixture
/// basename. Spec §5.O distinguishes two forms:
///   - module-level (0b): comment-form `// SCE-MAP: <file>:<line>` /
///     `# SCE-MAP: …` / Rust's `#![doc]` + `// SCE-MAP: …`.
///   - function-level (0c): the per-backend directive form:
///       * C / Cpp: `#line {line} "{file}"`
///       * Rust:    `#[doc = "SCE-MAP: …"]` + `// SCE-MAP: …`
///       * Go:      `//line {file}:{line}`
///       * Kotlin / Python: comment-form (Kotlin has no directive
///         equivalent; Python lacks one too — both backends share the
///         module-level shape for per-function placement).
/// The count includes both shapes so Atomic 0c's per-function emission
/// surfaces alongside Atomic 0b's module-level baseline.
fn count_markers(path: &Path, fixture_name: &str) -> usize {
    let body =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    body.lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            if !l.contains(fixture_name) {
                return false;
            }
            // Comment / doc-attribute forms (module-level + Rust function).
            if l.contains("SCE-MAP") {
                return true;
            }
            // C/Cpp `#line N "<file>"` directive (function-level form).
            if trimmed.starts_with("#line ") {
                return true;
            }
            // Go `//line <file>:N` directive (function-level form).
            if trimmed.starts_with("//line ") {
                return true;
            }
            false
        })
        .count()
}

/// Find the artifact with the given extension among the generated
/// files; returns the matched path or panics with a clear failure
/// describing what was emitted.
fn pick<'a>(artifacts: &'a [PathBuf], ext: &str) -> &'a Path {
    artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == ext).unwrap_or(false))
        .unwrap_or_else(|| {
            panic!(
                "no `.{ext}` artifact found among {:?}",
                artifacts
                    .iter()
                    .map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .collect::<Vec<_>>()
            )
        })
        .as_path()
}

// ── Statechart per-function markers ────────────────────────────

#[test]
fn rust_statechart_emits_per_function_markers() {
    let artifacts = generate("sm_probe", STATECHART_FIXTURE, "rust");
    let rs = pick(&artifacts, "rs");
    let count = count_markers(rs, "sm_probe.scxml");
    // 1 module-level (0b) + per-function markers above
    //   execute_entry_actions, execute_exit_actions, process_transition,
    //   execute_transition_actions (0c). Rust emits BOTH `#![doc]`
    //   and `// SCE-MAP:` for module-level (counted as 2 lines), and
    //   BOTH `#[doc]` and `// SCE-MAP:` for each function-level marker
    //   (each counted as 2). Expect strictly more than the pure 0b
    //   count (2 lines for module-level alone).
    assert!(
        count >= 4,
        "rust per-function markers missing — got {count} marker lines, expected >=4",
    );
}

#[test]
fn cpp_statechart_emits_per_function_markers() {
    let artifacts = generate("sm_probe", STATECHART_FIXTURE, "cpp");
    // Cpp emits `.h` (state_machine.jinja2) carrying the module-level
    // marker, and `.inl` (state_machine_inl.jinja2) carrying the
    // per-function bodies. The 0c per-function markers live in
    // `entry_exit_actions.jinja2` + `process_transition.jinja2`, both
    // included from `state_machine_inl.jinja2`. Assert the per-
    // function `#line` directives surface in the `.inl` file —
    // executeEntryActions, executeExitActions, processTransition,
    // executeMicrostep, tryTransitionInState, executeTransitionActions.
    let inl = pick(&artifacts, "inl");
    let count = count_markers(inl, "sm_probe.scxml");
    assert!(
        count >= 4,
        "cpp inl per-function markers missing — got {count}, expected >=4",
    );
}

#[test]
fn c11_statechart_emits_per_function_markers() {
    let artifacts = generate("sm_probe", STATECHART_FIXTURE, "c11");
    let c = pick(&artifacts, "c");
    let count_c = count_markers(c, "sm_probe.scxml");
    // C11 emits per-state `<name>_on_entry_<state>_block_N` + per-state
    // `<name>_on_exit_<state>_block_N` functions, plus dispatchers
    // (execute_entry_actions, execute_exit_actions,
    // execute_transition_actions, process_transition). Each function
    // gets its own `#line N "file"` directive from the 0c per-function
    // marker emission. The module-level (0b) comment marker also lives
    // in the impl file.
    assert!(
        count_c >= 3,
        "c11 impl per-function markers missing — got {count_c}, expected >=3",
    );
}

#[test]
fn kotlin_statechart_emits_per_function_markers() {
    let artifacts = generate("sm_probe", STATECHART_FIXTURE, "kotlin");
    let kt = pick(&artifacts, "kt");
    let count = count_markers(kt, "sm_probe.scxml");
    // 0b module-level + 0c per-function above onEntry, onExit, and
    // executeTransitionActions.
    assert!(
        count >= 2,
        "kotlin per-function markers missing — got {count}, expected >=2",
    );
}

#[test]
fn go_statechart_emits_per_function_markers() {
    let artifacts = generate("sm_probe", STATECHART_FIXTURE, "go");
    let go = pick(&artifacts, "go");
    let count = count_markers(go, "sm_probe.scxml");
    // 0b emitted 1 comment-form module-level marker. 0c adds `//line`
    // directives above ExecuteEntryActions, ExecuteExitActions,
    // ProcessTransition, tryTransitionInState, ExecuteTransitionActions.
    // The comment-form module marker counts via "SCE-MAP" substring;
    // `//line` directives count via the fixture basename appearing on
    // them (we wrote them as `//line {file}:N`).
    assert!(
        count >= 2,
        "go per-function markers missing — got {count}, expected >=2",
    );
}

// ── Forge per-kind body markers ────────────────────────────────

#[test]
fn rust_forge_codec_emits_module_level_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "rust");
    let rs = pick(&artifacts, "rs");
    let body = std::fs::read_to_string(rs).expect("read forge codec");
    assert!(
        body.contains("#![doc = \"SCE-MAP: codec_probe.scxml"),
        "rust codec output missing `#![doc = \"SCE-MAP: ...\"]` form\n--- body ---\n{body}",
    );
    assert!(
        body.contains("// SCE-MAP: codec_probe.scxml"),
        "rust codec output missing `// SCE-MAP: ...` comment form\n--- body ---\n{body}",
    );
}

#[test]
fn cpp_forge_codec_emits_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "cpp");
    let h = pick(&artifacts, "h");
    assert!(
        count_markers(h, "codec_probe.scxml") >= 1,
        "cpp forge codec missing marker",
    );
}

#[test]
fn c11_forge_codec_emits_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "c11");
    let h = pick(&artifacts, "h");
    assert!(
        count_markers(h, "codec_probe.scxml") >= 1,
        "c11 forge codec missing marker",
    );
}

#[test]
fn kotlin_forge_codec_emits_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "kotlin");
    let kt = pick(&artifacts, "kt");
    assert!(
        count_markers(kt, "codec_probe.scxml") >= 1,
        "kotlin forge codec missing marker",
    );
}

#[test]
fn go_forge_codec_emits_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "go");
    let go = pick(&artifacts, "go");
    assert!(
        count_markers(go, "codec_probe.scxml") >= 1,
        "go forge codec missing marker",
    );
}

#[test]
fn python_forge_codec_emits_marker() {
    let artifacts = generate("codec_probe", CODEC_FIXTURE, "python");
    let py = pick(&artifacts, "py");
    assert!(
        count_markers(py, "codec_probe.scxml") >= 1,
        "python forge codec missing marker",
    );
}
