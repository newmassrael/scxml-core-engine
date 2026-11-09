// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string>

namespace SCE {

/**
 * @brief The step of establishing a script session that failed (docs/adr/0096).
 *
 * A machine whose document needs a datamodel runs its conditions, assignments
 * and scripts in a script session. Establishing it is three steps in this
 * order: create the session, load the shared library into it, and bind the
 * system variables `_sessionid`, `_name` and `_ioprocessors`.
 */
enum class ScriptSessionStage {
    /// The script engine could not create the session.
    Create,
    /// The shared library did not load into the new session, which the engine
    /// then did not keep. A library that does not load is a defect of the
    /// build that shipped it.
    Library,
    /// The system variables could not be bound; the session was destroyed.
    SystemVariables,
};

/// The step's name, the same word on every backend: `create`, `library` or
/// `system-variables`.
[[nodiscard]] constexpr const char *scriptSessionStageName(ScriptSessionStage stage) noexcept {
    switch (stage) {
    case ScriptSessionStage::Create:
        return "create";
    case ScriptSessionStage::Library:
        return "library";
    case ScriptSessionStage::SystemVariables:
        return "system-variables";
    }
    return "";
}

/**
 * @brief A machine's script session could not be established, so its start was
 *        refused (docs/adr/0096)
 *
 * No state was entered, no executable content ran, no event was taken, and
 * what the failed steps made was released. A machine started over a session
 * one of the steps failed for would run with no datamodel, every condition an
 * error and every assignment lost, and the document could not even handle it:
 * its means of handling an error runs in the session that failed. So the start
 * is refused, and its host is handed this value.
 *
 * `StaticExecutionEngine::initialize()` answers it; `initializeAsRoot()`
 * carries it in a `RootStartRefusal`, and `enterAt()` in its
 * `Core::ConfigurationVerdict`.
 */
struct ScriptSessionFailure {
    /// The step that failed.
    ScriptSessionStage stage = ScriptSessionStage::Create;
    /// What the script engine said.
    std::string message;

    /// The failure as a sentence for a host to report, naming the step and the
    /// engine's words, in the words every backend uses.
    [[nodiscard]] std::string describe() const {
        std::string text = "the script session could not be established at step '";
        text += scriptSessionStageName(stage);
        text += "': ";
        text += message;
        return text;
    }
};

inline bool operator==(const ScriptSessionFailure &a, const ScriptSessionFailure &b) {
    return a.stage == b.stage && a.message == b.message;
}

inline bool operator!=(const ScriptSessionFailure &a, const ScriptSessionFailure &b) {
    return !(a == b);
}

}  // namespace SCE
