// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/ForwardedEvent.h"
#include "common/ScriptSessionFailure.h"

#include <optional>
#include <utility>

namespace SCE::Common {

/**
 * @brief The parent of a session its HOST started — there is none.
 *
 * A generated machine that sends to `#_parent` is a class template over its
 * parent's type, because an invoking parent hands itself in. A host starting
 * the same machine has no parent to hand in, and §scxml-C-1 still lets it
 * run: the send then raises error.communication on the sender's own queue.
 * This type is the default for that template parameter, so the host writes
 * `machine<> sm;` and the policy's parent pointer stays null.
 *
 * It declares only what the generated code names on a parent — delivery by
 * event NAME, the one identity two machines share (`ForwardedEvent`). The
 * member is never called: every delivery first asks whether the pointer is
 * null, and for this type it always is.
 */
struct NoParent {
    void raiseExternal(const ForwardedEvent &) {}
};

/**
 * @brief Why `StaticExecutionEngine::initializeAsRoot` refused a start.
 *
 * It refuses for either of two reasons. The first is asked before anything is
 * established (§scxml-6.2.4: the document sends to `#_parent`, and a session
 * its host started has no parent to reach): the default is to run a machine
 * that needs a parent — its
 * `#_parent` sends then raise error.communication (§scxml-C-1) — and a host
 * that would rather not start one, when it has none to give, asks for the
 * refusal by starting it with `initializeAsRoot`. The second is the plain
 * start's own: a machine whose script session cannot be established is not
 * started (docs/adr/0096), and the refusal carries the failure.
 *
 * Spelled as the two values `RootStartRefusal::NeedsParent` and
 * `RootStartRefusal::ScriptSession(failure)`, the shape of the refusal on the
 * other engines; `kind()` tells them apart.
 */
class RootStartRefusal {
public:
    enum class Kind {
        /// The document sends to `#_parent`, and a session its host started
        /// has no parent to reach.
        NeedsParent,
        /// docs/adr/0096: the machine's script session could not be
        /// established, so nothing was entered and nothing ran.
        ScriptSession,
    };

    /// The refusal of a machine that needs a parent.
    static const RootStartRefusal NeedsParent;

    /// The refusal of a machine whose script session could not be established.
    static RootStartRefusal ScriptSession(::SCE::ScriptSessionFailure failure) {
        return RootStartRefusal(std::move(failure));
    }

    [[nodiscard]] Kind kind() const noexcept {
        return failure_.has_value() ? Kind::ScriptSession : Kind::NeedsParent;
    }

    /// The failure a `ScriptSession` refusal carries; null for `NeedsParent`.
    [[nodiscard]] const ::SCE::ScriptSessionFailure *scriptSessionFailure() const noexcept {
        return failure_.has_value() ? &*failure_ : nullptr;
    }

    friend bool operator==(const RootStartRefusal &a, const RootStartRefusal &b) {
        return a.failure_ == b.failure_;
    }

    friend bool operator!=(const RootStartRefusal &a, const RootStartRefusal &b) {
        return !(a == b);
    }

private:
    RootStartRefusal() = default;

    explicit RootStartRefusal(::SCE::ScriptSessionFailure failure) : failure_(std::move(failure)) {}

    /// Empty exactly for `NeedsParent`.
    std::optional<::SCE::ScriptSessionFailure> failure_;
};

inline const RootStartRefusal RootStartRefusal::NeedsParent{};

/// The refusal as a sentence, for a host to report — the same words on every
/// engine. A `ScriptSession` refusal carries the step that failed and the
/// engine's words beside it (`scriptSessionFailure()->describe()`).
inline const char *reason(const RootStartRefusal &refusal) {
    switch (refusal.kind()) {
    case RootStartRefusal::Kind::NeedsParent:
        return "the machine sends to #_parent and was started with no parent session";
    case RootStartRefusal::Kind::ScriptSession:
        return "the machine's script session could not be established";
    }
    return "";
}

}  // namespace SCE::Common
