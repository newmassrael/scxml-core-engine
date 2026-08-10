// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `sce-codegen --lint` vs the library compile entry point.
//
// The two surfaces run different code paths by construction: the CLI
// re-implements parse -> analyze -> generate, while
// `compile_scxml_lang_typed` walks `lib.rs::compile_model`'s chain.
// Before `lint_statechart` existed, the design-time lints lived only in
// that chain, so `sce-codegen check` answered `status: ok` for
// documents the library rejected — the CLI is the surface consumers
// read, so the checks were effectively absent.
//
// Three claims are pinned here, and none is self-evident:
//
//   1. **Opt-in, not absent.** `--lint` reaches every lint the library
//      chain runs. One document per diagnostic, each rejected with the
//      code the library produces.
//   2. **Same verdict.** For the same document, CLI `--lint` and the
//      library entry either both accept or both reject with the same
//      diagnostic code. A lint added to one call sequence and not the
//      other reds here.
//   3. **Off by default.** Without the flag the CLI accepts these
//      documents. The lints reject *legal* SCXML — the W3C IRP corpus
//      deliberately declares unreachable states — so default-on would
//      refuse conformance documents that build and run.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).expect("write fixture");
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `check` exit code plus the first stderr line.
fn cli_check(path: &Path, lint: bool) -> (i32, String) {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("check").arg(path).arg("-l").arg("rust");
    if lint {
        cmd.arg("--lint");
    }
    let out = cmd.output().expect("run sce-codegen check");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (out.status.code().unwrap_or(-1), stderr)
}

/// The wire `code` the CLI emits, read from the NDJSON stream so the
/// comparison is against the same string the library produces rather
/// than against prose.
fn cli_check_wire_code(path: &Path, lint: bool) -> Option<String> {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("--error-format")
        .arg("json")
        .arg("check")
        .arg(path)
        .arg("-l")
        .arg("rust");
    if lint {
        cmd.arg("--lint");
    }
    let out = cmd.output().expect("run sce-codegen check");
    if out.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stderr.lines().find(|l| l.starts_with('{'))?;
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    value
        .get("code")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// The wire `code` the library entry point produces, or `None` when it
/// accepts the document.
fn library_wire_code(path: &Path) -> Option<String> {
    let template_dir = find_template_dir_for(Language::Rust);
    match compile_scxml_lang_typed(path.to_str().unwrap(), &template_dir, Language::Rust) {
        Ok(_) => None,
        Err(e) => {
            let diags = e.error.to_diagnostics();
            assert!(!diags.is_empty(), "a rejection must carry a diagnostic");
            Some(diags[0].code.as_str().to_string())
        }
    }
}

/// One document per lint, plus a clean control.
const CASES: &[(&str, &str, Option<&str>)] = &[
    (
        "unreachable_state",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="unreachable_state" initial="idle">
  <state id="idle">
    <transition event="go" target="done_state"/>
  </state>
  <final id="done_state"/>
  <final id="orphan"/>
</scxml>
"#,
        Some("scxml/unreachable-state"),
    ),
    (
        "dead_transition",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="dead_transition" initial="idle">
  <state id="idle">
    <transition event="go" target="done_state"/>
  </state>
  <state id="orphan">
    <transition event="never" target="idle"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
        Some("scxml/dead-transition"),
    ),
    (
        "non_exhaustive",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="non_exhaustive" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="idle"/>
    </state>
    <state id="active">
      <transition event="cmd.start" target="active"/>
    </state>
  </state>
</scxml>
"#,
        Some("scxml/non-exhaustive-event-handling"),
    ),
    (
        "always_false_guard",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="always_false_guard" initial="idle" datamodel="ecmascript">
  <state id="idle">
    <transition event="go" cond="1==2" target="done_state"/>
    <transition event="go" target="idle"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
        Some("scxml/always-false-guard"),
    ),
    (
        "clean",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="clean" initial="idle">
  <state id="idle">
    <transition event="go" target="done_state"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
        None,
    ),
];

#[test]
fn lint_flag_reaches_every_library_chain_lint() {
    let scratch = ScratchDir::new("lint-reaches");
    let mut reached = 0usize;
    for (name, body, expected) in CASES {
        let path = scratch.write(&format!("{name}.scxml"), body);
        let actual = cli_check_wire_code(&path, true);
        assert_eq!(
            actual.as_deref(),
            *expected,
            "`check --lint` verdict for {name}"
        );
        if expected.is_some() {
            reached += 1;
        }
    }
    // Lower bound: a run that silently stopped exercising the lints
    // (e.g. every fixture drifting into acceptance) would otherwise
    // report success with zero rejections.
    assert!(
        reached >= 4,
        "expected at least 4 lint rejections, got {reached}"
    );
}

#[test]
fn cli_lint_and_library_entry_agree() {
    let scratch = ScratchDir::new("lint-parity");
    for (name, body, _) in CASES {
        let path = scratch.write(&format!("{name}.scxml"), body);
        let cli = cli_check_wire_code(&path, true);
        let lib = library_wire_code(&path);
        assert_eq!(
            cli, lib,
            "CLI --lint and the library entry disagree about {name}"
        );
    }
}

#[test]
fn lints_are_off_by_default() {
    // The lints reject legal SCXML. `resources/278` — a W3C IRP
    // document whose second state exists only to host a datamodel read
    // from outside its lexical scope — must keep building without the
    // flag, or the conformance corpus stops generating.
    let scratch = ScratchDir::new("lint-default-off");
    for (name, body, expected) in CASES {
        let path = scratch.write(&format!("{name}.scxml"), body);
        let (code, stderr) = cli_check(&path, false);
        assert_eq!(
            code, 0,
            "{name} must pass without --lint (expected lint {expected:?}), stderr: {stderr}"
        );
    }

    let w3c = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("resources/278/test278.scxml");
    if w3c.exists() {
        let (code, stderr) = cli_check(&w3c, false);
        assert_eq!(
            code, 0,
            "W3C test278 must build without --lint, stderr: {stderr}"
        );
        let (lint_code, _) = cli_check(&w3c, true);
        assert_ne!(
            lint_code, 0,
            "W3C test278 declares a deliberately unreachable state — \
             `--lint` is expected to flag it, which is why the flag is \
             opt-in rather than default"
        );
    }
}
