// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A parent whose carrier holds, in a flag, the tag of a variant it embeds
//! under a gate writes in that flag the tag of the arm the body holds where the
//! body is there. Where it is not, nothing says the flag, and the carrier keeps
//! the bits the value presents there (docs/adr/0065 §6 R4a), which are the ones
//! a decode of the frame read: the decode reads the gate clear, leaves the body
//! out, and keeps the whole header.
//!
//! Each backend once wrote the flag clear for an absent body, so a frame whose
//! gate is clear and whose flag is set decoded and encoded again into another
//! frame. The render tests hold the fold of the flag to the presence of the
//! body in the backends that print every such parent from its plan (Rust, C++,
//! Python); the run tests compile each backend's codecs where its toolchain is
//! present and hold the encode of every decoded frame to the frame.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The module the Go codecs are generated into, and the run test's
/// `go.mod` declares.
const GO_MODULE: &str = "example.com/sce-gated-key";

/// A parent whose `header` holds `R` (bit 4), the gate of the embedded
/// variant `key`, and `M` (bit 6), the tag of its arm: `0` for the two-byte
/// `gk_wide` (the default arm), `1` for the one-byte `gk_local`. Written
/// with its documents to a fresh directory the caller removes.
fn gated_key_fixture() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sce_gated_key_parent_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("make the fixture directory");
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
            "gk_parent",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gk_parent">
  <sce:import src="gk_key.scxml" kind="codec" as="gk_key">
    <sce:variant-dispatch flag="header.M"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="R" bit="4"/>
      <sce:flag name="M" bit="6"/>
    </sce:flags>
    <sce:embed id="key" type="gk_key" sce:byte="1" sce:present-if="header.R"/>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        (
            "gk_key",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gk_key">
  <sce:import src="gk_local.scxml" kind="codec" as="gk_local"/>
  <sce:import src="gk_wide.scxml" kind="codec" as="gk_wide"/>
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="gk_wide" default="true"/>
      <sce:arm value="0x01" type="gk_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        ("gk_local", body("gk_local", "uint8", 8)),
        ("gk_wide", body("gk_wide", "uint16", 16)),
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
    for stem in ["gk_local", "gk_wide", "gk_key", "gk_parent"] {
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
fn parent_code(language: Language) -> String {
    let dir = gated_key_fixture();
    let files = generate_all(&dir, language);
    let _ = std::fs::remove_dir_all(&dir);
    files
        .into_iter()
        .find(|(name, _)| name.contains("gk_parent") || name.contains("GkParent"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the parent's file for {language:?}"))
}

/// Assert that `code` holds `wanted`, naming `what` when it does not.
fn assert_holds(code: &str, wanted: &str, what: &str) {
    assert!(
        code.contains(wanted),
        "{what}; wanted\n{wanted}\nin:\n{code}"
    );
}

// ── the fold: M from the body where it is there ─────────────────────────

/// The Rust source interface reads the tag as an `Option`, none where the key
/// is absent, and folds it into the header only where there is one.
///
/// Turns red where `rust_source::derive_parent_tag` writes `M` for an absent
/// body, through `None => 0` and an unconditional fold.
#[test]
fn the_rust_parent_folds_m_only_where_the_key_says_it() {
    let code = parent_code(Language::Rust);
    assert_holds(
        &code,
        "let _d_header_m: Option<u8> = src.key().map(|_x| match ",
        "the Rust parent does not read M as a tag there may be none of",
    );
    assert_holds(
        &code,
        "if let Some(_n) = _d_header_m {\n            _c_header = (_c_header & !0x40) | (_n << 6);",
        "the Rust parent folds M where the key is absent",
    );
}

/// The C++ parent folds M into the header it writes only where the key is
/// there.
///
/// Turns red where `cpp_source` hands the parent tag's `Fold` no `present`.
#[test]
fn the_cpp_parent_folds_m_only_where_the_key_says_it() {
    let code = parent_code(Language::Cpp);
    assert_holds(
        &code,
        ".has_value()) {\n            _f_header = (_f_header & ~0x40ull) | (_p_header_m << 6);\n        }\n",
        "the C++ parent folds M where the key is absent",
    );
}

/// The Python parent folds M into the header it writes only where the key is
/// there.
///
/// Turns red where `python_source` pushes the fold of a parent tag whose
/// bodies are all gated without the test that one is there.
#[test]
fn the_python_parent_folds_m_only_where_the_key_says_it() {
    let code = parent_code(Language::Python);
    assert_holds(
        &code,
        "    if src.key is not None:\n        \
         _c_header = (_c_header & ~0x40) | ((_p_header_m & 0x1) << 6)\n",
        "the Python parent folds M where the key is absent",
    );
}

// ── running what was generated ──────────────────────────────────────────
//
// Every driver decodes the same four frames and encodes the value each decoded
// into, which must be the frame:
//   - 0x40: R clear, so no key, and M set, which no body says;
//   - 0x00: R clear and M clear;
//   - 0x50 0x07: R set, M = 1, the one-byte local arm;
//   - 0x10 0x12 0x34: R set, M = 0, the two-byte wide arm.
// An encode that writes M clear for an absent key writes 0x00 for the first.

/// A fresh directory for one run, holding every file of extension `ext` the
/// fixture generates for `language`; a Go file is in a directory of its own,
/// named by its package, and the Python modules are in the package
/// `PYTHON_PACKAGE`, since they import each other relatively.
fn project(language: Language, ext: &str, label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let fixture = gated_key_fixture();
    let files = generate_all(&fixture, language);
    let _ = std::fs::remove_dir_all(&fixture);
    let dir = std::env::temp_dir().join(format!(
        "sce_gated_key_{label}_{}_{}",
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

const C_DRIVER: &str = r#"/* Decode each frame and hold the encode of what it decoded to the frame:
 * M is the arm's where the key is there and the frame's where it is not. */
#include "gk_parent.h"
#include <stdio.h>
#include <string.h>

static int fail(const char *what) {
    printf("%s\n", what);
    return 1;
}

/* Decode `frame`, encode the value it decoded into: the bytes must be the
 * frame's. */
static int round_trip(const uint8_t *frame, size_t len) {
    sce_forge_cursor_t cursor = sce_forge_cursor_init(frame, len);
    gk_parent_t view;
    memset(&view, 0, sizeof view);
    if (gk_parent_decode(&cursor, &view) != SCE_FORGE_CODEC_OK) {
        return fail("the frame must decode");
    }
    uint8_t buf[8];
    sce_forge_writer_t writer = sce_forge_writer_init_buf(buf, sizeof buf);
    if (gk_parent_encode(&view, &writer) != SCE_FORGE_CODEC_OK) {
        return fail("the decoded value must encode");
    }
    if (writer.pos != len || memcmp(buf, frame, len) != 0) {
        return fail("the encode must write back the frame it decoded");
    }
    return 0;
}

int main(void) {
    int failed = 0;
    const uint8_t absent_m[1] = { 0x40u };
    const uint8_t absent[1] = { 0x00u };
    const uint8_t local[2] = { 0x50u, 0x07u };
    const uint8_t wide[3] = { 0x10u, 0x12u, 0x34u };
    failed |= round_trip(absent_m, sizeof absent_m);
    failed |= round_trip(absent, sizeof absent);
    failed |= round_trip(local, sizeof local);
    failed |= round_trip(wide, sizeof wide);
    return failed;
}
"#;

const CPP_DRIVER: &str = r#"// Decode each frame and hold the encode of what it decoded to the frame:
// M is the arm's where the key is there and the frame's where it is not.
#include "gk_parent.h"

#include <cstdint>
#include <cstdio>
#include <vector>

namespace {

using Parent = ::SCE::Generated::GkParent::GkParent;

int fail(const char* what) {
    std::printf("%s\n", what);
    return 1;
}

// Decode `frame` and encode the value it decoded into: the bytes must be the
// frame's.
int round_trip(const std::vector<std::uint8_t>& frame) {
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const ::SCE::Forge::Decoded<Parent> decoded = Parent::decode(cursor);
    if (!decoded.has_value()) {
        return fail("the frame must decode");
    }
    std::vector<std::uint8_t> written;
    ::SCE::Forge::VectorSink sink(written);
    if (decoded->encode(sink).has_value()) {
        return fail("the decoded value must encode");
    }
    if (written != frame) {
        return fail("the encode must write back the frame it decoded");
    }
    return 0;
}

}  // namespace

int main() {
    int failed = 0;
    failed |= round_trip({0x40});
    failed |= round_trip({0x00});
    failed |= round_trip({0x50, 0x07});
    failed |= round_trip({0x10, 0x12, 0x34});
    return failed;
}
"#;

/// The package the Python run puts the generated modules in; the driver
/// imports from it.
const PYTHON_PACKAGE: &str = "gk";

const PYTHON_DRIVER: &str = r#"# Decode each frame and hold the encode of what it decoded to the frame:
# M is the arm's where the key is there and the frame's where it is not.
import sys

from sce_forge_runtime.codec import SceCursor

from gk.gk_parent import GkParent

failed = False


def fail(what):
    global failed
    print(what)
    failed = True


def round_trip(frame):
    """Decode `frame` and encode the value it decoded: the bytes must be the
    frame's."""
    value = GkParent.decode(SceCursor(frame))
    if value is None:
        fail("the frame must decode")
        return None
    if value.encode_to_bytes() != frame:
        fail("the encode must write back the frame it decoded")
    return value


absent = round_trip(bytes([0x40]))
if absent is None or absent.key is not None or absent.header != 0x40:
    fail("R clear must leave the key out and keep M")
round_trip(bytes([0x00]))
round_trip(bytes([0x50, 0x07]))
round_trip(bytes([0x10, 0x12, 0x34]))
sys.exit(1 if failed else 0)
"#;

const GO_DRIVER: &str = r#"// Decode each frame and hold the encode of what it decoded to the frame:
// M is the arm's where the key is there and the frame's where it is not.
package main

import (
	"bytes"
	"fmt"
	"os"

	"github.com/newmassrael/sce-forge-runtime/codec"

	"example.com/sce-gated-key/gk_parent"
)

var failed bool

func fail(what string) {
	fmt.Println(what)
	failed = true
}

// roundTrip decodes frame and encodes the value it decoded: the bytes must be
// the frame's.
func roundTrip(frame []byte) {
	cursor := codec.NewSceCursor(frame)
	value, err := gk_parent.DecodeGkParent(&cursor)
	if err != nil {
		fail("the frame must decode")
		return
	}
	var out []byte
	if err := value.Encode(codec.NewBytesSink(&out)); err != nil {
		fail("the decoded value must encode")
	} else if !bytes.Equal(out, frame) {
		fail("the encode must write back the frame it decoded")
	}
}

func main() {
	roundTrip([]byte{0x40})
	roundTrip([]byte{0x00})
	roundTrip([]byte{0x50, 0x07})
	roundTrip([]byte{0x10, 0x12, 0x34})
	if failed {
		os.Exit(1)
	}
}
"#;

const KOTLIN_DRIVER: &str = r#"// Decode each frame and hold the encode of what it decoded to the frame:
// M is the arm's where the key is there and the frame's where it is not.
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.generated.gk_parent.GkParent
import kotlin.system.exitProcess

var failed = false

fun fail(what: String) {
    println(what)
    failed = true
}

// Decode `frame` and encode the value it decoded: the bytes must be the
// frame's.
fun roundTrip(frame: ByteArray) {
    val value = GkParent.decode(SceCursor(frame)).valueOrNull()
    if (value == null) {
        fail("the frame must decode")
        return
    }
    val out = mutableListOf<Byte>()
    if (value.encode(MutableListSink(out)) != null) {
        fail("the decoded value must encode")
    } else if (!out.toByteArray().contentEquals(frame)) {
        fail("the encode must write back the frame it decoded")
    }
}

fun main() {
    roundTrip(byteArrayOf(0x40))
    roundTrip(byteArrayOf(0x00))
    roundTrip(byteArrayOf(0x50, 0x07))
    roundTrip(byteArrayOf(0x10, 0x12, 0x34))
    if (failed) {
        exitProcess(1)
    }
}
"#;

/// The C parent writes back each frame it decoded, M where no key says it
/// among them.
///
/// Turns red where `c11_source` hands the parent tag's `Fold` no `present`
/// (`parent_tag_said`), or, for a parent kept on the template's encode,
/// where `parent_tag_switch_block_c11` adds nothing for an absent key.
#[test]
fn the_c_parent_writes_back_each_frame() {
    let Some(gcc) =
        sce_build::toolchain::require_or_skip("gcc", "compile and run the C parent of a gated key")
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

/// The C++ parent writes back each frame it decoded.
#[test]
fn the_cpp_parent_writes_back_each_frame() {
    let Some(gxx) = sce_build::toolchain::require_or_skip(
        "g++",
        "compile and run the C++ parent of a gated key",
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

/// The Python parent writes back each frame it decoded, and keeps the header
/// of a frame whose key is absent.
#[test]
fn the_python_parent_writes_back_each_frame() {
    let Some(python) =
        sce_build::toolchain::require_or_skip("python3", "run the Python parent of a gated key")
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

/// The Go parent writes back each frame it decoded.
///
/// Turns red where `go_source::encode_source` folds the parent tag without
/// `fold_where_said`, or, for a parent kept on the template's encode, where
/// `parent_tag_switch_block_go` adds nothing for an absent key.
#[test]
fn the_go_parent_writes_back_each_frame() {
    let Some(go) =
        sce_build::toolchain::require_or_skip("go", "build and run the Go parent of a gated key")
    else {
        return;
    };
    let dir = project(Language::Go, "go", "go");
    std::fs::write(
        dir.join("go.mod"),
        format!(
            "module {GO_MODULE}\n\n\
             go 1.23.0\n\n\
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

/// The Kotlin parent writes back each frame it decoded.
///
/// Turns red where `kotlin_source::encode_source` folds the parent tag
/// without the test that the key is there, or, for a parent kept on the
/// template's encode, where `parent_tag_match_term_kotlin` gives `0` for an
/// absent key.
#[test]
fn the_kotlin_parent_writes_back_each_frame() {
    let purpose = "compile and run the Kotlin parent of a gated key";
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
    let mut build = common::kotlin::kotlinc(kotlinc);
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
