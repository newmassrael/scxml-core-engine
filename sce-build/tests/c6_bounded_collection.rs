//! C6-α — Bounded-collection kind schema + parse + 2 parse-time
//! structure validators.
//!
//! Per watching-zenoh RFC §5.L lines 2540-2655: the schema vertical
//! slice covers `<sce:element-type>` + `<sce:capacity>` (deploy-key OR
//! compile-const) + `<sce:index-by>` + `<sce:on-overflow>` +
//! `<sce:ordering>` + `<sce:concurrency>`, and the two parse-time
//! structure validators fire:
//!   1. `collection/ordering-sorted-requires-index-by` (spec line 2559)
//!      — `<sce:ordering>sorted-by(index-by)</sce:ordering>` declared
//!      without an accompanying `<sce:index-by field="..."/>`.
//!   2. `collection/overflow-policy-oldest-wins-requires-ordering-
//!      insertion` (spec line 2655) — `oldest-wins` overflow paired
//!      with `sorted-by(index-by)` ordering (the explicit anti-pattern).
//!
//! Cross-doc element-type kind resolution (`collection/element-type-not-
//! a-kind`), `<sce:index-by>` field verification
//! (`collection/index-by-field-missing`), `<sce:concurrency>multi-writer`
//! atomics-import check (`collection/multi-writer-without-atomics`), and
//! deploy-time capacity resolution (`collection/capacity-unresolved`)
//! all defer to C6-β/γ when the consumer wiring is in place — per the
//! `feedback_silently_broken_hooks` discipline.

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::forge::model::{
    BoundedCollectionModel, CapacitySource, CollectionOrdering, ConcurrencyMode, ForgeDocument,
    OverflowPolicy,
};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

fn parse(content: &str, name: &'static str) -> Result<BoundedCollectionModel, Located<ForgeError>> {
    match parse_forge(content, label(name))? {
        Some(ForgeDocument::BoundedCollection(c)) => Ok(c),
        Some(other) => panic!(
            "expected ForgeDocument::BoundedCollection, got {:?}",
            other.kind()
        ),
        None => panic!("statechart routed through forge entry — fixture mis-tagged?"),
    }
}

/// Happy path: minimal required schema — element-type + compile-const
/// capacity. All optional elements take their spec-line-2556/2558/2560
/// defaults (`diagnostic-event` / `insertion` / `single-writer`).
#[test]
fn bounded_collection_minimal_schema_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity const="8"/>
</scxml>"##;
    let c = parse(xml, "local_sub_table").expect("minimal bounded-collection parses");
    assert_eq!(c.name, "local_sub_table");
    assert_eq!(c.element_type, "SubscriptionEntry");
    assert!(matches!(
        c.capacity,
        CapacitySource::CompileConst { value: 8 }
    ));
    assert_eq!(c.index_by, None);
    assert!(matches!(c.on_overflow, OverflowPolicy::DiagnosticEvent));
    assert!(matches!(c.ordering, CollectionOrdering::Insertion));
    assert!(matches!(c.concurrency, ConcurrencyMode::SingleWriter));
}

/// Full schema vertical slice — every spec-line-2551..2562 element
/// present. Deploy-key capacity, sorted ordering with index-by,
/// `reject` overflow policy, multi-writer concurrency.
#[test]
fn bounded_collection_full_schema_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity source="deploy" key="machines.mcu_node.limits.local_subscriptions"/>
  <sce:index-by field="key_expr_id"/>
  <sce:on-overflow>reject</sce:on-overflow>
  <sce:ordering>sorted-by(index-by)</sce:ordering>
  <sce:concurrency>multi-writer</sce:concurrency>
</scxml>"##;
    let c = parse(xml, "local_sub_table").expect("full bounded-collection parses");
    assert_eq!(c.element_type, "SubscriptionEntry");
    assert!(
        matches!(&c.capacity, CapacitySource::DeployKey { key } if key == "machines.mcu_node.limits.local_subscriptions")
    );
    assert_eq!(c.index_by.as_deref(), Some("key_expr_id"));
    assert!(matches!(c.on_overflow, OverflowPolicy::Reject));
    assert!(matches!(c.ordering, CollectionOrdering::SortedByIndex));
    assert!(matches!(c.concurrency, ConcurrencyMode::MultiWriter));
}

/// `oldest-wins` overflow with `insertion` ordering — legal per spec.
#[test]
fn bounded_collection_oldest_wins_with_insertion_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="ring_buffer" version="1.0">
  <sce:element-type>FrameRecord</sce:element-type>
  <sce:capacity const="32"/>
  <sce:on-overflow>oldest-wins</sce:on-overflow>
  <sce:ordering>insertion</sce:ordering>
</scxml>"##;
    let c = parse(xml, "ring_buffer").expect("oldest-wins+insertion parses");
    assert!(matches!(c.on_overflow, OverflowPolicy::OldestWins));
    assert!(matches!(c.ordering, CollectionOrdering::Insertion));
}

/// Negative: missing `<sce:element-type>` — required element.
#[test]
fn missing_element_type_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:capacity const="8"/>
</scxml>"##;
    let err = parse(xml, "local_sub_table").expect_err("missing element-type rejects");
    let ForgeError::Validation(ValidationError::MissingElement { element, .. }) = err.error else {
        panic!("expected MissingElement, got {:?}", err.error);
    };
    assert!(element.contains("element-type"));
}

/// Negative: missing `<sce:capacity>` — required element.
#[test]
fn missing_capacity_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
</scxml>"##;
    let err = parse(xml, "local_sub_table").expect_err("missing capacity rejects");
    let ForgeError::Validation(ValidationError::MissingElement { element, .. }) = err.error else {
        panic!("expected MissingElement, got {:?}", err.error);
    };
    assert!(element.contains("capacity"));
}

/// Negative: `<sce:capacity const="0"/>` — zero is parse-rejected
/// because a zero-capacity collection cannot store any element.
#[test]
fn capacity_zero_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity const="0"/>
</scxml>"##;
    let err = parse(xml, "local_sub_table").expect_err("zero capacity rejects");
    let ForgeError::Validation(ValidationError::InvalidAttribute { .. }) = err.error else {
        panic!(
            "expected InvalidAttribute for capacity=0, got {:?}",
            err.error
        );
    };
}

/// Negative: `<sce:capacity/>` with neither `source/key` nor `const` —
/// exactly one attribute form is required.
#[test]
fn capacity_no_source_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity/>
</scxml>"##;
    let err = parse(xml, "local_sub_table").expect_err("empty capacity rejects");
    let ForgeError::Validation(ValidationError::InvalidAttribute { .. }) = err.error else {
        panic!(
            "expected InvalidAttribute for empty capacity, got {:?}",
            err.error
        );
    };
}

/// Spec-named diagnostic #1 — `collection/ordering-sorted-requires-
/// index-by` (RFC §5.L line 2559): `<sce:ordering>sorted-by(index-by)
/// </sce:ordering>` declared without an `<sce:index-by>` element.
#[test]
fn ordering_sorted_without_index_by_fires_spec_code() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity const="8"/>
  <sce:ordering>sorted-by(index-by)</sce:ordering>
</scxml>"##;
    let err =
        parse(xml, "local_sub_table").expect_err("sorted-by without index-by fires structure code");
    let ForgeError::Validation(ValidationError::CollectionOrderingSortedRequiresIndexBy {
        collection_name,
    }) = &err.error
    else {
        panic!(
            "expected CollectionOrderingSortedRequiresIndexBy, got {:?}",
            err.error
        );
    };
    assert_eq!(collection_name, "local_sub_table");

    // Diagnostic wire code lands at the canonical spec-named slug.
    let diags = err.error.to_diagnostics();
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::CollectionOrderingSortedRequiresIndexBy
    ));
}

/// Spec-named diagnostic #2 — `collection/overflow-policy-oldest-wins-
/// requires-ordering-insertion` (RFC §5.L line 2655): `oldest-wins`
/// overflow paired with `sorted-by(index-by)` ordering — the explicit
/// anti-pattern (no temporal "oldest" defined when iteration order is
/// comparator-derived).
#[test]
fn oldest_wins_with_sorted_by_fires_spec_code() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity const="8"/>
  <sce:index-by field="key_expr_id"/>
  <sce:on-overflow>oldest-wins</sce:on-overflow>
  <sce:ordering>sorted-by(index-by)</sce:ordering>
</scxml>"##;
    let err =
        parse(xml, "local_sub_table").expect_err("oldest-wins+sorted-by fires structure code");
    let ForgeError::Validation(
        ValidationError::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
            collection_name,
        },
    ) = &err.error
    else {
        panic!(
            "expected CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion, got {:?}",
            err.error
        );
    };
    assert_eq!(collection_name, "local_sub_table");

    let diags = err.error.to_diagnostics();
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion
    ));
}

/// Negative: `<sce:on-overflow>bogus</sce:on-overflow>` — unknown
/// policy value parse-rejects with `InvalidAttribute`.
#[test]
fn unknown_overflow_policy_rejects() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity const="8"/>
  <sce:on-overflow>bogus</sce:on-overflow>
</scxml>"##;
    let err = parse(xml, "local_sub_table").expect_err("unknown overflow rejects");
    let ForgeError::Validation(ValidationError::InvalidAttribute { value, .. }) = err.error else {
        panic!("expected InvalidAttribute, got {:?}", err.error);
    };
    assert_eq!(value, "bogus");
}

/// Closed-enum acceptance check — bounded-collection appears in the
/// `validation/unsupported-kind` candidate list when an unknown kind
/// is authored. Drift guard: D-1 invariant for ForgeKind::ALL_ATTR_NAMES.
#[test]
fn bounded_collection_in_unsupported_kind_candidates() {
    use sce_build::forge::model::ForgeKind;
    assert!(ForgeKind::ALL_ATTR_NAMES.contains(&"bounded-collection"));
    assert_eq!(
        ForgeKind::from_attr("bounded-collection"),
        Some(ForgeKind::BoundedCollection)
    );
    assert!(ForgeKind::BoundedCollection.is_supported());
    // Display surface for diagnostic messages.
    assert_eq!(
        ForgeKind::BoundedCollection.to_string(),
        "bounded-collection"
    );
}
