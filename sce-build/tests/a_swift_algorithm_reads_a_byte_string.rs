// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A byte string (`bytes`, SCE_FORGE.md §3.4), written for Swift as the `[UInt8]` whose
// length is `data.count` and whose bytes are read by their index: the parameter of an
// algorithm that hashes it or counts the code points of it, the argument one algorithm
// hands another, and (SCE_FORGE.md §4.12, byte-buffer-build) the one buffer a body fills
// by appending and returns by name.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn a_byte_string_parameter_is_an_array_of_bytes_read_by_its_count_and_index() {
    let code =
        swift_of("stdlib/hash/murmur3_32.scxml").expect("a byte string parameter is written");
    assert!(
        code.contains("murmur332(_ data: [UInt8], _ seed: UInt32) -> UInt32"),
        "the parameter is `[UInt8]`: {code}"
    );
    assert!(
        code.contains("UInt32(truncatingIfNeeded: data.count)"),
        "the length is the count: {code}"
    );
    assert!(
        code.contains("UInt64(data[Int(j)])"),
        "a byte is read by its index and widened before it is shifted: {code}"
    );
}

#[test]
fn a_byte_read_of_a_body_that_may_fail_is_a_checked_read() {
    let code =
        swift_of("stdlib/text/utf8_length.scxml").expect("a byte string parameter is written");
    assert!(
        code.contains("Checked.at(sceFailure, data, i, fallback: 0)"),
        "an index outside the bytes fails and is not read: {code}"
    );
}

#[test]
fn a_byte_string_is_handed_on_to_the_algorithm_that_measures_it() {
    let code = swift_of("stdlib/text/utf8_within.scxml").expect("a byte string is passed on");
    assert!(
        code.contains("Utf8Length.utf8Length(data)"),
        "the callee is given the bytes as they are: {code}"
    );
}

/// A body that builds a byte string holds it in an `Array` of bytes pre-sized for the
/// capacity its author declared, appends a byte or a byte string to it, reads its length
/// and bytes back, and returns it by name.
#[test]
fn a_byte_string_a_body_builds_is_an_array_it_appends_to_and_returns() {
    let code = swift_of("tests/forge/resources/algorithm_bytes_squeeze.scxml")
        .expect("a byte string a body builds is written");
    for needle in [
        "var out: [UInt8] = []",
        "out.reserveCapacity(16)",
        "out.append(b)",
        "-> [UInt8]",
        "out.count",
    ] {
        assert!(
            code.contains(needle),
            "the generated Swift holds `{needle}`: {code}"
        );
    }
}

/// The text a standard algorithm builds its output with: a byte at a time, into the buffer
/// its `capacity` sizes, returned as the bytes it is.
#[test]
fn a_standard_algorithm_builds_its_bytes_a_byte_at_a_time() {
    let code =
        swift_of("stdlib/merge/hlc_text.scxml").expect("a byte string a body returns is written");
    for needle in [
        "var out: [UInt8] = []",
        "out.reserveCapacity(46)",
        "out.append(digit)",
        "-> [UInt8]",
    ] {
        assert!(
            code.contains(needle),
            "the generated Swift holds `{needle}`: {code}"
        );
    }
}

/// A byte string appended whole extends the buffer by its bytes.
#[test]
fn a_byte_string_appended_whole_extends_the_buffer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("algorithm_bytes_copy.scxml");
    std::fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="algorithm_bytes_copy" version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="bytes" returns-max-size="16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="16"/>
    <sce:append target="out" expr="data"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#,
    )
    .expect("write the document");
    let output = compile_forge_file(&path, Language::Swift, &[], &ForgeCompileOptions::default())
        .map_err(|e| format!("{e:?}"))
        .expect("an append of a byte string is written");
    let code = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file")
        .1;
    assert!(
        code.contains("out.append(contentsOf: data)"),
        "the whole byte string extends the buffer: {code}"
    );
}
