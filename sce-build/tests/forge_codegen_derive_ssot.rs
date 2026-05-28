// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! End-to-end regression gate for `forge::rust_derive_policy`.
//!
//! The SSOT module's unit tests prove the policy itself is correct.
//! This file proves the policy is wired through: every Rust forge
//! template consumes the right context key, and every render function
//! injects it for the Rust language. A missed wire (template typo'd
//! key, render fn forgot the `if matches!(lang, Rust)` arm) renders
//! the SSOT inert without breaking the unit tests — this smoke
//! catches that class of drift by asserting the exact `#[derive(...)]`
//! line emitted by `sce-codegen generate -l rust` on representative
//! fixtures.
//!
//! Coverage:
//!   * Codec without `<sce:flag value=>` carrier → `Default` derived,
//!     SSOT trio appended (`#[derive(Default, Debug, Clone, PartialEq)]`).
//!   * Codec with `<sce:flag value=>` carrier → `Default` dropped
//!     (manual `impl Default` below); SSOT trio still emitted.
//!   * Codec with variant body → both struct and variant enum emit
//!     SSOT trio (transitive closure for `body: NameVariant`).
//!   * EventSchema payload, ForgeEnum, BoundedCollectionHandle +
//!     OverflowError — one fixture each.
//!
//! LinkBusEvent is a per-machine artifact emitted by
//! `render_machine_concurrency_artifacts` and isn't reachable through
//! the standalone `generate` CLI surface; the SSOT unit tests in
//! `rust_derive_policy.rs` cover its policy and any consumer that
//! wires up mesh/links exercises the end-to-end path.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

fn scratch(subdir: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_derive_ssot");
    let dir = base.join(subdir);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

fn run_generate(out_dir: &Path, fixture: &Path) {
    let output = Command::new(sce_codegen_bin())
        .args(["generate", "-l", "rust", "-o"])
        .arg(out_dir)
        .arg(fixture)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l rust {} failed (exit {:?})\nstdout: {}\nstderr: {}",
        fixture.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn read_emitted_rs(dir: &Path) -> String {
    let mut bodies = String::new();
    for entry in std::fs::read_dir(dir).expect("read scratch").flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            bodies.push_str(&std::fs::read_to_string(&p).expect("read .rs"));
            bodies.push('\n');
        }
    }
    assert!(!bodies.is_empty(), "no .rs emitted under {}", dir.display());
    bodies
}

/// Codec without `<sce:flag value=>` carrier: `Default` is derived
/// alongside the SSOT trio on the struct, and the SSOT trio alone
/// on any variant enum.
#[test]
fn codec_no_flag_default_emits_default_plus_ssot_trio() {
    let out = scratch("codec_no_flag_default");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/codec_zenoh_keep_alive.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Default, Debug, Clone, PartialEq)]"),
        "codec without flag_default must derive Default + SSOT trio in one attribute; got:\n{src}"
    );
}

/// Codec WITH `<sce:flag value=>` carrier: `Default` is dropped from
/// the derive attribute (manual `impl Default` is emitted below the
/// struct per RFC variant-default-uniformity Atomic β); the SSOT
/// trio still appears.
#[test]
fn codec_with_flag_default_drops_default_keeps_ssot_trio() {
    let out = scratch("codec_with_flag_default");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/codec_zenoh_declare.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Debug, Clone, PartialEq)]"),
        "codec with flag_default must keep SSOT trio without Default; got:\n{src}"
    );
    assert!(
        !src.contains("#[derive(Default, Debug, Clone, PartialEq)]"),
        "codec with flag_default must NOT derive Default (manual impl below); got:\n{src}"
    );
    assert!(
        // Lifetime-agnostic: a borrowed codec emits `impl<'a> Default
        // for Name<'a>`, a fixed-width one `impl Default for Name`. Both
        // satisfy the intent — a MANUAL Default impl exists (not derived).
        src.contains("Default for CodecZenohDeclare"),
        "codec with flag_default must emit manual impl Default; got:\n{src}"
    );
}

/// Codec with variant body: the variant enum carries the same SSOT
/// trio as the struct, because the struct's `body: NameVariant`
/// field requires its derives to be a transitive prefix.
#[test]
fn codec_variant_enum_emits_ssot_trio() {
    let out = scratch("codec_variant");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/codec_variant_dispatch.scxml"),
    );
    let src = read_emitted_rs(&out);
    // Variant enum line lives between #[allow(dead_code)] and `pub enum`.
    let has_variant_derive = src.split("pub enum ").skip(1).any(|after| {
        // Look back ~200 bytes for the derive attribute preceding `pub enum`.
        let prefix = src.split(after).next().unwrap_or("");
        let lookback = &prefix[prefix.len().saturating_sub(200)..];
        lookback.contains("#[derive(Debug, Clone, PartialEq)]")
    });
    assert!(
        has_variant_derive,
        "codec variant enum must derive Debug, Clone, PartialEq; got:\n{src}"
    );
}

/// EventSchema payload struct: SSOT trio (shares baseline with codec).
#[test]
fn event_schema_payload_emits_ssot_trio() {
    let out = scratch("event_schema");
    run_generate(
        &out,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/event_schema/schema_job_completed_multi.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Debug, Clone, PartialEq)]"),
        "event_schema payload must derive Debug, Clone, PartialEq; got:\n{src}"
    );
}

/// ForgeEnum: repr-tagged C-like enum with the Copy-trivial derive
/// set.
#[test]
fn forge_enum_emits_full_derive_set() {
    let out = scratch("forge_enum");
    run_generate(
        &out,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/enum/enum_minimal.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]"),
        "forge_enum must derive full Copy-trivial set; got:\n{src}"
    );
}

/// BoundedCollection emits two distinct categories on different
/// types: `Handle` (includes `Hash` for map-key use) and
/// `OverflowError` (no Hash).
#[test]
fn bounded_collection_handle_and_overflow_error_emit_distinct_sets() {
    let out = scratch("bounded_collection");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/local_sub_table.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]"),
        "BoundedCollectionHandle must include Hash; got:\n{src}"
    );
    assert!(
        src.contains("#[derive(Clone, Copy, PartialEq, Eq, Debug)]"),
        "BoundedCollectionOverflowError must drop Hash; got:\n{src}"
    );
}
