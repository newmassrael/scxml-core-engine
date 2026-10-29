//! Broadcast-log kind: parse, the selection table, the refusals the document
//! alone decides, the cross-document element type, the stamp obligations of a
//! narrow target, the catalog entry, and the refusal every language gives while
//! none has the runtime row.
//!
//! Per ADR 0062: a `sce:kind="broadcast-log"` document states a contract
//! (element type, producer cardinality, the progress it relies on, the
//! retention) and names no algorithm. The kind is parsed and checked on every
//! backend before any backend lowers it, so generating a valid document for any
//! language is refused by name (`broadcast-log/runtime-missing`), and never as
//! a template the generator forgot (`codegen/generic-kind-backend-emit-missing`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::codegen_matrix::{
    awaits_runtime, broadcast_log_progress_ceiling, check, emitting, language_wire_name, lookup,
    template_ships, EmitOutcome, KindClass, BROADCAST_LOG_BACKENDS,
};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::forge::generator::judge_broadcast_log_stamp;
use sce_build::forge::kind_catalog::{ExampleRecord, KindCatalog};
use sce_build::forge::model::{
    broadcast_log_longest_pause_ops, broadcast_log_slots_divide_the_stamp_space,
    broadcast_log_wrap_bound_ops, BroadcastLogModel, CapacitySource, ForgeDocument, ForgeKind,
    QueueCardinality, QueueProgress, RuntimeDep, BROADCAST_LOG_NARROW_STAMP_BITS,
};
use sce_build::forge::parser::parse_forge;
use sce_build::generator::Language;
use sce_build::mesh::deploy::{parse_deploy_str, DeployConfig};
use sce_build::{
    compile_forge_from_string, compile_forge_with_imports, resolve_broadcast_log_retention,
    BroadcastLogElementFacts, BroadcastLogResolution, DocumentLabel, ForgeCompileOptions,
};

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

fn parse(content: &str, name: &'static str) -> Result<BroadcastLogModel, Located<ForgeError>> {
    match parse_forge(content, label(name))? {
        Some(ForgeDocument::BroadcastLog(m)) => Ok(m),
        Some(other) => panic!(
            "expected ForgeDocument::BroadcastLog, got {:?}",
            other.kind()
        ),
        None => panic!("statechart routed through forge entry - fixture mis-tagged?"),
    }
}

/// A broadcast-log document whose four required lines the test writes. An empty
/// string leaves the line out.
fn log_doc(name: &str, element: &str, producers: &str, progress: &str, retention: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="broadcast-log" name="{name}" version="1.0">
  {element}
  {producers}
  {progress}
  {retention}
</scxml>"##
    )
}

/// The same with every line spelled the usual way, from the three words the
/// test varies.
fn simple_doc(name: &str, producers: &str, progress: &str, retention: &str) -> String {
    log_doc(
        name,
        "<sce:element-type>event_record</sce:element-type>",
        &format!("<sce:producers>{producers}</sce:producers>"),
        &format!("<sce:progress>{progress}</sce:progress>"),
        retention,
    )
}

fn resource(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests/forge/resources")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn validation(err: Located<ForgeError>) -> (ValidationError, Vec<DiagnosticCode>) {
    let codes = err.error.to_diagnostics().iter().map(|d| d.code).collect();
    match err.error {
        ForgeError::Validation(boxed) => (*boxed, codes),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn generate_error(err: Located<ForgeError>) -> GenerateError {
    match err.error {
        ForgeError::Generate(boxed) => *boxed,
        other => panic!("expected a generate error, got {other:?}"),
    }
}

/// The row a refusal points at, and that row's text.
fn row_text<'a>(located: &Located<ForgeError>, text: &'a str) -> &'a str {
    let row = located.location.line.expect("the refusal carries a row") as usize;
    text.lines()
        .nth(row - 1)
        .unwrap_or_else(|| panic!("the document has no row {row}"))
}

// --- Parse: the checked-in fixtures ---

#[test]
fn the_minimal_fixture_parses_and_selects_the_wait_free_row() {
    let m = parse(
        &resource("broadcast_log_ast_export_min.scxml"),
        "broadcast_log_ast_export_min",
    )
    .expect("the minimal broadcast log parses");
    assert_eq!(m.element_type, "event_record");
    assert_eq!(m.producers, QueueCardinality::One);
    assert_eq!(m.progress, QueueProgress::WaitFree);
    assert_eq!(m.retention, CapacitySource::CompileConst { value: 7 });

    let selection = m.selection().expect("one producer has a row");
    assert_eq!(
        (selection.publish, selection.read),
        (QueueProgress::WaitFree, QueueProgress::WaitFree)
    );
    assert_eq!(selection.reachable(), QueueProgress::WaitFree);

    // The rows are the elements' own, so a refusal can be placed on them.
    let text = resource("broadcast_log_ast_export_min.scxml");
    let line_of = |row: Option<u32>| {
        let row = row.expect("the model records the row") as usize;
        text.lines().nth(row - 1).expect("the row exists").trim()
    };
    assert!(line_of(m.element_type_line).starts_with("<sce:element-type>"));
    assert!(line_of(m.progress_line).starts_with("<sce:progress>"));
    assert!(line_of(m.retention_line).starts_with("<sce:retention"));
}

#[test]
fn a_deploy_key_retention_is_read_as_a_key_and_not_resolved() {
    let m = parse(
        &resource("broadcast_log_deploy_retention.scxml"),
        "broadcast_log_deploy_retention",
    )
    .expect("a deploy-key retention parses");
    assert_eq!(
        m.retention,
        CapacitySource::DeployKey {
            key: "machines.mcu_node.limits.event_retention".into()
        }
    );
    assert_eq!(m.progress, QueueProgress::LockFree);
}

#[test]
fn the_document_serialises_without_its_rows() {
    let doc = parse_forge(
        &resource("broadcast_log_ast_export_min.scxml"),
        label("broadcast_log_ast_export_min"),
    )
    .expect("parses")
    .expect("a forge document");
    let json = serde_json::to_value(&doc).expect("serialises");
    assert_eq!(json["kind"], "broadcast-log");
    assert_eq!(json["element_type"], "event_record");
    assert_eq!(json["producers"], "one");
    assert_eq!(json["progress"], "wait-free");
    assert_eq!(json["retention"]["value"], 7);
    for row in ["element_type_line", "progress_line", "retention_line"] {
        assert!(json.get(row).is_none(), "{row} is not on the wire: {json}");
    }
}

// --- The kind in the model ---

#[test]
fn the_kind_is_named_parsed_and_classified() {
    assert_eq!(ForgeKind::BroadcastLog.as_attr(), "broadcast-log");
    assert_eq!(ForgeKind::BroadcastLog.to_string(), "broadcast-log");
    assert_eq!(
        ForgeKind::from_attr("broadcast-log"),
        Some(ForgeKind::BroadcastLog)
    );
    assert!(ForgeKind::ALL_ATTR_NAMES.contains(&"broadcast-log"));
    assert!(ForgeKind::BroadcastLog.is_supported());
    assert!(ForgeKind::BroadcastLog.needs_instance());
    assert!(!ForgeKind::BroadcastLog.is_inline_eligible());
    assert_eq!(
        ForgeKind::BroadcastLog.max_runtime_dep(),
        RuntimeDep::ForgeRuntime
    );
    let doc = parse_forge(
        &resource("broadcast_log_ast_export_min.scxml"),
        label("broadcast_log_ast_export_min"),
    )
    .expect("parses")
    .expect("a forge document");
    assert_eq!(doc.kind(), ForgeKind::BroadcastLog);
    assert_eq!(doc.name(), "broadcast_log_ast_export_min");
    assert_eq!(doc.runtime_dep(), RuntimeDep::ForgeRuntime);
    assert!(doc.record_fields().is_none());
}

#[test]
fn many_producers_have_no_row_in_the_selection_table() {
    let mut m = parse(
        &resource("broadcast_log_ast_export_min.scxml"),
        "broadcast_log_ast_export_min",
    )
    .expect("parses");
    m.producers = QueueCardinality::Many;
    assert!(m.selection().is_none());
}

// --- Refusals the document alone decides ---

#[test]
fn every_required_element_is_required() {
    let element = "<sce:element-type>event_record</sce:element-type>";
    let producers = "<sce:producers>one</sce:producers>";
    let progress = "<sce:progress>wait-free</sce:progress>";
    let retention = r#"<sce:retention entries="7"/>"#;
    let whole = [element, producers, progress, retention];
    for (index, missing) in [
        "sce:element-type",
        "sce:producers",
        "sce:progress",
        "sce:retention",
    ]
    .into_iter()
    .enumerate()
    {
        let mut lines = whole;
        lines[index] = "";
        let xml = log_doc("event_ring", lines[0], lines[1], lines[2], lines[3]);
        let (error, _) = validation(parse(&xml, "event_ring").expect_err(missing));
        match error {
            ValidationError::MissingElement { kind, element } => {
                assert_eq!(kind, ForgeKind::BroadcastLog);
                assert_eq!(element, missing);
            }
            other => panic!("{missing}: expected MissingElement, got {other:?}"),
        }
    }
}

#[test]
fn an_empty_element_type_is_refused() {
    let xml = log_doc(
        "event_ring",
        "<sce:element-type> </sce:element-type>",
        "<sce:producers>one</sce:producers>",
        "<sce:progress>wait-free</sce:progress>",
        r#"<sce:retention entries="7"/>"#,
    );
    let (error, _) = validation(parse(&xml, "event_ring").expect_err("empty element type"));
    assert!(matches!(
        error,
        ValidationError::AttributeRuleViolated { ref element, .. } if element == "<sce:element-type>"
    ));
}

#[test]
fn a_word_outside_the_closed_set_is_refused_with_the_words_allowed() {
    let xml = simple_doc(
        "event_ring",
        "several",
        "wait-free",
        r#"<sce:retention entries="7"/>"#,
    );
    let (error, codes) = validation(parse(&xml, "event_ring").expect_err("unknown producers"));
    match error {
        ValidationError::InvalidAttribute {
            element,
            value,
            allowed,
            ..
        } => {
            assert_eq!(element, "<sce:producers>");
            assert_eq!(value, "several");
            assert_eq!(allowed, ["one", "many"]);
        }
        other => panic!("expected InvalidAttribute, got {other:?}"),
    }
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::ValidationInvalidAttribute]
    ));

    let xml = simple_doc(
        "event_ring",
        "one",
        "eventually",
        r#"<sce:retention entries="7"/>"#,
    );
    let (error, _) = validation(parse(&xml, "event_ring").expect_err("unknown progress"));
    match error {
        ValidationError::InvalidAttribute {
            element,
            value,
            allowed,
            ..
        } => {
            assert_eq!(element, "<sce:progress>");
            assert_eq!(value, "eventually");
            assert_eq!(allowed, ["wait-free", "lock-free", "blocking"]);
        }
        other => panic!("expected InvalidAttribute, got {other:?}"),
    }
}

#[test]
fn a_retention_must_be_written_one_way_and_be_a_positive_count() {
    let parse_retention = |retention: &str| {
        let xml = simple_doc("event_ring", "one", "wait-free", retention);
        validation(parse(&xml, "event_ring").expect_err(retention)).0
    };

    // Neither form, and both forms.
    for retention in [
        "<sce:retention/>",
        r#"<sce:retention entries="7" source="deploy" key="machines.m.limits.r"/>"#,
    ] {
        assert!(
            matches!(
                parse_retention(retention),
                ValidationError::ExactlyOneAttribute { ref element, .. } if element == "<sce:retention>"
            ),
            "{retention}"
        );
    }

    // A count that is zero, or not a count.
    for (retention, written) in [
        (r#"<sce:retention entries="0"/>"#, "0"),
        (r#"<sce:retention entries="many"/>"#, "many"),
        (r#"<sce:retention entries="-3"/>"#, "-3"),
    ] {
        match parse_retention(retention) {
            ValidationError::AttributeRuleViolated {
                element,
                attr,
                value,
                ..
            } => {
                assert_eq!(element, "<sce:retention>");
                assert_eq!(attr, "entries");
                assert_eq!(value, written);
            }
            other => panic!("{retention}: expected AttributeRuleViolated, got {other:?}"),
        }
    }

    // The deploy form, incomplete or naming another source.
    assert!(matches!(
        parse_retention(r#"<sce:retention source="deploy"/>"#),
        ValidationError::MissingAttribute { ref attr, .. } if attr == "key"
    ));
    assert!(matches!(
        parse_retention(r#"<sce:retention source="file" key="k"/>"#),
        ValidationError::InvalidAttribute { ref attr, ref allowed, .. }
            if attr == "source" && allowed == &["deploy"]
    ));
}

#[test]
fn many_producers_parse_and_are_then_refused_on_the_row_that_says_so() {
    let xml = simple_doc(
        "event_ring",
        "many",
        "blocking",
        r#"<sce:retention entries="7"/>"#,
    );
    let located = parse(&xml, "event_ring").expect_err("many producers are refused");
    assert!(
        row_text(&located, &xml).contains("<sce:producers>many<"),
        "the refusal is placed on the producers row"
    );
    let (error, codes) = validation(located);
    match error {
        ValidationError::BroadcastLogProducersManyUnsupported { log_name } => {
            assert_eq!(log_name, "event_ring");
        }
        other => panic!("expected BroadcastLogProducersManyUnsupported, got {other:?}"),
    }
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::BroadcastLogProducersManyUnsupported]
    ));
}

#[test]
fn every_progress_a_single_producer_row_gives_parses() {
    // The one row is wait-free on both operations, so a declaration at or
    // below it is met; `broadcast-log/progress-unreachable` waits for a row
    // that gives less.
    for progress in ["wait-free", "lock-free", "blocking"] {
        let xml = simple_doc(
            "event_ring",
            "one",
            progress,
            r#"<sce:retention entries="7"/>"#,
        );
        let m = parse(&xml, "event_ring").unwrap_or_else(|e| panic!("{progress}: {:?}", e.error));
        let selection = m.selection().expect("a row");
        assert!(m.progress <= selection.reachable(), "{progress}");
    }
}

// --- The stamp a narrow target keeps ---

#[test]
fn a_stamp_tells_apart_as_many_publishes_as_its_width_leaves_bits() {
    assert_eq!(broadcast_log_wrap_bound_ops(32), 1u64 << 31);
    assert_eq!(broadcast_log_wrap_bound_ops(16), 1u64 << 15);
    assert_eq!(broadcast_log_wrap_bound_ops(64), 1u64 << 63);
    assert_eq!(broadcast_log_wrap_bound_ops(80), 1u64 << 63);
    assert_eq!(broadcast_log_wrap_bound_ops(0), 1);
}

#[test]
fn the_slot_count_divides_the_stamp_space_only_when_it_is_a_power_of_two() {
    for retention in [1u32, 3, 7, 1023, u32::MAX] {
        assert!(
            broadcast_log_slots_divide_the_stamp_space(retention),
            "{retention} + 1 slots"
        );
    }
    for retention in [2u32, 5, 6, 1000, 1024] {
        assert!(
            !broadcast_log_slots_divide_the_stamp_space(retention),
            "{retention} + 1 slots"
        );
    }
}

fn model(retention: &str) -> BroadcastLogModel {
    let xml = simple_doc("event_ring", "one", "wait-free", retention);
    parse(&xml, "event_ring").expect("parses")
}

fn generate_of(result: Result<(), ForgeError>) -> GenerateError {
    // A refusal placed on a row of the document arrives wrapped in the row.
    fn unwrapped(error: ForgeError) -> GenerateError {
        match error {
            ForgeError::Generate(boxed) => *boxed,
            ForgeError::Positioned { error, .. } => unwrapped(*error),
            other => panic!("expected a generate error, got {other:?}"),
        }
    }
    unwrapped(result.expect_err("the stamp is refused"))
}

#[test]
fn a_full_width_stamp_is_asked_nothing() {
    let m = model(r#"<sce:retention entries="5"/>"#);
    judge_broadcast_log_stamp(&m, 5, 64, None).expect("a 64-bit stamp has no obligation");
}

#[test]
fn a_narrow_stamp_with_a_retention_that_is_not_one_less_than_a_power_of_two_is_refused() {
    let m = model(r#"<sce:retention entries="5"/>"#);
    match generate_of(judge_broadcast_log_stamp(&m, 5, 32, Some(1))) {
        GenerateError::BroadcastLogSlotsNotADivisorOfTheStampSpace {
            log_name,
            retention,
            slots,
            stamp_bits,
            nearest_below,
            nearest_above,
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!((retention, slots, stamp_bits), (5, 6, 32));
            assert_eq!((nearest_below, nearest_above), (3, 7));
        }
        other => panic!("expected BroadcastLogSlotsNotADivisorOfTheStampSpace, got {other:?}"),
    }
}

#[test]
fn a_narrow_stamp_needs_the_delay_the_design_relies_on_and_a_bound_that_covers_it() {
    let m = model(r#"<sce:retention entries="7"/>"#);
    // A 32-bit stamp tells 2^31 publishes of a slot apart, and 8 slots (retention 7
    // and one) of that are the cursor's lag and the publish in flight: the longest
    // pause a deploy may rely on is 2^31 - 8.
    let bound = (1u64 << 31) - 8;

    match generate_of(judge_broadcast_log_stamp(&m, 7, 32, None)) {
        GenerateError::BroadcastLogWrapBoundUnstated {
            log_name,
            stamp_bits,
            bound: stated,
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!((stamp_bits, stated), (32, bound));
        }
        other => panic!("expected BroadcastLogWrapBoundUnstated, got {other:?}"),
    }

    match generate_of(judge_broadcast_log_stamp(&m, 7, 32, Some(bound + 1))) {
        GenerateError::BroadcastLogWrapBoundBelowDeployMinimum {
            bound: stated,
            min_wrap_ops,
            ..
        } => assert_eq!((stated, min_wrap_ops), (bound, bound + 1)),
        other => panic!("expected BroadcastLogWrapBoundBelowDeployMinimum, got {other:?}"),
    }

    // A delay the bound covers, exactly and below, is met.
    judge_broadcast_log_stamp(&m, 7, 32, Some(bound)).expect("the bound covers the delay");
    judge_broadcast_log_stamp(&m, 7, 32, Some(1)).expect("the bound covers the delay");
}

// --- The retention a deploy key names ---

/// A deploy file with one machine, `mcu_node`, holding the limits written (each
/// line indented ten spaces, under `limits:`).
fn deploy_with_limits(limits: &str) -> DeployConfig {
    parse_deploy_str(&format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
{limits}
"##
    ))
    .expect("deploy parses")
}

fn deploy_key_model(key: &str) -> (BroadcastLogModel, String) {
    let xml = simple_doc(
        "event_ring",
        "one",
        "wait-free",
        &format!(r#"<sce:retention source="deploy" key="{key}"/>"#),
    );
    let m = parse(&xml, "event_ring").expect("a deploy-key retention parses");
    (m, xml)
}

#[test]
fn a_literal_retention_resolves_to_itself_whatever_the_deploy_says() {
    let m = model(r#"<sce:retention entries="7"/>"#);
    let cfg = deploy_with_limits("          event_retention: 99");
    let resolved = resolve_broadcast_log_retention(&m, &cfg, "mcu_node", ".scxml-fixture")
        .expect("a literal needs no limit");
    assert_eq!(resolved, Some(7));
}

#[test]
fn a_deploy_key_resolves_to_the_limit_of_the_target_machine() {
    let (m, _) = deploy_key_model("machines.mcu_node.limits.event_retention");
    let cfg = deploy_with_limits("          event_retention: 31\n          other_limit: 4");
    let resolved = resolve_broadcast_log_retention(&m, &cfg, "mcu_node", ".scxml-fixture")
        .expect("the limit is declared");
    assert_eq!(resolved, Some(31));
}

#[test]
fn a_key_for_another_machine_or_of_another_shape_is_left_to_that_machines_own_compile() {
    let cfg = deploy_with_limits("          event_retention: 31");
    for key in [
        // Another machine of the deploy, and one the deploy does not hold.
        "machines.other_node.limits.event_retention",
        "machines.ghost_node.limits.event_retention",
        // Not `machines.<machine>.limits.<limit>`.
        "limits.event_retention",
        "machines.mcu_node.event_retention",
    ] {
        let (m, _) = deploy_key_model(key);
        let resolved = resolve_broadcast_log_retention(&m, &cfg, "mcu_node", ".scxml-fixture")
            .unwrap_or_else(|e| panic!("{key}: {:?}", e.error));
        assert_eq!(resolved, None, "{key}");
    }
}

#[test]
fn a_machine_the_deploy_does_not_hold_resolves_a_key_naming_it_to_nothing() {
    let (m, _) = deploy_key_model("machines.ghost_node.limits.event_retention");
    let cfg = deploy_with_limits("          event_retention: 31");
    let resolved = resolve_broadcast_log_retention(&m, &cfg, "ghost_node", ".scxml-fixture")
        .expect("an unknown machine is not an undeclared limit");
    assert_eq!(resolved, None);
}

#[test]
fn a_limit_the_machine_does_not_declare_is_refused_with_the_limits_it_does() {
    let (m, xml) = deploy_key_model("machines.mcu_node.limits.event_retention");
    // Declared in the other order than they are listed, so the sort shows.
    let cfg = deploy_with_limits("          beta: 2\n          alpha: 1");
    let located = resolve_broadcast_log_retention(&m, &cfg, "mcu_node", ".scxml-fixture")
        .expect_err("the key names a limit the machine lacks");
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    match generate_error(located) {
        GenerateError::InvalidConfig(detail) => {
            assert!(detail.contains("broadcast-log 'event_ring'"), "{detail}");
            assert!(
                detail.contains("machines.mcu_node.limits.event_retention"),
                "{detail}"
            );
            assert!(detail.contains("alpha, beta"), "{detail}");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

// --- Cross-document: the element type ---

fn codec_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="sensor" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
  </datamodel>
</scxml>"##
    )
}

fn compile_set(
    language: Language,
    log_xml: &str,
) -> Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    let codec_path = dir.path().join("event_record.scxml");
    fs::write(&codec_path, codec_doc("event_record")).expect("write codec");
    let log_path = dir.path().join("event_ring.scxml");
    fs::write(&log_path, log_xml).expect("write log");
    compile_scxml_with_imports(
        &[],
        &[codec_path.as_path(), log_path.as_path()],
        &sce_build::find_template_dir_for(language),
        language,
        &ForgeCompileOptions::default(),
        None,
    )
}

#[test]
fn an_element_type_that_names_no_document_is_refused_with_the_documents_there_are() {
    let xml = simple_doc(
        "event_ring",
        "one",
        "wait-free",
        r#"<sce:retention entries="7"/>"#,
    )
    .replace("event_record", "event_recrd");
    let located = compile_set(Language::Rust, &xml)
        .err()
        .unwrap_or_else(|| panic!("the element type resolves nowhere, so the set is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_recrd<"),
        "the refusal is placed on the element-type row"
    );
    let diagnostics = located.error.to_diagnostics();
    assert!(matches!(
        diagnostics.as_slice(),
        [d] if matches!(d.code, DiagnosticCode::BroadcastLogElementTypeNotAKind)
    ));
    match validation(located).0 {
        ValidationError::BroadcastLogElementTypeNotAKind {
            log_name,
            element_type,
            candidates,
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!(element_type, "event_recrd");
            assert_eq!(candidates, ["event_record"], "the documents in the build");
        }
        other => panic!("expected BroadcastLogElementTypeNotAKind, got {other:?}"),
    }
}

// --- Cross-document: the element's encoded size ---

/// A codec document with the imports and the fields the test writes.
fn codec_with(name: &str, imports: &str, fields: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  {imports}
  <datamodel>
    {fields}
  </datamodel>
</scxml>"##
    )
}

/// The documents written to one directory and compiled for `language` in the
/// order given, so that a document placed first is the first one generated.
fn compile_documents(
    language: Language,
    documents: &[(&str, &String)],
) -> Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>> {
    compile_documents_with(language, documents, &options_for(language), None)
}

/// [`compile_documents`] with the options and the deploy the test gives.
fn compile_documents_with(
    language: Language,
    documents: &[(&str, &String)],
    options: &ForgeCompileOptions,
    deploy: Option<&DeployConfig>,
) -> Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    let paths: Vec<PathBuf> = documents
        .iter()
        .map(|(file, text)| {
            let path = dir.path().join(file);
            fs::write(&path, text).expect("write document");
            path
        })
        .collect();
    let forge_files: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    compile_scxml_with_imports(
        &[],
        &forge_files,
        &sce_build::find_template_dir_for(language),
        language,
        options,
        deploy,
    )
}

/// What a refusal of `broadcast-log/element-size-unbounded` says, once its code,
/// its row and the names it carries are checked: the reason it gives.
fn unbounded_reason(
    located: Located<ForgeError>,
    log_xml: &str,
    element: &str,
    element_kind: &str,
) -> String {
    assert!(
        row_text(&located, log_xml).contains("<sce:element-type>"),
        "the refusal is placed on the element-type row"
    );
    let codes: Vec<DiagnosticCode> = located
        .error
        .to_diagnostics()
        .iter()
        .map(|d| d.code)
        .collect();
    assert!(
        matches!(
            codes.as_slice(),
            [DiagnosticCode::BroadcastLogElementSizeUnbounded]
        ),
        "{codes:?}"
    );
    match validation(located).0 {
        ValidationError::BroadcastLogElementSizeUnbounded {
            log_name,
            element_type,
            element_kind: kind,
            reason,
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!(element_type, element);
            assert_eq!(kind, element_kind);
            reason
        }
        other => panic!("expected BroadcastLogElementSizeUnbounded, got {other:?}"),
    }
}

const UNBOUNDED_FIELDS: &str = r#"<data id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="tail"/>"#;

const BOUNDED_FIELDS: &str = r#"<data id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="tail" sce:max-size="64"/>"#;

fn event_ring() -> String {
    simple_doc(
        "event_ring",
        "one",
        "wait-free",
        r#"<sce:retention entries="7"/>"#,
    )
}

#[test]
fn an_element_with_a_payload_no_declaration_bounds_is_refused_on_the_element_type_row() {
    let xml = event_ring();
    let codec = codec_with("event_record", "", UNBOUNDED_FIELDS);
    let located = compile_documents(
        Language::Rust,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .err()
    .unwrap_or_else(|| panic!("the payload has no bound, so the set is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    let reason = unbounded_reason(located, &xml, "event_record", "codec");
    assert!(reason.contains("'payload'"), "{reason}");
    assert!(reason.contains("sce:max-size"), "{reason}");
}

#[test]
fn the_size_refusal_is_the_same_on_every_language() {
    let xml = event_ring();
    let codec = codec_with("event_record", "", UNBOUNDED_FIELDS);
    for language in Language::ALL
        .iter()
        .copied()
        .filter(|language| *language != Language::Swift)
    {
        let wire = sce_build::forge::codegen_matrix::language_wire_name(language);
        let located = compile_documents(
            language,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .err()
        .unwrap_or_else(|| panic!("{wire}: the payload has no bound, so the set is refused"));
        let reason = unbounded_reason(located, &xml, "event_record", "codec");
        assert!(reason.contains("'payload'"), "{wire}: {reason}");
    }
}

#[test]
fn a_procedure_has_no_wire_form_and_is_refused_as_an_element() {
    let procedure = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="frame_record" version="1.0" initial="run">
  <datamodel>
    <data id="serial" sce:type="uint32" sce:direction="in"/>
    <data id="checksum" sce:type="uint64" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="run">
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>"##
        .to_string();
    let xml = event_ring().replace("event_record", "frame_record");
    let located = compile_documents(
        Language::Rust,
        &[
            ("event_ring.scxml", &xml),
            ("frame_record.scxml", &procedure),
        ],
    )
    .err()
    .unwrap_or_else(|| panic!("a procedure has no encoded size, so the set is refused"));
    let reason = unbounded_reason(located, &xml, "frame_record", "procedure");
    assert!(reason.contains("no wire form"), "{reason}");
}

#[test]
fn an_element_that_embeds_a_codec_with_no_bound_is_refused_through_the_embed() {
    let xml = event_ring();
    let parent = codec_with(
        "event_record",
        r#"<sce:import src="payload_body.scxml" kind="codec" as="payload_body"/>"#,
        r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="body" type="payload_body" sce:byte="1"/>"#,
    );
    let body = codec_with(
        "payload_body",
        "",
        r#"<data id="blob" sce:type="bytes" sce:byte="0" sce:bit-size="tail"/>"#,
    );
    let located = compile_documents(
        Language::Rust,
        &[
            ("event_ring.scxml", &xml),
            ("event_record.scxml", &parent),
            ("payload_body.scxml", &body),
        ],
    )
    .err()
    .unwrap_or_else(|| panic!("the embedded body has no bound, so the set is refused"));
    let reason = unbounded_reason(located, &xml, "event_record", "codec");
    assert!(reason.contains("'body'"), "{reason}");
    assert!(reason.contains("payload_body"), "{reason}");
    assert!(reason.contains("'blob'"), "{reason}");
}

/// Whether the language's broadcast-log template has landed
/// ([`BROADCAST_LOG_BACKENDS`]).
fn has_landed(language: Language) -> bool {
    BROADCAST_LOG_BACKENDS.contains(&language)
}

/// The languages a document set can be compiled for: every one with an emitter
/// for a codec, which Swift has not.
fn set_languages() -> impl Iterator<Item = Language> {
    Language::ALL
        .iter()
        .copied()
        .filter(|language| *language != Language::Swift)
}

/// What a valid set that no size refusal touches comes to on `language`: the
/// generated log where its template has landed, and the one refusal a valid
/// document has where it has not, the missing runtime. The document declares
/// `wait-free`, so a language whose ceiling is lower (Python) refuses the
/// progress first, which is no size refusal either.
fn assert_valid_set_is_not_refused_for_its_size(
    language: Language,
    result: Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>>,
    what: &str,
) {
    let wire = language_wire_name(language);
    let (ceiling, _) = broadcast_log_progress_ceiling(language);
    let codes: Vec<DiagnosticCode> = match result {
        Ok(_) => {
            assert!(
                has_landed(language),
                "{wire}: {what}: generated with no template"
            );
            return;
        }
        Err(located) => located
            .error
            .to_diagnostics()
            .iter()
            .map(|d| d.code)
            .collect(),
    };
    let as_expected = if ceiling < QueueProgress::WaitFree {
        matches!(
            codes.as_slice(),
            [DiagnosticCode::BroadcastLogProgressUnreachableOnBackend]
        )
    } else {
        !has_landed(language)
            && matches!(
                codes.as_slice(),
                [DiagnosticCode::BroadcastLogRuntimeMissing]
            )
    };
    assert!(as_expected, "{wire}: {what}: {codes:?}");
}

#[test]
fn an_element_whose_every_field_is_bounded_is_not_refused_for_its_size() {
    let xml = event_ring();
    for language in set_languages() {
        for (name, codec) in [
            ("a fixed record", codec_doc("event_record")),
            (
                "a payload with a max-size",
                codec_with("event_record", "", BOUNDED_FIELDS),
            ),
        ] {
            assert_valid_set_is_not_refused_for_its_size(
                language,
                compile_documents(
                    language,
                    &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
                ),
                name,
            );
        }
    }
}

#[test]
fn an_element_that_embeds_a_bounded_codec_is_not_refused_for_its_size() {
    // The embedded codec is resolved through the element's own import, so a
    // bound that lives one document away is found and not guessed at.
    let xml = event_ring();
    let parent = codec_with(
        "event_record",
        r#"<sce:import src="payload_body.scxml" kind="codec" as="payload_body"/>"#,
        r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="body" type="payload_body" sce:byte="1"/>"#,
    );
    let body = codec_with(
        "payload_body",
        "",
        r#"<data id="blob" sce:type="bytes" sce:byte="0" sce:bit-size="tail" sce:max-size="16"/>"#,
    );
    for language in set_languages() {
        assert_valid_set_is_not_refused_for_its_size(
            language,
            compile_documents(
                language,
                &[
                    ("event_ring.scxml", &xml),
                    ("event_record.scxml", &parent),
                    ("payload_body.scxml", &body),
                ],
            ),
            "an embedded bounded codec",
        );
    }
}

// --- Generation: a language with no row refuses by name, Rust generates ---

fn options_for(language: Language) -> ForgeCompileOptions {
    match language {
        Language::Go => ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme/project/generated".to_string()),
            ..ForgeCompileOptions::default()
        },
        Language::Rust
        | Language::Cpp
        | Language::Kotlin
        | Language::Python
        | Language::C11
        | Language::Swift => ForgeCompileOptions::default(),
    }
}

#[test]
fn a_valid_document_is_refused_by_name_on_every_language_with_no_template() {
    let xml = simple_doc(
        "event_ring",
        "one",
        "blocking",
        r#"<sce:retention entries="1023"/>"#,
    );
    for language in Language::ALL
        .iter()
        .copied()
        .filter(|language| !BROADCAST_LOG_BACKENDS.contains(language))
    {
        let wire = sce_build::forge::codegen_matrix::language_wire_name(language);
        let located = if language == Language::Swift {
            compile_forge_from_string(&xml, label("event_ring"), language)
                .err()
                .unwrap_or_else(|| panic!("swift has no runtime, so it refuses the document"))
        } else {
            let dir = tempdir().expect("tempdir");
            let codec_path = dir.path().join("event_record.scxml");
            fs::write(&codec_path, codec_doc("event_record")).expect("write codec");
            let log_path = dir.path().join("event_ring.scxml");
            fs::write(&log_path, &xml).expect("write log");
            compile_scxml_with_imports(
                &[],
                &[codec_path.as_path(), log_path.as_path()],
                &sce_build::find_template_dir_for(language),
                language,
                &options_for(language),
                None,
            )
            .err()
            .unwrap_or_else(|| panic!("{wire}: this language has no broadcast-log row yet"))
        };
        let codes: Vec<DiagnosticCode> = located
            .error
            .to_diagnostics()
            .iter()
            .map(|d| d.code)
            .collect();
        assert!(
            matches!(
                codes.as_slice(),
                [DiagnosticCode::BroadcastLogRuntimeMissing]
            ),
            "{wire}: {codes:?}"
        );
        match generate_error(located) {
            GenerateError::BroadcastLogRuntimeMissing { log_name, language } => {
                assert_eq!(log_name, "event_ring");
                assert_eq!(language, wire);
            }
            other => panic!("{wire}: expected BroadcastLogRuntimeMissing, got {other:?}"),
        }
    }
}

#[test]
fn a_progress_the_python_backend_cannot_give_is_refused_before_the_missing_runtime() {
    let xml = simple_doc(
        "event_ring",
        "one",
        "wait-free",
        r#"<sce:retention entries="1023"/>"#,
    );
    let located = compile_forge_from_string(&xml, label("event_ring"), Language::Python)
        .err()
        .unwrap_or_else(|| panic!("python gives blocking at most, so it refuses wait-free"));
    assert!(
        row_text(&located, &xml).contains("<sce:progress>"),
        "the refusal is placed on the progress row"
    );
    match generate_error(located) {
        GenerateError::BroadcastLogProgressUnreachableOnBackend {
            log_name,
            declared,
            reachable,
            language,
            ..
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!(declared, "wait-free");
            assert_eq!(reachable, "blocking");
            assert_eq!(language, "python");
        }
        other => panic!("expected BroadcastLogProgressUnreachableOnBackend, got {other:?}"),
    }

    // A backend that can give it and has no row (Swift, which has no forge runtime
    // and is the one language the list of landed templates never holds) reaches the
    // missing runtime instead.
    assert!(!has_landed(Language::Swift));
    let located = compile_forge_from_string(&xml, label("event_ring"), Language::Swift)
        .err()
        .unwrap_or_else(|| panic!("swift has no row, so it refuses the document"));
    assert!(matches!(
        generate_error(located),
        GenerateError::BroadcastLogRuntimeMissing { .. }
    ));
}

#[test]
fn the_ceiling_of_each_language_is_its_runtimes() {
    for language in Language::ALL.iter().copied() {
        let (ceiling, because) = broadcast_log_progress_ceiling(language);
        if language == Language::Python {
            assert_eq!(ceiling, QueueProgress::Blocking);
            assert!(!because.is_empty());
        } else {
            assert_eq!(ceiling, QueueProgress::WaitFree, "{language:?}");
            assert!(because.is_empty(), "{language:?}");
        }
    }
}

// --- Rust generates the log over its runtime row ---

/// The text of the file `name` among the outputs of a set that generated.
fn generated_file(
    outputs: Vec<(String, sce_build::generator::GeneratedOutput)>,
    name: &str,
) -> String {
    outputs
        .into_iter()
        .flat_map(|(_, output)| output.files)
        .find(|(file, _)| file == name)
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the set generated no file `{name}`"))
}

/// The generated Rust of the log `event_ring`, out of the outputs of a set that
/// generated.
fn generated_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    generated_file(outputs, "event_ring.rs")
}

/// The text of a generated module must hold every line of `needles`.
fn assert_holds(code: &str, needles: &[&str]) {
    for needle in needles {
        assert!(code.contains(needle), "missing `{needle}` in:\n{code}");
    }
}

#[test]
fn rust_generates_a_log_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; a slot is then
    // 1 + ceil(4 / 8) = 1 + 1 = 2 words, which the module computes and does not
    // restate.
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_log(
        compile_documents(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("rust generates a broadcast log"),
    );
    assert_holds(
        &code,
        &[
            r#"// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")"#,
            "// Runtime: sce_forge_runtime::broadcast_log",
            "use sce_forge_runtime::broadcast_log as runtime;",
            "use sce_forge_runtime::codec::{CodecError, SceCursor, SliceSink};",
            "use super::event_record::EventRecord;",
            "pub const RETENTION: usize = 7;",
            "pub const SLOTS: usize = RETENTION + 1;",
            "pub const MAX_ENCODED_BYTES: usize = 4;",
            "pub const WORDS: usize = 1 + MAX_ENCODED_BYTES.div_ceil(8);",
            r#"pub const DECLARED_PROGRESS: &str = "wait-free";"#,
            "pub type EventRing = runtime::BroadcastLog<WORDS, SLOTS>;",
            "pub type EventRingPublisher<'a> = runtime::Publisher<'a, WORDS, SLOTS>;",
            "pub enum EventRingRead {",
            "Item { seq: u64, element: EventRecord },",
            "pub fn publish(",
            "publisher: &mut EventRingPublisher<'_>,",
            "element: &EventRecord,",
            "let mut sink = SliceSink::new(&mut bytes);",
            "element.encode(&mut sink)?;",
            "pub fn read(",
            "log: &EventRing,",
            "runtime::Read::Item { seq } => {",
            "let element = EventRecord::decode(&mut cursor)?;",
        ],
    );
    // A stored length above the bound is refused before any slice is cut by it.
    let length_check = code
        .find("if stored > MAX_ENCODED_BYTES as u64 {")
        .expect("the stored length is checked against the bound");
    let refusal = code[length_check..]
        .find("return Err(CodecError::LengthOutOfRange);")
        .expect("a length above the bound is a codec error");
    let slice = code[length_check..]
        .find("bytes[..length]")
        .expect("the entry's bytes are cut by the length");
    assert!(
        refusal < slice,
        "the refusal comes before the slice the length cuts:\n{code}"
    );
    assert!(
        !code.contains("EventRecordOwned"),
        "a record that borrows nothing is its own stored form:\n{code}"
    );
}

#[test]
fn the_compile_error_is_gated_by_the_cfg_the_runtime_module_is() {
    let lib = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/rust/forge-runtime/src/lib.rs"),
    )
    .expect("the runtime's lib.rs");
    let at = lib
        .find("pub mod broadcast_log;")
        .expect("the runtime has the broadcast_log module");
    let gate = lib[..at]
        .lines()
        .last()
        .expect("a line precedes the module")
        .trim()
        .strip_prefix("#[cfg(")
        .and_then(|rest| rest.strip_suffix(")]"))
        .expect("the module is gated by one cfg attribute")
        .to_string();

    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_log(
        compile_documents(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("rust generates a broadcast log"),
    );
    assert!(
        code.contains(&format!("#[cfg(not({gate}))]\ncompile_error!(")),
        "the generated module refuses a target the runtime module is absent on, under the \
         cfg `{gate}`:\n{code}"
    );
}

#[test]
fn an_element_that_borrows_its_buffer_is_stored_as_its_owned_form() {
    // A one-byte head and a payload of at most 64 bytes: 1 + 64 = 65 bytes, so
    // a slot is 1 + ceil(65 / 8) = 1 + 9 = 10 words. The payload is a view of
    // the buffer it was decoded from, so the log hands back the owned mirror.
    let xml = event_ring();
    let codec = codec_with("event_record", "", BOUNDED_FIELDS);
    let code = generated_log(
        compile_documents(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("rust generates a broadcast log over a borrowing element"),
    );
    assert_holds(
        &code,
        &[
            "pub const MAX_ENCODED_BYTES: usize = 65;",
            "use super::event_record::EventRecord;",
            "use super::event_record::EventRecordOwned;",
            "Item { seq: u64, element: EventRecordOwned<::sce_forge_runtime::codec::Inline> },",
            "element: &EventRecord,",
            "let element = EventRecord::decode(&mut cursor)?",
            ".try_into_owned_in::<::sce_forge_runtime::codec::Inline>()?;",
        ],
    );
}

/// A codec `event_record` whose `encode` and `decode` take a flag input from the
/// caller, which a log stores with the entry.
fn flag_input_codec() -> String {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="event_record" version="1.0">
  <sce:flag-inputs>
    <sce:flag-input name="S" width="1"/>
  </sce:flag-inputs>
  <datamodel>
    <sce:field id="version" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="detail" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="S"/>
  </datamodel>
</scxml>"##
        .to_string()
}

/// A CBOR codec `event_record` whose one entry is `entry`.
fn cbor_codec_with(entry: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="event_record" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    {entry}
  </datamodel>
</scxml>"##
    )
}

/// Whether the language's broadcast-log template stores the context of an element, the
/// flag inputs and the variant tag its codec asks its caller for
/// ([`BROADCAST_LOG_CONTEXT_BACKENDS`]).
fn stores_context(language: Language) -> bool {
    sce_build::forge::codegen_matrix::broadcast_log_stores_context(language)
}

#[test]
fn rust_stores_the_flag_inputs_an_element_asks_its_caller_for() {
    // The codec `event_record` takes the flag input `S` from its caller on `encode` and
    // `decode`. The log stores it in a context word of the entry, takes it on `publish`
    // as a bool, and gives it back with the item, so that every reader decodes the entry
    // under what it was published under. The element is a `uint8` and an optional
    // `uint8`: 2 bytes, an entry of 1 length word, 1 context word and 1 word of bytes.
    let xml = event_ring();
    let codec = flag_input_codec();
    let code = generated_log(
        compile_documents(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a log stores the flag input its element asks for"),
    );
    assert_holds(
        &code,
        &[
            "pub const MAX_ENCODED_BYTES: usize = 2;",
            "pub const CONTEXT_WORDS: usize = 1;",
            "pub const WORDS: usize = 1 + CONTEXT_WORDS + MAX_ENCODED_BYTES.div_ceil(8);",
            "pub struct EventRingFlags {",
            "pub s: bool,",
            "flags: EventRingFlags,",
            "element.encode(&mut sink, u8::from(flags.s))?;",
            "entry[1 + 0] |= u64::from(flags.s) << 0;",
            "entry[2 + index / 8] |= u64::from(*byte)",
            "s: ((entry[1 + 0] >> 0) & 1) == 1,",
            "let element = EventRecord::decode(&mut cursor, u8::from(flags.s))?;",
            "Ok(EventRingRead::Item {",
        ],
    );
    assert!(!code.contains("tag: u8"), "the element has no tag:\n{code}");
}

#[test]
fn rust_stores_the_flags_and_the_tag_of_an_element_in_context_words_of_their_own() {
    // Two flag inputs fit one word; the tag takes the next. The bytes follow the
    // length word and the two context words, so they start at word 3.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the flags and the tag its element asks for");
    assert_holds(
        &code,
        &[
            "pub const CONTEXT_WORDS: usize = 2;",
            "pub const WORDS: usize = 1 + CONTEXT_WORDS + MAX_ENCODED_BYTES.div_ceil(8);",
            "pub s: bool,",
            "pub t: bool,",
            "flags: EventRingFlags,",
            "tag: u8,",
            "element.encode(&mut sink, u8::from(flags.s), u8::from(flags.t))?;",
            "entry[1 + 0] |= u64::from(flags.s) << 0;",
            "entry[1 + 0] |= u64::from(flags.t) << 1;",
            "entry[1 + 1] = u64::from(tag);",
            "entry[3 + index / 8] |= u64::from(*byte)",
            "let tag = entry[1 + 1] as u8;",
            "EventRecord::decode(&mut cursor, u8::from(flags.s), u8::from(flags.t), tag)?",
        ],
    );
}

#[test]
fn a_tag_alone_is_one_context_word_and_the_signature_takes_no_flags() {
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the tag its element asks for");
    assert_holds(
        &code,
        &[
            "pub const CONTEXT_WORDS: usize = 1;",
            "tag: u8,",
            "entry[1 + 0] = u64::from(tag);",
            "entry[2 + index / 8] |= u64::from(*byte)",
            "let tag = entry[1 + 0] as u8;",
            "EventRecord::decode(&mut cursor, tag)?",
        ],
    );
    assert!(
        !code.contains("Flags"),
        "the element has no flag input:\n{code}"
    );
}

#[test]
fn the_narrow_row_stores_the_context_in_32_bit_words() {
    // The same entry over 32-bit words: 33 flags need two flag words, the tag a third.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: (0..33).map(|index| format!("f{index}")).collect(),
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let mut options = target_options(None, None, Some("thumbv7em-none-eabihf"), Some(1_000_000));
    options
        .broadcast_log_resolutions
        .as_mut()
        .and_then(|resolutions| resolutions.get_mut("event_ring"))
        .expect("the log has a resolution")
        .element = facts;
    let code = compile_single(&event_ring(), &options)
        .unwrap_or_else(|e| panic!("a 32-bit target stores the context: {:?}", e.error));
    assert_holds(
        &code,
        &[
            "pub const CONTEXT_WORDS: usize = 3;",
            "entry[1 + 0] |= u32::from(flags.f0) << 0;",
            "entry[1 + 0] |= u32::from(flags.f31) << 31;",
            "entry[1 + 1] |= u32::from(flags.f32) << 0;",
            "entry[1 + 2] = u32::from(tag);",
            "entry[4 + index / 4] |= u32::from(*byte)",
        ],
    );
}

#[test]
fn an_element_with_no_caller_input_has_the_entry_and_the_signatures_it_always_had() {
    let code = compile_single(
        &event_ring(),
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .expect("an element with no caller input is generated");
    assert!(!code.contains("CONTEXT_WORDS"), "{code}");
    assert!(!code.contains("Flags"), "{code}");
    assert_holds(
        &code,
        &[
            "pub const WORDS: usize = 1 + MAX_ENCODED_BYTES.div_ceil(8);",
            "Item { seq: u64, element: EventRecord },",
            "entry[1 + index / 8] |= u64::from(*byte)",
        ],
    );
}

#[test]
fn a_cbor_element_that_borrows_its_buffer_is_refused_and_one_that_does_not_is_held() {
    let xml = event_ring();
    let borrowing = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let located = compile_documents(
        Language::Rust,
        &[
            ("event_ring.scxml", &xml),
            ("event_record.scxml", &borrowing),
        ],
    )
    .err()
    .unwrap_or_else(|| panic!("a string entry borrows the buffer, so the set is refused"));
    match generate_error(located) {
        GenerateError::InvalidConfig(detail) => {
            assert!(detail.contains("a CBOR map with a string"), "{detail}");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }

    let scalar =
        cbor_codec_with(r#"<data id="level" sce:type="uint8" sce:key="1" sce:direction="out"/>"#);
    let code = generated_log(
        compile_documents(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &scalar)],
        )
        .expect("a map of scalars borrows nothing, so rust generates the log"),
    );
    assert_holds(&code, &["use super::event_record::EventRecord;"]);
}

// --- The numbers a render reads, supplied by hand ---

fn resolution_options(
    retention: Option<u32>,
    element_max_bytes: u64,
    element: BroadcastLogElementFacts,
) -> ForgeCompileOptions {
    ForgeCompileOptions {
        broadcast_log_resolutions: Some(HashMap::from([(
            "event_ring".to_string(),
            BroadcastLogResolution {
                retention,
                element_max_bytes,
                element,
                ..BroadcastLogResolution::default()
            },
        )])),
        ..ForgeCompileOptions::default()
    }
}

/// The generated Rust of the single document `xml`, compiled with `options`.
fn compile_single(xml: &str, options: &ForgeCompileOptions) -> Result<String, Located<ForgeError>> {
    compile_single_as(Language::Rust, "event_ring.rs", xml, options)
}

/// The file `file` generated for `language` from the single document `xml`,
/// compiled with `options`.
fn compile_single_as(
    language: Language,
    file: &str,
    xml: &str,
    options: &ForgeCompileOptions,
) -> Result<String, Located<ForgeError>> {
    compile_forge_with_imports(xml, label("event_ring"), language, Path::new("."), options).map(
        |output| {
            output
                .files
                .into_iter()
                .find(|(name, _)| name == file)
                .map(|(_, code)| code)
                .unwrap_or_else(|| panic!("the log's module `{file}`"))
        },
    )
}

const DEPLOY_KEY_RETENTION: &str =
    r#"<sce:retention source="deploy" key="machines.mcu_node.limits.event_retention"/>"#;

fn deploy_key_ring() -> String {
    simple_doc("event_ring", "one", "wait-free", DEPLOY_KEY_RETENTION)
}

fn invalid_config(located: Located<ForgeError>) -> String {
    match generate_error(located) {
        GenerateError::InvalidConfig(detail) => detail,
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn a_single_document_with_no_resolution_is_refused_naming_what_is_missing() {
    let located = compile_single(&event_ring(), &ForgeCompileOptions::default())
        .err()
        .unwrap_or_else(|| panic!("no number is guessed, so a log with no resolution is refused"));
    let detail = invalid_config(located);
    assert!(detail.contains("broadcast-log 'event_ring'"), "{detail}");
    assert!(detail.contains("the element 'event_record'"), "{detail}");
    assert!(
        detail.contains("ForgeCompileOptions::broadcast_log_resolutions"),
        "{detail}"
    );
}

#[test]
fn a_deploy_key_the_set_could_not_resolve_is_refused_on_the_retention_row() {
    let xml = deploy_key_ring();
    let located = compile_single(
        &xml,
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("an unresolved retention is not guessed"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    let detail = invalid_config(located);
    assert!(
        detail.contains("machines.mcu_node.limits.event_retention"),
        "{detail}"
    );
    assert!(detail.contains("target machine"), "{detail}");
}

#[test]
fn a_resolved_retention_and_size_become_the_modules_constants() {
    // 9 bytes encode to words of 1 + ceil(9 / 8) = 1 + 2 = 3, computed by the
    // module from the constant below.
    let code = compile_single(
        &deploy_key_ring(),
        &resolution_options(Some(15), 9, BroadcastLogElementFacts::default()),
    )
    .expect("a resolved log generates");
    assert_holds(
        &code,
        &[
            "pub const RETENTION: usize = 15;",
            "pub const SLOTS: usize = RETENTION + 1;",
            "pub const MAX_ENCODED_BYTES: usize = 9;",
        ],
    );
}

#[test]
fn a_literal_retention_is_the_documents_whatever_the_resolution_says() {
    let code = compile_single(
        &event_ring(),
        &resolution_options(Some(99), 4, BroadcastLogElementFacts::default()),
    )
    .expect("a literal needs no resolution of its own");
    assert!(code.contains("pub const RETENTION: usize = 7;"), "{code}");
    assert!(!code.contains("= 99;"), "{code}");
}

#[test]
fn a_retention_that_resolves_to_nothing_is_refused() {
    let xml = deploy_key_ring();
    let located = compile_single(
        &xml,
        &resolution_options(Some(0), 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("a log retains at least one entry"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    assert!(
        invalid_config(located).contains("the retention resolves to 0"),
        "a limit of 0 keeps one slot, which the protocol cannot represent"
    );
}

#[test]
fn an_element_the_template_cannot_hold_is_refused_on_the_element_type_row() {
    let xml = event_ring();
    let why = "a CBOR map with a string or byte string entry decodes to a value that borrows";
    let facts = BroadcastLogElementFacts {
        cbor_text_or_bytes_entry: true,
        ..BroadcastLogElementFacts::default()
    };
    let located = compile_single(&xml, &resolution_options(None, 4, facts))
        .err()
        .unwrap_or_else(|| panic!("an element the template cannot hold is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    let detail = invalid_config(located);
    assert!(
        detail.contains("the element 'event_record' cannot be stored by the Rust template"),
        "{detail}"
    );
    assert!(detail.contains(why), "{detail}");
}

// --- The facts of an element are the codec's; the verdict on them is a language's ---

#[test]
fn what_a_caller_must_give_is_stored_where_the_template_stores_it_and_refused_by_name_elsewhere() {
    let flags = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        ..BroadcastLogElementFacts::default()
    };
    let tag = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    for language in Language::ALL.iter().copied() {
        if stores_context(language) {
            assert_eq!(flags.refusal_for(language), None, "{language:?}");
            assert_eq!(tag.refusal_for(language), None, "{language:?}");
            continue;
        }
        let wire = sce_build::forge::codegen_matrix::language_wire_name(language);
        let why = flags
            .refusal_for(language)
            .unwrap_or_else(|| panic!("{language:?} does not store a flag input yet"));
        assert!(why.contains("flag input(s) S, T"), "{why}");
        assert!(
            why.contains(&format!("the {wire} log template does not yet store")),
            "{why}"
        );
        let why = tag
            .refusal_for(language)
            .unwrap_or_else(|| panic!("{language:?} does not store a variant tag yet"));
        assert!(why.contains("the variant's tag"), "{why}");
    }
}

#[test]
fn the_context_of_an_element_is_the_flag_words_and_one_word_for_the_tag() {
    let none = BroadcastLogElementFacts::default();
    assert_eq!((none.flag_words(64), none.context_words(64)), (0, 0));
    let flags = |count: usize, tag: bool| BroadcastLogElementFacts {
        caller_flag_inputs: (0..count).map(|index| format!("f{index}")).collect(),
        caller_variant_tag: tag,
        ..BroadcastLogElementFacts::default()
    };
    // One bit a flag, `ceil(n / w)` words of them, the tag a word after those.
    for (count, tag, bits, flag_words, words) in [
        (1, false, 64, 1, 1),
        (64, false, 64, 1, 1),
        (65, false, 64, 2, 2),
        (1, true, 64, 1, 2),
        (0, true, 64, 0, 1),
        (32, false, 32, 1, 1),
        (33, true, 32, 2, 3),
    ] {
        let facts = flags(count, tag);
        assert_eq!(
            (facts.flag_words(bits), facts.context_words(bits)),
            (flag_words, words),
            "{count} flags, tag {tag}, {bits}-bit words"
        );
    }
}

#[test]
fn an_element_with_nothing_unusual_is_refused_by_no_language() {
    for language in Language::ALL.iter().copied() {
        assert_eq!(
            BroadcastLogElementFacts::default().refusal_for(language),
            None,
            "{language:?}"
        );
    }
}

#[test]
fn a_cbor_entry_that_borrows_is_refused_only_where_the_decoded_form_borrows() {
    let facts = BroadcastLogElementFacts {
        cbor_text_or_bytes_entry: true,
        ..BroadcastLogElementFacts::default()
    };
    let why = facts
        .refusal_for(Language::Rust)
        .expect("Rust decodes a CBOR string to a borrowed view");
    assert!(why.contains("a CBOR map with a string"), "{why}");
    // Python's, Kotlin's and Go's decoded values own their bytes (their arms of
    // `refusal_for` say why: each decode copies a text or byte-string entry out of
    // the buffer it reads), so each holds the element, and a test of its own
    // generates one. The other languages have no template yet, so none has
    // decided that its decoded form borrows; the change that gives one its
    // template says which it is in its arm of `refusal_for`.
    assert_eq!(facts.refusal_for(Language::Go), None);
    for language in Language::ALL
        .iter()
        .copied()
        .filter(|language| *language != Language::Rust && *language != Language::Go)
    {
        assert_eq!(facts.refusal_for(language), None, "{language:?}");
    }
}

// --- Go generates the log over its runtime row ---

/// The generated Go of the log `event_ring`, out of the outputs of a set that
/// generated.
fn generated_go_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    generated_file(outputs, "event_ring.go")
}

/// The options of a Go render with the numbers supplied by hand: the module
/// prefix the element's package is imported by, and the resolution.
fn go_resolution_options(
    retention: Option<u32>,
    element_max_bytes: u64,
    element: BroadcastLogElementFacts,
) -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: options_for(Language::Go).go_module_prefix,
        ..resolution_options(retention, element_max_bytes, element)
    }
}

// --- C11 generates the log over its runtime row ---

/// The generated C header of the log `event_ring`, out of the outputs of a set
/// that generated.
fn generated_c_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    outputs
        .into_iter()
        .flat_map(|(_, output)| output.files)
        .find(|(file, _)| file == "event_ring.h")
        .map(|(_, code)| code)
        .expect("the set generated the log's header")
}

/// The single document `event_ring` generated for C11 with `options`.
fn compile_single_c(
    xml: &str,
    options: &ForgeCompileOptions,
) -> Result<String, Located<ForgeError>> {
    compile_single_as(Language::C11, "event_ring.h", xml, options)
}

#[test]
fn c11_generates_a_log_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; with a retention of
    // 7 the header computes 7 + 1 = 8 slots and 1 + ceil(4 / 8) = 1 + 1 = 2 words
    // an entry, and does not restate either.
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let outputs = compile_documents(
        Language::C11,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .expect("c11 generates a broadcast log");
    let files: Vec<String> = outputs
        .iter()
        .flat_map(|(_, output)| output.files.iter().map(|(file, _)| file.clone()))
        .collect();
    assert!(
        files.iter().any(|file| file == "event_record.h"),
        "the element's header is generated beside the log's: {files:?}"
    );
    let code = generated_c_log(outputs);
    assert_holds(
        &code,
        &[
            r#"/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log") */"#,
            "/* Runtime: sce/forge/broadcast_log.h */",
            "#ifndef SCE_FORGE_EVENT_RING_H",
            "#include <sce/forge/broadcast_log.h>",
            "#include <sce/forge/codec.h>",
            "#include \"event_record.h\"",
            "#define EVENT_RING_RETENTION ((uint32_t)7u)",
            "#define EVENT_RING_SLOTS ((uint32_t)(EVENT_RING_RETENTION + 1u))",
            "#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)4u)",
            "((uint32_t)(1u + EVENT_RING_MAX_ENCODED_BYTES / 8u + \
             (EVENT_RING_MAX_ENCODED_BYTES % 8u != 0u ? 1u : 0u)))",
            "(EVENT_RING_MAX_ENCODED_BYTES > 0u ? EVENT_RING_MAX_ENCODED_BYTES : 1u)",
            r#"#define EVENT_RING_DECLARED_PROGRESS "wait-free""#,
            "typedef struct event_ring {",
            "sce_blog_t core;",
            "uint64_t storage[SCE_BLOG_STORAGE_WORDS(EVENT_RING_SLOTS, EVENT_RING_WORDS)];",
            "} event_ring_t;",
            "typedef sce_blog_publisher_t event_ring_publisher_t;",
            "typedef sce_blog_reader_t event_ring_reader_t;",
            "static inline int event_ring_init(event_ring_t *log)",
            "sce_blog_init(&log->core, log->storage, EVENT_RING_SLOTS, EVENT_RING_WORDS)",
            "static inline event_ring_publish_result_t event_ring_publish(event_ring_publisher_t \
             *publisher, const event_record_t *element)",
            "event_record_encode_to_buf(element, bytes, EVENT_RING_MAX_ENCODED_BYTES, &length)",
            "seq = sce_blog_publish(publisher, entry);",
            "static inline event_ring_read_result_t event_ring_read(const event_ring_t *log, \
             event_ring_reader_t *reader, event_record_t *out)",
            "found = sce_blog_read(&log->core, reader, entry);",
            "event_record_decode(&cursor, out)",
            "EVENT_RING_PUBLISH_OK = 0,",
            "EVENT_RING_PUBLISH_ENCODE_FAILED = 1,",
            "EVENT_RING_PUBLISH_HORIZON = 2",
            "EVENT_RING_READ_ITEM = 0,",
            "EVENT_RING_READ_EMPTY = 1,",
            "EVENT_RING_READ_LAGGED = 2,",
            "EVENT_RING_READ_BAD_LENGTH = 3,",
            "EVENT_RING_READ_DECODE_FAILED = 4",
        ],
    );
    // The codec's own result decides a publish: nothing reaches the runtime's
    // publish when the element did not encode.
    let encode = code
        .find("event_record_encode_to_buf(")
        .expect("the element is encoded");
    let refused = code[encode..]
        .find("result.status = EVENT_RING_PUBLISH_ENCODE_FAILED;")
        .expect("an element the codec refuses is a failed publish");
    let publish = code[encode..]
        .find("sce_blog_publish(publisher, entry)")
        .expect("the entry is published");
    assert!(
        refused < publish,
        "the refusal comes before the runtime's publish:\n{code}"
    );
    // A stored length above the bound is refused before any byte is copied by it.
    let length_check = code
        .find("if (stored > (uint64_t)EVENT_RING_MAX_ENCODED_BYTES) {")
        .expect("the stored length is checked against the bound");
    let bad_length = code[length_check..]
        .find("result.status = EVENT_RING_READ_BAD_LENGTH;")
        .expect("a length above the bound is a refusal");
    let copy = code[length_check..]
        .find("bytes[index] = (uint8_t)(entry[1u + index / 8u]")
        .expect("the entry's bytes are cut by the length");
    assert!(
        bad_length < copy,
        "the refusal comes before the copy the length bounds:\n{code}"
    );
    // The element is decoded only once the runtime has answered with an item,
    // which it does after the copy was checked against the slot.
    let empty = code
        .find("if (found.status == SCE_BLOG_EMPTY) {")
        .expect("an empty read is answered");
    let lagged = code
        .find("if (found.status == SCE_BLOG_LAGGED) {")
        .expect("a lagged read is answered");
    let decode = code
        .find("event_record_decode(&cursor, out)")
        .expect("the element is decoded");
    assert!(
        empty < decode && lagged < decode,
        "decoding comes after the two answers that carry no entry:\n{code}"
    );
    // A no-allocation, no-abort header: nothing allocates, aborts or asserts on
    // data. Its only assertions are the compile-time ones.
    for forbidden in ["malloc", "calloc", "abort", "assert(", "<stdlib.h>"] {
        assert!(
            !code.contains(forbidden),
            "the generated header must not use `{forbidden}`:\n{code}"
        );
    }
}

#[test]
fn go_generates_a_log_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; a slot is then
    // 1 + (4 + 7) / 8 = 1 + 1 = 2 words, which the package computes and does not
    // restate.
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_go_log(
        compile_documents(
            Language::Go,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("go generates a broadcast log"),
    );
    assert_holds(
        &code,
        &[
            r#"// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")"#,
            "// Runtime: github.com/newmassrael/sce-forge-runtime/broadcastlog",
            "package event_ring",
            "\t\"github.com/newmassrael/sce-forge-runtime/broadcastlog\"",
            "\t\"github.com/newmassrael/sce-forge-runtime/codec\"",
            "\tsce_event_record \"github.com/acme/project/generated/event_record\"",
            "const Retention = 7",
            "const Slots = Retention + 1",
            "const MaxEncodedBytes = 4",
            "const Words = 1 + (MaxEncodedBytes+7)/8",
            r#"const DeclaredProgress = "wait-free""#,
            "type EventRing = broadcastlog.Log",
            "func NewEventRing() *EventRing {",
            "return broadcastlog.New(Words, Slots)",
            "func Publish(publisher *broadcastlog.Publisher, element *sce_event_record.EventRecord) (uint64, error) {",
            "var buffer [MaxEncodedBytes]byte",
            "sink := codec.NewBoundedSink(buffer[:])",
            "return publisher.Publish(entry[:]), nil",
            "func Read(log *EventRing, reader *broadcastlog.Reader) (broadcastlog.Read, *sce_event_record.EventRecord, error) {",
            "answer := log.Read(reader, entry[:])",
            "if answer.Kind != broadcastlog.ReadItem {",
            "element, err := sce_event_record.DecodeEventRecord(&cursor)",
        ],
    );
    // An element the codec refuses is returned before anything is published.
    let encode = code
        .find("if err := element.Encode(sink); err != nil {\n\t\treturn 0, err\n\t}")
        .expect("a refused encode returns its error");
    let publish = code
        .find("publisher.Publish(entry[:])")
        .expect("the packed words are published");
    assert!(
        encode < publish,
        "the encode, and its refusal, come before the publish:\n{code}"
    );
    // A stored length above the bound is refused before any slice is cut by it.
    let length_check = code
        .find("if stored > MaxEncodedBytes {")
        .expect("the stored length is checked against the bound");
    let refusal = code[length_check..]
        .find("return answer, nil, codec.ErrLengthOutOfRange")
        .expect("a length above the bound is a codec error");
    let slice = code[length_check..]
        .find("make([]byte, int(stored))")
        .expect("the entry's bytes are cut by the length");
    assert!(
        refusal < slice,
        "the refusal comes before the slice the length cuts:\n{code}"
    );
    assert!(
        !code.contains("Owned"),
        "Go has no owned mirror of an element, and a log needs none:\n{code}"
    );
}

#[test]
fn an_element_that_views_its_buffer_is_decoded_from_a_buffer_of_its_own() {
    // A one-byte head and a payload of at most 64 bytes: 1 + 64 = 65 bytes, so
    // a slot is 1 + (65 + 7) / 8 = 1 + 9 = 10 words. The payload is a slice of
    // the buffer the record was decoded from (`raw[1:]`), which stays valid for as
    // long as the record is held only if nothing else writes the buffer: each
    // read makes one of its own, inside the function.
    let xml = event_ring();
    let codec = codec_with("event_record", "", BOUNDED_FIELDS);
    let code = generated_go_log(
        compile_documents(
            Language::Go,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("go generates a broadcast log over an element that views its buffer"),
    );
    assert_holds(
        &code,
        &[
            "const MaxEncodedBytes = 65",
            "element, err := sce_event_record.DecodeEventRecord(&cursor)",
        ],
    );
    let read = code.find("func Read(").expect("the typed read");
    let buffer = code
        .find("encoded := make([]byte, int(stored))")
        .expect("the decode buffer is made from the stored length");
    assert!(
        read < buffer,
        "the decode buffer is made inside the read, once per entry:\n{code}"
    );
    assert_eq!(
        code.matches("make([]byte").count(),
        1,
        "no other buffer is shared between reads:\n{code}"
    );
}

#[test]
fn go_stores_the_flag_inputs_an_element_asks_its_caller_for() {
    // The codec `event_record` takes the flag input `S` from its caller on `Encode` and
    // `Decode`, as a `byte` after the sink and the cursor. The log stores it in a context
    // word of the entry, takes it on `Publish` as a bool, and gives it back in the item,
    // so that every reader decodes the entry under what it was published under. The
    // element is a `uint8` and an optional `uint8`: 2 bytes, an entry of 1 length word,
    // 1 context word and 1 word of bytes.
    let xml = event_ring();
    let codec = flag_input_codec();
    let code = generated_go_log(
        compile_documents(
            Language::Go,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a log stores the flag input its element asks for"),
    );
    assert_holds(
        &code,
        &[
            "const MaxEncodedBytes = 2",
            "const ContextWords = 1",
            "const Words = 1 + ContextWords + (MaxEncodedBytes+7)/8",
            "type EventRingFlags struct {",
            "\tS bool",
            "type EventRingItem struct {",
            "\tElement *sce_event_record.EventRecord",
            "\tFlags   EventRingFlags",
            "func flagInput(on bool) byte {",
            "func Publish(publisher *broadcastlog.Publisher, element *sce_event_record.EventRecord, flags EventRingFlags) (uint64, error) {",
            "if err := element.Encode(sink, flagInput(flags.S)); err != nil {",
            "if flags.S {",
            "entry[1+0] |= uint64(1) << 0",
            "entry[2+index/8] |= uint64(value) << (8 * (index % 8))",
            "func Read(log *EventRing, reader *broadcastlog.Reader) (broadcastlog.Read, *EventRingItem, error) {",
            "encoded[index] = byte(entry[2+index/8] >> (8 * (index % 8)))",
            "S: (entry[1+0]>>0)&1 == 1,",
            "element, err := sce_event_record.DecodeEventRecord(&cursor, flagInput(flags.S))",
            "return answer, &EventRingItem{",
            "Element: element,",
            "Flags:   flags,",
        ],
    );
    assert!(!code.contains("Tag"), "the element has no tag:\n{code}");
}

#[test]
fn go_stores_the_flags_and_the_tag_of_an_element_in_context_words_of_their_own() {
    // Two flag inputs fit one word; the tag takes the next. The bytes follow the
    // length word and the two context words, so they start at word 3. The codec takes
    // the flags and then the tag on `Decode`, and only the flags on `Encode`.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_as(
        Language::Go,
        "event_ring.go",
        &event_ring(),
        &go_resolution_options(None, 4, facts),
    )
    .expect("a log stores the flags and the tag its element asks for");
    assert_holds(
        &code,
        &[
            "const ContextWords = 2",
            "const Words = 1 + ContextWords + (MaxEncodedBytes+7)/8",
            "\tS bool",
            "\tT bool",
            "\tFlags   EventRingFlags",
            "\tTag     byte",
            "func Publish(publisher *broadcastlog.Publisher, element *sce_event_record.EventRecord, flags EventRingFlags, tag byte) (uint64, error) {",
            "if err := element.Encode(sink, flagInput(flags.S), flagInput(flags.T)); err != nil {",
            "entry[1+0] |= uint64(1) << 0",
            "entry[1+0] |= uint64(1) << 1",
            "entry[1+1] = uint64(tag)",
            "entry[3+index/8] |= uint64(value) << (8 * (index % 8))",
            "encoded[index] = byte(entry[3+index/8] >> (8 * (index % 8)))",
            "S: (entry[1+0]>>0)&1 == 1,",
            "T: (entry[1+0]>>1)&1 == 1,",
            "tag := byte(entry[1+1])",
            "element, err := sce_event_record.DecodeEventRecord(&cursor, flagInput(flags.S), flagInput(flags.T), tag)",
            "Flags:   flags,",
            "Tag:     tag,",
        ],
    );
}

#[test]
fn go_takes_a_tag_alone_as_one_context_word_and_the_signature_takes_no_flags() {
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_as(
        Language::Go,
        "event_ring.go",
        &event_ring(),
        &go_resolution_options(None, 4, facts),
    )
    .expect("a log stores the tag its element asks for");
    assert_holds(
        &code,
        &[
            "const ContextWords = 1",
            "func Publish(publisher *broadcastlog.Publisher, element *sce_event_record.EventRecord, tag byte) (uint64, error) {",
            "\tTag     byte",
            "if err := element.Encode(sink); err != nil {",
            "entry[1+0] = uint64(tag)",
            "entry[2+index/8] |= uint64(value) << (8 * (index % 8))",
            "tag := byte(entry[1+0])",
            "element, err := sce_event_record.DecodeEventRecord(&cursor, tag)",
            "Tag:     tag,",
        ],
    );
    assert!(
        !code.contains("Flags"),
        "the element has no flag input:\n{code}"
    );
    assert!(
        !code.contains("flagInput"),
        "no flag is converted for the codec:\n{code}"
    );
}

#[test]
fn go_keeps_sixty_four_flags_to_a_word_and_starts_the_next_word_at_the_sixty_fifth() {
    // Go's words are 64 bits: flag 63 is the last bit of the first context word and
    // flag 64 the first bit of the second; the bytes then start at word 3.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: (0..65).map(|index| format!("f{index}")).collect(),
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_as(
        Language::Go,
        "event_ring.go",
        &event_ring(),
        &go_resolution_options(None, 4, facts),
    )
    .expect("a log stores 65 flag inputs");
    assert_holds(
        &code,
        &[
            "const ContextWords = 2",
            "entry[1+0] |= uint64(1) << 0",
            "entry[1+0] |= uint64(1) << 63",
            "entry[1+1] |= uint64(1) << 0",
            "F63: (entry[1+0]>>63)&1 == 1,",
            "F64: (entry[1+1]>>0)&1 == 1,",
            "entry[3+index/8] |= uint64(value) << (8 * (index % 8))",
        ],
    );
}

#[test]
fn a_go_element_with_no_caller_input_has_the_entry_and_the_signatures_it_always_had() {
    let code = compile_single_as(
        Language::Go,
        "event_ring.go",
        &event_ring(),
        &go_resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .expect("an element with no caller input is generated");
    assert!(!code.contains("ContextWords"), "{code}");
    assert!(!code.contains("Flags"), "{code}");
    assert!(!code.contains("EventRingItem"), "{code}");
    assert!(!code.contains("flagInput"), "{code}");
    assert_holds(
        &code,
        &[
            "const Words = 1 + (MaxEncodedBytes+7)/8",
            "func Publish(publisher *broadcastlog.Publisher, element *sce_event_record.EventRecord) (uint64, error) {",
            "if err := element.Encode(sink); err != nil {",
            "entry[1+index/8] |= uint64(value) << (8 * (index % 8))",
            "func Read(log *EventRing, reader *broadcastlog.Reader) (broadcastlog.Read, *sce_event_record.EventRecord, error) {",
            "encoded[index] = byte(entry[1+index/8] >> (8 * (index % 8)))",
            "element, err := sce_event_record.DecodeEventRecord(&cursor)",
            "return answer, element, nil",
        ],
    );
}

#[test]
fn a_cbor_element_with_a_string_entry_is_held_by_go_because_its_decode_copies_it() {
    // The same document Rust refuses: a text entry. Go's runtime copies the entry
    // out of the cursor's buffer, so the element owns it and the log may hand it
    // back.
    let xml = event_ring();
    let codec = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let code = generated_go_log(
        compile_documents(
            Language::Go,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a map whose decode copies its strings is held, so go generates the log"),
    );
    assert_holds(
        &code,
        &["element, err := sce_event_record.DecodeEventRecord(&cursor)"],
    );
}

#[test]
fn a_go_log_with_no_module_prefix_is_refused_naming_the_prefix() {
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let located = compile_documents_with(
        Language::Go,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        &ForgeCompileOptions::default(),
        None,
    )
    .err()
    .unwrap_or_else(|| panic!("the element's package has no import path without the prefix"));
    let detail = invalid_config(located);
    assert!(detail.contains("broadcast-log 'event_ring'"), "{detail}");
    assert!(detail.contains("go_module_prefix"), "{detail}");
}

#[test]
fn a_go_single_document_with_no_resolution_is_refused_as_every_language_is() {
    let located = compile_single_as(
        Language::Go,
        "event_ring.go",
        &event_ring(),
        &options_for(Language::Go),
    )
    .err()
    .unwrap_or_else(|| panic!("no number is guessed, so a log with no resolution is refused"));
    let detail = invalid_config(located);
    assert!(detail.contains("broadcast-log 'event_ring'"), "{detail}");
    assert!(detail.contains("the element 'event_record'"), "{detail}");
    assert!(
        detail.contains("ForgeCompileOptions::broadcast_log_resolutions"),
        "{detail}"
    );
}

#[test]
fn c11_states_an_element_that_encodes_to_nothing_with_a_buffer_and_no_packing() {
    // An element of no bytes keeps one length word and has nothing to pack: the
    // header still gives it a buffer to point at and writes no loop over bytes
    // that a one-word entry has no room for.
    let code = compile_single_c(
        &event_ring(),
        &resolution_options(None, 0, BroadcastLogElementFacts::default()),
    )
    .expect("a log of an empty element generates");
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)0u)",
            "uint8_t bytes[EVENT_RING_BUFFER_BYTES] = {0};",
        ],
    );
    assert!(
        !code.contains("for (size_t index"),
        "no byte is packed or unpacked:\n{code}"
    );
}

#[test]
fn c11_packs_and_unpacks_the_bytes_of_an_element_that_has_some() {
    let code = compile_single_c(
        &event_ring(),
        &resolution_options(None, 9, BroadcastLogElementFacts::default()),
    )
    .expect("a log of a nine-byte element generates");
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)9u)",
            "entry[1u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
            "bytes[index] = (uint8_t)(entry[1u + index / 8u] >> (8u * (index % 8u)));",
        ],
    );
}

#[test]
fn c11_refuses_a_single_document_with_no_resolution_naming_what_is_missing() {
    let located = compile_single_c(&event_ring(), &ForgeCompileOptions::default())
        .err()
        .unwrap_or_else(|| panic!("no number is guessed, so a log with no resolution is refused"));
    let detail = invalid_config(located);
    assert!(detail.contains("broadcast-log 'event_ring'"), "{detail}");
    assert!(detail.contains("the element 'event_record'"), "{detail}");
    assert!(
        detail.contains("ForgeCompileOptions::broadcast_log_resolutions"),
        "{detail}"
    );
}

#[test]
fn a_resolved_retention_and_size_become_the_go_packages_constants() {
    // 9 bytes encode to words of 1 + (9 + 7) / 8 = 1 + 2 = 3, computed by the
    // package from the constant below.
    let code = compile_single_as(
        Language::Go,
        "event_ring.go",
        &deploy_key_ring(),
        &go_resolution_options(Some(15), 9, BroadcastLogElementFacts::default()),
    )
    .expect("a resolved log generates");
    assert_holds(
        &code,
        &["const Retention = 15", "const MaxEncodedBytes = 9"],
    );
}

#[test]
fn c11_refuses_what_the_shared_judgement_refuses_in_the_shared_words() {
    // An unresolved deploy key, on the retention row.
    let xml = deploy_key_ring();
    let located = compile_single_c(
        &xml,
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("an unresolved retention is not guessed"));
    assert!(row_text(&located, &xml).contains("<sce:retention"));
    assert!(invalid_config(located).contains("target machine"));

    // A retention of nothing, on the retention row.
    let located = compile_single_c(
        &xml,
        &resolution_options(Some(0), 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("a log retains at least one entry"));
    assert!(row_text(&located, &xml).contains("<sce:retention"));
    assert!(invalid_config(located).contains("the retention resolves to 0"));

    // No element is left that the C11 template cannot hold: a flag input and a
    // variant's tag are stored with the entry, and a CBOR text entry is copied into
    // the struct, so the facts that name them are accepted and generate a header.
    let element_xml = event_ring();
    for facts in [
        BroadcastLogElementFacts {
            caller_variant_tag: true,
            ..BroadcastLogElementFacts::default()
        },
        BroadcastLogElementFacts {
            caller_flag_inputs: vec!["S".to_string()],
            ..BroadcastLogElementFacts::default()
        },
        BroadcastLogElementFacts {
            cbor_text_or_bytes_entry: true,
            ..BroadcastLogElementFacts::default()
        },
    ] {
        compile_single_c(&element_xml, &resolution_options(None, 4, facts.clone()))
            .unwrap_or_else(|located| panic!("C11 holds {facts:?}, got {located:?}"));
    }
}

#[test]
fn c11_stores_the_flag_inputs_an_element_asks_its_caller_for() {
    // The codec `event_record` takes the flag input `S` from its caller on `encode` and
    // `decode`, as a `uint8_t` after the writer or the cursor. The log stores it in a
    // context word of the entry, takes it on `publish` as a `bool` of a struct, and gives
    // it back in the read result, so that every reader decodes the entry under what it was
    // published under. The element is a `uint8` and an optional `uint8`: 2 bytes, an entry
    // of 1 length word, 1 context word and 1 word of bytes.
    let xml = event_ring();
    let codec = flag_input_codec();
    let code = generated_c_log(
        compile_documents(
            Language::C11,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a log stores the flag input its element asks for"),
    );
    assert_holds(
        &code,
        &[
            "#include <stdbool.h>",
            "#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)2u)",
            "#define EVENT_RING_CONTEXT_WORDS ((uint32_t)1u)",
            "((uint32_t)(1u + EVENT_RING_CONTEXT_WORDS + EVENT_RING_MAX_ENCODED_BYTES / 8u + \
             (EVENT_RING_MAX_ENCODED_BYTES % 8u != 0u ? 1u : 0u)))",
            "typedef struct {\n    bool s;\n} event_ring_flags_t;",
            "static inline uint8_t event_ring_flag_input(bool on)",
            "    event_ring_flags_t flags;\n} event_ring_read_result_t;",
            "static inline event_ring_publish_result_t event_ring_publish(event_ring_publisher_t \
             *publisher, const event_record_t *element, const event_ring_flags_t *flags)",
            "event_record_encode_to_buf(element, bytes, EVENT_RING_MAX_ENCODED_BYTES, &length, \
             event_ring_flag_input(flags->s))",
            "if (flags->s) {\n        entry[1u + 0u] |= UINT64_C(1) << 0u;\n    }",
            "entry[2u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
            "result.flags.s = false;",
            "bytes[index] = (uint8_t)(entry[2u + index / 8u] >> (8u * (index % 8u)));",
            "result.flags.s = ((entry[1u + 0u] >> 0u) & UINT64_C(1)) != 0u;",
            "event_record_decode(&cursor, out, event_ring_flag_input(result.flags.s))",
        ],
    );
    assert!(
        !code.contains("uint8_t tag") && !code.contains("result.tag"),
        "the element has no tag:\n{code}"
    );
    // The flag is stored before the bytes are packed, and the bits are read back before
    // the decode that is given them.
    let stored = code
        .find("if (flags->s) {")
        .expect("the flag is stored in the entry");
    let packed = code
        .find("entry[2u + index / 8u] |=")
        .expect("the bytes are packed");
    assert!(stored < packed, "the flag word comes first:\n{code}");
    let read_back = code
        .find("result.flags.s = ((entry[")
        .expect("the flag is read back from the entry");
    let decoded = code
        .find("event_record_decode(&cursor, out")
        .expect("the element is decoded");
    assert!(
        read_back < decoded,
        "the decode is given the flag stored with the entry:\n{code}"
    );
}

#[test]
fn c11_stores_the_flags_and_the_tag_of_an_element_in_context_words_of_their_own() {
    // Two flag inputs fit one word; the tag takes the next. The bytes follow the
    // length word and the two context words, so they start at word 3. The codec takes
    // the flags and then the tag on `decode`, and only the flags on `encode`.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_c(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the flags and the tag its element asks for");
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_CONTEXT_WORDS ((uint32_t)2u)",
            "((uint32_t)(1u + EVENT_RING_CONTEXT_WORDS + EVENT_RING_MAX_ENCODED_BYTES / 8u + \
             (EVENT_RING_MAX_ENCODED_BYTES % 8u != 0u ? 1u : 0u)))",
            "typedef struct {\n    bool s;\n    bool t;\n} event_ring_flags_t;",
            "    event_ring_flags_t flags;\n",
            "    uint8_t tag;\n} event_ring_read_result_t;",
            "static inline event_ring_publish_result_t event_ring_publish(event_ring_publisher_t \
             *publisher, const event_record_t *element, const event_ring_flags_t *flags, \
             uint8_t tag)",
            "event_record_encode_to_buf(element, bytes, EVENT_RING_MAX_ENCODED_BYTES, &length, \
             event_ring_flag_input(flags->s), event_ring_flag_input(flags->t))",
            "if (flags->s) {\n        entry[1u + 0u] |= UINT64_C(1) << 0u;\n    }",
            "if (flags->t) {\n        entry[1u + 0u] |= UINT64_C(1) << 1u;\n    }",
            "entry[1u + 1u] = (uint64_t)tag;",
            "entry[3u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
            "result.flags.s = false;",
            "result.flags.t = false;",
            "result.tag = 0u;",
            "bytes[index] = (uint8_t)(entry[3u + index / 8u] >> (8u * (index % 8u)));",
            "result.flags.s = ((entry[1u + 0u] >> 0u) & UINT64_C(1)) != 0u;",
            "result.flags.t = ((entry[1u + 0u] >> 1u) & UINT64_C(1)) != 0u;",
            "result.tag = (uint8_t)(entry[1u + 1u] & UINT64_C(0xFF));",
            "event_record_decode(&cursor, out, event_ring_flag_input(result.flags.s), \
             event_ring_flag_input(result.flags.t), result.tag)",
        ],
    );
}

#[test]
fn c11_takes_a_tag_alone_as_one_context_word_and_the_signature_takes_no_flags() {
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_c(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the tag its element asks for");
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_CONTEXT_WORDS ((uint32_t)1u)",
            "    uint8_t tag;\n} event_ring_read_result_t;",
            "static inline event_ring_publish_result_t event_ring_publish(event_ring_publisher_t \
             *publisher, const event_record_t *element, uint8_t tag)",
            "event_record_encode_to_buf(element, bytes, EVENT_RING_MAX_ENCODED_BYTES, &length);",
            "entry[1u + 0u] = (uint64_t)tag;",
            "entry[2u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
            "result.tag = (uint8_t)(entry[1u + 0u] & UINT64_C(0xFF));",
            "event_record_decode(&cursor, out, result.tag)",
        ],
    );
    assert!(
        !code.contains("flags") && !code.contains("stdbool"),
        "the element has no flag input:\n{code}"
    );
}

#[test]
fn c11_keeps_sixty_four_flags_to_a_word_and_starts_the_next_word_at_the_sixty_fifth() {
    // C11's context words are `uint64_t`: flag 63 is the last bit of the first context
    // word (written with `UINT64_C(1) << 63u`, which a `uint64_t` holds whole) and flag
    // 64 the first bit of the second.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: (0..65).map(|index| format!("f{index}")).collect(),
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_c(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores 65 flag inputs");
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_CONTEXT_WORDS ((uint32_t)2u)",
            "    bool f0;\n",
            "    bool f64;\n} event_ring_flags_t;",
            "if (flags->f0) {\n        entry[1u + 0u] |= UINT64_C(1) << 0u;\n    }",
            "if (flags->f63) {\n        entry[1u + 0u] |= UINT64_C(1) << 63u;\n    }",
            "if (flags->f64) {\n        entry[1u + 1u] |= UINT64_C(1) << 0u;\n    }",
            "result.flags.f63 = ((entry[1u + 0u] >> 63u) & UINT64_C(1)) != 0u;",
            "result.flags.f64 = ((entry[1u + 1u] >> 0u) & UINT64_C(1)) != 0u;",
            "entry[3u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
        ],
    );
}

#[test]
fn a_c11_element_with_no_caller_input_has_the_entry_and_the_signatures_it_always_had() {
    let code = compile_single_c(
        &event_ring(),
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .expect("an element with no caller input is generated");
    assert!(!code.contains("CONTEXT_WORDS"), "{code}");
    assert!(!code.contains("flags"), "{code}");
    assert!(!code.contains("stdbool"), "{code}");
    assert!(!code.contains("result.tag"), "{code}");
    assert_holds(
        &code,
        &[
            "((uint32_t)(1u + EVENT_RING_MAX_ENCODED_BYTES / 8u + \
             (EVENT_RING_MAX_ENCODED_BYTES % 8u != 0u ? 1u : 0u)))",
            "static inline event_ring_publish_result_t event_ring_publish(event_ring_publisher_t \
             *publisher, const event_record_t *element)",
            "event_record_encode_to_buf(element, bytes, EVENT_RING_MAX_ENCODED_BYTES, &length);",
            "entry[1u + index / 8u] |= (uint64_t)bytes[index] << (8u * (index % 8u));",
            "bytes[index] = (uint8_t)(entry[1u + index / 8u] >> (8u * (index % 8u)));",
            "event_record_decode(&cursor, out);",
            "    sce_forge_codec_status_t codec_status;\n} event_ring_read_result_t;",
        ],
    );
}

#[test]
fn c11_counts_the_context_words_in_the_words_it_refuses_past_32_bits() {
    // words = 1 + context + ceil(bytes / 8) must be at most 4294967295. With one
    // context word the most bytes that fit are 8 * (4294967295 - 2) = 34359738344;
    // one more byte needs a word more, 4294967296.
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let element_xml = event_ring();
    compile_single_c(
        &element_xml,
        &resolution_options(None, 34_359_738_344, facts.clone()),
    )
    .expect("the widest entry with one context word generates");
    let located = compile_single_c(
        &element_xml,
        &resolution_options(None, 34_359_738_345, facts),
    )
    .err()
    .unwrap_or_else(|| panic!("one byte more than 32 bits of words hold is refused"));
    let detail = invalid_config(located);
    assert!(
        detail.contains("which is 4294967296 64-bit words"),
        "{detail}"
    );
}

#[test]
fn c11_holds_a_cbor_element_with_text_because_its_struct_owns_the_bytes() {
    // The C codec copies a CBOR text entry into a `char name[N]` of the struct, so
    // the decoded value outlives the buffer it was read from: the log holds an
    // element the Rust template refuses (its decoded form borrows the buffer).
    let facts = BroadcastLogElementFacts {
        cbor_text_or_bytes_entry: true,
        ..BroadcastLogElementFacts::default()
    };
    assert_eq!(facts.refusal_for(Language::C11), None);
    assert!(facts.refusal_for(Language::Rust).is_some());

    let xml = event_ring();
    let borrowing = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let code = generated_c_log(
        compile_documents(
            Language::C11,
            &[
                ("event_ring.scxml", &xml),
                ("event_record.scxml", &borrowing),
            ],
        )
        .expect("c11 generates a log over an element that holds text"),
    );
    assert_holds(&code, &["#include \"event_record.h\""]);
}

#[test]
fn c11_counts_slots_and_words_in_32_bits_and_refuses_what_does_not_fit() {
    // slots = retention + 1 must be at most u32::MAX = 4294967295, so the largest
    // retention is 4294967294 (4294967294 + 1 = 4294967295) and 4294967295 is one
    // too many (4294967295 + 1 = 4294967296).
    let xml = deploy_key_ring();
    let code = compile_single_c(
        &xml,
        &resolution_options(Some(4_294_967_294), 4, BroadcastLogElementFacts::default()),
    )
    .expect("the largest retention the runtime can count generates");
    assert!(
        code.contains("#define EVENT_RING_RETENTION ((uint32_t)4294967294u)"),
        "{code}"
    );
    let located = compile_single_c(
        &xml,
        &resolution_options(Some(4_294_967_295), 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("one slot more than 32 bits count is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    let detail = invalid_config(located);
    assert!(
        detail.contains("the retention resolves to 4294967295"),
        "{detail}"
    );
    assert!(
        detail.contains("counts a log's slots in 32 bits"),
        "{detail}"
    );
    assert!(
        detail.contains("Declare a retention of at most 4294967294"),
        "{detail}"
    );

    // words = 1 + ceil(bytes / 8) must be at most 4294967295. The most bytes that
    // fit are 8 * (4294967295 - 1) = 8 * 4294967294 = 34359738352 (a whole number
    // of words); one more byte needs a word more, 4294967296, which does not.
    let element_xml = event_ring();
    let code = compile_single_c(
        &element_xml,
        &resolution_options(None, 34_359_738_352, BroadcastLogElementFacts::default()),
    )
    .expect("the widest entry the runtime can count generates");
    assert!(
        code.contains("#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)34359738352u)"),
        "{code}"
    );
    let located = compile_single_c(
        &element_xml,
        &resolution_options(None, 34_359_738_353, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("one byte more than 32 bits of words hold is refused"));
    assert!(
        row_text(&located, &element_xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    let detail = invalid_config(located);
    assert!(
        detail.contains("encodes to at most 34359738353 bytes"),
        "{detail}"
    );
    assert!(
        detail.contains("which is 4294967296 64-bit words"),
        "{detail}"
    );
}

#[test]
fn c11_reads_a_deploy_key_retention_from_the_target_machines_limits() {
    // The element is one `uint32`: 4 bytes.
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let code = generated_c_log(
        compile_documents_with(
            Language::C11,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
            &options,
            Some(&cfg),
        )
        .expect("the deploy resolves the key"),
    );
    assert_holds(
        &code,
        &[
            "#define EVENT_RING_RETENTION ((uint32_t)15u)",
            "#define EVENT_RING_MAX_ENCODED_BYTES ((size_t)4u)",
        ],
    );
}

#[test]
fn c11_and_rust_refuse_one_log_with_one_message() {
    // The two renderers judge a log through one function, so the refusal of a log
    // that neither can generate differs only in the name of the language. A retention
    // of 0 is refused by both, with a text that names no language.
    let xml = deploy_key_ring();
    let options = resolution_options(Some(0), 4, BroadcastLogElementFacts::default());
    let rust = invalid_config(
        compile_single(&xml, &options)
            .err()
            .unwrap_or_else(|| panic!("rust refuses the element")),
    );
    let c = invalid_config(
        compile_single_c(&xml, &options)
            .err()
            .unwrap_or_else(|| panic!("c11 refuses the element")),
    );
    assert_eq!(rust.replace("Rust", "C11"), c);
}

// --- C++ generates the log over its runtime row ---

/// The generated C++ of the log `event_ring`, out of the outputs of a set that
/// generated.
fn generated_cpp_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    generated_file(outputs, "event_ring.h")
}

/// The generated C++ of the single document `xml`, compiled with `options`.
fn compile_single_cpp(
    xml: &str,
    options: &ForgeCompileOptions,
) -> Result<String, Located<ForgeError>> {
    compile_single_as(Language::Cpp, "event_ring.h", xml, options)
}

#[test]
fn cpp_generates_a_header_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; an entry is then
    // 1 + (4 + 7) / 8 = 1 + 1 = 2 words, which the header computes from the
    // constant and does not restate.
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_cpp_log(
        compile_documents(
            Language::Cpp,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("c++ generates a broadcast log"),
    );
    assert_holds(
        &code,
        &[
            r#"// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")"#,
            "// Runtime: sce/forge/broadcast_log.h",
            "#ifndef SCE_FORGE_EVENT_RING_H",
            r#"#include "sce/forge/broadcast_log.h""#,
            r#"#include "sce/forge/codec.h""#,
            r#"#include "event_record.h""#,
            "namespace SCE::Generated::EventRing {",
            "using EventRecordType = ::SCE::Generated::EventRecord::EventRecord;",
            "inline constexpr std::size_t RETENTION = 7;",
            "inline constexpr std::size_t SLOTS = RETENTION + 1;",
            "inline constexpr std::size_t MAX_ENCODED_BYTES = 4;",
            "inline constexpr std::size_t WORDS = 1 + (MAX_ENCODED_BYTES + 7) / 8;",
            r#"inline constexpr std::string_view DECLARED_PROGRESS = "wait-free";"#,
            "using EventRing = ::SCE::Forge::Broadcast::BroadcastLog<WORDS, SLOTS>;",
            "static_assert(EventRing::kRetention == RETENTION",
            "using EventRingPublisher = EventRing::Publisher;",
            "using EventRingReader = ::SCE::Forge::Broadcast::Reader;",
            "struct EventRingPublished {",
            "struct EventRingItem {",
            "EventRecordType element;",
            "struct EventRingEmpty {};",
            "struct EventRingLagged {",
            "struct EventRingMalformed {",
            "using EventRingRead = std::variant<EventRingItem, EventRingEmpty, EventRingLagged, EventRingMalformed>;",
            "[[nodiscard]] inline EventRingPublished publish(EventRingPublisher &publisher, const EventRecordType &element) noexcept {",
            "::SCE::Forge::SpanSink sink(bytes.data(), MAX_ENCODED_BYTES);",
            "[[nodiscard]] inline EventRingRead read(const EventRing &log, EventRingReader &reader) {",
            "std::optional<EventRecordType> element = EventRecordType::decode(cursor);",
            "#endif  // SCE_FORGE_EVENT_RING_H",
        ],
    );
    // An element past the bound is refused by the codec before anything is
    // published: the refusal returns ahead of the call that publishes.
    let encode = code
        .find("element.encode(sink)")
        .expect("the element is encoded through the bounded sink");
    let refusal = code[encode..]
        .find("return EventRingPublished{refused, ::SCE::Forge::Broadcast::kNoSequence};")
        .expect("a refused element publishes nothing");
    let publish = code[encode..]
        .find("publisher.publish(entry)")
        .expect("the packed entry is handed to the runtime");
    assert!(
        refusal < publish,
        "the refusal comes before the publish:\n{code}"
    );
    // A stored length above the bound is refused before any byte is cut by it.
    let length_check = code
        .find("if (stored > MAX_ENCODED_BYTES) {")
        .expect("the stored length is checked against the bound");
    let refusal = code[length_check..]
        .find("return EventRingMalformed{answer.seq};")
        .expect("a length above the bound is a malformed entry");
    let cut = code[length_check..]
        .find(
            "bytes[index] = static_cast<std::uint8_t>(entry[1 + index / 8] >> (8 * (index % 8)));",
        )
        .expect("the entry's bytes are cut by the length");
    assert!(
        refusal < cut,
        "the refusal comes before the copy the length bounds:\n{code}"
    );
}

#[test]
fn the_names_the_cpp_header_calls_are_the_ones_the_runtime_declares() {
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_cpp_log(
        compile_documents(
            Language::Cpp,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("c++ generates a broadcast log"),
    );
    let include = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../backends/cpp/forge-runtime/include/sce/forge");
    // What the header writes, the file that must declare it, and the declaration.
    let uses = [
        (
            "::SCE::Forge::Broadcast::BroadcastLog<WORDS, SLOTS>",
            "broadcast_log.h",
            "template <std::size_t W, std::size_t Slots> class BroadcastLog {",
        ),
        (
            "::SCE::Forge::Broadcast::kNoSequence",
            "broadcast_log.h",
            "inline constexpr std::uint64_t kNoSequence = UINT64_MAX;",
        ),
        (
            "::SCE::Forge::Broadcast::ReadKind::Empty",
            "broadcast_log.h",
            "enum class ReadKind : std::uint8_t {",
        ),
        (
            "::SCE::Forge::Broadcast::Reader",
            "broadcast_log.h",
            "class Reader {",
        ),
        (
            "EventRing::Publisher",
            "broadcast_log.h",
            "class Publisher {",
        ),
        (
            "EventRing::Entry",
            "broadcast_log.h",
            "using Entry = std::array<std::uint64_t, W>;",
        ),
        (
            "EventRing::kRetention",
            "broadcast_log.h",
            "static constexpr std::uint64_t kRetention = Slots - 1;",
        ),
        (
            "publisher.publish(entry)",
            "broadcast_log.h",
            "std::uint64_t publish(const Entry &entry) noexcept {",
        ),
        (
            "log.read(reader, entry)",
            "broadcast_log.h",
            "Read read(Reader &reader, Entry &out) const noexcept {",
        ),
        (
            "::SCE::Forge::SpanSink sink(",
            "codec.h",
            "class SpanSink final : public SceSink {",
        ),
        (
            "::SCE::Forge::SceCursor cursor(",
            "codec.h",
            "class SceCursor {",
        ),
        (
            "::SCE::Forge::CodecError",
            "codec.h",
            "enum class CodecError : std::uint8_t {",
        ),
        (
            "sink.position()",
            "codec.h",
            "[[nodiscard]] virtual std::size_t position() const noexcept = 0;",
        ),
    ];
    for (written, file, declaration) in uses {
        assert!(
            code.contains(written),
            "the header no longer writes `{written}`:\n{code}"
        );
        let runtime = fs::read_to_string(include.join(file))
            .unwrap_or_else(|e| panic!("read the runtime's {file}: {e}"));
        assert!(
            runtime.contains(declaration),
            "the runtime's {file} no longer declares `{declaration}`, which the header's \
             `{written}` names"
        );
    }
}

#[test]
fn cpp_states_the_resolved_retention_and_size_as_constants() {
    // 9 bytes encode to words of 1 + (9 + 7) / 8 = 1 + 2 = 3, computed by the
    // header from the constant below.
    let code = compile_single_cpp(
        &deploy_key_ring(),
        &resolution_options(Some(15), 9, BroadcastLogElementFacts::default()),
    )
    .expect("a resolved log generates");
    assert_holds(
        &code,
        &[
            "inline constexpr std::size_t RETENTION = 15;",
            "inline constexpr std::size_t MAX_ENCODED_BYTES = 9;",
        ],
    );
    // A literal retention is the document's whatever the resolution says.
    let code = compile_single_cpp(
        &event_ring(),
        &resolution_options(Some(99), 4, BroadcastLogElementFacts::default()),
    )
    .expect("a literal needs no resolution of its own");
    assert!(
        code.contains("inline constexpr std::size_t RETENTION = 7;"),
        "{code}"
    );
    assert!(!code.contains("= 99;"), "{code}");
}

#[test]
fn cpp_reads_a_deploy_key_retention_from_the_target_machines_limits() {
    // The element is one `uint32`: 4 bytes. The limit is 15 entries.
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let code = generated_cpp_log(
        compile_documents_with(
            Language::Cpp,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
            &options,
            Some(&cfg),
        )
        .expect("the deploy resolves the key"),
    );
    assert_holds(
        &code,
        &[
            "inline constexpr std::size_t RETENTION = 15;",
            "inline constexpr std::size_t MAX_ENCODED_BYTES = 4;",
        ],
    );
}

#[test]
fn cpp_refuses_an_element_that_asks_the_caller_for_more_and_names_its_template() {
    // The C++ codec's `decode` takes the flag input as a parameter, which a log
    // has none to give.
    let xml = event_ring();
    let codec = flag_input_codec();
    let located = compile_documents(
        Language::Cpp,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .err()
    .unwrap_or_else(|| panic!("a log has no flag input to give, so the set is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    let detail = invalid_config(located);
    assert!(
        detail.contains("the element 'event_record' cannot be stored by the C++ template"),
        "{detail}"
    );
    assert!(detail.contains("flag input(s) S"), "{detail}");
}

#[test]
fn cpp_holds_a_cbor_element_with_a_string_entry_because_its_decoded_value_owns_the_text() {
    let xml = event_ring();
    let facts = BroadcastLogElementFacts {
        cbor_text_or_bytes_entry: true,
        ..BroadcastLogElementFacts::default()
    };
    assert_eq!(
        facts.refusal_for(Language::Cpp),
        None,
        "the C++ CBOR reader returns a std::string and a std::vector, which own their bytes"
    );
    assert!(
        facts.refusal_for(Language::Rust).is_some(),
        "Rust's decoded form borrows, so the same element is refused there"
    );
    // What a caller must give is refused whatever the entries are.
    let tag = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..facts
    };
    assert!(tag.refusal_for(Language::Cpp).is_some());

    // The same element, as the document set sees it: the string entry is read
    // from the codec and the log is generated where Rust refuses it.
    let codec = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let code = generated_cpp_log(
        compile_documents(
            Language::Cpp,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a string entry is owned by the C++ value, so c++ generates the log"),
    );
    assert_holds(
        &code,
        &["using EventRecordType = ::SCE::Generated::EventRecord::EventRecord;"],
    );
}

// --- The set resolves a deploy key against the machine it is generated for ---

#[test]
fn a_deploy_key_retention_is_read_from_the_target_machines_limits() {
    // The element is one `uint32`: 4 bytes.
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let code = generated_log(
        compile_documents_with(
            Language::Rust,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
            &options,
            Some(&cfg),
        )
        .expect("the deploy resolves the key"),
    );
    assert_holds(
        &code,
        &[
            "pub const RETENTION: usize = 15;",
            "pub const MAX_ENCODED_BYTES: usize = 4;",
        ],
    );
}

#[test]
fn a_deploy_key_with_no_target_machine_is_refused_and_not_guessed() {
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let located = compile_documents_with(
        Language::Rust,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        &ForgeCompileOptions::default(),
        Some(&cfg),
    )
    .err()
    .unwrap_or_else(|| panic!("a deploy with no target machine resolves nothing"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    assert!(
        invalid_config(located).contains("target machine"),
        "the refusal says what is missing"
    );
}

#[test]
fn a_deploy_key_naming_an_undeclared_limit_is_refused_with_the_limits_there_are() {
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          other_limit: 3");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let located = compile_documents_with(
        Language::Rust,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        &options,
        Some(&cfg),
    )
    .err()
    .unwrap_or_else(|| panic!("the key names a limit the machine does not declare"));
    let detail = invalid_config(located);
    assert!(
        detail.contains("declared limits today: other_limit"),
        "{detail}"
    );
}

// --- Python generates the log over its runtime row ---

/// The log `event_ring` declaring the progress the Python backend can give: one
/// ring under one lock, which is `blocking` and no more.
fn blocking_ring() -> String {
    simple_doc(
        "event_ring",
        "one",
        "blocking",
        r#"<sce:retention entries="7"/>"#,
    )
}

/// The generated Python of the log `event_ring`, out of the outputs of a set
/// that generated.
fn generated_python_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    outputs
        .into_iter()
        .flat_map(|(_, output)| output.files)
        .find(|(file, _)| file == "event_ring.py")
        .map(|(_, code)| code)
        .expect("the set generated the log's module")
}

/// The generated Python of the single document `xml`, compiled with `options`.
fn compile_single_python(
    xml: &str,
    options: &ForgeCompileOptions,
) -> Result<String, Located<ForgeError>> {
    compile_forge_with_imports(
        xml,
        label("event_ring"),
        Language::Python,
        Path::new("."),
        options,
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .find(|(file, _)| file == "event_ring.py")
            .map(|(_, code)| code)
            .expect("the log's module")
    })
}

/// The detail of the refusal `language` gives the single document `xml` under
/// `options`, which must be a `generate/invalid-config` one.
fn single_refusal(language: Language, xml: &str, options: &ForgeCompileOptions) -> String {
    let located =
        compile_forge_with_imports(xml, label("event_ring"), language, Path::new("."), options)
            .err()
            .unwrap_or_else(|| panic!("{language:?}: the document is refused"));
    invalid_config(located)
}

#[test]
fn python_generates_a_log_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; an entry is then
    // 1 + ceil(4 / 8) = 1 + (4 + 7) // 8 = 1 + 1 = 2 words, which the module
    // computes and does not restate.
    let xml = blocking_ring();
    let codec = codec_doc("event_record");
    let code = generated_python_log(
        compile_documents(
            Language::Python,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("python generates a broadcast log"),
    );
    assert_holds(
        &code,
        &[
            r#"# SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")"#,
            "# Runtime: sce_forge_runtime.broadcast_log",
            "from sce_forge_runtime import broadcast_log as runtime",
            "from sce_forge_runtime import codec as runtime_codec",
            "from .event_record import EventRecord",
            "RETENTION: Final[int] = 7",
            "SLOTS: Final[int] = RETENTION + 1",
            "MAX_ENCODED_BYTES: Final[int] = 4",
            "WORDS: Final[int] = 1 + (MAX_ENCODED_BYTES + 7) // 8",
            r#"DECLARED_PROGRESS: Final[str] = "blocking""#,
            "EventRing = runtime.BroadcastLog",
            "def new_event_ring() -> EventRing:",
            "return runtime.BroadcastLog(WORDS, SLOTS)",
            "class EventRingItem:",
            "element: EventRecord",
            "EventRingRead = Union[EventRingItem, runtime.Empty, runtime.Lagged]",
            "def publish(publisher: runtime.Publisher, element: EventRecord) -> int:",
            "sink = runtime_codec.MemoryviewSink(memoryview(buffer))",
            "element.encode(sink)",
            r#"padded = bytes(buffer[:length]).ljust(8 * (WORDS - 1), b"\x00")"#,
            "return publisher.publish(entry)",
            "def read(log: EventRing, reader: runtime.Reader) -> EventRingRead:",
            "if not isinstance(found, runtime.Item):",
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]))",
            "return EventRingItem(found.seq, element)",
        ],
    );
    // A stored length above the bound is refused before any bytes are cut by it.
    let length_check = code
        .find("if stored > MAX_ENCODED_BYTES:")
        .expect("the stored length is checked against the bound");
    let refusal = code[length_check..]
        .find("raise runtime_codec.LengthOutOfRange(")
        .expect("a length above the bound is a codec error");
    let cut = code[length_check..]
        .find("packed[:stored]")
        .expect("the entry's bytes are cut by the length");
    assert!(
        refusal < cut,
        "the refusal comes before the cut the length makes:\n{code}"
    );
    // Python's decode returns values that own their bytes, so there is no owned
    // mirror to name and no copy to make.
    assert!(!code.contains("Owned"), "{code}");
}

#[test]
fn python_sizes_an_entry_from_an_element_with_a_payload() {
    // A one-byte head and a payload of at most 64 bytes: 1 + 64 = 65 bytes, so
    // an entry is 1 + ceil(65 / 8) = 1 + (65 + 7) // 8 = 1 + 9 = 10 words.
    let xml = blocking_ring();
    let codec = codec_with("event_record", "", BOUNDED_FIELDS);
    let code = generated_python_log(
        compile_documents(
            Language::Python,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("python generates a broadcast log over a payload element"),
    );
    assert_holds(
        &code,
        &[
            "MAX_ENCODED_BYTES: Final[int] = 65",
            "from .event_record import EventRecord",
        ],
    );
}

#[test]
fn a_progress_python_cannot_give_is_refused_for_a_document_set_on_the_progress_row() {
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let located = compile_documents(
        Language::Python,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .err()
    .unwrap_or_else(|| {
        panic!("python gives blocking at most, so a set declaring wait-free is refused")
    });
    assert!(
        row_text(&located, &xml).contains("<sce:progress>"),
        "the refusal is placed on the progress row"
    );
    match generate_error(located) {
        GenerateError::BroadcastLogProgressUnreachableOnBackend {
            log_name,
            declared,
            reachable,
            language,
            ..
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!(declared, "wait-free");
            assert_eq!(reachable, "blocking");
            assert_eq!(language, "python");
        }
        other => panic!("expected BroadcastLogProgressUnreachableOnBackend, got {other:?}"),
    }
}

#[test]
fn a_cbor_text_entry_is_held_by_python_because_its_decoded_value_owns_its_bytes() {
    // The element Rust refuses (a string entry decodes to a view of the buffer):
    // Python's decode copies, so the log holds it.
    let xml = blocking_ring();
    let codec = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let code = generated_python_log(
        compile_documents(
            Language::Python,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a map with a text entry owns its bytes in Python, so python generates the log"),
    );
    assert_holds(
        &code,
        &[
            "from .event_record import EventRecord",
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]))",
        ],
    );
}

#[test]
fn python_stores_the_flag_inputs_an_element_asks_its_caller_for() {
    // The codec `event_record` takes the flag input `S` from its caller on `encode` and
    // `decode`. The log stores it in a context word of the entry, takes it on `publish`
    // as a bool, and gives it back with the item, so that every reader decodes the entry
    // under what it was published under. The element is a `uint8` and an optional
    // `uint8`: 2 bytes, an entry of 1 length word, 1 context word and 1 word of bytes.
    let xml = blocking_ring();
    let codec = flag_input_codec();
    let code = generated_python_log(
        compile_documents(
            Language::Python,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a log stores the flag input its element asks for"),
    );
    assert_holds(
        &code,
        &[
            "MAX_ENCODED_BYTES: Final[int] = 2",
            "CONTEXT_WORDS: Final[int] = 1",
            "WORDS: Final[int] = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) // 8",
            "class EventRingFlags:",
            "s: bool = False",
            "flags: EventRingFlags\n",
            "def publish(publisher: runtime.Publisher, element: EventRecord, flags: EventRingFlags) -> int:",
            "element.encode(sink, int(flags.s))",
            r#"padded = bytes(buffer[:length]).ljust(8 * (WORDS - 2), b"\x00")"#,
            "entry.extend([0] * CONTEXT_WORDS)",
            "entry[1 + 0] |= int(flags.s) << 0",
            "s=bool((entry[1 + 0] >> 0) & 1),",
            r#"for word in entry[2:])"#,
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]), int(flags.s))",
            "return EventRingItem(found.seq, element, flags)",
        ],
    );
    assert!(
        !code.contains("tag: int"),
        "the element has no tag:\n{code}"
    );
    assert!(
        !code.contains("ValueError"),
        "a flag is a bool and has no range to refuse:\n{code}"
    );
}

#[test]
fn python_stores_the_flags_and_the_tag_of_an_element_in_context_words_of_their_own() {
    // Two flag inputs fit one word; the tag takes the next. The bytes follow the
    // length word and the two context words, so they start at word 3.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_python(&blocking_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the flags and the tag its element asks for");
    assert_holds(
        &code,
        &[
            "CONTEXT_WORDS: Final[int] = 2",
            "WORDS: Final[int] = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) // 8",
            "s: bool = False",
            "t: bool = False",
            "flags: EventRingFlags\n    tag: int\n",
            "def publish(publisher: runtime.Publisher, element: EventRecord, flags: EventRingFlags, tag: int) -> int:",
            "if not 0 <= tag <= 0xFF:",
            "raise ValueError(",
            "element.encode(sink, int(flags.s), int(flags.t))",
            r#"padded = bytes(buffer[:length]).ljust(8 * (WORDS - 3), b"\x00")"#,
            "entry[1 + 0] |= int(flags.s) << 0",
            "entry[1 + 0] |= int(flags.t) << 1",
            "entry[1 + 1] = tag",
            "for word in entry[3:])",
            "s=bool((entry[1 + 0] >> 0) & 1),",
            "t=bool((entry[1 + 0] >> 1) & 1),",
            "tag = entry[1 + 1] & 0xFF",
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]), int(flags.s), int(flags.t), tag)",
            "return EventRingItem(found.seq, element, flags, tag)",
        ],
    );
    // The tag is validated before the codec is asked anything, so a refused tag
    // publishes nothing and writes no byte.
    let checked_at = code
        .find("if not 0 <= tag <= 0xFF:")
        .expect("the tag is checked");
    let encoded_at = code
        .find("element.encode(")
        .expect("the element is encoded");
    assert!(
        checked_at < encoded_at,
        "the tag is checked before the encode:\n{code}"
    );
}

#[test]
fn python_takes_a_tag_alone_as_one_context_word_and_the_signature_takes_no_flags() {
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_python(&blocking_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the tag its element asks for");
    assert_holds(
        &code,
        &[
            "CONTEXT_WORDS: Final[int] = 1",
            "def publish(publisher: runtime.Publisher, element: EventRecord, tag: int) -> int:",
            "element.encode(sink)",
            r#"padded = bytes(buffer[:length]).ljust(8 * (WORDS - 2), b"\x00")"#,
            "entry[1 + 0] = tag",
            "for word in entry[2:])",
            "tag = entry[1 + 0] & 0xFF",
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]), tag)",
            "return EventRingItem(found.seq, element, tag)",
        ],
    );
    assert!(
        !code.contains("Flags"),
        "the element has no flag input:\n{code}"
    );
}

#[test]
fn python_keeps_sixty_four_flags_to_a_word_and_starts_the_next_word_at_the_sixty_fifth() {
    // Python's words are 64 bits: flag 63 is the last bit of the first context word and
    // flag 64 the first bit of the second.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: (0..65).map(|index| format!("f{index}")).collect(),
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_python(&blocking_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores 65 flag inputs");
    assert_holds(
        &code,
        &[
            "CONTEXT_WORDS: Final[int] = 2",
            "entry[1 + 0] |= int(flags.f0) << 0",
            "entry[1 + 0] |= int(flags.f63) << 63",
            "entry[1 + 1] |= int(flags.f64) << 0",
            "f64=bool((entry[1 + 1] >> 0) & 1),",
            "for word in entry[3:])",
        ],
    );
}

#[test]
fn a_python_element_with_no_caller_input_has_the_entry_and_the_signatures_it_always_had() {
    let code = compile_single_python(
        &blocking_ring(),
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .expect("an element with no caller input is generated");
    assert!(!code.contains("CONTEXT_WORDS"), "{code}");
    assert!(!code.contains("Flags"), "{code}");
    assert!(!code.contains("ValueError"), "{code}");
    assert_holds(
        &code,
        &[
            "WORDS: Final[int] = 1 + (MAX_ENCODED_BYTES + 7) // 8",
            "def publish(publisher: runtime.Publisher, element: EventRecord) -> int:",
            "element.encode(sink)",
            r#"padded = bytes(buffer[:length]).ljust(8 * (WORDS - 1), b"\x00")"#,
            r#"packed = b"".join(word.to_bytes(8, "little") for word in entry[1:])"#,
            "element = EventRecord.decode(runtime_codec.SceCursor(packed[:stored]))",
            "return EventRingItem(found.seq, element)",
        ],
    );
}

#[test]
fn a_resolved_retention_and_size_become_the_python_modules_constants() {
    // 9 bytes encode to 1 + (9 + 7) // 8 = 1 + 2 = 3 words, computed by the
    // module from the constant below.
    let code = compile_single_python(
        &simple_doc("event_ring", "one", "blocking", DEPLOY_KEY_RETENTION),
        &resolution_options(Some(15), 9, BroadcastLogElementFacts::default()),
    )
    .expect("a resolved log generates");
    assert_holds(
        &code,
        &[
            "RETENTION: Final[int] = 15",
            "SLOTS: Final[int] = RETENTION + 1",
            "MAX_ENCODED_BYTES: Final[int] = 9",
        ],
    );

    // A literal retention is the document's, whatever the resolution says.
    let code = compile_single_python(
        &blocking_ring(),
        &resolution_options(Some(99), 4, BroadcastLogElementFacts::default()),
    )
    .expect("a literal needs no resolution of its own");
    assert!(code.contains("RETENTION: Final[int] = 7"), "{code}");
    assert!(!code.contains("= 99"), "{code}");
}

#[test]
fn a_deploy_key_retention_is_read_from_the_target_machines_limits_for_python() {
    // The element is one `uint32`: 4 bytes.
    let xml = simple_doc("event_ring", "one", "blocking", DEPLOY_KEY_RETENTION);
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let code = generated_python_log(
        compile_documents_with(
            Language::Python,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
            &options,
            Some(&cfg),
        )
        .expect("the deploy resolves the key"),
    );
    assert_holds(
        &code,
        &[
            "RETENTION: Final[int] = 15",
            "MAX_ENCODED_BYTES: Final[int] = 4",
        ],
    );
}

#[test]
fn the_refusals_before_a_template_is_read_are_one_text_for_rust_kotlin_python_and_cpp() {
    // `blocking` is within the ceiling of every language, so none refuses the
    // progress and each reaches the checks that are the same for every
    // language: no resolution at all, a deploy key nothing resolved, a
    // retention of 0, and an element the language cannot hold (the one place
    // the language's name is written).
    let xml = simple_doc("event_ring", "one", "blocking", DEPLOY_KEY_RETENTION);
    let languages = [
        (Language::Rust, "Rust"),
        (Language::Kotlin, "Kotlin"),
        (Language::Python, "Python"),
        (Language::Cpp, "C++"),
    ];
    let details: Vec<Vec<String>> = languages
        .into_iter()
        .map(|(language, _)| {
            vec![
                single_refusal(language, &xml, &ForgeCompileOptions::default()),
                single_refusal(
                    language,
                    &xml,
                    &resolution_options(None, 4, BroadcastLogElementFacts::default()),
                ),
                single_refusal(
                    language,
                    &xml,
                    &resolution_options(Some(0), 4, BroadcastLogElementFacts::default()),
                ),
            ]
        })
        .collect();
    assert_eq!(details[0], details[1], "Rust and Kotlin");
    assert_eq!(details[0], details[2], "Rust and Python");
    assert_eq!(details[0], details[3], "Rust and C++");
    // What an element a language cannot hold is refused with differs by the fact
    // that language refuses (Rust borrows a CBOR string, a language whose template
    // does not store the context refuses a tag), and the sentence that names it is
    // the same one.
    for (language, name) in languages {
        let element = if stores_context(language) {
            BroadcastLogElementFacts {
                cbor_text_or_bytes_entry: true,
                ..BroadcastLogElementFacts::default()
            }
        } else {
            BroadcastLogElementFacts {
                caller_variant_tag: true,
                ..BroadcastLogElementFacts::default()
            }
        };
        if language != Language::Rust && stores_context(language) {
            // Holds the CBOR entry, and stores a tag: nothing is refused.
            continue;
        }
        let detail = single_refusal(language, &xml, &resolution_options(Some(3), 4, element));
        assert!(
            detail.contains(&format!(
                "the element 'event_record' cannot be stored by the {name} template"
            )),
            "{language:?}: {detail}"
        );
    }
}

// --- Kotlin generates the log over its runtime row ---

/// The generated Kotlin of the log `event_ring`, out of the outputs of a set that
/// generated.
fn generated_kotlin_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    outputs
        .into_iter()
        .flat_map(|(_, output)| output.files)
        .find(|(file, _)| file == "EventRing.kt")
        .map(|(_, code)| code)
        .expect("the set generated the log's file")
}

/// The generated Kotlin of the single document `xml`, compiled with `options`.
fn compile_single_kotlin(
    xml: &str,
    options: &ForgeCompileOptions,
) -> Result<String, Located<ForgeError>> {
    compile_forge_with_imports(
        xml,
        label("event_ring"),
        Language::Kotlin,
        Path::new("."),
        options,
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .find(|(file, _)| file == "EventRing.kt")
            .map(|(_, code)| code)
            .expect("the log's file")
    })
}

#[test]
fn kotlin_generates_a_log_over_the_runtime_sized_from_its_element() {
    // The element is one `uint32`, so it encodes to 4 bytes; a slot is then
    // 1 + (4 + 7) / 8 = 1 + 11 / 8 = 1 + 1 = 2 words (integer division), which the
    // file computes and does not restate.
    let xml = event_ring();
    let codec = codec_doc("event_record");
    let code = generated_kotlin_log(
        compile_documents(
            Language::Kotlin,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("kotlin generates a broadcast log"),
    );
    assert_holds(
        &code,
        &[
            r#"// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")"#,
            "// Runtime: com.sce.forge.runtime.broadcastlog",
            "package com.sce.generated.event_ring",
            "import com.sce.forge.runtime.ByteArraySink",
            "import com.sce.forge.runtime.CodecError",
            "import com.sce.forge.runtime.SceCursor",
            "import com.sce.forge.runtime.broadcastlog.BroadcastLog",
            "import com.sce.forge.runtime.broadcastlog.Read as RuntimeRead",
            "import com.sce.forge.runtime.broadcastlog.Reader",
            "import com.sce.generated.event_record.EventRecord",
            "const val RETENTION: Int = 7",
            "const val SLOTS: Int = RETENTION + 1",
            "const val MAX_ENCODED_BYTES: Int = 4",
            "const val WORDS: Int = 1 + (MAX_ENCODED_BYTES + 7) / 8",
            r#"const val DECLARED_PROGRESS: String = "wait-free""#,
            "typealias EventRing = BroadcastLog",
            "typealias EventRingPublisher = BroadcastLog.Publisher",
            "fun newEventRing(): EventRing = BroadcastLog(WORDS, SLOTS)",
            "sealed interface EventRingPublish {",
            "data class Published(val seq: Long) : EventRingPublish",
            "data class Refused(val error: CodecError) : EventRingPublish",
            "sealed interface EventRingRead {",
            "data class Item(val seq: Long, val element: EventRecord) : EventRingRead",
            "data object Empty : EventRingRead",
            "data class Lagged(val skipped: Long) : EventRingRead",
            "data class Unreadable(val seq: Long, val why: String) : EventRingRead",
            "fun publish(publisher: EventRingPublisher, element: EventRecord): EventRingPublish {",
            "val refused = element.encode(sink)",
            "fun read(log: EventRing, reader: Reader): EventRingRead {",
            "is RuntimeRead.Item -> decodeEntry(found.seq, entry)",
            "RuntimeRead.Empty -> EventRingRead.Empty",
            "is RuntimeRead.Lagged -> EventRingRead.Lagged(found.skipped)",
            "val element = EventRecord.decode(SceCursor(bytes))",
        ],
    );
    // The length is the first word and the bytes follow it, eight to a word,
    // little-endian: the packing and the unpacking are the same formula read in
    // the two directions.
    assert_holds(
        &code,
        &[
            "entry[0] = length.toLong()",
            "val word = 1 + index / 8",
            "(bytes[index].toLong() and 0xFFL) shl (8 * (index % 8))",
            "bytes[index] = (entry[1 + index / 8] ushr (8 * (index % 8))).toByte()",
        ],
    );
    // A stored length above the bound is refused before any buffer is sized or
    // indexed by it.
    let length_check = code
        .find("if (stored < 0L || stored > MAX_ENCODED_BYTES.toLong()) {")
        .expect("the stored length is checked against the bound, as a signed word");
    let refusal = code[length_check..]
        .find("return EventRingRead.Unreadable(seq, \"the stored length")
        .expect("a length above the bound is an unreadable entry");
    let buffer = code[length_check..]
        .find("ByteArray(length)")
        .expect("the entry's bytes are held in a buffer the length sizes");
    assert!(
        refusal < buffer,
        "the refusal comes before the buffer the length sizes:\n{code}"
    );
    assert!(
        !code.contains("!!"),
        "the file asserts nothing it could refuse:\n{code}"
    );
}

#[test]
fn kotlin_holds_the_cbor_element_that_rust_refuses_for_borrowing_its_buffer() {
    // A Kotlin text or byte-string entry is a `String` or a `ByteArray` of its own
    // (`Cbor.readText` decodes into a new string, `Cbor.readBytes` copies out of the
    // cursor), so the element outlives the buffer a read decodes it from and the
    // log holds it; Rust's decodes to a view of that buffer and refuses it.
    let xml = event_ring();
    let codec = cbor_codec_with(
        r#"<data id="note" sce:type="string" sce:key="1" sce:max-size="16" sce:direction="out"/>"#,
    );
    let code = generated_kotlin_log(
        compile_documents(
            Language::Kotlin,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a map with a string entry owns its text in Kotlin, so kotlin generates the log"),
    );
    assert_holds(
        &code,
        &[
            "import com.sce.generated.event_record.EventRecord",
            "const val MAX_ENCODED_BYTES: Int = ",
        ],
    );
    let located = compile_documents(
        Language::Rust,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .err()
    .unwrap_or_else(|| panic!("the same map borrows its buffer in Rust, so rust refuses it"));
    assert!(
        invalid_config(located).contains("a CBOR map with a string"),
        "rust names the borrow"
    );
}

#[test]
fn kotlin_stores_the_flag_inputs_an_element_asks_its_caller_for() {
    // The codec `event_record` takes the flag input `S` from its caller on `encode` and
    // `decode`, as an `UByte` after the sink and the cursor. The log stores it in a
    // context word of the entry, takes it on `publish` as a Boolean, and gives it back
    // with the item, so that every reader decodes the entry under what it was published
    // under. The element is a `uint8` and an optional `uint8`: 2 bytes, an entry of 1
    // length word, 1 context word and 1 word of bytes.
    let xml = event_ring();
    let codec = flag_input_codec();
    let code = generated_kotlin_log(
        compile_documents(
            Language::Kotlin,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        )
        .expect("a log stores the flag input its element asks for"),
    );
    assert_holds(
        &code,
        &[
            "const val MAX_ENCODED_BYTES: Int = 2",
            "const val CONTEXT_WORDS: Int = 1",
            "const val WORDS: Int = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) / 8",
            "data class EventRingFlags(",
            "    val S: Boolean",
            "        val flags: EventRingFlags",
            "fun publish(publisher: EventRingPublisher, element: EventRecord, flags: EventRingFlags): EventRingPublish {",
            "val refused = element.encode(sink, flagInput(flags.S))",
            "entry[1 + 0] = entry[1 + 0] or ((if (flags.S) 1L else 0L) shl 0)",
            "val word = 2 + index / 8",
            "bytes[index] = (entry[2 + index / 8] ushr (8 * (index % 8))).toByte()",
            "S = ((entry[1 + 0] ushr 0) and 1L) == 1L",
            "val element = EventRecord.decode(SceCursor(bytes), flagInput(flags.S))",
            "return EventRingRead.Item(",
            "private fun flagInput(on: Boolean): UByte = if (on) 1.toUByte() else 0.toUByte()",
        ],
    );
    assert!(
        !code.contains("tag: UByte"),
        "the element has no tag:\n{code}"
    );
}

#[test]
fn kotlin_stores_the_flags_and_the_tag_of_an_element_in_context_words_of_their_own() {
    // Two flag inputs fit one word; the tag takes the next. The bytes follow the
    // length word and the two context words, so they start at word 3. The codec takes
    // the flags and then the tag on `decode`, and only the flags on `encode`.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: vec!["S".to_string(), "T".to_string()],
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_kotlin(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the flags and the tag its element asks for");
    assert_holds(
        &code,
        &[
            "const val CONTEXT_WORDS: Int = 2",
            "const val WORDS: Int = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) / 8",
            "    val S: Boolean,",
            "    val T: Boolean",
            "        val flags: EventRingFlags,",
            "        val tag: UByte",
            "fun publish(publisher: EventRingPublisher, element: EventRecord, flags: EventRingFlags, tag: UByte): EventRingPublish {",
            "val refused = element.encode(sink, flagInput(flags.S), flagInput(flags.T))",
            "entry[1 + 0] = entry[1 + 0] or ((if (flags.S) 1L else 0L) shl 0)",
            "entry[1 + 0] = entry[1 + 0] or ((if (flags.T) 1L else 0L) shl 1)",
            "entry[1 + 1] = tag.toLong()",
            "val word = 3 + index / 8",
            "bytes[index] = (entry[3 + index / 8] ushr (8 * (index % 8))).toByte()",
            "val tag = entry[1 + 1].toUByte()",
            "EventRecord.decode(SceCursor(bytes), flagInput(flags.S), flagInput(flags.T), tag)",
            "        flags,",
            "        tag,",
        ],
    );
}

#[test]
fn a_tag_alone_is_one_context_word_and_the_kotlin_signature_takes_no_flags() {
    let facts = BroadcastLogElementFacts {
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_kotlin(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores the tag its element asks for");
    assert_holds(
        &code,
        &[
            "const val CONTEXT_WORDS: Int = 1",
            "fun publish(publisher: EventRingPublisher, element: EventRecord, tag: UByte): EventRingPublish {",
            "        val tag: UByte",
            "val refused = element.encode(sink)",
            "entry[1 + 0] = tag.toLong()",
            "val word = 2 + index / 8",
            "val tag = entry[1 + 0].toUByte()",
            "EventRecord.decode(SceCursor(bytes), tag)",
        ],
    );
    assert!(
        !code.contains("Flags"),
        "the element has no flag input:\n{code}"
    );
    assert!(
        !code.contains("flagInput"),
        "no flag is converted for the codec:\n{code}"
    );
}

#[test]
fn kotlin_carries_flags_past_a_64_bit_word_into_the_next_context_word() {
    // 65 flags need two flag words of 64 bits (the 64th flag is the sign bit of the
    // first), and the tag a third; the bytes then start at word 4.
    let facts = BroadcastLogElementFacts {
        caller_flag_inputs: (0..65).map(|index| format!("f{index}")).collect(),
        caller_variant_tag: true,
        ..BroadcastLogElementFacts::default()
    };
    let code = compile_single_kotlin(&event_ring(), &resolution_options(None, 4, facts))
        .expect("a log stores as many flags as its element asks for");
    assert_holds(
        &code,
        &[
            "const val CONTEXT_WORDS: Int = 3",
            "entry[1 + 0] = entry[1 + 0] or ((if (flags.f63) 1L else 0L) shl 63)",
            "entry[1 + 1] = entry[1 + 1] or ((if (flags.f64) 1L else 0L) shl 0)",
            "entry[1 + 2] = tag.toLong()",
            "val word = 4 + index / 8",
            "f63 = ((entry[1 + 0] ushr 63) and 1L) == 1L,",
            "f64 = ((entry[1 + 1] ushr 0) and 1L) == 1L",
        ],
    );
}

#[test]
fn a_kotlin_element_with_no_caller_input_has_the_entry_and_the_signatures_it_always_had() {
    let code = compile_single_kotlin(
        &event_ring(),
        &resolution_options(None, 4, BroadcastLogElementFacts::default()),
    )
    .expect("an element with no caller input is generated");
    assert!(!code.contains("CONTEXT_WORDS"), "{code}");
    assert!(!code.contains("Flags"), "{code}");
    assert!(!code.contains("flagInput"), "{code}");
    assert_holds(
        &code,
        &[
            "const val WORDS: Int = 1 + (MAX_ENCODED_BYTES + 7) / 8",
            "data class Item(val seq: Long, val element: EventRecord) : EventRingRead",
            "fun publish(publisher: EventRingPublisher, element: EventRecord): EventRingPublish {",
            "val word = 1 + index / 8",
            "bytes[index] = (entry[1 + index / 8] ushr (8 * (index % 8))).toByte()",
            "return EventRingRead.Item(seq, element)",
        ],
    );
}

#[test]
fn kotlin_refuses_a_retention_of_nothing_and_one_whose_slots_overflow_an_int() {
    let xml = deploy_key_ring();
    let located = compile_single_kotlin(
        &xml,
        &resolution_options(Some(0), 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("a log retains at least one entry"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    assert!(invalid_config(located).contains("the retention resolves to 0"));

    // i32::MAX = 2^31 - 1 = 2147483647. SLOTS = retention + 1 must fit an Int, so
    // the retention is at most 2147483646 (SLOTS = 2147483647) and 2147483647 is
    // refused (SLOTS = 2147483648).
    let code = compile_single_kotlin(
        &xml,
        &resolution_options(Some(2_147_483_646), 4, BroadcastLogElementFacts::default()),
    )
    .expect("2147483646 + 1 = 2147483647 slots fit an Int");
    assert!(
        code.contains("const val RETENTION: Int = 2147483646"),
        "{code}"
    );
    let located = compile_single_kotlin(
        &xml,
        &resolution_options(Some(2_147_483_647), 4, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("2147483647 + 1 slots do not fit an Int"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    let detail = invalid_config(located);
    assert!(detail.contains("the retention 2147483647"), "{detail}");
    assert!(detail.contains("at most 2147483646 entries"), "{detail}");
}

#[test]
fn kotlin_refuses_an_element_whose_words_would_overflow_an_int() {
    // WORDS = 1 + (MAX_ENCODED_BYTES + 7) / 8 is computed in an Int, so the sum must
    // not wrap: the largest size is i32::MAX - 7 = 2147483647 - 7 = 2147483640
    // (2147483640 + 7 = 2147483647 fits), and 2147483641 is refused
    // (2147483641 + 7 = 2147483648 does not).
    let xml = event_ring();
    let code = compile_single_kotlin(
        &xml,
        &resolution_options(None, 2_147_483_640, BroadcastLogElementFacts::default()),
    )
    .expect("2147483640 + 7 = 2147483647 fits an Int");
    assert!(
        code.contains("const val MAX_ENCODED_BYTES: Int = 2147483640"),
        "{code}"
    );
    let located = compile_single_kotlin(
        &xml,
        &resolution_options(None, 2_147_483_641, BroadcastLogElementFacts::default()),
    )
    .err()
    .unwrap_or_else(|| panic!("2147483641 + 7 wraps an Int"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    let detail = invalid_config(located);
    assert!(detail.contains("as many as 2147483641 bytes"), "{detail}");
    assert!(detail.contains("at most 2147483640"), "{detail}");
}

#[test]
fn a_deploy_key_retention_is_read_from_the_target_machines_limits_for_kotlin() {
    // The element is one `uint32`: 4 bytes.
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let cfg = deploy_with_limits("          event_retention: 15");
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let code = generated_kotlin_log(
        compile_documents_with(
            Language::Kotlin,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
            &options,
            Some(&cfg),
        )
        .expect("the deploy resolves the key"),
    );
    assert_holds(
        &code,
        &[
            "const val RETENTION: Int = 15",
            "const val MAX_ENCODED_BYTES: Int = 4",
        ],
    );
}

// --- The matrix: the registered backends emit, the others await their runtime ---

#[test]
fn the_matrix_emits_for_the_registered_backends_and_says_the_runtime_is_missing_elsewhere() {
    assert!(!awaits_runtime(ForgeKind::Queue, Language::Rust));
    assert!(!awaits_runtime(ForgeKind::Queue, Language::Cpp));
    assert_eq!(
        sce_build::forge::codegen_matrix::kind_class(ForgeKind::BroadcastLog),
        KindClass::Generic
    );
    for language in Language::ALL.iter().copied() {
        let landed = BROADCAST_LOG_BACKENDS.contains(&language);
        assert_eq!(
            template_ships(ForgeKind::BroadcastLog, language),
            landed,
            "{language:?}"
        );
        assert_eq!(
            awaits_runtime(ForgeKind::BroadcastLog, language),
            !landed,
            "{language:?}"
        );
        assert_eq!(
            lookup(ForgeKind::BroadcastLog, language),
            if landed {
                EmitOutcome::Emit
            } else {
                EmitOutcome::RuntimeMissing
            },
            "{language:?}"
        );
        assert!(
            check(ForgeKind::BroadcastLog, language).is_ok(),
            "a registered backend emits it and the dispatch of every other refuses it by \
             name, so the matrix lets it through: {language:?}"
        );
    }
    assert_eq!(
        emitting(ForgeKind::BroadcastLog).collect::<Vec<_>>(),
        Language::ALL
            .iter()
            .copied()
            .filter(|language| BROADCAST_LOG_BACKENDS.contains(language))
            .collect::<Vec<_>>()
    );
}

// --- The catalog and the page ---

#[test]
fn the_catalog_teaches_the_kind_and_names_the_backends_that_emit_it() {
    let catalog = KindCatalog::new(Some(ForgeKind::BroadcastLog));
    assert_eq!(catalog.kinds.len(), 1);
    let record = &catalog.kinds[0];
    assert_eq!(record.name, ForgeKind::BroadcastLog);
    // The catalog's `backends` is read from the matrix, in the order of
    // `Language::ALL`.
    let emitting_wire: Vec<&str> = Language::ALL
        .iter()
        .copied()
        .filter(|language| BROADCAST_LOG_BACKENDS.contains(language))
        .map(language_wire_name)
        .collect();
    assert_eq!(record.backends, emitting_wire);
    assert!(!record.choose_when.is_empty());
    assert!(
        record
            .distinct_from
            .iter()
            .any(|d| d.kind == ForgeKind::Queue),
        "a broadcast log is told apart from the queue it is confused with"
    );
    match &record.example {
        ExampleRecord::Document { path, text } => {
            assert_eq!(path, "sce-build/kind-examples/broadcast-log.scxml");
            assert!(text.contains(r#"sce:kind="broadcast-log""#));
        }
        other => panic!("expected an example document, got {other:?}"),
    }
}

#[test]
fn the_kind_example_parses_and_renders_as_one_head_line() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples/broadcast-log.scxml");
    let text = fs::read_to_string(&path).expect("the example");
    let doc = parse_forge(&text, label("event_ring"))
        .expect("the example parses")
        .expect("a forge document");
    let page = sce_build::forge::pseudo::render(&doc).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        page.contains(
            "broadcast-log event_ring of event_record producers one progress wait-free retention const 1023"
        ),
        "{page}"
    );
}

// --- Rust: the row the target chooses, the 64-bit row or the narrow one ---

#[test]
fn the_longest_pause_is_half_the_stamp_space_less_the_slots() {
    // A 32-bit stamp tells K = 2^31 publishes of a slot apart; a retention of 7
    // keeps 8 slots, so 2^31 - 8 is the pause a deploy may rely on.
    assert_eq!(broadcast_log_longest_pause_ops(32, 7), (1u64 << 31) - 8);
    // A 3-bit stamp tells K = 4 apart: 2 slots leave 2, 4 slots (as many as K)
    // leave none, and 8 slots (more than K) are none and not a wrapped number.
    assert_eq!(broadcast_log_longest_pause_ops(3, 1), 2);
    assert_eq!(broadcast_log_longest_pause_ops(3, 3), 0);
    assert_eq!(broadcast_log_longest_pause_ops(3, 7), 0);
    // The narrowest stamp tells 2 apart, and its 2 slots fill it; a width below 2 is
    // read as 2.
    assert_eq!(broadcast_log_longest_pause_ops(2, 1), 0);
    assert_eq!(broadcast_log_longest_pause_ops(0, 1), 0);
    assert_eq!(broadcast_log_longest_pause_ops(1, 1), 0);
    // The 64-bit stamp tells 2^63 apart.
    assert_eq!(broadcast_log_longest_pause_ops(64, 1), (1u64 << 63) - 2);
    assert_eq!(BROADCAST_LOG_NARROW_STAMP_BITS, 32);
}

/// The options of a log whose deploy says what it says of the target's atomics and of
/// the pause the design relies on. The element is one `uint32`: 4 bytes.
fn target_options(
    retention: Option<u32>,
    atomic_rmw_width: Option<u32>,
    target: Option<&str>,
    min_wrap_ops: Option<u64>,
) -> ForgeCompileOptions {
    ForgeCompileOptions {
        broadcast_log_resolutions: Some(HashMap::from([(
            "event_ring".to_string(),
            BroadcastLogResolution {
                retention,
                element_max_bytes: 4,
                atomic_rmw_width,
                target: target.map(str::to_string),
                min_wrap_ops,
                ..BroadcastLogResolution::default()
            },
        )])),
        ..ForgeCompileOptions::default()
    }
}

/// The longest pause a 32-bit stamp covers for the log `event_ring`, a retention of 7
/// (8 slots): half the stamp space, 2^31, less the 8 slots.
const EVENT_RING_PAUSE: u64 = (1u64 << 31) - 8;

#[test]
fn a_target_with_32_bit_atomics_is_generated_over_the_narrow_row() {
    // 4 bytes in 32-bit words: 1 + ceil(4 / 4) = 2 words an entry.
    let options = target_options(None, None, Some("thumbv7em-none-eabihf"), Some(1_000_000));
    let code = compile_single(&event_ring(), &options)
        .unwrap_or_else(|e| panic!("a 32-bit target gets the narrow row: {:?}", e.error));
    assert_holds(
        &code,
        &[
            "// Runtime: sce_forge_runtime::broadcast_log_narrow",
            "use sce_forge_runtime::broadcast_log_narrow as runtime;",
            "pub const WORDS: usize = 1 + MAX_ENCODED_BYTES.div_ceil(4);",
            "const _: () = assert!(MAX_ENCODED_BYTES <= u32::MAX as usize);",
            "pub type EventRing = runtime::BroadcastLog<WORDS, SLOTS>;",
            "let mut entry = [0u32; WORDS];",
            "entry[0] = length as u32;",
            "entry[1 + index / 4] |= u32::from(*byte) << (8 * (index % 4));",
            "let stored = u64::from(entry[0]);",
            "*byte = (entry[1 + index / 4] >> (8 * (index % 4))) as u8;",
            r#"#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]"#,
            "The publisher is `log.publisher()`, taken once by an 8-bit swap, or",
            "machines.<machine>.queues.event_ring.min_wrap_ops",
        ],
    );
    // 2^31 less the 8 slots of a retention of 7: 2147483648 - 8 = 2147483640.
    assert!(
        code.contains("begins at most 2147483640 publishes"),
        "the pause the stamp covers is stated:\n{code}"
    );
    assert!(
        !code.contains("target_has_atomic") && !code.contains("[0u64; WORDS]"),
        "nothing of the 64-bit row is left in the narrow module:\n{code}"
    );
}

#[test]
fn a_target_with_loads_and_stores_only_is_generated_with_an_ownership_publisher() {
    // A width of 0 stated, and a triple the table knows to have no read-modify-write
    // atomic (ARMv6-M), are the same statement.
    for (width, target) in [(Some(0), None), (None, Some("thumbv6m-none-eabi"))] {
        let options = target_options(None, width, target, Some(1_000_000));
        let code = compile_single(&event_ring(), &options)
            .unwrap_or_else(|e| panic!("{width:?} {target:?}: {:?}", e.error));
        assert_holds(
            &code,
            &[
                "use sce_forge_runtime::broadcast_log_narrow as runtime;",
                "// The publisher is `log.split()`, which takes an exclusive borrow of the log and",
                "`unsafe` `log.publisher_unchecked()`",
                "taken once with `split()` (or, for a log in a",
            ],
        );
        assert!(
            !code.contains("taken once by an 8-bit swap"),
            "no swap is promised where there is none ({width:?} {target:?}):\n{code}"
        );
    }
}

#[test]
fn a_64_bit_target_or_a_deploy_that_says_nothing_keeps_the_64_bit_row() {
    let generate = |width, target| {
        compile_single(&event_ring(), &target_options(None, width, target, None))
            .unwrap_or_else(|e| panic!("{width:?} {target:?}: {:?}", e.error))
    };
    let said_nothing = generate(None, None);
    // The row is the same module whether the deploy says 64 or says nothing, and a
    // pause it does not state is not asked for: the 64-bit stamp has no wrap bound.
    assert_eq!(said_nothing, generate(Some(64), None));
    assert_eq!(
        said_nothing,
        generate(None, Some("x86_64-unknown-linux-gnu"))
    );
    assert_holds(
        &said_nothing,
        &[
            "use sce_forge_runtime::broadcast_log as runtime;",
            "pub const WORDS: usize = 1 + MAX_ENCODED_BYTES.div_ceil(8);",
            "let mut entry = [0u64; WORDS];",
            r#"#[cfg(not(all(target_has_atomic = "64", target_has_atomic = "8")))]"#,
        ],
    );
    assert!(
        !said_nothing.contains("broadcast_log_narrow"),
        "{said_nothing}"
    );
}

#[test]
fn a_narrow_row_needs_the_pause_the_design_relies_on_stated_and_covered() {
    let at = |pause: Option<u64>| {
        compile_single(&event_ring(), &target_options(None, Some(32), None, pause))
    };

    let unstated = at(None).err().unwrap_or_else(|| {
        panic!("a narrow stamp wraps, so a log with no stated pause is refused")
    });
    match generate_error(unstated) {
        GenerateError::BroadcastLogWrapBoundUnstated {
            log_name,
            stamp_bits,
            bound,
        } => {
            assert_eq!(log_name, "event_ring");
            assert_eq!((stamp_bits, bound), (32, EVENT_RING_PAUSE));
        }
        other => panic!("expected BroadcastLogWrapBoundUnstated, got {other:?}"),
    }

    let too_long = at(Some(EVENT_RING_PAUSE + 1))
        .err()
        .unwrap_or_else(|| panic!("a pause above what the stamp covers is refused"));
    match generate_error(too_long) {
        GenerateError::BroadcastLogWrapBoundBelowDeployMinimum {
            bound,
            min_wrap_ops,
            ..
        } => assert_eq!(
            (bound, min_wrap_ops),
            (EVENT_RING_PAUSE, EVENT_RING_PAUSE + 1)
        ),
        other => panic!("expected BroadcastLogWrapBoundBelowDeployMinimum, got {other:?}"),
    }

    // The pause the stamp covers, exactly and below, is met.
    at(Some(EVENT_RING_PAUSE)).expect("the boundary is covered");
    at(Some(1)).expect("a short pause is covered");
}

#[test]
fn a_narrow_row_whose_slot_count_does_not_divide_the_stamp_space_is_refused() {
    // A retention of 5 keeps 5 + 1 = 6 slots, between 4 and 8.
    let xml = deploy_key_ring();
    let located = compile_single(&xml, &target_options(Some(5), Some(32), None, Some(1)))
        .err()
        .unwrap_or_else(|| panic!("a narrow stamp wraps, and 6 slots do not divide 2^32"));
    assert!(
        row_text(&located, &xml).contains("<sce:retention"),
        "the refusal is placed on the retention row"
    );
    match generate_error(located) {
        GenerateError::BroadcastLogSlotsNotADivisorOfTheStampSpace {
            retention,
            slots,
            stamp_bits,
            nearest_below,
            nearest_above,
            ..
        } => {
            assert_eq!((retention, slots, stamp_bits), (5, 6, 32));
            // One less than 4 and than 8.
            assert_eq!((nearest_below, nearest_above), (3, 7));
        }
        other => panic!("expected BroadcastLogSlotsNotADivisorOfTheStampSpace, got {other:?}"),
    }
    // The 64-bit row has no wrap and keeps any retention.
    compile_single(&xml, &target_options(Some(5), Some(64), None, None))
        .expect("the 64-bit row keeps a retention of 5");
}

#[test]
fn a_width_that_contradicts_the_target_is_refused_and_an_unknown_target_needs_a_width() {
    let located = compile_single(
        &event_ring(),
        &target_options(None, Some(64), Some("thumbv7em-none-eabihf"), None),
    )
    .err()
    .unwrap_or_else(|| panic!("two statements of the atomics that differ cannot both hold"));
    let detail = invalid_config(located);
    assert!(detail.contains("platform.atomic_rmw_width 64"), "{detail}");
    assert!(detail.contains("thumbv7em-none-eabihf"), "{detail}");
    assert!(detail.contains("atomic is 32 bits"), "{detail}");

    let unknown = Some("my-board-none-elf");
    let located = compile_single(&event_ring(), &target_options(None, None, unknown, Some(1)))
        .err()
        .unwrap_or_else(|| panic!("a target the generator does not know says nothing"));
    assert!(
        invalid_config(located).contains("does not know"),
        "the refusal says why"
    );
    // A width stated beside it stands alone for a triple the table does not know.
    compile_single(
        &event_ring(),
        &target_options(None, Some(32), unknown, Some(1)),
    )
    .expect("a stated width is taken for a target the table does not know");
}

#[test]
fn the_row_is_chosen_after_the_refusals_every_language_shares() {
    // A retention that resolves to nothing is refused by the shared helper before
    // any row is chosen, so a narrow target is not asked about its pause first.
    let located = compile_single(
        &deploy_key_ring(),
        &target_options(Some(0), Some(32), None, None),
    )
    .err()
    .unwrap_or_else(|| panic!("a retention of 0 is refused"));
    assert!(
        invalid_config(located).contains("the retention resolves to 0"),
        "the shared refusal comes before the row's"
    );
}

#[test]
fn the_narrow_compile_error_is_gated_by_the_cfg_the_narrow_runtime_module_is() {
    let lib = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/rust/forge-runtime/src/lib.rs"),
    )
    .expect("the runtime's lib.rs");
    let at = lib
        .find("pub mod broadcast_log_narrow;")
        .expect("the runtime has the narrow row's module");
    let gate = lib[..at]
        .lines()
        .last()
        .expect("a line precedes the module")
        .trim()
        .strip_prefix("#[cfg(")
        .and_then(|rest| rest.strip_suffix(")]"))
        .expect("the module is gated by one cfg attribute")
        .to_string();

    let code = compile_single(
        &event_ring(),
        &target_options(None, Some(32), None, Some(1)),
    )
    .expect("a 32-bit target gets the narrow row");
    assert!(
        code.contains(&format!("#[cfg(not({gate}))]\ncompile_error!(")),
        "the narrow module refuses a target the runtime's narrow module is absent on, under \
         the cfg `{gate}`:\n{code}"
    );
}

#[test]
fn the_stamp_the_generator_judges_is_the_one_the_narrow_row_keeps() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../backends/rust/forge-runtime/src/broadcast_log_narrow.rs"),
    )
    .expect("the narrow row's module");
    let declared = format!("pub const STAMP_BITS: u32 = {BROADCAST_LOG_NARROW_STAMP_BITS};");
    assert!(
        source.contains(&declared),
        "the runtime's narrow row keeps a stamp of the width the generator judges \
         ({declared})"
    );
}

#[test]
fn a_deploy_states_the_target_and_the_pause_and_the_set_generates_the_narrow_row() {
    let xml = deploy_key_ring();
    let codec = codec_doc("event_record");
    let deploy = |queues: &str| {
        parse_deploy_str(&format!(
            r##"
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          event_retention: 15
{queues}
"##
        ))
        .expect("deploy parses")
    };
    let options = ForgeCompileOptions {
        target_machine: Some("mcu_node".to_string()),
        ..ForgeCompileOptions::default()
    };
    let documents = [("event_ring.scxml", &xml), ("event_record.scxml", &codec)];

    // The machine's device is built for ARMv7E-M (32-bit atomics), and the deploy
    // states the pause for the log, so the set generates the narrow row, sized from the
    // limit: 16 slots, of which 2^31 - 16 = 2147483632 is what the stamp covers.
    let cfg = deploy("        queues:\n          event_ring:\n            min_wrap_ops: 1000000");
    let code = generated_log(
        compile_documents_with(Language::Rust, &documents, &options, Some(&cfg))
            .unwrap_or_else(|e| panic!("the deploy gives a target and a pause: {:?}", e.error)),
    );
    assert_holds(
        &code,
        &[
            "use sce_forge_runtime::broadcast_log_narrow as runtime;",
            "pub const RETENTION: usize = 15;",
            "begins at most 2147483632 publishes",
        ],
    );

    // The same deploy without the pause is refused: the target says the stamp wraps.
    let cfg = deploy("");
    let located = compile_documents_with(Language::Rust, &documents, &options, Some(&cfg))
        .err()
        .unwrap_or_else(|| panic!("a narrow stamp needs the pause stated"));
    match generate_error(located) {
        GenerateError::BroadcastLogWrapBoundUnstated {
            log_name, bound, ..
        } => assert_eq!(
            (log_name.as_str(), bound),
            ("event_ring", (1u64 << 31) - 16)
        ),
        other => panic!("expected BroadcastLogWrapBoundUnstated, got {other:?}"),
    }
}

#[test]
fn only_the_rust_renderer_reads_the_target_go_has_one_row() {
    // The resolution that gives Rust the narrow row leaves Go's package as it was:
    // its runtime has one row.
    let go = |options: ForgeCompileOptions| ForgeCompileOptions {
        go_module_prefix: options_for(Language::Go).go_module_prefix,
        ..options
    };
    let with_target = go(target_options(None, Some(32), None, Some(1)));
    let without = go(target_options(None, None, None, None));
    let a = compile_single_as(Language::Go, "event_ring.go", &event_ring(), &with_target)
        .expect("go generates the log");
    let b = compile_single_as(Language::Go, "event_ring.go", &event_ring(), &without)
        .expect("go generates the log");
    assert_eq!(a, b, "a target changes nothing for Go");
}
