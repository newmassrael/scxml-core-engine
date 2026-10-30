// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A parent whose carrier holds, in a flag, the tag of a variant it embeds
//! writes for a catch-all body the tag that body holds (ADR 0058: a catch-all
//! presents the tag it could not name), masked to the flag's width.
//!
//! The decode of the parent hands the embedded variant the flag's bits as its
//! tag, and a tag no arm names reaches the catch-all, which keeps it. Bits the
//! encode derives from anything else make a frame that decodes into another
//! arm, or into the catch-all with another tag. The Rust source interface
//! derives them from the tag (`derive_parent_tag`); the C parent once added
//! nothing for a catch-all body, and the C++ parent the bits of its last arm,
//! which the catch-all's wrapper fell through to.
//!
//! The render tests hold the term each backend writes; the run tests compile
//! the C and C++ codecs, decode a frame whose tag no arm names, clear the flag
//! on the decoded value, and hold the encode to the original frame, since the
//! carrier byte is the stored header with the derived bits added.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The catch-all's wrapper the C++ dispatcher declares next to its struct.
const CPP_WRAPPER: &str = "::SCE::Generated::CaChild::CaChildDefault";

/// A parent that dispatches an embedded tag-less variant on two bits of its
/// header, written with its documents to a fresh directory the caller
/// removes. The variant's arms are 0x00 (one byte; under the feature
/// `local-arm` when `gated`) and 0x01 (two bytes, the default arm), and its
/// catch-all reads one byte.
fn catch_all_fixture(gated: bool) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sce_catch_all_parent_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("make the fixture directory");
    let local_feature = if gated { r#" feature="local-arm""# } else { "" };
    let body = |name: &str, ty: &str, bits: u32| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}">
  <datamodel>
    <sce:field id="payload" sce:type="{ty}" sce:byte="0" sce:bit-size="{bits}"/>
  </datamodel>
</scxml>"#
        )
    };
    let documents = [
        (
            "ca_parent",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="ca_parent">
  <sce:import src="ca_child.scxml" kind="codec" as="ca_child">
    <sce:variant-dispatch flag="header.K"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="K" bit="5" width="2"/>
    </sce:flags>
    <sce:embed id="key" type="ca_child" sce:byte="1"/>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        (
            "ca_child",
            format!(
                r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="ca_child">
  <sce:import src="ca_local.scxml" kind="codec" as="ca_local"/>
  <sce:import src="ca_wide.scxml" kind="codec" as="ca_wide"/>
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="ca_local"{local_feature}/>
      <sce:arm value="0x01" type="ca_wide" default="true"/>
      <sce:default type="ca_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#
            ),
        ),
        ("ca_local", body("ca_local", "uint8", 8)),
        ("ca_wide", body("ca_wide", "uint16", 16)),
    ];
    for (name, text) in documents {
        std::fs::write(dir.join(format!("{name}.scxml")), text).expect("write a fixture");
    }
    dir
}

/// Every file `language` writes for the fixture's documents, each once, by
/// its file name.
fn generate_all(dir: &Path, language: Language) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for stem in ["ca_local", "ca_wide", "ca_child", "ca_parent"] {
        let text =
            std::fs::read_to_string(dir.join(format!("{stem}.scxml"))).expect("read a fixture");
        let output = sce_build::compile_forge_with_imports(
            &text,
            DocumentLabel::symmetric(stem),
            language,
            dir,
            &ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{stem} generates for {language:?}: {e}"));
        for (name, code) in output.files {
            let base = Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a generated file has a name")
                .to_string();
            if !files.iter().any(|(seen, _)| *seen == base) {
                files.push((base, code));
            }
        }
    }
    files
}

/// The parent's generated file for `language`.
fn parent_code(gated: bool, language: Language) -> String {
    let dir = catch_all_fixture(gated);
    let files = generate_all(&dir, language);
    let _ = std::fs::remove_dir_all(&dir);
    files
        .into_iter()
        .find(|(name, _)| name.contains("ca_parent"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the parent's file for {language:?}"))
}

/// The C++ term of the catch-all: its wrapper, found by type, fills `K` with
/// the tag it holds.
fn cpp_catch_all_term() -> String {
    format!(
        "std::get_if<{CPP_WRAPPER}>(&key.body) != nullptr ? \
         static_cast<std::uint8_t>((std::get_if<{CPP_WRAPPER}>(&key.body)->tag & 0x3u) << 5)"
    )
}

/// The Rust source interface reads the tag a catch-all body holds; the C and
/// C++ terms below are held to the same rule.
#[test]
fn the_rust_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Rust);
    assert!(
        code.contains("CaChildVariantSource::Default { tag, .. } => u8::from(tag) & 0x3"),
        "the Rust parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// The C parent's switch on the embedded body's kind has a case for the
/// catch-all, which adds the tag the body keeps in `default_tag`, before the
/// `default` that adds nothing.
#[test]
fn the_c_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::C11);
    let switch = [
        "    uint8_t _derived_header = 0u;",
        "    switch (self->key.body.kind) {",
        "    case CA_CHILD_BODY_KIND_CA_LOCAL: _derived_header |= 0x00u; break;",
        "    case CA_CHILD_BODY_KIND_CA_WIDE: _derived_header |= 0x20u; break;",
        "    case CA_CHILD_BODY_KIND_DEFAULT: _derived_header |= (uint8_t)((self->key.body.default_tag & 0x3u) << 5); break;",
        "    default: break;",
        "    }",
    ]
    .join("\n");
    assert!(
        code.contains(&switch),
        "the C parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// The C++ parent tests for the catch-all's wrapper ahead of the arms, whose
/// last one is the fallthrough the wrapper reached before.
#[test]
fn the_cpp_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Cpp);
    let derived = format!(
        "        const std::uint8_t _derived_header = static_cast<std::uint8_t>(({} : \
         key.body.index() == 0 ? static_cast<std::uint8_t>(0x00) : \
         static_cast<std::uint8_t>(0x20)));\n",
        cpp_catch_all_term()
    );
    assert!(
        code.contains(&derived),
        "the C++ parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// Over a variant with an arm under a build feature (docs/adr/0072) the
/// catch-all's term is first, outside every guard, and the arms after it count
/// their index per build as before.
#[test]
fn the_cpp_parent_of_a_gated_variant_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(true, Language::Cpp);
    let derived = [
        "        const std::uint8_t _derived_header = static_cast<std::uint8_t>((".to_string(),
        format!("            {} :", cpp_catch_all_term()),
        "#if SCE_FEATURE_LOCAL_ARM".to_string(),
        "            key.body.index() == static_cast<std::size_t>(0) ? static_cast<std::uint8_t>(0x00) :"
            .to_string(),
        "#endif".to_string(),
        "            static_cast<std::uint8_t>(0x20)));".to_string(),
    ]
    .join("\n");
    assert!(
        code.contains(&derived),
        "the C++ parent of a gated variant does not derive K from the catch-all's tag:\n{code}"
    );
}

// ── running what was generated ──────────────────────────────────────────

/// A fresh directory for one run, holding every header the fixture generates
/// for `language`.
fn project(language: Language, label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let fixture = catch_all_fixture(false);
    let files = generate_all(&fixture, language);
    let _ = std::fs::remove_dir_all(&fixture);
    let dir = std::env::temp_dir().join(format!(
        "sce_catch_all_{label}_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("make the project directory");
    for (name, code) in files {
        if name.ends_with(".h") {
            std::fs::write(dir.join(name), code).expect("write a header");
        }
    }
    dir
}

/// The include directory of a runtime under `backends/`.
fn runtime_include(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
        .join("include")
        .canonicalize()
        .unwrap_or_else(|e| panic!("{rel}/include: {e}"))
}

/// Run `command` in `dir` and fail with what it said when it fails.
fn run(mut command: Command, dir: &Path, what: &str) {
    let out = command
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("run {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed in {}:\n{}{}",
        dir.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

const C_DRIVER: &str = r#"/* Decode a frame whose tag no arm names, clear the tag on the
 * decoded value, and hold the encode to the frame: the carrier's bits come
 * from the catch-all's tag, not from the stored header. */
#include "ca_parent.h"
#include <stdio.h>
#include <string.h>

static int fail(const char *what) {
    printf("%s\n", what);
    return 1;
}

int main(void) {
    int failed = 0;
    /* The header holds mid (0x1d) and, in K (bits 5 and 6), the tag 3, which
     * no arm names; the catch-all's body is one byte. */
    const uint8_t frame[2] = { 0x7du, 0x07u };
    sce_forge_cursor_t cursor = sce_forge_cursor_init(frame, sizeof frame);
    ca_parent_t view;
    memset(&view, 0, sizeof view);
    if (ca_parent_decode(&cursor, &view) != SCE_FORGE_CODEC_OK) {
        return fail("the frame must decode");
    }
    if (view.key.body.kind != CA_CHILD_BODY_KIND_DEFAULT || view.key.body.default_tag != 3u
        || view.key.body.arm.default_body.payload != 0x07u) {
        failed |= fail("tag 3 must reach the catch-all, which keeps it");
    }
    view.header = (uint8_t)(view.header & 0x1Fu);
    uint8_t buf[8];
    sce_forge_writer_t writer = sce_forge_writer_init_buf(buf, sizeof buf);
    if (ca_parent_encode(&view, &writer) != SCE_FORGE_CODEC_OK) {
        return fail("the decoded value must encode");
    }
    if (writer.pos != sizeof frame || memcmp(buf, frame, sizeof frame) != 0) {
        failed |= fail("the encode must write the catch-all's tag into K (header 0x7d)");
    }
    ca_parent_t again;
    memset(&again, 0, sizeof again);
    cursor = sce_forge_cursor_init(buf, writer.pos);
    if (ca_parent_decode(&cursor, &again) != SCE_FORGE_CODEC_OK
        || again.key.body.kind != CA_CHILD_BODY_KIND_DEFAULT || again.key.body.default_tag != 3u) {
        failed |= fail("the written frame must decode into the catch-all with tag 3");
    }
    return failed;
}
"#;

const CPP_DRIVER: &str = r#"// Decode a frame whose tag no arm names, clear the tag on the decoded
// value, and hold the encode to the frame: the carrier's bits come from the
// catch-all's tag, not from the stored header or the last arm.
#include "ca_parent.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <variant>
#include <vector>

namespace {

using Parent = ::SCE::Generated::CaParent::CaParent;
using CatchAll = ::SCE::Generated::CaChild::CaChildDefault;

int fail(const char* what) {
    std::printf("%s\n", what);
    return 1;
}

}  // namespace

int main() {
    int failed = 0;
    // The header holds mid (0x1d) and, in K (bits 5 and 6), the tag 3, which
    // no arm names; the catch-all's body is one byte.
    const std::vector<std::uint8_t> frame{0x7d, 0x07};
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    std::optional<Parent> decoded = Parent::decode(cursor);
    if (!decoded.has_value()) {
        return fail("the frame must decode");
    }
    const CatchAll* held = std::get_if<CatchAll>(&decoded->key.body);
    if (held == nullptr || held->tag != 3 || held->body.payload != 0x07) {
        failed |= fail("tag 3 must reach the catch-all, which keeps it");
    }
    decoded->header = static_cast<std::uint8_t>(decoded->header & 0x1Fu);
    std::vector<std::uint8_t> written;
    ::SCE::Forge::VectorSink sink(written);
    if (decoded->encode(sink).has_value()) {
        return fail("the decoded value must encode");
    }
    if (written != frame) {
        failed |= fail("the encode must write the catch-all's tag into K (header 0x7d)");
    }
    ::SCE::Forge::SceCursor again_cursor(written.data(), written.size());
    const std::optional<Parent> again = Parent::decode(again_cursor);
    const CatchAll* again_held = again.has_value() ? std::get_if<CatchAll>(&again->key.body) : nullptr;
    if (again_held == nullptr || again_held->tag != 3) {
        failed |= fail("the written frame must decode into the catch-all with tag 3");
    }
    return failed;
}
"#;

/// The C parent writes the frame it decoded from a catch-all body.
#[test]
fn the_c_parent_of_a_catch_all_round_trips_its_frame() {
    let Some(gcc) =
        sce_build::toolchain::require_or_skip("gcc", "compile and run the C parent of a catch-all")
    else {
        return;
    };
    let dir = project(Language::C11, "c");
    std::fs::write(dir.join("driver.c"), C_DRIVER).expect("write the driver");
    let mut build = Command::new(gcc);
    build
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(format!(
            "-I{}",
            runtime_include("backends/c/forge-runtime").display()
        ))
        .arg(format!(
            "-I{}",
            runtime_include("backends/c/runtime").display()
        ))
        .arg(format!("-I{}", dir.display()))
        .args(["-o", "driver", "driver.c"]);
    run(build, &dir, "gcc");
    run(Command::new(dir.join("driver")), &dir, "the C driver");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The C++ parent writes the frame it decoded from a catch-all body.
#[test]
fn the_cpp_parent_of_a_catch_all_round_trips_its_frame() {
    let Some(gxx) = sce_build::toolchain::require_or_skip(
        "g++",
        "compile and run the C++ parent of a catch-all",
    ) else {
        return;
    };
    let dir = project(Language::Cpp, "cpp");
    std::fs::write(dir.join("driver.cpp"), CPP_DRIVER).expect("write the driver");
    let mut build = Command::new(gxx);
    build
        .args(["-std=c++17", "-Wall", "-Wextra", "-Werror"])
        .arg(format!(
            "-I{}",
            runtime_include("backends/cpp/forge-runtime").display()
        ))
        .arg(format!("-I{}", dir.display()))
        .args(["-o", "driver", "driver.cpp"]);
    run(build, &dir, "g++");
    run(Command::new(dir.join("driver")), &dir, "the C++ driver");
    let _ = std::fs::remove_dir_all(&dir);
}
