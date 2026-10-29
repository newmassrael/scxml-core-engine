// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What a validator answers (SCE_FORGE.md Section 4.7). Siblings: `ValidationResult` (Kotlin, a data
// class of the unit's package), `ValidationResult` (Rust and Go, a struct of the unit's module).
//
// The Swift units are files of one module, so a type each of them declared would be declared
// once per validator; the runtime declares it once, and every validator unit answers it.

/// The verdict of one check: `valid`, and the `reason` it is not (empty when it is).
public struct ValidationResult: Equatable, Sendable {
    public let valid: Bool
    public let reason: String

    public init(valid: Bool, reason: String) {
        self.valid = valid
        self.reason = reason
    }
}
