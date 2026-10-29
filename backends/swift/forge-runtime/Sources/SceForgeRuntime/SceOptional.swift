// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The read of a value of an optional type the build has proven present (docs/adr/0055,
// section 2): `x.value` where `x !== null` was tested on every path to it. The Swift
// emitter writes it as `SceOptional.proven(x)` over the `T?` an optional value is held in,
// as the Kotlin one writes `SceOptional.proven(x)` over its `T?`.

/// The reads of an optional value that have no operator of Swift's own.
public enum SceOptional {
    /// The present value of `value`, which the build proved is present.
    ///
    /// An absent one is a fault of the proof, not a value a document can hold: it is
    /// reported where it happens and never answered with a made-up value, which would
    /// let the wrong result travel on as a right one. Swift's own `!` fails the same
    /// way but names nothing, so a failed proof is not told apart from any other.
    public static func proven<T>(_ value: T?) -> T {
        guard let value else {
            preconditionFailure("SceOptional.proven: the value is absent, which the build proved it is not")
        }
        return value
    }
}
