// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// EmitContentLiteral emits an inline <content> body as _event.data (W3C SCXML 5.5).
//
// 1:1 port of C++ SCE::DoneDataHelper::emitContentLiteral
// (sce/include/common/DoneDataHelper.h). When <content> has no expr attribute
// the spec says "the children are used as the content value" — no evaluation
// happens and no script engine is required. The literal text is the value.
//
// This is the SSoT consumed by the `literal` branch of the Go AOT codegen
// (tools/codegen/templates/go/entry_exit_actions.go.jinja2), matching the
// C++ emitContentLiteral / Rust emit_content_literal / Kotlin
// emitContentLiteral helpers so all four backends share one semantic
// definition.
func EmitContentLiteral(literal string) string {
	return literal
}
