// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC rfc-eventschema-bytes-guard.md §bytesguard-3 / §bytesguard-6 — gate for a
// `bytes`-field EventSchema transition guard.
//
// `fixtures/event_schema/statechart_bytes.scxml` imports a schema with a
// single `bytes` field `raw` and guards a transition on
// `_event.data.raw === 'ack'`. With the gate flipped (commit 6) this
// lowers natively on all six backends — each to its own byte-equality
// primitive over the SAME decoded constant (0x61 0x63 0x6b = "ack"):
//
//   * Rust   `ev.raw == b"ack"`
//   * C++    `... == std::vector<uint8_t>{0x61, 0x63, 0x6b}`
//   * Go     `string(...raw) == "ack"`
//   * Kotlin `...raw.contentEquals("ack".toByteArray())`
//   * Python `...raw == b"ack"`           (NOT `== "ack"` — a `bytes ==
//            str` would silently evaluate `False` always; the `b"…"`
//            form is the silent-`False` guard, asserted below)
//   * C11    `raw_len == 3 && memcmp(..raw, "ack", 3) == 0`
//
// The form assertions (no toolchain, always run) pin those primitives so
// a regression to a non-byte-identical or silently-wrong comparison
// fails loudly. The C++ test additionally COMPILES + RUNS the generated
// SM to prove the typed inject -> native guard -> transition actually
// fires on a match and does NOT on a non-match (the runtime check the
// byte-golden / form layers cannot give). The C11 test compiles the
// generated TU under the strict freestanding MCU profile so the
// bounded-buffer + memcmp lowering builds with no hosted-libc guarantee.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/event_schema/statechart_bytes.scxml")
}

fn out_dir(lang: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("event_schema_bytes_guard")
        .join(lang);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn resolve_tool(name: &str) -> Option<PathBuf> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Generate the bytes fixture for `lang` into a fresh dir and return both
/// the output dir and every emitted file concatenated.
fn generate(lang: &str) -> (PathBuf, String) {
    let dir = out_dir(lang);
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let _ = std::fs::remove_file(entry.path());
    }
    let status = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(fixture())
        .arg("-o")
        .arg(&dir)
        .arg("-l")
        .arg(lang)
        .status()
        .expect("run sce-codegen");
    assert!(status.success(), "sce-codegen generate failed for {lang}");

    let mut combined = String::new();
    for entry in std::fs::read_dir(&dir).expect("read scratch dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            combined.push_str(&std::fs::read_to_string(&path).expect("read generated file"));
            combined.push('\n');
        }
    }
    (dir, combined)
}

/// `(language, byte-equality form the bytes guard must lower to)`. Each
/// marker carries the decoded constant for "ack" (0x61 0x63 0x6b) in that
/// backend's native byte-equality primitive. Presence proves the guard
/// was lowered (not routed to a script engine) AND that the comparison is
/// byte-correct — in particular the Python marker is the `bytes`-literal
/// `b"ack"`, never the `str` `"ack"` whose `bytes == str` is silently
/// `False`.
const EXPECTED: &[(&str, &str)] = &[
    ("rust", "ev.raw == b\"ack\""),
    (
        "cpp",
        "pendingSignalReceivedPayload_.raw == std::vector<uint8_t>{0x61, 0x63, 0x6b}",
    ),
    (
        "c11",
        "raw_len == 3 && memcmp(sm->pending_payload.as.signal_received.raw, \"ack\", 3) == 0",
    ),
    (
        "go",
        "string(p.pendingSignalReceivedPayload.raw) == \"ack\"",
    ),
    (
        "kotlin",
        "pendingSignalReceivedPayload!!.raw.contentEquals(\"ack\".toByteArray())",
    ),
    ("python", "_pending_signal_received_payload.raw == b\"ack\""),
];

#[test]
fn every_backend_lowers_bytes_guard_to_byte_identical_equality() {
    for (lang, marker) in EXPECTED {
        let (_dir, code) = generate(lang);
        // Marker presence proves the guard lowered to the backend's
        // byte-equality primitive (not a script-engine fallback, which
        // would emit none of these forms) AND that the compared constant
        // is the byte-identical decoded "ack" (0x61 0x63 0x6b). The
        // Python marker is the `b"ack"` bytes literal specifically — the
        // `str` form `== "ack"` is `bytes == str`, silently always False.
        assert!(
            code.contains(marker),
            "{lang}: missing native byte-equality guard `{marker}` — the \
             bytes guard regressed to a non-native / non-byte-identical form",
        );
    }
}

// Real compile + run: prove the typed inject -> native bytes guard ->
// transition fires on a matching payload and not on a non-matching one.
// Header-only (the no-script-engine value path links no runtime), so the
// gate needs only g++. Skipped (not failed) when g++ is absent.
#[test]
fn cpp_bytes_guard_compiles_and_runs() {
    let (dir, _code) = generate("cpp");
    let Some(gpp) = resolve_tool("g++") else {
        eprintln!("SKIP cpp_bytes_guard_compiles_and_runs: g++ not on PATH");
        return;
    };
    let driver = dir.join("driver.cpp");
    std::fs::write(
        &driver,
        r#"#include "statechart_bytes_sm.h"
#include <vector>
#include <cstdint>
#include <cstdio>
using SM = ::SCE::Generated::statechart_bytes::statechart_bytes;
int main() {
    SM sm; sm.initialize();
    if (sm.getCurrentState() != SM::State::Waiting) { std::puts("FAIL initial"); return 1; }
    sm.raiseSignalReceived(std::vector<uint8_t>{0x61, 0x63, 0x6b}); // "ack" -> guard matches
    sm.step();
    if (sm.getCurrentState() != SM::State::Done) { std::puts("FAIL match"); return 2; }
    SM sm2; sm2.initialize();
    sm2.raiseSignalReceived(std::vector<uint8_t>{0x6e, 0x6f}); // "no" -> guard rejects
    sm2.step();
    if (sm2.getCurrentState() != SM::State::Waiting) { std::puts("FAIL non-match"); return 3; }
    std::puts("OK"); return 0;
}
"#,
    )
    .expect("write driver");
    let bin = dir.join("driver");
    let inc = repo_root().join("sce/include");
    let compile = Command::new(&gpp)
        .arg("-std=c++17")
        .arg("-I")
        .arg(&inc)
        .arg("-I")
        .arg(&dir)
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run g++");
    assert!(
        compile.status.success(),
        "header-only compile of the bytes-guard C++ SM failed\nstderr: {}",
        String::from_utf8_lossy(&compile.stderr),
    );
    let run = Command::new(&bin).output().expect("run driver");
    assert!(
        run.status.success(),
        "bytes-guard C++ SM run failed (exit {:?})\nstdout: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
    );
}

// C11 parity: the bounded-buffer + memcmp lowering must build under the
// strict freestanding MCU profile (no hosted-libc guarantee — memcmp is
// one of the freestanding-required builtins). Skipped when no C compiler
// is present.
#[test]
fn c11_bytes_guard_compiles_freestanding() {
    let (dir, code) = generate("c11");
    assert!(
        code.contains("uint8_t raw[8];") && code.contains("size_t raw_len;"),
        "expected the no-alloc bounded-buffer payload field (CAP from sce:max-size=8)",
    );
    let Some(cc) = resolve_tool("clang").or_else(|| resolve_tool("gcc")) else {
        eprintln!("SKIP c11_bytes_guard_compiles_freestanding: clang/gcc not on PATH");
        return;
    };
    let c_files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect();
    assert_eq!(c_files.len(), 1, "expected exactly one generated C11 TU");
    let runtime_inc = repo_root().join("backends/c/runtime/include");
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c11",
        "-ffreestanding",
        "-c",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror",
    ]);
    cmd.arg("-I").arg(&runtime_inc);
    cmd.arg("-I").arg(&dir);
    cmd.arg("-o").arg(dir.join("sm.o"));
    cmd.arg(&c_files[0]);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");
    assert!(
        output.status.success(),
        "freestanding -std=c11 compile of the bytes-guard TU failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
