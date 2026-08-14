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
// ARCHITECTURE.md: Zero Duplication — the same three coercions back the C++
// `DataModelReadHelper`, the Rust `helpers::datamodel_read` and the Kotlin
// `DatamodelRead` surface, so every backend's accessor answers alike.
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
