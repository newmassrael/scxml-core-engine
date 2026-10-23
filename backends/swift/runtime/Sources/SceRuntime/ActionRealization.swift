// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// What a host does with the duties of a machine (docs/adr/0038).
///
/// A `<sce:action effect="duty">` is an operation the machine's correctness
/// depends on the host performing. The generated host interface of a machine that
/// states one carries `realization`, so the author of a host says beside the code
/// whether it performs the duties or only simulates them (a recorder, a test
/// double). The machine's initializer traps for a host that simulates; the
/// simulated initializer is the caller saying on purpose that it means to run the
/// machine on one.
///
/// `performs` is a statement the host's author makes about the bodies of its
/// methods, in a line a reviewer can find. SCE cannot read the bodies, and an empty
/// body under `performs` is that author's false statement and not an unwritten
/// fact.
public enum ActionRealization: Sendable, Equatable, CustomStringConvertible {
    /// The host does not perform the duties; it records, drops or fakes them.
    case simulates

    /// The host performs the duties; its methods do what the operations' names say.
    case performs

    /// The realization as the interface's documentation names it.
    public var description: String {
        switch self {
        case .performs: return "Performs"
        case .simulates: return "Simulates"
        }
    }
}
