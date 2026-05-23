//! NL→IR Mapping Roadmap Item C1 Path A Atomic 1 — Enum kind IR.
//!
//! Verifies the parse-time invariants of `sce:kind="enum"` documents
//! per design RFC §2 (DL-3'). Atomic 1 ships parser + IR only; codegen
//! is gated to `template_ships(Enum, *) = false` and lands in Atomic 2.
//!
//! Coverage:
//! - 3 positive fixtures: minimal (3 variants, uint8), wide (uint16
//!   spread), hex-formatted values
//! - 5 negative fixtures: no-variants, duplicate-name, duplicate-value,
//!   value-overflows-underlying, unsupported-underlying-type
//! - Cross-cutting: ForgeKind::Enum is supported, listed in
//!   `ALL_ATTR_NAMES`, displays as "enum", classifies as Generic, and
//!   `template_ships(Enum, *) = false` on every backend
//! - Lattice: `SceType::Enum(EnumRef)` parses from `enum:Alias`;
//!   `InferredType::from_sce_type` returns `Unknown` (cross-doc
//!   resolution defers to Atomic 5 narrowing)

use sce_build::forge::error::ValidationError;
use sce_build::forge::model::{EnumRef, ForgeDocument, ForgeKind, RuntimeDep, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::types::InferredType;
use sce_build::DocumentLabel;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/enum")
        .join(format!("{name}.scxml"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn label_for(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: name,
    }
}

// ── ForgeKind surface invariants ────────────────────────────────

#[test]
fn forge_kind_enum_is_supported() {
    assert!(ForgeKind::Enum.is_supported());
    assert_eq!(ForgeKind::Enum.to_string(), "enum");
    assert_eq!(ForgeKind::from_attr("enum"), Some(ForgeKind::Enum));
    assert!(ForgeKind::ALL_ATTR_NAMES.contains(&"enum"));
}

#[test]
fn forge_kind_enum_is_stateless_zero_runtime_dep() {
    assert!(!ForgeKind::Enum.needs_instance());
    assert!(!ForgeKind::Enum.is_inline_eligible());
    assert_eq!(ForgeKind::Enum.max_runtime_dep(), RuntimeDep::None);
}

#[test]
fn forge_kind_enum_count_is_seventeen() {
    // Atomic 1: pre-existing 16 + Enum = 17. EventSchema is the 18th
    // kind landed by Atomic 3.
    assert_eq!(ForgeKind::ALL_ATTR_NAMES.len(), 17);
}

// ── codegen_matrix gate (Atomic 1: no codegen) ───────────────────

#[test]
fn enum_template_does_not_ship_on_any_backend_in_atomic_1() {
    use sce_build::forge::codegen_matrix::template_ships;
    use sce_build::generator::Language;
    for lang in [
        Language::Cpp,
        Language::Rust,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert!(
            !template_ships(ForgeKind::Enum, lang),
            "Atomic 1 expects template_ships(Enum, {lang:?}) = false; \
             Atomic 2 flips this on per-backend lockstep"
        );
    }
}

// ── SceType lattice ──────────────────────────────────────────────

#[test]
fn sce_type_from_attr_parses_enum_alias() {
    let parsed = SceType::from_attr("enum:Result");
    assert_eq!(
        parsed,
        Some(SceType::Enum(EnumRef {
            alias: "Result".into()
        }))
    );
}

#[test]
fn sce_type_from_attr_rejects_empty_alias() {
    assert_eq!(SceType::from_attr("enum:"), None);
    assert_eq!(SceType::from_attr("enum:   "), None);
}

#[test]
fn sce_type_enum_inferred_type_is_unknown_until_resolution() {
    // Atomic 1 stance: the inference layer declines to claim the
    // type (Unknown). Atomic 5 literal-width narrowing resolves
    // against the imported EnumModel's underlying_type.
    let t = SceType::Enum(EnumRef {
        alias: "Result".into(),
    });
    assert!(matches!(
        InferredType::from_sce_type(&t),
        InferredType::Unknown
    ));
    assert!(!t.is_unsigned());
    assert!(!t.is_signed());
    assert!(!t.is_float());
    assert_eq!(t.int_bit_width(), None);
}

// ── Positive parsing ─────────────────────────────────────────────

#[test]
fn positive_enum_minimal_parses() {
    let xml = fixture("enum_minimal");
    let doc = parse_forge(&xml, label_for("enum_minimal"))
        .expect("parse succeeds")
        .expect("not a statechart");
    let m = match doc {
        ForgeDocument::Enum(m) => m,
        other => panic!("expected Enum, got {:?}", other.kind()),
    };
    assert_eq!(m.name, "enum_minimal");
    assert_eq!(m.underlying_type, SceType::Uint8);
    assert_eq!(m.variants.len(), 3);
    assert_eq!(m.variants[0].name, "ok");
    assert_eq!(m.variants[0].value, 0);
    assert_eq!(m.variants[1].name, "error");
    assert_eq!(m.variants[1].value, 1);
    assert_eq!(m.variants[2].name, "timeout");
    assert_eq!(m.variants[2].value, 2);
}

#[test]
fn positive_enum_wide_parses_uint16_underlying() {
    let xml = fixture("enum_wide");
    let doc = parse_forge(&xml, label_for("enum_wide"))
        .expect("parse succeeds")
        .expect("not a statechart");
    let m = match doc {
        ForgeDocument::Enum(m) => m,
        _ => panic!("expected Enum"),
    };
    assert_eq!(m.underlying_type, SceType::Uint16);
    assert_eq!(m.variants.len(), 6);
    // The 40000 variant requires uint16 (would overflow uint8).
    let custom = m.variants.iter().find(|v| v.name == "custom_high").unwrap();
    assert_eq!(custom.value, 40000);
}

#[test]
fn positive_enum_hex_values_parses() {
    let xml = fixture("enum_hex_values");
    let doc = parse_forge(&xml, label_for("enum_hex_values"))
        .expect("parse succeeds")
        .expect("not a statechart");
    let m = match doc {
        ForgeDocument::Enum(m) => m,
        _ => panic!("expected Enum"),
    };
    assert_eq!(
        m.variants.iter().find(|v| v.name == "read").unwrap().value,
        0x10
    );
    assert_eq!(
        m.variants.iter().find(|v| v.name == "write").unwrap().value,
        0x22
    );
    assert_eq!(
        m.variants.iter().find(|v| v.name == "reset").unwrap().value,
        0xff
    );
}

#[test]
fn positive_enum_doc_runtime_dep_is_none() {
    let xml = fixture("enum_minimal");
    let doc = parse_forge(&xml, label_for("enum_minimal"))
        .unwrap()
        .unwrap();
    assert_eq!(doc.runtime_dep(), RuntimeDep::None);
    assert_eq!(doc.kind(), ForgeKind::Enum);
    assert_eq!(doc.name(), "enum_minimal");
}

// ── Negative parsing ─────────────────────────────────────────────

fn expect_parse_error(fixture_name: &str) -> ValidationError {
    let xml = fixture(fixture_name);
    let err = parse_forge(&xml, label_for("neg")).expect_err("parse must fail");
    match err.error {
        sce_build::forge::error::ForgeError::Validation(v) => *v,
        other => panic!("expected ValidationError, got {other:?}"),
    }
}

#[test]
fn negative_enum_no_variants_rejects() {
    let err = expect_parse_error("negative_enum_no_variants");
    assert!(
        matches!(err, ValidationError::EnumNoVariants { ref name } if name == "neg"),
        "got {err:?}"
    );
}

#[test]
fn negative_enum_duplicate_name_rejects() {
    let err = expect_parse_error("negative_enum_duplicate_name");
    assert!(
        matches!(err, ValidationError::EnumVariantDuplicateName { ref name, .. } if name == "ok"),
        "got {err:?}"
    );
}

#[test]
fn negative_enum_duplicate_value_rejects() {
    let err = expect_parse_error("negative_enum_duplicate_value");
    assert!(
        matches!(
            err,
            ValidationError::EnumVariantDuplicateValue { value: 7, .. }
        ),
        "got {err:?}"
    );
}

#[test]
fn negative_enum_value_overflow_rejects() {
    let err = expect_parse_error("negative_enum_value_overflow");
    assert!(
        matches!(
            err,
            ValidationError::EnumVariantValueOverflowsUnderlying {
                value: 256, ref underlying, ..
            } if underlying == "uint8"
        ),
        "got {err:?}"
    );
}

#[test]
fn negative_enum_unsupported_underlying_rejects() {
    let err = expect_parse_error("negative_enum_unsupported_underlying");
    assert!(
        matches!(
            err,
            ValidationError::EnumUnsupportedUnderlyingType { ref declared, .. }
                if declared == "float32"
        ),
        "got {err:?}"
    );
}
