// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An analyzer that walks executable content reaches the NESTED content
//! too — wherever in the action tree it sits.
//!
//! # Why a green predicate was not enough
//!
//! The closure ledger's row S1 is measured by counting files that name
//! `then_actions` / `else_actions` / `elseif_branches` outside the model
//! and the parser. That count reaches zero the moment nobody spells the
//! fields — which a reader can achieve while still walking exactly the
//! containers it walked before, or while walking none. The count says
//! where the definition is spelled; it cannot say what the analyzer sees.
//!
//! This file is the half that can. Each analyzer is asked the SAME
//! question twice: once about a document whose trigger sits at the top
//! level, and once about a document whose trigger sits inside a
//! container. The top-level answer is the control — it is what the
//! analyzer says when nothing is hiding — and the nested answer must
//! equal it. An analyzer that has gone quietly shallow answers the two
//! differently, and that is the defect row S1 exists for: silence, not a
//! wrong answer.
//!
//! # ⭐ Why the containers are not listed here
//!
//! They are read from [`sce_build::model::Action::nested_blocks`], the
//! one definition of what lies inside an action. A test that wrote its
//! own list of containers would inherit exactly the blindness it is
//! checking for — the failure it is looking for IS a hand-written list
//! that is short by one — and would keep passing for the container it
//! forgot.
//!
//! So a block kind added to the model appears here on its next run. It
//! arrives as a PANIC rather than as silent non-coverage: `wrapper_for`
//! refuses a path it has no SCXML shape for, because a container nothing
//! places a trigger inside is coverage on paper and none in fact.

use sce_build::model::{Action, ElseIfBranch, SCXMLModel};
use sce_build::parser::SCXMLParser;
use sce_build::{analyzer, host_processor_analyzer, script_engine_analyzer};

/// Every container the model defines, by the path `nested_blocks` names
/// it with.
///
/// Built from an action carrying one of every shape, so the paths come
/// from the model's own answer rather than from this file's idea of it.
fn container_paths() -> Vec<String> {
    let probe = Action {
        action_type: "if".to_string(),
        actions: vec![Action::default()],
        then_actions: vec![Action::default()],
        else_actions: vec![Action::default()],
        elseif_branches: vec![ElseIfBranch {
            cond: "true".to_string(),
            actions: vec![Action::default()],
            ..Default::default()
        }],
        ..Default::default()
    };
    probe
        .nested_blocks()
        .into_iter()
        .map(|block| block.path)
        .collect()
}

/// The SCXML that puts `trigger` inside the container named by `path`.
///
/// ⚠ Panics on a path it does not know. That is deliberate: a container
/// the model defines and this file cannot build a document for is a
/// container nothing here tests, and a skip would read as a pass.
fn wrapper_for(path: &str, trigger: &str) -> String {
    match path {
        // `<foreach>`'s body.
        "actions" => format!(r#"<foreach array="items" item="it">{trigger}</foreach>"#),
        "then_actions" => format!(r#"<if cond="true">{trigger}</if>"#),
        "else_actions" => format!(r#"<if cond="false"><log expr="'x'"/><else/>{trigger}</if>"#),
        "elseif_branches[0].actions" => {
            format!(r#"<if cond="false"><log expr="'x'"/><elseif cond="true"/>{trigger}</if>"#)
        }
        other => panic!(
            "`Action::nested_blocks` defines the container {other:?} and this \
             file has no SCXML shape that puts an action inside it. Add one — \
             a container no document here reaches is coverage on paper and \
             none in fact, which is the shape row S1 exists to refuse."
        ),
    }
}

/// A document whose `<onentry>` holds `body`.
fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="nested_reach" initial="s" datamodel="ecmascript">
  <datamodel><data id="items" expr="[1]"/></datamodel>
  <state id="s">
    <onentry>{body}</onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

fn analyzed(body: &str) -> SCXMLModel {
    let doc = document(body);
    let mut model = SCXMLParser::new()
        .parse_string(&doc, "nested_reach")
        .unwrap_or_else(|e| panic!("fixture must parse: {:?}\n{doc}", e.error));
    analyzer::analyze(&mut model, "nested_reach");
    model
}

/// One analyzer, the action that should make it speak, and the question
/// its answer is read from.
struct Probe {
    analyzer: &'static str,
    trigger: &'static str,
    ask: fn(&SCXMLModel) -> bool,
}

const PROBES: &[Probe] = &[
    Probe {
        analyzer: "script_engine_analyzer::requires_script_engine",
        trigger: r#"<script>var x = 1;</script>"#,
        ask: script_engine_analyzer::requires_script_engine,
    },
    Probe {
        analyzer: "host_processor_analyzer::needs_host_processor",
        trigger: r#"<send event="e" type="BasicHTTPEventProcessor" target="http://h/x"/>"#,
        ask: host_processor_analyzer::needs_host_processor,
    },
    Probe {
        analyzer: "analyzer::analyze -> model.uses_cancel",
        trigger: r#"<cancel sendid="t1"/>"#,
        ask: |model| model.uses_cancel,
    },
];

#[test]
fn every_analyzer_answers_the_same_nested_as_it_does_at_the_top_level() {
    let paths = container_paths();
    assert!(
        paths.len() >= 4,
        "`Action::nested_blocks` reported only {} container(s); an empty or \
         shrunken list would make this sweep pass by asking nothing",
        paths.len()
    );

    let mut shallow: Vec<String> = Vec::new();
    let mut asked = 0usize;

    for probe in PROBES {
        let control = (probe.ask)(&analyzed(probe.trigger));
        assert!(
            control,
            "{}: the trigger {:?} does not make this analyzer speak even at \
             the TOP level, so the nested cases below could only ever compare \
             two silences",
            probe.analyzer, probe.trigger,
        );

        for path in &paths {
            let nested = (probe.ask)(&analyzed(&wrapper_for(path, probe.trigger)));
            asked += 1;
            if nested != control {
                shallow.push(format!(
                    "  {} answered {nested} for a trigger inside `{path}` and \
                     {control} for the same trigger at the top level",
                    probe.analyzer,
                ));
            }
        }
    }

    println!(
        "asked {} analyzer(s) about {} container(s) — {asked} nested case(s), \
         {} disagreeing with their own top-level answer",
        PROBES.len(),
        paths.len(),
        shallow.len(),
    );
    assert!(
        shallow.is_empty(),
        "these analyzers do not reach nested executable content, so a \
         document hides from them exactly what it declares:\n{}",
        shallow.join("\n"),
    );
}
