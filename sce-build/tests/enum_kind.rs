//! NL→IR Mapping Roadmap Item C1 Path A — Enum kind IR.
//!
//! Verifies the parse-time invariants of `sce:kind="enum"` documents
//! per design RFC §2 (DL-3'). This file covers parser + IR plus the
//! `template_ships` codegen-matrix gate.
//!
//! Coverage:
//! - 3 positive fixtures: minimal (3 variants, uint8), wide (uint16
//!   spread), hex-formatted values
//! - 5 negative fixtures: no-variants, duplicate-name, duplicate-value,
//!   value-overflows-underlying, unsupported-underlying-type
//! - Cross-cutting: ForgeKind::Enum is supported, listed in
//!   `ALL_ATTR_NAMES`, displays as "enum", classifies as Generic, and
//!   `template_ships(Enum, *)` holds on every backend
//! - Lattice: `SceType::Enum(EnumRef)` parses from `enum:Alias`;
//!   `InferredType::from_sce_type` returns `Unknown` (cross-doc
//!   resolution happens at literal-width narrowing)

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
fn forge_kind_enum_count_is_eighteen() {
    // Pre-existing 16 + Enum = 17; EventSchema is the 18th kind.
    assert_eq!(ForgeKind::ALL_ATTR_NAMES.len(), 18);
}

// ── codegen_matrix gate (6-backend lockstep) ────────────────────

#[test]
fn enum_template_ships_on_all_six_backends() {
    use sce_build::forge::codegen_matrix::template_ships;
    use sce_build::generator::Language;
    // NL→IR Item C1 Path A acceptance gate per design RFC §5.2:
    // Enum lowers to a backend-native typed enum on every Generic-class
    // backend; the matrix flips in lockstep with the 6
    // templates under `tools/codegen/templates/forge/<lang>/enum.<ext>.jinja2`.
    for lang in [
        Language::Cpp,
        Language::Rust,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert!(
            template_ships(ForgeKind::Enum, lang),
            "Enum codegen expects template_ships(Enum, {lang:?}) = true; \
             per-backend Jinja2 template must ship in lockstep with this flag"
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
    // The inference layer declines to claim the type (Unknown);
    // literal-width narrowing resolves against the imported
    // EnumModel's underlying_type.
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

// ── Signed underlying carriers ───────────────────────────────────

/// C and C++ have always let an enumeration rest on a signed carrier,
/// and a negative sentinel beside a measurement range is the ordinary
/// way a signal catalogue spells "no reading" — DBC and AUTOSAR
/// catalogues are full of them. Restricting the Enum kind to unsigned
/// carriers forced those catalogues to be re-encoded (a -1 becoming
/// 0xFF) and pushed the sign convention out of the schema and into
/// every consumer's head.
///
/// The Kotlin backend makes the cost concrete: it has no unsigned
/// primitive in its ordinary type vocabulary, so an unsigned-only Enum
/// kind hands a Kotlin consumer `UByte` where the domain says `Byte`.
#[test]
fn positive_enum_signed_parses_int8_underlying() {
    let xml = fixture("enum_signed");
    let doc = parse_forge(&xml, label_for("enum_signed"))
        .expect("parse succeeds")
        .expect("not a statechart");
    let m = match doc {
        ForgeDocument::Enum(m) => m,
        other => panic!("expected Enum, got {:?}", other.kind()),
    };
    assert_eq!(m.underlying_type, SceType::Int8);
    assert_eq!(m.variants.len(), 4);
    // Both boundaries, so an off-by-one in either direction of the
    // range check fails rather than passing on a mid-range sample.
    assert_eq!(m.variants[0].name, "not_available");
    assert_eq!(m.variants[0].value, -128);
    assert_eq!(m.variants[1].value, -1);
    assert_eq!(m.variants[2].value, 0);
    assert_eq!(m.variants[3].value, 127);
}

#[test]
fn negative_enum_signed_positive_overflow_rejects() {
    // 128 fits uint8 and not int8: a check that kept the unsigned
    // ceiling while admitting the signed carrier would accept it.
    let err = expect_parse_error("negative_enum_signed_positive_overflow");
    assert!(
        matches!(
            err,
            ValidationError::EnumVariantValueOverflowsUnderlying {
                ref variant_name,
                value,
                ref underlying,
                ..
            } if variant_name == "too_big" && value == 128 && underlying == "int8"
        ),
        "got {err:?}"
    );
}

#[test]
fn negative_enum_negative_value_on_unsigned_underlying_rejects() {
    // The sign is a property of the declared carrier, not of the
    // literal: -1 against uint8 reports as the same overflow the
    // out-of-range positive case does.
    let err = expect_parse_error("negative_enum_negative_on_unsigned");
    assert!(
        matches!(
            err,
            ValidationError::EnumVariantValueOverflowsUnderlying {
                ref variant_name,
                value,
                ref underlying,
                ..
            } if variant_name == "bad" && value == -1 && underlying == "uint8"
        ),
        "got {err:?}"
    );
}

/// A signed carrier must reach every backend's emitted enum, not just
/// the IR. The six type maps already carry `int8`…`int64`, so what is
/// actually at risk is the value rendering: a negative variant has to
/// survive into the literal each language wants (Kotlin needs
/// `(-128).toByte()`, not `-128u.toUByte()`).
#[test]
fn signed_enum_reaches_every_backend_with_negative_values() {
    use sce_build::compile_forge_from_string;
    use sce_build::generator::Language;

    let xml = fixture("enum_signed");
    let label = label_for("temperature_status");
    for (lang, want) in [
        (Language::Cpp, "int8_t"),
        (Language::Rust, "i8"),
        (Language::Kotlin, "Byte"),
        (Language::Go, "int8"),
        (Language::C11, "int8_t"),
    ] {
        let out = compile_forge_from_string(&xml, label, lang)
            .unwrap_or_else(|e| panic!("{lang:?} codegen must succeed: {e:?}"));
        let code = out.files[0].1.clone();
        assert!(
            code.contains(want),
            "{lang:?} must carry the signed carrier {want}:\n{code}"
        );
        assert!(
            code.contains("-128"),
            "{lang:?} must render the negative boundary variant:\n{code}"
        );
    }

    // Python's IntEnum lowers to plain `int`, so the carrier survives
    // only as the documented `sce:underlying-type` name — which is the
    // one backend where a missing signed arm would show up as a panic
    // rather than a wrong type.
    let out = compile_forge_from_string(&xml, label, Language::Python)
        .expect("python codegen must succeed");
    let code = out.files[0].1.clone();
    assert!(code.contains("int8"), "python must document int8:\n{code}");
    assert!(code.contains("-128"), "python must render -128:\n{code}");
}

// ── strict-variants doc-as-contract: codegen ignores it ──────────

/// `forge::generator::render_enum` carries a comment asserting that
/// `EnumModel::strict_variants` is never consumed by codegen — the
/// strict-variants opt-out is a parse-time validator concern. This drift guard
/// mechanises that claim: two enum documents differing only in
/// `sce:strict-variants` (and sharing the same identifier so the
/// emitted symbol names stay byte-identical) must produce
/// byte-identical [`GeneratedOutput`] at every backend. Any
/// divergence signals codegen has started reading the opt-out
/// flag — at which point either the codegen-side read must revert
/// or design RFC §8 (strict variant membership) must expand to
/// cover the new surface.
#[test]
fn enum_codegen_ignores_strict_variants_at_every_backend() {
    use sce_build::compile_forge_from_string;
    use sce_build::generator::Language;

    const SHARED_LABEL: DocumentLabel<'static> = DocumentLabel {
        identifier: "result",
        diagnostic_label: "result.scxml",
    };

    fn enum_doc(strict_attr: Option<&str>) -> String {
        let extra = strict_attr
            .map(|v| format!(" sce:strict-variants=\"{v}\""))
            .unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       sce:kind="enum"
       name="result"
       sce:underlying-type="uint8"{extra}>
  <datamodel>
    <data id="variants">
      <sce:variant name="ok" value="0"/>
      <sce:variant name="error" value="1"/>
      <sce:variant name="timeout" value="2"/>
    </data>
  </datamodel>
</scxml>
"#
        )
    }

    // Three surface forms must all collapse to the same emitted
    // bytes: explicit strict, explicit open, and the default
    // (attribute absent). The default-absent case is the implicit
    // strict path most authors hit.
    let implicit = enum_doc(None);
    let explicit_strict = enum_doc(Some("true"));
    let explicit_open = enum_doc(Some("false"));

    for lang in [
        Language::Cpp,
        Language::Rust,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let a = compile_forge_from_string(&implicit, SHARED_LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?} compile (implicit): {e:?}"));
        let b = compile_forge_from_string(&explicit_strict, SHARED_LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?} compile (strict): {e:?}"));
        let c = compile_forge_from_string(&explicit_open, SHARED_LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?} compile (open): {e:?}"));

        // GeneratedOutput is not Eq/Debug — compare the load-bearing
        // `files: Vec<(filename, content)>` field directly. The
        // `deps` field is empty for `from_string` routes (no
        // preprocessor inputs).
        assert_eq!(
            a.files, b.files,
            "{lang:?}: implicit-default and explicit strict=true diverge — render_enum is reading strict_variants",
        );
        assert_eq!(
            b.files, c.files,
            "{lang:?}: explicit strict=true and strict=false diverge — render_enum is reading strict_variants",
        );
    }
}
