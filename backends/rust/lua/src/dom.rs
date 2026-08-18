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

    /// cpp `XMLElement::hasAttribute` — DOM Level 2 Core's answer to the
    /// ambiguity in [`Self::get_attribute`], which cannot tell an absent
    /// attribute from one present and empty.
    pub fn has_attribute(&self, node_id: usize, name: &str) -> bool {
        match self.nodes.get(node_id) {
            Some(node) => node.attrs.iter().any(|(k, _)| k == name),
            None => false,
        }
    }

    /// The node's children in document order — DOM Level 1 Core's
    /// `Node.childNodes`.
    pub fn child_ids(&self, node_id: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut child = self.nodes.get(node_id).and_then(|n| n.first_child);
        while let Some(c) = child {
            out.push(c);
            child = self.nodes[c].next_sibling;
        }
        out
    }

    /// `Node.lastChild`. The arena links forward only — cpp reads
    /// pugixml's `last_child()` — so this is the walk that link
    /// direction costs.
    pub fn last_child(&self, node_id: usize) -> Option<usize> {
        let mut child = self.nodes.get(node_id).and_then(|n| n.first_child)?;
        while let Some(next) = self.nodes[child].next_sibling {
            child = next;
        }
        Some(child)
    }

    /// `Node.previousSibling`, found by walking the parent's children —
    /// the same cost, for the same reason, as [`Self::last_child`].
    pub fn previous_sibling(&self, node_id: usize) -> Option<usize> {
        let parent = self.nodes.get(node_id)?.parent?;
        let mut child = self.nodes[parent].first_child?;
        if child == node_id {
            return None;
        }
        while let Some(next) = self.nodes[child].next_sibling {
            if next == node_id {
                return Some(child);
            }
            child = next;
        }
        None
    }

    /// `Node.textContent` (DOM Level 3 Core) — every descendant
    /// character-data node's content, concatenated in document order.
    ///
    /// Element and document nodes have no `nodeValue` of their own, so
    /// this is the only way a document can read the text an element
    /// wraps. The whitespace it reports is the tree's: a run that is
    /// nothing but whitespace never became a node (pugixml
    /// `parse_default` omits `parse_ws_pcdata`), so
    /// `<books>\n  <book/>\n</books>` has no text at all rather than
    /// two runs of it.
    pub fn text_content(&self, node_id: usize) -> String {
        let mut out = String::new();
        self.append_text_content(node_id, &mut out);
        out
    }

    fn append_text_content(&self, node_id: usize, out: &mut String) {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return,
        };
        match node.node_type {
            XmlNodeType::Pcdata | XmlNodeType::Cdata => out.push_str(&node.text),
            XmlNodeType::Element => {
                let mut child = node.first_child;
                while let Some(c) = child {
                    self.append_text_content(c, out);
                    child = self.nodes[c].next_sibling;
                }
            }
        }
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

    /// A PI / comment / DOCTYPE outside an element, where whitespace is
    /// insignificant and skipping it is part of getting there.
    fn skip_misc(&mut self) -> bool {
        self.skip_ws();
        self.skip_misc_here()
    }

    /// The same three, at the position the parser already stands on.
    ///
    /// Inside an element body the whitespace belongs to the text run that
    /// follows it, so consuming it before deciding what comes next loses
    /// it: measured 2026-08-18, `<p>a <b/> c</p>` reported its last text
    /// node as `"c"` here and as `" c"` on the cpp reference backend, and
    /// `textContent` came back a character short. It was unobservable for
    /// as long as nothing could read text at all.
    fn skip_misc_here(&mut self) -> bool {
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
            // The body form: whitespace here is the text run's, not the
            // parser's to consume.
            if self.skip_misc_here() {
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
                // A run that is nothing but whitespace becomes no node:
                // `parse_ws_pcdata` is absent from pugixml's
                // `parse_default`, so the cpp reference backend's tree
                // does not have one either. It cost nothing to keep one
                // while `getElementsByTagName` was the only reader —
                // that call collects elements — and it is a divergence
                // the moment `childNodes` and `firstChild` are readable,
                // which is why the alignment lands with them.
                if !text.is_empty() && !text.trim().is_empty() {
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
    /// Any node reached from the document — an element, or a character
    /// data node handed back by `childNodes` / `firstChild`.
    Node,
}

/// DOM Level 1 Core node types, the numbers `nodeType` reports.
///
/// Four of the twelve, because four is what this surface's trees hold:
/// comments and processing instructions are dropped at parse time
/// (pugixml `parse_default` omits `parse_comments` and `parse_pi`) and
/// the rest — attributes as nodes, entities, fragments — belong to
/// interfaces this surface does not carry.
pub const NODE_TYPE_ELEMENT: i64 = 1;
pub const NODE_TYPE_TEXT: i64 = 3;
pub const NODE_TYPE_CDATA_SECTION: i64 = 4;
pub const NODE_TYPE_DOCUMENT: i64 = 9;

impl XmlRef {
    pub fn document(doc: Arc<XmlDoc>) -> Option<Self> {
        let root_id = doc.root?;
        Some(Self {
            doc,
            node_id: root_id,
            kind: XmlRefKind::Document,
        })
    }

    /// A handle on another node of the same tree, keeping the `Arc` that
    /// owns it alive.
    pub fn node_at(&self, node_id: usize) -> Self {
        Self {
            doc: self.doc.clone(),
            node_id,
            kind: XmlRefKind::Node,
        }
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        match self.kind {
            XmlRefKind::Document => self.doc.get_elements_by_tag_name(tag),
            XmlRefKind::Node => self.doc.get_elements_by_tag_name_from(self.node_id, tag),
        }
    }

    pub fn get_attribute(&self, name: &str) -> &str {
        self.doc.get_attribute(self.node_id, name)
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        self.doc.has_attribute(self.node_id, name)
    }

    pub fn get_tag_name(&self) -> &str {
        self.doc.get_tag_name(self.node_id)
    }

    // ─── DOM Level 1 Core: the Node interface's read surface ─────────
    //
    // A `Document`-kind handle answers this interface as the document it
    // is — `nodeType` 9, one child, no parent — while the three methods
    // above keep answering for its document element, which is what they
    // have always done and what the committed trees call. Those are the
    // two halves of §scxml-B-2-1's "corresponding DOM structure": the
    // variable holds the document, and the element vocabulary reaches
    // the root without a hop nobody's document writes.

    pub fn node_type(&self) -> i64 {
        if self.kind == XmlRefKind::Document {
            return NODE_TYPE_DOCUMENT;
        }
        match self.doc.nodes.get(self.node_id).map(|n| n.node_type) {
            Some(XmlNodeType::Element) | None => NODE_TYPE_ELEMENT,
            Some(XmlNodeType::Pcdata) => NODE_TYPE_TEXT,
            Some(XmlNodeType::Cdata) => NODE_TYPE_CDATA_SECTION,
        }
    }

    pub fn node_name(&self) -> String {
        match self.node_type() {
            NODE_TYPE_DOCUMENT => "#document".to_string(),
            NODE_TYPE_TEXT => "#text".to_string(),
            NODE_TYPE_CDATA_SECTION => "#cdata-section".to_string(),
            _ => self.get_tag_name().to_string(),
        }
    }

    /// `Node.nodeValue` — character data's content, and nothing for an
    /// element or the document, which DOM Level 1 Core gives null.
    pub fn node_value(&self) -> Option<String> {
        match self.node_type() {
            NODE_TYPE_TEXT | NODE_TYPE_CDATA_SECTION => self
                .doc
                .nodes
                .get(self.node_id)
                .map(|n| n.text.clone())
                .or(Some(String::new())),
            _ => None,
        }
    }

    /// `Element.tagName` — the same string SCE's own `getTagName()`
    /// returns, under the name DOM Level 1 Core gives it. Character data
    /// has no tag name; the document answers for its document element,
    /// as the method does.
    pub fn tag_name(&self) -> Option<String> {
        match self.node_type() {
            NODE_TYPE_TEXT | NODE_TYPE_CDATA_SECTION => None,
            _ => Some(self.get_tag_name().to_string()),
        }
    }

    pub fn parent_node(&self) -> Option<Self> {
        if self.kind == XmlRefKind::Document {
            return None;
        }
        match self.doc.nodes.get(self.node_id).and_then(|n| n.parent) {
            Some(parent_id) => Some(self.node_at(parent_id)),
            // The root element's parent is the document — DOM Level 1
            // Core 1.3's "Document" is the parent of its documentElement
            // — and the handle for it is the one every `<data>` variable
            // already holds.
            None => Some(Self {
                doc: self.doc.clone(),
                node_id: self.node_id,
                kind: XmlRefKind::Document,
            }),
        }
    }

    pub fn child_node_ids(&self) -> Vec<usize> {
        if self.kind == XmlRefKind::Document {
            return vec![self.node_id];
        }
        self.doc.child_ids(self.node_id)
    }

    pub fn first_child(&self) -> Option<Self> {
        if self.kind == XmlRefKind::Document {
            return Some(self.node_at(self.node_id));
        }
        let id = self.doc.nodes.get(self.node_id)?.first_child?;
        Some(self.node_at(id))
    }

    pub fn last_child(&self) -> Option<Self> {
        if self.kind == XmlRefKind::Document {
            return Some(self.node_at(self.node_id));
        }
        Some(self.node_at(self.doc.last_child(self.node_id)?))
    }

    pub fn next_sibling(&self) -> Option<Self> {
        if self.kind == XmlRefKind::Document {
            return None;
        }
        let id = self.doc.nodes.get(self.node_id)?.next_sibling?;
        Some(self.node_at(id))
    }

    pub fn previous_sibling(&self) -> Option<Self> {
        if self.kind == XmlRefKind::Document {
            return None;
        }
        Some(self.node_at(self.doc.previous_sibling(self.node_id)?))
    }

    pub fn has_child_nodes(&self) -> bool {
        if self.kind == XmlRefKind::Document {
            return true;
        }
        self.doc
            .nodes
            .get(self.node_id)
            .is_some_and(|n| n.first_child.is_some())
    }

    /// `Document.documentElement` — nothing on a node that is not the
    /// document, which is how a document can tell the two handles apart.
    pub fn document_element(&self) -> Option<Self> {
        if self.kind != XmlRefKind::Document {
            return None;
        }
        Some(self.node_at(self.node_id))
    }

    pub fn text_content(&self) -> String {
        self.doc.text_content(self.node_id)
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
            node_list(lua, this, &ids)
        });
        methods.add_method("getAttribute", |_, this, name: String| {
            Ok(this.get_attribute(&name).to_string())
        });
        methods.add_method("hasAttribute", |_, this, name: String| {
            Ok(this.has_attribute(&name))
        });
        methods.add_method("getTagName", |_, this, ()| {
            Ok(this.get_tag_name().to_string())
        });
        methods.add_method("hasChildNodes", |_, this, ()| Ok(this.has_child_nodes()));
    }

    /// The Node interface's read surface, as fields.
    ///
    /// A property is what an author writes — `d.firstChild.nodeName`,
    /// not `d.getFirstChild()` — and the frontend emits a member read as
    /// a Lua field read, so a field is what has to answer. Before these,
    /// every one of them was a nil index on this userdata.
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("nodeType", |_, this| Ok(this.node_type()));
        fields.add_field_method_get("nodeName", |_, this| Ok(this.node_name()));
        fields.add_field_method_get("nodeValue", |_, this| Ok(this.node_value()));
        fields.add_field_method_get("data", |_, this| Ok(this.node_value()));
        fields.add_field_method_get("tagName", |_, this| Ok(this.tag_name()));
        fields.add_field_method_get("textContent", |_, this| Ok(this.text_content()));
        fields.add_field_method_get("parentNode", |_, this| Ok(this.parent_node()));
        fields.add_field_method_get("firstChild", |_, this| Ok(this.first_child()));
        fields.add_field_method_get("lastChild", |_, this| Ok(this.last_child()));
        fields.add_field_method_get("nextSibling", |_, this| Ok(this.next_sibling()));
        fields.add_field_method_get("previousSibling", |_, this| Ok(this.previous_sibling()));
        fields.add_field_method_get("documentElement", |_, this| Ok(this.document_element()));
        fields.add_field_method_get("childNodes", |lua, this| {
            let ids = this.child_node_ids();
            node_list(lua, this, &ids)
        });
    }
}

/// A NodeList: the host language's own array, 1-based because the
/// frontend rewrites `[0]` to `[1]` upstream and `length` to Lua's `#`.
///
/// Every one of the seven bindings hands back its language's array for
/// the same reason, which is why `item(i)` is refused by the frontend
/// rather than implemented here — there is no receiver for it to bind.
fn node_list(lua: &mlua::Lua, this: &XmlRef, ids: &[usize]) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    for (i, &id) in ids.iter().enumerate() {
        table.raw_set(i + 1, this.node_at(id))?;
    }
    Ok(table)
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

    // ─── DOM Level 1 Core read surface ──────────────────────────────

    /// The tree the author walks has no whitespace-only text in it, so
    /// `firstChild` of a pretty-printed document is its first element.
    ///
    /// This is the pugixml `parse_default` alignment: while
    /// `getElementsByTagName` was the only reader the difference could
    /// not be seen, and `<books>\n  <book/>` would otherwise answer
    /// `firstChild.nodeName == "#text"` here and `"book"` on the cpp
    /// reference backend.
    #[test]
    fn whitespace_between_elements_is_not_a_node() {
        let doc = XmlDoc::parse("<books xmlns=\"\">\n  <book title=\"t1\"/>\n</books>");
        assert!(doc.is_valid(), "{:?}", doc.error);
        let root = XmlRef::document(Arc::new(doc)).unwrap();
        let element = root.document_element().expect("document element");
        let first = element.first_child().expect("a first child");
        assert_eq!(first.node_name(), "book");
        assert_eq!(first.node_type(), NODE_TYPE_ELEMENT);
        assert_eq!(element.child_node_ids().len(), 1);
        assert_eq!(element.text_content(), "");
    }

    /// The document handle answers the Node interface as a document and
    /// the Element vocabulary for its document element.
    #[test]
    fn the_document_handle_answers_both_interfaces() {
        let doc = XmlDoc::parse("<books count=\"2\"><book title=\"t1\"/></books>");
        let root = XmlRef::document(Arc::new(doc)).unwrap();
        assert_eq!(root.node_type(), NODE_TYPE_DOCUMENT);
        assert_eq!(root.node_name(), "#document");
        assert_eq!(root.node_value(), None);
        assert!(root.parent_node().is_none(), "a document has no parent");
        assert!(root.next_sibling().is_none());
        assert!(root.has_child_nodes());
        assert_eq!(root.child_node_ids().len(), 1);
        // The element vocabulary the three shipped methods already
        // delegated, plus the property that spells the same thing.
        assert_eq!(root.get_tag_name(), "books");
        assert_eq!(root.tag_name().as_deref(), Some("books"));
        assert_eq!(root.get_attribute("count"), "2");
        assert!(root.has_attribute("count"));
        assert!(!root.has_attribute("title"));
        let element = root.document_element().expect("document element");
        assert_eq!(element.node_type(), NODE_TYPE_ELEMENT);
        assert!(
            element.document_element().is_none(),
            "only the document handle carries documentElement"
        );
    }

    /// Character data reports itself as DOM Level 1 Core does, and the
    /// two kinds are distinguishable — which is what `nodeType` is for.
    #[test]
    fn character_data_reports_its_own_kind() {
        let doc = XmlDoc::parse("<p>before<b>bold</b><![CDATA[raw & <kept>]]></p>");
        assert!(doc.is_valid(), "{:?}", doc.error);
        let root = XmlRef::document(Arc::new(doc)).unwrap();
        let p = root.document_element().unwrap();
        let text = p.first_child().unwrap();
        assert_eq!(text.node_type(), NODE_TYPE_TEXT);
        assert_eq!(text.node_name(), "#text");
        assert_eq!(text.node_value().as_deref(), Some("before"));
        assert_eq!(text.tag_name(), None, "character data has no tag name");
        assert!(!text.has_child_nodes());

        let bold = text.next_sibling().unwrap();
        assert_eq!(bold.node_name(), "b");
        assert_eq!(bold.text_content(), "bold");
        assert_eq!(
            bold.previous_sibling().map(|n| n.node_name()),
            Some("#text".to_string())
        );

        let cdata = p.last_child().unwrap();
        assert_eq!(cdata.node_type(), NODE_TYPE_CDATA_SECTION);
        assert_eq!(cdata.node_name(), "#cdata-section");
        assert_eq!(cdata.node_value().as_deref(), Some("raw & <kept>"));
        assert!(cdata.next_sibling().is_none());

        // §DOM-3 textContent is every descendant's character data in
        // document order, CDATA included.
        assert_eq!(p.text_content(), "beforeboldraw & <kept>");
        // The root element's parent is the document, not nothing.
        assert_eq!(
            p.parent_node().map(|n| n.node_type()),
            Some(NODE_TYPE_DOCUMENT)
        );
        assert_eq!(
            bold.parent_node().map(|n| n.node_name()),
            Some("p".to_string())
        );
    }

    /// A node handle keeps its tree alive on its own.
    ///
    /// The `Arc` is what makes that true here; the cpp reference backend
    /// stores only the element in `DOMObjectData` / `LuaDOMElementUD`,
    /// which is the same shape without the ownership.
    #[test]
    fn a_node_handle_outlives_the_handle_it_came_from() {
        let leaf = {
            let doc = XmlDoc::parse("<root><leaf title=\"t\"/></root>");
            let root = XmlRef::document(Arc::new(doc)).unwrap();
            let ids = root.get_elements_by_tag_name("leaf");
            root.node_at(ids[0])
        };
        assert_eq!(leaf.node_name(), "leaf");
        assert_eq!(leaf.get_attribute("title"), "t");
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
