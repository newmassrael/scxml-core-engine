// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An exclusion this document states is measured against the producer.
//!
//! # What went wrong without this file
//!
//! `docs/SCE_ACCEPTED_SUBSET.md` §3 tells an author which constructs the
//! static path refuses. Two of its rows described refusals the producer does
//! not make:
//!
//! - §3.1 said an `<invoke>` naming its child through an expression was
//!   rejected as `validation/dynamic-features`. Every backend generates it.
//! - §3.2 said a document relying on the default initial state was rejected.
//!   The parser resolves the default before any gate sees the model.
//!
//! A wrong refusal is not a harmless stale sentence. It is read by whoever
//! is choosing between the engines, and it sends a document to the
//! Interpreter for a construct the static path handles — the opposite of
//! the decision the page exists to support.
//!
//! §3.3 is the third row of the same kind and it was corrected before these
//! two, with a test pinning the behaviour so the paragraph "cannot go stale
//! again without a test going red". §3.1 and §3.2 had no such pin, which is
//! the whole reason they outlived their cause. This file is theirs.
//!
//! # Why both halves are asserted
//!
//! The producer's answer is the truth and the document is a description of
//! it, so each row is measured twice: the construct is put through the real
//! parse-and-gate path, and the section is read for the claim it now makes.
//! Asserting only the producer would let the prose drift again; asserting
//! only the prose would pass for a page that describes a producer nobody
//! ran.

use sce_build::analyzer::can_generate_static;
use sce_build::parser::SCXMLParser;

const SUBSET_DOC: &str = include_str!("../../docs/SCE_ACCEPTED_SUBSET.md");

/// The text of one `###` section, from its heading to the next one.
///
/// Slicing by heading rather than by line number because a section moves
/// whenever anything above it grows, and a test pinned to a line number
/// measures the file's length.
fn section(heading_starts_with: &str) -> &'static str {
    let start = SUBSET_DOC
        .find(heading_starts_with)
        .unwrap_or_else(|| panic!("no section heading starting {heading_starts_with:?}"));
    let rest = &SUBSET_DOC[start + heading_starts_with.len()..];
    match rest.find("\n### ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// §scxml-3.2, §scxml-3.3: neither `initial=` nor an `<initial>` child.
const NO_INITIAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="probe">
  <state id="a">
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>"#;

/// §scxml-6.4: a child named by an expression rather than by `src`.
const HYBRID_INVOKE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a" name="probe">
  <datamodel><data id="pathVar" expr="'child.scxml'"/></datamodel>
  <state id="a">
    <invoke type="scxml" srcexpr="pathVar"/>
    <transition event="done.invoke" target="b"/>
  </state>
  <final id="b"/>
</scxml>"#;

#[test]
fn a_document_without_an_initial_state_is_generated_not_refused() {
    let model = SCXMLParser::new()
        .parse_string(NO_INITIAL, "probe")
        .expect("a document relying on the default initial state must parse");

    // The resolution is the point, not merely the acceptance: §scxml-3.3
    // makes the first child state in document order the default, and a
    // machine that started anywhere else would be accepted and wrong.
    assert_eq!(
        model.initial, "a",
        "the first child state in document order is the default entry state"
    );

    can_generate_static(&model, "probe.scxml")
        .expect("the static path generates this document; §3.2 said it refused one");
}

#[test]
fn a_hybrid_invoke_is_generated_and_selects_between_exactly_one_child() {
    let model = SCXMLParser::new()
        .parse_string(HYBRID_INVOKE, "probe")
        .expect("an invoke naming its child by expression must parse");

    let hybrids: Vec<_> = model.iter_hybrid_invokes().collect();
    assert_eq!(
        hybrids.len(),
        1,
        "one `<invoke srcexpr>` is one hybrid invoke"
    );
    assert_eq!(hybrids[0].srcexpr, "pathVar");

    // The residue §2.13 states, asserted where it is decided rather than
    // where it is observed. The child is named at parse time, so whatever
    // the expression evaluates to at runtime, this is the machine that
    // runs — one name, and no set to choose from.
    assert_eq!(
        hybrids[0].common.child_name, "probe_hybrid0",
        "the child a hybrid invoke runs is fixed at build time"
    );

    can_generate_static(&model, "probe.scxml")
        .expect("the static path generates this document; §3.1 said it refused one");
}

#[test]
fn the_retracted_rows_send_the_reader_to_what_replaced_them() {
    for heading in ["### §3.1 ", "### §3.2 "] {
        let body = section(heading);
        assert!(
            body.contains("Nothing here is excluded any more"),
            "{heading} still states an exclusion the producer does not make"
        );
    }

    assert!(
        section("### §3.1 ").contains("§2.13"),
        "§3.1 must name the section that describes what is generated instead"
    );

    // A backend that handles hybrid invokes differently from the rest is
    // the defect §2.13 exists to make visible, so the section has to answer
    // for every backend rather than for the ones that agree.
    let hybrid = section("### §2.13 ");
    for backend in ["C++", "Rust", "Go", "C11", "Python", "Kotlin"] {
        assert!(
            hybrid.contains(backend),
            "§2.13 does not say what {backend} does with a hybrid invoke"
        );
    }

    // ⚠ Naming a backend is not answering for it. §2.13 lists all six in
    // one sentence — the contract they share — so the loop above holds for
    // a section that has lost the record of which backends broke that
    // contract, which is exactly the "consensus" reading this test exists
    // to refuse. That record is each diverging backend's own entry, bold
    // where it begins; it is what a reader of §2.13 needs, so it is what is
    // held here. (Before this, removing Python's entry left every assertion
    // true, and its mutation case survived.)
    for diverged in ["Python", "Kotlin"] {
        assert!(
            hybrid.contains(&format!("**{diverged}**")),
            "§2.13 no longer records what {diverged} did differently with a hybrid invoke"
        );
    }
}
