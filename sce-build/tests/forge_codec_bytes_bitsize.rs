// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `bytes` codec field carries a LENGTH, never a bit count.
//
// Found and failing the same way as the procedure donedata/payload defects
// and the lookup miss-default: the document was accepted with exit 0 and the
// EMITTED C++ DID NOT COMPILE. `sce:type="bytes"` lowers to a byte container
// (`std::vector<uint8_t>` / `Vec<u8>` / `uint8_t[CAP]`), while the decode
// path for a FIXED bit-size assembles an integer by shifting bytes together,
// so the two meet as
//
//     std::vector<uint8_t> dataRecord = static_cast<uint32_t>(...);
//
// ⚠ The mistake is the natural one, not an exotic one. A diagnostic frame
// table reads `| 4-6 | Data Record | Format: ASCII (3Byte) |`, and three
// bytes is twenty-four bits — so `sce:bit-size="24"` is what an author
// writes first. Measured 2026-09-17 while converting one.

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

fn codec(field: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame">
  <datamodel>
    <sce:field id="serviceId" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    {field}
  </datamodel>
</scxml>
"#
    )
}

fn generate(doc: &str, label: &str) -> (Option<i32>, String) {
    let out = Out::new(label);
    let path = out.0.join("frame.scxml");
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

/// A fixed bit-size on a bytes field is refused, and the refusal says where.
#[test]
fn a_bytes_field_with_a_fixed_bit_size_is_refused() {
    let (code, stderr) = generate(
        &codec(
            r#"<sce:field id="record" sce:type="bytes" sce:byte="1" sce:bit-size="24" sce:max-size="3"/>"#,
        ),
        "bytes-fixed",
    );
    assert_ne!(code, Some(0), "a 24-bit bytes field generated");
    assert!(
        stderr.contains("record"),
        "the refusal does not name the field:\n{stderr}"
    );
    // The way OUT matters as much as the refusal: an author who wrote a byte
    // run has to be told which spelling carries one.
    assert!(
        stderr.contains("length-ref") && stderr.contains("tail"),
        "the refusal does not name the spellings that do carry a byte run:\n{stderr}"
    );
}

/// ⚠ And the refusal stays narrow. `tail` is the documented shape for a byte
/// run and must still generate — a check that refused it would be the same
/// defect pointing the other way.
#[test]
fn a_tail_bytes_field_still_generates() {
    let (code, stderr) = generate(
        &codec(
            r#"<sce:field id="record" sce:type="bytes" sce:byte="1" sce:bit-size="tail" sce:max-size="3"/>"#,
        ),
        "bytes-tail",
    );
    assert_eq!(code, Some(0), "a tail bytes field was refused:\n{stderr}");
}

/// And so does `length-ref`, the other variable form.
#[test]
fn a_length_ref_bytes_field_still_generates() {
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame">
  <datamodel>
    <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="record" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="len" sce:max-size="16"/>
  </datamodel>
</scxml>
"#;
    let (code, stderr) = generate(doc, "bytes-lenref");
    assert_eq!(
        code,
        Some(0),
        "a length-ref bytes field was refused:\n{stderr}"
    );
}
