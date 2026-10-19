// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <stdexcept>
#include <string>

#include "runtime/ActionRealization.h"

namespace SCE {

/**
 * @brief Whether a machine that states a duty takes a host that only simulates (docs/adr/0038)
 *
 * The trailing argument of a generated machine's constructor. It is an enum
 * class and not a `bool` so that it is written out at the call
 * (`Machine m(host, SCE::SimulatedHost::Accept)`) and cannot be reached by an
 * argument that happens to convert: a `true` passed one position early, or a
 * flag forwarded from a wrapper, would otherwise opt a production wiring in
 * without anyone having said so.
 */
enum class SimulatedHost {
    /// The default: a host whose `realization()` is `Simulates` is refused.
    Refuse,
    /// The caller built the machine for a recorder or a test double, by name.
    Accept,
};

/**
 * @brief The refusal a generated machine makes at construction
 *
 * One definition serves every generated C++ machine that states a duty, so the
 * refusal reads the same whichever machine makes it. A machine's constructor
 * calls it with the host's declared realization before the host is stored; a
 * child machine a parent builds with the host the parent's host answers
 * calls it with the opt-in its parent was given, so the check is made where
 * each machine meets its host and is repeated nowhere else.
 *
 * @throws std::invalid_argument when `realization` is `Simulates` and
 *         `simulated` is not `Accept`; the message names the machine, says the
 *         host declares that it only simulates the duties, and names the opt-in.
 */
inline void requireHostPerforms(const char *machine, ActionRealization realization, SimulatedHost simulated) {
    if (realization == ActionRealization::Simulates && simulated != SimulatedHost::Accept) {
        throw std::invalid_argument(
            std::string("machine '") + machine +
            "' states host duties, and its host declares that it only simulates them "
            "(realization() is Simulates): build the machine with a host that performs them, "
            "or pass ::SCE::SimulatedHost::Accept as the trailing constructor argument to accept "
            "a simulated host on purpose");
    }
}

}  // namespace SCE
