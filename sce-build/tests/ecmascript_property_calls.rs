// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What happens when a name this datamodel provides is written as a call.
//
// `ecmascript_builtin_vocabulary` asks whether a name exists and
// `ecmascript_identifier_scope` asks whether the document declares it.
// Neither asks what was *done* with a name that does exist, and that gap
// had a measurable shape: `t.length()` generated cleanly on all six
// backends, `check --lint` answered exit 0 with no record on any stream,
// and the machine died at runtime attempting to call a nil value. Its
// neighbour was worse than silent — `Math.PI()` was answered *"Math.PI
// is not provided by SCE's ECMAScript datamodel"*, which is false: the
// emitter lowers `Math.PI` to `math.pi` and always has.
//
// Every claim below is made by lowering an expression and reading the
// answer, and the claims about what a lowering *means* are made by
// running it on `sce-rust-lua` — the engine a generated Rust machine
// uses — rather than by matching the emitted text. A constant that
// lowered to the wrong arithmetic would satisfy a text assertion and
// fail here.

use sce_build::ecmascript::builtins::{
    Namespace, INSTALLED_GLOBALS, MATH_CONSTANTS, MATH_FUNCTIONS, VALUE_GLOBALS, VALUE_PROPERTIES,
};
use sce_build::ecmascript::{to_lua_value, DocumentScope, ExprError};
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};
use std::collections::BTreeSet;

/// The receivers these probes hang a call on.
///
/// `handlers` is the counterweight: an author's own object, whose method
/// call has to survive every rule this file asserts.
fn probes() -> DocumentScope {
    DocumentScope::declaring(["arr", "handlers", "t", "out"])
}

/// The refusal `source` produced, or a panic naming what came back
/// instead — a probe that lowered cleanly is the defect this file
/// exists for, so it must not read as a pass.
fn refusal(source: &str) -> ExprError {
    match to_lua_value(source, &probes()) {
        Err(err) => err,
        Ok(lua) => panic!("{source} was accepted and lowered to {lua}"),
    }
}

/// The name a `PropertyNotCallable` refusal offers as the repair.
fn property_refused(source: &str) -> String {
    match refusal(source) {
        ExprError::PropertyNotCallable { name, .. } => name,
        other => panic!("{source} was refused, but as {other:?} rather than a property call"),
    }
}

// ── The property called as a method ──────────────────────────────

/// `.length` is a property on every receiver the emitter lowers it for,
/// so calling it is refused on every receiver too.
///
/// The receivers are varied deliberately: the rule is decided from the
/// name, and a rule that only fired on a bare identifier would let
/// `_event.data.items.length()` — the shape a document handling an
/// event payload actually writes — through.
#[test]
fn a_length_call_is_refused_whatever_it_is_called_on() {
    for source in [
        "t.length()",
        "arr.length()",
        "arr[0].length()",
        "_event.name.length()",
        "t.charAt(0).length()",
        "handlers.items.length()",
    ] {
        assert_eq!(
            property_refused(source),
            ".length",
            "{source} did not name .length as its repair"
        );
    }
}

/// The property itself still lowers, on both receivers ECMA-262 gives it
/// to.
///
/// The refusal above is one token away from this, so a rule that reached
/// too far would take the whole of `.length` with it. That would be a
/// worse defect than the one being fixed: `length` is the only way this
/// datamodel measures a string or an array.
#[test]
fn the_property_itself_is_untouched() {
    let engine = engine_with(&[
        ("t", "'hello'"),
        ("arr", "{'a', 'b', 'c'}"),
        ("handlers", "{ items = {1, 2} }"),
    ]);
    for (source, expected) in [
        ("t.length", 5.0),
        ("arr.length", 3.0),
        ("handlers.items.length", 2.0),
    ] {
        assert_eq!(
            evaluate(&engine, source),
            expected,
            "{source} did not measure what it names"
        );
    }
}

/// An author's own method call survives, which is the half of the rule
/// that is easy to lose.
///
/// The datamodel has no prototype chain and no types, so a member call
/// is an author's function unless a closed list says otherwise. If this
/// test ever fails, the list stopped being closed.
#[test]
fn an_authored_method_call_is_still_lowered() {
    for source in [
        "handlers.retry()",
        "handlers.retry(1)",
        "handlers.length_check()",
        "t.charAt(0)",
        "arr.join(',')",
    ] {
        to_lua_value(source, &probes())
            .unwrap_or_else(|err| panic!("{source} is the author's own and was refused: {err}"));
    }
}

// ── The namespace constant called as a method ────────────────────

/// Every constant ECMA-262 15.8.1 defines is lowered, and lowers to the
/// number the clause gives.
///
/// Read against the specification rather than against the emitter: the
/// expected values are ECMA-262's, so an arm that computed
/// `1 / math.log(2)` as `math.log(2)` would fail here even though both
/// are plausible-looking Lua. Four of these eight were missing when this
/// test was written, and the four that were missing generated a runtime
/// `error(...)` while `check` answered ok.
#[test]
fn every_math_constant_the_specification_defines_is_provided() {
    let engine = engine_with(&[]);
    // ECMA-262 15.8.1.1 through 15.8.1.8, in the clause's order.
    let clause: [(&str, f64); 8] = [
        ("Math.E", std::f64::consts::E),
        ("Math.LN10", std::f64::consts::LN_10),
        ("Math.LN2", std::f64::consts::LN_2),
        ("Math.LOG10E", std::f64::consts::LOG10_E),
        ("Math.LOG2E", std::f64::consts::LOG2_E),
        ("Math.PI", std::f64::consts::PI),
        ("Math.SQRT1_2", std::f64::consts::FRAC_1_SQRT_2),
        ("Math.SQRT2", std::f64::consts::SQRT_2),
    ];
    for (source, expected) in clause {
        let actual = evaluate(&engine, source);
        assert!(
            (actual - expected).abs() <= expected.abs() * 1e-15,
            "{source} evaluated to {actual}, and ECMA-262 15.8.1 says {expected}"
        );
    }
    let named: BTreeSet<&str> = clause
        .iter()
        .map(|(source, _)| source.trim_start_matches("Math."))
        .collect();
    assert_eq!(
        named,
        MATH_CONSTANTS.iter().copied().collect::<BTreeSet<_>>(),
        "the constants this test runs and the constants the frontend \
         lists are not one set"
    );
}

/// A constant called as a function is refused as the property call it
/// is, not as a missing name.
///
/// This is the misclassification the new code exists to end. Asserting
/// the variant rather than the exit code is the point: both readings
/// refuse the expression, and only one of them tells the truth about
/// `Math.PI`.
#[test]
fn a_math_constant_called_as_a_method_is_refused_by_the_property_rule() {
    for &constant in MATH_CONSTANTS {
        let source = format!("Math.{constant}()");
        assert_eq!(
            property_refused(&source),
            format!("Math.{constant}"),
            "{source} was not answered as a property call"
        );
    }
}

/// A `Math` member that is neither a function nor a constant is still
/// answered against the vocabulary, and the vocabulary a *call* is
/// offered holds only the callable half.
///
/// Offering `Math.PI` as the repair for `Math.tanh(x)` would turn one
/// refusal into another, which is the failure mode a candidate list has
/// that a single replacement does not.
#[test]
fn an_unknown_math_call_is_offered_only_the_callable_members() {
    match refusal("Math.tanh(1)") {
        ExprError::UnsupportedBuiltin { name, available } => {
            assert_eq!(name, "Math.tanh");
            let offered: BTreeSet<String> = available.into_iter().collect();
            let callable: BTreeSet<String> =
                MATH_FUNCTIONS.iter().map(|m| format!("Math.{m}")).collect();
            assert_eq!(
                offered, callable,
                "the candidates for a call are not the callable members"
            );
        }
        other => panic!("Math.tanh(1) was refused as {other:?}"),
    }
}

/// A read of an unknown `Math` member names the vocabulary too, and that
/// vocabulary includes the constants — a read may legally name one.
#[test]
fn an_unknown_math_read_is_offered_both_halves() {
    match refusal("Math.TAU") {
        ExprError::UnsupportedBuiltin { name, available } => {
            assert_eq!(name, "Math.TAU");
            assert!(
                available.contains(&"Math.PI".to_string())
                    && available.contains(&"Math.abs".to_string()),
                "a read was offered {available:?}, which is not the whole namespace"
            );
        }
        other => panic!("Math.TAU was refused as {other:?}"),
    }
}

// ── The system variable called as a function ─────────────────────

/// A system variable holds a value, so calling one is refused.
#[test]
fn a_system_variable_called_as_a_function_is_refused() {
    for &global in VALUE_GLOBALS {
        let source = format!("{global}()");
        assert_eq!(
            property_refused(&source),
            global,
            "{source} was not answered as a property call"
        );
    }
}

// ── The namespace written as the call ────────────────────────────

/// The members a `NamespaceNotCallable` refusal says may stand there.
fn namespace_refused(source: &str) -> (String, Vec<String>) {
    match refusal(source) {
        ExprError::NamespaceNotCallable { namespace, members } => (namespace, members),
        other => panic!("{source} was refused, but as {other:?} rather than a namespace call"),
    }
}

/// Every namespace this datamodel installs is refused in the position
/// where it is the call rather than the receiver of one.
///
/// Driven from `Namespace::ALL` rather than a written-out list: the
/// three differ in what they are at runtime — `Math` is not bound on the
/// engines at all, `JSON` and `Object` are tables — and a rule written
/// against one of those facts would leave the other two silent.
#[test]
fn every_namespace_is_refused_when_it_is_the_call_itself() {
    for namespace in Namespace::ALL {
        let name = namespace.name();
        for source in [
            format!("{name}()"),
            format!("{name}(1, 2)"),
            format!("new {name}()"),
            // The receiver position of an author's own expression, so a
            // rule that only read the top-level callee would miss it.
            format!("handlers.retry({name}())"),
        ] {
            let (refused, _) = namespace_refused(&source);
            assert_eq!(refused, name, "{source} named the wrong namespace");
        }
    }
}

/// The refusal names the members that may stand where the call was
/// written, and names the callable half only.
///
/// `Math.PI` is a member of `Math` and cannot stand where a call was
/// written, so offering it would repair one refusal into another — the
/// same reasoning `an_unknown_math_call_is_offered_only_the_callable_members`
/// pins one AST node down.
#[test]
fn the_namespace_refusal_offers_the_callable_members_qualified() {
    let (_, members) = namespace_refused("Math()");
    assert!(
        members.contains(&"Math.abs".to_string()),
        "the members are not qualified by their namespace: {members:?}"
    );
    for constant in MATH_CONSTANTS {
        assert!(
            !members.contains(&format!("Math.{constant}")),
            "Math.{constant} cannot stand where a call was written: {members:?}"
        );
    }
    let (_, json) = namespace_refused("JSON()");
    assert_eq!(
        json,
        vec!["JSON.parse".to_string(), "JSON.stringify".to_string()],
        "the JSON members drifted from the library that installs them"
    );
}

/// Dropping the call — the repair `PropertyNotCallable` offers — is not
/// a repair here, which is why the two are different codes.
///
/// Asserted on the engine rather than argued: `Math` on its own lowers
/// to a name nothing binds there, so a consumer that applied "write it
/// without the call" would trade a reported refusal for a failure at
/// evaluation. A record that carried that as its `fix` would be telling
/// an author to make the document worse.
#[test]
fn dropping_the_call_would_not_repair_the_namespace() {
    let lowered = to_lua_value("Math", &probes()).expect("a bare namespace still lowers");
    let (engine, session) = engine_with(&[]);
    let evaluated = engine.evaluate_expression(&session, &lowered);
    assert!(
        evaluated.is_err(),
        "Math evaluated to {evaluated:?} — if the engine binds it after all, \
         the refusal could offer dropping the call as its repair"
    );
}

/// A member call on every namespace still lowers and still evaluates.
///
/// The counterweight the rule above needs: the refusal is decided from
/// the callee position alone, and a rule that read the identifier
/// without reading its position would take the whole standard library
/// with it.
#[test]
fn a_member_call_on_each_namespace_still_evaluates() {
    let session = engine_with(&[("arr", "{1, 2, 3}"), ("handlers", "{}")]);
    assert_eq!(evaluate(&session, "Math.abs(-3)"), 3.0);
    assert_eq!(evaluate(&session, "JSON.parse('[1,2]').length"), 2.0);
    assert_eq!(evaluate(&session, "Object.keys(handlers).length"), 0.0);
}

/// Every namespace `Namespace::ALL` lists is one an author can reach by
/// name, and every name that reaches one is listed.
///
/// The two-way binding is what keeps a fourth namespace from being
/// installed into `from_ident` and forgotten by every rule written over
/// the list.
#[test]
fn every_namespace_this_datamodel_installs_is_reachable_by_ident() {
    for namespace in Namespace::ALL {
        assert_eq!(
            Namespace::from_ident(namespace.name()),
            Some(*namespace),
            "{} is listed but no identifier reaches it",
            namespace.name()
        );
        assert!(
            INSTALLED_GLOBALS.contains(&namespace.name()),
            "{} is a namespace this datamodel does not bind",
            namespace.name()
        );
    }
    for &global in INSTALLED_GLOBALS {
        if let Some(namespace) = Namespace::from_ident(global) {
            assert!(
                Namespace::ALL.contains(&namespace),
                "{global} reaches a namespace missing from Namespace::ALL"
            );
        }
    }
}

/// Every name declared uncallable is a name this datamodel binds.
///
/// The containment is the claim that keeps the list honest: a name
/// outside `INSTALLED_GLOBALS` is the author's own, and the author's own
/// names may perfectly well hold functions.
#[test]
fn every_value_global_is_installed() {
    for &global in VALUE_GLOBALS {
        assert!(
            INSTALLED_GLOBALS.contains(&global),
            "{global} is declared uncallable but this datamodel does not bind it"
        );
    }
}

/// The globals this datamodel installs as functions are still callable.
///
/// The complement of the test above, and the one that would catch a
/// `VALUE_GLOBALS` that grew past the system variables.
#[test]
fn the_installed_functions_are_still_callable() {
    for source in [
        "parseInt('7')",
        "parseFloat('1.5')",
        "String(1)",
        "Number('2')",
        "Boolean(1)",
        "JSON.stringify(arr)",
        "Object.keys(handlers)",
        "Math.abs(1)",
    ] {
        to_lua_value(source, &probes()).unwrap_or_else(|err| {
            panic!("{source} names an installed function and was refused: {err}")
        });
    }
}

// ── The lists themselves ─────────────────────────────────────────

/// A `Math` member is a function or a constant, never both.
///
/// The two lists decide different answers for the same syntax, so an
/// overlap would make the answer depend on which check ran first.
#[test]
fn the_math_functions_and_constants_are_disjoint() {
    let functions: BTreeSet<&str> = MATH_FUNCTIONS.iter().copied().collect();
    let constants: BTreeSet<&str> = MATH_CONSTANTS.iter().copied().collect();
    let overlap: Vec<&&str> = functions.intersection(&constants).collect();
    assert!(
        overlap.is_empty(),
        "these Math members are listed as both callable and constant: {overlap:?}"
    );
}

/// Both new lists are sorted and free of duplicates, so a reader can
/// find a name and a reviewer can see an addition.
#[test]
fn the_new_vocabularies_are_sorted_and_unique() {
    for (label, list) in [
        ("MATH_CONSTANTS", MATH_CONSTANTS),
        ("VALUE_GLOBALS", VALUE_GLOBALS),
        ("VALUE_PROPERTIES", VALUE_PROPERTIES),
    ] {
        let mut sorted = list.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted,
            list.to_vec(),
            "{label} is not sorted or has a duplicate"
        );
    }
}

/// Every standard data property listed lowers as a property, which is
/// what makes refusing the call form safe.
///
/// The rule's whole justification is that the emitter has already taken
/// the name for the standard property whatever the receiver holds. If a
/// name were listed here without that lowering, the refusal would be
/// stealing a member from an author's object.
#[test]
fn every_value_property_is_lowered_as_a_property() {
    for &property in VALUE_PROPERTIES {
        let lua = to_lua_value(&format!("handlers.{property}"), &probes())
            .unwrap_or_else(|err| panic!("handlers.{property} is listed and was refused: {err}"));
        assert!(
            !lua.contains(&format!(".{property}")),
            "handlers.{property} lowered to {lua}, which is an ordinary field \
             read — the call form cannot be refused on a name the emitter \
             leaves to the author"
        );
    }
}

// ── Running the lowered expression ───────────────────────────────

/// A session on the engine a generated Rust machine runs, with the named
/// variables already assigned.
fn engine_with(variables: &[(&str, &str)]) -> (LuaEngine, String) {
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "property_calls".to_string();
    engine.create_session(&session);
    for (name, lua) in variables {
        engine
            .execute_script(&session, &format!("{name} = {lua}"))
            .unwrap_or_else(|err| panic!("could not establish {name}: {err}"));
    }
    (engine, session)
}

/// The number `source` evaluates to once lowered — the production path,
/// not a reimplementation of it.
fn evaluate((engine, session): &(LuaEngine, String), source: &str) -> f64 {
    let lua = to_lua_value(source, &probes())
        .unwrap_or_else(|err| panic!("{source} did not lower: {err}"));
    match engine.evaluate_expression(session, &lua) {
        // Lua's `#` yields an integer and its `math` library a float, so
        // both spellings of a number are the same answer here.
        Ok(ScriptValue::Int(n)) => n as f64,
        Ok(ScriptValue::Double(n)) => n,
        Ok(other) => panic!("{source} lowered to {lua} and evaluated to {other:?}"),
        Err(err) => panic!("{source} lowered to {lua} and did not evaluate: {err}"),
    }
}
