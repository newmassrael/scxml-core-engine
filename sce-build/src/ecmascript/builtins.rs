// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The standard-library vocabulary of SCE's ECMAScript datamodel.
//!
//! [`super::lua`] dispatches a method call by name against an allowlist:
//! `substring` becomes `_scxml_substring`, `indexOf` becomes `_indexOf`,
//! and so on. What the allowlist did not decide was the *other* answer.
//! A name it did not know fell through to an ordinary field call —
//! `words.map(...)` was emitted as Lua `words.map(...)` — so a document
//! reaching for a method this datamodel does not implement generated
//! cleanly, on every backend, and died at runtime with a Lua message
//! about indexing a nil value. `check` answered `status: "ok"`.
//!
//! An allowlist whose fallthrough is *accept* is not an allowlist. This
//! module supplies the missing half: the names that are known to be
//! standard and known not to be implemented, so the frontend can refuse
//! them and [`crate::ecmascript_acceptance`] can report the refusal
//! against the element that wrote it.
//!
//! # Why a name, and not a type
//!
//! Nothing here is decided from the receiver's type, because there is no
//! type to decide from: the datamodel is untyped and the value in `words`
//! is known only at runtime. The name is what carries the author's
//! intent. Someone who writes `.map(` in a `datamodel="ecmascript"`
//! document is asking for ECMA-262's `Array.prototype.map`, and the
//! honest answer is that SCE has none — not a field lookup that yields
//! `nil`.
//!
//! # The two rules
//!
//! **A closed namespace is closed.** `Math`, `JSON` and `Object` are
//! tables *this repository* installs, in `ecma_semantics.lua` and
//! `json_builtins.lua`. Their member sets are therefore facts rather
//! than guesses, and a member outside one is refused with no further
//! reasoning needed — `JSON.serialize` cannot be anything but a mistake.
//!
//! **A standard prototype method SCE does not lower is refused.**
//! [`UNIMPLEMENTED_METHODS`] is ECMA-262's prototype vocabulary minus
//! [`LOWERED_METHODS`]. A name in neither list is still emitted as a
//! field call, because that is what an author's own object needs:
//! `<data id="handlers" expr="{ retry: function() {...} }"/>` followed by
//! `handlers.retry()` is legal in this datamodel and must stay so.
//!
//! **A standard global SCE does not install is refused.**
//! [`UNINSTALLED_GLOBALS`] is the third rule, and it is the one that
//! needs more than an expression: `Date` could name an author's own
//! `<data id="Date">`, so the name alone does not settle it. The
//! document does. [`super::scope::DocumentScope`] carries the
//! declarations and [`super::resolve`] asks this module only about the
//! names left over — which is why `Date.now()` is answered here and
//! decided there.

/// Members of the `JSON` table, from `sce/include/scripting/
/// json_builtins.lua`, which every engine loads.
pub const JSON_MEMBERS: &[&str] = &["parse", "stringify"];

/// Members of the `Object` table, from `sce/include/scripting/
/// ecma_semantics.lua`. ECMA-262 15.2.3.14 `Object.keys` is the only one:
/// it is the single way this datamodel can walk an object whose shape
/// arrived with an event payload.
pub const OBJECT_MEMBERS: &[&str] = &["keys"];

/// Members of `Math` the emitter lowers — ECMA-262 15.8.2.
///
/// `pow` becomes Lua's `^` and `round` becomes `_scxml_round`; the rest
/// are `math.<same name>`, which is why membership is decided here and
/// the emitter only spells out the two exceptions.
pub const MATH_MEMBERS: &[&str] = &[
    "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "max", "min", "pow",
    "random", "round", "sin", "sqrt", "tan",
];

/// The method names [`super::lua`] lowers on an arbitrary receiver.
///
/// Each maps to a helper in `ecma_semantics.lua` or to a Lua construct.
/// `ecmascript_builtin_vocabulary::every_lowered_method_has_an_emitter`
/// pins that this list and the emitter's match arms are one set, so a
/// name cannot be promised here and dropped there.
pub const LOWERED_METHODS: &[&str] = &[
    "charAt",
    "concat",
    "indexOf",
    "join",
    "push",
    "replace",
    "reverse",
    "slice",
    "sort",
    "split",
    "substring",
    "toLowerCase",
    "toString",
    "toUpperCase",
];

/// Methods SCE's XML DOM objects carry (`LuaDOMBinding`), lowered with
/// Lua's `:` call syntax because they are the only values in this
/// datamodel whose methods bind a receiver.
pub const DOM_METHODS: &[&str] = &["getElementsByTagName", "getAttribute", "getTagName"];

/// ECMA-262's standard prototype methods that this datamodel does not
/// implement.
///
/// Every entry is a method some standard prototype defines, so every
/// entry is a name whose meaning an author is entitled to expect and SCE
/// cannot supply. Grouped by the prototype that owns it; a name owned by
/// two (`lastIndexOf` is `String.prototype` 15.5.4.8 and
/// `Array.prototype` 15.4.4.15) is listed once.
///
/// The set is deliberately wider than the ES3 edition the rest of this
/// datamodel targets. A document written against a later edition is
/// exactly the case that needs the diagnostic most: `startsWith` and
/// `includes` are what a modern author reaches for first, and answering
/// them with `nil` teaches nothing.
///
/// Disjointness from [`LOWERED_METHODS`] is asserted rather than
/// eyeballed — adding `trim` to the emitter without removing it here
/// would make the frontend refuse a construct it can lower.
pub const UNIMPLEMENTED_METHODS: &[&str] = &[
    // Object.prototype — ECMA-262 15.2.4
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "valueOf",
    // Function.prototype — 15.3.4
    "apply",
    "bind",
    "call",
    // Array.prototype — 15.4.4 and ES5 15.4.4.14-22
    "every",
    "filter",
    "forEach",
    "lastIndexOf",
    "map",
    "pop",
    "reduce",
    "reduceRight",
    "shift",
    "some",
    "splice",
    "unshift",
    // String.prototype — 15.5.4, plus Annex B `substr`
    "charCodeAt",
    "localeCompare",
    "match",
    "search",
    "substr",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "trim",
    // Number.prototype — 15.7.4
    "toExponential",
    "toFixed",
    "toPrecision",
    // Date.prototype — 15.9.5
    "getDate",
    "getDay",
    "getFullYear",
    "getHours",
    "getMilliseconds",
    "getMinutes",
    "getMonth",
    "getSeconds",
    "getTime",
    "getTimezoneOffset",
    "getUTCDate",
    "getUTCDay",
    "getUTCFullYear",
    "getUTCHours",
    "getUTCMilliseconds",
    "getUTCMinutes",
    "getUTCMonth",
    "getUTCSeconds",
    "setDate",
    "setFullYear",
    "setHours",
    "setMilliseconds",
    "setMinutes",
    "setMonth",
    "setSeconds",
    "setTime",
    "toDateString",
    "toISOString",
    "toJSON",
    "toLocaleDateString",
    "toLocaleTimeString",
    "toTimeString",
    "toUTCString",
    // RegExp.prototype — 15.10.6
    "exec",
    "test",
    // ES2015 and later, on String.prototype and Array.prototype
    "at",
    "codePointAt",
    "copyWithin",
    "endsWith",
    "entries",
    "fill",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "includes",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "startsWith",
    "trimEnd",
    "trimStart",
];

/// Every name this datamodel binds in global position.
///
/// Three groups, and the reason each is here is different:
///
/// * `Math` — not a Lua table at all. [`super::lua`] rewrites
///   `Math.<member>` to Lua's `math`, so the identifier never reaches
///   the engine, but it is a name the author may legally write.
/// * `JSON`, `Object`, `String`, `Number`, `Boolean`, `parseInt`,
///   `parseFloat` — installed by `sce/include/scripting/*.lua`, which
///   every engine loads. `String`/`Number`/`Boolean` are ECMA-262
///   15.5.1 / 15.6.1 / 15.7.1 conversion functions, not constructors:
///   this datamodel has no `new`.
/// * `In` and the system variables the specification reserves —
///   `_event`, `_sessionid`, `_name`, `_ioprocessors`, `_x` — bound per
///   session by each engine's session setup rather than by the shared
///   library.
///
/// A name outside this list and outside the document is what
/// [`super::resolve`] refuses. Keeping the list here rather than in the
/// resolver is what lets `ecmascript_identifier_scope` assert each entry
/// against the library that installs it.
pub const INSTALLED_GLOBALS: &[&str] = &[
    "Boolean",
    "In",
    "JSON",
    "Math",
    "Number",
    "Object",
    "String",
    "_event",
    "_ioprocessors",
    "_name",
    "_sessionid",
    "_x",
    "parseFloat",
    "parseInt",
];

/// ECMA-262 globals this datamodel does not install.
///
/// Same reasoning as [`UNIMPLEMENTED_METHODS`], one AST node up: every
/// entry is a name the language defines, so every entry is a name an
/// author is entitled to expect and SCE cannot supply. Answering
/// `Date.now()` with a nil index at runtime teaches nothing; naming it
/// here lets the frontend say which globals do exist.
///
/// Wider than ES3 for the same reason the method list is: `Promise`,
/// `console` and `setTimeout` are what a modern author reaches for
/// first, and they are exactly the reaches a design-time answer is worth
/// most on.
///
/// `Array` is on the list even though [`super::lua`] reads it in
/// `x instanceof Array` — that arm consumes the name as part of the
/// operator rather than as a global, and [`super::resolve`] skips it
/// there for that reason. Every other position is a real reach for a
/// constructor this datamodel has no `new` for.
pub const UNINSTALLED_GLOBALS: &[&str] = &[
    // ECMA-262 15.1 — the global object's own properties and functions
    "Infinity",
    "NaN",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "eval",
    "isFinite",
    "isNaN",
    "unescape",
    // ECMA-262 15.3-15.11 — constructors
    "Array",
    "Date",
    "Error",
    "EvalError",
    "Function",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "SyntaxError",
    "TypeError",
    "URIError",
    // ES2015 and later
    "BigInt",
    "Map",
    "Promise",
    "Proxy",
    "Reflect",
    "Set",
    "Symbol",
    "WeakMap",
    "WeakSet",
    "globalThis",
    // Host globals of the two environments an author is likely writing
    // from — a browser or Node. Neither is what an SCXML datamodel is.
    "clearInterval",
    "clearTimeout",
    "console",
    "document",
    "fetch",
    "module",
    "process",
    "require",
    "setInterval",
    "setTimeout",
    "window",
];

/// The refusal for a free identifier, or `None` when the name is not one
/// ECMA-262 defines — in which case it is the author's own and
/// [`super::resolve`] answers it from the document rather than from here.
pub fn unsupported_global(name: &str) -> Option<crate::forge::error::ExprError> {
    if !UNINSTALLED_GLOBALS.contains(&name) {
        return None;
    }
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: name.to_string(),
        // The system variables are filtered out: `_event` is not a
        // substitute for `Date`, it is a different kind of name, and a
        // candidate list a consumer cannot choose from is noise.
        available: INSTALLED_GLOBALS
            .iter()
            .filter(|installed| !installed.starts_with('_'))
            .map(|installed| (*installed).to_string())
            .collect(),
    })
}

/// A namespace SCE installs, for [`unsupported_member`] to name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Math,
    Json,
    Object,
}

impl Namespace {
    /// The identifier an author writes.
    pub fn name(self) -> &'static str {
        match self {
            Namespace::Math => "Math",
            Namespace::Json => "JSON",
            Namespace::Object => "Object",
        }
    }

    /// The namespace an identifier names, or `None` if SCE installs no
    /// such table.
    pub fn from_ident(ident: &str) -> Option<Self> {
        match ident {
            "Math" => Some(Namespace::Math),
            "JSON" => Some(Namespace::Json),
            "Object" => Some(Namespace::Object),
            _ => None,
        }
    }

    pub fn members(self) -> &'static [&'static str] {
        match self {
            Namespace::Math => MATH_MEMBERS,
            Namespace::Json => JSON_MEMBERS,
            Namespace::Object => OBJECT_MEMBERS,
        }
    }
}

/// The refusal for `<namespace>.<member>` when the namespace does not
/// carry that member, or `None` when it does.
pub fn unsupported_member(
    namespace: Namespace,
    member: &str,
) -> Option<crate::forge::error::ExprError> {
    if namespace.members().contains(&member) {
        return None;
    }
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: format!("{}.{member}", namespace.name()),
        available: namespace
            .members()
            .iter()
            .map(|m| format!("{}.{m}", namespace.name()))
            .collect(),
    })
}

/// The refusal for a bare method name, or `None` when the emitter can
/// lower it or the name is the author's own.
pub fn unsupported_method(method: &str) -> Option<crate::forge::error::ExprError> {
    if !UNIMPLEMENTED_METHODS.contains(&method) {
        return None;
    }
    let mut available: Vec<String> = LOWERED_METHODS.iter().map(|m| format!(".{m}()")).collect();
    available.sort();
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: format!(".{method}()"),
        available,
    })
}
