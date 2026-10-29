// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A byte string (`bytes`, SCE_FORGE.md §3.4), written for Swift as a read-only
// parameter: the `[UInt8]` whose length is `data.count` and whose bytes are read by
// their index, as the parameter of an algorithm that hashes it or counts the code
// points of it, and as the argument one algorithm hands another. A byte string a
// body builds, holds in a local or returns has no spelling yet and is refused by
// name, as every construct the Swift backend does not write is.

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

#[test]
fn a_byte_string_a_body_returns_is_refused_by_name() {
    let refusal = swift_of("stdlib/merge/hlc_text.scxml")
        .expect_err("a byte string a body returns is not written yet");
    assert!(
        refusal.contains("a <sce:return> of bytes"),
        "the refusal names the return: {refusal}"
    );
}
