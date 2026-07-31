// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Invocation determinism for `sce-codegen generate`.
//!
//! A consumer that commits generated output — or pins the generator as a
//! hermetic build dependency — needs one property: the same generator on
//! the same input produces the same bytes. Two things in the emit path can
//! break it without touching the input at all, and both are asserted here
//! end-to-end against the real binary rather than against a helper:
//!
//! - the `// From:` source path, which must not vary with the directory
//!   the build happened to run from (§synth-6.2.6 provenance is only
//!   auditable if it is reproducible);
//! - the `generated-at` stamp, which `SOURCE_DATE_EPOCH` pins per the
//!   reproducible-builds convention so a "regenerate and expect no diff"
//!   CI gate is writable.
//!
//! Both are documented in `docs/SCE_CODEGEN_DETERMINISM.md`; these tests
//! are what keep that document true.

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn codegen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sce-codegen")
}

const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="determinism" initial="s0">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
"#;

/// A source tree holding one document, plus a scratch area for outputs.
struct Fixture {
    _root: TempDir,
    doc: PathBuf,
    src_dir: PathBuf,
    out_base: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let src_dir = root.path().join("src").join("scxml");
        let out_base = root.path().join("out");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&out_base).unwrap();
        let doc = src_dir.join("determinism.scxml");
        std::fs::write(&doc, FIXTURE).unwrap();
        Fixture {
            _root: root,
            doc,
            src_dir,
            out_base,
        }
    }

    /// Generate into a fresh output directory from `cwd`, returning it.
    /// `SOURCE_DATE_EPOCH` is pinned so the only variable under test is
    /// the working directory.
    fn generate_from(&self, cwd: &Path, tag: &str, extra: &[&str]) -> PathBuf {
        let out = self.out_base.join(tag);
        std::fs::create_dir_all(&out).unwrap();
        let result = Command::new(codegen_bin())
            .arg("generate")
            .arg(&self.doc)
            .arg("-o")
            .arg(&out)
            .arg("-l")
            .arg("rust")
            .args(extra)
            .env("SOURCE_DATE_EPOCH", "0")
            .current_dir(cwd)
            .output()
            .expect("sce-codegen must be runnable");
        assert!(
            result.status.success(),
            "generate from cwd={} failed: {}",
            cwd.display(),
            String::from_utf8_lossy(&result.stderr),
        );
        out
    }
}

fn emitted(dir: &Path) -> String {
    let sm = dir.join("determinism_sm.rs");
    std::fs::read_to_string(&sm).unwrap_or_else(|e| panic!("reading {}: {e}", sm.display()))
}

fn from_line(content: &str) -> String {
    // The license header wraps long paths onto a continuation line, so the
    // provenance value is "everything after `// From:` up to the next bare
    // `//`" rather than the remainder of one line.
    let mut out = String::new();
    let mut in_from = false;
    for line in content.lines() {
        let body = line.trim_start().trim_start_matches("//").trim();
        if in_from {
            if body.is_empty() {
                break;
            }
            out.push_str(body);
            continue;
        }
        if let Some(rest) = body.strip_prefix("From:") {
            in_from = true;
            out.push_str(rest.trim());
        }
    }
    assert!(!out.is_empty(), "no `// From:` provenance line in output");
    out
}

/// The reported defect: the emitted provenance path had the process
/// working directory stripped as a prefix, so the same generator and the
/// same absolute input produced different bytes depending on where the
/// build ran. Reproducing an artifact then required reproducing the
/// directory it was generated in, which no build system guarantees.
#[test]
fn from_line_does_not_vary_with_working_directory() {
    let fx = Fixture::new();
    let tmp = TempDir::new().unwrap();

    // Three shapes that previously produced three different answers: a cwd
    // that is a proper prefix of the input path, the filesystem root (whose
    // prefix strip also consumed the leading separator), and an unrelated
    // directory.
    let under = fx.generate_from(fx.src_dir.parent().unwrap(), "under", &[]);
    let root = fx.generate_from(Path::new("/"), "root", &[]);
    let away = fx.generate_from(tmp.path(), "away", &[]);

    let a = emitted(&under);
    let b = emitted(&root);
    let c = emitted(&away);

    assert_eq!(
        from_line(&a),
        from_line(&b),
        "provenance path must not depend on the invoking directory"
    );
    assert_eq!(from_line(&b), from_line(&c));
    assert_eq!(a, b, "whole artifact must be byte-identical across cwds");
    assert_eq!(b, c);
}

/// With no `--source-root`, the emitted path is the one the caller named.
/// "As given on the command line" is the only spelling that needs no
/// ambient state to reproduce.
#[test]
fn from_line_is_the_path_as_given() {
    let fx = Fixture::new();
    let out = fx.generate_from(Path::new("/"), "verbatim", &[]);
    assert_eq!(from_line(&emitted(&out)), fx.doc.display().to_string());
}

/// `--source-root` re-expresses provenance relative to an explicit root,
/// which is what a consumer wants committed: a repo-relative path that
/// stays meaningful on a machine that never ran the generator.
#[test]
fn source_root_makes_the_from_line_relative_to_it() {
    let fx = Fixture::new();
    let root = fx.src_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let out = fx.generate_from(
        Path::new("/"),
        "rooted",
        &["--source-root", root.to_str().unwrap()],
    );
    assert_eq!(from_line(&emitted(&out)), "src/scxml/determinism.scxml");

    // ...and the answer is the same from a different cwd, which is the
    // whole point of naming the root explicitly.
    let elsewhere = fx.generate_from(
        &fx.src_dir,
        "rooted2",
        &["--source-root", root.to_str().unwrap()],
    );
    assert_eq!(emitted(&out), emitted(&elsewhere));
}

/// An input outside the named root has no relative spelling, so it falls
/// back to the path as given rather than emitting a `../..` chain that
/// only resolves on the generating machine.
#[test]
fn source_root_falls_back_when_input_is_outside_it() {
    let fx = Fixture::new();
    let unrelated = TempDir::new().unwrap();
    let out = fx.generate_from(
        Path::new("/"),
        "outside",
        &["--source-root", unrelated.path().to_str().unwrap()],
    );
    assert_eq!(from_line(&emitted(&out)), fx.doc.display().to_string());
}

/// `generated-at` is wall-clock by default, so two runs never match. The
/// reproducible-builds convention pins it, and pinning it is what makes a
/// "regenerate, expect no diff" CI gate expressible — the check the field
/// report could not write.
#[test]
fn source_date_epoch_pins_generated_at_for_byte_stable_regen() {
    let fx = Fixture::new();
    let first = fx.generate_from(Path::new("/"), "epoch_a", &[]);
    let second = fx.generate_from(Path::new("/"), "epoch_b", &[]);

    let a = emitted(&first);
    let b = emitted(&second);
    assert!(
        a.contains("// generated-at: 0"),
        "SOURCE_DATE_EPOCH must drive the stamp, got:\n{}",
        a.lines().take(4).collect::<Vec<_>>().join("\n"),
    );
    assert_eq!(
        a, b,
        "regeneration under a pinned epoch must be byte-stable"
    );
}

/// Without the pin the stamp tracks the clock, which is the behaviour the
/// pin exists to switch off. Asserting it keeps the documented default
/// honest rather than leaving the two paths indistinguishable.
#[test]
fn generated_at_tracks_the_clock_without_the_pin() {
    let fx = Fixture::new();
    let out = fx.out_base.join("unpinned");
    std::fs::create_dir_all(&out).unwrap();
    let result = Command::new(codegen_bin())
        .arg("generate")
        .arg(&fx.doc)
        .arg("-o")
        .arg(&out)
        .arg("-l")
        .arg("rust")
        .env_remove("SOURCE_DATE_EPOCH")
        .current_dir("/")
        .output()
        .expect("sce-codegen must be runnable");
    assert!(result.status.success());
    let stamp = emitted(&out)
        .lines()
        .find_map(|l| l.strip_prefix("// generated-at: ").map(str::to_string))
        .expect("header carries a generated-at line");
    let secs: u64 = stamp.parse().expect("stamp is unix seconds");
    assert!(
        secs > 1_700_000_000,
        "unpinned stamp should be a real wall-clock time, got {secs}"
    );
}
