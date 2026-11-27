// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A C11 test-vector sidecar (`<stem>_test.h`) reports a row that does not
//! hold through the log seam (`sce/log.h`, docs/adr/0110 D2), by the one
//! report both sidecars write through (`sce/forge/test_vector.h`), so it
//! includes no hosted header and runs wherever its unit does (D1).
//!
//! What holds that here is the run: a document of each kind whose rows hold
//! and fail, generated for C11, linked with the host's seams
//! (`backends/c/runtime/posix/log.c`) and run. A row that holds writes
//! nothing; each one that fails writes one line, labelled by its unit, naming
//! the line of the document the row stands on and what it found, with each
//! number spelled by `sce/text.h`; and the count the sidecar returns is the
//! number of lines.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// CRC16-CCITT-FALSE, whose first row holds and whose second, of no bytes,
/// states a value that is not the empty input's 0xFFFF: an unsigned value is
/// written in hexadecimal.
const UNSIGNED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="row_crc16"
       version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="crc" type="uint16" init="0xFFFF"/>
    <sce:foreach item="b" in="data">
      <sce:var name="hi" type="uint16" init="b"/>
      <sce:assign target="crc" expr="crc ^ (hi &lt;&lt; 8)"/>
      <sce:var name="i" type="uint8" init="0"/>
      <sce:while cond="i &lt; 8" max-iter="8">
        <sce:if cond="(crc &amp; 0x8000) !== 0">
          <sce:assign target="crc" expr="(crc &lt;&lt; 1) ^ 0x1021"/>
          <sce:else>
            <sce:assign target="crc" expr="crc &lt;&lt; 1"/>
          </sce:else>
        </sce:if>
        <sce:assign target="i" expr="i + 1"/>
      </sce:while>
    </sce:foreach>
    <sce:return expr="crc"/>
  </sce:body>
  <sce:test-vector hex="313233343536373839" value="0x29B1"/>
  <sce:test-vector hex="" value="0x1234"/>
</scxml>
"#;

/// An algorithm that may fail: its first row has a value that is not the one
/// it states, written in decimal with its sign, and its second, of no bytes,
/// fails reading the first byte, and names the failure.
const SIGNED_MAY_FAIL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="row_negate"
       version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="int32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="int32" init="0"/>
    <sce:if cond="data[0] === 5">
      <sce:assign target="r" expr="-5"/>
    </sce:if>
    <sce:return expr="r"/>
  </sce:body>
  <sce:test-vector hex="05" value="-4"/>
  <sce:test-vector hex="" value="0"/>
</scxml>
"#;

/// An algorithm with a boolean value, written as `true` or `false`.
const BOOLEAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="row_empty"
       version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="len(data) === 0"/>
  </sce:body>
  <sce:test-vector hex="00" value="true"/>
</scxml>
"#;

/// A codec whose first row holds and whose second states a field its bytes do
/// not carry: the encode writes other bytes, and the decode reads another
/// value.
const CODEC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="row_close">
  <datamodel>
    <sce:field id="reason" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
  <sce:test-vector hex="00">
    <sce:decoded field="reason" value="0"/>
  </sce:test-vector>
  <sce:test-vector hex="01">
    <sce:decoded field="reason" value="2"/>
  </sce:test-vector>
</scxml>
"#;

/// The line of `document`, counted from 1, that first holds `needle`: the line a
/// row's report names.
fn line_of(document: &str, needle: &str) -> usize {
    document
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("the document holds `{needle}`"))
        + 1
}

/// What running the sidecar of `document`, whose unit is `stem`, came to: the
/// count of rows that failed, which the program exits with, and what it wrote
/// to `stderr`. `None` when no C compiler is on the path.
fn run_rows(document: &str, stem: &str) -> Option<(i32, String)> {
    let Some(cc) = sce_build::toolchain::locate_any(&["gcc", "cc"]) else {
        sce_build::toolchain::skipped(&format!(
            "the test-vector sidecar of {stem}: no C compiler on PATH"
        ));
        return None;
    };
    let dir = tempdir().expect("tempdir");
    let output = sce_build::compile_forge_with_imports(
        document,
        sce_build::DocumentLabel::symmetric(stem),
        sce_build::generator::Language::C11,
        dir.path(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{stem} generates for C11: {e}"));
    let sidecar = format!("{stem}_test.h");
    assert!(
        output.files.iter().any(|(name, _)| *name == sidecar),
        "{stem} writes {sidecar}: {:?}",
        output
            .files
            .iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>()
    );
    for (name, text) in &output.files {
        std::fs::write(dir.path().join(name), text).expect("write a generated file");
    }
    let driver = dir.path().join("driver.c");
    std::fs::write(
        &driver,
        format!(
            "#include \"{sidecar}\"\nint main(void) {{\n    return test_vector_{stem}();\n}}\n"
        ),
    )
    .expect("write the driver");
    let bin = dir.path().join("driver");
    let built = Command::new(&cc)
        .args([
            "-std=c11",
            "-O0",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
        ])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(repo_root().join("backends/c/forge-runtime/include"))
        .arg("-I")
        .arg(dir.path())
        .arg(&driver)
        .arg(repo_root().join("backends/c/runtime/posix/log.c"))
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run the C compiler");
    assert!(
        built.status.success(),
        "the sidecar of {stem}, the host's seams and the driver build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&bin).output().expect("run the driver");
    let failed = ran
        .status
        .code()
        .unwrap_or_else(|| panic!("the driver of {stem} exits with a status"));
    Some((failed, String::from_utf8_lossy(&ran.stderr).into_owned()))
}

#[test]
fn an_unsigned_value_that_does_not_hold_is_reported_in_hexadecimal() {
    let Some((failed, lines)) = run_rows(UNSIGNED, "row_crc16") else {
        return;
    };
    let row = line_of(UNSIGNED, r#"hex="" value="0x1234""#);
    assert_eq!(
        lines,
        format!("row_crc16: test_vector @SCXML L{row}: actual=0xffff expected=0x1234\n")
    );
    assert_eq!(failed, 1, "one row of two fails:\n{lines}");
}

#[test]
fn a_signed_value_and_a_failure_are_reported_as_the_row_found_them() {
    let Some((failed, lines)) = run_rows(SIGNED_MAY_FAIL, "row_negate") else {
        return;
    };
    let wrong = line_of(SIGNED_MAY_FAIL, r#"hex="05""#);
    let empty = line_of(SIGNED_MAY_FAIL, r#"hex="""#);
    assert_eq!(
        lines,
        format!(
            "row_negate: test_vector @SCXML L{wrong}: actual=-5 expected=-4\n\
             row_negate: test_vector @SCXML L{empty}: failed with out-of-range\n"
        )
    );
    assert_eq!(failed, 2, "both rows fail:\n{lines}");
}

#[test]
fn a_boolean_value_that_does_not_hold_is_reported_as_a_word() {
    let Some((failed, lines)) = run_rows(BOOLEAN, "row_empty") else {
        return;
    };
    let row = line_of(BOOLEAN, "<sce:test-vector");
    assert_eq!(
        lines,
        format!("row_empty: test_vector @SCXML L{row}: actual=false expected=true\n")
    );
    assert_eq!(failed, 1, "the one row fails:\n{lines}");
}

#[test]
fn a_codec_row_that_does_not_hold_reports_its_encode_and_its_field() {
    let Some((failed, lines)) = run_rows(CODEC, "row_close") else {
        return;
    };
    let row = line_of(CODEC, r#"hex="01""#);
    assert_eq!(
        lines,
        format!(
            "row_close: test_vector @SCXML L{row}: encode length=1 (expected 1)\n\
             row_close: test_vector @SCXML L{row}: field `reason` mismatch\n"
        )
    );
    assert_eq!(
        failed, 2,
        "the second row fails its encode and its decode:\n{lines}"
    );
}
