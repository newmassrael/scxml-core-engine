//! Fragment / reassembly buffer-pool variant codegen surface.
//!
//! Per watching-zenoh RFC §synth-5-M lines 2680-2698 (variant schema) +
//! 2864-2876 (fragment FSM consumer) + 2976-2981 (codegen self-check
//! anchor) + 2659-2664 (backend coverage). `8c6b4e1e` shipped the
//! `BufferPoolVariant::Reassembly(ReassemblyConfig)` schema + 2
//! parse-time validators; the cross-doc validators folded into the
//! deploy-time reassembly checks (`c7287424`). This surface closes
//! the chain by extending the Rust + C11
//! buffer-pool templates with the reassembly-variant per-slot state
//! (fragment-index bitmap + deadline + ZID peer-id) the author-level
//! Fragment FSM (`docs/reassembly-fsm.md` §2) consumes.
//!
//! Backend coverage per RFC §synth-5-M lines 2659-2664 — emits only on
//! `(rust, *)` + `(c11, bare_metal)`. Non-MCU backends inherit
//! `codegen/mcu-class-kind-on-non-mcu-language` rejection from the
//! existing `ForgeKind::BufferPool` axis.
//!
//! ## Test surface
//!
//! - Positive emit-shape (Rust): reassembly variant emits the spec-
//!   anchored constants + ZID typedef + ReassemblySlot struct + drift
//!   guard asserts.
//! - Positive emit-shape (C11): same shape with macro + typedef +
//!   `_Static_assert` mirrors.
//! - Variant exclusivity (Rust + C11): the reassembly state ONLY
//!   emits under `<sce:variant>reassembly`; default-variant pools
//!   never carry the bitmap/deadline/peer-id machinery.
//! - Codegen self-check force-fixture: synthesize the
//!   ValidationError → Diagnostic → DiagnosticCode round-trip
//!   directly so the regression-guard diagnostic has a live consumer
//!   per `feedback_silently_broken_hooks.md`. In normal use the
//!   template always emits the 16-byte ZID typedef (the cross-doc
//!   validator `reassembly/untrusted-link-binding` gates non-
//!   `established_session` bindings upstream), so the self-check
//!   never fires from a live render — mirrors the
//!   `mem/inter-pool-padding-not-emitted` precedent at
//!   `c5_cache_maintenance.rs::pool_cache_pre_arm_invalidate_missing_force_fixture`.
//! - Cross-backend drift guard: `FRAGMENT_BITMAP_WORDS` derivation
//!   agrees between Rust and C11 emit paths.

use sce_build::compile_forge_with_imports;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::generator::Language;
use sce_build::{find_template_dir_for, DocumentLabel, ForgeCompileOptions};

fn reassembly_scxml(
    name: &str,
    slot_count: u32,
    slot_size: u32,
    max_fragments_per_message: u32,
    reassembly_timeout_ms: u32,
    per_peer_quota: u32,
) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="{name}" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>{slot_count}</sce:slot-count>
  <sce:slot-size>{slot_size}</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
  <sce:max-fragments-per-message>{max_fragments_per_message}</sce:max-fragments-per-message>
  <sce:reassembly-timeout-ms>{reassembly_timeout_ms}</sce:reassembly-timeout-ms>
  <sce:per-peer-quota>{per_peer_quota}</sce:per-peer-quota>
</scxml>"##,
    )
}

fn default_pool_scxml(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="{name}" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##,
    )
}

fn label_for(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

fn compile_for(language: Language, scxml: &str, label_id: &'static str) -> String {
    let label = label_for(label_id);
    let out = compile_forge_with_imports(
        scxml,
        label,
        language,
        &find_template_dir_for(language),
        &ForgeCompileOptions::default(),
    )
    .expect("buffer-pool reassembly variant compiles");
    out.files
        .iter()
        .find(|(name, _)| {
            // Rust emits `<snake>.rs`, C11 emits `<snake>.h`.
            name.starts_with(label_id) || name.starts_with("rx_reassembly_pool")
        })
        .map_or_else(
            || {
                // Fall back to the first emitted file (handles linker
                // sidecars trailing the .h header).
                out.files.first().unwrap().1.clone()
            },
            |(_, body)| body.clone(),
        )
}

// ────────────────────────────────────────────────────────────────────
// Positive emit-shape — Rust backend
// ────────────────────────────────────────────────────────────────────

#[test]
fn rust_reassembly_variant_emits_zid_peer_id_typedef() {
    // RFC §synth-5-M lines 2708-2714: peer-id keys on ZID, never wire source.
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::Rust, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("pub type PeerId = [u8; 16]"),
        "ZID peer-id typedef must emit on reassembly variant; got:\n{body}"
    );
}

#[test]
fn rust_reassembly_variant_emits_bitmap_word_const() {
    // RFC §synth-5-M line 2696 + 2688: bitmap width = ceil(max-fragments / 32).
    // For max_fragments_per_message=16 → 1 word. For 64 → 2 words.
    let scxml16 = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body16 = compile_for(Language::Rust, &scxml16, "rx_reassembly_pool");
    assert!(
        body16.contains("pub const FRAGMENT_BITMAP_WORDS: usize = (16 + 31) / 32;"),
        "FRAGMENT_BITMAP_WORDS const must derive from max-fragments-per-message; got:\n{body16}"
    );

    let scxml64 = reassembly_scxml("rx_reassembly_pool", 4, 4096, 64, 500, 2);
    let body64 = compile_for(Language::Rust, &scxml64, "rx_reassembly_pool");
    assert!(
        body64.contains("pub const FRAGMENT_BITMAP_WORDS: usize = (64 + 31) / 32;"),
        "FRAGMENT_BITMAP_WORDS recomputes per max-fragments value; got:\n{body64}"
    );
}

#[test]
fn rust_reassembly_variant_emits_reassembly_slot_struct() {
    // RFC §synth-5-M lines 2680-2698: per-slot state = bitmap + deadline + peer_id.
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::Rust, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("pub struct ReassemblySlot"),
        "ReassemblySlot struct must emit;\n{body}"
    );
    assert!(
        body.contains("pub bitmap: [u32; FRAGMENT_BITMAP_WORDS]"),
        "ReassemblySlot.bitmap field width fixed to [u32; ...]; got:\n{body}"
    );
    assert!(
        body.contains("pub deadline: u64"),
        "ReassemblySlot.deadline width fixed to u64; got:\n{body}"
    );
    assert!(
        body.contains("pub peer_id: PeerId"),
        "ReassemblySlot.peer_id field must use the ZID typedef; got:\n{body}"
    );
}

#[test]
fn rust_reassembly_variant_emits_config_constants() {
    // RFC §synth-5-M lines 2688-2690: MAX_FRAGMENTS_PER_MESSAGE +
    // REASSEMBLY_TIMEOUT_MS + PER_PEER_QUOTA round-trip from the
    // schema into the generated code so the SCXML algorithm body
    // can reach them without re-parsing deploy.yaml.
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::Rust, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("pub const MAX_FRAGMENTS_PER_MESSAGE: u32 = 16;"),
        "MAX_FRAGMENTS_PER_MESSAGE const must round-trip from schema; got:\n{body}"
    );
    assert!(
        body.contains("pub const REASSEMBLY_TIMEOUT_MS: u32 = 500;"),
        "REASSEMBLY_TIMEOUT_MS const must round-trip from schema; got:\n{body}"
    );
    assert!(
        body.contains("pub const PER_PEER_QUOTA: u32 = 2;"),
        "PER_PEER_QUOTA const must round-trip from schema; got:\n{body}"
    );
}

#[test]
fn rust_reassembly_variant_emits_size_drift_asserts() {
    // Drift guards pin the wire-shape: u32 bitmap word, u64 deadline,
    // 16-byte PeerId. Without these a future 16-bit port silently
    // redefines the layout the author FSM observes.
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::Rust, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("core::mem::size_of::<u32>() == 4"),
        "u32 width drift guard must emit;\n{body}"
    );
    assert!(
        body.contains("core::mem::size_of::<u64>() == 8"),
        "u64 width drift guard must emit;\n{body}"
    );
    assert!(
        body.contains("core::mem::size_of::<PeerId>() == 16"),
        "PeerId 16-byte drift guard must emit;\n{body}"
    );
}

// ────────────────────────────────────────────────────────────────────
// Positive emit-shape — C11 backend
// ────────────────────────────────────────────────────────────────────

#[test]
fn c11_reassembly_variant_emits_zid_peer_id_typedef() {
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("typedef uint8_t rx_reassembly_pool_peer_id_t[16];"),
        "ZID peer-id typedef must emit on reassembly variant; got:\n{body}"
    );
}

#[test]
fn c11_reassembly_variant_emits_bitmap_word_macro() {
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("#define RX_REASSEMBLY_POOL_FRAGMENT_BITMAP_WORDS"),
        "FRAGMENT_BITMAP_WORDS macro must emit; got:\n{body}"
    );
    assert!(
        body.contains("(16 + 31) / 32"),
        "FRAGMENT_BITMAP_WORDS macro body must derive from max-fragments; got:\n{body}"
    );
}

#[test]
fn c11_reassembly_variant_emits_reassembly_slot_struct() {
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("rx_reassembly_pool_reassembly_slot_t"),
        "reassembly_slot_t typedef must emit;\n{body}"
    );
    assert!(
        body.contains("uint32_t bitmap[RX_REASSEMBLY_POOL_FRAGMENT_BITMAP_WORDS]"),
        "Bitmap field width fixed to uint32_t array; got:\n{body}"
    );
    assert!(
        body.contains("uint64_t deadline"),
        "Deadline field width fixed to uint64_t; got:\n{body}"
    );
    assert!(
        body.contains("rx_reassembly_pool_peer_id_t peer_id"),
        "peer_id field must use the per-pool ZID typedef; got:\n{body}"
    );
}

#[test]
fn c11_reassembly_variant_emits_config_macros() {
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("#define RX_REASSEMBLY_POOL_MAX_FRAGMENTS_PER_MESSAGE ((uint32_t)16)"),
        "MAX_FRAGMENTS_PER_MESSAGE macro must round-trip from schema; got:\n{body}"
    );
    assert!(
        body.contains("#define RX_REASSEMBLY_POOL_REASSEMBLY_TIMEOUT_MS ((uint32_t)500)"),
        "REASSEMBLY_TIMEOUT_MS macro must round-trip from schema; got:\n{body}"
    );
    assert!(
        body.contains("#define RX_REASSEMBLY_POOL_PER_PEER_QUOTA ((uint32_t)2)"),
        "PER_PEER_QUOTA macro must round-trip from schema; got:\n{body}"
    );
}

#[test]
fn c11_reassembly_variant_emits_static_assert_drift_guards() {
    let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, 16, 500, 2);
    let body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
    assert!(
        body.contains("_Static_assert(sizeof(uint32_t) == 4"),
        "uint32_t width drift guard must emit;\n{body}"
    );
    assert!(
        body.contains("_Static_assert(sizeof(uint64_t) == 8"),
        "uint64_t width drift guard must emit;\n{body}"
    );
    assert!(
        body.contains("_Static_assert(sizeof(rx_reassembly_pool_peer_id_t) == 16"),
        "peer-id 16-byte drift guard must emit;\n{body}"
    );
}

// ────────────────────────────────────────────────────────────────────
// Variant exclusivity — default pools do NOT carry reassembly state
// ────────────────────────────────────────────────────────────────────

#[test]
fn rust_default_variant_does_not_emit_reassembly_state() {
    let scxml = default_pool_scxml("rx_default_pool");
    let body = compile_for(Language::Rust, &scxml, "rx_default_pool");
    assert!(
        !body.contains("pub type PeerId"),
        "default variant must not carry ZID peer-id typedef;\n{body}"
    );
    assert!(
        !body.contains("FRAGMENT_BITMAP_WORDS"),
        "default variant must not carry bitmap word const;\n{body}"
    );
    assert!(
        !body.contains("ReassemblySlot"),
        "default variant must not carry ReassemblySlot struct;\n{body}"
    );
    assert!(
        !body.contains("REASSEMBLY_TIMEOUT_MS"),
        "default variant must not carry reassembly config consts;\n{body}"
    );
}

#[test]
fn c11_default_variant_does_not_emit_reassembly_state() {
    let scxml = default_pool_scxml("rx_default_pool");
    let body = compile_for(Language::C11, &scxml, "rx_default_pool");
    assert!(
        !body.contains("peer_id_t"),
        "default variant must not carry peer_id_t typedef;\n{body}"
    );
    assert!(
        !body.contains("FRAGMENT_BITMAP_WORDS"),
        "default variant must not carry bitmap word macro;\n{body}"
    );
    assert!(
        !body.contains("reassembly_slot_t"),
        "default variant must not carry reassembly_slot_t typedef;\n{body}"
    );
    assert!(
        !body.contains("REASSEMBLY_TIMEOUT_MS"),
        "default variant must not carry reassembly config macros;\n{body}"
    );
}

// ────────────────────────────────────────────────────────────────────
// Codegen self-check — force-fixture round-trip
//
// The `reassembly/peer-id-not-zid-on-established-session` diagnostic
// guards against a future template edit that drops the 16-byte ZID
// typedef. In normal use the template always emits the typedef by
// construction; this fixture drives the
// ValidationError → Diagnostic → DiagnosticCode pipeline directly so
// the diagnostic has a live consumer per
// `feedback_silently_broken_hooks.md`. Mirrors the β
// `mem/inter-pool-padding-not-emitted` codegen self-check shape and
// the C5 `pool_cache_pre_arm_invalidate_missing_force_fixture`
// precedent at `c5_cache_maintenance.rs:518`.
// ────────────────────────────────────────────────────────────────────

#[test]
fn reassembly_peer_id_zid_self_check_force_fixture_rust() {
    let err: ForgeError = ValidationError::ReassemblyPeerIdNotZidOnEstablishedSession {
        pool_name: "rx_reassembly_pool".into(),
        language: "rust".into(),
    }
    .into();
    let located: Located<ForgeError> = Located::new(err, "rx_reassembly_pool.scxml", None, None);
    let diags = located.to_diagnostics();
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert!(
        matches!(
            d.code,
            DiagnosticCode::ReassemblyPeerIdNotZidOnEstablishedSession
        ),
        "must be DiagnosticCode::ReassemblyPeerIdNotZidOnEstablishedSession; got {:?}",
        d.code,
    );
    assert!(
        d.message.contains("rx_reassembly_pool"),
        "message must name the offending pool; got {}",
        d.message,
    );
    assert!(
        d.message.contains("rust backend"),
        "message must name the offending backend; got {}",
        d.message,
    );
    assert!(
        d.message.contains("§5.M line 2976-2981"),
        "message must cite the spec anchor; got {}",
        d.message,
    );
    assert!(
        d.message.contains("16-byte ZID"),
        "message must name the required peer-id shape; got {}",
        d.message,
    );
}

#[test]
fn reassembly_peer_id_zid_self_check_force_fixture_c11() {
    let err: ForgeError = ValidationError::ReassemblyPeerIdNotZidOnEstablishedSession {
        pool_name: "rx_reassembly_pool".into(),
        language: "c11".into(),
    }
    .into();
    let located: Located<ForgeError> = Located::new(err, "rx_reassembly_pool.scxml", None, None);
    let diags = located.to_diagnostics();
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert!(matches!(
        d.code,
        DiagnosticCode::ReassemblyPeerIdNotZidOnEstablishedSession
    ));
    assert!(
        d.message.contains("c11 backend"),
        "message must name the c11 backend; got {}",
        d.message,
    );
}

// ────────────────────────────────────────────────────────────────────
// Cross-backend drift guard
// ────────────────────────────────────────────────────────────────────

#[test]
fn rust_and_c11_agree_on_bitmap_words_derivation() {
    // The bitmap-word count expression `(max_fragments + 31) / 32`
    // must agree between Rust + C11 — both consume the same emitted
    // constant when the author FSM marks fragments received. A
    // backend-specific override would silently desync the wire shape
    // on cross-language bridges.
    for max_fragments in [1u32, 16, 32, 33, 64, 96, 128] {
        let scxml = reassembly_scxml("rx_reassembly_pool", 4, 4096, max_fragments, 500, 2);
        let rust_body = compile_for(Language::Rust, &scxml, "rx_reassembly_pool");
        let c11_body = compile_for(Language::C11, &scxml, "rx_reassembly_pool");
        let expected = format!("({max_fragments} + 31) / 32");
        assert!(
            rust_body.contains(&expected),
            "rust backend must derive bitmap words from `{expected}` (max_fragments={max_fragments}); got:\n{rust_body}"
        );
        assert!(
            c11_body.contains(&expected),
            "c11 backend must derive bitmap words from `{expected}` (max_fragments={max_fragments}); got:\n{c11_body}"
        );
    }
}
