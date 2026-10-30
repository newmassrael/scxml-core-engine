// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A parent whose carrier holds, in a flag, the tag of a variant it embeds
//! writes in that flag what the embedded body says: the tag of its arm, or for
//! a catch-all body the tag that body holds (ADR 0058: a catch-all presents
//! the tag it could not name), masked to the flag's width. The flag is the
//! body's, so bits the stored carrier holds there are not written.
//!
//! The decode of the parent hands the embedded variant the flag's bits as its
//! tag, and a tag no arm names reaches the catch-all, which keeps it. Bits the
//! encode derives from anything else make a frame that decodes into another
//! arm, or into the catch-all with another tag. The Rust source interface
//! derives the flag from the body and writes it in place of the stored one
//! (`derive_parent_tag`). The other backends once differed: C added nothing
//! for a catch-all body and C++ the bits of the last arm, and each added the
//! derived bits to the stored carrier, so stale bits there were written too.
//!
//! The render tests hold the term each backend writes and its carrier write;
//! the run tests compile the C and C++ codecs where gcc and g++ are present,
//! decode a frame, store stale bits in the flag, and hold the encode to the
//! frame: once for a catch-all body, once for an arm's.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The catch-all's wrapper the C++ dispatcher declares next to its struct.
const CPP_WRAPPER: &str = "::SCE::Generated::CaChild::CaChildDefault";

/// The module the Go codecs are generated into, and the run test's
/// `go.mod` declares.
const GO_MODULE: &str = "example.com/sce-catch-all";

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

/// The options `language` is generated with: Go names the module its
/// codecs import one another from.
fn options(language: Language) -> ForgeCompileOptions {
    let mut options = ForgeCompileOptions::default();
    if matches!(language, Language::Go) {
        options.go_module_prefix = Some(GO_MODULE.to_string());
    }
    options
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
            &options(language),
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

/// The parent's generated file for `language`, which Kotlin names by the
/// parent's type and every other backend by its document.
fn parent_code(gated: bool, language: Language) -> String {
    let dir = catch_all_fixture(gated);
    let files = generate_all(&dir, language);
    let _ = std::fs::remove_dir_all(&dir);
    files
        .into_iter()
        .find(|(name, _)| name.contains("ca_parent") || name.contains("CaParent"))
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

/// Assert that `code` holds `wanted`, naming `what` when it does not.
fn assert_holds(code: &str, wanted: &str, what: &str) {
    assert!(
        code.contains(wanted),
        "{what}; wanted\n{wanted}\nin:\n{code}"
    );
}

// ── the term of a catch-all body ────────────────────────────────────────

/// The Rust source interface reads the tag a catch-all body holds; the other
/// backends' terms below are held to the same rule.
#[test]
fn the_rust_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Rust);
    assert_holds(
        &code,
        "CaChildVariantSource::Default { tag, .. } => u8::from(tag) & 0x3",
        "the Rust parent does not derive K from the catch-all's tag",
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
    assert_holds(
        &code,
        &switch,
        "the C parent does not derive K from the catch-all's tag",
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
    assert_holds(
        &code,
        &derived,
        "the C++ parent does not derive K from the catch-all's tag",
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
    assert_holds(
        &code,
        &derived,
        "the C++ parent of a gated variant does not derive K from the catch-all's tag",
    );
}

// ── the carrier: the stored flag cleared, the derived bits written ──────

/// The C parent clears K in the stored header before it adds the derived bits.
#[test]
fn the_c_parent_writes_k_from_the_body_alone() {
    let code = parent_code(false, Language::C11);
    assert_holds(
        &code,
        "sce_forge_writer_write_u8(w, (self->header & 0x9Fu) | _derived_header)",
        "the C parent writes the stored K bits with the derived ones",
    );
}

/// The C++ parent clears K in the stored header before it adds the derived
/// bits.
#[test]
fn the_cpp_parent_writes_k_from_the_body_alone() {
    let code = parent_code(false, Language::Cpp);
    assert_holds(
        &code,
        "w.write_u8((header & 0x9Fu) | _derived_header);",
        "the C++ parent writes the stored K bits with the derived ones",
    );
}

/// The Python parent clears K in the stored header before it adds the derived
/// bits.
#[test]
fn the_python_parent_writes_k_from_the_body_alone() {
    let code = parent_code(false, Language::Python);
    assert_holds(
        &code,
        "w.write_u8(((self.header & 0x9F) | _derived_header) & 0xFF)",
        "the Python parent writes the stored K bits with the derived ones",
    );
}

/// The Go parent clears K in the stored header before it adds the derived
/// bits.
#[test]
fn the_go_parent_writes_k_from_the_body_alone() {
    let code = parent_code(false, Language::Go);
    assert_holds(
        &code,
        "w.WriteBytes([]byte{ (s.Header & 0x9F) | _derivedHeader })",
        "the Go parent writes the stored K bits with the derived ones",
    );
}

/// The Kotlin parent clears K in the stored header before it adds the derived
/// bits.
#[test]
fn the_kotlin_parent_writes_k_from_the_body_alone() {
    let code = parent_code(false, Language::Kotlin);
    assert_holds(
        &code,
        "w.writeU8(((this.header and 0x9Fu.toUByte()) or _derivedHeader).toByte())",
        "the Kotlin parent writes the stored K bits with the derived ones",
    );
}

// ── running what was generated ──────────────────────────────────────────
//
// Every driver runs the same two frames. The header holds mid (0x1d) and, in K
// (bits 5 and 6), the tag:
//   - 0x5d 0x07: tag 2, which no arm names, so the catch-all reads its one
//     byte; the decoded value's header is then given K = 1 (0x3d);
//   - 0x3d 0x12 0x34: tag 1, the two-byte arm; its header is given K = 2
//     (0x5d).
// Each encode must write the frame back: K from the body, not from the
// stored header, which would make 0x7d of either.

/// A fresh directory for one run, holding every file of extension `ext` the
/// fixture generates for `language`; a Go file is in a directory of its own,
/// named by its package.
fn project(language: Language, ext: &str, label: &str) -> PathBuf {
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
        let path = Path::new(&name);
        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }
        let target = if matches!(language, Language::Go) {
            let package = dir.join(path.file_stem().expect("a Go file has a stem"));
            std::fs::create_dir_all(&package).expect("make a package directory");
            package
        } else {
            dir.clone()
        };
        std::fs::write(target.join(name), code).expect("write a generated file");
    }
    dir
}

/// A directory of a runtime under `backends/`.
fn backend(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
        .canonicalize()
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
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

const C_DRIVER: &str = r#"/* Decode each frame, store stale bits in K, and hold the encode to the
 * frame: K comes from the body, not from the stored header. */
#include "ca_parent.h"
#include <stdio.h>
#include <string.h>

static int fail(const char *what) {
    printf("%s\n", what);
    return 1;
}

/* Decode `frame` into `view`, store `stale` as its header, and encode: the
 * bytes must be the frame's. */
static int rewrite(const uint8_t *frame, size_t len, uint8_t stale, ca_parent_t *view) {
    sce_forge_cursor_t cursor = sce_forge_cursor_init(frame, len);
    memset(view, 0, sizeof *view);
    if (ca_parent_decode(&cursor, view) != SCE_FORGE_CODEC_OK) {
        return fail("the frame must decode");
    }
    view->header = stale;
    uint8_t buf[8];
    sce_forge_writer_t writer = sce_forge_writer_init_buf(buf, sizeof buf);
    if (ca_parent_encode(view, &writer) != SCE_FORGE_CODEC_OK) {
        return fail("the decoded value must encode");
    }
    if (writer.pos != len || memcmp(buf, frame, len) != 0) {
        return fail("the encode must write K from the body");
    }
    return 0;
}

int main(void) {
    int failed = 0;
    ca_parent_t view;
    const uint8_t catch_all[2] = { 0x5du, 0x07u };
    failed |= rewrite(catch_all, sizeof catch_all, 0x3du, &view);
    if (view.key.body.kind != CA_CHILD_BODY_KIND_DEFAULT || view.key.body.default_tag != 2u
        || view.key.body.arm.default_body.payload != 0x07u) {
        failed |= fail("tag 2 must reach the catch-all, which keeps it");
    }
    const uint8_t wide[3] = { 0x3du, 0x12u, 0x34u };
    failed |= rewrite(wide, sizeof wide, 0x5du, &view);
    if (view.key.body.kind != CA_CHILD_BODY_KIND_CA_WIDE || view.key.body.arm.ca_wide.payload != 0x1234u) {
        failed |= fail("tag 1 must reach the two-byte arm");
    }
    return failed;
}
"#;

const CPP_DRIVER: &str = r#"// Decode each frame, store stale bits in K, and hold the encode to the
// frame: K comes from the body, not from the stored header.
#include "ca_parent.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <variant>
#include <vector>

namespace {

using Parent = ::SCE::Generated::CaParent::CaParent;
using CatchAll = ::SCE::Generated::CaChild::CaChildDefault;
using Wide = ::SCE::Generated::CaWide::CaWide;

int fail(const char* what) {
    std::printf("%s\n", what);
    return 1;
}

// Decode `frame`, store `stale` as its header, and encode: the bytes must be
// the frame's.
std::optional<Parent> rewrite(const std::vector<std::uint8_t>& frame, std::uint8_t stale, int& failed) {
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    std::optional<Parent> decoded = Parent::decode(cursor);
    if (!decoded.has_value()) {
        failed |= fail("the frame must decode");
        return decoded;
    }
    decoded->header = stale;
    std::vector<std::uint8_t> written;
    ::SCE::Forge::VectorSink sink(written);
    if (decoded->encode(sink).has_value()) {
        failed |= fail("the decoded value must encode");
    } else if (written != frame) {
        failed |= fail("the encode must write K from the body");
    }
    return decoded;
}

}  // namespace

int main() {
    int failed = 0;
    const std::optional<Parent> held = rewrite({0x5d, 0x07}, 0x3d, failed);
    const CatchAll* catch_all = held.has_value() ? std::get_if<CatchAll>(&held->key.body) : nullptr;
    if (catch_all == nullptr || catch_all->tag != 2 || catch_all->body.payload != 0x07) {
        failed |= fail("tag 2 must reach the catch-all, which keeps it");
    }
    const std::optional<Parent> named = rewrite({0x3d, 0x12, 0x34}, 0x5d, failed);
    const Wide* wide = named.has_value() ? std::get_if<Wide>(&named->key.body) : nullptr;
    if (wide == nullptr || wide->payload != 0x1234) {
        failed |= fail("tag 1 must reach the two-byte arm");
    }
    return failed;
}
"#;

/// The C parent writes back each frame it decoded, whatever its stored K.
#[test]
fn the_c_parent_writes_back_each_frame() {
    let Some(gcc) =
        sce_build::toolchain::require_or_skip("gcc", "compile and run the C parent of a catch-all")
    else {
        return;
    };
    let dir = project(Language::C11, "h", "c");
    std::fs::write(dir.join("driver.c"), C_DRIVER).expect("write the driver");
    let mut build = Command::new(gcc);
    build
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(format!(
            "-I{}",
            backend("backends/c/forge-runtime/include").display()
        ))
        .arg(format!(
            "-I{}",
            backend("backends/c/runtime/include").display()
        ))
        .arg(format!("-I{}", dir.display()))
        .args(["-o", "driver", "driver.c"]);
    run(build, &dir, "gcc");
    run(Command::new(dir.join("driver")), &dir, "the C driver");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The C++ parent writes back each frame it decoded, whatever its stored K.
#[test]
fn the_cpp_parent_writes_back_each_frame() {
    let Some(gxx) = sce_build::toolchain::require_or_skip(
        "g++",
        "compile and run the C++ parent of a catch-all",
    ) else {
        return;
    };
    let dir = project(Language::Cpp, "h", "cpp");
    std::fs::write(dir.join("driver.cpp"), CPP_DRIVER).expect("write the driver");
    let mut build = Command::new(gxx);
    build
        .args(["-std=c++17", "-Wall", "-Wextra", "-Werror"])
        .arg(format!(
            "-I{}",
            backend("backends/cpp/forge-runtime/include").display()
        ))
        .arg(format!("-I{}", dir.display()))
        .args(["-o", "driver", "driver.cpp"]);
    run(build, &dir, "g++");
    run(Command::new(dir.join("driver")), &dir, "the C++ driver");
    let _ = std::fs::remove_dir_all(&dir);
}
