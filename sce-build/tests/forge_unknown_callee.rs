// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A forge expression may not call a name nothing provides.
//
// `infer_types` types an unresolved callee `Unknown`, and the emitters then
// print it verbatim. So both of these generated with exit 0:
//
//     expr="round(v * 100.0) / 100.0"   ->  return round(v * 100.0) / 100.0;
//     expr="totallyMadeUpFn(v)"         ->  return totallyMadeUpFn(v);
//
// The first does not compile — `round` lives in `<cmath>` and the emitted
// header includes `<cstdint>` and `<string>`. Neither does the second, which
// is the only reason the hole was survivable: a typo in a helper's name took
// the same path and was caught by the C++ compiler rather than by SCE, in a
// message about a name the author never wrote.
//
// ⚠ Found 2026-09-17 while looking for a form for a specification's
// `Rounds off`, used 149 times in that corpus. It has none — which is a
// separate decision from this one. What this file locks is that the ABSENCE
// is reported by the generator instead of by a downstream compiler.
//
// ⚠⚠ THE NARROWNESS IS THE OTHER HALF, and the first version of the check
// did not have it. A STATECHART guard may legitimately call what the host
// provides — `computeKey(seed, 0x01)`, `securityResponse.decode(_event.data)`
// — and those are emitted verbatim by design. An ungated check refused three
// such cases that the unit suite asserts. The check is therefore gated on
// `TypeCtx::reject_unknown_callees`, which only the per-kind forge builders
// set, and the statechart case below is what keeps that gate honest.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH: AtomicU64 = AtomicU64::new(0);

struct Out(PathBuf);

impl Out {
    fn new(label: &str) -> Self {
        let id = SCRATCH.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Out(dir)
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn generate(doc: &str, label: &str) -> (Option<i32>, String) {
    let out = Out::new(label);
    let path = out.0.join("doc.scxml");
    std::fs::write(&path, doc).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            "cpp",
            "-o",
            out.0.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

fn transform_with(expr: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="doc" version="1.0">
  <datamodel>
    <data id="v" sce:type="float64" sce:direction="in"/>
    <data id="r" sce:type="float64" sce:direction="out" expr="{expr}"/>
  </datamodel>
</scxml>
"#
    )
}

/// A maths function the expression language does not carry. The refusal must
/// NAME it and say what is available, because "available" is the whole of the
/// answer an author needs.
///
/// ⚠ THE SUBJECT MOVED, AND THE REASON IS THE POINT. This test was written
/// against `round(v * 100.0) / 100.0`, because `round` was the name that
/// started the whole check. `round` is a builtin now, so the document it built
/// GENERATES — and the assertion, being `assert_ne!(code, Some(0))`, then
/// failed with `left: Some(0), right: Some(0)`. A test whose subject becomes
/// supported does not weaken quietly; it goes red, which is the right way
/// round. Its sibling in `expr.rs` had already been re-pointed for exactly
/// this reason and this one had not, so the vocabulary grew in two places and
/// only one of them was swept.
///
/// `sqrt` is the subject now. Whoever adds `sqrt` will land here next, and the
/// fix is the same: re-point the test at a name the vocabulary still lacks,
/// never relax the assertion.
#[test]
fn a_transform_calling_an_unprovided_maths_name_is_refused() {
    let (code, stderr) = generate(&transform_with("sqrt(v) / 100.0"), "sqrt");
    assert_ne!(code, Some(0), "a transform calling sqrt() generated");
    assert!(
        stderr.contains("sqrt"),
        "the refusal does not name the callee:\n{stderr}"
    );
    assert!(
        stderr.contains("len") && stderr.contains("eq"),
        "the refusal does not offer the builtins that ARE available:\n{stderr}"
    );
}

/// ⚠ And the builtins the vocabulary DOES carry must still generate, so the
/// test above cannot be satisfied by refusing everything. `round` and `floor`
/// are the two that arrived after this file was written; both take the shape
/// the refused document had, which is what makes them the right control.
#[test]
fn the_rounding_builtins_still_generate() {
    for (expr, label) in [
        ("round(v * 100.0) / 100.0", "roundok"),
        ("floor(v * 100.0) / 100.0", "floorok"),
    ] {
        let (code, stderr) = generate(&transform_with(expr), label);
        assert_eq!(code, Some(0), "`{expr}` was refused:\n{stderr}");
    }
}

/// ⚠ And a misspelt helper takes the same path. Without this the check could
/// be read as "maths is unsupported" and narrowed to a maths name list, which
/// would let every typo back through.
#[test]
fn a_transform_calling_an_unknown_name_is_refused() {
    let (code, stderr) = generate(&transform_with("totallyMadeUpFn(v)"), "madeup");
    assert_ne!(
        code,
        Some(0),
        "a transform calling an unknown name generated"
    );
    assert!(
        stderr.contains("totallyMadeUpFn"),
        "the refusal does not name the callee:\n{stderr}"
    );
}

/// The builtins still generate. A check that refused `len` would be the same
/// defect pointing the other way.
#[test]
fn the_length_builtin_still_generates() {
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="doc" version="1.0">
  <datamodel>
    <data id="payload" sce:type="bytes" sce:direction="in"/>
    <data id="n" sce:type="uint32" sce:direction="out" expr="len(payload)"/>
  </datamodel>
</scxml>
"#;
    let (code, stderr) = generate(doc, "lenok");
    assert_eq!(code, Some(0), "len(payload) was refused:\n{stderr}");
}

// ⚠⚠⚠ THE GATE'S WITNESS IS NOT HERE, AND A TEST THAT CLAIMED IT WAS HAS
// BEEN REMOVED.
//
// This file first carried `a_statechart_guard_may_still_call_the_host`: a
// document with `cond="computeKey(seed, 1) > 0"`, asserted to still generate.
// It passed. It also passed with the gate deleted — measured by replacing the
// gate with `if false && …` and re-running — so it was proving nothing. A
// statechart guard on the ECMAScript data model does not reach
// `transpile_typed` at all, so no document-level test placed here can tell
// whether the gate is present.
//
// The real witnesses are the three UNIT tests the first, ungated version of
// the check actually broke:
//
//     forge::expr::tests::cpp_function_call_multi_args
//     forge::expr::tests::cpp_method_call_on_member
//     forge::expr::tests::rust_nested_function_calls_rendered_snake_case
//
// They call the transpiler directly with an EMPTY TypeCtx — which is what an
// ungated check refuses and a gated one must not — so they are exactly the
// arm this file cannot express. They live in the unit suite and run beside
// these.
//
// Leaving the passing-but-vacuous version would have been worse than having
// no test: it promised a witness, so the next person would not go looking for
// one.
