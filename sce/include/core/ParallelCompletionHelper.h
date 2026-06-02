// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
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
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "core/StatePolicyConcepts.h"
#include <algorithm>
#include <vector>

namespace SCE::Core {

/**
 * @brief Helper for parallel state completion detection (§scxml-3.4)
 *
 * Single Source of Truth for "all regions in final state" logic.
 * Shared between Interpreter and AOT engines following Zero Duplication Principle.
 *
 * §scxml-3.4: "When all of the children reach final states,
 * the <parallel> element itself is considered to be in a final state"
 *
 * §scxml-3.4: done.state.id event is generated upon parallel completion
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Single implementation shared by both engines
 * - Helper Pattern: Follows SendHelper, ForeachHelper, HistoryHelper patterns
 */
class ParallelCompletionHelper {
public:
    /**
     * @brief Check if all child regions of a parallel state are in final states
     *
     * §scxml-3.4: Parallel state is complete when ALL child regions
     * have at least one active final state.
     *
     * Algorithm:
     * 1. Get all child regions of the parallel state
     * 2. For each region, check if any of its final states are active
     * 3. Return true only if ALL regions have an active final state
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class providing state information methods:
     *                    - getParallelRegions(StateType) -> vector of region IDs
     *                    - getParent(StateType) -> parent state ID
     *                    - isFinalState(StateType) -> bool
     * @param parallelState The parallel state to check for completion
     * @param activeStates Vector of currently active states
     * @return true if all regions have at least one active final state, false otherwise
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static bool areAllRegionsInFinal(StateType parallelState, const std::vector<StateType> &activeStates) {
        // §scxml-3.4: Get all child regions of this parallel state
        auto regions = PolicyType::getParallelRegions(parallelState);

        if (regions.empty()) {
            // No regions means not a valid parallel state
            return false;
        }

        // Check each region for completion
        for (const auto &region : regions) {
            bool regionHasFinalState = false;

            // Check if any final state child of this region is currently active
            for (const auto &activeState : activeStates) {
                // §scxml-3.4: A region is complete if any of its final children are active
                if (PolicyType::getParent(activeState) == region && PolicyType::isFinalState(activeState)) {
                    regionHasFinalState = true;
                    break;
                }
            }

            if (!regionHasFinalState) {
                // At least one region is not complete yet
                return false;
            }
        }

        // §scxml-3.4: All regions have final states active
        return true;
    }
};

}  // namespace SCE::Core
