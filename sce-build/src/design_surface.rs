// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a statechart presents to the outside, and whether it is what its owner
//! accepted.
//!
//! A scenario set proposes an interface: the names its examples are written
//! against, which becomes the machine's interface when the owner accepts it with
//! the examples. A design is drafted separately, from the same prose, and
//! nothing held the two to each other. Measured 2026-09-29, five drafts of one
//! specification invented five interfaces; and an example that names a state the
//! design calls something else FAILS, which reads as the design misbehaving when
//! it is two names for one thing.
//!
//! [`Surface::of`] is what the design presents, taken from the analyzer's own
//! facts and not read off the document again: the events a caller can deliver
//! ([`SCXMLModel::externally_drivable_events`], the set §2.16 closes an interface
//! on), the events it sends out of the session (the same `<send>` the closed
//! interface calls "leaving the session", [`sends_to_itself`] false), its states
//! and its data items. It rides in the generate manifest, and a driver copies it
//! into the observation trace, so the judge compares two things the product
//! produced and no driver re-derives either.
//!
//! [`Surface::compare`] is the set equality: each name the interface accepts must
//! be answered by the design, and each name the design presents must be one the
//! interface accepts. Inputs are compared as event descriptors do (W3C SCXML
//! 3.12.1: a transition on `coin` takes `coin.inserted`), outputs as the closed
//! interface does (exactly, since a send names its event literally).
//!
//! A send that names its event by expression cannot be listed, so the outputs a
//! design never sends cannot be asserted: [`Surface::computed_outputs`] says so
//! and the report does not claim a match.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::event_descriptor::EventDescriptor;
use crate::host_processor_analyzer::walk_model_actions;
use crate::model::SCXMLModel;
use crate::scenario_set::Interface;
use crate::scxml_self_send::sends_to_itself;

/// What one statechart presents: names only, sorted, each once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// The events a caller can deliver: [`SCXMLModel::externally_drivable_events`].
    pub inputs: Vec<String>,
    /// A transition takes `*`, so every event is taken and none can be left
    /// unserved. Omitted when no transition does.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub takes_any_input: bool,
    /// The events a `<send>` puts outside the session, by their literal names.
    pub outputs: Vec<String>,
    /// A `<send>` outside the session names its event by expression, so
    /// `outputs` may be missing a name it sends. Omitted when none does.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub computed_outputs: bool,
    /// Every state id, the compound and final ones included.
    pub states: Vec<String>,
    /// Every `<data>` id the document declares.
    pub data: Vec<String>,
}

impl Surface {
    /// The surface of an ANALYZED statechart ([`crate::analyzer::analyze`]): the
    /// events a caller can deliver are the analyzer's own set, so a model that was
    /// only parsed presents none.
    pub fn of(model: &SCXMLModel) -> Self {
        let mut outputs: BTreeSet<String> = BTreeSet::new();
        let mut computed_outputs = false;
        walk_model_actions(model, &mut |_state, action| {
            if action.action_type != "send" || sends_to_itself(action) {
                return;
            }
            if !action.eventexpr.is_empty() {
                computed_outputs = true;
            } else if !action.event.is_empty() {
                outputs.insert(action.event.clone());
            }
        });
        // A transition on `*` takes whatever arrives, and `external_ingress_events`
        // leaves it out on purpose (a pattern names no event a caller could
        // target), so it is read from the descriptors themselves, the way the
        // closed interface reads them: no prefix is a `*`.
        let takes_any_input = model.states.values().any(|state| {
            state.transitions.iter().any(|transition| {
                transition
                    .event
                    .split_whitespace()
                    .any(|token| EventDescriptor::parse(token).prefix().is_none())
            })
        });
        Surface {
            inputs: model.externally_drivable_events.iter().cloned().collect(),
            takes_any_input,
            outputs: outputs.into_iter().collect(),
            computed_outputs,
            states: model.states.keys().cloned().collect(),
            data: model
                .variables
                .iter()
                .map(|variable| variable.id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    /// Whether the design takes an event of this name: some descriptor it
    /// reacts to matches it (§scxml-3.12.1), or it takes `*`.
    pub fn takes(&self, event: &str) -> bool {
        self.takes_any_input
            || self
                .inputs
                .iter()
                .any(|taken| EventDescriptor::parse(taken).matches(event))
    }

    /// The design held to the interface its owner accepted, in both directions.
    pub fn compare(&self, accepted: &Interface) -> InterfaceReport {
        let unserved_inputs: BTreeSet<String> = accepted
            .inputs
            .iter()
            .filter(|input| !self.takes(&input.name))
            .map(|input| input.name.clone())
            .collect();
        // A transition on `*` is not among `inputs` (a pattern names no event a
        // caller could target), so it is never an event the interface fails to
        // name; what it takes is `takes_any_input`, which serves every name above.
        let unaccepted_inputs: BTreeSet<String> = self
            .inputs
            .iter()
            .filter(|taken| {
                !accepted
                    .inputs
                    .iter()
                    .any(|input| EventDescriptor::parse(taken).matches(&input.name))
            })
            .cloned()
            .collect();
        let named_outputs: BTreeSet<&str> = accepted
            .outputs
            .iter()
            .map(|output| output.name.as_str())
            .collect();
        let unaccepted_outputs: BTreeSet<String> = self
            .outputs
            .iter()
            .filter(|sent| !named_outputs.contains(sent.as_str()))
            .cloned()
            .collect();
        // What a design never sends cannot be said while a send names its event
        // by expression: it may be that one.
        let unsent_outputs: BTreeSet<String> = if self.computed_outputs {
            BTreeSet::new()
        } else {
            named_outputs
                .iter()
                .filter(|name| !self.outputs.iter().any(|sent| sent == **name))
                .map(|name| (*name).to_string())
                .collect()
        };
        let missing_conditions: BTreeSet<String> = accepted
            .conditions
            .iter()
            .filter(|state| !self.states.contains(state))
            .cloned()
            .collect();
        let missing_data: BTreeSet<String> = accepted
            .data
            .iter()
            .filter(|item| !self.data.contains(item))
            .cloned()
            .collect();

        let mut report = InterfaceReport {
            matches: false,
            unserved_inputs: unserved_inputs.into_iter().collect(),
            unaccepted_inputs: unaccepted_inputs.into_iter().collect(),
            unsent_outputs: unsent_outputs.into_iter().collect(),
            unaccepted_outputs: unaccepted_outputs.into_iter().collect(),
            computed_outputs: self.computed_outputs,
            missing_conditions: missing_conditions.into_iter().collect(),
            missing_data: missing_data.into_iter().collect(),
        };
        report.matches = report.differences() == 0 && !report.computed_outputs;
        report
    }
}

/// Where a design and the interface its owner accepted part. Every list is
/// sorted and holds each name once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceReport {
    /// Nothing differs, and nothing is left unknown. False when a send names its
    /// event by expression, however many lists are empty: the outputs it never
    /// sends cannot be listed.
    pub matches: bool,
    /// Inputs the interface names that no event the design takes answers: an
    /// example that sends one reaches nothing.
    pub unserved_inputs: Vec<String>,
    /// Events the design takes that no input of the interface names: a caller
    /// can deliver what its owner never accepted.
    pub unaccepted_inputs: Vec<String>,
    /// Outputs the interface names that the design never sends.
    pub unsent_outputs: Vec<String>,
    /// Events the design sends out that the interface does not name.
    pub unaccepted_outputs: Vec<String>,
    /// A send names its event by expression, so `unsent_outputs` is not claimed.
    pub computed_outputs: bool,
    /// States the interface says the specification names that the design does
    /// not have: an example about one cannot be told from a state absent.
    pub missing_conditions: Vec<String>,
    /// Data items the interface names that the design does not declare.
    pub missing_data: Vec<String>,
}

impl InterfaceReport {
    /// How many names differ, across every list.
    pub fn differences(&self) -> usize {
        self.unserved_inputs.len()
            + self.unaccepted_inputs.len()
            + self.unsent_outputs.len()
            + self.unaccepted_outputs.len()
            + self.missing_conditions.len()
            + self.missing_data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interface(json: &str) -> Interface {
        serde_json::from_str(json).expect("a readable interface")
    }

    fn surface(inputs: &[&str], outputs: &[&str], states: &[&str], data: &[&str]) -> Surface {
        let own = |names: &[&str]| names.iter().map(|n| (*n).to_string()).collect();
        Surface {
            inputs: own(inputs),
            outputs: own(outputs),
            states: own(states),
            data: own(data),
            ..Surface::default()
        }
    }

    const ACCEPTED: &str = r#"{
        "inputs": [{"name": "coin.inserted"}, {"name": "refund"}],
        "outputs": [{"name": "dispense"}],
        "conditions": ["idle", "paid"],
        "data": ["credit"]
    }"#;

    #[test]
    fn a_design_that_presents_what_was_accepted_matches() {
        let design = surface(
            &["coin", "refund"],
            &["dispense"],
            &["idle", "paid", "done"],
            &["credit"],
        );
        let report = design.compare(&interface(ACCEPTED));
        assert_eq!(report.differences(), 0, "{report:?}");
        assert!(report.matches);
    }

    /// §scxml-3.12.1: a transition on `coin` takes `coin.inserted`, so the
    /// shorter descriptor serves the longer name and is accepted by it.
    #[test]
    fn a_descriptor_serves_the_longer_name_the_interface_gives() {
        let design = surface(
            &["coin", "refund"],
            &["dispense"],
            &["idle", "paid"],
            &["credit"],
        );
        let report = design.compare(&interface(ACCEPTED));
        assert!(report.unserved_inputs.is_empty(), "{report:?}");
        assert!(report.unaccepted_inputs.is_empty(), "{report:?}");
    }

    /// The other direction: an interface name SHORTER than the descriptor is not
    /// taken, because a transition on `coin.inserted` does not take `coin`.
    #[test]
    fn a_more_specific_descriptor_does_not_serve_a_shorter_name() {
        let design = surface(&["coin.inserted", "refund"], &[], &[], &[]);
        let accepted =
            interface(r#"{"inputs": [{"name": "coin"}, {"name": "refund"}], "outputs": []}"#);
        let report = design.compare(&accepted);
        assert_eq!(report.unserved_inputs, ["coin"]);
        assert_eq!(report.unaccepted_inputs, ["coin.inserted"]);
    }

    #[test]
    fn a_renamed_state_and_output_are_told_apart_from_a_missing_one() {
        let design = surface(
            &["coin", "refund"],
            &["release"],
            &["idle", "pending"],
            &["credit"],
        );
        let report = design.compare(&interface(ACCEPTED));
        assert_eq!(report.missing_conditions, ["paid"]);
        assert_eq!(report.unsent_outputs, ["dispense"]);
        assert_eq!(report.unaccepted_outputs, ["release"]);
        assert!(!report.matches);
    }

    #[test]
    fn a_design_that_takes_everything_leaves_no_input_unserved() {
        let mut design = surface(&[], &["dispense"], &["idle", "paid"], &["credit"]);
        design.takes_any_input = true;
        let report = design.compare(&interface(ACCEPTED));
        assert!(report.unserved_inputs.is_empty(), "{report:?}");
    }

    /// A send by expression may be any event, so the outputs a design never
    /// sends cannot be listed and no match is claimed.
    #[test]
    fn a_computed_send_withholds_the_unsent_list_and_the_match() {
        let mut design = surface(&["coin", "refund"], &[], &["idle", "paid"], &["credit"]);
        design.computed_outputs = true;
        let report = design.compare(&interface(ACCEPTED));
        assert!(report.unsent_outputs.is_empty(), "{report:?}");
        assert!(report.computed_outputs);
        assert!(!report.matches);
    }

    #[test]
    fn a_data_item_the_design_does_not_declare_is_missing() {
        let design = surface(&["coin", "refund"], &["dispense"], &["idle", "paid"], &[]);
        assert_eq!(
            design.compare(&interface(ACCEPTED)).missing_data,
            ["credit"]
        );
    }

    // ---- the surface read off a real document ----

    const VENDING: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" initial="idle">
        <datamodel><data id="credit" expr="0"/></datamodel>
        <state id="idle"><transition event="coin" target="paid"/></state>
        <state id="paid">
          <onentry>
            <send event="dispense" type="x-sce-host"/>
            <send event="retry" delay="1s"/>
          </onentry>
          <transition event="refund" target="idle">
            <send event="refunded" type="x-sce-host"/>
          </transition>
          <transition event="retry" target="idle"/>
        </state>
      </scxml>"#;

    /// A parsed AND analyzed model: the drivable events are the analyzer's, so a
    /// model only parsed has none.
    fn model(xml: &str) -> SCXMLModel {
        let mut model = crate::parser::SCXMLParser::new()
            .parse_string(xml, "surface")
            .expect("the document parses");
        crate::analyzer::analyze(&mut model, "surface.scxml");
        model
    }

    #[test]
    fn the_surface_is_what_the_analyzer_knows_of_the_document() {
        let found = Surface::of(&model(VENDING));
        // A caller delivers what a transition takes. `retry` is sent to the
        // session with no target, which is the EXTERNAL queue (W3C SCXML
        // 6.2.4), so a caller can deliver it too: the same reading as the
        // closed interface's.
        assert_eq!(found.inputs, ["coin", "refund", "retry"], "{found:?}");
        // Outside the session: the two host sends. The self-send is not one.
        assert_eq!(found.outputs, ["dispense", "refunded"], "{found:?}");
        assert!(
            !found.computed_outputs && !found.takes_any_input,
            "{found:?}"
        );
        for state in ["idle", "paid"] {
            assert!(
                found.states.iter().any(|s| s == state),
                "{state}: {found:?}"
            );
        }
        assert_eq!(found.data, ["credit"]);
    }

    #[test]
    fn a_send_by_expression_outside_the_session_is_marked_computed() {
        let found = Surface::of(&model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                    datamodel="ecmascript" initial="a">
                <state id="a"><onentry>
                  <send eventexpr="'out'" type="x-sce-host"/>
                </onentry></state>
              </scxml>"#,
        ));
        assert!(found.computed_outputs, "{found:?}");
        assert!(found.outputs.is_empty(), "{found:?}");
    }

    #[test]
    fn a_transition_on_the_wildcard_takes_any_input() {
        let found = Surface::of(&model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                    datamodel="ecmascript" initial="a">
                <state id="a"><transition event="*" target="a"/></state>
              </scxml>"#,
        ));
        assert!(found.takes_any_input, "{found:?}");
    }

    // ---- the wire ----

    const MANIFEST_SCHEMA: &str = include_str!("../../schemas/sce-manifest.v1.schema.json");
    const TRACE_SCHEMA: &str = include_str!("../../schemas/sce-observation-trace.v1.schema.json");

    fn definition(schema: &str) -> serde_json::Value {
        let tree: serde_json::Value = serde_json::from_str(schema).expect("a schema");
        tree["definitions"]["surface"].clone()
    }

    /// The manifest carries the surface and the trace repeats it: one shape in
    /// two files, so a field added to one and not the other is a driver that
    /// cannot copy what the product wrote.
    #[test]
    fn the_manifest_and_the_trace_describe_one_surface() {
        assert!(definition(MANIFEST_SCHEMA).is_object());
        assert_eq!(definition(MANIFEST_SCHEMA), definition(TRACE_SCHEMA));
    }

    /// Every field the producer can write is one the schema names, and the
    /// reverse: `additionalProperties: false` would refuse the manifest the day
    /// a field is added here and not there.
    #[test]
    fn what_the_producer_writes_validates_against_the_schema() {
        let mut full = surface(&["coin"], &["dispense"], &["idle"], &["credit"]);
        full.takes_any_input = true;
        full.computed_outputs = true;
        let wrapper = serde_json::json!({
            "definitions": serde_json::from_str::<serde_json::Value>(MANIFEST_SCHEMA)
                .expect("a schema")["definitions"],
            "$ref": "#/definitions/surface",
        });
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&wrapper)
            .expect("the surface definition compiles as draft-07");
        for found in [full, Surface::of(&model(VENDING))] {
            let instance = serde_json::to_value(&found).expect("serialises");
            let problems: Vec<String> = match validator.validate(&instance) {
                Ok(()) => Vec::new(),
                Err(errors) => errors.map(|e| e.to_string()).collect(),
            };
            assert!(problems.is_empty(), "{problems:?}\n{instance}");
        }
    }

    #[test]
    fn the_surface_round_trips_through_the_wire() {
        let design = surface(&["coin"], &["dispense"], &["idle"], &["credit"]);
        let line = serde_json::to_string(&design).expect("serialises");
        assert!(!line.contains("takes_any_input"), "{line}");
        assert!(!line.contains("computed_outputs"), "{line}");
        let back: Surface = serde_json::from_str(&line).expect("reads back");
        assert_eq!(back, design);
    }
}
