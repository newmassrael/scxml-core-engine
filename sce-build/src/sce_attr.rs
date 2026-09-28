// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one channel an `sce:` attribute is read through, so a parse knows
//! which of them it asked for.
//!
//! Every read here is recorded on the parse's read ledger
//! ([`crate::read_ledger`]), and when the parse ends the `sce:` attributes
//! on its W3C elements that nothing asked for are refused
//! (`validation/sce-attribute-unread`). The set of attributes an element
//! takes is therefore the parser's own reads — derived from the code, not
//! kept beside it. The ledger's header says why the schema cannot answer
//! this.
//!
//! # ⚠ Deliberately unread is not unread
//!
//! Some attributes are legal where the parser does not consume them: a
//! `<data>`'s `sce:type` and `sce:direction` are the model's only under
//! `datamodel="sce-static"`, and elsewhere they are the authoring tool's
//! typed I/O declaration. A site that consults such an attribute and
//! chooses not to carry it calls [`acknowledge`] with the reason, so the
//! choice is written where it is made rather than discovered as a false
//! refusal.

use crate::forge::model::SCE_NAMESPACE;

/// Record that `attribute`, found on `node` by a reader of its own, was
/// asked for — for the helpers that locate an attribute by namespace to
/// read its spelling or position. A non-`sce:` attribute is ignored.
pub fn note(node: &roxmltree::Node, attribute: &roxmltree::Attribute) {
    if attribute.namespace() == Some(SCE_NAMESPACE) {
        crate::read_ledger::attribute_read(node, attribute);
    }
}

/// Read `sce:<local_name>` from `node`, recording that it was asked for.
pub fn read<'a>(node: &roxmltree::Node<'a, '_>, local_name: &str) -> Option<&'a str> {
    attribute(node, local_name).map(|a| a.value())
}

/// [`read`], answering with the attribute itself for a caller that needs
/// its position.
pub fn attribute<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<roxmltree::Attribute<'a, 'input>> {
    let found = node.attribute_node((SCE_NAMESPACE, local_name))?;
    crate::read_ledger::attribute_read(node, &found);
    Some(found)
}

/// Every `sce:` attribute on `node`, each recorded as asked for — for a
/// reader that dispatches on the name itself.
pub fn all<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Attribute<'a, 'input>> {
    node.attributes()
        .filter(|a| a.namespace() == Some(SCE_NAMESPACE))
        .inspect(|a| crate::read_ledger::attribute_read(node, a))
        .collect()
}

/// Record that `sce:<local_name>` on `node` was consulted and is left
/// unread on purpose. `_why` is the reason, kept at the call site.
pub fn acknowledge(node: &roxmltree::Node, local_name: &str, _why: &str) {
    if let Some(found) = node.attribute_node((SCE_NAMESPACE, local_name)) {
        crate::read_ledger::attribute_read(node, &found);
    }
}
