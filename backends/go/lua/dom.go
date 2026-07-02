// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML B.2 — XML DOM tree for the Go Lua backend.
//
// 1:1 algorithmic mirror of `sce/include/scripting/XMLDOMWrapper.h` and
// `sce/src/scripting/XMLDOMWrapper.cpp` (cpp ref-backend, pugixml-based),
// reimplemented in pure Go without the third_party `pugi::xml_*` types.
// Coverage matches the cpp `parse_default` feature set:
//
//   * paired `<tag>...</tag>` + self-close `<tag/>`
//   * `attr="value"` and `attr='value'` both quote styles
//   * `xmlns=""` / `xmlns:x=""` as regular attributes (no namespace
//     prefix processing — pugixml's default)
//   * named entity refs `&amp;` / `&lt;` / `&gt;` / `&quot;` / `&apos;`
//     and numeric refs `&#N;` / `&#xN;` (UTF-8 encoded), in attribute
//     values and text content
//   * `<?xml ?>` PI prologue + `<!-- comment -->` skip (anywhere)
//   * `<!DOCTYPE ...>` skip (with optional internal subset `[...]`)
//   * `<![CDATA[...]]>` content as a CDATA node child
//   * mixed text content as PCDATA node children
//
// Storage uses an arena (`Nodes []XmlNode` + index-based pointers) so
// the tree is GC-friendly: any closure that captures the document
// pointer keeps the whole arena alive. `GetElementsByTagName` returns
// `[]int` of element node ids; the Lua binding wraps each id in a
// closure capturing the document so the document outlives any element
// reference — cpp's `shared_ptr<XMLElement>` semantics.

package scelua

import (
	"fmt"
	"strconv"
)

type XmlNodeType int

const (
	XmlNodeElement XmlNodeType = iota
	XmlNodePcdata
	XmlNodeCdata
)

type XmlAttr struct {
	Name  string
	Value string
}

type XmlNode struct {
	Type        XmlNodeType
	Tag         string  // element: tag name; else empty
	Text        string  // pcdata/cdata: content; else empty
	Attrs       []XmlAttr
	FirstChild  int // -1 = none
	NextSibling int
	Parent      int
}

const xmlNoChild = -1

type XmlDoc struct {
	Nodes []XmlNode
	Root  int    // -1 = none
	Error string // empty = no error
}

// IsValid reports whether the document parsed cleanly and has a root.
func (d *XmlDoc) IsValid() bool {
	return d.Root != xmlNoChild && d.Error == ""
}

// GetElementsByTagName — cpp `XMLDocument::getElementsByTagName`,
// recursive descent from root (root included if it matches).
func (d *XmlDoc) GetElementsByTagName(tag string) []int {
	var out []int
	if d.Root != xmlNoChild {
		d.collect(d.Root, tag, &out)
	}
	return out
}

// GetElementsByTagNameFrom — cpp `XMLElement::getElementsByTagName`,
// descends into each child (the node itself is not matched).
func (d *XmlDoc) GetElementsByTagNameFrom(nodeID int, tag string) []int {
	var out []int
	if nodeID < 0 || nodeID >= len(d.Nodes) {
		return out
	}
	c := d.Nodes[nodeID].FirstChild
	for c != xmlNoChild {
		d.collect(c, tag, &out)
		c = d.Nodes[c].NextSibling
	}
	return out
}

// GetAttribute — cpp `XMLElement::getAttribute`. Returns "" on miss
// (matches cpp's `node_.attribute(...)` empty-attr behaviour).
func (d *XmlDoc) GetAttribute(nodeID int, name string) string {
	if nodeID < 0 || nodeID >= len(d.Nodes) {
		return ""
	}
	for _, a := range d.Nodes[nodeID].Attrs {
		if a.Name == name {
			return a.Value
		}
	}
	return ""
}

// GetTagName — cpp `XMLElement::getTagName`. Returns "" for non-element
// nodes or out-of-range ids.
func (d *XmlDoc) GetTagName(nodeID int) string {
	if nodeID < 0 || nodeID >= len(d.Nodes) {
		return ""
	}
	return d.Nodes[nodeID].Tag
}

func (d *XmlDoc) collect(nodeID int, tag string, out *[]int) {
	if nodeID < 0 || nodeID >= len(d.Nodes) {
		return
	}
	n := &d.Nodes[nodeID]
	if n.Type == XmlNodeElement && n.Tag == tag {
		*out = append(*out, nodeID)
	}
	c := n.FirstChild
	for c != xmlNoChild {
		d.collect(c, tag, out)
		c = d.Nodes[c].NextSibling
	}
}

// ─── Parser ─────────────────────────────────────────────────────────

type xmlParser struct {
	src   []byte
	pos   int
	error string
}

func (p *xmlParser) setError(msg string) {
	if p.error == "" {
		p.error = fmt.Sprintf("%s (at byte %d)", msg, p.pos)
	}
}

func (p *xmlParser) skipWS() {
	for p.pos < len(p.src) {
		c := p.src[p.pos]
		if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
			p.pos++
		} else {
			break
		}
	}
}

func (p *xmlParser) matchLit(lit string) bool {
	if p.pos+len(lit) > len(p.src) {
		return false
	}
	if string(p.src[p.pos:p.pos+len(lit)]) != lit {
		return false
	}
	p.pos += len(lit)
	return true
}

func xmlIsNameStart(c byte) bool {
	return c == '_' || c == ':' ||
		(c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

func xmlIsNameChar(c byte) bool {
	return xmlIsNameStart(c) || c == '-' || c == '.' ||
		(c >= '0' && c <= '9')
}

func (p *xmlParser) parseName() (string, bool) {
	p.skipWS()
	if p.pos >= len(p.src) || !xmlIsNameStart(p.src[p.pos]) {
		p.setError("expected name")
		return "", false
	}
	start := p.pos
	p.pos++
	for p.pos < len(p.src) && xmlIsNameChar(p.src[p.pos]) {
		p.pos++
	}
	return string(p.src[start:p.pos]), true
}

func (p *xmlParser) skipPI() {
	if p.matchLit("<?") {
		for p.pos+1 < len(p.src) {
			if p.src[p.pos] == '?' && p.src[p.pos+1] == '>' {
				p.pos += 2
				return
			}
			p.pos++
		}
		p.setError("unterminated processing instruction")
	}
}

func (p *xmlParser) skipComment() {
	if p.matchLit("<!--") {
		for p.pos+2 < len(p.src) {
			if p.src[p.pos] == '-' && p.src[p.pos+1] == '-' && p.src[p.pos+2] == '>' {
				p.pos += 3
				return
			}
			p.pos++
		}
		p.setError("unterminated comment")
	}
}

func (p *xmlParser) skipDoctype() {
	if !p.matchLit("<!DOCTYPE") {
		return
	}
	inSubset := false
	for p.pos < len(p.src) {
		c := p.src[p.pos]
		switch {
		case c == '[':
			inSubset = true
		case c == ']':
			inSubset = false
		case c == '>' && !inSubset:
			p.pos++
			return
		}
		p.pos++
	}
	p.setError("unterminated DOCTYPE")
}

func (p *xmlParser) skipMisc() bool {
	p.skipWS()
	if p.pos+1 < len(p.src) && p.src[p.pos] == '<' {
		if p.src[p.pos+1] == '?' {
			p.skipPI()
			return true
		}
		if p.pos+3 < len(p.src) &&
			p.src[p.pos+1] == '!' && p.src[p.pos+2] == '-' && p.src[p.pos+3] == '-' {
			p.skipComment()
			return true
		}
		if p.pos+8 < len(p.src) &&
			p.src[p.pos+1] == '!' && string(p.src[p.pos+2:p.pos+9]) == "DOCTYPE" {
			p.skipDoctype()
			return true
		}
	}
	return false
}

func (p *xmlParser) parseAttrValue() (string, bool) {
	if p.pos >= len(p.src) {
		p.setError("expected attribute value")
		return "", false
	}
	quote := p.src[p.pos]
	if quote != '"' && quote != '\'' {
		p.setError("attribute value missing quote")
		return "", false
	}
	p.pos++
	start := p.pos
	for p.pos < len(p.src) && p.src[p.pos] != quote {
		p.pos++
	}
	if p.pos >= len(p.src) {
		p.setError("unterminated attribute value")
		return "", false
	}
	raw := p.src[start:p.pos]
	p.pos++
	return decodeEntities(raw), true
}

func (p *xmlParser) parseAttributes(attrs *[]XmlAttr) bool {
	for {
		p.skipWS()
		if p.pos >= len(p.src) {
			p.setError("unterminated start tag")
			return false
		}
		c := p.src[p.pos]
		if c == '/' || c == '>' {
			return true
		}
		name, ok := p.parseName()
		if !ok {
			return false
		}
		p.skipWS()
		if p.pos >= len(p.src) || p.src[p.pos] != '=' {
			p.setError("expected '=' in attribute")
			return false
		}
		p.pos++
		p.skipWS()
		value, ok := p.parseAttrValue()
		if !ok {
			return false
		}
		*attrs = append(*attrs, XmlAttr{Name: name, Value: value})
	}
}

func (p *xmlParser) parseCdata() (string, bool) {
	if !p.matchLit("<![CDATA[") {
		p.setError("expected CDATA")
		return "", false
	}
	start := p.pos
	for p.pos+2 < len(p.src) {
		if p.src[p.pos] == ']' && p.src[p.pos+1] == ']' && p.src[p.pos+2] == '>' {
			body := string(p.src[start:p.pos])
			p.pos += 3
			return body, true
		}
		p.pos++
	}
	p.setError("unterminated CDATA section")
	return "", false
}

func (p *xmlParser) consumeText() (string, bool) {
	start := p.pos
	for p.pos < len(p.src) && p.src[p.pos] != '<' {
		p.pos++
	}
	if p.pos == start {
		return "", true
	}
	return decodeEntities(p.src[start:p.pos]), true
}

func (p *xmlParser) parseElement(doc *XmlDoc, parent int) (int, bool) {
	if p.pos >= len(p.src) || p.src[p.pos] != '<' {
		p.setError("expected '<'")
		return -1, false
	}
	p.pos++
	tag, ok := p.parseName()
	if !ok {
		return -1, false
	}
	node := XmlNode{
		Type:        XmlNodeElement,
		Tag:         tag,
		FirstChild:  xmlNoChild,
		NextSibling: xmlNoChild,
		Parent:      parent,
	}
	if !p.parseAttributes(&node.Attrs) {
		return -1, false
	}
	nodeID := len(doc.Nodes)
	doc.Nodes = append(doc.Nodes, node)

	p.skipWS()
	if p.pos < len(p.src) && p.src[p.pos] == '/' {
		p.pos++
		if p.pos >= len(p.src) || p.src[p.pos] != '>' {
			p.setError("expected '>' after '/'")
			return -1, false
		}
		p.pos++
		return nodeID, true
	}
	if p.pos >= len(p.src) || p.src[p.pos] != '>' {
		p.setError("expected '>' to close start tag")
		return -1, false
	}
	p.pos++

	// Children + text content.
	childTail := xmlNoChild
	appendChild := func(childID int) {
		if childTail != xmlNoChild {
			doc.Nodes[childTail].NextSibling = childID
		} else {
			doc.Nodes[nodeID].FirstChild = childID
		}
		childTail = childID
	}
	for {
		// CDATA tested before generic skipMisc.
		if p.pos+8 < len(p.src) &&
			p.src[p.pos] == '<' && p.src[p.pos+1] == '!' && p.src[p.pos+2] == '[' &&
			string(p.src[p.pos+3:p.pos+9]) == "CDATA[" {
			body, ok := p.parseCdata()
			if !ok {
				return -1, false
			}
			cdataID := len(doc.Nodes)
			doc.Nodes = append(doc.Nodes, XmlNode{
				Type:        XmlNodeCdata,
				Text:        body,
				FirstChild:  xmlNoChild,
				NextSibling: xmlNoChild,
				Parent:      nodeID,
			})
			appendChild(cdataID)
			continue
		}
		if p.skipMisc() {
			continue
		}
		if p.pos >= len(p.src) {
			p.setError("unterminated element body")
			return -1, false
		}
		if p.src[p.pos] == '<' {
			if p.pos+1 < len(p.src) && p.src[p.pos+1] == '/' {
				break
			}
			childID, ok := p.parseElement(doc, nodeID)
			if !ok {
				return -1, false
			}
			appendChild(childID)
		} else {
			text, ok := p.consumeText()
			if !ok {
				return -1, false
			}
			if text != "" {
				textID := len(doc.Nodes)
				doc.Nodes = append(doc.Nodes, XmlNode{
					Type:        XmlNodePcdata,
					Text:        text,
					FirstChild:  xmlNoChild,
					NextSibling: xmlNoChild,
					Parent:      nodeID,
				})
				appendChild(textID)
			}
		}
	}
	if !p.matchLit("</") {
		p.setError("expected end tag")
		return -1, false
	}
	endTag, ok := p.parseName()
	if !ok {
		return -1, false
	}
	if endTag != doc.Nodes[nodeID].Tag {
		p.setError("end tag name mismatch")
		return -1, false
	}
	p.skipWS()
	if p.pos >= len(p.src) || p.src[p.pos] != '>' {
		p.setError("expected '>' to close end tag")
		return -1, false
	}
	p.pos++
	return nodeID, true
}

// ─── Public entry ───────────────────────────────────────────────────

// ParseXml parses src into a full DOM tree. The returned document is
// always non-nil; check IsValid() / Error to detect parse failures.
func ParseXml(src string) *XmlDoc {
	doc := &XmlDoc{Root: xmlNoChild}
	p := &xmlParser{src: []byte(src)}

	for {
		p.skipWS()
		if !p.skipMisc() {
			break
		}
	}

	if p.pos >= len(p.src) || p.src[p.pos] != '<' {
		doc.Error = "Failed to parse XML content: missing root element"
		return doc
	}
	rootID, ok := p.parseElement(doc, xmlNoChild)
	if !ok {
		errMsg := p.error
		if errMsg == "" {
			errMsg = "unknown"
		}
		doc.Error = "Failed to parse XML content: " + errMsg
		return doc
	}
	doc.Root = rootID

	for {
		p.skipWS()
		if !p.skipMisc() {
			break
		}
	}
	if p.pos != len(p.src) {
		doc.Error = "Failed to parse XML content: trailing data after root"
		doc.Root = xmlNoChild
		return doc
	}
	return doc
}

// ─── Entity decoding ────────────────────────────────────────────────

func decodeEntities(src []byte) string {
	out := make([]byte, 0, len(src))
	i := 0
	for i < len(src) {
		c := src[i]
		if c != '&' {
			out = append(out, c)
			i++
			continue
		}
		end := i + 1
		cap := i + 32
		if cap > len(src) {
			cap = len(src)
		}
		for end < cap && src[end] != ';' {
			end++
		}
		if end >= cap || src[end] != ';' {
			out = append(out, '&')
			i++
			continue
		}
		name := src[i+1 : end]
		matched := false
		switch {
		case len(name) >= 2 && name[0] == '#':
			var radix int
			var digits []byte
			if name[1] == 'x' || name[1] == 'X' {
				radix = 16
				digits = name[2:]
			} else {
				radix = 10
				digits = name[1:]
			}
			if cp, err := strconv.ParseInt(string(digits), radix, 64); err == nil && cp >= 0 && cp < 0x110000 {
				out = utf8AppendCodepoint(out, rune(cp))
				matched = true
			}
		case string(name) == "lt":
			out = append(out, '<')
			matched = true
		case string(name) == "gt":
			out = append(out, '>')
			matched = true
		case string(name) == "amp":
			out = append(out, '&')
			matched = true
		case string(name) == "quot":
			out = append(out, '"')
			matched = true
		case string(name) == "apos":
			out = append(out, '\'')
			matched = true
		}
		if matched {
			i = end + 1
		} else {
			out = append(out, '&')
			i++
		}
	}
	return string(out)
}

func utf8AppendCodepoint(buf []byte, cp rune) []byte {
	if cp < 0 || cp >= 0x110000 {
		return buf
	}
	switch {
	case cp < 0x80:
		return append(buf, byte(cp))
	case cp < 0x800:
		return append(buf,
			byte(0xC0|(cp>>6)),
			byte(0x80|(cp&0x3F)))
	case cp < 0x10000:
		return append(buf,
			byte(0xE0|(cp>>12)),
			byte(0x80|((cp>>6)&0x3F)),
			byte(0x80|(cp&0x3F)))
	default:
		return append(buf,
			byte(0xF0|(cp>>18)),
			byte(0x80|((cp>>12)&0x3F)),
			byte(0x80|((cp>>6)&0x3F)),
			byte(0x80|(cp&0x3F)))
	}
}
