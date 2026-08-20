// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! End-to-end regression gate for `crate::rust_derive_policy`.
//!
//! The SSOT module's unit tests prove the policy itself is correct.
//! This file proves the policy is wired through BOTH Rust-emitting
//! engines: every template consumes the right context key, and every
//! render function injects it for the Rust language. A missed wire
//! (template typo'd key, render fn forgot the injection) renders the
//! SSOT inert without breaking the unit tests — this smoke catches
//! that class of drift by asserting the exact `#[derive(...)]` line
//! emitted on representative fixtures.
//!
//! Forge-engine coverage (`sce-codegen generate -l rust`):
//!   * Codec without `<sce:flag value=>` carrier → `Default` derived,
//!     SSOT trio appended (`#[derive(Default, Debug, Clone, PartialEq)]`).
//!   * Codec with `<sce:flag value=>` carrier → `Default` dropped
//!     (manual `impl Default` below); SSOT trio still emitted.
//!   * Codec with variant body → both struct and variant enum emit
//!     SSOT trio (transitive closure for `body: NameVariant`).
//!   * EventSchema payload, ForgeEnum, BoundedCollectionHandle +
//!     OverflowError — one fixture each.
//!   * Forge procedure `State` / `Event` enums → SSOT defaults,
//!     byte-identical to the pre-SSOT hardcoded line.
//!
//! Statechart-engine coverage:
//!   * `{machine}State` / `{machine}Event` enums → SSOT defaults
//!     (includes `Hash`), byte-identical to the pre-SSOT hardcoded
//!     line (via CLI).
//!   * Caller-injected extra derives appear on both enums, and an
//!     empty extras set leaves the line unchanged (via the library
//!     `compile_scxml_lang_typed_with_section` + `StatechartCodegenOptions`
//!     path — the CLI has no extras flag).
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
/// struct per RFC variant-default-uniformity); the SSOT
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

/// Lookup string-output enum flows through the SSOT (ForgeEnum shape).
#[test]
fn lookup_enum_emits_forge_enum_set() {
    let out = scratch("lookup");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/lookup_engine_status.scxml"),
    );
    let src = read_emitted_rs(&out);
    assert!(
        src.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]"),
        "lookup enum must carry the ForgeEnum set via SSOT; got:\n{src}"
    );
}

/// Observer `ForgeDomainTag` flows through the SSOT (ForgeEnum shape).
#[test]
fn observer_domain_tag_emits_forge_enum_set() {
    let out = scratch("observer");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/crossfile_observer_condition.scxml"),
    );
    let src = read_emitted_rs(&out);
    let has = src
        .split("pub enum ForgeDomainTag")
        .next()
        .is_some_and(|before| {
            before
                .rsplit("#[derive(")
                .next()
                .map(|_| before.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]"))
                .unwrap_or(false)
        });
    assert!(
        has,
        "ForgeDomainTag must carry the ForgeEnum set via SSOT; got:\n{src}"
    );
}

/// Validator `ValidationResult` flows through the SSOT (Debug-only).
#[test]
fn validator_result_emits_debug_only() {
    let out = scratch("validator");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/crossfile_validator_lookup.scxml"),
    );
    let src = read_emitted_rs(&out);
    // The Debug-only line precedes `pub struct ValidationResult`.
    let before = src
        .split("pub struct ValidationResult")
        .next()
        .unwrap_or("");
    assert!(
        before.trim_end().ends_with("#[derive(Debug)]")
            || before.contains("#[derive(Debug)]\n#[allow(dead_code)]"),
        "ValidationResult must carry Debug via SSOT; got:\n{src}"
    );
}

/// Codec owned mirror (`{Struct}Owned`) reuses the wire-typed codec
/// trio through the SSOT — it cannot drift from the borrowed struct.
#[test]
fn codec_owned_mirror_emits_ssot_trio() {
    let out = scratch("codec_owned");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/codec_zenoh_declare.scxml"),
    );
    let src = read_emitted_rs(&out);
    let before = src
        .split("pub struct CodecZenohDeclareOwned")
        .next()
        .unwrap_or("");
    assert!(
        before
            .trim_end()
            .ends_with("#[derive(Debug, Clone, PartialEq)]"),
        "codec owned mirror must carry the codec trio via SSOT; got:\n{src}"
    );
}

/// Buffer-pool `SlotState` repr(u8) enum flows through the SSOT
/// (ForgeEnum shape — order normalized to the SSOT).
#[test]
fn buffer_pool_slot_state_emits_forge_enum_set() {
    let out = scratch("buffer_pool");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/buffer_pool_ast_export_min.scxml"),
    );
    let src = read_emitted_rs(&out);
    let before = src.split("pub enum SlotState").next().unwrap_or("");
    assert!(
        before.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]"),
        "SlotState must carry the ForgeEnum set via SSOT; got:\n{src}"
    );
}

/// Forge procedure `State` / `Event` enums flow through the SSOT with
/// defaults only — byte-identical to the pre-SSOT hardcoded line.
#[test]
fn procedure_state_event_enums_emit_ssot_defaults() {
    let out = scratch("procedure");
    run_generate(
        &out,
        &repo_root().join("tests/forge/resources/procedure_linear.scxml"),
    );
    let src = read_emitted_rs(&out);
    // Both `pub enum State` and `pub enum Event` carry the repr-tagged
    // set with no Hash. Exactly two occurrences (one per enum).
    let count = src
        .matches("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
        .count();
    assert_eq!(
        count, 2,
        "procedure must emit the SSOT default derive on both State and Event; got {count}:\n{src}"
    );
    assert!(
        !src.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"),
        "procedure enums must NOT carry Hash (that is the statechart set); got:\n{src}"
    );
}

/// Statechart `{machine}State` / `{machine}Event` enums flow through
/// the SSOT with defaults (including `Hash`). The two enums differ by
/// exactly `Default`: the state enum carries it (its initial-state
/// `#[default]` marker), the event enum does not (no default event).
#[test]
fn statechart_state_event_enums_emit_ssot_defaults() {
    let out = scratch("statechart_defaults");
    run_generate(
        &out,
        &repo_root().join("examples/traffic_light/traffic_light.scxml"),
    );
    let src = read_emitted_rs(&out);
    // State enum: SSOT set + `Default` (exactly once).
    assert_eq!(
        src.matches("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]")
            .count(),
        1,
        "statechart State enum must carry the SSOT set plus Default; got:\n{src}"
    );
    // Event enum: SSOT set WITHOUT Default (exactly once). The
    // `Hash, Default)]` state line does not match this `Hash)]` needle.
    assert_eq!(
        src.matches("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
            .count(),
        1,
        "statechart Event enum must carry the SSOT set without Default; got:\n{src}"
    );
    // The state enum marks its `<scxml initial="red">` variant `#[default]`
    // so `Default` resolves to the initial state — exactly one marker.
    assert_eq!(
        src.matches("#[default]").count(),
        1,
        "exactly one #[default] marker (the initial state); got:\n{src}"
    );
    assert!(
        src.contains("#[default]\n    Red,"),
        "the #[default] marker must sit on the initial-state variant Red; got:\n{src}"
    );
}

/// Statechart publishes its structural external-trigger surface as a
/// `pub const EXTERNALLY_DRIVABLE_EVENTS` slice: non-reserved
/// `<transition event>` targets minus internally `<raise>`d events and
/// the `Null` sentinel. The `widget_patterns/button` fixture exercises
/// the subtraction — it `<raise event="click">`s an event that never
/// triggers a transition, so `Click` is an enum variant but NOT a member
/// of the drivable set. This locks the derive-consumable partition a
/// downstream name-parser reconstructs `from_name` from.
#[test]
fn statechart_emits_externally_drivable_event_const() {
    let out = scratch("externally_drivable");
    run_generate(
        &out,
        &repo_root().join("examples/widget_patterns/button.scxml"),
    );
    let src = read_emitted_rs(&out);
    // The drivable surface is an associated const on the Event enum
    // (`ButtonEvent::EXTERNALLY_DRIVABLE_EVENTS`) — associated, not free,
    // so glob-re-exported machines never collide on the name.
    assert!(
        src.contains("impl ButtonEvent {")
            && src.contains("pub const EXTERNALLY_DRIVABLE_EVENTS: &'static [ButtonEvent] = &["),
        "must emit the associated drivable-events const on the Event enum; got:\n{src}"
    );
    // Every non-reserved transition trigger is a member.
    for ev in [
        "PointerEnter",
        "PointerLeave",
        "PointerDown",
        "PointerUp",
        "Enable",
        "Disable",
    ] {
        assert!(
            src.contains(&format!("ButtonEvent::{ev},")),
            "external trigger {ev} must be a drivable member; got:\n{src}"
        );
    }
    // `Click` is `<raise>`d only (never a transition trigger): it is an
    // enum variant but must be excluded from the drivable const.
    assert!(
        src.contains("Click,"),
        "Click must still be an Event enum variant; got:\n{src}"
    );
    // Isolate the const body and assert Click / Null are absent from it.
    let body_start = src
        .find("EXTERNALLY_DRIVABLE_EVENTS")
        .expect("const present");
    let const_body = &src[body_start..src[body_start..].find("];").unwrap() + body_start];
    assert!(
        !const_body.contains("ButtonEvent::Click"),
        "internally-raised Click must NOT be externally drivable; got:\n{const_body}"
    );
    assert!(
        !const_body.contains("ButtonEvent::Null"),
        "the Null sentinel must NOT be externally drivable; got:\n{const_body}"
    );
}

/// Statechart caller-injected extra derives appear on both generated
/// enums. Exercised through the library
/// `compile_scxml_lang_typed_with_section` with `StatechartCodegenOptions`
/// — the exact plumbing a downstream `build.rs` consumer
/// (`compile_scxml_with_derives`) drives — because the `generate` CLI
/// surface has no extras flag.
#[test]
fn statechart_caller_injected_extra_derives_appear() {
    use sce_build::generator::{Language, StatechartCodegenOptions};

    let template_dir = sce_build::find_template_dir_for(Language::Rust);
    let fixture = repo_root().join("examples/traffic_light/traffic_light.scxml");
    let opts = StatechartCodegenOptions {
        no_std: false,
        state_extra_derives: vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
            "my_ui::StateName".to_string(),
        ],
        event_extra_derives: vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
            "my_ui::EventName".to_string(),
        ],
        host_processor_types: Vec::new(),
        host_invoker_types: Vec::new(),
    };
    let output = sce_build::compile_scxml_lang_typed_with_section(
        fixture.to_str().unwrap(),
        &template_dir,
        Language::Rust,
        None,
        None,
        &opts,
    )
    .expect("compile statechart with extra derives");
    let src = &output.files[0].1;

    // State enum carries defaults + its extras, appended after the
    // SSOT set and deduped (Debug etc. appear once).
    assert!(
        src.contains(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize, my_ui::StateName)]"
        ),
        "State enum must carry defaults (incl. Default) + state extras verbatim; got:\n{src}"
    );
    // Event enum carries its own extras (StateName vs EventName differ,
    // proving state/event extras are wired to distinct fields).
    assert!(
        src.contains(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, my_ui::EventName)]"
        ),
        "Event enum must carry defaults + event extras verbatim; got:\n{src}"
    );
}

/// Statechart with empty extras is byte-identical to the defaults path
/// — the injection channel is inert when unused, so existing consumers
/// are unaffected.
#[test]
fn statechart_empty_extras_leaves_line_unchanged() {
    use sce_build::generator::{Language, StatechartCodegenOptions};

    let template_dir = sce_build::find_template_dir_for(Language::Rust);
    let fixture = repo_root().join("examples/traffic_light/traffic_light.scxml");
    let output = sce_build::compile_scxml_lang_typed_with_section(
        fixture.to_str().unwrap(),
        &template_dir,
        Language::Rust,
        None,
        None,
        &StatechartCodegenOptions::default(),
    )
    .expect("compile statechart with default options");
    let src = &output.files[0].1;
    // State enum: SSOT default + Default; Event enum: SSOT default without
    // Default. Empty extras leave both at the category default (nothing
    // appended after the SSOT set).
    assert_eq!(
        src.matches("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]")
            .count(),
        1,
        "empty extras must leave the State enum at its SSOT default (incl. Default); got:\n{src}"
    );
    assert_eq!(
        src.matches("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
            .count(),
        1,
        "empty extras must leave the Event enum at its SSOT default; got:\n{src}"
    );
    assert!(
        !src.contains("serde::Serialize"),
        "empty extras must not inject anything; got:\n{src}"
    );
}
