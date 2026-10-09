//! Queue kind: parse, the selection table, the parse-time refusals, and the
//! Rust emit.
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-P: a `sce:kind="queue"` document
//! states a contract (element type, producer and consumer cardinality, the
//! progress it relies on, one storage mode) and names no algorithm. The
//! generator picks the algorithm from the selection table and refuses a
//! contract the table cannot keep:
//!
//! 1. `queue/storage-not-exactly-one`
//! 2. `queue/allocator-progress-missing`
//! 3. `queue/progress-unreachable`
//! 4. `queue/participants-unresolved`
//!
//! It also refuses, at emit, a valid document whose storage mode the Rust
//! runtime does not have yet (`queue/storage-runtime-missing`).
//!
//! The emit tests follow `c6_bounded_collection_rust_emit.rs`: the emitted
//! module is parsed through `syn`, then checked for the shape that matters,
//! which here is the runtime type the table chose and the ring it sized.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::forge::generator::QUEUE_SCQ_GO_ARCHITECTURES;
use sce_build::forge::model::{
    CapacitySource, ForgeDocument, QueueAlgorithm, QueueCardinality, QueueModel, QueueProgress,
    QueueStorage,
};
use sce_build::forge::parser::parse_forge;
use sce_build::generator::Language;
use sce_build::DocumentLabel;
use sce_build::ForgeCompileOptions;

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

fn parse(content: &str, name: &'static str) -> Result<QueueModel, Located<ForgeError>> {
    match parse_forge(content, label(name))? {
        Some(ForgeDocument::Queue(q)) => Ok(q),
        Some(other) => panic!("expected ForgeDocument::Queue, got {:?}", other.kind()),
        None => panic!("statechart routed through forge entry — fixture mis-tagged?"),
    }
}

/// A queue document with the storage and participants lines the test needs.
fn queue_doc(name: &str, producers: &str, consumers: &str, progress: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="queue" name="{name}" version="1.0">
  <sce:element-type>rx_event</sce:element-type>
  <sce:producers>{producers}</sce:producers>
  <sce:consumers>{consumers}</sce:consumers>
  <sce:progress>{progress}</sce:progress>
  {body}
</scxml>"##
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

// ─── Parse: the three storage modes, from the checked-in fixtures ───

#[test]
fn bounded_one_one_fixture_parses_and_selects_the_lamport_ring() {
    let q = parse(
        &resource("queue_ast_export_min.scxml"),
        "queue_ast_export_min",
    )
    .expect("minimal queue parses");
    assert_eq!(q.element_type, "rx_event");
    assert_eq!(q.producers, QueueCardinality::One);
    assert_eq!(q.consumers, QueueCardinality::One);
    assert_eq!(q.progress, QueueProgress::WaitFree);
    assert!(matches!(
        &q.storage,
        QueueStorage::Bounded {
            capacity: CapacitySource::CompileConst { value: 8 }
        }
    ));
    assert!(q.participants.is_none());

    let s = q.selection();
    assert_eq!(s.algorithm, QueueAlgorithm::LamportRing);
    assert_eq!(
        (s.push, s.pop),
        (QueueProgress::WaitFree, QueueProgress::WaitFree)
    );
}

#[test]
fn segmented_fixture_parses_and_selects_lscq_bounded_by_its_allocator() {
    let q = parse(
        &resource("queue_segmented_lscq.scxml"),
        "queue_segmented_lscq",
    )
    .expect("segmented queue parses");
    assert_eq!(
        q.storage,
        QueueStorage::Segmented {
            segment: 64,
            allocator_progress: QueueProgress::LockFree,
        }
    );
    let s = q.selection();
    assert_eq!(s.algorithm, QueueAlgorithm::Lscq);
    assert_eq!(
        (s.push, s.pop),
        (QueueProgress::LockFree, QueueProgress::LockFree)
    );
}

#[test]
fn intrusive_fixture_parses_and_selects_the_vyukov_mpsc_list() {
    let q = parse(
        &resource("queue_intrusive_mpsc.scxml"),
        "queue_intrusive_mpsc",
    )
    .expect("intrusive queue parses");
    assert_eq!(
        q.storage,
        QueueStorage::Intrusive {
            link_field: "next".into(),
        }
    );
    let s = q.selection();
    assert_eq!(s.algorithm, QueueAlgorithm::VyukovMpsc);
    assert_eq!(
        (s.push, s.pop),
        (QueueProgress::WaitFree, QueueProgress::Blocking)
    );
}

// ─── The selection table: every bounded row, and a segmented allocator ───

#[test]
fn bounded_other_cardinalities_select_scq_and_need_participants() {
    for (producers, consumers) in [("many", "one"), ("one", "many"), ("many", "many")] {
        let xml = queue_doc(
            "q",
            producers,
            consumers,
            "lock-free",
            r#"<sce:bounded capacity="8"/><sce:participants const="3"/>"#,
        );
        let q =
            parse(&xml, "q").unwrap_or_else(|e| panic!("{producers}/{consumers}: {:?}", e.error));
        let s = q.selection();
        assert_eq!(s.algorithm, QueueAlgorithm::Scq, "{producers}/{consumers}");
        assert_eq!(
            (s.push, s.pop),
            (QueueProgress::LockFree, QueueProgress::LockFree)
        );
        assert!(matches!(
            q.participants,
            Some(CapacitySource::CompileConst { value: 3 })
        ));
    }
}

#[test]
fn a_segmented_push_is_no_stronger_than_its_allocator() {
    let xml = queue_doc(
        "q",
        "one",
        "one",
        "blocking",
        r#"<sce:segmented segment="16" allocator-progress="blocking"/>"#,
    );
    let q = parse(&xml, "q").expect("segmented one/one parses");
    let s = q.selection();
    assert_eq!(s.algorithm, QueueAlgorithm::LinkedLamportRings);
    assert_eq!(s.push, QueueProgress::Blocking);
    assert_eq!(s.pop, QueueProgress::WaitFree);
}

// ─── Refusals the document alone decides ───

#[test]
fn two_storage_elements_are_refused() {
    let xml = queue_doc(
        "rx_events",
        "one",
        "one",
        "blocking",
        r#"<sce:bounded capacity="8"/><sce:intrusive link-field="next"/>"#,
    );
    let (error, codes) = validation(parse(&xml, "rx_events").expect_err("two storages refuse"));
    match error {
        ValidationError::QueueStorageNotExactlyOne {
            queue_name,
            written,
        } => {
            assert_eq!(queue_name, "rx_events");
            assert_eq!(written, ["<sce:bounded>", "<sce:intrusive>"]);
        }
        other => panic!("expected QueueStorageNotExactlyOne, got {other:?}"),
    }
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::QueueStorageNotExactlyOne]
    ));
}

#[test]
fn no_storage_element_is_refused() {
    let xml = queue_doc("rx_events", "one", "one", "blocking", "");
    let (error, _) = validation(parse(&xml, "rx_events").expect_err("no storage refuses"));
    assert!(matches!(
        error,
        ValidationError::QueueStorageNotExactlyOne { ref written, .. } if written.is_empty()
    ));
}

#[test]
fn a_segmented_queue_must_state_its_allocator_progress() {
    let xml = queue_doc(
        "rx_events",
        "one",
        "one",
        "blocking",
        r#"<sce:segmented segment="16"/>"#,
    );
    let (error, codes) = validation(parse(&xml, "rx_events").expect_err("no allocator progress"));
    assert!(matches!(
        error,
        ValidationError::QueueAllocatorProgressMissing { .. }
    ));
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::QueueAllocatorProgressMissing]
    ));
}

#[test]
fn a_declared_progress_the_algorithm_cannot_keep_is_refused_not_weakened() {
    // Bounded many/one is SCQ, which is lock-free; wait-free is not reachable.
    let xml = queue_doc(
        "rx_events",
        "many",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/><sce:participants const="3"/>"#,
    );
    let (error, codes) = validation(parse(&xml, "rx_events").expect_err("unreachable progress"));
    match error {
        ValidationError::QueueProgressUnreachable {
            declared,
            reachable,
            storage,
            ..
        } => {
            assert_eq!(declared, "wait-free");
            assert_eq!(reachable, "lock-free");
            assert_eq!(storage, "bounded");
        }
        other => panic!("expected QueueProgressUnreachable, got {other:?}"),
    }
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::QueueProgressUnreachable]
    ));
}

#[test]
fn an_intrusive_many_one_queue_cannot_declare_a_non_blocking_pop() {
    let xml = queue_doc(
        "rx_events",
        "many",
        "one",
        "lock-free",
        r#"<sce:intrusive link-field="next"/>"#,
    );
    let (error, _) = validation(parse(&xml, "rx_events").expect_err("pop blocks"));
    assert!(matches!(
        error,
        ValidationError::QueueProgressUnreachable { ref reachable, .. } if reachable == "blocking"
    ));
}

#[test]
fn an_scq_queue_without_participants_is_refused() {
    let xml = queue_doc(
        "rx_events",
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let (error, codes) = validation(parse(&xml, "rx_events").expect_err("no participants"));
    assert!(matches!(
        error,
        ValidationError::QueueParticipantsUnresolved { .. }
    ));
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::QueueParticipantsUnresolved]
    ));
}

#[test]
fn an_empty_intrusive_link_field_is_refused() {
    let xml = queue_doc(
        "rx_events",
        "one",
        "one",
        "wait-free",
        r#"<sce:intrusive link-field=" "/>"#,
    );
    let (error, _) = validation(parse(&xml, "rx_events").expect_err("empty link field"));
    assert!(matches!(
        error,
        ValidationError::AttributeRuleViolated { ref attr, .. } if attr == "link-field"
    ));
}

#[test]
fn a_zero_segment_is_refused() {
    let xml = queue_doc(
        "rx_events",
        "one",
        "one",
        "blocking",
        r#"<sce:segmented segment="0" allocator-progress="blocking"/>"#,
    );
    let (error, _) = validation(parse(&xml, "rx_events").expect_err("zero segment"));
    assert!(matches!(
        error,
        ValidationError::AttributeRuleViolated { ref attr, .. } if attr == "segment"
    ));
}

#[test]
fn an_unknown_cardinality_word_is_refused() {
    let xml = queue_doc(
        "rx_events",
        "several",
        "one",
        "blocking",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let (error, _) = validation(parse(&xml, "rx_events").expect_err("unknown word"));
    assert!(matches!(error, ValidationError::InvalidAttribute { .. }));
}

// ─── Rust emit ───

fn template_dir(language: Language) -> PathBuf {
    sce_build::find_template_dir_for(language)
}

/// The element document: a `sensor` value and a `next` field, the one an
/// intrusive queue links its elements through.
fn codec_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="sensor" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
    <sce:field id="next" sce:type="uint32" sce:byte="4" sce:bit-size="32"/>
  </datamodel>
</scxml>"##
    )
}

/// Compile the queue with its element codec for Rust; the queue's single file
/// body.
fn compile(queue_xml: &str, queue_basename: &str) -> Result<String, Located<ForgeError>> {
    compile_for(Language::Rust, queue_xml, queue_basename)
}

/// The same for `language`.
fn compile_for(
    language: Language,
    queue_xml: &str,
    queue_basename: &str,
) -> Result<String, Located<ForgeError>> {
    let files = compile_files(
        language,
        &ForgeCompileOptions::default(),
        queue_xml,
        queue_basename,
    )?;
    Ok(files
        .into_iter()
        .map(|(_, content)| content)
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// Every file the queue's compile writes for `language`, by name.
fn compile_files(
    language: Language,
    options: &ForgeCompileOptions,
    queue_xml: &str,
    queue_basename: &str,
) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    let codec_path = dir.path().join("rx_event.scxml");
    fs::write(&codec_path, codec_doc("rx_event")).expect("write codec");
    let queue_path = dir.path().join(queue_basename);
    fs::write(&queue_path, queue_xml).expect("write queue");
    let outputs = compile_scxml_with_imports(
        &[],
        &[codec_path.as_path(), queue_path.as_path()],
        &template_dir(language),
        language,
        options,
        None,
    )?;
    let output = outputs
        .iter()
        .find(|(name, _)| name == queue_basename)
        .unwrap_or_else(|| panic!("no output for {queue_basename}"));
    Ok(output.1.files.clone())
}

fn assert_parses(label: &str, code: &str) {
    if let Err(e) = syn::parse_file(code) {
        panic!("{label}: emit does not parse as Rust: {e}\n--- code ---\n{code}");
    }
}

#[test]
fn a_lamport_ring_emits_the_spsc_runtime_type_and_no_ring_constants() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let code = compile(&xml, "frame_queue.scxml").expect("spsc queue emits");
    assert_parses("lamport", &code);
    assert!(code.contains("pub const CAPACITY: usize = 8;"), "{code}");
    assert!(
        code.contains("pub type FrameQueue = Spsc<RxEvent, CAPACITY>;"),
        "{code}"
    );
    assert!(
        code.contains("use sce_forge_runtime::queue::spsc::Spsc;"),
        "{code}"
    );
    assert!(
        code.contains(r#"pub const DECLARED_PROGRESS: &str = "wait-free";"#),
        "{code}"
    );
    assert!(
        !code.contains("RING_SLOTS"),
        "a Lamport ring has no index rings:\n{code}"
    );
}

#[test]
fn an_scq_ring_is_sized_by_the_larger_of_capacity_and_participants() {
    // capacity 6 rounds to 8; participants 9 forces 16.
    for (capacity, participants, ring) in [(6, 5, 8), (4, 9, 16)] {
        let xml = queue_doc(
            "work_queue",
            "many",
            "many",
            "lock-free",
            &format!(
                r#"<sce:bounded capacity="{capacity}"/><sce:participants const="{participants}"/>"#
            ),
        );
        let code = compile(&xml, "work_queue.scxml").expect("scq queue emits");
        assert_parses("scq", &code);
        assert!(
            code.contains(&format!("pub const RING_SLOTS: usize = {ring};")),
            "capacity {capacity}, participants {participants}:\n{code}"
        );
        assert!(
            code.contains(&format!("pub const PARTICIPANTS: usize = {participants};")),
            "{code}"
        );
        assert!(
            code.contains("pub type WorkQueue = Scq<RxEvent, CAPACITY, RING_SLOTS>;"),
            "{code}"
        );
        assert!(code.contains("target_has_atomic = \"64\""), "{code}");
    }
}

#[test]
fn a_storage_the_rust_runtime_lacks_is_refused_by_name() {
    for (fixture, storage) in [
        ("queue_segmented_lscq.scxml", "segmented"),
        ("queue_intrusive_mpsc.scxml", "intrusive"),
    ] {
        let located = compile(&resource(fixture), fixture)
            .expect_err("a storage without a runtime is refused");
        // The refusal is placed on the storage element, so the mode it
        // names is found on the row it points at and not by searching a
        // document that spells the word in its comment, its element and its
        // `sce:kind`.
        let text = resource(fixture);
        let row = located
            .location
            .line
            .expect("the refusal carries the storage element's row");
        let line = text
            .lines()
            .nth(row as usize - 1)
            .unwrap_or_else(|| panic!("{fixture} has no row {row}"));
        assert!(
            line.contains(&format!("<sce:{storage}")),
            "{fixture}: row {row} is not the storage element: {line}"
        );
        let err = located.error;
        let codes: Vec<_> = err.to_diagnostics().iter().map(|d| d.code).collect();
        assert!(
            matches!(
                codes.as_slice(),
                [DiagnosticCode::QueueStorageRuntimeMissing]
            ),
            "{fixture}"
        );
        match err {
            ForgeError::Generate(boxed) => match *boxed {
                GenerateError::QueueStorageRuntimeMissing {
                    storage: got,
                    language,
                    implemented,
                    ..
                } => {
                    assert_eq!(got, storage);
                    assert_eq!(language, "rust");
                    assert_eq!(implemented, "bounded");
                }
                other => panic!("expected QueueStorageRuntimeMissing, got {other:?}"),
            },
            other => panic!("expected a generate error, got {other:?}"),
        }
    }
}

#[test]
fn a_deploy_key_capacity_is_not_lowered_as_if_it_were_a_constant() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.m.limits.frames"/>"#,
    );
    let err = compile(&xml, "frame_queue.scxml").expect_err("deploy capacity is not resolved");
    assert!(
        err.error.to_string().contains("machines.m.limits.frames"),
        "the refusal names the key: {err}"
    );
}

// ─── C++ emit ───

#[test]
fn a_lamport_ring_emits_the_cpp_runtime_type_and_no_ring_constants() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let code = compile_for(Language::Cpp, &xml, "frame_queue.scxml").expect("spsc queue emits");
    assert!(
        code.contains("namespace SCE::Generated::FrameQueue {"),
        "{code}"
    );
    assert!(code.contains("#include \"sce/forge/queue.h\""), "{code}");
    assert!(code.contains("#include \"rx_event.h\""), "{code}");
    assert!(
        code.contains("inline constexpr std::size_t CAPACITY = 8;"),
        "{code}"
    );
    assert!(
        code.contains("using FrameQueue = ::SCE::Forge::Queue::Spsc<RxEventType, CAPACITY>;"),
        "{code}"
    );
    assert!(
        code.contains("DECLARED_PROGRESS = \"wait-free\";"),
        "{code}"
    );
    assert!(
        !code.contains("RING_SLOTS") && !code.contains("<atomic>"),
        "a Lamport ring has no index rings:\n{code}"
    );
}

#[test]
fn an_scq_ring_is_sized_the_same_way_in_cpp() {
    // Capacity 6 and 5 participants: the next power of two at or above both.
    let xml = queue_doc(
        "work_queue",
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="6"/><sce:participants const="5"/>"#,
    );
    let code = compile_for(Language::Cpp, &xml, "work_queue.scxml").expect("scq queue emits");
    assert!(
        code.contains("inline constexpr std::size_t RING_SLOTS = 8;"),
        "{code}"
    );
    assert!(
        code.contains("inline constexpr std::size_t PARTICIPANTS = 5;"),
        "{code}"
    );
    assert!(
        code.contains(
            "using WorkQueue = ::SCE::Forge::Queue::Scq<RxEventType, CAPACITY, RING_SLOTS>;"
        ),
        "{code}"
    );
    assert!(
        code.contains("std::atomic<std::uint64_t>::is_always_lock_free"),
        "the row asserts the atomics it needs:\n{code}"
    );
    assert!(
        code.contains("WRAP_BOUND_OPS = ::SCE::Forge::Queue::kWrapBoundOps;"),
        "{code}"
    );
}

#[test]
fn a_storage_the_cpp_runtime_lacks_is_refused_by_name() {
    for (fixture, storage) in [
        ("queue_segmented_lscq.scxml", "segmented"),
        ("queue_intrusive_mpsc.scxml", "intrusive"),
    ] {
        let located = compile_for(Language::Cpp, &resource(fixture), fixture)
            .expect_err("a storage without a runtime is refused");
        let text = resource(fixture);
        let row = located
            .location
            .line
            .expect("the refusal carries the storage element's row");
        let line = text.lines().nth(row as usize - 1).expect("the row exists");
        assert!(
            line.contains(&format!("<sce:{storage}")),
            "{fixture}: row {row} is not the storage element: {line}"
        );
        match located.error {
            ForgeError::Generate(boxed) => match *boxed {
                GenerateError::QueueStorageRuntimeMissing {
                    storage: got,
                    language,
                    implemented,
                    ..
                } => {
                    assert_eq!(got, storage);
                    assert_eq!(language, "cpp");
                    assert_eq!(implemented, "bounded");
                }
                other => panic!("expected QueueStorageRuntimeMissing, got {other:?}"),
            },
            other => panic!("expected a generate error, got {other:?}"),
        }
    }
}

// ─── Go emit ───

const GO_PREFIX: &str = "github.com/acme/project/generated";

fn go_options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some(GO_PREFIX.to_string()),
        ..ForgeCompileOptions::default()
    }
}

#[test]
fn a_lamport_ring_emits_a_go_package_over_the_spsc_runtime_and_no_companion() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let files = compile_files(Language::Go, &go_options(), &xml, "frame_queue.scxml")
        .expect("spsc queue emits");
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["frame_queue.go"], "a Lamport row has no companion");
    let code = &files[0].1;
    assert!(code.contains("package frame_queue\n"), "{code}");
    assert!(
        code.contains("\t\"github.com/newmassrael/sce-forge-runtime/queue\"\n"),
        "{code}"
    );
    assert!(
        code.contains(&format!("\t\"{GO_PREFIX}/rx_event\"\n")),
        "{code}"
    );
    assert!(code.contains("const Capacity = 8\n"), "{code}");
    assert!(
        code.contains("type FrameQueue = queue.Spsc[rx_event.RxEvent]\n"),
        "{code}"
    );
    assert!(
        code.contains("queue.NewSpsc[rx_event.RxEvent](Capacity)"),
        "{code}"
    );
    assert!(
        !code.contains("//go:build") && !code.contains("RingSlots"),
        "a Lamport ring has no build constraint and no index rings:\n{code}"
    );
}

#[test]
fn an_scq_row_is_constrained_to_the_architectures_the_runtime_lists_and_has_a_companion() {
    let xml = queue_doc(
        "work_queue",
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="6"/><sce:participants const="5"/>"#,
    );
    let files = compile_files(Language::Go, &go_options(), &xml, "work_queue.scxml")
        .expect("scq queue emits");
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["work_queue.go", "work_queue_unsupported.go"]);

    let constraint = QUEUE_SCQ_GO_ARCHITECTURES.join(" || ");
    let (code, companion) = (&files[0].1, &files[1].1);
    assert!(
        code.contains(&format!(
            "\n//go:build {constraint}\n\npackage work_queue\n"
        )),
        "the SCQ file names the architectures it can be built on:\n{code}"
    );
    assert!(code.contains("const RingSlots = 8\n"), "{code}");
    assert!(code.contains("const Participants = 5\n"), "{code}");
    assert!(
        code.contains("type WorkQueue = queue.Scq[rx_event.RxEvent]\n"),
        "{code}"
    );
    assert!(
        code.contains("queue.NewScq[rx_event.RxEvent](Capacity, RingSlots)"),
        "{code}"
    );
    assert!(
        companion.contains(&format!(
            "\n//go:build !({constraint})\n\npackage work_queue\n"
        )),
        "the companion holds the negated constraint:\n{companion}"
    );
    assert!(
        companion.contains(
            "var _ = THE_SCQ_QUEUE_NEEDS_NATIVE_64_BIT_ATOMICS_WHICH_THIS_GOARCH_DOES_NOT_HAVE"
        ),
        "the companion fails to compile and names the reason:\n{companion}"
    );
}

/// The generator and the runtime state the architectures an SCQ row can be
/// built on in two places, and this holds them equal: a row the generator
/// writes for an architecture the runtime has no SCQ for would fail to compile
/// with a missing type, the failure the companion exists to prevent.
#[test]
fn the_generators_scq_architectures_are_the_runtimes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("backends/go/forge-runtime/queue/scq.go");
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let constraint: Vec<&str> = source
        .lines()
        .filter_map(|line| line.strip_prefix("//go:build "))
        .collect();
    assert_eq!(
        constraint,
        [QUEUE_SCQ_GO_ARCHITECTURES.join(" || ")],
        "scq.go carries one build constraint, the generator's list"
    );
}

#[test]
fn a_go_queue_without_a_module_prefix_is_refused_not_guessed() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let located = compile_for(Language::Go, &xml, "frame_queue.scxml")
        .expect_err("the element's package cannot be imported without a prefix");
    match located.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::InvalidConfig(message) => {
                assert!(
                    message.contains("queue 'frame_queue'") && message.contains("go_module_prefix"),
                    "{message}"
                );
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

#[test]
fn a_storage_the_go_runtime_lacks_is_refused_by_name() {
    let located = compile_files(
        Language::Go,
        &go_options(),
        &resource("queue_segmented_lscq.scxml"),
        "queue_segmented_lscq.scxml",
    )
    .expect_err("a storage without a runtime is refused");
    match located.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::QueueStorageRuntimeMissing {
                language, storage, ..
            } => {
                assert_eq!(language, "go");
                assert_eq!(storage, "segmented");
            }
            other => panic!("expected QueueStorageRuntimeMissing, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

// ─── Kotlin emit ───

#[test]
fn a_lamport_ring_emits_a_kotlin_file_over_the_spsc_runtime_and_no_ring_constants() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    let code = compile_for(Language::Kotlin, &xml, "frame_queue.scxml").expect("spsc queue emits");
    assert!(
        code.contains("package com.sce.generated.frame_queue"),
        "{code}"
    );
    assert!(
        code.contains("import com.sce.forge.runtime.queue.Spsc"),
        "{code}"
    );
    assert!(
        code.contains("import com.sce.generated.rx_event.RxEvent"),
        "{code}"
    );
    assert!(code.contains("const val CAPACITY: Int = 8"), "{code}");
    assert!(
        code.contains("typealias FrameQueue = Spsc<RxEvent>"),
        "{code}"
    );
    assert!(code.contains("Spsc(CAPACITY)"), "{code}");
    assert!(
        code.contains("const val DECLARED_PROGRESS: String = \"wait-free\""),
        "{code}"
    );
    assert!(
        !code.contains("RING_SLOTS") && !code.contains("PARTICIPANTS"),
        "a Lamport ring has no index rings:\n{code}"
    );
}

#[test]
fn an_scq_ring_is_sized_the_same_way_in_kotlin() {
    // Capacity 6 and 5 participants: the next power of two at or above both.
    let xml = queue_doc(
        "work_queue",
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="6"/><sce:participants const="5"/>"#,
    );
    let code = compile_for(Language::Kotlin, &xml, "work_queue.scxml").expect("scq queue emits");
    assert!(code.contains("const val RING_SLOTS: Int = 8"), "{code}");
    assert!(code.contains("const val PARTICIPANTS: Int = 5"), "{code}");
    assert!(
        code.contains("typealias WorkQueue = Scq<RxEvent>"),
        "{code}"
    );
    assert!(code.contains("Scq(CAPACITY, RING_SLOTS)"), "{code}");
    assert!(
        code.contains("const val WRAP_BOUND_OPS: Long = RUNTIME_WRAP_BOUND_OPS"),
        "{code}"
    );
}

#[test]
fn a_storage_the_kotlin_runtime_lacks_is_refused_by_name() {
    let located = compile_for(
        Language::Kotlin,
        &resource("queue_intrusive_mpsc.scxml"),
        "queue_intrusive_mpsc.scxml",
    )
    .expect_err("a storage without a runtime is refused");
    match located.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::QueueStorageRuntimeMissing {
                language, storage, ..
            } => {
                assert_eq!(language, "kotlin");
                assert_eq!(storage, "intrusive");
            }
            other => panic!("expected QueueStorageRuntimeMissing, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

// ─── Cross-document resolution ───

/// The row a refusal points at, and that row's text.
fn row_text<'a>(located: &Located<ForgeError>, text: &'a str) -> &'a str {
    let row = located.location.line.expect("the refusal carries a row") as usize;
    text.lines()
        .nth(row - 1)
        .unwrap_or_else(|| panic!("the document has no row {row}"))
}

#[test]
fn an_element_type_that_names_no_document_is_refused_with_the_documents_there_are() {
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    )
    .replace("rx_event", "rx_evnt");
    let located =
        compile(&xml, "frame_queue.scxml").expect_err("the element type resolves nowhere");
    assert!(
        row_text(&located, &xml).contains("<sce:element-type>rx_evnt<"),
        "the refusal is placed on the element-type row"
    );
    let diagnostics = located.error.to_diagnostics();
    assert!(matches!(
        diagnostics.as_slice(),
        [d] if matches!(d.code, DiagnosticCode::QueueElementTypeNotAKind)
    ));
    match located.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::QueueElementTypeNotAKind {
                queue_name,
                element_type,
                candidates,
            } => {
                assert_eq!(queue_name, "frame_queue");
                assert_eq!(element_type, "rx_evnt");
                assert_eq!(candidates, ["rx_event"], "the documents in the build");
            }
            other => panic!("expected QueueElementTypeNotAKind, got {other:?}"),
        },
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn an_intrusive_link_field_the_element_lacks_is_refused_with_its_fields() {
    let xml = queue_doc(
        "frame_queue",
        "many",
        "one",
        "blocking",
        r#"<sce:intrusive link-field="nxt"/>"#,
    );
    let located = compile(&xml, "frame_queue.scxml").expect_err("the element has no such field");
    assert!(
        row_text(&located, &xml).contains("link-field=\"nxt\""),
        "the refusal is placed on the storage row"
    );
    match located.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::QueueIntrusiveLinkFieldMissing {
                link_field,
                element_type,
                element_kind,
                candidates,
                ..
            } => {
                assert_eq!(link_field, "nxt");
                assert_eq!(element_type, "rx_event");
                assert_eq!(element_kind, "codec");
                assert_eq!(
                    candidates,
                    ["next", "sensor"],
                    "the element's fields, sorted"
                );
            }
            other => panic!("expected QueueIntrusiveLinkFieldMissing, got {other:?}"),
        },
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn only_an_intrusive_queue_is_asked_for_a_link_field() {
    // A bounded queue writes none, and the element has the field a document
    // would name: neither the element type nor the link field is refused.
    let xml = queue_doc(
        "frame_queue",
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded capacity="8"/>"#,
    );
    compile(&xml, "frame_queue.scxml").expect("a resolvable element type is accepted");
}
