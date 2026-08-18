// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-1 / §scxml-B-2-8-1: XML in the data model is a DOM
// structure, not three method names.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/dom_read_surface.json`, one claim per case with the
// DOM clause that backs it, and the two C++ engines, the three Kotlin
// engines, the Python binding and the frontend read the same file — a
// per-backend copy drifts toward the backend that reads it, which is the
// blindness that let seven bindings disagree with one specification.
// Measured 2026-08-18, every read in it answered nil here: what this
// binding carried was `getElementsByTagName`, `getAttribute` and
// `getTagName`, which are the two names the W3C IRP suite reads plus one.
//
// The `lua` spelling is what this backend is asked, and it is all it ever
// sees: the frontend lowered the author's ECMAScript at build time, so a
// member read this binding does not answer is a nil index in Lua rather
// than a compile error in Go.

package scelua

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type domSurfaceExpectation struct {
	Number *float64 `json:"number"`
	String *string  `json:"string"`
	Bool   *bool    `json:"bool"`
	Empty  *bool    `json:"empty"`
}

type domSurfaceCase struct {
	Document string                `json:"document"`
	Source   string                `json:"source"`
	Lua      string                `json:"lua"`
	Clause   string                `json:"clause"`
	Expect   domSurfaceExpectation `json:"expect"`
}

type domSurfaceTable struct {
	Documents map[string]string `json:"documents"`
	Cases     []domSurfaceCase  `json:"cases"`
}

// readDomSurfaceTable finds the shared table from this file's own
// location, the same way every other reader names it.
func readDomSurfaceTable(t *testing.T) domSurfaceTable {
	t.Helper()
	// backends/go/lua → repository root
	path := filepath.Join("..", "..", "..", "tests", "ecmascript", "dom_read_surface.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read the shared table at %s: %v", path, err)
	}
	var table domSurfaceTable
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("cannot parse %s: %v", path, err)
	}
	// A floor, not an equality: adding a case must not have to touch this
	// number, but a table that stopped being read must not pass either.
	if len(table.Cases) < 30 {
		t.Fatalf("the shared table produced only %d case(s), so this is not measuring the surface it claims to",
			len(table.Cases))
	}
	return table
}

// evalWithDom binds `var1` to the parsed document and evaluates one
// lowered expression against it.
func evalWithDom(t *testing.T, xml, expr string) interface{} {
	t.Helper()
	engine := NewLuaEngine()
	if err := engine.CreateSession("dom"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession("dom")
	if err := engine.SetVariableAsDOM("dom", "var1", xml); err != nil {
		t.Fatalf("SetVariableAsDOM: %v", err)
	}
	value, err := engine.EvaluateExpression("dom", expr)
	if err != nil {
		t.Fatalf("evaluating %q: %v", expr, err)
	}
	return value
}

func TestDomReadSurfaceAnswersDomLevel1Core(t *testing.T) {
	table := readDomSurfaceTable(t)
	for _, testCase := range table.Cases {
		xml, ok := table.Documents[testCase.Document]
		if !ok {
			t.Errorf("case %q names document %q, which the table lacks", testCase.Source, testCase.Document)
			continue
		}
		got := evalWithDom(t, xml, testCase.Lua)
		if !domSurfaceMatches(got, testCase.Expect) {
			t.Errorf("%s answered %#v (%T), %s says %s", testCase.Lua, got, got, testCase.Clause,
				describeExpectation(testCase.Expect))
		}
	}
}

// domSurfaceMatches reads a whole number as a number rather than as its Go
// type: go-lua hands an integral Lua number back as int64 and a fractional
// one as float64, and which of the two a `nodeType` arrives as is not part
// of the DOM contract.
func domSurfaceMatches(got interface{}, expect domSurfaceExpectation) bool {
	switch {
	case expect.Number != nil:
		number, isNumber := asFloat(got)
		return isNumber && number == *expect.Number
	case expect.String != nil:
		text, isText := got.(string)
		return isText && text == *expect.String
	case expect.Bool != nil:
		boolean, isBool := got.(bool)
		return isBool && boolean == *expect.Bool
	case expect.Empty != nil:
		return got == nil
	default:
		return false
	}
}

func describeExpectation(expect domSurfaceExpectation) string {
	switch {
	case expect.Number != nil:
		return "a number"
	case expect.String != nil:
		return "\"" + *expect.String + "\""
	case expect.Bool != nil:
		if *expect.Bool {
			return "true"
		}
		return "false"
	case expect.Empty != nil:
		return "nil"
	default:
		return "an unreadable expectation"
	}
}

func asFloat(value interface{}) (float64, bool) {
	switch typed := value.(type) {
	case float64:
		return typed, true
	case int64:
		return float64(typed), true
	case int:
		return float64(typed), true
	default:
		return 0, false
	}
}

// The tree an author walks has no whitespace-only text in it, so the arena
// itself has to agree with the cpp reference backend and not only the
// binding on top of it.
func TestWhitespaceBetweenElementsIsNotANode(t *testing.T) {
	doc := ParseXml("<books xmlns=\"\">\n  <book title=\"t1\"/>\n</books>")
	if !doc.IsValid() {
		t.Fatalf("doc must be valid, error=%q", doc.Error)
	}
	children := doc.ChildIDs(doc.Root)
	if len(children) != 1 {
		t.Fatalf("root children: got %d want 1", len(children))
	}
	if got := doc.NodeName(children[0]); got != "book" {
		t.Errorf("first child: got %q want %q", got, "book")
	}
	if got := doc.TextContent(doc.Root); got != "" {
		t.Errorf("textContent of a pretty-printed element: got %q want empty", got)
	}
}

// Mixed content keeps the text that is not only whitespace, and the two
// character-data kinds stay distinguishable — which is what nodeType is
// for.
func TestCharacterDataReportsItsOwnKind(t *testing.T) {
	doc := ParseXml("<p>plain<b>bold</b><![CDATA[raw & <kept>]]></p>")
	if !doc.IsValid() {
		t.Fatalf("doc must be valid, error=%q", doc.Error)
	}
	children := doc.ChildIDs(doc.Root)
	if len(children) != 3 {
		t.Fatalf("children of <p>: got %d want 3", len(children))
	}
	if got := doc.NodeType(children[0]); got != DomNodeTypeText {
		t.Errorf("first child nodeType: got %d want %d", got, DomNodeTypeText)
	}
	if got := doc.NodeValue(children[0]); got != "plain" {
		t.Errorf("first child nodeValue: got %q want %q", got, "plain")
	}
	if got := doc.NodeType(children[2]); got != DomNodeTypeCdataSection {
		t.Errorf("last child nodeType: got %d want %d", got, DomNodeTypeCdataSection)
	}
	if got := doc.PreviousSibling(children[1]); got != children[0] {
		t.Errorf("previousSibling of <b>: got %d want %d", got, children[0])
	}
	if got := doc.LastChild(doc.Root); got != children[2] {
		t.Errorf("lastChild of <p>: got %d want %d", got, children[2])
	}
	if got := doc.TextContent(doc.Root); got != "plainboldraw & <kept>" {
		t.Errorf("textContent: got %q", got)
	}
}
