// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * The stem of the document a hybrid `<invoke>`'s evaluated `srcexpr` names — what
 * the value is matched against the declared `sce:candidates` by (§scxml-6.4,
 * SCE_ACCEPTED_SUBSET.md §2.13). Port of Rust `document_stem`.
 *
 * An expression is free to compute `file:x.scxml`, `./x.scxml`, an absolute path
 * or a Windows one for the same document, so the value is reduced to what the
 * build named the generated child by: the last path segment, without a `file:`
 * scheme and without its extension. A leading dot is a name, not an extension, as
 * the build reads it when it derives the candidate's stem.
 *
 * `tests/document_stem/document_stem.json` is the one table every engine's reader
 * and the build's are measured against.
 */
object DocumentStem {
    fun of(value: String): String {
        val name = value
            .substringAfterLast('/')
            .substringAfterLast('\\')
            .removePrefix("file:")
        val dot = name.lastIndexOf('.')
        return if (dot > 0) name.substring(0, dot) else name
    }
}
