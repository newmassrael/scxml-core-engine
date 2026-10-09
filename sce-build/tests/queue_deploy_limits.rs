//! Queue deploy-time capacity and participants resolution.
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-P: `<sce:bounded>` and
//! `<sce:participants>` take the bounded collection's `CapacitySource` forms, and a
//! `source="deploy"` key `machines.<m>.limits.<k>` resolves against that
//! machine's `limits:` in deploy.yaml when the document is compiled for it.
//! The ring is sized from both numbers, so an unresolved key blocks emission
//! (`queue/deploy-limit-unresolved`, carrying the declared limits as the
//! candidates of its fix) instead of being given a default.
//!
//! The same discipline as `c6_bounded_collection_deploy_capacity.rs`:
//! resolution runs on the deploy-aware path only, and a key that names
//! another machine is left to that machine's own compile.

use sce_build::compile_forge_with_deploy;
use sce_build::forge::diagnostic::{DiagnosticCode, Fix, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::DocumentLabel;

fn deploy_yaml(limits: &str) -> String {
    format!(
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
    )
}

fn queue_doc(producers: &str, consumers: &str, progress: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="queue" name="rx_events" version="1.0">
  <sce:element-type>rx_event</sce:element-type>
  <sce:producers>{producers}</sce:producers>
  <sce:consumers>{consumers}</sce:consumers>
  <sce:progress>{progress}</sce:progress>
  {body}
</scxml>"##
    )
}

/// The generated source for the queue, or why the compile refused.
fn compile(
    scxml: &str,
    yaml: Option<&str>,
    target_machine: Option<&str>,
) -> Result<String, Located<ForgeError>> {
    let deploy = yaml.map(|text| parse_deploy_str(text).expect("deploy parses"));
    let output = compile_forge_with_deploy(
        scxml,
        DocumentLabel::symmetric("rx_events"),
        Language::Rust,
        deploy.as_ref(),
        target_machine,
    )?;
    Ok(output
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn unresolved(err: Located<ForgeError>) -> (ValidationError, Vec<Fix>) {
    let fixes = err
        .error
        .to_diagnostics()
        .iter()
        .filter_map(|d| d.fix.clone())
        .collect();
    match err.error {
        ForgeError::Validation(boxed) => (*boxed, fixes),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn a_deploy_capacity_and_participants_size_the_ring() {
    // Capacity 10 and 5 participants: the ring is the next power of two at or
    // above both, which is sixteen.
    let scxml = queue_doc(
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>
  <sce:participants source="deploy" key="machines.mcu_node.limits.rx_participants"/>"#,
    );
    let yaml = deploy_yaml("          rx_capacity: 10\n          rx_participants: 5");
    let code = compile(&scxml, Some(&yaml), Some("mcu_node")).expect("deploy keys resolve");
    assert!(code.contains("pub const CAPACITY: usize = 10;"), "{code}");
    assert!(
        code.contains("pub const PARTICIPANTS: usize = 5;"),
        "{code}"
    );
    assert!(code.contains("pub const RING_SLOTS: usize = 16;"), "{code}");
}

#[test]
fn a_deploy_capacity_sizes_a_lamport_ring() {
    let scxml = queue_doc(
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>"#,
    );
    let yaml = deploy_yaml("          rx_capacity: 12");
    let code = compile(&scxml, Some(&yaml), Some("mcu_node")).expect("deploy key resolves");
    assert!(code.contains("pub const CAPACITY: usize = 12;"), "{code}");
}

#[test]
fn a_deploy_capacity_reaches_every_backend_that_takes_the_deploy_entry() {
    // Kotlin, Go and Python used to be handed the options-less generators, so a
    // `source="deploy"` capacity was refused as unresolved on them whatever the
    // deploy said. Go also needs a module prefix the deploy entry does not carry,
    // so it is judged by the Go arm's own tests.
    let scxml = queue_doc(
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>"#,
    );
    let yaml = deploy_yaml("          rx_capacity: 12");
    for (language, needle) in [
        (Language::Rust, "pub const CAPACITY: usize = 12;"),
        (Language::Cpp, "CAPACITY = 12;"),
        (Language::Kotlin, "const val CAPACITY: Int = 12"),
        (Language::C11, "RX_EVENTS_CAPACITY ((uint32_t)12)"),
    ] {
        let deploy = parse_deploy_str(&yaml).expect("deploy parses");
        let output = compile_forge_with_deploy(
            &scxml,
            DocumentLabel::symmetric("rx_events"),
            language,
            Some(&deploy),
            Some("mcu_node"),
        )
        .unwrap_or_else(|e| panic!("{language:?} resolves the deploy key: {e:?}"));
        let text: String = output.files.iter().map(|(_, c)| c.as_str()).collect();
        assert!(text.contains(needle), "{language:?}: {text}");
    }
    // Python gives every queue `blocking`, so its document declares that.
    let blocking = queue_doc(
        "one",
        "one",
        "blocking",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>"#,
    );
    let deploy = parse_deploy_str(&yaml).expect("deploy parses");
    let output = compile_forge_with_deploy(
        &blocking,
        DocumentLabel::symmetric("rx_events"),
        Language::Python,
        Some(&deploy),
        Some("mcu_node"),
    )
    .expect("Python resolves the deploy key");
    let text: String = output.files.iter().map(|(_, c)| c.as_str()).collect();
    assert!(text.contains("CAPACITY: Final[int] = 12"), "{text}");
}

#[test]
fn a_constant_and_a_deploy_key_may_be_mixed() {
    let scxml = queue_doc(
        "many",
        "one",
        "lock-free",
        r#"<sce:bounded capacity="6"/>
  <sce:participants source="deploy" key="machines.mcu_node.limits.rx_participants"/>"#,
    );
    let yaml = deploy_yaml("          rx_participants: 3");
    let code = compile(&scxml, Some(&yaml), Some("mcu_node")).expect("mixed forms resolve");
    assert!(code.contains("pub const CAPACITY: usize = 6;"), "{code}");
    assert!(
        code.contains("pub const PARTICIPANTS: usize = 3;"),
        "{code}"
    );
}

#[test]
fn an_undeclared_capacity_limit_is_refused_with_the_declared_ones_as_candidates() {
    let scxml = queue_doc(
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>"#,
    );
    let yaml = deploy_yaml("          tx_capacity: 4\n          alpha: 2");
    let err = compile(&scxml, Some(&yaml), Some("mcu_node")).expect_err("undeclared limit");
    let (error, fixes) = unresolved(err);
    match error {
        ValidationError::QueueDeployLimitUnresolved {
            queue_name,
            element,
            key,
            machine,
            limit,
            candidates,
        } => {
            assert_eq!(queue_name, "rx_events");
            assert_eq!(element, "bounded");
            assert_eq!(key, "machines.mcu_node.limits.rx_capacity");
            assert_eq!(machine, "mcu_node");
            assert_eq!(limit, "rx_capacity");
            assert_eq!(
                candidates,
                ["alpha", "tx_capacity"],
                "the declared limits, sorted"
            );
        }
        other => panic!("expected QueueDeployLimitUnresolved, got {other:?}"),
    }
    assert!(
        matches!(fixes.as_slice(), [Fix::ReplaceOneOf { candidates }] if candidates == &["alpha", "tx_capacity"]),
        "{fixes:?}"
    );
}

#[test]
fn an_undeclared_participants_limit_names_the_participants_element() {
    let scxml = queue_doc(
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="8"/>
  <sce:participants source="deploy" key="machines.mcu_node.limits.rx_participants"/>"#,
    );
    let yaml = deploy_yaml("          tx_capacity: 4");
    let err = compile(&scxml, Some(&yaml), Some("mcu_node")).expect_err("undeclared limit");
    let codes: Vec<_> = err.error.to_diagnostics().iter().map(|d| d.code).collect();
    assert!(matches!(
        codes.as_slice(),
        [DiagnosticCode::QueueDeployLimitUnresolved]
    ));
    let (error, _) = unresolved(err);
    assert!(matches!(
        error,
        ValidationError::QueueDeployLimitUnresolved { ref element, .. } if element == "participants"
    ));
}

#[test]
fn a_key_that_names_another_machine_is_left_to_that_machines_compile() {
    let scxml = queue_doc(
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.other_node.limits.rx_capacity"/>"#,
    );
    let yaml = deploy_yaml("          tx_capacity: 4");
    let err = compile(&scxml, Some(&yaml), Some("mcu_node"))
        .expect_err("another machine's key is not resolved here");
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::InvalidConfig(message) => {
                assert!(message.contains("resolution missing"), "{message}");
                assert!(
                    message.contains("machines.other_node.limits.rx_capacity"),
                    "{message}"
                );
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

#[test]
fn a_deploy_key_without_a_deploy_file_is_refused_not_defaulted() {
    let scxml = queue_doc(
        "one",
        "one",
        "wait-free",
        r#"<sce:bounded source="deploy" key="machines.mcu_node.limits.rx_capacity"/>"#,
    );
    let err = compile(&scxml, None, None).expect_err("no deploy file, no number");
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::InvalidConfig(message) => {
                assert!(message.contains("rx_events"), "{message}");
                assert!(message.contains("compile_forge_with_deploy"), "{message}");
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

// ─── What the deploy says of the target's atomics (C11) ───

/// A deploy for the machine `mcu_node` with a `platform:` block and, when
/// `queue_block` is not empty, a `queues:` block under it.
fn platform_deploy_yaml(platform_extra: &str, queue_block: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
{platform_extra}
{queue_block}
"##
    )
}

/// The C11 header for the queue, or why the compile refused.
fn compile_c(scxml: &str, yaml: &str) -> Result<String, Located<ForgeError>> {
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    let output = compile_forge_with_deploy(
        scxml,
        DocumentLabel::symmetric("rx_events"),
        Language::C11,
        Some(&deploy),
        Some("mcu_node"),
    )?;
    Ok(output
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn scq_queue() -> String {
    queue_doc(
        "many",
        "many",
        "lock-free",
        r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
    )
}

#[test]
fn the_platforms_atomic_width_chooses_the_ring_a_c11_queue_is_built_on() {
    let scxml = scq_queue();

    let wide = platform_deploy_yaml("          atomic_rmw_width: 64", "");
    let code = compile_c(&scxml, &wide).expect("64-bit atomics build SCQ");
    assert!(code.contains("sce_queue_scq64_t core;"), "{code}");

    let narrow = platform_deploy_yaml(
        "          atomic_rmw_width: 32",
        "        queues:\n          rx_events:\n            min_wrap_ops: 1000000",
    );
    let code = compile_c(&scxml, &narrow).expect("32-bit atomics build SCQ with a stated delay");
    assert!(code.contains("sce_queue_scq32_t core;"), "{code}");

    let none = platform_deploy_yaml("          atomic_rmw_width: 0\n          core_count: 1", "");
    let blocking = queue_doc(
        "many",
        "many",
        "blocking",
        r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
    );
    let code =
        compile_c(&blocking, &none).expect("a single core with no atomics gets the irq ring");
    assert!(code.contains("sce_queue_irq_t core;"), "{code}");
}

#[test]
fn a_deploy_that_states_no_atomic_width_leaves_an_scq_row_on_c11_refused() {
    let yaml = platform_deploy_yaml("          core_count: 4", "");
    let err = compile_c(&scq_queue(), &yaml).expect_err("the generator does not guess");
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::QueueAtomicWidthUnstated { queue_name, .. } => {
                assert_eq!(queue_name, "rx_events");
            }
            other => panic!("expected QueueAtomicWidthUnstated, got {other:?}"),
        },
        other => panic!("expected a generate error, got {other:?}"),
    }
}

#[test]
fn the_deploy_refuses_a_width_the_rfc_has_no_queue_for() {
    for width in ["16", "128", "8"] {
        let yaml = platform_deploy_yaml(&format!("          atomic_rmw_width: {width}"), "");
        let message = match parse_deploy_str(&yaml) {
            Ok(_) => panic!("atomic_rmw_width {width} must be refused where the deploy is read"),
            Err(e) => e.to_string(),
        };
        assert!(
            message.contains("atomic_rmw_width must be 0, 32 or 64"),
            "{width}: {message}"
        );
    }
}

// ─── Placement: which side runs in an interrupt handler ───

/// A `queues:` block that places each side of `rx_events` as the arguments say
/// (`thread` or `isr`), under the keys of a deploy for machine `mcu_node`.
fn placement_block(producer: &str, consumer: &str, extra_queue_keys: &str) -> String {
    format!(
        "        queues:\n          rx_events:\n{extra_queue_keys}            placement:\n              producers:\n                - context: {producer}\n                  cores: [0]\n              consumers:\n                - context: {consumer}\n                  cores: [0]"
    )
}

/// The generated text for `scxml` on `language` for machine `mcu_node` of `yaml`.
fn compile_for_language(
    language: Language,
    scxml: &str,
    yaml: &str,
) -> Result<String, Located<ForgeError>> {
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    let output = compile_forge_with_deploy(
        scxml,
        DocumentLabel::symmetric("rx_events"),
        language,
        Some(&deploy),
        Some("mcu_node"),
    )?;
    Ok(output
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn generate(err: Located<ForgeError>) -> GenerateError {
    match err.error {
        ForgeError::Generate(boxed) => *boxed,
        other => panic!("expected a generate error, got {other:?}"),
    }
}

#[test]
fn an_isr_side_may_use_any_queue_that_is_lock_free_or_better() {
    let yaml = platform_deploy_yaml(
        "          atomic_rmw_width: 64",
        &placement_block("isr", "isr", ""),
    );
    // SCQ is lock-free on both sides; the Lamport ring is wait-free.
    for (producers, consumers, progress, body) in [
        (
            "many",
            "many",
            "lock-free",
            r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
        ),
        ("one", "one", "wait-free", r#"<sce:bounded capacity="6"/>"#),
    ] {
        let scxml = queue_doc(producers, consumers, progress, body);
        compile_for_language(Language::C11, &scxml, &yaml)
            .unwrap_or_else(|e| panic!("{producers}/{consumers} with both sides in an ISR: {e:?}"));
        compile_for_language(Language::Rust, &scxml, &yaml)
            .unwrap_or_else(|e| panic!("{producers}/{consumers} on Rust: {e:?}"));
    }
}

#[test]
fn an_isr_side_cannot_use_a_queue_that_blocks() {
    // Python's queue is one ring under one lock: every row blocks.
    let scxml = queue_doc(
        "many",
        "many",
        "blocking",
        r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
    );
    for (producer, consumer, side, operation) in [
        ("isr", "thread", "producer", "push"),
        ("thread", "isr", "consumer", "pop"),
    ] {
        let yaml = platform_deploy_yaml(
            "          atomic_rmw_width: 64",
            &placement_block(producer, consumer, ""),
        );
        let parsed = parse_deploy_str(&yaml).expect("deploy parses");
        let placement = parsed
            .device_for_machine("mcu_node")
            .and_then(|device| device.machines.get("mcu_node"))
            .and_then(|machine| machine.queues.get("rx_events"))
            .and_then(|queue| queue.placement.as_ref())
            .expect("the deploy's placement is read");
        assert_eq!(placement.producers.len(), 1, "{placement:?}");
        let err = compile_for_language(Language::Python, &scxml, &yaml)
            .expect_err("a blocking queue is not for an interrupt handler");
        match generate(err) {
            GenerateError::QueueProgressInsufficientForIsr {
                queue_name,
                side: got_side,
                operation: got_operation,
                reachable,
                ..
            } => {
                assert_eq!(queue_name, "rx_events");
                assert_eq!(got_side, side);
                assert_eq!(got_operation, operation);
                assert_eq!(reachable, "blocking");
            }
            other => panic!("expected QueueProgressInsufficientForIsr, got {other:?}"),
        }
        // The same placement is fine in a thread on both sides.
        let threads = platform_deploy_yaml(
            "          atomic_rmw_width: 64",
            &placement_block("thread", "thread", ""),
        );
        compile_for_language(Language::Python, &scxml, &threads)
            .expect("a blocking queue is for threads");
    }
}

#[test]
fn the_interrupt_masked_ring_is_the_one_blocking_queue_an_isr_may_use() {
    // A single-core target without atomics: no handler can preempt the section.
    let scxml = queue_doc(
        "many",
        "many",
        "blocking",
        r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
    );
    let yaml = platform_deploy_yaml(
        "          atomic_rmw_width: 0\n          core_count: 1",
        &placement_block("isr", "isr", ""),
    );
    let code = compile_for_language(Language::C11, &scxml, &yaml)
        .expect("the interrupt-masked ring serves an ISR on one core");
    assert!(code.contains("sce_queue_irq_t core;"), "{code}");
}

#[test]
fn a_deploy_that_places_nothing_asks_nothing_about_placement() {
    let scxml = queue_doc(
        "many",
        "many",
        "blocking",
        r#"<sce:bounded capacity="6"/><sce:participants const="3"/>"#,
    );
    let yaml = platform_deploy_yaml("          atomic_rmw_width: 64", "");
    compile_for_language(Language::Python, &scxml, &yaml)
        .expect("without a placement there is no ISR side to judge");
}

const SEGMENTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="queue" name="rx_events" version="1.0">
  <sce:element-type>rx_event</sce:element-type>
  <sce:producers>many</sce:producers>
  <sce:consumers>many</sce:consumers>
  <sce:progress>lock-free</sce:progress>
  <sce:segmented segment="64" allocator-progress="lock-free"/>
  <sce:participants const="4"/>
</scxml>"#;

#[test]
fn a_segmented_queue_is_refused_where_nothing_allocates_or_where_a_handler_pushes() {
    // The no-alloc profile is a machine of platform.class mcu.
    let mcu = platform_deploy_yaml("          atomic_rmw_width: 64", "");
    let err = compile_for_language(Language::Rust, SEGMENTED, &mcu)
        .expect_err("an MCU machine has no allocator to bound the queue");
    match generate(err) {
        GenerateError::QueueSegmentedNeedsAlloc {
            queue_name,
            platform_class,
        } => {
            assert_eq!(queue_name, "rx_events");
            assert_eq!(platform_class, "mcu");
        }
        other => panic!("expected QueueSegmentedNeedsAlloc, got {other:?}"),
    }

    // A machine with a heap, and a producer in an interrupt handler.
    let ap = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: ap
          os: linux
        queues:
          rx_events:
            placement:
              producers:
                - context: isr
                  cores: [0]
              consumers:
                - context: thread
                  cores: [0]
"##;
    let err = compile_for_language(Language::Rust, SEGMENTED, ap)
        .expect_err("a push that may allocate is not for an interrupt handler");
    match generate(err) {
        GenerateError::QueueAllocInIsr { queue_name } => assert_eq!(queue_name, "rx_events"),
        other => panic!("expected QueueAllocInIsr, got {other:?}"),
    }

    // Neither applies to a thread producer on a machine with a heap: the queue is
    // then refused only for the runtime it does not have yet.
    let threads = ap.replace("context: isr", "context: thread");
    let err = compile_for_language(Language::Rust, SEGMENTED, &threads)
        .expect_err("segmented has no runtime yet");
    assert!(
        matches!(
            generate(err),
            GenerateError::QueueStorageRuntimeMissing { .. }
        ),
        "the placement checks do not change what a missing runtime says"
    );
}

#[test]
fn the_deploy_refuses_a_context_the_rfc_has_no_word_for() {
    let yaml = platform_deploy_yaml(
        "          atomic_rmw_width: 64",
        &placement_block("coroutine", "thread", ""),
    );
    let message = match parse_deploy_str(&yaml) {
        Ok(_) => panic!("a placement context is thread or isr"),
        Err(e) => e.to_string(),
    };
    assert!(
        message.contains("coroutine") || message.contains("context"),
        "{message}"
    );
}

#[test]
fn a_queue_the_deploy_names_only_in_another_machine_states_nothing_about_this_one() {
    let yaml = platform_deploy_yaml(
        "          atomic_rmw_width: 32",
        "        queues:\n          another_queue:\n            min_wrap_ops: 1000",
    );
    let err = compile_c(&scq_queue(), &yaml)
        .expect_err("min_wrap_ops of another queue is not this one's");
    match err.error {
        ForgeError::Generate(boxed) => assert!(
            matches!(*boxed, GenerateError::QueueWrapBoundUnstated { .. }),
            "{boxed:?}"
        ),
        other => panic!("expected a generate error, got {other:?}"),
    }
}
