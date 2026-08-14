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
