// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a `sce:kind="procedure"` document produces for its two value-carrying
// surfaces — `<donedata><param expr>` and `<send sce:payload>`.
//
// Both were found the same way and failed the same way: the document was
// accepted with exit 0 and the EMITTED C++ did not compile. That is a refusal
// arriving in a consumer's build log instead of on the document that caused
// it, and it is the shape this file exists to keep closed.
//
//   * donedata — `doneData_` is a `map<string, string>`. The C++ template
//     assigned the author's expression straight in, so `expr="7"` emitted
//     `doneData_["n"] = 7;`. The Rust template had always written
//     `{{ expr }}.to_string()`; C++ was the one backend with no rendering.
//   * payload  — every runtime types it as a wire blob. A scalar field
//     emitted `req.payload = themeFileId_;`, assigning a `uint32_t` to an
//     `optional<vector<uint8_t>>`.
//
// Measured 2026-09-17 at `9d82f0d4f299`.

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

    fn emitted(&self) -> String {
        let mut all = String::new();
        for entry in std::fs::read_dir(&self.0).expect("read scratch dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "h") {
                all.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
        all
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    exit: Option<i32>,
    stderr: String,
    emitted: String,
}

fn generate(doc: &str, label: &str) -> Run {
    let out = Out::new(label);
    let path = out.0.join(format!("{label}.scxml"));
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
    Run {
        exit: run.status.code(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
        emitted: out.emitted(),
    }
}

fn procedure(datamodel: &str, send: &str, param: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="probe" initial="a" version="1.0">
  <datamodel>{datamodel}</datamodel>
  <state id="a">
    <onentry>{send}</onentry>
    <transition event="ok" target="d"/>
  </state>
  <final id="d"><donedata>{param}</donedata></final>
</scxml>
"#
    )
}

/// A donedata value is RENDERED, whatever its type.
///
/// The assertion is not the spelling of the helper but that the raw
/// expression never lands alone in the assignment: `doneData_["n"] = 7;` is
/// the exact text that did not compile.
#[test]
fn a_donedata_value_is_rendered_not_pasted() {
    for (expr, label) in [("7", "int"), ("'ok'", "str"), ("true", "bool")] {
        let run = generate(
            &procedure("", "", &format!(r#"<param name="n" expr="{expr}"/>"#)),
            &format!("donedata-{label}"),
        );
        assert_eq!(
            run.exit,
            Some(0),
            "{label}: generation refused:\n{}",
            run.stderr
        );
        let bare = run
            .emitted
            .lines()
            .find(|l| l.contains("doneData_[") && !l.contains("doneDataValue"));
        assert!(
            bare.is_none(),
            "{label}: a donedata value reached the map unrendered: {bare:?}"
        );
    }
}

/// A payload that is provably not bytes is refused, not emitted.
#[test]
fn a_scalar_payload_is_refused() {
    let run = generate(
        &procedure(
            r#"<data id="fileId" sce:type="uint32" sce:direction="in"/>"#,
            r#"<send sce:service="svc" sce:payload="fileId"/>"#,
            r#"<param name="n" expr="1"/>"#,
        ),
        "payload-scalar",
    );
    assert_ne!(
        run.exit,
        Some(0),
        "a uint32 payload generated instead of being refused:\n{}",
        run.emitted
    );
    assert!(
        run.stderr.contains("sce:payload"),
        "the refusal does not name the attribute that caused it:\n{}",
        run.stderr
    );
}

/// ⚠ And the refusal stays narrow. A bytes field is the documented shape and
/// must still pass — a check that refused it would be the same defect
/// pointing the other way.
#[test]
fn a_bytes_payload_still_generates() {
    let run = generate(
        &procedure(
            r#"<data id="blob" sce:type="bytes" sce:direction="in"/>"#,
            r#"<send sce:service="svc" sce:payload="blob"/>"#,
            r#"<param name="n" expr="1"/>"#,
        ),
        "payload-bytes",
    );
    assert_eq!(
        run.exit,
        Some(0),
        "a bytes payload was refused:\n{}",
        run.stderr
    );
    assert!(
        run.emitted.contains("req.payload"),
        "the payload never reached the request:\n{}",
        run.emitted
    );
}
