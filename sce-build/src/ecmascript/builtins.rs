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
//! # The four rules
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
//! **A name that holds a value cannot be called.** The rules around this
//! one ask what a name *is*; this one asks what was *done* with a name
//! that exists. `t.length()`, `Math.PI()`, `_sessionid()` and
//! `_event.name()` each reach for something this datamodel provides and
//! then call it, which ECMA-262 11.2.3 answers with a TypeError and Lua
//! answers by indexing a nil. [`VALUE_PROPERTIES`], [`MATH_CONSTANTS`],
//! [`VALUE_GLOBALS`] and [`EVENT_FIELDS`] are those names, read in call
//! position, and the repair is the only deterministic one in this
//! module: drop the call.
//!
//! Each of those four lists is closed on its own terms, and
//! [`EVENT_FIELDS`] is the one whose closure is narrower than it looks:
//! it says which fields a *call* is refused on, not which fields an
//! event has. See its own note for why an event carrying more than the
//! specification names is a conformance fixture rather than a mistake.
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

/// Members of `Math` the emitter lowers as function calls —
/// ECMA-262 15.8.2.
///
/// `pow` becomes Lua's `^` and `round` becomes `_scxml_round`; the rest
/// are `math.<same name>`, which is why membership is decided here and
/// the emitter only spells out the two exceptions.
pub const MATH_FUNCTIONS: &[&str] = &[
    "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "max", "min", "pow",
    "random", "round", "sin", "sqrt", "tan",
];

/// Members of `Math` that hold a number — ECMA-262 15.8.1, all eight of
/// them.
///
/// They are members of the same namespace as [`MATH_FUNCTIONS`] and a
/// different kind of thing, which is the whole reason this list exists
/// separately. Before it, the namespace had one member list holding only
/// the functions while [`super::lua`] lowered four of the constants from
/// arms of its own: `Math.PI` evaluated correctly and `Math.PI()` was
/// answered *"Math.PI is not provided by SCE's ECMAScript datamodel"*,
/// which is false, and `Math.SQRT2` — a constant the emitter had no arm
/// for — generated cleanly and raised at runtime.
///
/// Listing all eight closes 15.8.1 rather than covering the half a
/// previous reader happened to need, and splitting them from the
/// functions is what lets a call on one be answered as the property call
/// it is.
pub const MATH_CONSTANTS: &[&str] = &[
    "E", "LN10", "LN2", "LOG10E", "LOG2E", "PI", "SQRT1_2", "SQRT2",
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

/// Methods SCE's XML DOM objects carry (`LuaDOMBinding` and its six
/// mirrors), lowered with Lua's `:` call syntax because they are the only
/// values in this datamodel whose methods bind a receiver.
///
/// The ECMAScript data model appendix obliges the Processor to *"create
/// the corresponding DOM structure"* for XML — once for a `<data>`
/// element's content and once for an arriving event payload — so the
/// vocabulary a handle carries is DOM Level 1 Core's, not a set chosen
/// from what the W3C IRP suite happens to read. Measured 2026-08-18, that suite reads exactly
/// two of these names — which is why three of them were all any backend
/// implemented while `d.tagName`, `d.childNodes` and `d.firstChild`
/// reached the engine as a nil index with `status: "ok"` at generation
/// time.
///
/// `getTagName` is SCE's own — DOM Level 1 spells the same thing as the
/// `tagName` *property* — and stays because committed trees call it.
/// The property is what an author writes, and the read surface a handle
/// carries as properties (`nodeName`, `nodeType`, `nodeValue`,
/// `parentNode`, `childNodes`, `firstChild`, `lastChild`,
/// `nextSibling`, `previousSibling`, `textContent`, `tagName`, `data`,
/// `documentElement`, `length`) needs no arm here: a member read is
/// already emitted as a field read, so what those names needed was a
/// backend that answers them. `docs/SCE_ACCEPTED_SUBSET.md` §1 states
/// the whole surface once; this list is only the callable half of it.
pub const DOM_METHODS: &[&str] = &[
    "getAttribute",
    "getElementsByTagName",
    "getTagName",
    "hasAttribute",
    "hasChildNodes",
];

/// DOM interface methods SCE's handles do not carry.
///
/// [`UNIMPLEMENTED_METHODS`]' reasoning, one specification over: every
/// entry is a method a DOM interface defines, so every entry is a name
/// an author reading the appendix's *"corresponding DOM structure"* is
/// entitled to expect, and answering it with a nil index teaches
/// nothing. The candidates the refusal carries are [`DOM_METHODS`]
/// rather than [`LOWERED_METHODS`], because a document that wrote
/// `d.setAttribute(...)` is holding a DOM handle and the string
/// vocabulary is no help to it.
///
/// Every entry is also a *write*, a *creation*, or a namespace-aware
/// read — the three things this surface deliberately does not do. A DOM
/// built from `<data>` content or from an arriving `_event.data` is the
/// document that was parsed; the appendix's location expressions are how
/// this datamodel changes values, and a mutation applied to a handle
/// would change a tree nothing reads back. So the list is closed by that
/// reasoning rather than by enumeration of a level: a *read* this
/// surface lacks is a gap to fill in the seven bindings, and a write is
/// a refusal.
pub const DOM_UNIMPLEMENTED_METHODS: &[&str] = &[
    // Node — DOM Level 1 Core 1.2
    "appendChild",
    "cloneNode",
    "insertBefore",
    "removeChild",
    "replaceChild",
    // Element / Attr — 1.2
    "getAttributeNode",
    "removeAttribute",
    "removeAttributeNode",
    "setAttribute",
    "setAttributeNode",
    // Document — 1.2, the factory half
    "createAttribute",
    "createCDATASection",
    "createComment",
    "createDocumentFragment",
    "createElement",
    "createProcessingInstruction",
    "createTextNode",
    "getElementById",
    "importNode",
    // CharacterData / Text — 1.2
    "appendData",
    "deleteData",
    "insertData",
    "replaceData",
    "splitText",
    "substringData",
    // NodeList / NamedNodeMap — 1.2. `item` is here rather than
    // implemented because a NodeList is the host language's own array in
    // every one of the seven bindings — a Lua table in five, a JS array
    // in two — so `[i]` reads it and `length` measures it, and there is
    // nothing for a method to bind a receiver to.
    "getNamedItem",
    "item",
    "removeNamedItem",
    "setNamedItem",
    // DOM Level 2 Core — the namespace-aware and feature-query surface
    "getAttributeNS",
    "getAttributeNodeNS",
    "getElementsByTagNameNS",
    "hasAttributeNS",
    "hasAttributes",
    "isSupported",
    "removeAttributeNS",
    "setAttributeNS",
    "setAttributeNodeNS",
];

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

/// The names in [`INSTALLED_GLOBALS`] that hold a value rather than
/// something a call can name.
///
/// The system variables the specification reserves, and only those:
/// `Math`, `JSON` and `Object` are namespaces whose members are reached
/// through a member expression, and `String`, `Number`, `Boolean`,
/// `parseInt`, `parseFloat` and `In` are functions. A session binds each
/// name here to a string or a table, so `_sessionid()` calls something
/// that was never a function.
///
/// `ecmascript_property_calls::every_value_global_is_installed` pins the
/// containment, so a name cannot be declared uncallable here without
/// this datamodel binding it at all.
pub const VALUE_GLOBALS: &[&str] = &["_event", "_ioprocessors", "_name", "_sessionid", "_x"];

/// The standard data properties this datamodel provides on a value.
///
/// `length` is the whole list, and the list is closed rather than
/// partial: ECMA-262 gives a data property to a string (15.5.5.1) and to
/// an array (15.4.5.2) under that one name, and [`super::lua`] lowers
/// both to Lua's `#` for every receiver. A property outside this list is
/// an author's own field, which is why the name is what decides and no
/// type is consulted — the same reasoning [`UNIMPLEMENTED_METHODS`]
/// rests on, applied to the other half of the member grammar.
pub const VALUE_PROPERTIES: &[&str] = &["length"];

/// The fields the specification's internal structure of events obliges
/// every event to carry, internal and external alike.
///
/// Every one of them holds a value. Six are typed by the clause itself —
/// `name` and `type` are character strings, `origin` is a URI,
/// `origintype` is what a `<send>` writes in `type`, `sendid` and
/// `invokeid` are ids the Processor sets or leaves blank — and `data` is
/// "whatever data the sending entity chose to include", which reaches
/// the datamodel as a `ScriptValue`: null, boolean, number, string,
/// array, object or DOM handle, a union with no callable member. So
/// `_event.name()` is the TypeError of ECMA-262 11.2.3 however the event
/// was raised, and dropping the call is the whole repair.
///
/// # A floor, not a ceiling
///
/// The clause says the Processor MUST ensure these fields are *present*.
/// It does not say an event carries nothing else, and W3C test178 reads
/// `_event.raw` — a field the specification never mentions, which an
/// Event I/O Processor supplies and which this repository generates and
/// runs as a registered conformance fixture. Membership here therefore
/// decides only what a *call* is refused on: a member outside this list
/// stays an ordinary field, read and called exactly as an author's own
/// object is. Closing the namespace the way [`Namespace`] closes `Math`
/// would reject that fixture.
pub const EVENT_FIELDS: &[&str] = &[
    "data",
    "invokeid",
    "name",
    "origin",
    "origintype",
    "sendid",
    "type",
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
    /// Every namespace this datamodel installs.
    ///
    /// A list rather than a derive so the rules that must hold for all
    /// three — a call on the namespace itself is refused, a call on a
    /// member is not — can be asserted over the whole set. A fourth
    /// namespace added to [`Namespace::from_ident`] and forgotten here
    /// reds `every_namespace_this_datamodel_installs_is_reachable_by_ident`.
    pub const ALL: &'static [Namespace] = &[Namespace::Math, Namespace::Json, Namespace::Object];

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

    /// The members a call may name — the ones that hold a function.
    pub fn callable_members(self) -> &'static [&'static str] {
        match self {
            Namespace::Math => MATH_FUNCTIONS,
            Namespace::Json => JSON_MEMBERS,
            Namespace::Object => OBJECT_MEMBERS,
        }
    }

    /// The members that hold a value rather than a function.
    ///
    /// `Math` is the only namespace with any: `JSON` and `Object` are
    /// tables of functions, so a call is the only thing an author can do
    /// with a member of either.
    pub fn value_members(self) -> &'static [&'static str] {
        match self {
            Namespace::Math => MATH_CONSTANTS,
            Namespace::Json | Namespace::Object => &[],
        }
    }
}

/// The refusal for a call on `<namespace>.<member>`, or `None` when the
/// member is one this datamodel lets a call name.
///
/// Two refusals, because two different things are wrong. A member the
/// namespace does not carry is a name this datamodel lacks, and the
/// vocabulary it does carry is the answer. A member it carries as a
/// *value* is a name this datamodel has — calling it is what the author
/// got wrong, and dropping the call is the whole repair.
pub fn unsupported_member(
    namespace: Namespace,
    member: &str,
    arguments: usize,
) -> Option<crate::forge::error::ExprError> {
    if namespace.callable_members().contains(&member) {
        return None;
    }
    if namespace.value_members().contains(&member) {
        return Some(crate::forge::error::ExprError::PropertyNotCallable {
            name: format!("{}.{member}", namespace.name()),
            arguments,
        });
    }
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: format!("{}.{member}", namespace.name()),
        // The candidates are the callable members alone: this record
        // stands where a call was written, and offering `Math.PI` as a
        // replacement for `Math.tanh` would repair one refusal into
        // another.
        available: namespace
            .callable_members()
            .iter()
            .map(|m| format!("{}.{m}", namespace.name()))
            .collect(),
    })
}

/// The refusal for a read of `<namespace>.<member>` the namespace does
/// not carry, or `None` when it does.
///
/// The counterpart of [`unsupported_member`] one AST node down. A read
/// can name either kind of member — `Math.PI` is a number and `Math.abs`
/// is a function this datamodel has no way to hand out as a value — so
/// the membership question and the "can it be read" question are asked
/// separately, and only the first is answered here.
pub fn unknown_member(
    namespace: Namespace,
    member: &str,
) -> Option<crate::forge::error::ExprError> {
    if namespace.callable_members().contains(&member) || namespace.value_members().contains(&member)
    {
        return None;
    }
    let mut available: Vec<String> = namespace
        .callable_members()
        .iter()
        .chain(namespace.value_members())
        .map(|m| format!("{}.{m}", namespace.name()))
        .collect();
    available.sort();
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: format!("{}.{member}", namespace.name()),
        available,
    })
}

/// The refusal for a call on a bare identifier, or `None` when the name
/// is one a call may legally reach.
///
/// Only [`VALUE_GLOBALS`] is consulted. An identifier this datamodel
/// does not install has already been answered by [`super::resolve`] —
/// either as the author's own declaration, which may well hold a
/// function, or as a name nothing declares.
pub fn uncallable_global(name: &str, arguments: usize) -> Option<crate::forge::error::ExprError> {
    if !VALUE_GLOBALS.contains(&name) {
        return None;
    }
    Some(crate::forge::error::ExprError::PropertyNotCallable {
        name: name.to_string(),
        arguments,
    })
}

/// The refusal for a call on a namespace itself, or `None` when the
/// identifier names no namespace this datamodel installs.
///
/// The other half of the rule [`unsupported_member`] keeps. That one
/// closes the member set of `Math`, `JSON` and `Object`; this one closes
/// the position the namespace name may stand in at all. Both halves are
/// needed for the same reason: the member set is a fact this repository
/// owns, and so is the fact that none of the three is a function.
///
/// `Math()` used to reach Lua verbatim, where `Math` is not bound at
/// all — the emitter rewrites `Math.<member>` to Lua's `math`, so the
/// capitalised name exists nowhere — and the machine died calling a
/// `nil`. `JSON()` and `Object()` reached tables that no engine makes
/// callable, with the same result and the same silence.
pub fn uncallable_namespace(name: &str) -> Option<crate::forge::error::ExprError> {
    let namespace = Namespace::from_ident(name)?;
    Some(crate::forge::error::ExprError::NamespaceNotCallable {
        namespace: namespace.name().to_string(),
        // The callable members alone, for the reason `unsupported_member`
        // gives: this record stands where a call was written, and
        // `Math.PI` cannot stand there.
        members: namespace
            .callable_members()
            .iter()
            .map(|member| format!("{}.{member}", namespace.name()))
            .collect(),
    })
}

/// The value fields a system variable carries, or an empty slice when
/// the specification names none.
///
/// `_event` is the only system variable whose structure W3C SCXML
/// enumerates. `_sessionid` and `_name` are strings with no fields at
/// all, `_ioprocessors` is keyed by the Event I/O Processors a platform
/// happens to support (5.10, C), and `_x` is by definition the
/// platform's own root — none of the three has a field list a producer
/// could state as a fact. A lookup rather than one flat list is what
/// keeps that asymmetry visible, and what makes a second variable one
/// arm rather than a second rule.
pub fn system_value_fields(variable: &str) -> &'static [&'static str] {
    match variable {
        "_event" => EVENT_FIELDS,
        _ => &[],
    }
}

/// The refusal for a call on a system variable's field, or `None` when
/// the specification does not name that field as one of its own.
///
/// The receiver is consulted here, unlike in [`uncallable_property`]:
/// `name` and `data` are ordinary words, and an author's
/// `handlers.data()` must survive. Only the variable the specification
/// reserves carries the rule.
pub fn uncallable_system_field(
    variable: &str,
    field: &str,
    arguments: usize,
) -> Option<crate::forge::error::ExprError> {
    if !system_value_fields(variable).contains(&field) {
        return None;
    }
    Some(crate::forge::error::ExprError::PropertyNotCallable {
        name: format!("{variable}.{field}"),
        arguments,
    })
}

/// The refusal for a call on a standard data property, or `None` when
/// the property is not one this datamodel provides.
pub fn uncallable_property(
    property: &str,
    arguments: usize,
) -> Option<crate::forge::error::ExprError> {
    if !VALUE_PROPERTIES.contains(&property) {
        return None;
    }
    Some(crate::forge::error::ExprError::PropertyNotCallable {
        name: format!(".{property}"),
        arguments,
    })
}

/// The refusal for a bare method name, or `None` when the emitter can
/// lower it or the name is the author's own.
///
/// Two vocabularies answer here, and which one matched decides the
/// candidates. A document that wrote `.map()` is holding a value and
/// wants to know what this datamodel does to values; one that wrote
/// `.setAttribute()` is holding a DOM handle, and offering it
/// `.substring()` would repair one refusal into another.
pub fn unsupported_method(method: &str) -> Option<crate::forge::error::ExprError> {
    let vocabulary: &[&str] = if UNIMPLEMENTED_METHODS.contains(&method) {
        LOWERED_METHODS
    } else if DOM_UNIMPLEMENTED_METHODS.contains(&method) {
        DOM_METHODS
    } else {
        return None;
    };
    let mut available: Vec<String> = vocabulary.iter().map(|m| format!(".{m}()")).collect();
    available.sort();
    Some(crate::forge::error::ExprError::UnsupportedBuiltin {
        name: format!(".{method}()"),
        available,
    })
}
