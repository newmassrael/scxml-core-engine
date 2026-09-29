// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * The author's own marks: `sce:unresolved` and `sce:assumed`.
 *
 * ## What they are, and why they are drawn unconditionally
 *
 * An author writing a document from a specification leaves two kinds of
 * marker where the specification did not decide: UNRESOLVED (the question
 * is open) and ASSUMED (the author chose, and the choice needs a
 * reviewer's yes). The printed diagram and the checklist carry both; this
 * file makes the interactive diagram carry them too.
 *
 * Unlike the requirement family in `annotation-overlay.js`, there is no
 * "is this document under review" switch. A requirement's ABSENCE is only
 * meaningful on a document that claims requirements at all, so that family
 * stays unpainted otherwise. A marker's PRESENCE is meaningful on any
 * document: it exists only because the author wrote it. So an element is
 * marked exactly when it carries one, and nothing is marked otherwise.
 *
 * ## This file derives nothing
 *
 * The ids arrive on the structure's states and transitions, placed there by
 * `gui_structure()` in the sce-build WASM from the parsed model. Every
 * function below reads what arrived.
 */

const SCE_MARK_UNRESOLVED = 'sce-unresolved';
const SCE_MARK_ASSUMED = 'sce-assumed';

/** The two kinds, in the order a reviewer should look at them. */
const SCE_MARK_KINDS = [
    { key: 'unresolved', className: SCE_MARK_UNRESOLVED },
    { key: 'assumed', className: SCE_MARK_ASSUMED },
];

/**
 * The marks a structure element carries, copied onto what the diagram
 * draws for it.
 *
 * Keys are absent rather than empty when the element carries none, which
 * is the shape the structure itself uses.
 */
function authorMarksOf(element) {
    const marks = {};
    for (const { key } of SCE_MARK_KINDS) {
        if (element && Array.isArray(element[key]) && element[key].length) {
            marks[key] = [...element[key]];
        }
    }
    return marks;
}

/**
 * The marks of one arrow drawn for several transitions.
 *
 * ⚠ A union, for the same reason the requirement family takes one: an
 * arrow showing only its first member's marks would hide an open question
 * behind a merge the reviewer cannot see.
 */
function mergeAuthorMarks(into, from) {
    for (const { key } of SCE_MARK_KINDS) {
        const ids = new Set([...(into[key] || []), ...(from[key] || [])]);
        if (ids.size) {
            into[key] = [...ids];
        }
    }
    return into;
}

/**
 * The class suffix the renderer appends to a drawn element.
 *
 * ⭐ The ONE place the marks become classes, so a state and an edge
 * cannot be marked by different rules. Empty string, not `undefined`: the
 * caller concatenates it.
 */
function authorMarkClassFor(d) {
    let classes = '';
    for (const { key, className } of SCE_MARK_KINDS) {
        if (d && Array.isArray(d[key]) && d[key].length) {
            classes += ` ${className}`;
        }
    }
    return classes;
}

/**
 * The marker ids of one kind as an attribute value, or `null` to omit the
 * attribute, so the ids are readable off the element and not only
 * inferable from its outline.
 */
function authorMarkIdsFor(d, key) {
    return d && Array.isArray(d[key]) && d[key].length ? d[key].join(' ') : null;
}

/**
 * The hover text naming the marks, or `null` when there are none.
 */
function authorMarkTitleFor(d) {
    const lines = [];
    for (const { key } of SCE_MARK_KINDS) {
        const ids = authorMarkIdsFor(d, key);
        if (ids) {
            lines.push(`${key}: ${ids}`);
        }
    }
    return lines.length ? lines.join('\n') : null;
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        authorMarksOf,
        mergeAuthorMarks,
        authorMarkClassFor,
        authorMarkIdsFor,
        authorMarkTitleFor,
        SCE_MARK_UNRESOLVED,
        SCE_MARK_ASSUMED,
        SCE_MARK_KINDS,
    };
}
