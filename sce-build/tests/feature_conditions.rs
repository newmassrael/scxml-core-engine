// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Conditions on a statechart's elements (ADR 0057 §3, §5): what a parse reads
//! of them, what it refuses and where, the variant a configuration denotes, and
//! the family check that no variant keeps a reference to a state it removed.

use sce_build::feature_model::Configuration;
use sce_build::forge::error::{ForgeError, Located, RelatedRole, ValidationError};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

const FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
  <sce:feature name="session-auth"/>
</scxml>
"#;

/// A statechart that imports the model on line 4; `body` starts on line 5.
fn machine(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" name="machine">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
{body}
</scxml>
"#
    )
}

fn parse(document: &str) -> Result<SCXMLModel, Located<ForgeError>> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let path = dir.path().join("machine.scxml");
    std::fs::write(&path, document).unwrap();
    SCXMLParser::new().parse_file(path.to_str().unwrap())
}

fn refusal(document: &str) -> Located<ForgeError> {
    parse(document).expect_err("the document is refused")
}

/// The validation error a record carries.
fn validation(err: &Located<ForgeError>) -> &ValidationError {
    match &err.error {
        ForgeError::Validation(v) => v,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn place(err: &Located<ForgeError>) -> (Option<u32>, Option<u32>) {
    (err.location.line, err.location.col)
}

const ACCEPT_PATH: &str = r#"  <state id="idle">
    <onentry><sce:action name="announce" effect="notice" requires="session-auth"/></onentry>
    <transition event="inbound.start" target="accepting" sce:requires="session-unicast-accept"/>
    <transition event="connect" target="open" sce:requires="session-unicast"/>
  </state>
  <state id="accepting" sce:requires="session-unicast-accept">
    <transition event="done" target="open"/>
  </state>
  <state id="open" sce:requires="session-unicast"/>"#;

#[test]
fn a_parse_reads_each_condition_and_the_condition_its_element_is_present_under() {
    let model = parse(&machine(ACCEPT_PATH)).expect("the document parses");
    let line = model.product_line.as_ref().expect("a product line");
    // A forge document is named by its file, as every kind is.
    assert_eq!(line.model_name(), "features");
    let gates: Vec<(&str, String)> = line
        .gates()
        .iter()
        .map(|gate| {
            let spelled = gate
                .effective
                .canonical_form(line.model())
                .map(|form| form.to_string())
                .unwrap_or_default();
            (gate.tag.as_str(), spelled)
        })
        .collect();
    assert_eq!(
        gates,
        [
            ("sce:action", "session-auth".to_string()),
            ("transition", "session-unicast-accept".to_string()),
            ("transition", "session-unicast".to_string()),
            ("state", "session-unicast-accept".to_string()),
            ("state", "session-unicast".to_string()),
        ]
    );
}

#[test]
fn a_variant_blanks_what_its_configuration_removes_and_keeps_every_position() {
    let source = machine(ACCEPT_PATH);
    let model = parse(&source).expect("the document parses");
    let line = model.product_line.as_ref().expect("a product line");
    let unicast = line.model().select(["session-unicast"]).unwrap();
    let derived = line.derive(&source, &unicast);

    // Every line stays where it was, and so does every column of what stays.
    assert_eq!(derived.lines().count(), source.lines().count());
    for (kept, original) in derived.lines().zip(source.lines()) {
        assert_eq!(kept.chars().count(), original.chars().count());
    }
    assert!(!derived.contains("accepting"), "{derived}");
    assert!(!derived.contains("announce"), "{derived}");
    assert!(!derived.contains("requires"), "{derived}");
    assert!(!derived.contains("feature-model"), "{derived}");
    let open = source.find(r#"<state id="open""#).unwrap();
    assert_eq!(&derived[open..open + 16], r#"<state id="open""#);

    // The variant is a plain document: no import, no condition.
    let variant = SCXMLParser::new()
        .parse_string(&derived, "machine")
        .unwrap_or_else(|e| panic!("the variant parses: {e:?}"));
    assert!(variant.product_line.is_none());
    assert!(variant.states.contains_key("open"));
    assert!(!variant.states.contains_key("accepting"));
}

#[test]
fn each_distinct_variant_is_named_by_its_least_configuration() {
    let model = parse(&machine(ACCEPT_PATH)).expect("the document parses");
    let line = model.product_line.as_ref().expect("a product line");
    let names = |c: &Configuration| line.model().names_of(c).collect::<Vec<_>>().join(" ");
    let variants: Vec<String> = line.variants().iter().map(names).collect();
    assert_eq!(
        variants,
        // The model's order puts session-auth first: it and session-unicast
        // require nothing and come by name, and session-unicast-accept comes
        // after what it requires.
        [
            "",
            "session-auth",
            "session-unicast",
            "session-auth session-unicast",
            "session-unicast session-unicast-accept",
            "session-auth session-unicast session-unicast-accept",
        ]
    );
}

#[test]
fn a_transition_that_stays_where_its_target_goes_is_refused_with_the_least_configuration() {
    let err = refusal(&machine(
        r#"  <state id="idle">
    <transition event="inbound.start" target="accepting" sce:requires="session-unicast"/>
  </state>
  <state id="accepting" sce:requires="session-unicast-accept"/>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(
            error,
            ValidationError::FeatureGateEscapes { element, attribute, referent, witness }
                if element == "transition" && attribute == "target" && referent == "accepting"
                    && witness == &["session-unicast"]
        ),
        "{error:?}"
    );
    // On the value of `target`, which spells the id the record quotes.
    assert_eq!(place(&err), (Some(6), Some(47)));
    let related = err.related();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].role, RelatedRole::Referent);
    assert_eq!(related[0].location.line, Some(8));
}

#[test]
fn an_ungated_reference_to_a_gated_state_dangles_where_no_feature_is_on() {
    let err = refusal(&machine(
        r#"  <state id="idle"><transition event="go" target="open"/></state>
  <state id="open" sce:requires="session-unicast"/>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(error, ValidationError::FeatureGateEscapes { witness, .. } if witness.is_empty()),
        "{error:?}"
    );
    // The document's own `initial` names a state the configuration without
    // session-auth removes.
    let err = refusal(&machine(
        r#"  <state id="idle" sce:requires="session-auth"/>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(
            error,
            ValidationError::FeatureGateEscapes { element, attribute, .. }
                if element == "scxml" && attribute == "initial"
        ),
        "{error:?}"
    );
}

#[test]
fn a_reference_whose_condition_implies_its_targets_is_admitted() {
    parse(&machine(
        r#"  <state id="idle">
    <transition event="go" target="accepting" sce:requires="all(session-unicast-accept, session-auth)"/>
  </state>
  <state id="accepting" sce:requires="any(session-unicast-accept, session-auth)"/>"#,
    ))
    .expect("accept and auth together hold wherever accept or auth does");
}

#[test]
fn a_name_the_model_does_not_declare_is_refused_where_it_is_written() {
    let err = refusal(&machine(
        r#"  <state id="idle"/>
  <state id="other" sce:requires="any(session-auth, session-unicst)"/>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(
            error,
            ValidationError::FeatureUndeclared { feature, model: Some(model) }
                if feature == "session-unicst" && model == "features"
        ),
        "{error:?}"
    );
    // The value starts on column 35; the name is 18 bytes into it.
    assert_eq!(place(&err), (Some(6), Some(53)));
}

#[test]
fn a_condition_in_a_document_that_imports_no_model_is_refused() {
    let err = refusal(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" name="machine">
  <state id="idle" sce:requires="session-auth"/>
</scxml>"#,
    );
    let error = validation(&err);
    assert!(
        matches!(
            error,
            ValidationError::FeatureUndeclared { feature, model: None } if feature == "session-auth"
        ),
        "{error:?}"
    );
}

#[test]
fn a_condition_that_negates_is_refused_and_says_how_to_write_it() {
    let err = refusal(&machine(
        r#"  <state id="idle"/>
  <state id="other" sce:requires="not(session-auth)"/>"#,
    ));
    let ValidationError::FeatureConditionMalformed { condition, reason } = validation(&err) else {
        panic!("{:?}", err.error);
    };
    assert_eq!(condition, "not(session-auth)");
    assert!(reason.contains("cannot negate"), "{reason}");
    assert_eq!(place(&err), (Some(6), Some(35)));
}

#[test]
fn an_element_that_takes_no_condition_is_refused() {
    for (body, element) in [
        (
            r#"  <state id="idle"><initial sce:requires="session-auth"><transition target="a"/></initial><state id="a"/></state>"#,
            "initial",
        ),
        (
            r#"  <datamodel><data id="x" sce:requires="session-auth"/></datamodel>
  <state id="idle"/>"#,
            "data",
        ),
        (
            r#"  <state id="idle"><onentry><if cond="true"><log expr="1"/><else sce:requires="session-auth"/></if></onentry></state>"#,
            "else",
        ),
        (r#"  <state id="idle"/>"#, "sce:import"),
    ] {
        let document = if element == "sce:import" {
            machine(body).replace(
                r#"as="features"/>"#,
                r#"as="features" requires="session-auth"/>"#,
            )
        } else {
            machine(body)
        };
        let err = refusal(&document);
        let error = validation(&err);
        assert!(
            matches!(error, ValidationError::FeatureGateUnsupported { element: e, .. } if e == element),
            "{element}: {error:?}"
        );
    }
}

#[test]
fn a_condition_that_removes_nothing_beyond_its_enclosing_condition_is_refused() {
    let err = refusal(&machine(
        r#"  <state id="idle"/>
  <state id="accepting" sce:requires="session-unicast-accept">
    <transition event="done" target="accepting" sce:requires="session-unicast"/>
  </state>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(
            error,
            ValidationError::FeatureConditionRedundant { condition, enclosing, .. }
                if condition == "session-unicast" && enclosing == "session-unicast-accept"
        ),
        "{error:?}"
    );
}

#[test]
fn a_second_feature_model_is_refused() {
    let err = refusal(&machine(
        r#"  <sce:import kind="feature-model" src="features.scxml" as="more"/>
  <state id="idle"/>"#,
    ));
    let error = validation(&err);
    assert!(
        matches!(error, ValidationError::FeatureModelImportedTwice { .. }),
        "{error:?}"
    );
    assert_eq!(err.location.line, Some(5));
}

#[test]
fn a_feature_model_is_not_read_by_a_parse_with_no_directory() {
    let err = SCXMLParser::new()
        .parse_string(&machine(r#"  <state id="idle"/>"#), "machine")
        .expect_err("the import cannot be read");
    let error = validation(&err);
    assert!(
        matches!(error, ValidationError::FeatureModelNotRead { src } if src == "features.scxml"),
        "{error:?}"
    );
}

#[test]
fn a_condition_on_an_import_of_a_forge_document_is_refused() {
    let err = sce_build::forge::parser::parse_forge_with_imports(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="algorithm" name="f">
  <sce:import kind="enum" src="e.scxml" as="e" requires="session-auth"/>
</scxml>"#,
        sce_build::DocumentLabel {
            identifier: "f",
            diagnostic_label: "f.scxml",
        },
    )
    .expect_err("the condition is refused");
    let error = validation(&err);
    assert!(
        matches!(error, ValidationError::FeatureGateUnsupported { element, .. } if element == "sce:import"),
        "{error:?}"
    );
}

/// The family check of every variant (ADR 0057 §5), on a document kept on
/// disk for as long as the check reads it.
fn check_every_variant(document: &str) -> Result<(), Located<ForgeError>> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let path = dir.path().join("machine.scxml");
    std::fs::write(&path, document).unwrap();
    let path = path.to_str().unwrap();
    let mut model = SCXMLParser::new()
        .parse_file(path)
        .expect("the document as written parses");
    sce_build::judge_statechart(&mut model, path, path, true).expect("and is accepted");
    sce_build::check_every_variant(&model, path, &[], true)
}

#[test]
fn every_variant_of_a_sound_product_line_is_accepted() {
    check_every_variant(&machine(ACCEPT_PATH)).expect("every variant is a document");
}

#[test]
fn a_variant_refused_by_a_rule_the_document_passes_is_named_by_its_least_configuration() {
    // Without session-unicast the document keeps no state at all, which no
    // statechart may be; the document itself has one.
    let document = machine(r#"  <state id="open" sce:requires="session-unicast"/>"#)
        .replace(r#" initial="idle""#, "");
    let err = check_every_variant(&document).expect_err("a variant is refused");
    let ValidationError::FeatureVariantIllFormed { witness, code, .. } = validation(&err) else {
        panic!("{:?}", err.error);
    };
    assert!(
        witness.is_empty(),
        "the least configuration is the empty one: {witness:?}"
    );
    assert!(
        !code.contains("/feature-"),
        "the inner refusal is the document rule the variant breaks: {code}"
    );
}

/// Each element of the model carries the selector its items are emitted under
/// (ADR 0057 §9): its effective condition, and nothing where it is present in
/// every variant.
#[test]
fn each_element_of_the_model_carries_the_condition_it_is_present_under() {
    let model = parse(&machine(ACCEPT_PATH)).expect("the document parses");
    let spelled = |p: &Option<sce_build::feature_selector::Selector>| p.as_ref().map(|s| s.rust());
    let accept = Some(r#"feature = "session-unicast-accept""#.to_string());
    let unicast = Some(r#"feature = "session-unicast""#.to_string());
    assert_eq!(spelled(&model.states["idle"].presence), None);
    assert_eq!(spelled(&model.states["accepting"].presence), accept);
    assert_eq!(spelled(&model.states["open"].presence), unicast);
    let idle = &model.states["idle"];
    let by_event = |event: &str| {
        idle.transitions
            .iter()
            .find(|t| t.event == event)
            .unwrap_or_else(|| panic!("a transition on {event}"))
    };
    assert_eq!(spelled(&by_event("inbound.start").presence), accept);
    assert_eq!(spelled(&by_event("connect").presence), unicast);
    // A transition inside a gated state takes its state's condition.
    assert_eq!(
        spelled(&model.states["accepting"].transitions[0].presence),
        accept
    );
    let announce = &idle.on_entry_blocks[0][0];
    assert_eq!(
        spelled(&announce.presence),
        Some(r#"feature = "session-auth""#.to_string())
    );
}
