// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package scelua provides a Lua 5.2 script engine for sce-go-runtime.
//
// 1:1 port of Rust sce-rust-lua (backends/rust/lua/src/lib.rs). Uses the pure-Go
// Shopify/go-lua library for Lua 5.2 embedding (no CGo required).
//
// Each state machine gets its own Lua session (isolated L state) via
// CreateSession/DestroySession. System variables (_sessionid, _name, _event)
// are set per the W3C SCXML specification.
package scelua

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"sync"

	sce "github.com/newmassrael/sce-go-runtime"
	lua "github.com/Shopify/go-lua"
)

//go:embed json_builtins.lua
var jsonBuiltinsLua string

// session holds per-state-machine Lua state.
type session struct {
	l              *lua.State
	declaredVars   map[string]bool
	stateQueryCB   func(string) bool
}

// LuaEngine implements sce.IScriptEngine using Shopify/go-lua.
type LuaEngine struct {
	mu       sync.Mutex
	sessions map[string]*session
	globals  map[string]func(args ...interface{}) (interface{}, error)
}

// NewLuaEngine creates a new Lua script engine.
func NewLuaEngine() *LuaEngine {
	return &LuaEngine{
		sessions: make(map[string]*session),
		globals:  make(map[string]func(args ...interface{}) (interface{}, error)),
	}
}

func (e *LuaEngine) Initialize() error {
	return nil
}

func (e *LuaEngine) Shutdown() {
	e.mu.Lock()
	defer e.mu.Unlock()
	for id := range e.sessions {
		delete(e.sessions, id)
	}
}

func (e *LuaEngine) CreateSession(sessionID string) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	l := lua.NewState()
	lua.OpenLibraries(l)

	sess := &session{
		l:            l,
		declaredVars: make(map[string]bool),
	}

	// Register builtins
	e.setupBuiltins(sess)

	// Load JSON builtins from shared source (JSON.stringify works in go-lua)
	if err := lua.DoString(l, jsonBuiltinsLua); err != nil {
		return fmt.Errorf("failed to load JSON builtins: %w", err)
	}

	// Register ECMAScript compatibility helpers
	e.setupECMAScriptCompat(sess)

	// Override JSON.parse with Go-native implementation (go-lua lacks string.gsub
	// which the Lua-based JSON.parse needs).
	e.registerNativeJSONParse(sess)

	e.sessions[sessionID] = sess
	return nil
}

func (e *LuaEngine) DestroySession(sessionID string) {
	e.mu.Lock()
	defer e.mu.Unlock()
	delete(e.sessions, sessionID)
}

func (e *LuaEngine) HasSession(sessionID string) bool {
	e.mu.Lock()
	defer e.mu.Unlock()
	_, ok := e.sessions[sessionID]
	return ok
}

func (e *LuaEngine) getSession(sessionID string) (*session, error) {
	e.mu.Lock()
	defer e.mu.Unlock()
	sess, ok := e.sessions[sessionID]
	if !ok {
		return nil, fmt.Errorf("session not found: %s", sessionID)
	}
	return sess, nil
}

func (e *LuaEngine) ExecuteScript(sessionID, script string) error {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return err
	}
	return lua.DoString(sess.l, script)
}

func (e *LuaEngine) EvaluateExpression(sessionID, expr string) (interface{}, error) {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return nil, err
	}

	// W3C SCXML: Detect undeclared simple variable references (C++ LuaEngine parity).
	// JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
	if isUndeclaredSimpleVariable(expr, sess) {
		return nil, fmt.Errorf("ReferenceError: %s is not defined", expr)
	}

	// Wrap expression in "return" statement for evaluation
	top := sess.l.Top()
	script := "return " + expr
	if err := lua.DoString(sess.l, script); err != nil {
		sess.l.SetTop(top)
		return nil, err
	}

	result := e.luaToGo(sess.l, -1)
	sess.l.Pop(1)
	return result, nil
}

func (e *LuaEngine) SetVariable(sessionID, name string, value interface{}) error {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return err
	}
	e.pushGoValue(sess.l, value)
	sess.l.SetGlobal(name)
	sess.declaredVars[name] = true
	return nil
}

func (e *LuaEngine) GetVariable(sessionID, name string) (interface{}, error) {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return nil, err
	}
	sess.l.Global(name)
	result := e.luaToGo(sess.l, -1)
	sess.l.Pop(1)
	return result, nil
}

func (e *LuaEngine) HasVariable(sessionID, name string) bool {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return false
	}
	return sess.declaredVars[name]
}

func (e *LuaEngine) SetupSystemVariables(sessionID, sessionName string, ioProcessors []sce.IoProcessorDescriptor) error {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return err
	}
	// W3C SCXML 5.10: Initialize system variables
	_ = e.SetVariable(sessionID, "_sessionid", sessionID)
	_ = e.SetVariable(sessionID, "_name", sessionName)
	// W3C SCXML C.1.1 / C.2.3: one entry per processor the deployment
	// supports, each with a location field holding the address that reaches
	// this session through it. Names and locations are decided by
	// BuildIoProcessors, so this engine's view of _ioprocessors matches every
	// other backend's.
	ioTable := make(map[string]interface{}, len(ioProcessors))
	for _, processor := range ioProcessors {
		ioTable[processor.Name] = map[string]interface{}{
			"location": processor.Location,
		}
	}
	_ = e.SetVariable(sessionID, "_ioprocessors", ioTable)
	sess.declaredVars["_event"] = true
	sess.declaredVars["_sessionid"] = true
	sess.declaredVars["_name"] = true
	sess.declaredVars["_ioprocessors"] = true
	return nil
}

func (e *LuaEngine) SetCurrentEvent(sessionID string, args sce.SetCurrentEventArgs) error {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return err
	}

	// W3C SCXML 5.10: Create _event table
	sess.l.NewTable()

	sess.l.PushString(args.Name)
	sess.l.SetField(-2, "name")

	if args.Data != "" {
		trimmed := strings.TrimSpace(args.Data)
		if strings.HasPrefix(trimmed, "<") {
			// W3C SCXML B.2: XML data -> DOM object (test 561)
			e.pushDOMTable(sess, args.Data)
			sess.l.SetField(-2, "data")
		} else {
			// Try to evaluate as Lua expression first
			top := sess.l.Top()
			if err := lua.DoString(sess.l, "return "+args.Data); err == nil {
				sess.l.SetField(-2, "data")
			} else {
				sess.l.SetTop(top)
				// W3C SCXML B.2 test 562/578: Try JSON-to-Lua conversion
				luaSyntax := jsonToLuaTable(args.Data)
				if err := lua.DoString(sess.l, "return "+luaSyntax); err == nil {
					sess.l.SetField(-2, "data")
				} else {
					sess.l.SetTop(top)
					// W3C SCXML B.2: Fall back to whitespace-normalized string
					normalized := strings.Join(strings.Fields(args.Data), " ")
					sess.l.PushString(normalized)
					sess.l.SetField(-2, "data")
				}
			}
		}
	} else {
		sess.l.PushString("")
		sess.l.SetField(-2, "data")
	}

	sess.l.PushString(args.Type)
	sess.l.SetField(-2, "type")

	sess.l.PushString(args.SendID)
	sess.l.SetField(-2, "sendid")

	// W3C SCXML 5.10.1: Always set origin/origintype so targetexpr="_event.origin"
	// evaluates to empty string (not nil) when origin is unset (test 336).
	sess.l.PushString(args.Origin)
	sess.l.SetField(-2, "origin")

	sess.l.PushString(args.OriginType)
	sess.l.SetField(-2, "origintype")

	sess.l.PushString(args.InvokeID)
	sess.l.SetField(-2, "invokeid")

	sess.l.SetGlobal("_event")
	return nil
}

// jsonToLuaTable converts JSON syntax ("key": val) to Lua table syntax (["key"] = val).
// Port of Rust sce-rust-lua json_to_lua_table().
func jsonToLuaTable(json string) string {
	var result strings.Builder
	result.Grow(len(json))
	bytes := []byte(json)
	n := len(bytes)
	i := 0
	for i < n {
		if bytes[i] == '"' {
			// Capture the full quoted string
			var key strings.Builder
			key.WriteByte('"')
			i++
			for i < n {
				c := bytes[i]
				key.WriteByte(c)
				i++
				if c == '"' {
					break
				}
				if c == '\\' && i < n {
					key.WriteByte(bytes[i])
					i++
				}
			}
			// Skip whitespace after string
			var spaces strings.Builder
			for i < n && (bytes[i] == ' ' || bytes[i] == '\t' || bytes[i] == '\n' || bytes[i] == '\r') {
				spaces.WriteByte(bytes[i])
				i++
			}
			if i < n && bytes[i] == ':' {
				i++ // consume ':'
				// JSON key -> Lua: ["key"] =
				result.WriteByte('[')
				result.WriteString(key.String())
				result.WriteByte(']')
				result.WriteString(spaces.String())
				result.WriteByte('=')
			} else {
				// Just a string value
				result.WriteString(key.String())
				result.WriteString(spaces.String())
			}
		} else {
			result.WriteByte(bytes[i])
			i++
		}
	}
	return result.String()
}

// pushDOMTable parses xml into a full DOM tree and pushes a Lua table
// onto the stack whose three methods (getElementsByTagName /
// getAttribute / getTagName) dispatch through the parsed tree.  Each
// method is a closure that captures (*XmlDoc, nodeID), which keeps the
// arena alive for as long as any Lua reference exists — equivalent to
// cpp's `shared_ptr<XMLElement>` semantics and Rust's `Arc<XmlDoc>`
// UserData.  Parse failure leaves the stack empty and pushes nil so
// callers (W3C SCXML B.2 paths) get the spec's "leave variable
// undefined on parse error" behaviour.  Mirrors cpp
// `LuaDOMBinding::pushDOMObject` (sce/src/scripting/LuaDOMBinding.cpp:74).
func (e *LuaEngine) pushDOMTable(sess *session, xml string) {
	doc := ParseXml(xml)
	if !doc.IsValid() {
		sess.l.PushNil()
		return
	}
	e.pushDOMNode(sess, doc, doc.Root, true)
}

// pushDOMNode pushes a Lua table for a single tree node (document or
// element).  `isDocument` toggles cpp `XMLDocument`-vs-`XMLElement`
// semantics: getElementsByTagName recurses from the root inclusively
// for documents, descends into children only for elements.
func (e *LuaEngine) pushDOMNode(sess *session, doc *XmlDoc, nodeID int, isDocument bool) {
	l := sess.l
	l.NewTable()

	// getElementsByTagName(tag) -> 1-based array of element tables
	// (ECMAScript [0]/[1] are lowered to Lua [1]/[2] by the
	// transformer upstream).
	l.PushGoFunction(func(l *lua.State) int {
		tag, _ := l.ToString(2)
		var ids []int
		if isDocument {
			ids = doc.GetElementsByTagName(tag)
		} else {
			ids = doc.GetElementsByTagNameFrom(nodeID, tag)
		}
		l.NewTable()
		for i, id := range ids {
			e.pushDOMNode(sess, doc, id, false)
			l.RawSetInt(-2, i+1)
		}
		return 1
	})
	l.SetField(-2, "getElementsByTagName")

	// getAttribute(name) -> string ("" on miss, matches cpp)
	l.PushGoFunction(func(l *lua.State) int {
		attrName, _ := l.ToString(2)
		l.PushString(doc.GetAttribute(nodeID, attrName))
		return 1
	})
	l.SetField(-2, "getAttribute")

	// getTagName() -> string ("" on non-element, matches cpp)
	l.PushGoFunction(func(l *lua.State) int {
		l.PushString(doc.GetTagName(nodeID))
		return 1
	})
	l.SetField(-2, "getTagName")
}

// SetVariableAsDOM sets a variable as an XML DOM table with getElementsByTagName/getAttribute methods.
// W3C SCXML B.2: XML content in <data> elements must be assigned as DOM structures.
// Matches C++ setVariableAsDOM / Rust set_variable_as_dom.
func (e *LuaEngine) SetVariableAsDOM(sessionID, name, xmlContent string) error {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return err
	}
	e.pushDOMTable(sess, xmlContent)
	sess.l.SetGlobal(name)
	sess.declaredVars[name] = true
	return nil
}

func (e *LuaEngine) RegisterGlobalFunction(name string, fn func(args ...interface{}) (interface{}, error)) {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.globals[name] = fn
}

func (e *LuaEngine) SetStateQueryCallback(sessionID string, cb func(stateID string) bool) {
	sess, err := e.getSession(sessionID)
	if err != nil {
		return
	}

	sess.stateQueryCB = cb

	// Register In() function in Lua
	sess.l.PushGoFunction(func(l *lua.State) int {
		stateID, ok := l.ToString(1)
		if !ok {
			l.PushBoolean(false)
			return 1
		}
		l.PushBoolean(cb(stateID))
		return 1
	})
	sess.l.SetGlobal("In")
}

// ── Internal helpers ───────────────────────────────────────────────

func (e *LuaEngine) setupBuiltins(sess *session) {
	l := sess.l

	// _scxml_truthy: ECMAScript truthiness semantics
	l.PushGoFunction(func(l *lua.State) int {
		if l.IsNoneOrNil(1) {
			l.PushBoolean(false)
			return 1
		}
		if l.IsBoolean(1) {
			l.PushBoolean(l.ToBoolean(1))
			return 1
		}
		if l.IsNumber(1) {
			n, _ := l.ToNumber(1)
			l.PushBoolean(n != 0)
			return 1
		}
		if l.IsString(1) {
			s, _ := l.ToString(1)
			l.PushBoolean(s != "")
			return 1
		}
		l.PushBoolean(true)
		return 1
	})
	l.SetGlobal("_scxml_truthy")

	// _typeof: ECMAScript typeof operator
	l.PushGoFunction(func(l *lua.State) int {
		if l.IsNoneOrNil(1) {
			l.PushString("undefined")
		} else if l.IsBoolean(1) {
			l.PushString("boolean")
		} else if l.IsNumber(1) {
			l.PushString("number")
		} else if l.IsString(1) {
			l.PushString("string")
		} else if l.IsTable(1) {
			l.PushString("object")
		} else if l.IsFunction(1) {
			l.PushString("function")
		} else {
			l.PushString("undefined")
		}
		return 1
	})
	l.SetGlobal("_typeof")

	// parseInt: ECMAScript parseInt
	l.PushGoFunction(func(l *lua.State) int {
		s, ok := l.ToString(1)
		if !ok {
			l.PushNumber(0)
			return 1
		}
		s = strings.TrimSpace(s)
		radix := 10
		if l.IsNumber(2) {
			n, _ := l.ToNumber(2)
			radix = int(n)
		}
		if radix == 0 {
			radix = 10
		}
		val, err := strconv.ParseInt(s, radix, 64)
		if err != nil {
			l.PushNumber(0)
		} else {
			l.PushNumber(float64(val))
		}
		return 1
	})
	l.SetGlobal("parseInt")

	// parseFloat: ECMAScript parseFloat
	l.PushGoFunction(func(l *lua.State) int {
		s, ok := l.ToString(1)
		if !ok {
			l.PushNumber(0)
			return 1
		}
		val, err := strconv.ParseFloat(strings.TrimSpace(s), 64)
		if err != nil {
			l.PushNumber(0)
		} else {
			l.PushNumber(val)
		}
		return 1
	})
	l.SetGlobal("parseFloat")
}

func (e *LuaEngine) setupECMAScriptCompat(sess *session) {
	l := sess.l

	// _isArray: Check if a table is an array (sequential integer keys)
	l.PushGoFunction(func(l *lua.State) int {
		if !l.IsTable(1) {
			l.PushBoolean(false)
			return 1
		}
		l.Length(1)
		n, _ := l.ToNumber(-1)
		l.Pop(1)
		l.PushBoolean(int(n) > 0)
		return 1
	})
	l.SetGlobal("_isArray")

	// _indexOf: String/array indexOf
	l.PushGoFunction(func(l *lua.State) int {
		if l.IsString(1) {
			haystack, _ := l.ToString(1)
			needle, _ := l.ToString(2)
			idx := strings.Index(haystack, needle)
			l.PushNumber(float64(idx))
		} else {
			l.PushNumber(-1)
		}
		return 1
	})
	l.SetGlobal("_indexOf")

	// _concat: Array concatenation
	l.PushGoFunction(func(l *lua.State) int {
		if l.IsString(1) && l.IsString(2) {
			s1, _ := l.ToString(1)
			s2, _ := l.ToString(2)
			l.PushString(s1 + s2)
		} else {
			l.PushString("")
		}
		return 1
	})
	l.SetGlobal("_concat")
}

// luaToGo converts a Lua stack value to a Go interface{}.
func (e *LuaEngine) luaToGo(l *lua.State, idx int) interface{} {
	if l.IsNoneOrNil(idx) {
		return nil
	}
	if l.IsBoolean(idx) {
		return l.ToBoolean(idx)
	}
	if l.IsNumber(idx) {
		n, _ := l.ToNumber(idx)
		// Return as int64 if it's a whole number
		if n == float64(int64(n)) {
			return int64(n)
		}
		return n
	}
	if l.IsString(idx) {
		s, _ := l.ToString(idx)
		return s
	}
	if l.IsTable(idx) {
		return e.luaTableToGo(l, idx)
	}
	return nil
}

// luaTableToGo converts a Lua table to a Go map or slice.
func (e *LuaEngine) luaTableToGo(l *lua.State, idx int) interface{} {
	if idx < 0 {
		idx = l.AbsIndex(idx)
	}

	// Check if it's an array (has sequential integer keys)
	l.Length(idx)
	n, _ := l.ToNumber(-1)
	l.Pop(1)
	length := int(n)

	if length > 0 {
		// Treat as array
		result := make([]interface{}, length)
		for i := 1; i <= length; i++ {
			l.RawGetInt(idx, i)
			result[i-1] = e.luaToGo(l, -1)
			l.Pop(1)
		}
		return result
	}

	// Treat as object
	result := make(map[string]interface{})
	l.PushNil()
	for l.Next(idx) {
		key, ok := l.ToString(-2)
		if ok {
			result[key] = e.luaToGo(l, -1)
		}
		l.Pop(1)
	}
	return result
}

// pushGoValue pushes a Go value onto the Lua stack.
func (e *LuaEngine) pushGoValue(l *lua.State, value interface{}) {
	if value == nil {
		l.PushNil()
		return
	}
	switch v := value.(type) {
	case bool:
		l.PushBoolean(v)
	case int:
		l.PushNumber(float64(v))
	case int64:
		l.PushNumber(float64(v))
	case float64:
		l.PushNumber(v)
	case string:
		l.PushString(v)
	case []interface{}:
		l.NewTable()
		for i, item := range v {
			e.pushGoValue(l, item)
			l.RawSetInt(-2, i+1)
		}
	case map[string]interface{}:
		l.NewTable()
		for k, val := range v {
			l.PushString(k)
			e.pushGoValue(l, val)
			l.SetTable(-3)
		}
	default:
		l.PushString(fmt.Sprintf("%v", v))
	}
}

// isUndeclaredSimpleVariable checks if an expression is a simple variable reference
// to an undeclared variable. Port of Rust is_undeclared_simple_variable().
// W3C SCXML: JavaScript throws ReferenceError for undeclared variables; Lua returns nil.
func isUndeclaredSimpleVariable(expr string, sess *session) bool {
	if expr == "" {
		return false
	}
	// Must start with letter or underscore
	first := expr[0]
	if !((first >= 'a' && first <= 'z') || (first >= 'A' && first <= 'Z') || first == '_') {
		return false
	}
	// Extract base identifier (before first '.' or '[')
	baseEnd := len(expr)
	for i := 0; i < len(expr); i++ {
		c := expr[i]
		if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_') {
			baseEnd = i
			break
		}
	}
	if baseEnd == 0 {
		return false
	}
	baseName := expr[:baseEnd]

	// Lua keywords are not variables
	switch baseName {
	case "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
		"if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while":
		return false
	}

	// Check declared vars
	if sess.declaredVars[baseName] {
		return false
	}

	// Check Lua standard library globals (math, string, table, etc.)
	sess.l.Global(baseName)
	isNil := sess.l.IsNoneOrNil(-1)
	sess.l.Pop(1)

	return isNil
}

// registerNativeJSONParse overrides JSON.parse with a Go-native implementation.
// Required because go-lua lacks string.gsub which the Lua-based JSON.parse needs.
// Matches the behavior of the shared json_builtins.lua JSON.parse.
func (e *LuaEngine) registerNativeJSONParse(sess *session) {
	l := sess.l

	// Get the JSON table (created by json_builtins.lua)
	l.Global("JSON")
	if !l.IsTable(-1) {
		l.Pop(1)
		return
	}

	l.PushGoFunction(func(l *lua.State) int {
		s, ok := l.ToString(1)
		if !ok || s == "" {
			l.PushNil()
			return 1
		}

		// Use Go's encoding/json to parse the JSON string
		var jsonVal interface{}
		if err := json.Unmarshal([]byte(s), &jsonVal); err != nil {
			l.PushNil()
			return 1
		}

		pushJSONValue(l, jsonVal)
		return 1
	})
	l.SetField(-2, "parse")
	l.Pop(1) // pop JSON table
}

// pushJSONValue pushes a parsed JSON value onto the Lua stack.
func pushJSONValue(l *lua.State, val interface{}) {
	if val == nil {
		l.PushNil()
		return
	}
	switch v := val.(type) {
	case bool:
		l.PushBoolean(v)
	case float64:
		l.PushNumber(v)
	case string:
		l.PushString(v)
	case []interface{}:
		l.NewTable()
		for i, item := range v {
			pushJSONValue(l, item)
			l.RawSetInt(-2, i+1) // Lua 1-based indexing
		}
	case map[string]interface{}:
		l.NewTable()
		for k, item := range v {
			l.PushString(k)
			pushJSONValue(l, item)
			l.SetTable(-3)
		}
	default:
		l.PushString(fmt.Sprintf("%v", v))
	}
}
