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
use sce_build::{compile_forge_from_string, DocumentLabel, ForgeCompileOptions};

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

// --- Generation: every language refuses by name while none has the row ---

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
fn a_valid_document_is_refused_by_name_on_every_language() {
    let xml = simple_doc(
        "event_ring",
        "one",
        "blocking",
        r#"<sce:retention entries="1023"/>"#,
    );
    for language in Language::ALL.iter().copied() {
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
            .unwrap_or_else(|| panic!("{wire}: no runtime has a broadcast-log row yet"))
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

    // A backend that can give it reaches the missing runtime instead.
    let located = compile_forge_from_string(&xml, label("event_ring"), Language::Rust)
        .err()
        .unwrap_or_else(|| panic!("no runtime has a row yet, so rust refuses the document"));
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

// --- The matrix: a valid kind that awaits its runtime ---

#[test]
fn the_matrix_says_the_runtime_is_missing_and_not_that_a_template_is() {
    assert!(awaits_runtime(ForgeKind::BroadcastLog));
    assert!(!awaits_runtime(ForgeKind::Queue));
    assert_eq!(
        sce_build::forge::codegen_matrix::kind_class(ForgeKind::BroadcastLog),
        KindClass::Generic
    );
    for language in Language::ALL.iter().copied() {
        assert!(!template_ships(ForgeKind::BroadcastLog, language));
        assert_eq!(
            lookup(ForgeKind::BroadcastLog, language),
            EmitOutcome::RuntimeMissing,
            "{language:?}"
        );
        assert!(
            check(ForgeKind::BroadcastLog, language).is_ok(),
            "the dispatch refuses it by name, so the matrix lets it through: {language:?}"
        );
    }
    assert_eq!(emitting(ForgeKind::BroadcastLog).count(), 0);
}

// --- The catalog and the page ---

#[test]
fn the_catalog_teaches_the_kind_and_says_no_backend_emits_it() {
    let catalog = KindCatalog::new(Some(ForgeKind::BroadcastLog));
    assert_eq!(catalog.kinds.len(), 1);
    let record = &catalog.kinds[0];
    assert_eq!(record.name, ForgeKind::BroadcastLog);
    assert!(record.backends.is_empty());
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
