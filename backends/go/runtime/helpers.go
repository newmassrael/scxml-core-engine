// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"math"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"
)

// ── Delay parsing ──────────────────────────────────────────────────

// ParseDelay parses a CSS2-style delay string into a time.Duration.
//
// Accepts:
//   - "1s", "1.5s" -> seconds
//   - "250ms" -> milliseconds
//   - bare number "500" -> milliseconds
//
// Returns 0 for empty or unparseable input.
func ParseDelay(s string) time.Duration {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0
	}

	// Try "Xs" format (check before "ms" to avoid matching "ms" suffix)
	if strings.HasSuffix(s, "s") && !strings.HasSuffix(s, "ms") {
		numStr := strings.TrimSuffix(s, "s")
		if f, err := strconv.ParseFloat(numStr, 64); err == nil {
			return time.Duration(f * float64(time.Second))
		}
	}

	// Try "Xms" format
	if strings.HasSuffix(s, "ms") {
		numStr := strings.TrimSuffix(s, "ms")
		if f, err := strconv.ParseFloat(numStr, 64); err == nil {
			return time.Duration(f * float64(time.Millisecond))
		}
	}

	// Bare number = milliseconds
	if f, err := strconv.ParseFloat(s, 64); err == nil {
		return time.Duration(f * float64(time.Millisecond))
	}

	return 0
}

// ── Script engine helpers ──────────────────────────────────────────

// ScriptToBool converts a script engine result to a boolean, following SCXML
// truthiness rules (§scxml-B-2-3).
func ScriptToBool(value interface{}) bool {
	if value == nil {
		return false
	}
	switch v := value.(type) {
	case bool:
		return v
	case int64:
		return v != 0
	case float64:
		return v != 0
	case string:
		return v != ""
	case int:
		return v != 0
	default:
		return true
	}
}

// ToSlice attempts to convert a script value to a slice of interface{} values.
func ToSlice(value interface{}) ([]interface{}, bool) {
	if value == nil {
		return nil, false
	}
	if s, ok := value.([]interface{}); ok {
		return s, true
	}
	return nil, false
}

// ToWireString renders a value as the text a form-encoded §scxml-C-2 param
// carries.
//
// The BasicHTTP Event I/O Processor sends each <param> as one name=value pair,
// so the value crosses as *text* and the receiving end hands that text to
// _event.data — no script engine reads it at either end. That is why this is
// neither of the two serialisations beside it: [ScriptValueToJSON] would wrap a
// string in quotes that are not part of it, and an engine literal
// (IScriptEngine.ToScriptLiteral) would put the sender's *language* on the
// wire, so one value read `nil` from this backend and `` from the C++ one.
//
// The rendering is ECMAScript's String(value) — §scxml-B-1 makes the data model
// ECMAScript — with the two amendments C++ ScriptResultUtils::resultToString
// already made: absence renders empty rather than as a word (§scxml-C-1), and a
// structured value renders as JSON, because a receiver that is not a script
// engine has no other reading of it.
func ToWireString(value interface{}) string {
	if value == nil {
		// §scxml-C-1: a value that is not there is the empty string, on the
		// wire as in a target expression.
		return ""
	}
	switch v := value.(type) {
	case bool:
		if v {
			return "true"
		}
		return "false"
	case int:
		return fmt.Sprintf("%d", v)
	case int64:
		return fmt.Sprintf("%d", v)
	case float64:
		if math.IsNaN(v) {
			return "NaN"
		}
		if math.IsInf(v, 1) {
			return "Infinity"
		}
		if math.IsInf(v, -1) {
			return "-Infinity"
		}
		if v == float64(int64(v)) {
			// ECMAScript String(5) is "5"; a .0 tail is Go's spelling of the
			// number, not the document's.
			return fmt.Sprintf("%d", int64(v))
		}
		return fmt.Sprintf("%g", v)
	case string:
		// Already text. Quoting it here would deliver characters the document
		// never wrote, and the trim that used to undo such quotes ate the ones
		// the value itself carried.
		return v
	case []interface{}, map[string]interface{}:
		return ScriptValueToJSON(v)
	default:
		return fmt.Sprintf("%v", v)
	}
}

// ScriptValueToJSON serialises a value that leaves the ECMAScript data model.
//
// The clause cited in the body names JSON as that serialisation — it is what
// the BasicHTTP Event I/O Processor sends — and an event payload always leaves
// the data model: the reader is another dequeue, often another session, and in
// a mesh another process running another backend.
//
// This is the counterpart of IScriptEngine.ToScriptLiteral, and the difference
// is the point. An engine literal is *source*: reading it back needs an
// interpreter for the language the sender happened to be written in. That made
// `_event.data` mean one thing on a Lua backend and another on a JavaScript
// one, and made a payload executable at the receiving end. JSON is read by a
// parser.
//
// 1:1 port of the C++ `scriptValueToJson` static in
// `sce/src/common/EventDataHelper.cpp`. Object keys are sorted: Go map
// iteration order is deliberately randomised, and the wire form has to be
// byte-identical for equal content.
func ScriptValueToJSON(value interface{}) string {
	// §scxml-B-2-9: a value that has to leave the ECMAScript data model is
	// serialized to JSON, which reconstructs it in full rather than falling
	// back to a lossy platform format.
	if value == nil {
		// JSON has no `undefined`; the C++ port maps both to null.
		return "null"
	}
	switch v := value.(type) {
	case bool:
		if v {
			return "true"
		}
		return "false"
	case int:
		return fmt.Sprintf("%d", v)
	case int64:
		return fmt.Sprintf("%d", v)
	case float64:
		if math.IsNaN(v) || math.IsInf(v, 0) {
			// RFC 8259 has no spelling for either.
			return "null"
		}
		if v == float64(int64(v)) && math.Abs(v) < 1e15 {
			return fmt.Sprintf("%d", int64(v))
		}
		return fmt.Sprintf("%g", v)
	case string:
		return `"` + EscapeJSONString(v) + `"`
	case []interface{}:
		items := make([]string, len(v))
		for i, item := range v {
			items[i] = ScriptValueToJSON(item)
		}
		return "[" + strings.Join(items, ",") + "]"
	case map[string]interface{}:
		keys := make([]string, 0, len(v))
		for k := range v {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		items := make([]string, 0, len(keys))
		for _, k := range keys {
			items = append(items, `"`+EscapeJSONString(k)+`":`+ScriptValueToJSON(v[k]))
		}
		return "{" + strings.Join(items, ",") + "}"
	default:
		return `"` + EscapeJSONString(fmt.Sprintf("%v", v)) + `"`
	}
}

// EscapeJSONString escapes a string for a JSON string literal.
// Ports C++ `DoneDataHelper::escapeJsonString`.
func EscapeJSONString(s string) string {
	escaped := strings.ReplaceAll(s, `\`, `\\`)
	escaped = strings.ReplaceAll(escaped, `"`, `\"`)
	escaped = strings.ReplaceAll(escaped, "\n", `\n`)
	escaped = strings.ReplaceAll(escaped, "\r", `\r`)
	escaped = strings.ReplaceAll(escaped, "\t", `\t`)
	escaped = strings.ReplaceAll(escaped, "\b", `\b`)
	escaped = strings.ReplaceAll(escaped, "\f", `\f`)
	return escaped
}

// EventDataParam is one evaluated `<send>` param, in document order.
type EventDataParam struct {
	Name  string
	Value interface{}
}

// BuildJSONFromTypedParams builds the JSON `_event.data` a `<send>` ships.
//
// W3C test178: a name may repeat and every value must be delivered, so one
// occurrence is the value itself and more than one is an Array of them in
// document order — an object cannot hold one name twice.
// Names are sorted, matching the C++ `std::map` original and the Rust port,
// so the same params produce the same bytes on every backend.
//
// 1:1 port of C++ `EventDataHelper::buildJsonFromTypedParams`.
func BuildJSONFromTypedParams(params []EventDataParam) string {
	// §scxml-6.2: the `<param>` elements a `<send>` carries become the data
	// the receiving event exposes, evaluated at send time.
	grouped := make(map[string][]interface{})
	names := make([]string, 0, len(params))
	for _, p := range params {
		if _, seen := grouped[p.Name]; !seen {
			names = append(names, p.Name)
		}
		grouped[p.Name] = append(grouped[p.Name], p.Value)
	}
	sort.Strings(names)
	parts := make([]string, 0, len(names))
	for _, name := range names {
		values := grouped[name]
		published := ""
		if len(values) == 1 {
			published = ScriptValueToJSON(values[0])
		} else {
			items := make([]string, len(values))
			for i, v := range values {
				items[i] = ScriptValueToJSON(v)
			}
			published = "[" + strings.Join(items, ",") + "]"
		}
		parts = append(parts, `"`+EscapeJSONString(name)+`":`+published)
	}
	return "{" + strings.Join(parts, ",") + "}"
}

// LoadFileContent loads file content for datamodel src attributes (W3C SCXML 5.2.2).
// Supports "file:" prefix (strips it) and relative paths.
func LoadFileContent(path string) (string, error) {
	// W3C SCXML 5.2.2: Strip "file:" prefix
	filePath := path
	if strings.HasPrefix(filePath, "file:") {
		filePath = filePath[5:]
	}
	data, err := os.ReadFile(filePath)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

// QuoteLuaString wraps a string in Lua-safe quotes with escaping.
func QuoteLuaString(s string) string {
	escaped := strings.ReplaceAll(s, `\`, `\\`)
	escaped = strings.ReplaceAll(escaped, `"`, `\"`)
	escaped = strings.ReplaceAll(escaped, "\n", `\n`)
	escaped = strings.ReplaceAll(escaped, "\r", `\r`)
	return `"` + escaped + `"`
}

// InitializeVariableFromContent initializes a datamodel variable from loaded content.
// Matches Rust sce_rust_runtime::helpers::datamodel_init::initialize_variable.
// Tries: direct Lua eval → JSON.parse → whitespace-normalized string.
func InitializeVariableFromContent(se IScriptEngine, sessionID, varID, content string) {
	trimmed := strings.TrimSpace(content)
	if trimmed == "" {
		_ = se.SetVariable(sessionID, varID, nil)
		return
	}

	// W3C SCXML B.2: XML content -> DOM object with getElementsByTagName/getAttribute
	if strings.HasPrefix(trimmed, "<") {
		_ = se.SetVariableAsDOM(sessionID, varID, trimmed)
		return
	}

	// Try evaluating as Lua expression
	if result, err := se.EvaluateExpression(sessionID, trimmed); err == nil {
		_ = se.SetVariable(sessionID, varID, result)
		return
	}

	// W3C SCXML B.2 test 446: Try JSON.parse for JSON content
	if strings.HasPrefix(trimmed, "[") || strings.HasPrefix(trimmed, "{") {
		jsonExpr := varID + " = JSON.parse(" + QuoteLuaString(trimmed) + ")"
		if err := se.ExecuteScript(sessionID, jsonExpr); err == nil {
			return
		}
	}

	// W3C SCXML B.2 test 558: Fall back to whitespace-normalized string
	normalized := strings.Join(strings.Fields(trimmed), " ")
	_ = se.SetVariable(sessionID, varID, normalized)
}

// IsValidIdentifier checks if a name is a valid Lua/ECMAScript identifier
// (W3C SCXML 4.6: foreach item must be a valid location expression).
func IsValidIdentifier(name string) bool {
	if len(name) == 0 {
		return false
	}
	for i, c := range name {
		if i == 0 {
			if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_') {
				return false
			}
		} else {
			if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_') {
				return false
			}
		}
	}
	return true
}

// ── History helpers ────────────────────────────────────────────────

// FilterShallowHistory returns immediate children of parentState from active
// states (W3C SCXML 3.11 shallow history).
func FilterShallowHistory[S comparable](
	activeStates []S,
	parentState S,
	getParent func(S) (S, bool),
) []S {
	var result []S
	for _, s := range activeStates {
		if parent, ok := getParent(s); ok && parent == parentState {
			result = append(result, s)
		}
	}
	return result
}

// FilterDeepHistory returns all leaf descendants of parentState from active
// states (W3C SCXML 3.11 deep history).
func FilterDeepHistory[S comparable](
	activeStates []S,
	parentState S,
	getParent func(S) (S, bool),
	isCompound func(S) bool,
) []S {
	var result []S
	for _, s := range activeStates {
		if !isCompound(s) && isDescendantOfFunc(s, parentState, getParent) {
			result = append(result, s)
		}
	}
	return result
}

func isDescendantOfFunc[S comparable](
	desc, anc S,
	getParent func(S) (S, bool),
) bool {
	current := desc
	for i := 0; i < MaxHierarchyDepth; i++ {
		parent, ok := getParent(current)
		if !ok {
			return false
		}
		if parent == anc {
			return true
		}
		current = parent
	}
	return false
}

// ── Parent event ───────────────────────────────────────────────────

// ParentEvent represents an event sent from child to parent (W3C SCXML 6.4).
type ParentEvent struct {
	Name string
	Data string
}

// SetCurrentEventArgs is the parameter object for the W3C SCXML 5.10
// IScriptEngine.SetCurrentEvent boundary. Bundles the seven _event.*
// metadata fields (name + 6 metadata) that every script engine impl
// must surface before guard evaluation / action execution. Cross-language
// siblings: SCE::SetCurrentEventArgs (C++) and SetCurrentEventArgs
// (Rust / Kotlin / Python).
type SetCurrentEventArgs struct {
	Name       string
	Data       string
	Type       string
	SendID     string
	Origin     string
	OriginType string
	InvokeID   string
}

// ── Script engine global registry ──────────────────────────────────
//
// IScriptEngine is the interface for script engine implementations.
// This is the minimal interface that generated code and LuaEngine actually use.
// The full Rust IScriptEngine has more methods; we add them as needed.
type IScriptEngine interface {
	// Lifecycle
	Initialize() error
	Shutdown()
	CreateSession(sessionID string) error
	DestroySession(sessionID string)
	HasSession(sessionID string) bool

	// Core execution
	ExecuteScript(sessionID, script string) error
	EvaluateExpression(sessionID, expr string) (interface{}, error)

	// ToScriptLiteral renders a value as source this engine can evaluate back
	// — the inverse of EvaluateExpression.
	//
	// §scxml-6.4.1 has the parent evaluate an <invoke>'s <param> and namelist
	// expressions and hand the values to the child, and the child seeds them
	// by evaluating source. That round trip is the only reason a literal is
	// spelled at all, and it is why the spelling belongs to the engine: `nil`
	// versus `null`, `{1, 2}` versus `[1, 2]` are answers about the engine,
	// not about the value. This was a free function (ToLuaLiteral) on the
	// runtime package until 2026-08-21, where a value that had never met an
	// engine already knew Lua.
	//
	// Not to be confused with [ToWireString] or [ScriptValueToJSON]: a value
	// that leaves the process is read by a parser or a human, never by an
	// engine.
	ToScriptLiteral(value interface{}) string

	// Variable management
	SetVariable(sessionID, name string, value interface{}) error
	GetVariable(sessionID, name string) (interface{}, error)
	HasVariable(sessionID, name string) bool

	// SCXML-specific
	// SetupSystemVariables binds _sessionid / _name / _ioprocessors
	// (W3C SCXML 5.10). The descriptors arrive fully resolved from
	// BuildIoProcessors; an implementation files each one under its name with
	// its location and invents neither, so _ioprocessors reads identically
	// whichever engine backs the session.
	SetupSystemVariables(sessionID, sessionName string, ioProcessors []IoProcessorDescriptor) error
	SetCurrentEvent(sessionID string, args SetCurrentEventArgs) error
	SetStateQueryCallback(sessionID string, cb func(stateID string) bool)

	// W3C SCXML B.2: Set a variable as an XML DOM object with getElementsByTagName/getAttribute methods.
	// Matches C++ setVariableAsDOM / Rust set_variable_as_dom.
	SetVariableAsDOM(sessionID, name, xmlContent string) error

	// Global functions
	RegisterGlobalFunction(name string, fn func(args ...interface{}) (interface{}, error))
}
