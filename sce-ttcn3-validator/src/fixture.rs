// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One export written by hand, for the tests of the second and third checks:
//! a suite whose test case runs one component, `c.scxml`, with an event
//! schema and a template it imports, the two modules the export writes of
//! them, and the witness of the places of those modules.
//!
//! The component's `outer` state holds `a` and `b` and leaves for the
//! top-level `<final>` `done` when the timer `u` times out; `a` leaves for
//! `b` when the timer `t` times out. So the `alt` of `a` holds its own
//! branch and then its parent's (P3), the block of the first exits `a` and
//! enters `b`, and the block of the second exits `a` and `outer`, each with
//! its onexit, and returns.

use std::collections::BTreeMap;

use crate::documents::statechart_tests::{CASE, CONFIG, SUITE};
use crate::documents::tests::Memory;
use crate::documents::Documents;
use crate::ttcn3::{self, ast::Module};
use crate::witness::{self, Witness};

pub(crate) const COMPONENT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="c" datamodel="sce-static" initial="outer">
  <sce:import kind="test-template" src="ask.scxml" as="Ask"/>
  <sce:test-timer id="t" duration="1s"/>
  <sce:test-timer id="u" duration="2s"/>
  <state id="outer" initial="a">
    <onexit><log label="bye"/></onexit>
    <transition sce:timeout="u" target="done"/>
    <state id="a">
      <onentry><sce:start-timer timer="t"/></onentry>
      <onexit><log label="leaving a"/></onexit>
      <transition sce:timeout="t" target="b"/>
    </state>
    <state id="b"/>
  </state>
  <final id="done"/>
</scxml>"#;

const TEMPLATE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="ask" sce:type="record:Msg">
  <sce:import kind="event-schema" src="m.scxml" as="Msg"/>
  <datamodel><data id="limit" sce:type="uint8" sce:direction="in"/></datamodel>
  <sce:record-match><sce:value-range field="n"><sce:upper param="limit"/></sce:value-range></sce:record-match>
</scxml>"#;

const SCHEMA: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="m">
  <datamodel><data id="n" sce:type="uint8" sce:direction="in"/></datamodel>
</scxml>"#;

/// The types module.
pub(crate) const TYPES: &str = "module s_Types {
type record m {
  uint8 n
}
}";

/// The behaviour module.
pub(crate) const BEHAVIOUR: &str = "module s {
import from s_Types all;
template m ask(uint8 limit) := {
  n := (-infinity .. limit)
};
type component c {
  timer t := 1.0;
  timer u := 2.0;
}
function f_c() runs on c {
  t.start;
  goto a;
  label a;
  alt {
    [] t.timeout {
      log(\"leaving a\");
      goto b;
    }
    [] u.timeout {
      log(\"leaving a\");
      log(\"bye\");
      return;
    }
  }
  label b;
  alt {
    [] u.timeout {
      log(\"bye\");
      return;
    }
  }
}
}";

/// The witness of the two modules: every entry the places of the types,
/// the template, the component type and the function have.
pub(crate) const WITNESS: &str = r#"{"schema": "sce-ttcn3-witness.v1", "suite": "s", "modules": ["s_Types", "s"],
  "entries": [
    {"construct": {"document": "m.scxml", "node": "scxml", "line": 1}, "rule": "E1",
     "produced": [{"module": "s_Types", "path": [{"type": "m"}]}]},
    {"construct": {"document": "ask.scxml", "node": "scxml", "line": 1}, "rule": "E2",
     "produced": [{"module": "s", "path": [{"template": "ask"}]}]},
    {"construct": {"document": "c.scxml", "node": "scxml", "line": null}, "rule": "E4",
     "produced": [{"module": "s", "path": [{"component-type": "c"}]}]},
    {"construct": {"document": "c.scxml", "node": "scxml", "line": null}, "rule": "E5",
     "produced": [{"module": "s", "path": [{"function": "f_c"}]}]},
    {"construct": {"document": "c.scxml", "node": "scxml", "line": null}, "rule": "E8",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"statement": 1}]}]},
    {"construct": {"document": "c.scxml", "node": "sce:test-timer 't'", "line": 4}, "rule": "E4",
     "produced": [{"module": "s", "path": [{"component-type": "c"}, {"timer": "t"}]}]},
    {"construct": {"document": "c.scxml", "node": "sce:test-timer 'u'", "line": 5}, "rule": "E4",
     "produced": [{"module": "s", "path": [{"component-type": "c"}, {"timer": "u"}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'outer' / onexit 1 / log 1", "line": 6}, "rule": "E13",
     "produced": [
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 1}, {"statement": 1}]},
       {"module": "s", "path": [{"function": "f_c"}, {"label": "b"}, {"statement": 0}, {"alternative": 0}, {"statement": 0}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'outer' / transition 1", "line": 8}, "rule": "E8",
     "produced": [
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 1}],
        "claim": {"kind": "branch",
          "state": {"document": "c.scxml", "node": "state 'a'", "line": 9},
          "transition": {"document": "c.scxml", "node": "state 'outer' / transition 1", "line": 8},
          "exited": [{"document": "c.scxml", "node": "state 'a'", "line": 9},
                     {"document": "c.scxml", "node": "state 'outer'", "line": 6}],
          "entered": [{"document": "c.scxml", "node": "final 'done'", "line": 16}]}},
       {"module": "s", "path": [{"function": "f_c"}, {"label": "b"}, {"statement": 0}, {"alternative": 0}],
        "claim": {"kind": "branch",
          "state": {"document": "c.scxml", "node": "state 'b'", "line": 14},
          "transition": {"document": "c.scxml", "node": "state 'outer' / transition 1", "line": 8},
          "exited": [{"document": "c.scxml", "node": "state 'b'", "line": 14},
                     {"document": "c.scxml", "node": "state 'outer'", "line": 6}],
          "entered": [{"document": "c.scxml", "node": "final 'done'", "line": 16}]}}]},
    {"construct": {"document": "c.scxml", "node": "state 'a'", "line": 9}, "rule": "E6",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"label": "a"}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'a'", "line": 9}, "rule": "E7",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}],
       "claim": {"kind": "alt",
         "state": {"document": "c.scxml", "node": "state 'a'", "line": 9},
         "alternatives": [{"document": "c.scxml", "node": "state 'a' / transition 1", "line": 12},
                          {"document": "c.scxml", "node": "state 'outer' / transition 1", "line": 8}]}}]},
    {"construct": {"document": "c.scxml", "node": "state 'a' / onentry 1 / sce_start_timer 1", "line": 9}, "rule": "E10",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"statement": 0}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'a' / onexit 1 / log 1", "line": 9}, "rule": "E13",
     "produced": [
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 0}, {"statement": 0}]},
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 1}, {"statement": 0}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'a' / transition 1", "line": 12}, "rule": "E8",
     "produced": [
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 0}],
        "claim": {"kind": "branch",
          "state": {"document": "c.scxml", "node": "state 'a'", "line": 9},
          "transition": {"document": "c.scxml", "node": "state 'a' / transition 1", "line": 12},
          "exited": [{"document": "c.scxml", "node": "state 'a'", "line": 9}],
          "entered": [{"document": "c.scxml", "node": "state 'b'", "line": 14}]}},
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 0}, {"statement": 1}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'b'", "line": 14}, "rule": "E6",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"label": "b"}]}]},
    {"construct": {"document": "c.scxml", "node": "state 'b'", "line": 14}, "rule": "E7",
     "produced": [{"module": "s", "path": [{"function": "f_c"}, {"label": "b"}, {"statement": 0}],
       "claim": {"kind": "alt",
         "state": {"document": "c.scxml", "node": "state 'b'", "line": 14},
         "alternatives": [{"document": "c.scxml", "node": "state 'outer' / transition 1", "line": 8}]}}]},
    {"construct": {"document": "c.scxml", "node": "final 'done'", "line": 16}, "rule": "E12",
     "produced": [
       {"module": "s", "path": [{"function": "f_c"}, {"label": "a"}, {"statement": 0}, {"alternative": 1}, {"statement": 2}]},
       {"module": "s", "path": [{"function": "f_c"}, {"label": "b"}, {"statement": 0}, {"alternative": 0}, {"statement": 1}]}]}
  ]}"#;

/// The documents of the suite.
pub(crate) fn documents() -> Documents {
    let files = Memory(BTreeMap::from([
        ("s.scxml", SUITE),
        ("case.scxml", CASE),
        ("config.scxml", CONFIG),
        ("c.scxml", COMPONENT),
        ("ask.scxml", TEMPLATE),
        ("m.scxml", SCHEMA),
    ]));
    Documents::read(&files, "s.scxml").expect("the documents")
}

/// The module `text` is.
pub(crate) fn module(text: &str) -> Module {
    ttcn3::parse(text).unwrap_or_else(|error| panic!("{error}"))
}

/// The witness `value` is.
pub(crate) fn witness_of(value: &serde_json::Value) -> Witness {
    witness::read(&value.to_string()).expect("a witness")
}

/// The witness, as JSON to change.
pub(crate) fn witness_value() -> serde_json::Value {
    serde_json::from_str(WITNESS).expect("the witness is JSON")
}

/// The position of the entry of the construct whose node is `node` and of
/// `rule` among the entries of `witness`.
pub(crate) fn position(witness: &serde_json::Value, node: &str, rule: &str) -> usize {
    witness["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .position(|entry| entry["construct"]["node"] == node && entry["rule"] == rule)
        .unwrap_or_else(|| panic!("no entry ({node}, {rule})"))
}

/// Each location of `witness` whose path begins with `prefix`, the segment
/// after the prefix changed by `change`.
pub(crate) fn rewrite_paths(
    witness: &mut serde_json::Value,
    prefix: &serde_json::Value,
    change: impl Fn(&serde_json::Value) -> serde_json::Value,
) {
    let prefix = prefix.as_array().expect("a path");
    for entry in witness["entries"].as_array_mut().expect("entries") {
        for location in entry["produced"].as_array_mut().expect("produced") {
            let path = location["path"].as_array_mut().expect("a path");
            if path.len() > prefix.len() && path[..prefix.len()] == prefix[..] {
                let at = prefix.len();
                path[at] = change(&path[at]);
            }
        }
    }
}
