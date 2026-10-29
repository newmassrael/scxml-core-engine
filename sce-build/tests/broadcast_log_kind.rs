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
    awaits_runtime, broadcast_log_progress_ceiling, check, emitting, lookup, template_ships,
    EmitOutcome, KindClass,
};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::forge::generator::judge_broadcast_log_stamp;
use sce_build::forge::kind_catalog::{ExampleRecord, KindCatalog};
use sce_build::forge::model::{
    broadcast_log_slots_divide_the_stamp_space, broadcast_log_wrap_bound_ops, BroadcastLogModel,
    CapacitySource, ForgeDocument, ForgeKind, QueueCardinality, QueueProgress, RuntimeDep,
};
use sce_build::forge::parser::parse_forge;
use sce_build::generator::Language;
use sce_build::mesh::deploy::{parse_deploy_str, DeployConfig};
use sce_build::{
    compile_forge_from_string, compile_forge_with_imports, resolve_broadcast_log_retention,
    BroadcastLogResolution, DocumentLabel, ForgeCompileOptions,
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
    let bound = 1u64 << 31;

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

/// The codes of the refusal `result` is, which must be one.
fn refusal_codes(
    result: Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>>,
) -> Vec<DiagnosticCode> {
    result
        .err()
        .unwrap_or_else(|| {
            panic!("the language has no broadcast-log row yet, so the set is refused")
        })
        .error
        .to_diagnostics()
        .iter()
        .map(|d| d.code)
        .collect()
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

#[test]
fn an_element_whose_every_field_is_bounded_is_not_refused_for_its_size() {
    // Reached the missing runtime instead: the one refusal a valid document
    // has on a language that has no row yet (C++ has none).
    let xml = event_ring();
    for (name, codec) in [
        ("a fixed record", codec_doc("event_record")),
        (
            "a payload with a max-size",
            codec_with("event_record", "", BOUNDED_FIELDS),
        ),
    ] {
        let codes = refusal_codes(compile_documents(
            Language::Cpp,
            &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
        ));
        assert!(
            matches!(
                codes.as_slice(),
                [DiagnosticCode::BroadcastLogRuntimeMissing]
            ),
            "{name}: {codes:?}"
        );
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
    let codes = refusal_codes(compile_documents(
        Language::Cpp,
        &[
            ("event_ring.scxml", &xml),
            ("event_record.scxml", &parent),
            ("payload_body.scxml", &body),
        ],
    ));
    assert!(
        matches!(
            codes.as_slice(),
            [DiagnosticCode::BroadcastLogRuntimeMissing]
        ),
        "{codes:?}"
    );
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
fn a_valid_document_is_refused_by_name_on_every_language_but_rust() {
    let xml = simple_doc(
        "event_ring",
        "one",
        "blocking",
        r#"<sce:retention entries="1023"/>"#,
    );
    for language in Language::ALL
        .iter()
        .copied()
        .filter(|language| *language != Language::Rust)
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

    // A backend that can give it and has no row yet (C++) reaches the missing
    // runtime instead.
    let located = compile_forge_from_string(&xml, label("event_ring"), Language::Cpp)
        .err()
        .unwrap_or_else(|| panic!("c++ has no row yet, so it refuses the document"));
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

/// The generated Rust of the log `event_ring`, out of the outputs of a set that
/// generated.
fn generated_log(outputs: Vec<(String, sce_build::generator::GeneratedOutput)>) -> String {
    outputs
        .into_iter()
        .flat_map(|(_, output)| output.files)
        .find(|(file, _)| file == "event_ring.rs")
        .map(|(_, code)| code)
        .expect("the set generated the log's module")
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

/// A codec `event_record` that no log can hold: its `encode` and `decode` take a
/// flag input from the caller.
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

#[test]
fn an_element_whose_signature_asks_the_caller_for_more_is_refused_by_name() {
    let xml = event_ring();
    let codec = flag_input_codec();
    let located = compile_documents(
        Language::Rust,
        &[("event_ring.scxml", &xml), ("event_record.scxml", &codec)],
    )
    .err()
    .unwrap_or_else(|| panic!("a log has no flag input to give, so the set is refused"));
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>event_record<"),
        "the refusal is placed on the element-type row"
    );
    match generate_error(located) {
        GenerateError::InvalidConfig(detail) => {
            assert!(
                detail.contains("the element 'event_record' cannot be stored by the Rust template"),
                "{detail}"
            );
            assert!(detail.contains("flag input(s) S"), "{detail}");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
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
    element_unsupported: Option<&str>,
) -> ForgeCompileOptions {
    ForgeCompileOptions {
        broadcast_log_resolutions: Some(HashMap::from([(
            "event_ring".to_string(),
            BroadcastLogResolution {
                retention,
                element_max_bytes,
                element_unsupported: element_unsupported.map(str::to_string),
            },
        )])),
        ..ForgeCompileOptions::default()
    }
}

/// The generated Rust of the single document `xml`, compiled with `options`.
fn compile_single(xml: &str, options: &ForgeCompileOptions) -> Result<String, Located<ForgeError>> {
    compile_forge_with_imports(
        xml,
        label("event_ring"),
        Language::Rust,
        Path::new("."),
        options,
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .find(|(file, _)| file == "event_ring.rs")
            .map(|(_, code)| code)
            .expect("the log's module")
    })
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
    let located = compile_single(&xml, &resolution_options(None, 4, None))
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
    let code = compile_single(&deploy_key_ring(), &resolution_options(Some(15), 9, None))
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
    let code = compile_single(&event_ring(), &resolution_options(Some(99), 4, None))
        .expect("a literal needs no resolution of its own");
    assert!(code.contains("pub const RETENTION: usize = 7;"), "{code}");
    assert!(!code.contains("= 99;"), "{code}");
}

#[test]
fn a_retention_that_resolves_to_nothing_is_refused() {
    let xml = deploy_key_ring();
    let located = compile_single(&xml, &resolution_options(Some(0), 4, None))
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
    let why = "its decode takes the variant's tag from the caller, and a log has none to give";
    let located = compile_single(&xml, &resolution_options(None, 4, Some(why)))
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

// --- The matrix: Rust emits, the other languages await their runtime ---

#[test]
fn the_matrix_emits_for_rust_and_says_the_runtime_is_missing_elsewhere() {
    assert!(!awaits_runtime(ForgeKind::Queue, Language::Rust));
    assert!(!awaits_runtime(ForgeKind::Queue, Language::Cpp));
    assert_eq!(
        sce_build::forge::codegen_matrix::kind_class(ForgeKind::BroadcastLog),
        KindClass::Generic
    );
    for language in Language::ALL.iter().copied() {
        let rust = language == Language::Rust;
        assert_eq!(
            template_ships(ForgeKind::BroadcastLog, language),
            rust,
            "{language:?}"
        );
        assert_eq!(
            awaits_runtime(ForgeKind::BroadcastLog, language),
            !rust,
            "{language:?}"
        );
        assert_eq!(
            lookup(ForgeKind::BroadcastLog, language),
            if rust {
                EmitOutcome::Emit
            } else {
                EmitOutcome::RuntimeMissing
            },
            "{language:?}"
        );
        assert!(
            check(ForgeKind::BroadcastLog, language).is_ok(),
            "Rust emits it and the dispatch of every other refuses it by name, so the matrix \
             lets it through: {language:?}"
        );
    }
    assert_eq!(
        emitting(ForgeKind::BroadcastLog).collect::<Vec<_>>(),
        vec![Language::Rust]
    );
}

// --- The catalog and the page ---

#[test]
fn the_catalog_teaches_the_kind_and_says_only_rust_emits_it() {
    let catalog = KindCatalog::new(Some(ForgeKind::BroadcastLog));
    assert_eq!(catalog.kinds.len(), 1);
    let record = &catalog.kinds[0];
    assert_eq!(record.name, ForgeKind::BroadcastLog);
    assert_eq!(record.backends, ["rust"]);
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
