// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Why a machine's script session could not be established, which refuses its start
// (docs/adr/0096).
//
// A machine whose document needs a datamodel runs its conditions, assignments and scripts in a
// script session. Establishing it is three steps in this order: create the session, load the
// shared library into it (the ECMAScript semantics, the JSON builtins and the DOM read surface),
// and bind the system variables `_sessionid`, `_name` and `_ioprocessors`. A machine started over
// a session one of them failed for would run with no datamodel, every condition an error and every
// assignment lost, and the document could not even handle it: its means of handling an error runs
// in the session that failed. So the start is refused, and the host is handed a
// `ScriptSessionFailure`.
//
// Cross-language siblings: `sce_rust_runtime::ScriptSessionFailure` (Rust),
// `SCE::ScriptSessionFailure` (C++), `sce.ScriptSessionFailure` (Go) and
// `com.sce.runtime.ScriptSessionFailure` (Kotlin), whose step names and sentences are these.

/// The step of establishing a script session that failed.
public enum ScriptSessionStage: Sendable, Equatable, CaseIterable, CustomStringConvertible {
    /// The script engine could not create the session.
    case create

    /// The shared library did not load into the new session, which was then released. A library
    /// chunk that does not load is a defect of the build that shipped it.
    case library

    /// The system variables `_sessionid`, `_name` and `_ioprocessors` could not be bound; the
    /// session was released.
    case systemVariables

    /// The step's name, the same word on every backend.
    public var description: String {
        switch self {
        case .create:
            return "create"
        case .library:
            return "library"
        case .systemVariables:
            return "system-variables"
        }
    }
}

/// Why a machine's script session could not be established, so its start was refused: no state
/// was entered, no executable content ran, no event was taken, and what the failed steps made was
/// released.
///
/// `Engine.initialize` throws it, `Engine.initializeAsRoot` throws it as
/// `RootStartRefusal.scriptSession`, and `Engine.enterAt` answers it as
/// `ConfigurationRejection.scriptSessionUnavailable`. A machine without a script engine has no
/// session to fail, and none of the three ever refuses it for this.
public struct ScriptSessionFailure: Error, Sendable, Equatable, CustomStringConvertible {
    /// The step that failed.
    public let stage: ScriptSessionStage

    /// What the script engine said.
    public let message: String

    public init(stage: ScriptSessionStage, message: String) {
        self.stage = stage
        self.message = message
    }

    /// The failure as a sentence: the step that failed, then the engine's words, the sentence
    /// every backend writes.
    public var description: String {
        "the script session could not be established at step '\(stage)': \(message)"
    }

    /// The data of the `error.execution` a parent raises for the child of the `<invoke>`
    /// `invokeID` whose start this failure refused: the invoke, and the step its child's script
    /// session failed at. The parent keeps no child and raises no `done.invoke` for it, the words
    /// every backend writes.
    public func childStartRefusal(invokeID: String) -> String {
        "<invoke id='\(invokeID)'> did not start: its child's script session failed at step '\(stage)'"
    }
}
