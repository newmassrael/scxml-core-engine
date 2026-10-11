// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The `plain` lexicon writes a statechart page without the words of the
//! standard the document is in, and the page still reads back as the
//! document.
//!
//! # What was wrong
//!
//! The pseudocode page of a statechart named the standard's own switches
//! as the standard does: `datamodel: ecmascript`, `binding: early`,
//! `initial-children`, `[external]`, `raise`, `parallel`. A specification
//! owner who does not know that standard met them on a page meant to be
//! read without it. Measured 2026-10-11 on 742 committed statechart pages:
//! 349 carry the `datamodel` head and 269 carry `[external]`.
//!
//! Most of them were not words the lexicon could name: the head clause
//! list's keys, the transition type and `raise` were text the mapping
//! built, so a lexicon could neither rename nor drop them.
//!
//! # What holds
//!
//! * The default page is byte for byte what it was.
//! * In the `plain` lexicon none of those words reaches the page, and each
//!   is said as what it does.
//! * It is a LEXICON, so the page is the canonical one under other words:
//!   `normalise_page` returns the canonical page byte for byte. A page
//!   that said less than the document could not be read back, and this
//!   one says all of it.

use sce_build::forge::model::ForgeDocument;
use sce_build::forge::page::{normalise_page, write_page, Endmark, Indent, EN, PLAIN};
use sce_build::forge::pseudo::{render, render_nodes, Deployment};
use sce_build::parser::SCXMLParser;

const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="lock" initial="main" datamodel="ecmascript" binding="early">
  <parallel id="main">
    <state id="left" initial="left_off">
      <state id="left_off">
        <transition event="lock" target="left_on" type="external">
          <raise event="eval"/>
        </transition>
      </state>
      <state id="left_on">
        <transition event="unlock" target="left_off" cond="ready" type="internal"/>
      </state>
    </state>
    <state id="right" initial="right_off">
      <state id="right_off"/>
    </state>
  </parallel>
</scxml>"#;

fn document(source: &str) -> ForgeDocument {
    let model = SCXMLParser::new()
        .parse_string(source, "lock")
        .unwrap_or_else(|e| panic!("the document parses: {:?}", e.error));
    ForgeDocument::Statechart(Box::new(model))
}

fn plain_page(doc: &ForgeDocument) -> String {
    let nodes = render_nodes(doc, &Deployment::default()).expect("the document renders");
    write_page(&nodes, &Indent, &PLAIN).expect("the indent shape writes it")
}

#[test]
fn the_default_page_is_what_it_was() {
    let page = render(&document(DOC)).expect("the document renders");
    assert!(page.starts_with(
        "machine lock (name: lock, datamodel: ecmascript, initial: main, binding: early)\n"
    ));
    assert!(page.contains("parallel main:"));
    assert!(page.contains("-> left_on [external]"));
    assert!(page.contains("-> left_off [internal] when ready"));
    assert!(page.contains("raise eval"));
}

#[test]
fn the_plain_page_has_none_of_the_standards_words() {
    let page = plain_page(&document(DOC));
    for word in [
        "datamodel",
        "binding",
        "initial-children",
        "[external]",
        "[internal]",
        "raise",
        "parallel",
    ] {
        assert!(
            !page.contains(word),
            "the plain page still says '{word}':\n{page}"
        );
    }
    for word in [
        "expression language: ecmascript",
        "first state: main",
        "initial values assigned: early",
        "concurrent main:",
        "starts together in left_off",
        "(leaves its source state first)",
        "(stays in its source state when the target is inside it)",
        "tell itself eval",
    ] {
        assert!(
            page.contains(word),
            "the plain page lacks '{word}':\n{page}"
        );
    }
}

#[test]
fn the_plain_page_reads_back_as_the_canonical_one() {
    for source in [
        DOC.to_string(),
        DOC.replace(r#"binding="early""#, r#"binding="late""#),
        DOC.replace(r#" type="internal""#, ""),
    ] {
        let doc = document(&source);
        let canonical = render(&doc).expect("the document renders");
        let page = plain_page(&doc);
        assert!(
            page.starts_with("#!sce-pseudo shape=indent lexicon=plain\n"),
            "{page}"
        );
        let back = normalise_page(&page).expect("the plain page normalises");
        assert_eq!(
            back, canonical,
            "the plain page did not come back as the canonical one"
        );
    }
}

#[test]
fn the_plain_words_are_the_default_words_wherever_the_standard_has_no_better() {
    let nodes = render_nodes(&document(DOC), &Deployment::default()).expect("renders");
    let default = write_page(&nodes, &Indent, &EN).expect("writes");
    let plain = plain_page(&document(DOC));
    // A state, an event and a transition are written in the same words.
    assert!(default.contains("state left_off:"));
    assert!(plain.contains("state left_off:"));
    assert!(default.contains("on lock -> left_on"));
    assert!(plain.contains("on lock -> left_on"));
}

#[test]
fn the_endmark_shape_writes_the_plain_words_too() {
    let nodes = render_nodes(&document(DOC), &Deployment::default()).expect("renders");
    let page = write_page(&nodes, &Endmark, &PLAIN).expect("the endmark shape writes it");
    assert!(page.contains("concurrent begin main:"), "{page}");
    let back = normalise_page(&page).expect("it normalises");
    assert_eq!(back, render(&document(DOC)).expect("renders"));
}
