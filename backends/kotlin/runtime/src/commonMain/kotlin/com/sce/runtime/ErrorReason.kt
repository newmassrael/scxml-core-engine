// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Why an engine raised `error.execution` or `error.communication`: the one
 * vocabulary every engine and the Mesh write `_event.data.reason` from
 * (sce:std/engine/error_reason.scxml, docs/adr/0106 #111), generated into this
 * runtime by the tree `tools/codegen/trees/runtime-kotlin.toml`. Named here so
 * a generated machine, which imports `com.sce.runtime.*`, reaches it as the
 * Rust runtime's `sce_rust_runtime::ErrorReason`.
 */
typealias ErrorReason = com.sce.generated.error_reason.ErrorReason

/**
 * The `errorName` of an `error.*` event raised for [reason] (SCE_MESH.md
 * section 10.7.1): `"execution"` or `"communication"`, by the range the
 * vocabulary gives the code (1 to 63, 64 to 127), so a code and the event it
 * is raised as cannot disagree.
 */
fun errorName(reason: ErrorReason): String =
    if (reason.toUnderlying().toInt() < 64) "execution" else "communication"

/**
 * The `_event.data` of an `error.*` event raised for [reason], as JSON text:
 * `{"errorName":…,"reason":…,"detail":…}` (SCE_MESH.md section 10.7.1), the
 * code spelled by the vocabulary's declared name and [detail] quoted as every
 * engine quotes a JSON string ([Json.quote]).
 */
fun errorData(reason: ErrorReason, detail: String): String =
    "{\"errorName\":\"" + errorName(reason) + "\",\"reason\":\"" + reason.declaredName() +
        "\",\"detail\":" + Json.quote(detail) + "}"
