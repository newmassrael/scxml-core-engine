// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which guards a backend may emit without a data model, and what it
// emits for them.
//
// Every backend has an arm for a guard that needs no script engine, and
// the arm printed the author's `cond` text straight into target-language
// source. What chose it was a substring classifier whose fallthrough was
// *native*, so a condition it did not recognise went that way — and the
// list it recognised was operators, quote characters and a handful of
// reserved words. The measurable shape:
//
//     cond="1"            Rust `if 1 {`, Go `if 1 {`, Kotlin `&& 1 ->`
//                         — none of which compiles. C++ and C11 compiled
//                         it as *their* truthiness, and Python alone
//                         lowered it through `_scxml_truthy`.
//     cond="x"            `if x {`, naming nothing, on six backends
//     cond="Math.abs(1)"  reached the backend without ever passing the
//                         frontend that owns the name `Math`
//
// All of them: `check` exit 0, no record on any stream.
//
// Two claims are pinned here, and the second is what keeps the first
// from being bought with a regression:
//
//   1. **A guard the data model has to evaluate reaches the data
//      model** — and therefore reaches the expression frontend, whose
//      rules then apply to it like any other expression.
//   2. **A guard SCE can decide is still decided at build time**, and
//      the value emitted is ECMA-262's, not the target language's.
//      W3C 403a, 403b and 403c write `cond="false"` and are generated as
//      pure-static machines; making them carry a Lua engine to evaluate
//      a literal would be the wrong repair.

use sce_build::ecmascript::constant_truthiness;

/// ECMA-262 9.2 ToBoolean, one case per type the clause defines.
///
/// The expected values are the specification's rather than the
/// emitter's: `0` is false where every language agrees, and `"0"` is
/// **true** where C and C++ would call the number it parses to false —
/// which is the difference between folding the author's language and
/// splicing text into the host's.
#[test]
fn a_literal_folds_to_what_the_specification_says_it_is() {
    for (source, expected) in [
        ("true", true),
        ("false", false),
        ("1", true),
        ("0", false),
        ("0.5", true),
        ("-1", true),
        ("-0", false),
        ("0x0", false),
        ("0x1f", true),
        ("''", false),
        ("'0'", true),
        ("'text'", true),
        ("null", false),
        ("undefined", false),
        ("!false", true),
        ("!1", false),
        ("!!''", false),
    ] {
        assert_eq!(
            constant_truthiness(source),
            Some(expected),
            "{source} did not fold to {expected}"
        );
    }
}

/// A guard that names anything is not decidable here, however simple it
/// looks.
///
/// This is the half the old classifier got wrong: `x` has no operator,
/// no quote and is not a reserved word, so it fell through to *native*
/// and was emitted as target source. A name is exactly what the data
/// model holds, so it is exactly what cannot be answered without one.
#[test]
fn a_guard_that_names_something_is_not_decidable_at_build_time() {
    for source in [
        "x",
        "ready",
        "foo.bar",
        "f(1)",
        "Math.abs(1)",
        "_event.name",
        "1 == 1",
        "0 && 1",
        "'a' + 'b'",
        "In('s1') && x",
        // Not an expression at all — W3C test 344 writes this on
        // purpose, and the frontend refuses it where the guard is
        // lowered rather than here.
        "return",
        "@@",
    ] {
        assert_eq!(
            constant_truthiness(source),
            None,
            "{source} was decided without the data model"
        );
    }
}

/// `1 == 1` is not folded even though its value is obvious.
///
/// Deliberate, and worth its own test so a later reader does not "fix"
/// it: an operator's ECMAScript semantics are the data model's to
/// apply, and a constant folder that grew into them would be a second
/// evaluator to keep in step with the six that already exist. Sending
/// it to the engine costs a document nothing it was not already paying.
#[test]
fn the_fold_stops_at_the_literal_grammar() {
    assert_eq!(constant_truthiness("1 == 1"), None);
    assert_eq!(constant_truthiness("'a' === 'a'"), None);
    assert_eq!(constant_truthiness("!(1 == 1)"), None);
}
