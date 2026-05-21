//! C9-α — Fragment / reassembly buffer-pool variant schema + parse +
//! 2 parse-time structure validators.
//!
//! Per watching-zenoh RFC §5.M lines 2676-2698 + 2944-2945: the
//! `<sce:variant>reassembly</sce:variant>` discriminator on a
//! `<scxml sce:kind="buffer-pool">` document opens three required
//! sibling elements:
//!   - `<sce:max-fragments-per-message>` (spec line 2688) — fragment-
//!     index bitmap width per slot
//!   - `<sce:reassembly-timeout-ms>` (spec line 2689) — per-slot
//!     deadline field
//!   - `<sce:per-peer-quota>` (spec line 2690, 2841-2861) — peer-id-
//!     scoped slot allocation cap
//!
//! Two spec-named codes fire on the first two siblings' absence
//! (`mem/reassembly-pool-variant-missing-max-fragments` +
//! `mem/reassembly-pool-variant-missing-timeout` per spec line
//! 2944-2945); the third (`per-peer-quota`) reuses the generic
//! `ValidationError::MissingElement` per `[[feedback-no-versioning]]`
//! since spec only names the first two codes explicitly.
//!
//! Cross-arm exclusivity: under `variant=default` (absent
//! `<sce:variant>` or explicit `default` body text) the three
//! reassembly-only siblings are **forbidden** — their presence raises
//! `ValidationError::InvalidAttribute` naming the misapplied element
//! (the type-system mirror of the sum-type's "only-on-arm" invariant
//! per Q-C9-1 (a) lock).
//!
//! Cross-doc validators referencing §5.K
//! `links.<name>.{mtu_bytes, expected_p99_bytes, domain_attrs.trust_class}`
//! (6-8 codes including `reassembly/max-fragments-insufficient-for-mtu` +
//! `reassembly/untrusted-link-binding`) defer to **C9-β** co-landing
//! with C13. Codegen-side per-slot bitmap/deadline/peer-id emission +
//! `reassembly/peer-id-not-zid-on-established-session` template-
//! regression guard defer to **C9-γ**. Listener-link sibling-split
//! codes (`link/listener-link-not-paired-with-established-sibling` +
//! `reassembly/binding-on-unpaired-listener`) belong to **C10/C11**
//! per spec line 2820-2824 (§5.C codegen contract). The Q-C9-1..6
//! locks are documented in
//! `claudedocs/rfc-c9-fragment-reassembly-kind.md`.

use sce_build::forge::diagnostic::DiagnosticCode;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::forge::model::{BufferPoolModel, BufferPoolVariant, CachePolicy, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

fn parse(content: &str, name: &'static str) -> Result<BufferPoolModel, Located<ForgeError>> {
    match parse_forge(content, label(name))? {
        Some(ForgeDocument::BufferPool(p)) => Ok(p),
        Some(other) => panic!("expected ForgeDocument::BufferPool, got {:?}", other.kind()),
        None => panic!("statechart routed through forge entry — fixture mis-tagged?"),
    }
}

/// Happy path: pre-C9 baseline shape — no `<sce:variant>` element. The
/// parser maps this to `BufferPoolVariant::Default`, preserving the
/// existing B7-α / C5 buffer-pool semantics byte-for-byte. C9-α is
/// purely additive at this arm.
#[test]
fn buffer_pool_no_variant_element_parses_as_default() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
    let p = parse(xml, "rx_pool_sram1").expect("absent <sce:variant> parses as Default");
    assert!(matches!(p.variant, BufferPoolVariant::Default));
}

/// Happy path: explicit `<sce:variant>default</sce:variant>` body text
/// is canonicalized to the same `Default` arm (semantically a no-op
/// vs. omitting the element). The parser-level closed-enum check
/// accepts `default` alongside `reassembly`.
#[test]
fn buffer_pool_explicit_default_variant_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:variant>default</sce:variant>
</scxml>"##;
    let p = parse(xml, "rx_pool_sram1").expect("<sce:variant>default</sce:variant> parses");
    assert!(matches!(p.variant, BufferPoolVariant::Default));
}

/// Happy path: full reassembly-variant schema — `<sce:variant>reassembly` +
/// all three required siblings present and positive. Verifies the
/// sum-type `Reassembly(ReassemblyConfig { ... })` arm carries the
/// three field values verbatim from the parsed XML body text.
///
/// Verbatim spec example values come from RFC §5.M lines 2683-2691.
#[test]
fn buffer_pool_reassembly_variant_full_schema_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>16</sce:max-fragments-per-message>
  <sce:reassembly-timeout-ms>500</sce:reassembly-timeout-ms>
  <sce:per-peer-quota>2</sce:per-peer-quota>
</scxml>"##;
    let p = parse(xml, "rx_reassembly_pool").expect("reassembly-variant parses");
    assert_eq!(p.name, "rx_reassembly_pool");
    assert_eq!(p.slot_count, 4);
    assert_eq!(p.slot_size, 4096);
    assert!(matches!(p.cache_policy, CachePolicy::Maintain));
    match &p.variant {
        BufferPoolVariant::Reassembly(cfg) => {
            assert_eq!(cfg.max_fragments_per_message, 16);
            assert_eq!(cfg.reassembly_timeout_ms, 500);
            assert_eq!(cfg.per_peer_quota, 2);
        }
        other => panic!("expected Reassembly arm, got {other:?}"),
    }
}

/// Spec-named code #1: RFC §5.M line 2944 —
/// `<sce:variant>reassembly</sce:variant>` declared without
/// `<sce:max-fragments-per-message>` raises
/// `mem/reassembly-pool-variant-missing-max-fragments`.
#[test]
fn reassembly_variant_missing_max_fragments_fires_spec_code() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:reassembly-timeout-ms>500</sce:reassembly-timeout-ms>
  <sce:per-peer-quota>2</sce:per-peer-quota>
</scxml>"##;
    let err = parse(xml, "rx_reassembly_pool")
        .expect_err("missing <sce:max-fragments-per-message> raises spec-named code");
    let ForgeError::Validation(ValidationError::MemReassemblyPoolVariantMissingMaxFragments {
        pool_name,
    }) = err.error
    else {
        panic!(
            "expected MemReassemblyPoolVariantMissingMaxFragments, got {:?}",
            err.error
        );
    };
    assert_eq!(pool_name, "rx_reassembly_pool");
}

/// Spec-named code #2: RFC §5.M line 2945 —
/// `<sce:variant>reassembly</sce:variant>` declared without
/// `<sce:reassembly-timeout-ms>` raises
/// `mem/reassembly-pool-variant-missing-timeout`.
#[test]
fn reassembly_variant_missing_timeout_fires_spec_code() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>16</sce:max-fragments-per-message>
  <sce:per-peer-quota>2</sce:per-peer-quota>
</scxml>"##;
    let err = parse(xml, "rx_reassembly_pool")
        .expect_err("missing <sce:reassembly-timeout-ms> raises spec-named code");
    let ForgeError::Validation(ValidationError::MemReassemblyPoolVariantMissingTimeout {
        pool_name,
    }) = err.error
    else {
        panic!(
            "expected MemReassemblyPoolVariantMissingTimeout, got {:?}",
            err.error
        );
    };
    assert_eq!(pool_name, "rx_reassembly_pool");
}

/// Generic MissingElement: `<sce:per-peer-quota>` absence under
/// `variant=reassembly` reuses the generic
/// `ValidationError::MissingElement` per `[[feedback-no-versioning]]`
/// — spec line 2944-2945 names only the first two reassembly-specific
/// codes. C9-α holds the surface tight.
#[test]
fn reassembly_variant_missing_per_peer_quota_uses_generic_code() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>16</sce:max-fragments-per-message>
  <sce:reassembly-timeout-ms>500</sce:reassembly-timeout-ms>
</scxml>"##;
    let err = parse(xml, "rx_reassembly_pool")
        .expect_err("missing <sce:per-peer-quota> raises generic MissingElement");
    let ForgeError::Validation(ValidationError::MissingElement { element, .. }) = err.error else {
        panic!("expected MissingElement, got {:?}", err.error);
    };
    assert!(
        element.contains("per-peer-quota"),
        "diagnostic must name per-peer-quota element, got {element:?}"
    );
}

/// Cross-arm exclusivity: reassembly-only sibling
/// `<sce:max-fragments-per-message>` present without
/// `<sce:variant>reassembly</sce:variant>` (default arm) raises
/// `InvalidAttribute` — the parse-time mirror of the sum-type's
/// "only-on-arm" invariant.
#[test]
fn default_variant_with_reassembly_sibling_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>16</sce:max-fragments-per-message>
</scxml>"##;
    let err = parse(xml, "rx_pool_sram1")
        .expect_err("reassembly sibling without <sce:variant>reassembly</sce:variant> rejects");
    let ForgeError::Validation(ValidationError::InvalidAttribute { element, .. }) = err.error
    else {
        panic!(
            "expected InvalidAttribute for misapplied sibling, got {:?}",
            err.error
        );
    };
    assert!(
        element.contains("max-fragments-per-message"),
        "diagnostic must name the misapplied element, got {element:?}"
    );
}

/// Closed-enum gate: `<sce:variant>unknown</sce:variant>` body text
/// outside the `{default, reassembly}` set rejects via
/// `InvalidAttribute`. Spec §5.M line 2682 fixes the only currently-
/// defined variant value; future spec extensions (per §5.E "FSM
/// extension policy") will add new values additively.
#[test]
fn variant_unknown_body_text_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:variant>tx_only</sce:variant>
</scxml>"##;
    let err = parse(xml, "rx_pool_sram1").expect_err("unknown <sce:variant> body text rejects");
    let ForgeError::Validation(ValidationError::InvalidAttribute {
        element,
        value,
        expected,
        ..
    }) = err.error
    else {
        panic!("expected InvalidAttribute, got {:?}", err.error);
    };
    assert_eq!(element, "<sce:variant>");
    assert_eq!(value, "tx_only");
    assert!(
        expected.contains("default") && expected.contains("reassembly"),
        "expected hint must enumerate the closed set, got {expected:?}"
    );
}

/// Zero-value rejection: `<sce:max-fragments-per-message>0` (boundary)
/// is rejected at the XSD layer (`xs:positiveInteger`), surfacing as a
/// `XmlSchemaValidation` ForgeError before reaching `parse_buffer_pool`.
/// The parser-level `reject_zero_field` helper is defense-in-depth — it
/// would fire if the XSD relaxed the positiveInteger restriction. This
/// test pins the actual XSD-level path; the parser-level helper is
/// covered by direct call coverage in the unit-test layer.
#[test]
fn reassembly_max_fragments_zero_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>4096</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>0</sce:max-fragments-per-message>
  <sce:reassembly-timeout-ms>500</sce:reassembly-timeout-ms>
  <sce:per-peer-quota>2</sce:per-peer-quota>
</scxml>"##;
    let err = parse(xml, "rx_reassembly_pool").expect_err("zero max-fragments-per-message rejects");
    // XSD's xs:positiveInteger refuses `0` before the parser runs. The
    // surface is XmlSchemaValidation rather than ValidationError —
    // matches the existing slot-count/slot-size/alignment zero
    // rejection path (those fields also use xs:positiveInteger).
    let err_str = format!("{:?}", err.error);
    assert!(
        err_str.contains("max-fragments-per-message")
            && (err_str.contains("positiveInteger") || err_str.contains("SchemaValidation")),
        "expected XSD positiveInteger rejection on <sce:max-fragments-per-message>0, got {err_str:?}"
    );
}

/// Closed-enum drift guard — every C9-α spec-named code's
/// `#[serde(rename = "...")]` renders exactly the spec-line-2944-2945
/// slash-path string. `serde_json::to_string` exercises the wire-side
/// surface that downstream consumers read (vs. the crate-private
/// `as_str` form used inside the diagnostic pipeline). Mirrors C6-α
/// `c6_bounded_collection.rs::closed_enum_drift_guard` precedent.
#[test]
fn c9_alpha_codes_serialize_as_spec_paths() {
    let rendered =
        serde_json::to_string(&DiagnosticCode::MemReassemblyPoolVariantMissingMaxFragments)
            .expect("serde-serialize");
    assert_eq!(
        rendered,
        "\"mem/reassembly-pool-variant-missing-max-fragments\""
    );
    let rendered = serde_json::to_string(&DiagnosticCode::MemReassemblyPoolVariantMissingTimeout)
        .expect("serde-serialize");
    assert_eq!(rendered, "\"mem/reassembly-pool-variant-missing-timeout\"");
}
