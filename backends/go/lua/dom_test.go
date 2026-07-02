// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Unit tests for backends/go/lua/dom.go — verifies the recursive-descent
// parser's coverage of cpp pugixml's `parse_default` feature set.
// 1:1 mirror of backends/rust/lua/src/dom.rs's tests.

package scelua

import (
	"testing"
)

func TestPairedAndSelfClose(t *testing.T) {
	doc := ParseXml("<root><a/><b>x</b></root>")
	if !doc.IsValid() {
		t.Fatalf("doc must be valid, error=%q", doc.Error)
	}
	if got := doc.GetTagName(doc.Root); got != "root" {
		t.Errorf("root tag: got %q want %q", got, "root")
	}
	if got := len(doc.GetElementsByTagName("a")); got != 1 {
		t.Errorf("<a> count: got %d want 1", got)
	}
	if got := len(doc.GetElementsByTagName("b")); got != 1 {
		t.Errorf("<b> count: got %d want 1", got)
	}
}

func TestDoctypePrologue(t *testing.T) {
	xml := `<?xml version="1.0"?><!DOCTYPE root SYSTEM "r.dtd"><root><leaf/></root>`
	doc := ParseXml(xml)
	if !doc.IsValid() {
		t.Fatalf("valid after DOCTYPE: %q", doc.Error)
	}
	if got := doc.GetTagName(doc.Root); got != "root" {
		t.Errorf("root tag: got %q want %q", got, "root")
	}
}

func TestDoctypeInternalSubset(t *testing.T) {
	xml := `<!DOCTYPE root [ <!ELEMENT root (leaf*)> ]><root><leaf/></root>`
	doc := ParseXml(xml)
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
}

func TestCdataSection(t *testing.T) {
	xml := "<root><leaf><![CDATA[ <not-a-tag> & </not-a-tag> ]]></leaf></root>"
	doc := ParseXml(xml)
	if !doc.IsValid() {
		t.Fatalf("CDATA-bearing doc: %q", doc.Error)
	}
	leaves := doc.GetElementsByTagName("leaf")
	if len(leaves) != 1 {
		t.Fatalf("leaf count: got %d want 1", len(leaves))
	}
	leafID := leaves[0]
	cdataID := doc.Nodes[leafID].FirstChild
	if cdataID == xmlNoChild {
		t.Fatal("leaf has no CDATA child")
	}
	if doc.Nodes[cdataID].Type != XmlNodeCdata {
		t.Errorf("expected CDATA type, got %v", doc.Nodes[cdataID].Type)
	}
	want := " <not-a-tag> & </not-a-tag> "
	if got := doc.Nodes[cdataID].Text; got != want {
		t.Errorf("CDATA verbatim: got %q want %q", got, want)
	}
}

func TestNamedEntitiesInAttribute(t *testing.T) {
	doc := ParseXml(`<root attr="&amp;&lt;&gt;&quot;&apos;"/>`)
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	if got := doc.GetAttribute(doc.Root, "attr"); got != "&<>\"'" {
		t.Errorf("named entities: got %q want %q", got, "&<>\"'")
	}
}

func TestNumericEntitiesInAttribute(t *testing.T) {
	// 'A'=65, 'B'=0x42, '€'=U+20AC.
	doc := ParseXml(`<root attr="&#65;&#x42;&#x20AC;"/>`)
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	if got := doc.GetAttribute(doc.Root, "attr"); got != "AB€" {
		t.Errorf("numeric entities: got %q want %q", got, "AB€")
	}
}

func TestMixedTextPcdataWithEntity(t *testing.T) {
	doc := ParseXml("<root>before<inner/>after &amp; tail</root>")
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	rootID := doc.Root
	first := doc.Nodes[rootID].FirstChild
	if first == xmlNoChild || doc.Nodes[first].Type != XmlNodePcdata {
		t.Fatalf("first child must be PCDATA, got type=%v", doc.Nodes[first].Type)
	}
	if doc.Nodes[first].Text != "before" {
		t.Errorf("first PCDATA: got %q want %q", doc.Nodes[first].Text, "before")
	}
	inner := doc.Nodes[first].NextSibling
	if inner == xmlNoChild || doc.Nodes[inner].Type != XmlNodeElement || doc.Nodes[inner].Tag != "inner" {
		t.Fatalf("second child must be <inner>")
	}
	trailing := doc.Nodes[inner].NextSibling
	if trailing == xmlNoChild || doc.Nodes[trailing].Type != XmlNodePcdata {
		t.Fatalf("trailing child must be PCDATA")
	}
	if doc.Nodes[trailing].Text != "after & tail" {
		t.Errorf("trailing PCDATA: got %q want %q", doc.Nodes[trailing].Text, "after & tail")
	}
}

func TestCommentInElementBody(t *testing.T) {
	doc := ParseXml("<root><a/><!-- ignore --><b/></root>")
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	if got := len(doc.GetElementsByTagName("a")); got != 1 {
		t.Errorf("<a> still found: got %d want 1", got)
	}
	if got := len(doc.GetElementsByTagName("b")); got != 1 {
		t.Errorf("<b> still found: got %d want 1", got)
	}
}

func TestGetElementsSkipsTextNodes(t *testing.T) {
	xml := `<root>t1<book title="a"/>t2<![CDATA[raw]]><book title="b"/></root>`
	doc := ParseXml(xml)
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	books := doc.GetElementsByTagName("book")
	if len(books) != 2 {
		t.Fatalf("book count: got %d want 2", len(books))
	}
	if got := doc.GetAttribute(books[0], "title"); got != "a" {
		t.Errorf("first book title: got %q want %q", got, "a")
	}
	if got := doc.GetAttribute(books[1], "title"); got != "b" {
		t.Errorf("second book title: got %q want %q", got, "b")
	}
}

func TestW3CCorpusTest557InlineBooks(t *testing.T) {
	xml := "<books xmlns=\"\">\n  <book title=\"title1\"/>\n  <book title=\"title2\"/>\n</books>"
	doc := ParseXml(xml)
	if !doc.IsValid() {
		t.Fatalf("valid: %q", doc.Error)
	}
	books := doc.GetElementsByTagName("book")
	if len(books) != 2 {
		t.Fatalf("book count: got %d want 2", len(books))
	}
	if got := doc.GetAttribute(books[0], "title"); got != "title1" {
		t.Errorf("first book title: got %q want %q", got, "title1")
	}
	if got := doc.GetAttribute(books[1], "title"); got != "title2" {
		t.Errorf("second book title: got %q want %q", got, "title2")
	}
}
