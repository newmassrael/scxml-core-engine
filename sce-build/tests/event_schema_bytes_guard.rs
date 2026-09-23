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

use sce_build::toolchain;
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

/// Scratch dir keyed by CALLING TEST as well as language.
///
/// cargo runs these tests in parallel threads of one process. Keying only by
/// language made all three share `…/<lang>/`, so the compile tests dropped an
/// object file and a linked executable into the directory the byte-identity
/// test walks — and that test reads every entry as UTF-8. The result was an
/// order-dependent "stream did not contain valid UTF-8" that passes or fails on
/// thread timing. Per-test dirs remove the shared state instead of teaching the
/// reader to tolerate binaries, which would have hidden real emission bugs.
fn out_dir(scope: &str, lang: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("event_schema_bytes_guard")
        .join(scope)
        .join(lang);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Generate the bytes fixture for `lang` into a fresh dir and return both
/// the output dir and every emitted file concatenated.
fn generate(scope: &str, lang: &str) -> (PathBuf, String) {
    let dir = out_dir(scope, lang);
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
        let (_dir, code) = generate("byte_identity", lang);
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
    let (dir, _code) = generate("cpp_run", "cpp");
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped("cpp_bytes_guard_compiles_and_runs: g++ not on PATH");
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
    let (dir, code) = generate("c11_freestanding", "c11");
    assert!(
        code.contains("uint8_t raw[8];") && code.contains("size_t raw_len;"),
        "expected the no-alloc bounded-buffer payload field (CAP from sce:max-size=8)",
    );
    let Some(cc) = toolchain::locate_any(&["clang", "gcc"]) else {
        toolchain::skipped("c11_bytes_guard_compiles_freestanding: clang/gcc not on PATH");
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

#[test]
fn c11_typed_payload_preserves_values_and_rejects_overflow() {
    let Some(cc) = toolchain::locate("gcc") else {
        toolchain::skipped("c11_typed_payload_preserves_values_and_rejects_overflow: gcc missing");
        return;
    };
    let dir = out_dir("bounded_payload", "c11");
    std::fs::write(dir.join("schema.scxml"), r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" name="sample_schema" sce:kind="event-schema" sce:event-name="sample.received">
<datamodel>
<data id="raw" sce:type="bytes" sce:direction="in" sce:max-size="512"/>
<data id="label" sce:type="string" sce:direction="in"/>
<data id="ratio" sce:type="float64" sce:direction="in"/>
<data id="count" sce:type="uint32" sce:direction="in"/>
</datamodel></scxml>"#).unwrap();
    let input = dir.join("bounded_payload.scxml");
    std::fs::write(&input, r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" name="bounded_payload" version="1.0" initial="waiting" datamodel="ecmascript">
<sce:import src="schema.scxml" kind="event-schema" as="Sample"/>
<state id="waiting"><transition event="sample.received" cond="_event.data.label === 'ready'" target="done"/></state>
<state id="done"/></scxml>"#).unwrap();
    assert!(Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&input)
        .arg("-o")
        .arg(&dir)
        .args(["-l", "c11"])
        .status()
        .unwrap()
        .success());
    let driver = dir.join("driver.c");
    std::fs::write(&driver, r#"#include "bounded_payload_sm.h"
#include "sce/event_payload.h"
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>
int main(void) {
    bounded_payload_t sm;
    char label[SCE_MAX_DATA_LEN + 8];
    bounded_payload_sample_received_payload_t p = {0};
    memcpy(p.raw, "ack", 3); p.raw_len = 3; p.label = label; p.ratio = 1.5; p.count = 7;
    for (size_t n = 0; n < sizeof(label); ++n) {
        memset(label, 'x', n); label[n] = 0;
        bounded_payload_init(&sm);
        int needed = snprintf(NULL, 0, "{\"raw\":\"ack\",\"label\":\"%s\",\"ratio\":1.5,\"count\":7}", label);
        bool accepted = bounded_payload_raise_sample_received_typed(&sm, &p);
        assert(accepted == (needed < SCE_MAX_DATA_LEN));
        assert(sm.external_queue.count == (accepted ? 1 : 0));
        if (accepted) {
            sce_payload_fields_t fields; char decoded[SCE_MAX_DATA_LEN];
            assert(sce_payload_decode(sm.external_queue.buf[sm.external_queue.head].data, &fields) == NULL);
            assert(sce_payload_read_text(&fields, "label", decoded, sizeof(decoded)) == NULL);
            assert(strlen(decoded) == n);
        }
    }
    strcpy(label, "ready");
    bounded_payload_init(&sm);
    const unsigned char bytes[] = {0, 0x80, 0xff, '"', '\\'};
    memcpy(p.raw, bytes, sizeof(bytes)); p.raw_len = sizeof(bytes);
    assert(bounded_payload_raise_sample_received_typed(&sm, &p));
    sce_payload_fields_t fields; unsigned char decoded[512]; size_t len = 0;
    assert(sce_payload_decode(sm.external_queue.buf[sm.external_queue.head].data, &fields) == NULL);
    assert(sce_payload_read_bytes(&fields, "raw", decoded, sizeof(decoded), &len) == NULL);
    assert(len == sizeof(bytes) && memcmp(decoded, bytes, len) == 0);
    strcpy(label, "other"); memset(p.raw, 0, sizeof(p.raw));
    bounded_payload_run(&sm);
    assert(bounded_payload_in_state(&sm, BOUNDED_PAYLOAD_STATE_DONE));
    bounded_payload_init(&sm);
    p.raw_len = sizeof(p.raw) + 1;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    p.raw_len = SIZE_MAX;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    p.raw_len = 42;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    p.raw_len = 43;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    p.raw_len = 0; p.ratio = NAN;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    p.ratio = INFINITY;
    assert(!bounded_payload_raise_sample_received_typed(&sm, &p));
    assert(sm.external_queue.count == 0);
    assert(bounded_payload_raise_sample_received_typed(&sm, NULL));
    assert(sce_payload_decode(sm.external_queue.buf[sm.external_queue.head].data, &fields) == NULL);
    assert(sce_payload_read_bytes(&fields, "raw", decoded, sizeof(decoded), &len) == NULL && len == 0);
    assert(sce_payload_read_text(&fields, "label", (char *)decoded, sizeof(decoded)) == NULL && decoded[0] == 0);
    uint64_t count = 1; double ratio = 1;
    assert(sce_payload_read_u64(&fields, "count", &count) == NULL && count == 0);
    assert(sce_payload_read_f64(&fields, "ratio", &ratio) == NULL && ratio == 0);
    return 0;
}
"#).unwrap();
    let bin = dir.join("driver");
    let output = Command::new(cc)
        .args([
            "-std=c11",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
        ])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(&dir)
        .arg(dir.join("bounded_payload_sm.c"))
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Command::new(bin).status().unwrap().success());
}
