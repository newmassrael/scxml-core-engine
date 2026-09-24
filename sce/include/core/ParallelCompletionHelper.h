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
 * @brief Appendix D's isInFinalState, over a configuration
 *
 * Lambda-injected, the shape `ExitSetAlgorithms` and
 * `ConflictResolutionAlgorithms` use, so the Interpreter (states are
 * `std::string`) and the generated code (states are an enum) ask one
 * definition rather than two that agree only where no `<parallel>` nests.
 */
struct CompletionAlgorithms {
    /**
     * @brief Appendix D's isInFinalState
     *
     * @param parentOf `(const StateType&) -> std::optional<StateType>`
     * @param isParallel `(const StateType&) -> bool`
     * @param childStates `(const StateType&) -> std::vector<StateType>`, asked
     *        of a `<parallel>` only: its child states in document order
     * @param isFinal `(const StateType&) -> bool`: the state is a `<final>`
     *        element — not whether it is IN a final state
     */
    template <typename StateType, typename ParentOfFn, typename IsParallelFn, typename ChildStatesFn,
              typename IsFinalFn>
    [[nodiscard]] static bool isInFinalState(const StateType &state, const std::vector<StateType> &configuration,
                                             ParentOfFn parentOf, IsParallelFn isParallel, ChildStatesFn childStates,
                                             IsFinalFn isFinal) {
        // §scxml-D-isInFinalState: a compound state is in a final state when
        // one of its `<final>` children is active; a `<parallel>` when every
        // one of its child states is — asked recursively, so a region that is
        // itself a `<parallel>` counts only once all of ITS regions do.
        // Nothing else ever is.
        if (isParallel(state)) {
            const auto children = childStates(state);
            return !children.empty() && std::all_of(children.begin(), children.end(), [&](const StateType &child) {
                return isInFinalState(child, configuration, parentOf, isParallel, childStates, isFinal);
            });
        }
        return std::any_of(configuration.begin(), configuration.end(), [&](const StateType &active) {
            const auto parent = parentOf(active);
            return parent.has_value() && *parent == state && isFinal(active);
        });
    }
};

/**
 * @brief Parallel state completion (§scxml-3.4), bound to a StatePolicy
 *
 * §scxml-3.4: "When all of the children reach final states, the <parallel>
 * element itself is considered to be in a final state" — and done.state.id is
 * generated when it does. The rule itself is `CompletionAlgorithms`; this binds
 * it to the generated policy's static tables.
 */
class ParallelCompletionHelper {
public:
    /**
     * @brief Whether a `<parallel>` is in a final state
     *
     * Every region is asked Appendix D's isInFinalState, which is recursive: a
     * region that is itself a `<parallel>` is complete only when all of its own
     * regions are. The earlier form asked every region for an active `<final>`
     * child, which no `<parallel>` region ever has, so a `<parallel>` holding
     * one never completed.
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy providing getParallelRegions, getParent,
     *         isParallelState and isFinalState
     * @param parallelState The `<parallel>` to ask about
     * @param activeStates The current configuration
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static bool areAllRegionsInFinal(StateType parallelState, const std::vector<StateType> &activeStates) {
        if (!PolicyType::isParallelState(parallelState)) {
            return false;
        }
        return CompletionAlgorithms::isInFinalState(
            parallelState, activeStates, [](const StateType &s) { return PolicyType::getParent(s); },
            [](const StateType &s) { return PolicyType::isParallelState(s); },
            [](const StateType &s) { return PolicyType::getParallelRegions(s); },
            [](const StateType &s) { return PolicyType::isFinalState(s); });
    }
};

}  // namespace SCE::Core
