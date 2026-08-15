// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// Typed reads of a live datamodel variable.
//
// The counterpart to the datamodel initialisation the generated
// `initializeScriptEngine` performs. These take a value back out in the
// host's own type, so a generated machine can answer a question about its own
// datamodel without the caller holding a script engine, a session id and the
// variable's name spelled as a string.
//
// ARCHITECTURE.md: Zero Duplication — the same four readers back the C++
// `DataModelReadHelper`, the Rust `helpers::datamodel_read`, the Kotlin
// `DatamodelRead` and the Python `datamodel_read` surface, and the C11
// template inlines the same rules, so every backend's accessor answers alike.
// Three hand a value back in a host type; the fourth hands a structured one
// over as JSON, because there is no host type six languages share for it.
//
// Why the read goes to the engine rather than to a copy: a `<data>` variable
// with an initialiser is owned by the script engine for the life of the
// session — `<assign>` writes there and guards read from there. Anything the
// generated struct kept alongside it would be a second representation of one
// variable, wrong from the first `<assign>` onwards.
//
// Why the second return value: the session may not be initialised yet, the
// variable may have been assigned a value of another type mid-run, or the
// engine may refuse. All three mean the same thing to a consumer — the
// machine cannot answer that right now.

// currentDatamodelValue fetches a variable's value, or reports that it cannot
// be read.
func currentDatamodelValue(engine IScriptEngine, sessionID, name string) (interface{}, bool) {
	if engine == nil || sessionID == "" {
		return nil, false
	}
	value, err := engine.GetVariable(sessionID, name)
	if err != nil {
		return nil, false
	}
	return value, true
}

// ReadDatamodelInt reads an integer-declared datamodel variable.
//
// Every whole-valued numeric width is accepted, and that leniency is about
// engines rather than about types: go-lua has no integer subtype at all, so
// the same authored 40 crosses back as a float64 here and as an int64 from the
// Lua 5.4 bindings the C++ and Rust runtimes use. Refusing the float would
// make the accessor's answer depend on which engine the deployment injected,
// which is exactly what a typed accessor exists to hide. A fractional value is
// a different number and is refused.
func ReadDatamodelInt(engine IScriptEngine, sessionID, name string) (int64, bool) {
	// §scxml-5.3: the value a <data> declaration populated into the session,
	// read back out in the host's own type. Reading, not declaring — the
	// clause's own verb belongs to the generated initialiser.
	value, ok := currentDatamodelValue(engine, sessionID, name)
	if !ok {
		return 0, false
	}
	switch v := value.(type) {
	case int64:
		return v, true
	case int:
		return int64(v), true
	case int32:
		return int64(v), true
	case float64:
		if v == float64(int64(v)) {
			return int64(v), true
		}
	case float32:
		if float64(v) == float64(int64(v)) {
			return int64(v), true
		}
	}
	return 0, false
}

// ReadDatamodelString reads a string-declared datamodel variable.
//
// Strict: a number that happens to print as text is not a string, and coercing
// it would let a consumer read a value the datamodel never held.
func ReadDatamodelString(engine IScriptEngine, sessionID, name string) (string, bool) {
	// §scxml-5.3: the value a <data> declaration populated into the session,
	// read back out in the host's own type.
	value, ok := currentDatamodelValue(engine, sessionID, name)
	if !ok {
		return "", false
	}
	if s, isString := value.(string); isString {
		return s, true
	}
	return "", false
}

// ReadDatamodelBool reads a boolean-declared datamodel variable.
//
// Strict, and deliberately not the SCXML truthiness rule: that rule answers a
// question every value has an answer to. This one answers whether the variable
// is holding a boolean, and a consumer inspecting a declared flag wants to be
// told when it is not.
func ReadDatamodelBool(engine IScriptEngine, sessionID, name string) (bool, bool) {
	// §scxml-5.3: the value a <data> declaration populated into the session,
	// read back out in the host's own type.
	value, ok := currentDatamodelValue(engine, sessionID, name)
	if !ok {
		return false, false
	}
	if b, isBool := value.(bool); isBool {
		return b, true
	}
	return false, false
}

// ReadDatamodelJSON reads an array- or object-declared datamodel variable, as
// JSON text.
//
// Why the engine serialises it rather than this function: every engine SCE
// can be given carries JSON.stringify — the clause cited in the body is what
// requires it — and that one serialiser is the answer. Walking whatever
// go-lua handed back would be a second serialiser
// disagreeing with the first, and it would not even agree with itself — a Lua
// table arrives as a map, whose range order Go deliberately randomises, so two
// reads of an unchanged variable would hand a consumer the keys in different
// orders. What the engine produces is stable for that engine (the shared Lua
// builtin sorts object keys; an ECMAScript engine emits property order), and
// stability is what a consumer diffing two reads needs. It is the engine's
// encoding, not a normal form across engines, which is the same shape of
// promise ReadDatamodelInt makes about numeric width.
//
// Why this expression survives either engine family: EvaluateExpression takes
// the ENGINE's language, not the document's — a Lua-backed session is handed
// Lua. `JSON.stringify(x)` is spelled the same in both, member access and a
// call, in a language the datamodel clause requires that exact name to exist
// in.
//
// Why the answer is strict: the scalar readers refuse a value of another type
// and so does this one. A variable declared [...] and later assigned 5 reports
// false, not "5". The test is the first character of the serialiser's output,
// where JSON's grammar puts the type — [ opens an array and { an object, and
// nothing else stringifies to either.
func ReadDatamodelJSON(engine IScriptEngine, sessionID, name string) (string, bool) {
	// §scxml-5.3: the value a <data> declaration populated into the session,
	// handed over in the encoding §scxml-B-2 already requires the engine to
	// produce. `name` reaches here only for a name the classifier confirmed is
	// a bare identifier — see `analyzer::reachable_as_an_expression`.
	if engine == nil || sessionID == "" {
		return "", false
	}
	value, err := engine.EvaluateExpression(sessionID, "JSON.stringify("+name+")")
	if err != nil {
		return "", false
	}
	json, isString := value.(string)
	if !isString || json == "" {
		return "", false
	}
	if json[0] != '[' && json[0] != '{' {
		return "", false
	}
	return json, true
}
