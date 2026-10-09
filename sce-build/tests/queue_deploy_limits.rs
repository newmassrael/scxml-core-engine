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
