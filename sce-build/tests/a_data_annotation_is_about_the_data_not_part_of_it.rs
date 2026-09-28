// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `<data>` carries the §2.10 annotations, and one written in element
//! form is ABOUT the variable rather than part of its value.
//!
//! `docs/SCE_ACCEPTED_SUBSET.md` §2.10. A `<data>` is the place an author
//! most often records a guess — a threshold a specification names without
//! a number is a variable's initial value — and until 2026-09-28 the
//! parser dropped every annotation written there. The reach test
//! (`an_annotation_the_model_holds_is_one_every_reader_reaches.rs`) holds
//! the attribute form to every reader; this file holds what is particular
//! to `<data>`: §scxml-5.4 reads its element children as an in-line value,
//! so an annotation in element form has to be read as an annotation and
//! kept out of that value.

use sce_build::parser::SCXMLParser;
use sce_build::provenance::MarkerKind;

const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="guess" initial="s" datamodel="ecmascript">
  <datamodel>
    <data id="limit" expr="15" sce:assumed="low_speed" sce:assumed-reason="spec gives no number"
          sce:assumed-candidates="5 10 15"/>
    <data id="table">
      <sce:unresolved id="tbd_rows" reason="row set not agreed" candidates="short long"/>
      <sce:provenance doc-id="SPEC" rev="2" section="3.4"/>
      <row key="a"/>
    </data>
  </datamodel>
  <state id="s"/>
</scxml>"#;

fn model() -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOC, "guess")
        .unwrap_or_else(|e| panic!("the document must parse: {:?}", e.error))
}

#[test]
fn an_assumption_on_a_data_is_held_with_its_reason_and_candidates() {
    let model = model();
    let limit = &model.variables[0];
    assert_eq!(limit.id, "limit");
    assert_eq!(limit.unresolved.len(), 1, "{:?}", limit.unresolved);
    let marker = &limit.unresolved[0];
    assert_eq!(marker.kind, MarkerKind::Assumed);
    assert_eq!(marker.id, "low_speed");
    assert_eq!(marker.reason.as_deref(), Some("spec gives no number"));
    assert_eq!(marker.candidates, ["5", "10", "15"]);
}

#[test]
fn an_annotation_in_element_form_is_read_and_is_not_part_of_the_value() {
    let model = model();
    let table = &model.variables[1];
    assert_eq!(table.id, "table");

    let ids: Vec<&str> = table.unresolved.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["tbd_rows"], "the element-form marker is read");
    assert_eq!(table.provenance.len(), 1, "the element-form anchor is read");
    assert_eq!(table.provenance[0].doc_id, "SPEC");

    assert!(
        table.content.contains("row")
            && !table.content.contains("unresolved")
            && !table.content.contains("provenance"),
        "the in-line value is the value's own markup and nothing the author \
         wrote about it; got {:?}",
        table.content
    );
}
