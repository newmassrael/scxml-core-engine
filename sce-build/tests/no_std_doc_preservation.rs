// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §synth-5-O — empirical preservation
// guard for Rust's dual-emit SCE-MAP marker contract.
//
// Spec lines 3135-3136 (verbatim): "Rust MUST emit BOTH `#[doc =
// \"SCE-MAP: …\"]` AND `// SCE-MAP: …`". The macro
// (`tools/codegen/templates/_macros/sce_map_marker.jinja2`) emits both
// forms unconditionally; this test pins the emission against a future
// template edit that drops one form (silently breaking the fallback
// that the `// SCE-MAP:` line comment provides if rustdoc strips the
// `#[doc]` attribute under release / no_std).
//
// The empirical preservation test scope:
//   1. Generate Rust SM source with the default (std) profile and
//      assert both marker forms appear.
//   2. Generate again with `--no-std` (the C3 B-β CLI flag) and assert
//      both forms are preserved across the alternate template path.
//   3. (Deferred) A rustdoc JSON dump (`cargo doc --output-format
//      json`) would catch downstream strip-by-rustdoc behaviour, but
//      that flag is nightly-only on stable Rust today. The
//      `traceability/sce-map-attribute-stripped` diagnostic
//      is the runtime channel that surfaces a future strip — the
//      empirical test of "rustdoc preserves `#[doc]`" lands when a
//      consumer materialises that exercises the JSON dump path.
//
// A template edit that drops the dual-emit form surfaces here as a
// content-match failure, not as a silent contract regression.

use std::path::{Path, PathBuf};
use std::process::Command;

const STATECHART_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

/// Variant fixture with a literal-label `<log>` action — exercises
/// the per-function marker emission on the std code path without
/// invoking the script engine (the `--no-std` rejection only fires
/// on expr-bearing actions). Used by the strict-pair test which
/// runs against the default std profile.
const STATECHART_FIXTURE_WITH_LOG: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log label="enter s1"/>
    </onentry>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn rand_suffix() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    h.finish()
}

/// Run sce-codegen against the fixture under a chosen profile and
/// return the generated `*_sm.rs` body. `extra_args` plumbs `--no-std`
/// when the alternate path is exercised. Caller picks which fixture
/// is staged via `fixture`.
fn generate_rust_with_fixture(fixture_name: &str, fixture: &str, extra_args: &[&str]) -> String {
    let tmp = std::env::temp_dir().join(format!(
        "sce_no_std_preserve_{}_{}_{:x}",
        fixture_name,
        std::process::id(),
        rand_suffix(),
    ));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let scxml = tmp.join(format!("{fixture_name}.scxml"));
    std::fs::write(&scxml, fixture).expect("write fixture");

    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("generate")
        .arg(&scxml)
        .arg("-l")
        .arg("rust")
        .arg("-o")
        .arg(&tmp);
    for a in extra_args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("sce-codegen invocation");
    assert!(
        out.status.success(),
        "sce-codegen generate -l rust on {fixture_name}.scxml failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let sm_file = tmp.join(format!("{fixture_name}_sm.rs"));
    let body = std::fs::read_to_string(&sm_file)
        .unwrap_or_else(|e| panic!("read {}: {e}", sm_file.display()));
    // Cleanup eagerly — large tmp directories under
    // `cargo test --no-fail-fast` accumulate otherwise.
    let _ = std::fs::remove_dir_all(&tmp);
    body
}

/// Assert the source carries BOTH the `#[doc = "SCE-MAP: ..."]` and
/// `// SCE-MAP: ...` forms. Counts each so a template edit that drops
/// one form (regressing the dual-emit contract) is observable.
fn assert_dual_emit(body: &str, label: &str) {
    let doc_count =
        body.matches("#[doc = \"SCE-MAP:").count() + body.matches("#![doc = \"SCE-MAP:").count();
    let comment_count = body.matches("// SCE-MAP:").count();
    assert!(
        doc_count >= 1,
        "{label}: no `#[doc = \"SCE-MAP: ...\"]` form emitted. \
         Spec lines 3135-3136 require BOTH forms.\nbody:\n{}",
        body.chars().take(2000).collect::<String>(),
    );
    assert!(
        comment_count >= 1,
        "{label}: no `// SCE-MAP: ...` form emitted. Spec lines \
         3135-3136 require BOTH forms.\nbody:\n{}",
        body.chars().take(2000).collect::<String>(),
    );
    // Per spec the two emission counts should match in production:
    // the macro emits them in pairs. A `>=` rather than `==` check
    // tolerates module-level emission (inner attribute `#![doc]` +
    // sibling `// SCE-MAP:` comment) which carries `module_level=true`
    // through both fork arms — same count expected.
    assert_eq!(
        doc_count, comment_count,
        "{label}: dual-emit count mismatch. doc={doc_count} comment={comment_count}. \
         Macro should pair the forms 1:1.",
    );
}

#[test]
fn rust_default_profile_emits_both_marker_forms() {
    let body = generate_rust_with_fixture("default_path", STATECHART_FIXTURE_WITH_LOG, &[]);
    assert_dual_emit(&body, "rust default (std)");
}

#[test]
fn rust_no_std_profile_emits_both_marker_forms() {
    // The C3 B-β `--no-std` flag toggles the alternate template path
    // (`#![no_std]` header + `core::*` swaps + invoke/HTTP rejection).
    // The dual-emit must survive across both paths so
    // the line-comment fallback covers a future rustdoc strip. The
    // fixture is the script-free variant because `--no-std` rejects
    // any `<log expr="...">` (the analyzer flags it as script-bound).
    let body = generate_rust_with_fixture("nostd_path", STATECHART_FIXTURE, &["--no-std"]);
    assert_dual_emit(&body, "rust --no-std");
}

#[test]
fn marker_dual_emit_is_strict_pairing() {
    // Sanity check that the macro emits the two forms in lockstep —
    // the line right after `#[doc = "SCE-MAP: ..."]` is always a
    // matching `// SCE-MAP: ...` comment. A drift here would mean a
    // future template added one form without the other.
    let body = generate_rust_with_fixture("pair_strict", STATECHART_FIXTURE_WITH_LOG, &[]);
    let lines: Vec<&str> = body.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("#[doc = \"SCE-MAP:") && !trimmed.starts_with("#![doc = \"SCE-MAP:")
        {
            continue;
        }
        // Next non-blank line must be the matching `// SCE-MAP:` comment.
        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        let Some(next) = lines.get(j) else {
            panic!(
                "doc-form marker at line {} has no following // SCE-MAP: pair:\n{}",
                i + 1,
                lines[..=i].join("\n"),
            );
        };
        let next_trimmed = next.trim_start();
        assert!(
            next_trimmed.starts_with("// SCE-MAP:"),
            "doc-form marker at line {} not followed by line-comment pair. \
             Next non-blank line was: {next:?}",
            i + 1,
        );
    }
}

// Defensive — touch the binary path at compile time so an invocation
// before this test runs surfaces a clean stub message rather than
// failing inside generate_rust.
#[allow(dead_code)]
fn _force_bin_link() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-codegen"))
}
