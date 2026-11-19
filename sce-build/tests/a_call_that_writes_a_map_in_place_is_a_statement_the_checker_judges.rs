// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart calls an algorithm that writes a map in place by a statement,
//! `<sce:call expr="Algo(m, x)" location="took"/>` (docs/adr/0108, D6 item 2),
//! judged by the `sce-static` checker: `check` accepts
//! `fixtures/static_datamodel/static_map_take.scxml`, which hands
//! `algorithm_take_generation.scxml` its table of each peer's latest generation
//! and branches on the union it answers, and refuses each call that is no such
//! statement as `scxml/static-datamodel-rule`, saying what is accepted: an
//! `expr` that is not one call of an imported algorithm, an algorithm that
//! writes no map in place, an argument written in place that is no map
//! variable of the parameter's entries or is the map a `<foreach>` around the
//! call walks, a map named twice, and a `location` on an algorithm that returns
//! nothing; a call of such an algorithm inside any expression; and the
//! attributes of the other reader's `<sce:call>`, in a statechart and in an
//! algorithm's body. The `location` takes what the call returns by the rule an
//! `<assign>` of that value follows. The Interpreter's lowering lends the map
//! ("Order of work", step 7), and every generated lane that lowers no
//! algorithm's map yet refuses the document by name.
//!
//! Each refusal is the well-formed document with one thing changed, so the
//! refusal is the change's and nothing else's.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The statechart's name in the fixture directory.
const STATECHART: &str = "static_map_take.scxml";

/// The documents the statechart imports, and the scalar algorithm a refusal
/// below calls, copied beside a variant of it.
const SIBLINGS: [&str; 6] = [
    "schema_map_peer.scxml",
    "enum_take_kind.scxml",
    "schema_take_duplicate.scxml",
    "union_take_result.scxml",
    "algorithm_take_generation.scxml",
    "algorithm_days_in_month.scxml",
];

/// An algorithm that empties the table it is lent in place and returns
/// nothing, written beside a variant of the statechart as
/// `algorithm_forget_peers.scxml`.
const FORGET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="forget_peers"
       version="1.0">
  <sce:import kind="event-schema" src="schema_map_peer.scxml" as="Zid"/>
  <sce:signature>
    <sce:param name="resets" type="map&lt;record:Zid, uint64&gt;" inout="true"/>
  </sce:signature>
  <sce:body>
    <sce:clear target="resets"/>
  </sce:body>
</scxml>
"#;

/// The well-formed statechart, as committed.
fn take() -> String {
    std::fs::read_to_string(fixtures().join(STATECHART)).expect("read the statechart")
}

/// `text` with the one occurrence of `from` replaced by `to`.
fn edited(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` stands once in the document"
    );
    text.replacen(from, to, 1)
}

/// The call the committed statechart makes, as written.
const CALL: &str =
    r#"<sce:call expr="TakeGeneration(resets, _event.data, generation)" location="took"/>"#;

/// The statechart with `call` in place of its own call.
fn calling(call: &str) -> String {
    edited(&take(), CALL, call)
}

/// The statechart with `import` among its imports and a transition on
/// `forget` that runs `statement`, its own call left as it is.
fn with_transition(import: &str, statement: &str) -> String {
    let imported = edited(
        &take(),
        "  <datamodel>\n",
        &format!("  {import}\n  <datamodel>\n"),
    );
    edited(
        &imported,
        "    <transition event=\"peer.seen\" type=\"internal\">",
        &format!(
            "    <transition event=\"forget\" type=\"internal\">\n      {statement}\n    \
             </transition>\n    <transition event=\"peer.seen\" type=\"internal\">"
        ),
    )
}

/// `sce-codegen --error-format=json <args> <path>`: whether it passed, and
/// what it printed.
fn run(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `check` of `document`, written beside the documents it may import.
fn check(document: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy an import");
    }
    std::fs::write(dir.path().join("algorithm_forget_peers.scxml"), FORGET)
        .expect("write the algorithm that forgets");
    let path = dir.path().join(STATECHART);
    std::fs::write(&path, document).expect("write the statechart");
    run(&["check"], &path)
}

/// `document` is refused by `check` with `code`, its message holding `part`.
fn refused(document: &str, code: &str, part: &str) {
    let (ok, out) = check(document);
    assert!(!ok, "expected a refusal holding `{part}`:\n{out}");
    assert!(
        out.contains(&format!("\"code\":\"{code}\"")) && out.contains(part),
        "expected {code} holding `{part}`:\n{out}"
    );
}

const RULE: &str = "scxml/static-datamodel-rule";

/// docs/adr/0108, D6 item 2: the call of an algorithm that writes the table
/// in place and answers a union is accepted, as the committed statechart and
/// beside copies of what it imports; so is a call of one that returns nothing,
/// written with no location.
#[test]
fn a_call_that_writes_a_map_in_place_is_accepted() {
    let (ok, out) = run(&["check"], &fixtures().join(STATECHART));
    assert!(ok, "check accepts the statechart:\n{out}");
    let (ok, out) = check(&take());
    assert!(ok, "check accepts it beside copies of its imports:\n{out}");
    let (ok, out) = check(&with_transition(
        r#"<sce:import kind="algorithm" src="algorithm_forget_peers.scxml" as="Forget"/>"#,
        r#"<sce:call expr="Forget(resets)"/>"#,
    ));
    assert!(
        ok,
        "a call with no location drops what nothing returns:\n{out}"
    );
}

/// docs/adr/0108, D6 item 2: what `expr` calls, and each argument written in
/// place, each refused by the rule it breaks.
#[test]
fn a_call_is_refused_naming_what_it_takes() {
    for (call, part) in [
        // One call of an imported algorithm, written whole.
        (
            r#"<sce:call expr="TakeGeneration(resets, _event.data, generation) + 1" location="took"/>"#,
            "its expr is that call written whole",
        ),
        (
            r#"<sce:call expr="Forget(resets)" location="took"/>"#,
            "its expr is that call written whole",
        ),
        // An argument written in place is a map variable named bare.
        (
            r#"<sce:call expr="TakeGeneration(_event.data, _event.data, generation)" location="took"/>"#,
            "is the name of a map or a set variable, and `_event.data` is not one",
        ),
        // ... and no map named twice in one call.
        (
            r#"<sce:call expr="TakeGeneration(resets, resets, generation)" location="took"/>"#,
            "no map or set is named twice in one call",
        ),
        // ... and not the map a loop around the call walks.
        (
            r#"<foreach array="resets" item="g" sce:key="k"><sce:call expr="TakeGeneration(resets, _event.data, generation)" location="took"/></foreach>"#,
            "a <foreach> around the call walks `resets`",
        ),
    ] {
        refused(&calling(call), RULE, part);
    }

    // An argument written in place holds the parameter's entries.
    let other_map = edited(
        &calling(r#"<sce:call expr="TakeGeneration(others, _event.data, generation)" location="took"/>"#),
        "    <data id=\"generation\"",
        "    <data id=\"others\" sce:type=\"map&lt;uint32, uint64&gt;[2]\" sce:direction=\"out\"/>\n    <data id=\"generation\"",
    );
    refused(
        &other_map,
        RULE,
        "whose key and value are the parameter's: `others` is of type map<uint32, uint64>",
    );

    // An algorithm that writes no map in place is called in an expression.
    refused(
        &with_transition(
            r#"<sce:import kind="algorithm" src="algorithm_days_in_month.scxml" as="MonthLength"/>"#,
            r#"<sce:call expr="MonthLength(2026, 2)"/>"#,
        ),
        RULE,
        "and `MonthLength` writes none",
    );

    // A location takes what the algorithm returns, so one that returns
    // nothing is called with none.
    refused(
        &with_transition(
            r#"<sce:import kind="algorithm" src="algorithm_forget_peers.scxml" as="Forget"/>"#,
            r#"<sce:call expr="Forget(resets)" location="taken"/>"#,
        ),
        RULE,
        "and `Forget` returns nothing",
    );
}

/// docs/adr/0108, D6 item 2: the location takes what the call returns by the
/// rule an `<assign>` of that value follows, the `<assign>`'s own judge: a
/// union is taken whole into a variable of its union, and into a number it is
/// the value an assignment refuses, in that judge's words.
#[test]
fn a_location_takes_the_return_by_the_rule_of_an_assign() {
    refused(
        &calling(
            r#"<sce:call expr="TakeGeneration(resets, _event.data, generation)" location="taken"/>"#,
        ),
        "expression/unsupported-construct",
        "used as a value",
    );
}

/// docs/adr/0108, D6 item 2: an algorithm that writes a map in place is called
/// by the statement and never in an expression, which has no effect: not as an
/// `<assign>`'s value and not in a condition.
#[test]
fn a_call_that_writes_in_place_is_refused_in_an_expression() {
    let in_place = "`TakeGeneration` writes a map in place, and an algorithm that does is called \
                    by the statement";
    refused(
        &calling(
            r#"<assign location="took" expr="TakeGeneration(resets, _event.data, generation)"/>"#,
        ),
        RULE,
        in_place,
    );
    refused(
        &edited(
            &take(),
            r#"<if cond="took.kind === TakeKind.taken">"#,
            r#"<if cond="TakeGeneration(resets, _event.data, generation).kind === TakeKind.taken">"#,
        ),
        RULE,
        in_place,
    );
}

/// docs/adr/0108, D6 item 2, and `schemas/sce-forge-ext.xsd`: the grammar
/// declares one `<sce:call>` for both readers, so each refuses the other's
/// attributes. A statechart's call names neither `target` nor `args`, and
/// names its call in `expr`; an algorithm's body names neither `expr` nor
/// `location`.
#[test]
fn each_reader_refuses_the_other_shape_of_a_call() {
    refused(
        &calling(r#"<sce:call target="TakeGeneration" args="resets, _event.data, generation"/>"#),
        RULE,
        "is an algorithm body's spelling of a call",
    );
    refused(
        &calling(r#"<sce:call location="took"/>"#),
        RULE,
        "a <sce:call> writes the call of the algorithm in `expr`",
    );

    let algorithm = std::fs::read_to_string(fixtures().join("algorithm_take_generation.scxml"))
        .expect("read the algorithm");
    let body_call = edited(
        &algorithm,
        "    <sce:var name=\"taken\" type=\"union:TakeResult\">",
        "    <sce:call target=\"Other\" args=\"generation\" location=\"stored\"/>\n    <sce:var \
         name=\"taken\" type=\"union:TakeResult\">",
    );
    let label = sce_build::DocumentLabel::symmetric("algorithm_take_generation");
    let refusal = match sce_build::forge::parser::parse_forge_with_imports(&body_call, label) {
        Err(located) => located.error,
        Ok(_) => panic!("an algorithm's <sce:call> with a location is refused"),
    };
    assert!(
        refusal
            .to_string()
            .contains("`expr` and `location` are a statechart's spelling of a call"),
        "{refusal}"
    );
}

/// An algorithm that empties the table `b` in place and is lent the table `a`
/// to read, written beside the statechart [`WALKED`] as
/// `algorithm_refill.scxml`.
const REFILL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="refill"
       version="1.0">
  <sce:signature>
    <sce:param name="b" type="map&lt;uint32, uint64&gt;" inout="true"/>
    <sce:param name="a" type="map&lt;uint32, uint64&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:clear target="b"/>
  </sce:body>
</scxml>
"#;

/// A statechart whose `<foreach>` over `a` calls `Refill(b, a)`: `b` is
/// written in place, and `a`, the map the loop walks, is lent to be read.
const WALKED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="walk_lend">
  <sce:import kind="algorithm" src="algorithm_refill.scxml" as="Refill"/>
  <datamodel>
    <data id="a" sce:type="map&lt;uint32, uint64&gt;[4]" sce:direction="out"/>
    <data id="b" sce:type="map&lt;uint32, uint64&gt;[4]" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <foreach array="a" item="v" sce:key="k">
        <sce:call expr="Refill(b, a)"/>
      </foreach>
    </transition>
  </state>
</scxml>
"#;

/// docs/adr/0108, D6 items 2 and 3: a call writes only the maps its callee
/// writes in place, as the callee resolved where the document is read names
/// them, so a `<foreach>` over a map lent to the call to be read walks it
/// without a copy, and the map lent to be written is written.
#[test]
fn a_loop_over_a_map_lent_to_be_read_walks_it_without_a_copy() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(dir.path().join("algorithm_refill.scxml"), REFILL).expect("write the algorithm");
    let path = dir.path().join("walk_lend.scxml");
    std::fs::write(&path, WALKED).expect("write the statechart");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(path.to_str().expect("a utf-8 path"))
        .unwrap_or_else(|e| panic!("the statechart is read and judged: {e:?}"));
    let walk = &model.states["s"].transitions[0].actions[0];
    assert_eq!(walk.action_type, "foreach");
    assert_eq!(walk.actions[0].in_place_arguments, ["b"]);
    let writes = sce_build::forge::narrowing::Writes::of_block(&walk.actions);
    assert!(!writes.reaches("a"), "the walked map is lent to be read");
    assert!(writes.reaches("b"), "the map written in place is written");
}

/// docs/adr/0108, "Order of work", step 7: a lane lends an algorithm a map in
/// the step that lowers an algorithm's map on it. The Interpreter's lowering,
/// the first (D10), lowers the statechart: the `<sce:call>` becomes a
/// `<script>` that lends the table in a holder carrying the variable's bound,
/// makes the call, and writes the table and then `took` once the call has
/// returned, and the algorithm reads the table out of the holder, holds its
/// put to that bound and leaves the table it wrote there however it returns.
/// `generate` refuses the statechart, as an unsupported feature, on every
/// generated lane that lowers no algorithm's map yet
/// (`codegen_matrix::ALGORITHM_MAP_BACKENDS`), by the one refusal each such
/// lane asks first, naming the callee, the map slot it takes and that step,
/// before the lane reads the map the statechart holds or lowers the algorithm
/// it imports.
#[test]
fn the_interpreter_lends_the_map_and_every_other_lane_refuses_the_call_by_name() {
    let path = fixtures().join(STATECHART);
    let named = |out: &str, lane: &str| {
        out.contains("`TakeGeneration(")
            && out.contains("an imported algorithm that takes the inout map<record:Peer, uint64>")
            && out.contains(&format!(
                "has no {lane} lowering of the call yet (docs/adr/0108, Order of work, step 7)"
            ))
    };
    let unlowered = sce_build::generator::Language::ALL
        .iter()
        .filter(|language| {
            !sce_build::forge::codegen_matrix::ALGORITHM_MAP_BACKENDS.contains(language)
        });
    for &language in unlowered {
        let lane = language.canonical_name();
        let out_dir = tempfile::tempdir().expect("an output directory");
        let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
        let mut args = vec!["generate", "-l", lane, "-o", out_dir.as_str()];
        if language == sce_build::generator::Language::Go {
            args.extend(["--go-module-prefix", "example.test/generated"]);
        }
        let (ok, out) = run(&args, &path);
        assert!(!ok, "{lane} lowers no call that lends a map yet:\n{out}");
        assert!(
            out.contains("\"code\":\"generate/unsupported-feature\"") && named(&out, lane),
            "{lane}:\n{out}"
        );
    }
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers the call: {e:?}"));
    for part in [
        // The statement: the table lent in a holder of its bound, under the
        // placeholder its argument took, and written back, then the answer
        // stored, once the call has returned.
        "take_generation($sceLent.__st0, ",
        "resets = $sceLent.__st0.map; took = $sceCalled; })({__st0: {map: resets, capacity: 2}});",
        // The algorithm: the table read out of the holder, a put held to the
        // holder's bound, and the table left in the holder however it returns.
        "function (__lent0, peer, generation) { var resets = __lent0.map; try { ",
        "__lent0.capacity, peer, generation); resets = __wrote.map; stored = __wrote.outcome; }",
        " } finally { __lent0.map = resets; } }",
    ] {
        assert!(
            lowered.contains(part),
            "the lowered statechart holds `{part}`:\n{lowered}"
        );
    }
    assert!(
        !lowered.contains("<sce:call expr="),
        "the <sce:call> is replaced whole:\n{lowered}"
    );
}
