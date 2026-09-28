// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a parse read of a document, so what it did not read can be refused
//! rather than ignored.
//!
//! # Why
//!
//! A reader reaches what it understands by asking for it by name, and
//! something written where nobody asks is never seen. The schema cannot
//! catch either kind:
//!
//! * **SCE elements.** A kind parser asks a parent for a child by name —
//!   the root for `datamodel`, a `<datamodel>` for its `<sce:helper>`s —
//!   and the `<scxml>` root admits SCE elements laxly, because one root
//!   element serves seventeen kinds. So a `<sce:helper>` written under the
//!   procedure root was dropped without a word, and the refusal that
//!   followed blamed the call that named it (measured 2026-09-24).
//! * **`sce:` attributes on W3C elements.** `schemas/sce-forge.xsd` takes
//!   them as `<xs:anyAttribute namespace="##other" processContents="lax"/>`,
//!   and `lax` validates an attribute only when a global declaration names
//!   it. XSD 1.0 cannot say "this global attribute may sit on these
//!   elements", so a name that exists nowhere, or a real name on an element
//!   that does not take it, passed. Measured 2026-09-28: `sce:req` on a
//!   forge root, `sce:unresolved` on a `<data>` and on an `<onentry>`, and
//!   an invented `sce:totally-invented` all built with exit 0.
//!
//! # How
//!
//! While a document is parsed, the readers record:
//!
//! * **asked** — the SCE element names a reader asked a parent for, found
//!   or not;
//! * **taken** — every element an accessor handed to a reader;
//! * **closed** — a parent whose every child a reader judged itself (a loop
//!   that refuses what it does not take), so nothing under it is left over;
//! * **delegated** — a subtree handed whole to another reader;
//! * **attributes** — every `sce:` attribute a reader asked for, through
//!   the one channel [`crate::sce_attr`], including one it consulted and
//!   left unread on purpose ([`crate::sce_attr::acknowledge`]).
//!
//! [`refuse_unread`] then refuses the first SCE element that nothing took,
//! naming what its parent was asked for as the alternatives, and
//! [`unread_sce_attributes`] answers with every `sce:` attribute on a W3C
//! element that nothing asked for. Nothing here is a list of where elements
//! or attributes belong: the parsers' own reads are the list, so a kind
//! added tomorrow is covered by the reads it makes.
//!
//! The ledger is scoped to one parse on one thread ([`Recording`]); the
//! readers record into the innermost open recording of the same document
//! and do nothing when none is open, so a reader outside a parse — a test, a
//! tool reading a fragment — is unaffected. A recording nested inside
//! another on the SAME document — an inline forge kind, which is a `<data>`
//! of the statechart around it — hands the attributes it recorded, and the
//! subtrees it delegated, to the enclosing recording when it ends, because the statechart's check covers
//! the inline kind's elements too and a read is a read whichever reader
//! made it.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};

use roxmltree::{Node, NodeId};

use crate::forge::error::{ForgeError, Located};
use crate::forge::model::SCE_NAMESPACE;

/// What one parse read of one document.
#[derive(Debug, Default)]
pub(crate) struct Ledger {
    document: usize,
    asked: HashMap<NodeId, BTreeSet<String>>,
    taken: HashSet<NodeId>,
    closed: HashSet<NodeId>,
    delegated: HashSet<NodeId>,
    /// Byte offsets of the `sce:` attributes asked for — an attribute's
    /// identity within its document.
    attributes: HashSet<usize>,
}

thread_local! {
    static OPEN: RefCell<Vec<Ledger>> = const { RefCell::new(Vec::new()) };
}

fn document_of(node: &Node) -> usize {
    node.document() as *const _ as usize
}

/// Apply `record` to the innermost open ledger for `node`'s document.
fn with_ledger(node: &Node, record: impl FnOnce(&mut Ledger)) {
    let document = document_of(node);
    OPEN.with(|open| {
        if let Some(ledger) = open
            .borrow_mut()
            .iter_mut()
            .rev()
            .find(|ledger| ledger.document == document)
        {
            record(ledger);
        }
    });
}

/// A ledger open for one parse. Dropping it without [`Self::finish`] — a
/// parse that returned early with a refusal — discards what it recorded.
pub(crate) struct Recording {
    depth: usize,
    finished: bool,
}

impl Recording {
    /// Start recording what is read of the document `root` belongs to.
    pub(crate) fn open(root: &Node) -> Self {
        let depth = OPEN.with(|open| {
            let mut open = open.borrow_mut();
            open.push(Ledger {
                document: document_of(root),
                ..Ledger::default()
            });
            open.len()
        });
        Self {
            depth,
            finished: false,
        }
    }

    /// Stop recording and hand back what was read.
    pub(crate) fn finish(mut self) -> Ledger {
        self.finished = true;
        close(self.depth)
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if !self.finished {
            close(self.depth);
        }
    }
}

/// Pop the innermost ledger, handing its attribute reads to the next
/// enclosing ledger of the same document (see the module header).
fn close(depth: usize) -> Ledger {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        debug_assert_eq!(open.len(), depth, "recordings close innermost first");
        let ledger = open.pop().expect("an open recording");
        if let Some(outer) = open
            .iter_mut()
            .rev()
            .find(|outer| outer.document == ledger.document)
        {
            outer.attributes.extend(ledger.attributes.iter().copied());
            outer.delegated.extend(ledger.delegated.iter().copied());
        }
        ledger
    })
}

/// A reader asked `parent` for its SCE child named `local`.
pub(crate) fn asked(parent: &Node, local: &str) {
    let id = parent.id();
    with_ledger(parent, |ledger| {
        ledger
            .asked
            .entry(id)
            .or_default()
            .insert(local.to_string());
    });
}

/// An accessor handed `node` to a reader.
pub(crate) fn taken(node: &Node) {
    let id = node.id();
    with_ledger(node, |ledger| {
        ledger.taken.insert(id);
    });
}

/// A reader judged every child of `parent` itself.
pub(crate) fn closed(parent: &Node) {
    let id = parent.id();
    with_ledger(parent, |ledger| {
        ledger.closed.insert(id);
    });
}

/// `node`'s subtree was handed whole to another reader.
pub(crate) fn delegated(node: &Node) {
    let id = node.id();
    with_ledger(node, |ledger| {
        ledger.delegated.insert(id);
    });
}

/// A reader asked for `attribute`, which sits on `node`. Only
/// [`crate::sce_attr`] calls this, so every `sce:` read is one shape.
pub(crate) fn attribute_read(node: &Node, attribute: &roxmltree::Attribute) {
    let offset = attribute.range().start;
    with_ledger(node, |ledger| {
        ledger.attributes.insert(offset);
    });
}

/// The refusal of the first SCE element under `root`, in document order,
/// that nothing read: not taken, under a parent no reader closed, and outside
/// every delegated subtree. `refuse` builds it from the element, its parent,
/// and the SCE names the parent was asked for.
pub(crate) fn refuse_unread(
    ledger: &Ledger,
    root: &Node,
    refuse: impl Fn(&Node, &Node, &[&str]) -> Located<ForgeError>,
) -> Result<(), Located<ForgeError>> {
    let mut pending: Vec<Node> = root.children().filter(Node::is_element).collect();
    pending.reverse();
    while let Some(node) = pending.pop() {
        if ledger.delegated.contains(&node.id()) {
            continue;
        }
        let parent = node
            .parent_element()
            .expect("an element below the root has a parent");
        let is_sce = node.tag_name().namespace() == Some(SCE_NAMESPACE);
        if is_sce && !ledger.taken.contains(&node.id()) && !ledger.closed.contains(&parent.id()) {
            let asked: Vec<&str> = ledger
                .asked
                .get(&parent.id())
                .map(|names| names.iter().map(String::as_str).collect())
                .unwrap_or_default();
            return Err(refuse(&node, &parent, &asked));
        }
        let mut children: Vec<Node> = node.children().filter(Node::is_element).collect();
        children.reverse();
        pending.extend(children);
    }
    Ok(())
}

/// An `sce:` attribute a W3C element carries that the parse never asked
/// for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unread {
    /// The element's local name, as the author wrote it.
    pub element: String,
    /// The attribute's name as the author wrote it, prefix included — the
    /// record's `actual` (SCE_ERROR_CONTRACT.md §2.1).
    pub attribute: String,
    /// 1-based row and column of the attribute.
    pub row: u32,
    pub col: u32,
}

/// The `sce:` attributes on the W3C-namespace elements at and below `root`
/// that nothing asked for, in document order.
///
/// Elements in the `sce:` namespace are left to the grammar, which declares
/// their attributes locally and refuses one it does not know. A delegated
/// subtree — a worker's `<sce:body>`, a `<content>` or `<data>` value
/// carried as written — is its own reader's to judge.
pub(crate) fn unread_sce_attributes(ledger: &Ledger, root: &Node) -> Vec<Unread> {
    debug_assert_eq!(
        ledger.document,
        document_of(root),
        "a ledger is asked about its own document"
    );
    let document = root.document();
    let mut out = Vec::new();
    let mut pending = vec![*root];
    while let Some(node) = pending.pop() {
        if ledger.delegated.contains(&node.id()) {
            continue;
        }
        let mut children: Vec<Node> = node.children().filter(Node::is_element).collect();
        children.reverse();
        pending.extend(children);
        if node.tag_name().namespace() == Some(SCE_NAMESPACE) {
            continue;
        }
        for attribute in node.attributes() {
            if attribute.namespace() != Some(SCE_NAMESPACE)
                || ledger.attributes.contains(&attribute.range().start)
            {
                continue;
            }
            let pos = document.text_pos_at(attribute.range().start);
            let prefix = node.lookup_prefix(SCE_NAMESPACE).unwrap_or("sce");
            out.push(Unread {
                element: node.tag_name().name().to_string(),
                attribute: format!("{prefix}:{}", attribute.name()),
                row: pos.row,
                col: pos.col,
            });
        }
    }
    out
}

/// Refuse every `sce:` attribute [`unread_sce_attributes`] finds, as ONE
/// error that fans out to a record per attribute
/// (`validation/sce-attribute-unread`), located at the first. The one
/// refusal both parsers make, so a statechart and a forge document are
/// held to the rule in the same words.
pub(crate) fn refuse_unread_attributes(
    ledger: &Ledger,
    root: &Node,
    doc_name: &str,
) -> Result<(), Located<ForgeError>> {
    let unread = unread_sce_attributes(ledger, root);
    let Some(first) = unread.first() else {
        return Ok(());
    };
    let (row, col) = (first.row, first.col);
    Err(Located::new(
        crate::forge::error::ValidationError::UnreadSceAttributes(unread).into(),
        doc_name,
        Some(row),
        Some(col),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext">
  <state id="s" sce:req="R1" sce:invented="x"><sce:entry sce:odd="y"/></state>
</scxml>"#;

    fn state<'a, 'i>(document: &'a roxmltree::Document<'i>) -> Node<'a, 'i> {
        document
            .descendants()
            .find(|n| n.has_tag_name("state"))
            .unwrap()
    }

    #[test]
    fn what_was_asked_for_is_not_unread_and_the_rest_is() {
        let document = roxmltree::Document::parse(DOC).unwrap();
        let root = document.root_element();
        let recording = Recording::open(&root);
        assert_eq!(crate::sce_attr::read(&state(&document), "req"), Some("R1"));
        let unread = unread_sce_attributes(&recording.finish(), &root);
        assert_eq!(unread.len(), 1, "{unread:?}");
        assert_eq!(
            (unread[0].element.as_str(), unread[0].attribute.as_str()),
            ("state", "sce:invented")
        );
        assert_eq!(unread[0].row, 2);
    }

    #[test]
    fn an_acknowledged_attribute_is_not_unread() {
        let document = roxmltree::Document::parse(DOC).unwrap();
        let root = document.root_element();
        let recording = Recording::open(&root);
        crate::sce_attr::read(&state(&document), "req");
        crate::sce_attr::acknowledge(&state(&document), "invented", "test");
        assert!(unread_sce_attributes(&recording.finish(), &root).is_empty());
    }

    #[test]
    fn a_read_with_no_recording_open_records_nothing() {
        let document = roxmltree::Document::parse(DOC).unwrap();
        let root = document.root_element();
        assert_eq!(crate::sce_attr::read(&state(&document), "req"), Some("R1"));
        let recording = Recording::open(&root);
        assert_eq!(
            unread_sce_attributes(&recording.finish(), &root).len(),
            2,
            "the earlier read was not recorded"
        );
        OPEN.with(|open| {
            assert!(
                open.borrow().is_empty(),
                "the ledger closes with its recording"
            )
        });
    }

    #[test]
    fn a_nested_recording_hands_its_attribute_reads_outward() {
        let document = roxmltree::Document::parse(DOC).unwrap();
        let root = document.root_element();
        let outer = Recording::open(&root);
        {
            let inner = Recording::open(&state(&document));
            crate::sce_attr::read(&state(&document), "req");
            crate::sce_attr::read(&state(&document), "invented");
            drop(inner.finish());
        }
        assert!(
            unread_sce_attributes(&outer.finish(), &root).is_empty(),
            "the reads the inner parse made count for the enclosing check"
        );
    }
}
