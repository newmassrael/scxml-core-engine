// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"os"
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
// truthiness rules (W3C SCXML B.2.3).
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

// ToLuaLiteral converts a script value to its Lua literal representation.
// W3C SCXML 5.5: Used by donedata/send to produce event data strings.
func ToLuaLiteral(value interface{}) string {
	if value == nil {
		return "nil"
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
		if v == float64(int64(v)) {
			return fmt.Sprintf("%.1f", v)
		}
		return fmt.Sprintf("%g", v)
	case string:
		escaped := strings.ReplaceAll(v, `\`, `\\`)
		escaped = strings.ReplaceAll(escaped, `"`, `\"`)
		escaped = strings.ReplaceAll(escaped, "\n", `\n`)
		escaped = strings.ReplaceAll(escaped, "\r", `\r`)
		escaped = strings.ReplaceAll(escaped, "\t", `\t`)
		return fmt.Sprintf(`"%s"`, escaped)
	case []interface{}:
		items := make([]string, len(v))
		for i, item := range v {
			items[i] = ToLuaLiteral(item)
		}
		return "{" + strings.Join(items, ", ") + "}"
	case map[string]interface{}:
		items := make([]string, 0, len(v))
		for k, val := range v {
			items = append(items, fmt.Sprintf(`["%s"] = %s`, k, ToLuaLiteral(val)))
		}
		return "{" + strings.Join(items, ", ") + "}"
	default:
		return fmt.Sprintf("%v", v)
	}
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

	// Variable management
	SetVariable(sessionID, name string, value interface{}) error
	GetVariable(sessionID, name string) (interface{}, error)
	HasVariable(sessionID, name string) bool

	// SCXML-specific
	SetupSystemVariables(sessionID string) error
	SetCurrentEvent(sessionID, name, data, eventType, sendID, origin, originType, invokeID string) error
	SetStateQueryCallback(sessionID string, cb func(stateID string) bool)

	// W3C SCXML B.2: Set a variable as an XML DOM object with getElementsByTagName/getAttribute methods.
	// Matches C++ setVariableAsDOM / Rust set_variable_as_dom.
	SetVariableAsDOM(sessionID, name, xmlContent string) error

	// Global functions
	RegisterGlobalFunction(name string, fn func(args ...interface{}) (interface{}, error))
}

var globalScriptEngine IScriptEngine

// RegisterScriptEngine registers a global script engine.
func RegisterScriptEngine(engine IScriptEngine) {
	globalScriptEngine = engine
}

// GetScriptEngine returns the registered global script engine.
func GetScriptEngine() IScriptEngine {
	return globalScriptEngine
}
