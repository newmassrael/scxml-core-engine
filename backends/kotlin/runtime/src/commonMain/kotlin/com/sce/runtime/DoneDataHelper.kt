// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.runtime

/**
 * W3C SCXML 5.5: Emit an inline `<content>` literal as `_event.data`.
 *
 * 1:1 port of C++ `SCE::DoneDataHelper::emitContentLiteral`
 * ([`sce/include/common/DoneDataHelper.h`]). When `<content>` has no `expr`
 * attribute the spec says "the children are used as the content value" —
 * no evaluation happens and no script engine is required. The literal text
 * **is** the value.
 *
 * This is the SSoT consumed by the `literal` branch of the Kotlin AOT
 * codegen (`tools/codegen/templates/kotlin/entry_exit_actions.kt.jinja2`),
 * matching the C++ `emitContentLiteral` / Rust `emit_content_literal` / Go
 * `EmitContentLiteral` helpers so all four backends share one semantic
 * definition.
 *
 * @param literal Inline text content from `<content>literal</content>`
 * @return The literal as `_event.data` (raw string — no JSON quoting).
 */
fun emitContentLiteral(literal: String): String = literal
