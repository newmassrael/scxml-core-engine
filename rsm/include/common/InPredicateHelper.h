// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include <string>
#include <vector>

namespace RSM::InPredicateHelper {

/**
 * @brief W3C SCXML 5.9.2: Check if state is active (Single Source of Truth)
 *
 * ARCHITECTURE.md Zero Duplication Principle: Shared logic for In() predicate
 * between Interpreter and AOT engines. Both engines must use this Helper to
 * avoid duplicating state membership checking logic.
 *
 * This Helper follows the established pattern of GuardHelper, DataModelInitHelper,
 * and other Helpers that provide Single Source of Truth for shared algorithms.
 *
 * @tparam StateType Type representing a state (e.g., State enum, std::string)
 * @tparam StateGetter Callable that converts StateType to state name string
 * @param activeStates Vector of currently active states
 * @param getStateName Function/lambda to convert state to string name
 * @param stateId State ID to check for membership
 * @return true if state is active, false otherwise
 *
 * @note Thread-safety: Caller must protect activeStates with appropriate mutex
 */
template <typename StateType, typename StateGetter>
inline bool isStateActive(const std::vector<StateType> &activeStates, StateGetter getStateName,
                          const std::string &stateId) {
    // W3C SCXML 5.9.2: In(stateId) returns true if stateId is in active state configuration
    for (const auto &state : activeStates) {
        if (getStateName(state) == stateId) {
            return true;
        }
    }
    return false;
}

}  // namespace RSM::InPredicateHelper
