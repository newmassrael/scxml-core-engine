// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"strings"

	errorreason "github.com/newmassrael/sce-go-runtime/generated/error_reason"
)

// ErrorReason is why an engine raised error.execution or error.communication:
// the one vocabulary every engine and the Mesh write `_event.data.reason` from
// (sce:std/engine/error_reason.scxml, docs/adr/0106 #111), generated into this
// module by the tree tools/codegen/trees/runtime-go.toml. Its constants are
// the generated package's (`errorreason.ErrorReasonExpressionFailed`, ...); a
// generated machine imports that package beside this one.
type ErrorReason = errorreason.ErrorReason

// ErrorName is the `errorName` of an error.* event raised for reason
// (SCE_MESH.md section 10.7.1): "execution" or "communication", by the range
// the vocabulary gives the code (1 to 63, 64 to 127), so a code and the event
// it is raised as cannot disagree.
func ErrorName(reason ErrorReason) string {
	if reason.ToUnderlying() < 64 {
		return "execution"
	}
	return "communication"
}

// ErrorData is the `_event.data` of an error.* event raised for reason, as
// JSON text: {"errorName":…,"reason":…,"detail":…} (SCE_MESH.md section
// 10.7.1), the code spelled by the vocabulary's declared name and detail
// escaped as every engine escapes a JSON string (EscapeJSONString).
func ErrorData(reason ErrorReason, detail string) string {
	name, _ := reason.DeclaredName().Get()
	var b strings.Builder
	b.Grow(len(detail) + 64)
	b.WriteString(`{"errorName":"`)
	b.WriteString(ErrorName(reason))
	b.WriteString(`","reason":"`)
	b.WriteString(name)
	b.WriteString(`","detail":"`)
	b.WriteString(EscapeJSONString(detail))
	b.WriteString(`"}`)
	return b.String()
}
