//! Worker kind schema + parse-time shared-state author guard.
//!
//! Per RFC §synth-5-D lines 858-913 + spec line 911 layered enforcement
//! (locked 2026-05-10): the schema vertical slice covers
//! `<sce:link-rx>` / `<sce:inbox>` / `<sce:outbox>` / `<sce:body>`, and
//! the parse-time author guard fires `worker/shared-mutable-state` on:
//!   1. `<sce:import kind="worker">` siblings (cross-worker imports
//!      forbidden — workers communicate only through inbox + outbox).
//!   2. Body SCXML data-refs into a foreign namespace (any prefix
//!      not in the allowlist `[<self-name>, _event, _data, _name,
//!      _iolocation, <outbox-target>]`).
//!
//! The inbox is a queue the worker imports (RFC §synth-5-P, Migration):
//! `<sce:inbox ref>` names the import, and the removed inline `depth` and
//! `ordering` fire `worker/inbox-inline-removed`.
//!
//! Cross-resolution (link-rx ref, inbox ref + single-consumer, outbox
//! ref) and deploy-aware scheduler-config validators live in
//! `c2_worker_outbox.rs`, `c2_worker_codegen.rs` and
//! `c2_worker_scheduler_codes.rs`.

use sce_build::forge::diagnostic::{DiagnosticCode, Fix, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located, ValidationError, WorkerSharedStateReason};
use sce_build::forge::model::{ForgeDocument, ForgeKind, InboxConfig, WorkerModel};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        // Test fixtures pin the file label to "<name>.scxml" so
        // diagnostic spec-anchor checks have a stable surface.
        diagnostic_label: ".scxml-fixture",
    }
}

fn parse(content: &str, name: &'static str) -> Result<WorkerModel, Located<ForgeError>> {
    match parse_forge(content, label(name))? {
        Some(ForgeDocument::Worker(w)) => Ok(w),
        Some(other) => panic!("expected ForgeDocument::Worker, got {:?}", other.kind()),
        None => panic!("statechart routed through forge entry — fixture mis-tagged?"),
    }
}

/// Happy path: minimal worker with only the required schema slots
/// (link-rx + inbox).
#[test]
fn worker_minimal_schema_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let w = parse(xml, "rx_loop").expect("minimal worker parses");
    assert_eq!(w.name, "rx_loop");
    assert_eq!(w.link_rx, "udp_scout");
    assert_eq!(w.inbox.queue_ref, "rx_events");
    assert_eq!(w.outbox, None);
}

/// Full schema vertical slice: link-rx + inbox + outbox + empty body.
#[test]
fn worker_full_schema_parses() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:outbox ref="session_fsm.inbox"/>
  <sce:body/>
</scxml>"##;
    let w = parse(xml, "rx_loop").expect("full worker parses");
    assert_eq!(w.name, "rx_loop");
    assert_eq!(w.link_rx, "udp_scout");
    assert_eq!(w.inbox.queue_ref, "rx_events");
    assert_eq!(w.outbox.as_deref(), Some("session_fsm.inbox"));
}

/// Negative: missing `<sce:link-rx>` — required element.
#[test]
fn worker_missing_link_rx_rejected() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("missing link-rx must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingElement {
                kind: ForgeKind::Worker,
                element,
            } => assert_eq!(element, "sce:link-rx"),
            other => panic!("expected MissingElement(sce:link-rx), got {other:?}"),
        },
        other => panic!("expected MissingElement(sce:link-rx), got {other:?}"),
    }
}

/// Negative: missing `<sce:inbox>` — required element.
#[test]
fn worker_missing_inbox_rejected() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("missing inbox must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingElement {
                kind: ForgeKind::Worker,
                element,
            } => assert_eq!(element, "sce:inbox"),
            other => panic!("expected MissingElement(sce:inbox), got {other:?}"),
        },
        other => panic!("expected MissingElement(sce:inbox), got {other:?}"),
    }
}

/// Negative: `<sce:inbox>` that names no queue and carries no removed
/// attribute — the `ref` is the required attribute.
#[test]
fn worker_inbox_without_ref_rejected() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("an inbox with no ref must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingAttribute { element, attr } => {
                assert_eq!(element, "<sce:inbox>");
                assert_eq!(attr, "ref");
            }
            other => panic!("expected MissingAttribute(ref), got {other:?}"),
        },
        other => panic!("expected MissingAttribute(ref), got {other:?}"),
    }
}

/// Negative: an empty `ref` names nothing.
#[test]
fn worker_inbox_empty_ref_rejected() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref=""/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("an empty ref must reject");
    match &err.error {
        // Layer 1 (XSD): `xs:NCName` refuses the empty string.
        ForgeError::Xml(_) => {}
        // Layer 2 (parser): the non-empty guard inside `parse_worker`.
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::AttributeRuleViolated { element, attr, .. } => {
                assert_eq!(element, "<sce:inbox>");
                assert_eq!(attr, "ref");
            }
            other => panic!("expected an empty-ref rejection, got {other:?}"),
        },
        other => panic!("expected an empty-ref rejection, got {other:?}"),
    }
}

/// The removed inline form: `depth` and `ordering` fire
/// `worker/inbox-inline-removed`, whose message names the queue document that
/// holds exactly what the ring held (`capacity = depth - 1`, one producer, one
/// consumer, `wait-free`).
#[test]
fn worker_inline_depth_and_ordering_are_removed() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("the inline form is removed");
    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerInboxInlineRemoved {
                worker_name,
                removed,
                replacement,
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(removed, "depth=\"16\" ordering=\"acq_rel\"");
                assert!(
                    replacement.contains("capacity=\"15\"")
                        && replacement.contains("producers `one`")
                        && replacement.contains("consumers `one`")
                        && replacement.contains("`wait-free`"),
                    "the replacement names the queue that holds what the ring held: {replacement}"
                );
            }
            other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
        },
        other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
    }
    let diags = err.to_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::WorkerInboxInlineRemoved
    ));
    // The repair of the element is the removal of exactly those attributes.
    match &diags[0].fix {
        Some(Fix::RemoveFields { fields, .. }) => {
            assert_eq!(fields, &["depth".to_string(), "ordering".to_string()]);
        }
        other => panic!("expected a remove_fields fix, got {other:?}"),
    }
    assert_eq!(
        diags[0].code.spec_anchor(),
        Some("SCE Protocol-Synthesis RFC §5.P")
    );
}

/// Either removed attribute alone is the same refusal, and it is named first:
/// a document that carries the old attributes alone is told what to write
/// instead, not that the `ref` is missing.
#[test]
fn worker_inline_depth_alone_is_removed_before_the_missing_ref() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox depth="0"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("depth is removed");
    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerInboxInlineRemoved {
                removed,
                replacement,
                ..
            } => {
                assert_eq!(removed, "depth=\"0\"");
                // A depth that leaves no capacity names the formula.
                assert!(replacement.contains("depth - 1"), "{replacement}");
            }
            other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
        },
        other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
    }

    let ordering_only = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ordering="relaxed"/>
</scxml>"##;
    let err = parse(ordering_only, "rx_loop").expect_err("ordering is removed");
    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerInboxInlineRemoved { removed, .. } => {
                assert_eq!(removed, "ordering=\"relaxed\"");
            }
            other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
        },
        other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
    }
}

/// Layer 1 guard: sibling `<sce:import kind="worker">` rejects with
/// `worker/shared-mutable-state` carrying the
/// `WorkerImportForbidden` reason.
#[test]
fn worker_import_kind_worker_fires_layer1_guard() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="tx_loop" src="tx_loop.scxml" kind="worker"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("worker-kind import must reject");
    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerSharedMutableState {
                worker_name,
                reason,
            } => {
                assert_eq!(worker_name, "rx_loop");
                match reason {
                    WorkerSharedStateReason::WorkerImportForbidden {
                        imported_alias,
                        imported_src,
                    } => {
                        assert_eq!(imported_alias, "tx_loop");
                        assert_eq!(imported_src, "tx_loop.scxml");
                    }
                    other => panic!("expected WorkerImportForbidden, got {other:?}"),
                }
            }
            other => panic!("expected WorkerSharedMutableState, got {other:?}"),
        },
        other => panic!("expected WorkerSharedMutableState, got {other:?}"),
    }
    // Diagnostic must surface as `worker/shared-mutable-state` per
    // spec line 911.
    let diags = err.to_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::WorkerSharedMutableState
    ));
}

/// Layer 1 guard with partial-attr import. XSD enforces `as` / `src`
/// required on `<sce:import>`, so an import missing those attributes
/// is rejected at the schema layer before the Rust parser sees it.
/// Both rejections are honored: schema-level (XSD) or parser-level
/// (`WorkerImportForbidden`). The test confirms a `kind="worker"`
/// import surfaces a rejection through *either* layer when bundled
/// in the same worker document.
#[test]
fn worker_import_kind_worker_minimal_attrs_fires_layer1() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import kind="worker"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("alias-less worker import must reject");
    match &err.error {
        // XSD-layer rejection (missing required attrs on <sce:import>).
        ForgeError::Xml(_) => {}
        // Parser-layer rejection (Rust guard runs ahead of import-spec
        // parser, surfaces WorkerSharedMutableState with empty attrs).
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerSharedMutableState {
                reason:
                    WorkerSharedStateReason::WorkerImportForbidden {
                        imported_alias,
                        imported_src,
                    },
                ..
            } => {
                assert_eq!(imported_alias, "");
                assert_eq!(imported_src, "");
            }
            other => panic!(
                "expected XSD missing-attr rejection or parser WorkerImportForbidden, got {other:?}"
            ),
        },
        other => panic!(
            "expected XSD missing-attr rejection or parser WorkerImportForbidden, got {other:?}"
        ),
    }
}

/// Negative-of-negative: `<sce:import kind="codec">` etc. do NOT fire
/// the layer 1 guard (only worker-kind imports are forbidden inside a
/// worker; other kinds are legitimate cross-file references). The
/// import is unsupported by the worker parser's own logic but doesn't
/// fire the shared-state code.
#[test]
fn worker_import_non_worker_kind_does_not_fire_layer1() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_codec" src="udp.scxml" kind="codec"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let w = parse(xml, "rx_loop").expect("non-worker import is fine");
    assert_eq!(w.name, "rx_loop");
}

/// Layer 2 guard: body `<assign location="foreign.field">` reaches a
/// namespace outside the allowlist — fires `WorkerSharedMutableState`
/// with `BodyForeignNamespace` reason.
#[test]
fn worker_body_assign_to_foreign_namespace_fires_layer2() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:body>
    <assign location="other_worker.counter" expr="0"/>
  </sce:body>
</scxml>"##;
    let err = parse(xml, "rx_loop").expect_err("foreign-namespace assign must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerSharedMutableState {
                reason:
                    WorkerSharedStateReason::BodyForeignNamespace {
                        element,
                        attr,
                        value,
                        foreign_prefix,
                    },
                ..
            } => {
                assert_eq!(element, "assign");
                assert_eq!(attr, "location");
                assert_eq!(value, "other_worker.counter");
                assert_eq!(foreign_prefix, "other_worker");
            }
            other => panic!("expected BodyForeignNamespace, got {other:?}"),
        },
        other => panic!("expected BodyForeignNamespace, got {other:?}"),
    }
}

/// Layer 2 allowlist: `_event.data` is the canonical SCXML event-data
/// access; must NOT fire the guard.
#[test]
fn worker_body_event_data_refs_pass_layer2() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:body>
    <assign location="_event.data" expr="42"/>
  </sce:body>
</scxml>"##;
    parse(xml, "rx_loop").expect("_event.* is in the allowlist");
}

/// Layer 2 allowlist: self-name prefix passes (worker's own state can
/// be read/written by its body).
#[test]
fn worker_body_self_namespace_passes_layer2() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:body>
    <assign location="rx_loop.counter" expr="0"/>
  </sce:body>
</scxml>"##;
    parse(xml, "rx_loop").expect("self-namespace is in the allowlist");
}

/// Layer 2 allowlist: when an `<sce:outbox ref="X.inbox">` is declared,
/// the outbox-target prefix `X` joins the allowlist — sends to that
/// recipient's inbox are the *one* legitimate cross-namespace path.
#[test]
fn worker_outbox_target_prefix_passes_layer2() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:outbox ref="session_fsm.inbox"/>
  <sce:body>
    <send target="session_fsm.inbox" event="rx.tick"/>
  </sce:body>
</scxml>"##;
    let w = parse(xml, "rx_loop").expect("outbox target prefix is allowlisted");
    assert_eq!(w.outbox.as_deref(), Some("session_fsm.inbox"));
}

/// Layer 2 numeric-literal guard: `<param expr="3.14"/>` carries a dot
/// but the prefix isn't a valid NCName (starts with a digit) — must
/// NOT fire the guard.
#[test]
fn worker_body_numeric_literal_does_not_fire_layer2() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
  <sce:body>
    <assign location="rx_loop.x" expr="3.14"/>
  </sce:body>
</scxml>"##;
    parse(xml, "rx_loop").expect("numeric literals are not namespace prefixes");
}

/// Diagnostic surface check: layer 1 + layer 2 both surface as
/// `worker/shared-mutable-state` (single code, multi-layer per spec
/// line 911 unification).
#[test]
fn worker_shared_state_layers_share_diagnostic_code() {
    // Layer 1.
    let xml1 = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="w1" version="1.0">
  <sce:import as="w2" src="w2.scxml" kind="worker"/>
  <sce:link-rx ref="l"/>
  <sce:inbox ref="q"/>
</scxml>"##;
    let err1 = parse(xml1, "w1").expect_err("layer 1");
    assert!(matches!(
        err1.to_diagnostics()[0].code,
        DiagnosticCode::WorkerSharedMutableState
    ));

    // Layer 2.
    let xml2 = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="w1" version="1.0">
  <sce:link-rx ref="l"/>
  <sce:inbox ref="q"/>
  <sce:body><assign location="foo.bar" expr="0"/></sce:body>
</scxml>"##;
    let err2 = parse(xml2, "w1").expect_err("layer 2");
    assert!(matches!(
        err2.to_diagnostics()[0].code,
        DiagnosticCode::WorkerSharedMutableState
    ));
}

/// Spec anchor: `worker/shared-mutable-state` resolves to
/// "SCE Protocol-Synthesis RFC §synth-5-D" per the worker spec-anchor mapping.
#[test]
fn worker_shared_state_spec_anchor_matches_rfc_section() {
    let anchor = DiagnosticCode::WorkerSharedMutableState.spec_anchor();
    assert_eq!(anchor, Some("SCE Protocol-Synthesis RFC §5.D"));
}

/// Schema-level confirmation: a parsed Worker carries the right
/// `WorkerModel` fields and the inbox stays a fresh struct (helps
/// catch accidental shadowing of `InboxConfig` if a future atomic
/// renames it).
#[test]
fn worker_model_field_shape_is_stable() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="rx_events"/>
</scxml>"##;
    let w = parse(xml, "rx_loop").unwrap();
    // Walk through the exhaustive field set to surface any drop or
    // rename in a future refactor.
    let WorkerModel {
        name,
        link_rx,
        inbox: InboxConfig { queue_ref },
        outbox,
        source_location: _,
    } = w;
    assert_eq!(name, "rx_loop");
    assert_eq!(link_rx, "udp_scout");
    assert_eq!(queue_ref, "rx_events");
    assert_eq!(outbox, None);
}
