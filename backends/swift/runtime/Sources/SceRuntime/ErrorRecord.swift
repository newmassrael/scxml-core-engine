// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The record an `error.*` event the engine raises carries as its data, built from
// `ErrorReason`, the one vocabulary every engine and the Mesh write
// `_event.data.reason` from (sce:std/engine/error_reason.scxml, docs/adr/0106
// #111), generated into this module by the tree
// tools/codegen/trees/runtime-swift.toml.

/// The `errorName` of an `error.*` event raised for `reason` (SCE_MESH.md section
/// 10.7.1): `"execution"` or `"communication"`, by the range the vocabulary gives
/// the code (1 to 63, 64 to 127), so a code and the event it is raised as cannot
/// disagree.
public func errorName(_ reason: ErrorReason) -> String {
    ErrorReason.toUnderlying(reason) < 64 ? "execution" : "communication"
}

/// The `_event.data` of an `error.*` event raised for `reason`, as JSON text:
/// `{"errorName":…,"reason":…,"detail":…}` (SCE_MESH.md section 10.7.1), the code
/// spelled by the vocabulary's declared name and `detail` written as every engine
/// writes a JSON string (`quoteJSON(_:)`).
public func errorData(_ reason: ErrorReason, detail: String) -> String {
    "{\"errorName\":\"" + errorName(reason) + "\",\"reason\":\"" + reason.declaredName
        + "\",\"detail\":" + quoteJSON(detail) + "}"
}
