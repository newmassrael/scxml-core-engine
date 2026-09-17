// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `condition` answers one question, and a second output used to erase the
// first without a word.
//
// `ConditionModel` holds a single `expr` and the emitted function returns a
// single bool. `parse_condition` nevertheless looped over every `out` field
// and assigned `expr = e.clone()` each time, so a document declaring two
// outputs generated cleanly and emitted ONE function, named after the
// document, computing the LAST expression:
//
//     <data id="supported" sce:direction="out" expr="A"/>
//     <data id="decided"   sce:direction="out" expr="B"/>
//   ->
//     inline bool thatDocument(...) { return B; }
//
// ⚠ This is the worst-behaved member of the family of defects this survey
// found. The lookup miss-default and the codec bytes bit-size both emitted
// C++ THAT DID NOT COMPILE, so a build caught them the moment anyone tried.
// This one compiles and runs and answers a different question than the
// document asks, which nothing downstream can detect — the only witness is
// reading the emitted function beside the source. Measured 2026-09-17 while
// converting a platform-support matrix whose columns carry three marks (`O`,
// `-`, blank) and therefore needed two propositions, not one.

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

fn condition(outputs: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="platform_support" version="1.0">
  <datamodel>
    <data id="isTwelve" sce:type="bool" sce:direction="in"/>
    <data id="markedFour" sce:type="bool" sce:direction="in"/>
    <data id="markedTwelve" sce:type="bool" sce:direction="in"/>
    {outputs}
  </datamodel>
</scxml>
"#
    )
}

fn generate(doc: &str, label: &str) -> (Option<i32>, String, PathBuf) {
    let out = Out::new(label);
    let path = out.0.join("platform_support.scxml");
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
    let dir = out.0.clone();
    // The scratch dir must outlive this call for the emitted-header
    // assertions below, so the guard is leaked deliberately and the caller
    // reads from `dir`.
    std::mem::forget(out);
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
        dir,
    )
}

/// Two outputs are refused, and the refusal names BOTH — the one that already
/// claimed the slot and the one being rejected. A message naming only the
/// second would leave the author hunting for what took the slot.
#[test]
fn a_second_condition_output_is_refused() {
    let (code, stderr, dir) = generate(
        &condition(
            r#"<data id="supported" sce:type="bool" sce:direction="out"
              expr="isTwelve ? markedTwelve : markedFour"/>
    <data id="decided" sce:type="bool" sce:direction="out"
              expr="isTwelve || markedFour"/>"#,
        ),
        "cond-two-out",
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_ne!(code, Some(0), "a two-output condition generated");
    assert!(
        stderr.contains("supported") && stderr.contains("decided"),
        "the refusal does not name both outputs:\n{stderr}"
    );
    // The way OUT matters as much as the refusal, as it did for the codec
    // bytes field: an author with two propositions has to be told where they
    // go. `transform` is the kind that emits one function per output.
    assert!(
        stderr.contains("transform"),
        "the refusal does not name the kind that carries several outputs:\n{stderr}"
    );
}

/// ⚠ And the refusal stays narrow: one output is the ordinary shape and must
/// still generate. A check that refused it would be the same defect pointing
/// the other way.
#[test]
fn a_single_condition_output_still_generates() {
    let (code, stderr, dir) = generate(
        &condition(
            r#"<data id="supported" sce:type="bool" sce:direction="out"
              expr="isTwelve ? markedTwelve : markedFour"/>"#,
        ),
        "cond-one-out",
    );
    assert_eq!(
        code,
        Some(0),
        "a one-output condition was refused:\n{stderr}"
    );

    // ⚠⚠ And it computes the expression that was declared. The defect was
    // never a missing artefact — it was an artefact carrying the wrong body,
    // so the body is what this asserts. Without this the test would pass
    // against a generator that emitted an empty function.
    let header = std::fs::read_to_string(dir.join("platform_support.h"))
        .expect("the emitted header is there to read");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        header.contains("markedTwelve") && header.contains("markedFour"),
        "the emitted condition does not carry its declared expression:\n{header}"
    );
}
