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
//! (`derive_parent_tag`). The other backends once differed: C and Go added
//! nothing for a catch-all body, C++ and Python the bits of the last arm,
//! Kotlin named no catch-all in an exhaustive `when`, and each added the
//! derived bits to the stored carrier, so stale bits there were written too.
//!
//! The render tests hold the term each backend writes and its carrier write;
//! the run tests compile each backend's codecs where its toolchain is
//! present, decode a frame, store stale bits in the flag, and hold the encode
//! to the frame: once for a catch-all body, once for an arm's.

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

/// The Python parent tests the catch-all's `kind` ahead of the arms, whose
/// last one is the fallthrough the catch-all reached before, and reads the
/// tag the body keeps in `default_tag`.
#[test]
fn the_python_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Python);
    assert_holds(
        &code,
        "        _derived_header = (((self.key.body.default_tag & 0x3) << 5) \
         if self.key.body.kind == \"Default\" \
         else ((0x00 if self.key.body.kind == \"CaLocal\" else 0x20)))\n",
        "the Python parent does not derive K from the catch-all's tag",
    );
}

/// The Go parent's switch on which arm's field is set has a case for the
/// catch-all's, which adds the tag it holds.
#[test]
fn the_go_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Go);
    let switch = [
        "\tvar _derivedHeader byte = 0",
        "\tswitch {",
        "\tcase s.Key.Body.CaLocal != nil:",
        "\t\t_derivedHeader |= 0x00",
        "\tcase s.Key.Body.CaWide != nil:",
        "\t\t_derivedHeader |= 0x20",
        "\tcase s.Key.Body.Default != nil:",
        "\t\t_derivedHeader |= byte((s.Key.Body.Default.Tag & 0x3) << 5)",
        "\t}",
    ]
    .join("\n");
    assert_holds(
        &code,
        &switch,
        "the Go parent does not derive K from the catch-all's tag",
    );
}

/// The Kotlin parent's `when` names the catch-all, which makes it exhaustive
/// over the sealed variant, and reads the tag through the `when`'s own binding
/// of the body; its entries are parted by `;`.
#[test]
fn the_kotlin_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Kotlin);
    assert_holds(
        &code,
        "        val _derivedHeader: UByte = ((when (val _held = this.key.body) { \
         is CaChildVariant.CaLocal -> 0x00; \
         is CaChildVariant.CaWide -> 0x20; \
         is CaChildVariant.Default -> ((_held.tag.toInt() and 0x3) shl 5) })).toUByte()\n",
        "the Kotlin parent does not derive K from the catch-all's tag",
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
/// named by its package, and the Python modules are in the package
/// `PYTHON_PACKAGE`, since they import each other relatively.
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
    if matches!(language, Language::Python) {
        let package = dir.join(PYTHON_PACKAGE);
        std::fs::create_dir_all(&package).expect("make the Python package");
        std::fs::write(package.join("__init__.py"), "").expect("write __init__.py");
    }
    for (name, code) in files {
        let path = Path::new(&name);
        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }
        let target = if matches!(language, Language::Go) {
            let package = dir.join(path.file_stem().expect("a Go file has a stem"));
            std::fs::create_dir_all(&package).expect("make a package directory");
            package
        } else if matches!(language, Language::Python) {
            dir.join(PYTHON_PACKAGE)
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

/// The package the Python run puts the generated modules in; the driver
/// imports from it.
const PYTHON_PACKAGE: &str = "ca";

const PYTHON_DRIVER: &str = r#"# Decode each frame, store stale bits in K, and hold the encode to the
# frame: K comes from the body, not from the stored header.
import sys

from sce_forge_runtime.codec import SceCursor

from ca.ca_parent import CaParent

failed = False


def fail(what):
    global failed
    print(what)
    failed = True


def rewrite(frame, stale):
    """Decode `frame`, store `stale` as its header, and encode: the bytes
    must be the frame's."""
    value = CaParent.decode(SceCursor(frame))
    if value is None:
        fail("the frame must decode")
        return None
    value.header = stale
    if value.encode_to_bytes() != frame:
        fail("the encode must write K from the body")
    return value


held = rewrite(bytes([0x5D, 0x07]), 0x3D)
if (
    held is None
    or held.key.body.kind != "Default"
    or held.key.body.default_tag != 2
    or held.key.body.default_body.payload != 0x07
):
    fail("tag 2 must reach the catch-all, which keeps it")
named = rewrite(bytes([0x3D, 0x12, 0x34]), 0x5D)
if named is None or named.key.body.kind != "CaWide" or named.key.body.ca_wide.payload != 0x1234:
    fail("tag 1 must reach the two-byte arm")
sys.exit(1 if failed else 0)
"#;

const GO_DRIVER: &str = r#"// Decode each frame, store stale bits in K, and hold the encode to the
// frame: K comes from the body, not from the stored header.
package main

import (
	"bytes"
	"fmt"
	"os"

	"github.com/newmassrael/sce-forge-runtime/codec"

	"example.com/sce-catch-all/ca_parent"
)

var failed bool

func fail(what string) {
	fmt.Println(what)
	failed = true
}

// rewrite decodes frame, stores stale as its header, and encodes: the bytes
// must be the frame's.
func rewrite(frame []byte, stale byte) *ca_parent.CaParent {
	cursor := codec.NewSceCursor(frame)
	value, err := ca_parent.DecodeCaParent(&cursor)
	if err != nil {
		fail("the frame must decode")
		return nil
	}
	value.Header = stale
	var out []byte
	if err := value.Encode(codec.NewBytesSink(&out)); err != nil {
		fail("the decoded value must encode")
	} else if !bytes.Equal(out, frame) {
		fail("the encode must write K from the body")
	}
	return value
}

func main() {
	held := rewrite([]byte{0x5d, 0x07}, 0x3d)
	if held == nil || held.Key.Body.Default == nil || held.Key.Body.Default.Tag != 2 || held.Key.Body.Default.Body.Payload != 0x07 {
		fail("tag 2 must reach the catch-all, which keeps it")
	}
	named := rewrite([]byte{0x3d, 0x12, 0x34}, 0x5d)
	if named == nil || named.Key.Body.CaWide == nil || named.Key.Body.CaWide.Payload != 0x1234 {
		fail("tag 1 must reach the two-byte arm")
	}
	if failed {
		os.Exit(1)
	}
}
"#;

const KOTLIN_DRIVER: &str = r#"// Decode each frame, store stale bits in K, and hold the encode to the
// frame: K comes from the body, not from the stored header.
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.generated.ca_child.CaChildVariant
import com.sce.generated.ca_parent.CaParent
import kotlin.system.exitProcess

var failed = false

fun fail(what: String) {
    println(what)
    failed = true
}

// Decode `frame`, store `stale` as its header, and encode: the bytes must be
// the frame's.
fun rewrite(frame: ByteArray, stale: Int): CaParent? {
    val value = CaParent.decode(SceCursor(frame))
    if (value == null) {
        fail("the frame must decode")
        return null
    }
    value.header = stale.toUByte()
    val out = mutableListOf<Byte>()
    if (value.encode(MutableListSink(out)) != null) {
        fail("the decoded value must encode")
    } else if (!out.toByteArray().contentEquals(frame)) {
        fail("the encode must write K from the body")
    }
    return value
}

fun main() {
    val held = rewrite(byteArrayOf(0x5d, 0x07), 0x3d)?.key?.body as? CaChildVariant.Default
    if (held == null || held.tag.toInt() != 2 || held.body.payload.toInt() != 0x07) {
        fail("tag 2 must reach the catch-all, which keeps it")
    }
    val wide = rewrite(byteArrayOf(0x3d, 0x12, 0x34), 0x5d)?.key?.body as? CaChildVariant.CaWide
    if (wide == null || wide.body.payload.toInt() != 0x1234) {
        fail("tag 1 must reach the two-byte arm")
    }
    if (failed) {
        exitProcess(1)
    }
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

/// The Python parent writes back each frame it decoded, whatever its stored
/// K.
#[test]
fn the_python_parent_writes_back_each_frame() {
    let Some(python) =
        sce_build::toolchain::require_or_skip("python3", "run the Python parent of a catch-all")
    else {
        return;
    };
    let dir = project(Language::Python, "py", "py");
    std::fs::write(dir.join("driver.py"), PYTHON_DRIVER).expect("write the driver");
    let mut driver = Command::new(python);
    driver
        .args(["-W", "error", "driver.py"])
        .env("PYTHONPATH", backend("backends/python/forge-runtime"));
    run(driver, &dir, "the Python driver");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The Go parent writes back each frame it decoded, whatever its stored K.
#[test]
fn the_go_parent_writes_back_each_frame() {
    let Some(go) =
        sce_build::toolchain::require_or_skip("go", "build and run the Go parent of a catch-all")
    else {
        return;
    };
    let dir = project(Language::Go, "go", "go");
    std::fs::write(
        dir.join("go.mod"),
        format!(
            "module {GO_MODULE}\n\n\
             go 1.22\n\n\
             require github.com/newmassrael/sce-forge-runtime v0.0.0\n\n\
             replace github.com/newmassrael/sce-forge-runtime => {}\n",
            backend("backends/go/forge-runtime").display()
        ),
    )
    .expect("write go.mod");
    std::fs::write(dir.join("main.go"), GO_DRIVER).expect("write the driver");
    let mut vet = Command::new(&go);
    vet.args(["vet", "./..."]);
    run(vet, &dir, "go vet");
    let mut driver = Command::new(go);
    driver.args(["run", "."]);
    run(driver, &dir, "the Go driver");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The Kotlin forge runtime jar, which Gradle builds into the runtime's
/// `build/libs`, if it has been built.
fn kotlin_runtime_jar() -> Option<PathBuf> {
    std::fs::read_dir(backend("backends/kotlin/forge-runtime").join("build/libs"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with("sce-forge-runtime-kotlin-jvm") && n.ends_with(".jar")
            })
        })
}

/// The Kotlin parent writes back each frame it decoded, whatever its stored
/// K. The parent's `when` over the embedded variant names the catch-all,
/// without which it does not compile.
#[test]
fn the_kotlin_parent_writes_back_each_frame() {
    let purpose = "compile and run the Kotlin parent of a catch-all";
    let Some(kotlinc) = sce_build::toolchain::require_or_skip("kotlinc", purpose) else {
        return;
    };
    let Some(java) = sce_build::toolchain::require_or_skip("java", purpose) else {
        return;
    };
    let Some(jar) = kotlin_runtime_jar() else {
        sce_build::toolchain::skipped(
            "the Kotlin forge runtime jar is not built \
             (run `./gradlew :sce-forge-runtime-kotlin:jvmJar` from the workspace root)",
        );
        return;
    };
    let dir = project(Language::Kotlin, "kt", "kt");
    std::fs::write(dir.join("Driver.kt"), KOTLIN_DRIVER).expect("write the driver");
    let sources: Vec<String> = std::fs::read_dir(&dir)
        .expect("read the project directory")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".kt"))
        .collect();
    let mut build = Command::new(kotlinc);
    build
        .args(["-Werror", "-include-runtime", "-d", "driver.jar", "-cp"])
        .arg(&jar)
        .args(sources);
    run(build, &dir, "kotlinc");
    let mut driver = Command::new(java);
    driver
        .arg("-cp")
        .arg(format!("driver.jar:{}", jar.display()))
        .arg("DriverKt");
    run(driver, &dir, "the Kotlin driver");
    let _ = std::fs::remove_dir_all(&dir);
}
