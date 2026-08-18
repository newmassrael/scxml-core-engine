// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — Go AOT.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, every backend created an object carrying
// three methods — `getElementsByTagName`, `getAttribute` and a non-standard
// `getTagName`, which are the two names the W3C IRP suite reads plus one — so
// `doc.tagName`, `doc.firstChild` and `doc.childNodes.length` answered nil on
// all seven channels with the whole W3C suite green.
//
// What this adds to `backends/go/lua/dom_read_surface_test.go`, which measures
// the same surface against the same shared table, is the SEAM: the `<data>`
// initializer the code generator emits, and the guards it lowered. A binding
// being right does not say a document reaches it.
//
// Fixture: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_xml_data_is_a_dom_tree_go.sh

package xml_data_is_a_dom_tree

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []XmlDataIsADomTreeState, want XmlDataIsADomTreeState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestADataElementsXMLIsADomTreeTheDocumentCanWalk(t *testing.T) {
	policy := NewXmlDataIsADomTreePolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture reads the DOM in its guards, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[XmlDataIsADomTreeState, XmlDataIsADomTreeEvent](&policy)
	engine.Initialize()

	// Every transition is eventless, so the verdict is reached in the first
	// macrostep and no event is needed to ask the question.
	engine.Step()

	states := engine.GetActiveStates()
	if active(states, XmlDataIsADomTreeStateNotADocument) {
		t.Fatalf("the variable did not hold a document: nodeType === 9, "+
			"nodeName === '#document', documentElement.tagName === 'books' or "+
			"hasAttribute('count') did not hold (active: %v)", states)
	}
	if active(states, XmlDataIsADomTreeStateWrongTree) {
		t.Fatalf("the document element's children are not the two <book> elements in "+
			"document order — the whitespace between them may have become nodes, or a "+
			"sibling/parent link is missing (active: %v)", states)
	}
	if active(states, XmlDataIsADomTreeStateNoText) {
		t.Fatalf("character data did not report itself as a text node, or textContent "+
			"did not read the text below the element (active: %v)", states)
	}
	if !active(states, XmlDataIsADomTreeStateSettled) {
		t.Fatalf("the machine reached none of its four verdicts, so the guards did not "+
			"evaluate at all (active: %v)", states)
	}
}
