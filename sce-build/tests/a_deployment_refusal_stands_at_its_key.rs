// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A refusal of a deployment carries the row and column of the key it names
//! (docs/adr/0106, #76): the key path it already names is looked up in a marked
//! reading of the same text. Where the record carries `actual`, the column is
//! where `actual` begins (SCE_ERROR_CONTRACT.md section 2.2).

use sce_build::mesh::deploy::{parse_deploy_str, validate_links_cross_doc};
use sce_build::mesh::deploy_marks::{DeployKeyPath, Mark};
use sce_build::mesh::error::{DeployError, MeshError};

fn refusal(text: &str) -> DeployError {
    parse_deploy_str(text).expect_err("the deployment is refused")
}

#[test]
fn an_unsupported_version_stands_at_the_version_written() {
    let text = "version: \"2.0\"\ntopology: {}\n";
    let err = refusal(text);
    assert!(
        matches!(err, DeployError::UnsupportedVersion { .. }),
        "{err:?}"
    );
    assert_eq!(err.key_path(), Some(DeployKeyPath::root("version")));
    // `2.0` begins after the quote at column 10.
    assert_eq!(err.mark_in(text), Some(Mark { line: 1, col: 11 }));
}

#[test]
fn an_unknown_driver_stands_at_the_driver_its_link_names() {
    let text = "\
version: \"1.0\"
topology:
  mcu:
    machines:
      door:
        source: door.scxml
        links:
          udp_scout:
            bind: \"224.0.0.224:7446\"
            driver: lwip_udpp
";
    let err = refusal(text);
    assert!(
        matches!(err, DeployError::LinkDriverUnknown { .. }),
        "{err:?}"
    );
    assert_eq!(
        err.key_path(),
        Some(DeployKeyPath::link("door", "udp_scout").key("driver"))
    );
    assert_eq!(err.mark_in(text), Some(Mark { line: 10, col: 21 }));
}

#[test]
fn a_missing_key_stands_where_its_actual_is_written() {
    // `worker_stack_budget` is not written; the refusal carries the machine as
    // its `actual`, so it stands on the machine's key.
    let text = "\
version: \"1.0\"
topology:
  mcu:
    machines:
      door:
        source: door.scxml
        scheduler:
          kind: cooperative
";
    let err = refusal(text);
    assert!(
        matches!(
            err,
            DeployError::SchedulerCooperativeMissingStackBudget { .. }
        ),
        "{err:?}"
    );
    assert_eq!(err.mark_in(text), Some(Mark { line: 5, col: 7 }));
}

#[test]
fn a_refusal_after_the_parse_is_placed_through_the_text_the_deployment_keeps() {
    let text = "\
version: \"1.0\"
topology:
  mcu:
    machines:
      door:
        source: door.scxml
        links:
          udp_scout:
            bind: \"224.0.0.224:7446\"
            driver: lwip_udp
";
    let cfg = parse_deploy_str(text).expect("the deployment is accepted");
    let err = MeshError::from(
        validate_links_cross_doc(&cfg, &[]).expect_err("no forge document declares the link"),
    );
    // `udp_scout` is the record's `actual`, written at row 8.
    assert_eq!(cfg.mark_of(&err), Some(Mark { line: 8, col: 11 }));
}

#[test]
fn a_refusal_of_the_yaml_carries_its_place_apart_from_its_words() {
    // `topolgy` is no key of a deployment; serde reads the refusal on it, at
    // the start of row 2.
    let text = "version: \"1.0\"\ntopolgy: {}\n";
    let err = refusal(text);
    let DeployError::Yaml { message, at } = &err else {
        panic!("the schema refuses the key: {err:?}");
    };
    assert_eq!(*at, Some(Mark { line: 2, col: 1 }));
    assert_eq!(
        err.mark_in(text),
        *at,
        "the record stands where serde read it"
    );
    assert!(
        message.contains("unknown field `topolgy`"),
        "serde's words stay: {message}"
    );
    assert!(
        !message.contains(" at line "),
        "the place is the record's location, not part of its words: {message}"
    );
}

#[test]
fn a_refusal_of_the_yaml_syntax_keeps_its_context_without_its_place() {
    let text = "version: [unterminated\n";
    let err = refusal(text);
    let DeployError::Yaml { message, at } = &err else {
        panic!("the YAML is refused: {err:?}");
    };
    let mark = at.expect("serde places a syntax refusal");
    assert!(
        !message.contains(&format!(" at line {} column {}", mark.line, mark.col)),
        "the place serde gives is not repeated in the words: {message}"
    );
    assert!(!message.is_empty());
}

#[test]
fn a_refusal_of_no_key_is_not_placed() {
    let err = DeployError::SomeipLivenessServiceIdOverflow {
        participant_count: 70_000,
        ceiling: 65_535,
    };
    assert_eq!(err.key_path(), None);
    assert_eq!(err.mark_in("version: \"1.0\"\n"), None);
}
