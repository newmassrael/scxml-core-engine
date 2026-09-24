// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What the forge parser read of a document, so what it did not read can be
//! refused rather than ignored.
//!
//! # Why
//!
//! A kind parser reaches its elements by asking a parent for a child by name
//! — the root for `datamodel`, a `<datamodel>` for its `<sce:helper>`s — and
//! an element written anywhere else is never asked for. The schema cannot
//! catch it: the `<scxml>` root admits SCE elements laxly, because one root
//! element serves seventeen kinds. So a `<sce:helper>` written under the
//! procedure root was dropped without a word, and the refusal that followed
//! blamed the call that named it (measured 2026-09-24).
//!
//! # How
//!
//! The parser's child accessors record, while a document is parsed:
//!
//! * **asked** — the SCE names a reader asked a parent for, found or not;
//! * **taken** — every element an accessor handed to a reader;
//! * **closed** — a parent whose every child a reader judged itself (a loop
//!   that refuses what it does not take), so nothing under it is left over;
//! * **delegated** — a subtree handed whole to another reader.
//!
//! [`refuse_unread`] then refuses the first SCE element that nothing took,
//! naming what its parent was asked for as the alternatives. Nothing here is
//! a list of where elements belong: the parser's own reads are the list, so
//! a kind added tomorrow is covered by the reads it makes.
//!
//! The ledger is scoped to one parse on one thread ([`Recording`]); the
//! accessors record into the innermost open recording of the same document
//! and do nothing when none is open, so a reader outside a parse — a test, a
//! tool reading a fragment — is unaffected.

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
        OPEN.with(|open| {
            let mut open = open.borrow_mut();
            debug_assert_eq!(open.len(), self.depth, "recordings close innermost first");
            open.pop().expect("an open recording")
        })
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if !self.finished {
            OPEN.with(|open| {
                let mut open = open.borrow_mut();
                debug_assert_eq!(open.len(), self.depth, "recordings close innermost first");
                open.pop();
            });
        }
    }
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
