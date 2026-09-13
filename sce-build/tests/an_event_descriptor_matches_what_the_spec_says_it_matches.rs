// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! W3C SCXML 3.12.1 — an event descriptor matches exactly the event names
//! the specification says it matches, in every reader that decides it at
//! build time.
//!
//! Found while designing the acceptance report's per-requirement fragment
//! (Requirement-closure RFC §12, Atomic C), which has to link a transition
//! to whatever raises its event. Asking which code decides "does this
//! descriptor match that event" turned up no single answer. Measured
//! 2026-09-13 against the mirrored §3.12.1 text: `mesh/topology.rs`,
//! `scxml_exhaustiveness.rs` and `analyzer.rs` each carry their own
//! matcher, and they disagree with the specification and with each other.
//!
//! ```text
//!   the specification: "a transition with 'event' of "error", one with
//!   "error.", and one with "error.*" are functionally equivalent since they
//!   are token prefixes of exactly the same set of event names"
//!
//!                                         error.* vs "error"   error. vs "error"
//!   mesh/topology::event_matches_any       match                NO
//!   scxml_exhaustiveness (live validator)  NO                   NO
//!   analyzer::build_prefix_matching        NO                   NO   <- read by C11 and Kotlin codegen
//! ```
//!
//! No W3C fixture delivers a bare `foo` to a `foo.*` handler — test 399 sends
//! `foo.zoo` to `foo.*` and catches its bare `foo` with `*` — which is how
//! the disagreement survived a green conformance suite.
//!
//! # ⭐ Where the expectations come from
//!
//! Every row of [`SPEC_EXAMPLES`] is a sentence of §3.12.1, not a
//! judgement of this file's author. One sentence of the section is left
//! out on purpose: its example list says `"error foo"` "would not match
//! ... "error.send"", which contradicts the definition two sentences
//! earlier and the example list's own first half. Taking it literally
//! would make the table disagree with the rule it illustrates.
//!
//! Each reader is asked through a path this crate publishes, so the test
//! holds the behaviour and not a private helper's name.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::analyzer;
use sce_build::compile_scxml_lang_typed;
use sce_build::find_template_dir_for;
use sce_build::generator::Language;
use sce_build::parser::SCXMLParser;

/// `(transition event attribute, event name, whether it matches)`, each
/// taken from a sentence of W3C SCXML §3.12.1.
const SPEC_EXAMPLES: &[(&str, &str, bool)] = &[
    // "a transition with an 'event' attribute of "error foo" will match
    // event names "error", "error.send", "error.send.failed", etc. (or
    // "foo", "foo.bar" etc.)"
    ("error foo", "error", true),
    ("error foo", "error.send", true),
    ("error foo", "error.send.failed", true),
    ("error foo", "foo", true),
    ("error foo", "foo.bar", true),
    // "but would not match events named "errors.my.custom",
    // "errorhandler.mistake" ... or "foobar""
    ("error foo", "errors.my.custom", false),
    ("error foo", "errorhandler.mistake", false),
    ("error foo", "foobar", false),
    // "a transition with 'event' of "error", one with "error.", and one
    // with "error.*" are functionally equivalent"
    ("error", "error", true),
    ("error.", "error", true),
    ("error.*", "error", true),
    ("error", "error.send", true),
    ("error.", "error.send", true),
    ("error.*", "error.send", true),
    ("error.", "errors", false),
    ("error.*", "errors", false),
    // "An event designator consisting solely of "*" can be used as a
    // wildcard matching any sequence of tokens, and thus any event."
    ("*", "anything.at.all", true),
    // "In all cases, the token matching is case sensitive."
    ("error", "Error", false),
    ("error.*", "Error.send", false),
];

/// The analyzer decides, per transition, which of the document's events
/// the transition catches, and C11 and Kotlin generate their dispatch from
/// that list. So the list is asked directly: for every example, a document
/// that raises the event and offers a transition on the descriptor.
#[test]
fn the_analyzer_lists_exactly_the_events_a_descriptor_matches() {
    const FLOOR: usize = 19;

    let mut examined = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for (descriptor, event, expected) in SPEC_EXAMPLES {
        let doc = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                      name="descriptor" initial="s" datamodel="null">
                 <state id="s">
                   <onentry><raise event="{event}"/></onentry>
                   <transition event="{descriptor}" target="t"/>
                 </state>
                 <state id="t"/>
               </scxml>"#
        );
        let mut model = SCXMLParser::new()
            .parse_string(&doc, "descriptor")
            .unwrap_or_else(|e| panic!("{descriptor:?} / {event:?} must parse: {:?}", e.error));
        analyzer::analyze(&mut model, "descriptor");
        let transition = &model.states["s"].transitions[0];
        let listed = transition.prefix_matching_events.iter().any(|e| e == event);
        examined += 1;
        if listed != *expected {
            wrong.push(format!(
                "  event=\"{descriptor}\" vs \"{event}\": the spec says {}, the analyzer \
                 listed {:?}",
                if *expected { "match" } else { "no match" },
                transition.prefix_matching_events,
            ));
        }
    }

    println!(
        "asked the analyzer about {examined} §3.12.1 example(s), {} answered against the spec",
        wrong.len()
    );
    assert!(
        wrong.is_empty(),
        "the analyzer's per-transition event list disagrees with W3C SCXML \
         §3.12.1, and C11 and Kotlin dispatch from that list:\n{}",
        wrong.join("\n"),
    );
    assert!(
        examined >= FLOOR,
        "only {examined} example(s) examined; floor {FLOOR}"
    );
}

/// The same equivalence, through the validator that runs on every compile.
///
/// Three siblings share `cmd.stop` as common ground, which is what makes
/// the exhaustiveness heuristic compare them, and each handles `go`
/// through a different spelling of one descriptor. By §3.12.1 all three
/// handle it, so there is no gap and the document must compile. A validator
/// that reads `go.*` or `go.` as unable to catch `go` rejects a correct
/// document.
#[test]
fn equivalent_spellings_of_one_descriptor_leave_no_gap() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("equivalent_descriptors.scxml");
    fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="equivalent_descriptors" initial="dispatch">
  <state id="dispatch" initial="plain">
    <state id="plain">
      <transition event="go" target="suffixed"/>
      <transition event="cmd.stop" target="dotted"/>
    </state>
    <state id="suffixed">
      <transition event="go.*" target="dotted"/>
      <transition event="cmd.stop" target="plain"/>
    </state>
    <state id="dotted">
      <transition event="go." target="plain"/>
      <transition event="cmd.stop" target="suffixed"/>
    </state>
  </state>
</scxml>
"#,
    )
    .expect("write fixture");

    let template_dir = find_template_dir_for(Language::Rust);
    let compiled = compile_scxml_lang_typed(path_str(&path), &template_dir, Language::Rust);
    assert!(
        compiled.is_ok(),
        "`go`, `go.*` and `go.` are functionally equivalent under W3C SCXML \
         §3.12.1, so every sibling handles `go` and there is no gap — but the \
         compile rejected the document: {:?}",
        compiled.err().map(|e| e.error),
    );
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("the tempdir path is UTF-8")
}
