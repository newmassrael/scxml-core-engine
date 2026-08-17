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
	"io"
	"strings"
	"sync"

	sce "github.com/newmassrael/sce-go-runtime"
	lua "github.com/Shopify/go-lua"
)

//go:embed json_builtins.lua
var jsonBuiltinsLua string

// W3C SCXML B.2 ECMAScript operator semantics. Byte copy of
// sce/include/scripting/ecma_semantics.lua — `go:embed` cannot reach
// outside the module, the same reason json_builtins.lua is copied here.
// `shared_lua_assets_are_byte_identical` fails if the copies drift.
//
//go:embed ecma_semantics.lua
var ecmaSemanticsLua string

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

	// W3C SCXML B.2: the ECMAScript operators and library Lua does not
	// share, from the same shared source every other backend loads. Written
	// to be Lua 5.2 compatible precisely because go-lua is: it has no
	// bitwise operators at all, so the bit helpers there are arithmetic.
	//
	// This engine used to define `_scxml_truthy`, `_typeof`, `_isArray`,
	// `_indexOf`, `_concat`, `parseInt` and `parseFloat` here in Go, one
	// implementation among six. Measured 2026-08-16 against the shared
	// ECMA-262 table: two of them had no Array branch at all, so
	// `[1,2,3].indexOf(2)` answered -1 and `[].concat(a, [4])` answered ""
	// on this backend and nowhere else — with the W3C suite green, because
	// no fixture in it asks. They are in ecma_semantics.lua now.
	if err := lua.DoString(l, ecmaSemanticsLua); err != nil {
		return fmt.Errorf("failed to load ECMAScript semantics: %w", err)
	}

	// Load JSON builtins from shared source (JSON.stringify works in go-lua)
	if err := lua.DoString(l, jsonBuiltinsLua); err != nil {
		return fmt.Errorf("failed to load JSON builtins: %w", err)
	}

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
			// §scxml-B-2-8-1 gives `_event.data` three readings and no
			// fourth: XML becomes a DOM, JSON becomes the value, and
			// anything else becomes a space-normalized string. There used to
			// be a rung above these two — `lua.DoString("return "+data)`,
			// running the payload as Lua source before anything looked at it
			// — and it decided all three of the following, measured
			// 2026-08-17 on the sibling Rust engine that had the same rung:
			//
			//   * `2 + 3` from a host arrived as the number 5, and as the
			//     string "2 + 3" on the cpp and Rhino engines that read the
			//     clause instead. One payload, two answers.
			//   * a payload `(function() ... end)()` RAN, in the session's
			//     own globals. `_event.data` is the one field an SCXML
			//     document takes from outside itself.
			//   * it was load-bearing: `<send>` shipped `_scxml_params({...})`
			//     — Lua source — so this rung was the deserializer.
			//
			// The sender now ships JSON (§scxml-B-2-9: data that leaves the
			// data model is serialized to JSON), which is what the cpp engine
			// has always shipped, so the two rungs the clause names are the
			// two that are here.
			if jsonVal, ok := decodeJSON(args.Data); ok {
				pushJSONValue(sess.l, jsonVal)
				sess.l.SetField(-2, "data")
			} else {
				// W3C SCXML B.2 test 562: Fall back to whitespace-normalized string
				normalized := strings.Join(strings.Fields(args.Data), " ")
				sess.l.PushString(normalized)
				sess.l.SetField(-2, "data")
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

// luaToGo converts a Lua stack value to a Go interface{}.
//
// The value's TYPE is what decides, not what it could be converted to.
// `IsNumber` answers true for a string that parses as one — that is Lua's
// `lua_isnumber`, and it is the same coercion `tonumber` performs — so asking
// it first handed every numeric string back to the host as an integer. W3C
// SCXML B.2 makes that a datamodel defect rather than a formatting one:
// `1 + ''` is the string "1" in ECMAScript, and a host that receives 1 cannot
// tell it from arithmetic. Measured 2026-08-16 against
// `tests/ecmascript/ecma262_semantics.json`, which is also where it is pinned.
func (e *LuaEngine) luaToGo(l *lua.State, idx int) interface{} {
	switch l.TypeOf(idx) {
	case lua.TypeBoolean:
		return l.ToBoolean(idx)
	case lua.TypeNumber:
		n, _ := l.ToNumber(idx)
		// ECMA-262 has one Number type; an integral value is handed over as
		// an integer so a host printing it does not render "3" as "3.0".
		if n == float64(int64(n)) {
			return int64(n)
		}
		return n
	case lua.TypeString:
		s, _ := l.ToString(idx)
		return s
	case lua.TypeTable:
		return e.luaTableToGo(l, idx)
	default:
		// Nil, none, and the value kinds the datamodel cannot carry.
		return nil
	}
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

		jsonVal, ok := decodeJSON(s)
		if !ok {
			l.PushNil()
			return 1
		}

		pushJSONValue(l, jsonVal)
		return 1
	})
	l.SetField(-2, "parse")
	l.Pop(1) // pop JSON table
}

// decodeJSON parses a JSON document, reporting `false` when the text is not
// one. Both JSON entry points — the author-facing `JSON.parse` and the
// `_event.data` payload path — go through here so a document means the same
// thing whichever of the two reads it.
//
// The trailing-token check makes the accepted set a whole document rather than
// a prefix of one: `Decode` alone stops at the end of the first value, so
// `{"a":1} garbage` would otherwise parse.
func decodeJSON(s string) (interface{}, bool) {
	dec := json.NewDecoder(strings.NewReader(s))
	var val interface{}
	if err := dec.Decode(&val); err != nil {
		return nil, false
	}
	if _, err := dec.Token(); err != io.EOF {
		return nil, false
	}
	return val, true
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
		// Every JSON number lands here. This engine is go-lua, whose
		// `PushInteger` is `apiPush(float64(n))` — it has no integer subtype
		// to reach for, unlike the cpp and Rust engines' Lua 5.4. Splitting
		// integral values out would therefore be a distinction with no
		// observable difference; `tostring` already renders a whole float
		// without a fractional part.
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
