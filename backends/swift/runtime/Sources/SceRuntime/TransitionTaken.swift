// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What a run tells the observer of each transition it takes (docs/adr/0089): the
// transition as its row of the emitted transition table names it
// (docs/SCE_TRANSITION_TABLE.md).

/// One transition the engine took, as its row of the emitted transition table names it.
///
/// `source` is the machine's own state type, the type the table's `from` holds, and `index`
/// is the table's `index` type, so a pair compares with a row (`row.from`, `row.index`)
/// without a conversion.
public struct TransitionTaken<State: Hashable>: Hashable {
    /// The state the transition is written in: the `from` of its row.
    public var source: State

    /// Its position among `source`'s own transitions in document order: the `index` of its
    /// row.
    public var index: Int

    public init(source: State, index: Int) {
        self.source = source
        self.index = index
    }
}

extension TransitionTaken: Sendable where State: Sendable {}
