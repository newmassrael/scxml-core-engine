// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `addr2sce --pc` / `--hardfault` — program counter to SCXML coordinates.
//
// SCE Protocol-Synthesis RFC §synth-5-O (lines 3253-3278) fixes the
// resolution contract as PC -> function symbol -> sourcemap -> SCXML
// file:line + state path. The first hop is the ELF symbol table; the
// second is the sidecar `--symbol` already reads. Both modes previously
// exited 2 with "not implemented".
//
// The fixtures are ELF images synthesised in-process rather than
// checked-in blobs: the symbol layout a case depends on (address,
// size, the gap between two functions) then sits next to the assertion
// that reads it, and the suite carries no binary whose provenance
// nobody can re-derive.
//
// Cases:
//
//   * a PC inside a function resolves to that function's SCXML source,
//   * a PC in the padding between two functions is a miss, not a
//     silent attribution to the preceding symbol,
//   * an ARM Thumb PC (bit 0 set, as an exception frame's stacked LR
//     carries it) resolves to the same function as the even address —
//     the MCU consumer this mode exists for is Cortex-M,
//   * `--hardfault` resolves every line of a stack dump and fails when
//     any frame is unresolvable.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use object::write::{Object, StandardSection, Symbol, SymbolSection};
use object::{Architecture, BinaryFormat, Endianness, SymbolFlags, SymbolKind, SymbolScope};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Write an ELF carrying `symbols` as sized `STT_FUNC` entries in
/// `.text`, at the offsets the caller names.
fn write_elf(path: &Path, arch: Architecture, symbols: &[(&str, u64, u64)]) {
    let mut obj = Object::new(BinaryFormat::Elf, arch, Endianness::Little);
    let text = obj.section_id(StandardSection::Text);
    // One byte per addressable slot so the section covers every symbol
    // range the test names.
    let end = symbols
        .iter()
        .map(|(_, addr, size)| addr + size)
        .max()
        .unwrap_or(0);
    obj.append_section_data(text, &vec![0u8; end as usize], 1);

    // An ARM toolchain sets bit 0 of a Thumb function's `st_value`;
    // the function still begins at the even address. Reproducing that
    // is what makes the reader's normalisation load-bearing — with
    // even symbol values the Thumb path is never exercised.
    let thumb = matches!(arch, Architecture::Arm);
    for (name, addr, size) in symbols {
        obj.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: if thumb { addr | 1 } else { *addr },
            size: *size,
            kind: SymbolKind::Text,
            scope: SymbolScope::Dynamic,
            weak: false,
            section: SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
    }

    let bytes = obj.write().expect("serialise ELF");
    std::fs::write(path, bytes).expect("write ELF fixture");
}

/// Minimal sourcemap sidecar naming `symbols`. Built from the
/// producer's own types rather than hand-written JSON so the fixture
/// cannot drift from the shape `addr2sce` deserialises.
fn write_sourcemap(dir: &Path, symbols: &[(&str, &str, u32)]) {
    let mut map = sce_build::forge::sourcemap::Sourcemap {
        version: 1,
        source_hash: "0".repeat(64),
        template_hash: "0".repeat(64),
        symbols: Default::default(),
    };
    for (sym, file, line) in symbols {
        map.symbols.insert(
            (*sym).to_string(),
            sce_build::forge::sourcemap::SourceSymbol {
                scxml_file: (*file).to_string(),
                scxml_state_path: format!("root.{sym}"),
                scxml_xpath: format!("/scxml/state[@id='{sym}']"),
                line_range: [*line, *line + 2],
                kind: "state".to_string(),
                event: None,
                wcet_us: None,
            },
        );
    }
    std::fs::write(
        dir.join("sce_sourcemap.json"),
        serde_json::to_string(&map).expect("serialise sourcemap"),
    )
    .expect("write sourcemap");
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn addr2sce(args: &[&str], stdin: Option<&str>) -> Run {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("addr2sce")
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn sce-codegen");
    if let Some(text) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(text.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait for sce-codegen");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

/// The `symbol` field of the first NDJSON record on stdout.
fn resolved_symbol(run: &Run) -> Option<String> {
    let line = run.stdout.lines().find(|l| l.starts_with('{'))?;
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    value
        .get("symbol")
        .and_then(|s| s.as_str())
        .map(str::to_string)
}

/// Two functions with a gap between them, plus a sourcemap naming both.
fn staged_fixture(label: &str, arch: Architecture) -> ScratchDir {
    let scratch = ScratchDir::new(label);
    write_elf(
        &scratch.path().join("probe.elf"),
        arch,
        &[
            ("probe__armed__on_entry", 0x1000, 0x40),
            ("probe__idle__on_entry", 0x1080, 0x20),
        ],
    );
    write_sourcemap(
        scratch.path(),
        &[
            ("probe__armed__on_entry", "probe.scxml", 12),
            ("probe__idle__on_entry", "probe.scxml", 30),
        ],
    );
    scratch
}

#[test]
fn pc_inside_a_function_resolves_to_its_scxml_source() {
    let scratch = staged_fixture("pc-inside", Architecture::X86_64);
    let elf = scratch.path().join("probe.elf");
    let run = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1024",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(
        run.code, 0,
        "a PC inside a mapped function must resolve; stderr: {}",
        run.stderr
    );
    assert_eq!(
        resolved_symbol(&run).as_deref(),
        Some("probe__armed__on_entry"),
        "stdout: {}",
        run.stdout
    );
    assert!(
        run.stdout.contains("probe.scxml"),
        "the record must carry the SCXML coordinates the sourcemap holds: {}",
        run.stdout
    );
}

#[test]
fn pc_at_a_function_entry_resolves() {
    // The boundary the containment test has to include: `addr` itself
    // is inside `[addr, addr + size)`.
    let scratch = staged_fixture("pc-entry", Architecture::X86_64);
    let elf = scratch.path().join("probe.elf");
    let run = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1080",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    assert_eq!(
        resolved_symbol(&run).as_deref(),
        Some("probe__idle__on_entry")
    );
}

#[test]
fn pc_in_inter_function_padding_is_a_miss() {
    // `0x1040` is one past the end of the first function and before the
    // second starts. Attributing it to the nearest preceding symbol
    // would hand a crash triage the wrong state — a miss the caller can
    // see is the honest answer.
    let scratch = staged_fixture("pc-gap", Architecture::X86_64);
    let elf = scratch.path().join("probe.elf");
    let run = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1050",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    assert_ne!(
        run.code, 0,
        "a PC outside every function range must not resolve; stdout: {}",
        run.stdout
    );
}

#[test]
fn arm_thumb_pc_resolves_like_its_even_address() {
    // On Cortex-M the stacked LR of an exception frame carries the
    // Thumb bit, so a PC harvested from a hardfault dump is odd. The
    // MCU consumer this mode exists for is exactly that case.
    let scratch = staged_fixture("pc-thumb", Architecture::Arm);
    let elf = scratch.path().join("probe.elf");
    let even = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1024",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    let odd = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1025",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(even.code, 0, "stderr: {}", even.stderr);
    assert_eq!(
        odd.code, 0,
        "an ARM Thumb PC must resolve; stderr: {}",
        odd.stderr
    );
    assert_eq!(
        resolved_symbol(&odd),
        resolved_symbol(&even),
        "the Thumb bit must not change which function a PC lands in"
    );

    // The address the normalisation actually turns on: a Thumb symbol's
    // `st_value` is the entry address with bit 0 set, so the function's
    // own first byte sits *below* the recorded value. Without clearing
    // the bit on the symbol side, a fault at the first instruction
    // falls through to whatever function precedes it — or, at the
    // lowest function, to nothing at all.
    let entry = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--pc",
            "0x1000",
            "--elf",
            elf.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(
        entry.code, 0,
        "a fault at a Thumb function's first instruction must resolve; stderr: {}",
        entry.stderr
    );
    assert_eq!(
        resolved_symbol(&entry).as_deref(),
        Some("probe__armed__on_entry")
    );
}

#[test]
fn hardfault_resolves_every_frame_of_a_stack_dump() {
    let scratch = staged_fixture("hardfault", Architecture::Arm);
    let elf = scratch.path().join("probe.elf");
    let run = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--hardfault",
            "--elf",
            elf.to_str().unwrap(),
        ],
        Some("0x1024\n0x1085\n"),
    );
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let records: Vec<&str> = run.stdout.lines().filter(|l| l.starts_with('{')).collect();
    assert_eq!(
        records.len(),
        2,
        "one record per frame, in stack order: {}",
        run.stdout
    );
    assert!(records[0].contains("probe__armed__on_entry"));
    assert!(records[1].contains("probe__idle__on_entry"));
}

#[test]
fn hardfault_fails_when_a_frame_is_unresolvable() {
    // A stack dump whose frames cannot all be attributed must not exit
    // 0 — a triage pipeline that greens on a partial answer reports a
    // narrative with a hole in it.
    let scratch = staged_fixture("hardfault-miss", Architecture::Arm);
    let elf = scratch.path().join("probe.elf");
    let run = addr2sce(
        &[
            scratch.path().to_str().unwrap(),
            "--hardfault",
            "--elf",
            elf.to_str().unwrap(),
        ],
        Some("0x1024\n0x9999\n"),
    );
    assert_ne!(run.code, 0, "stdout: {}", run.stdout);
    assert!(
        run.stdout.contains("probe__armed__on_entry"),
        "frames that did resolve stay on stdout: {}",
        run.stdout
    );
}

#[test]
fn pc_without_elf_is_rejected() {
    // `--pc` needs the ELF to map an address to a symbol; the flag pair
    // is not optional and the failure must name that, not crash.
    let scratch = staged_fixture("pc-no-elf", Architecture::X86_64);
    let run = addr2sce(&[scratch.path().to_str().unwrap(), "--pc", "0x1024"], None);
    assert_ne!(run.code, 0);
    assert!(
        run.stderr.contains("--elf"),
        "the diagnostic must name the missing flag: {}",
        run.stderr
    );
}
