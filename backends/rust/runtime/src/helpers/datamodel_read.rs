// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Typed reads of a live datamodel variable.
//!
//! The counterpart to [`crate::helpers::datamodel_init`], which puts a
//! declared `<data>` variable into the session. This module takes one back
//! out in the host's own type, and it exists so that a generated machine can
//! answer a question about its own datamodel without the caller holding a
//! script engine, a session id and the variable's name as a string.
//!
//! ## Why the read goes to the engine rather than to a copy
//!
//! A `<data>` variable with an initializer is owned by the script engine for
//! the whole life of the session — `<assign>` writes there, guards read from
//! there. Anything the generated struct kept alongside it would be a second
//! representation of one variable, and it would be wrong from the first
//! `<assign>` onwards. So there is exactly one home for the value, and these
//! helpers read it.
//!
//! ## Why the answer is optional
//!
//! Three things can make a typed read impossible, and none of them is a
//! defect in the caller: the session is not initialized yet, the variable was
//! assigned a value of another type while the run was going, or the engine
//! refused. All three mean the same thing to a consumer — the machine cannot
//! answer that right now — so they collapse into `None` rather than into an
//! error type the caller would have to match on to reach the same decision.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2: the whole module is gated to
//! `!no_std`. A typed accessor exists only for a `<data>` that carries an
//! initializer, and such a document needs a script engine, which the no_std
//! codegen rejects up front (`codegen/no-std-script-not-supported`).

use crate::scripting::{IScriptEngine, ScriptValue};

/// Fetch a variable's current value, or `None` if it cannot be read.
fn current(se: &dyn IScriptEngine, sid: Option<&str>, var_id: &str) -> Option<ScriptValue> {
    se.get_variable(sid?, var_id).ok()
}

/// Read an integer-declared datamodel variable.
///
/// A whole-valued `Double` is accepted as well as an `Int`, and that
/// leniency is about engines rather than about types: Lua 5.2-family
/// bindings have no integer subtype at all, so the same authored `40`
/// crosses back as `Int(40)` from one engine and `Double(40.0)` from
/// another. Rejecting the second would make the accessor's answer depend on
/// which engine the deployment injected, which is exactly what a typed
/// accessor is supposed to hide. A fractional value is a different number
/// and is refused.
pub fn read_int(se: &dyn IScriptEngine, sid: Option<&str>, var_id: &str) -> Option<i64> {
    // §scxml-5.3: the value a `<data>` declaration populated into the session,
    // read back out in the host's own type. Reading, not declaring — the
    // clause's own verb belongs to `datamodel_init`.
    match current(se, sid, var_id)? {
        ScriptValue::Int(i) => Some(i),
        ScriptValue::Double(d)
            if d.fract() == 0.0 && d >= i64::MIN as f64 && d <= i64::MAX as f64 =>
        {
            Some(d as i64)
        }
        _ => None,
    }
}

/// Read a string-declared datamodel variable.
///
/// Strict: a number that happens to print as text is not a string, and
/// coercing it would let a consumer read a value the datamodel never held.
pub fn read_string(se: &dyn IScriptEngine, sid: Option<&str>, var_id: &str) -> Option<String> {
    // §scxml-5.3: the value a `<data>` declaration populated into the session,
    // read back out in the host's own type.
    match current(se, sid, var_id)? {
        ScriptValue::String(s) => Some(s),
        _ => None,
    }
}

/// Read a boolean-declared datamodel variable.
///
/// Strict, and deliberately not [`ScriptValue::to_bool`]: that function
/// answers SCXML's truthiness question, which every value has an answer to.
/// This one answers whether the variable is holding a boolean, and a
/// consumer inspecting a declared flag wants to be told when it is not.
pub fn read_bool(se: &dyn IScriptEngine, sid: Option<&str>, var_id: &str) -> Option<bool> {
    // §scxml-5.3: the value a `<data>` declaration populated into the session,
    // read back out in the host's own type.
    match current(se, sid, var_id)? {
        ScriptValue::Bool(b) => Some(b),
        _ => None,
    }
}

/// Read an array- or object-declared datamodel variable, as JSON text.
///
/// ## Why the engine serializes it rather than this function
///
/// Every engine SCE can be given carries `JSON.stringify` — the clause cited
/// in the body is what requires it — and that one serializer is the answer
/// here. Walking the [`ScriptValue`] tree instead
/// would be a second serializer disagreeing with the first, and it
/// would not even agree with itself: this runtime's object arrives as a
/// `HashMap`, so two reads of an unchanged variable could hand a consumer
/// the keys in different orders. What the engine produces is stable for that
/// engine — the Lua family's shared builtin sorts object keys, an ECMAScript
/// engine emits property order — and a stable answer is what a consumer
/// diffing two reads needs. It is the engine's encoding, not a normal form
/// across engines, which is the same shape of promise the integer reader
/// makes about numeric width.
///
/// It also keeps the six backends to one implementation each. C11 has no
/// allocator to build a JSON tree in, and there is no shared JSON writer for
/// six languages to agree through — but every one of them can ask the engine
/// it already holds.
///
/// ## Why this expression survives either engine family
///
/// [`IScriptEngine::evaluate_expression`] takes the ENGINE's language, not
/// the document's: a Lua-backed session is handed Lua, which is why generated
/// initializers carry Lua table syntax. `JSON.stringify(x)` is spelled the
/// same in both families — member access and a call, in a language the
/// datamodel clause requires that exact name to exist in. It is the one form
/// this helper can emit without knowing which engine was injected. The Kotlin
/// lane holds every backend to that: `DatamodelReadJsonTest` puts this same
/// expression to all three engines SCE can be given (Rhino, QuickJS, Lua),
/// which is more engines than any single runtime can reach.
///
/// ## Why the answer is strict
///
/// The scalar readers refuse a value of another type, and this one refuses
/// too: a variable declared `[…]` and later assigned `5` answers `None`, not
/// `"5"`. The test is the first character of the serializer's output, where
/// JSON's grammar puts the type — `[` opens an array and `{` an object, and
/// nothing else stringifies to either. Reading the *result* this way is not
/// the mistake this axis made with quotes one layer up: there, a character
/// test stood in for parsing the author's source text; here the text is a
/// canonical encoding whose first byte is defined to be the type tag.
pub fn read_json(se: &dyn IScriptEngine, sid: Option<&str>, var_id: &str) -> Option<String> {
    // §scxml-5.3: the value a `<data>` declaration populated into the session,
    // handed over in the encoding §scxml-B-2 already requires the engine to
    // produce. `var_id` reaches here only for a name the classifier confirmed
    // is a bare identifier — see `analyzer::reachable_as_an_expression`.
    let json = match se
        .evaluate_expression(sid?, &format!("JSON.stringify({var_id})"))
        .ok()?
    {
        ScriptValue::String(s) => s,
        _ => return None,
    };
    (json.starts_with('[') || json.starts_with('{')).then_some(json)
}
