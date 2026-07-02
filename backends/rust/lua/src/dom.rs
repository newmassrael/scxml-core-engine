// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! W3C SCXML B.2 — XML DOM tree for the Rust Lua backend.
//!
//! 1:1 algorithmic mirror of `sce/include/scripting/XMLDOMWrapper.h` and
//! `sce/src/scripting/XMLDOMWrapper.cpp` (cpp ref-backend, pugixml-based),
//! reimplemented in Rust without the third_party `pugi::xml_*` C++ types.
//! Coverage matches the cpp `parse_default` feature set:
//!
//! * paired `<tag>...</tag>` + self-close `<tag/>`
//! * `attr="value"` and `attr='value'` both quote styles
//! * `xmlns=""` / `xmlns:x=""` as regular attributes (no namespace
//!   prefix processing — pugixml's default)
//! * named entity refs `&amp;` / `&lt;` / `&gt;` / `&quot;` / `&apos;`
//!   and numeric refs `&#N;` / `&#xN;` (UTF-8 encoded), in attribute
//!   values and text content
//! * `<?xml ?>` PI prologue + `<!-- comment -->` skip (anywhere)
//! * `<!DOCTYPE ...>` skip (with optional internal subset `[...]`)
//! * `<![CDATA[...]]>` content as a CDATA node child
//! * mixed text content as PCDATA node children
//!
//! Storage uses an arena (`Vec<XmlNode>` + index-based pointers) so the
//! whole tree is `Send + Sync` and ownership is the document's alone.
//! `getElementsByTagName` returns `Vec<usize>` of element node ids; the
//! Lua binding wraps each id in an `XmlElementRef { doc: Arc<XmlDoc>,
//! node_id }` userdata so the document keeps the tree alive for as long
//! as any element ref survives — cpp's `shared_ptr<XMLElement>` semantics.

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlNodeType {
    Element,
    Pcdata,
    Cdata,
}

#[derive(Debug, Clone)]
pub struct XmlNode {
    pub node_type: XmlNodeType,
    pub tag: String,                  // element: tag name; else empty
    pub text: String,                 // pcdata/cdata: content; else empty
    pub attrs: Vec<(String, String)>, // ordered, key/value pairs
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
    pub parent: Option<usize>,
}

impl XmlNode {
    fn new(node_type: XmlNodeType) -> Self {
        Self {
            node_type,
            tag: String::new(),
            text: String::new(),
            attrs: Vec::new(),
            first_child: None,
            next_sibling: None,
            parent: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct XmlDoc {
    pub nodes: Vec<XmlNode>,
    pub root: Option<usize>,
    pub error: Option<String>,
}

impl XmlDoc {
    pub fn parse(src: &str) -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            root: None,
            error: None,
        };
        let bytes = src.as_bytes();
        let mut p = Parser {
            src: bytes,
            pos: 0,
            error: None,
        };

        // Optional prologue: PI / comment / DOCTYPE / whitespace.
        loop {
            p.skip_ws();
            if !p.skip_misc() {
                break;
            }
        }

        if p.pos >= p.src.len() || p.src[p.pos] != b'<' {
            doc.error = Some("Failed to parse XML content: missing root element".to_string());
            return doc;
        }
        match p.parse_element(&mut doc, None) {
            Some(root_id) => {
                doc.root = Some(root_id);
            }
            None => {
                doc.error = Some(format!(
                    "Failed to parse XML content: {}",
                    p.error.unwrap_or_else(|| "unknown".to_string())
                ));
                return doc;
            }
        }

        loop {
            p.skip_ws();
            if !p.skip_misc() {
                break;
            }
        }
        if p.pos != p.src.len() {
            doc.error = Some("Failed to parse XML content: trailing data after root".to_string());
            doc.root = None;
        }

        doc
    }

    pub fn is_valid(&self) -> bool {
        self.root.is_some() && self.error.is_none()
    }

    /// cpp `XMLDocument::getElementsByTagName` — recursive find from
    /// root (root included).
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        let mut out = Vec::new();
        if let Some(root_id) = self.root {
            self.collect(root_id, tag, &mut out);
        }
        out
    }

    /// cpp `XMLElement::getElementsByTagName` — recursive descent from
    /// each child (self not matched).
    pub fn get_elements_by_tag_name_from(&self, node_id: usize, tag: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let mut child = self.nodes.get(node_id).and_then(|n| n.first_child);
        while let Some(c) = child {
            self.collect(c, tag, &mut out);
            child = self.nodes[c].next_sibling;
        }
        out
    }

    /// cpp `XMLElement::getAttribute` — linear lookup, returns "" on miss
    /// (matches cpp's `node_.attribute(...)` empty-attr behaviour).
    pub fn get_attribute(&self, node_id: usize, name: &str) -> &str {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return "",
        };
        for (k, v) in &node.attrs {
            if k == name {
                return v;
            }
        }
        ""
    }

    pub fn get_tag_name(&self, node_id: usize) -> &str {
        self.nodes
            .get(node_id)
            .map(|n| n.tag.as_str())
            .unwrap_or("")
    }

    fn collect(&self, node_id: usize, tag: &str, out: &mut Vec<usize>) {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return,
        };
        if node.node_type == XmlNodeType::Element && node.tag == tag {
            out.push(node_id);
        }
        let mut c = node.first_child;
        while let Some(child_id) = c {
            self.collect(child_id, tag, out);
            c = self.nodes[child_id].next_sibling;
        }
    }
}

// ─── Parser ─────────────────────────────────────────────────────────

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    error: Option<String>,
}

impl Parser<'_> {
    fn set_error(&mut self, msg: &str) {
        if self.error.is_none() {
            self.error = Some(format!("{} (at byte {})", msg, self.pos));
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn match_lit(&mut self, lit: &[u8]) -> bool {
        if self.pos + lit.len() > self.src.len() {
            return false;
        }
        if &self.src[self.pos..self.pos + lit.len()] != lit {
            return false;
        }
        self.pos += lit.len();
        true
    }

    fn is_name_start(c: u8) -> bool {
        c == b'_' || c == b':' || c.is_ascii_alphabetic()
    }

    fn is_name_char(c: u8) -> bool {
        Self::is_name_start(c) || c == b'-' || c == b'.' || c.is_ascii_digit()
    }

    fn parse_name(&mut self) -> Option<String> {
        self.skip_ws();
        if self.pos >= self.src.len() || !Self::is_name_start(self.src[self.pos]) {
            self.set_error("expected name");
            return None;
        }
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.src.len() && Self::is_name_char(self.src[self.pos]) {
            self.pos += 1;
        }
        std::str::from_utf8(&self.src[start..self.pos])
            .ok()
            .map(|s| s.to_string())
    }

    fn skip_pi(&mut self) {
        if self.match_lit(b"<?") {
            while self.pos + 1 < self.src.len() {
                if self.src[self.pos] == b'?' && self.src[self.pos + 1] == b'>' {
                    self.pos += 2;
                    return;
                }
                self.pos += 1;
            }
            self.set_error("unterminated processing instruction");
        }
    }

    fn skip_comment(&mut self) {
        if self.match_lit(b"<!--") {
            while self.pos + 2 < self.src.len() {
                if self.src[self.pos] == b'-'
                    && self.src[self.pos + 1] == b'-'
                    && self.src[self.pos + 2] == b'>'
                {
                    self.pos += 3;
                    return;
                }
                self.pos += 1;
            }
            self.set_error("unterminated comment");
        }
    }

    fn skip_doctype(&mut self) {
        if !self.match_lit(b"<!DOCTYPE") {
            return;
        }
        let mut in_subset = false;
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c == b'[' {
                in_subset = true;
            } else if c == b']' {
                in_subset = false;
            } else if c == b'>' && !in_subset {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
        self.set_error("unterminated DOCTYPE");
    }

    fn skip_misc(&mut self) -> bool {
        self.skip_ws();
        if self.pos + 1 < self.src.len() && self.src[self.pos] == b'<' {
            if self.src[self.pos + 1] == b'?' {
                self.skip_pi();
                return true;
            }
            if self.pos + 3 < self.src.len()
                && self.src[self.pos + 1] == b'!'
                && self.src[self.pos + 2] == b'-'
                && self.src[self.pos + 3] == b'-'
            {
                self.skip_comment();
                return true;
            }
            if self.pos + 8 < self.src.len()
                && self.src[self.pos + 1] == b'!'
                && &self.src[self.pos + 2..self.pos + 9] == b"DOCTYPE"
            {
                self.skip_doctype();
                return true;
            }
        }
        false
    }

    fn parse_attr_value(&mut self) -> Option<String> {
        if self.pos >= self.src.len() {
            self.set_error("expected attribute value");
            return None;
        }
        let quote = self.src[self.pos];
        if quote != b'"' && quote != b'\'' {
            self.set_error("attribute value missing quote");
            return None;
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos] != quote {
            self.pos += 1;
        }
        if self.pos >= self.src.len() {
            self.set_error("unterminated attribute value");
            return None;
        }
        let raw = &self.src[start..self.pos];
        self.pos += 1; // consume closing quote
        decode_entities(raw)
    }

    fn parse_attributes(&mut self, attrs: &mut Vec<(String, String)>) -> bool {
        loop {
            self.skip_ws();
            if self.pos >= self.src.len() {
                self.set_error("unterminated start tag");
                return false;
            }
            let c = self.src[self.pos];
            if c == b'/' || c == b'>' {
                return true;
            }
            let name = match self.parse_name() {
                Some(n) => n,
                None => return false,
            };
            self.skip_ws();
            if self.pos >= self.src.len() || self.src[self.pos] != b'=' {
                self.set_error("expected '=' in attribute");
                return false;
            }
            self.pos += 1;
            self.skip_ws();
            let value = match self.parse_attr_value() {
                Some(v) => v,
                None => return false,
            };
            attrs.push((name, value));
        }
    }

    fn parse_cdata(&mut self) -> Option<String> {
        if !self.match_lit(b"<![CDATA[") {
            self.set_error("expected CDATA");
            return None;
        }
        let start = self.pos;
        while self.pos + 2 < self.src.len() {
            if self.src[self.pos] == b']'
                && self.src[self.pos + 1] == b']'
                && self.src[self.pos + 2] == b'>'
            {
                let body = std::str::from_utf8(&self.src[start..self.pos])
                    .ok()
                    .map(|s| s.to_string())?;
                self.pos += 3;
                return Some(body);
            }
            self.pos += 1;
        }
        self.set_error("unterminated CDATA section");
        None
    }

    fn consume_text(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos] != b'<' {
            self.pos += 1;
        }
        if self.pos == start {
            return Some(String::new());
        }
        decode_entities(&self.src[start..self.pos])
    }

    fn parse_element(&mut self, doc: &mut XmlDoc, parent: Option<usize>) -> Option<usize> {
        if self.pos >= self.src.len() || self.src[self.pos] != b'<' {
            self.set_error("expected '<'");
            return None;
        }
        self.pos += 1;
        let tag = self.parse_name()?;
        let mut node = XmlNode::new(XmlNodeType::Element);
        node.tag = tag;
        node.parent = parent;
        if !self.parse_attributes(&mut node.attrs) {
            return None;
        }
        let node_id = doc.nodes.len();
        doc.nodes.push(node);

        self.skip_ws();
        if self.pos < self.src.len() && self.src[self.pos] == b'/' {
            self.pos += 1;
            if self.pos >= self.src.len() || self.src[self.pos] != b'>' {
                self.set_error("expected '>' after '/'");
                return None;
            }
            self.pos += 1;
            return Some(node_id);
        }
        if self.pos >= self.src.len() || self.src[self.pos] != b'>' {
            self.set_error("expected '>' to close start tag");
            return None;
        }
        self.pos += 1;

        // Children + text content.
        let mut child_tail: Option<usize> = None;
        let append_child = |doc: &mut XmlDoc, child_id: usize, tail: &mut Option<usize>| {
            if let Some(t) = *tail {
                doc.nodes[t].next_sibling = Some(child_id);
            } else {
                doc.nodes[node_id].first_child = Some(child_id);
            }
            *tail = Some(child_id);
        };
        loop {
            // CDATA tested before generic skip_misc.
            if self.pos + 8 < self.src.len()
                && self.src[self.pos] == b'<'
                && self.src[self.pos + 1] == b'!'
                && self.src[self.pos + 2] == b'['
                && &self.src[self.pos + 3..self.pos + 9] == b"CDATA["
            {
                let body = self.parse_cdata()?;
                let mut cdata_node = XmlNode::new(XmlNodeType::Cdata);
                cdata_node.text = body;
                cdata_node.parent = Some(node_id);
                let cdata_id = doc.nodes.len();
                doc.nodes.push(cdata_node);
                append_child(doc, cdata_id, &mut child_tail);
                continue;
            }
            if self.skip_misc() {
                continue;
            }
            if self.pos >= self.src.len() {
                self.set_error("unterminated element body");
                return None;
            }
            if self.src[self.pos] == b'<' {
                if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'/' {
                    break;
                }
                let child_id = self.parse_element(doc, Some(node_id))?;
                append_child(doc, child_id, &mut child_tail);
            } else {
                let text = self.consume_text()?;
                if !text.is_empty() {
                    let mut text_node = XmlNode::new(XmlNodeType::Pcdata);
                    text_node.text = text;
                    text_node.parent = Some(node_id);
                    let text_id = doc.nodes.len();
                    doc.nodes.push(text_node);
                    append_child(doc, text_id, &mut child_tail);
                }
            }
        }
        if !self.match_lit(b"</") {
            self.set_error("expected end tag");
            return None;
        }
        let end_tag = self.parse_name()?;
        if end_tag != doc.nodes[node_id].tag {
            self.set_error("end tag name mismatch");
            return None;
        }
        self.skip_ws();
        if self.pos >= self.src.len() || self.src[self.pos] != b'>' {
            self.set_error("expected '>' to close end tag");
            return None;
        }
        self.pos += 1;
        Some(node_id)
    }
}

// ─── Entity decoding ────────────────────────────────────────────────

fn utf8_encode(codepoint: u32, out: &mut String) -> bool {
    if let Some(c) = char::from_u32(codepoint) {
        out.push(c);
        true
    } else {
        false
    }
}

fn decode_entities(src: &[u8]) -> Option<String> {
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        if c != b'&' {
            // Append byte as ASCII-friendly char; decode_entities is
            // only ever called on UTF-8 input slices, so out remains
            // valid UTF-8.
            out.push(c as char);
            i += 1;
            continue;
        }
        let mut end = i + 1;
        let cap = (i + 32).min(src.len());
        while end < cap && src[end] != b';' {
            end += 1;
        }
        if end >= cap || src[end] != b';' {
            out.push('&');
            i += 1;
            continue;
        }
        let name = &src[i + 1..end];
        let mut matched = false;
        if name.len() >= 2 && name[0] == b'#' {
            let (radix, digits): (u32, &[u8]) = if name[1] == b'x' || name[1] == b'X' {
                (16, &name[2..])
            } else {
                (10, &name[1..])
            };
            if let Ok(s) = std::str::from_utf8(digits) {
                if let Ok(cp) = u32::from_str_radix(s, radix) {
                    if utf8_encode(cp, &mut out) {
                        matched = true;
                    }
                }
            }
        } else if name == b"lt" {
            out.push('<');
            matched = true;
        } else if name == b"gt" {
            out.push('>');
            matched = true;
        } else if name == b"amp" {
            out.push('&');
            matched = true;
        } else if name == b"quot" {
            out.push('"');
            matched = true;
        } else if name == b"apos" {
            out.push('\'');
            matched = true;
        }
        if matched {
            i = end + 1;
        } else {
            out.push('&');
            i += 1;
        }
    }
    Some(out)
}

// ─── Lua-side ref ───────────────────────────────────────────────────
//
// Lua-facing handle.  Holds an `Arc<XmlDoc>` so the document outlives
// any reference, plus the document-relative node_id.  When `kind ==
// Document` the binding methods treat the underlying root specially —
// `XMLDocument::getElementsByTagName` matches the root inclusively,
// `XMLElement::getElementsByTagName` only descends into children.

#[derive(Clone)]
pub struct XmlRef {
    pub doc: Arc<XmlDoc>,
    pub node_id: usize,
    pub kind: XmlRefKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum XmlRefKind {
    Document,
    Element,
}

impl XmlRef {
    pub fn document(doc: Arc<XmlDoc>) -> Option<Self> {
        let root_id = doc.root?;
        Some(Self {
            doc,
            node_id: root_id,
            kind: XmlRefKind::Document,
        })
    }

    pub fn child_element(&self, node_id: usize) -> Self {
        Self {
            doc: self.doc.clone(),
            node_id,
            kind: XmlRefKind::Element,
        }
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        match self.kind {
            XmlRefKind::Document => self.doc.get_elements_by_tag_name(tag),
            XmlRefKind::Element => self.doc.get_elements_by_tag_name_from(self.node_id, tag),
        }
    }

    pub fn get_attribute(&self, name: &str) -> &str {
        self.doc.get_attribute(self.node_id, name)
    }

    pub fn get_tag_name(&self) -> &str {
        self.doc.get_tag_name(self.node_id)
    }
}

// ─── mlua UserData binding ──────────────────────────────────────────
//
// `XmlRef` is exposed to Lua as userdata so the wrapped `Arc<XmlDoc>`
// owns the tree until every Lua-side reference is GC'd — cpp's
// `LuaDOMBinding::pushDOMObject` + `pushElementObject` use shared_ptr
// to achieve the same lifetime.  The three methods mirror cpp
// `XMLElement::getElementsByTagName` / `getAttribute` / `getTagName`
// 1:1; `getElementsByTagName` returns a 1-based Lua array (the
// EcmaScript-to-Lua transformer rewrites `[0]` → `[1]` upstream).

impl mlua::UserData for XmlRef {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getElementsByTagName", |lua, this, tag: String| {
            let ids = this.get_elements_by_tag_name(&tag);
            let table = lua.create_table()?;
            for (i, &id) in ids.iter().enumerate() {
                let elem = this.child_element(id);
                table.raw_set(i + 1, elem)?;
            }
            Ok(table)
        });
        methods.add_method("getAttribute", |_, this, name: String| {
            Ok(this.get_attribute(&name).to_string())
        });
        methods.add_method("getTagName", |_, this, ()| {
            Ok(this.get_tag_name().to_string())
        });
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_and_self_close() {
        let doc = XmlDoc::parse("<root><a/><b>x</b></root>");
        assert!(doc.is_valid(), "valid doc, got error: {:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(xref.get_tag_name(), "root");
        let books = xref.get_elements_by_tag_name("b");
        assert_eq!(books.len(), 1);
        let leaves = xref.get_elements_by_tag_name("a");
        assert_eq!(leaves.len(), 1);
    }

    #[test]
    fn doctype_prologue() {
        let xml = "<?xml version=\"1.0\"?><!DOCTYPE root SYSTEM \"r.dtd\"><root><leaf/></root>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "valid after DOCTYPE: {:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(xref.get_tag_name(), "root");
    }

    #[test]
    fn doctype_internal_subset() {
        let xml = "<!DOCTYPE root [ <!ELEMENT root (leaf*)> ]><root><leaf/></root>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "valid: {:?}", doc.error);
    }

    #[test]
    fn cdata_section() {
        let xml = "<root><leaf><![CDATA[ <not-a-tag> & </not-a-tag> ]]></leaf></root>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        let leaves = xref.get_elements_by_tag_name("leaf");
        assert_eq!(leaves.len(), 1);
        let leaf_id = leaves[0];
        let cdata_id = xref.doc.nodes[leaf_id].first_child.unwrap();
        let cdata = &xref.doc.nodes[cdata_id];
        assert_eq!(cdata.node_type, XmlNodeType::Cdata);
        assert_eq!(cdata.text, " <not-a-tag> & </not-a-tag> ");
    }

    #[test]
    fn named_entities_in_attribute() {
        let doc = XmlDoc::parse("<root attr=\"&amp;&lt;&gt;&quot;&apos;\"/>");
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(xref.get_attribute("attr"), "&<>\"'");
    }

    #[test]
    fn numeric_entities_in_attribute() {
        // 'A'=65, 'B'=0x42, '€'=U+20AC.
        let doc = XmlDoc::parse("<root attr=\"&#65;&#x42;&#x20AC;\"/>");
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(xref.get_attribute("attr"), "AB€");
    }

    #[test]
    fn mixed_text_pcdata_with_entity() {
        let xml = "<root>before<inner/>after &amp; tail</root>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        // root → first_child = "before" PCDATA
        let root_id = xref.node_id;
        let first = xref.doc.nodes[root_id].first_child.unwrap();
        assert_eq!(xref.doc.nodes[first].node_type, XmlNodeType::Pcdata);
        assert_eq!(xref.doc.nodes[first].text, "before");
        let inner = xref.doc.nodes[first].next_sibling.unwrap();
        assert_eq!(xref.doc.nodes[inner].node_type, XmlNodeType::Element);
        assert_eq!(xref.doc.nodes[inner].tag, "inner");
        let trailing = xref.doc.nodes[inner].next_sibling.unwrap();
        assert_eq!(xref.doc.nodes[trailing].node_type, XmlNodeType::Pcdata);
        assert_eq!(xref.doc.nodes[trailing].text, "after & tail");
    }

    #[test]
    fn comment_in_element_body() {
        let doc = XmlDoc::parse("<root><a/><!-- ignore --><b/></root>");
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(xref.get_elements_by_tag_name("a").len(), 1);
        assert_eq!(xref.get_elements_by_tag_name("b").len(), 1);
    }

    #[test]
    fn get_elements_skips_text_nodes() {
        let xml = "<root>t1<book title=\"a\"/>t2<![CDATA[raw]]><book title=\"b\"/></root>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        let books = xref.get_elements_by_tag_name("book");
        assert_eq!(books.len(), 2);
        assert_eq!(xref.doc.get_attribute(books[0], "title"), "a");
        assert_eq!(xref.doc.get_attribute(books[1], "title"), "b");
    }

    #[test]
    fn w3c_corpus_test557_inline_books() {
        // Mirrors the test557 corpus body so this module is anchored to
        // the W3C SCXML B.2 fixture's input.
        let xml =
            "<books xmlns=\"\">\n  <book title=\"title1\"/>\n  <book title=\"title2\"/>\n</books>";
        let doc = XmlDoc::parse(xml);
        assert!(doc.is_valid(), "{:?}", doc.error);
        let xref = XmlRef::document(Arc::new(doc)).unwrap();
        let books = xref.get_elements_by_tag_name("book");
        assert_eq!(books.len(), 2);
        assert_eq!(xref.doc.get_attribute(books[0], "title"), "title1");
        assert_eq!(xref.doc.get_attribute(books[1], "title"), "title2");
    }
}
