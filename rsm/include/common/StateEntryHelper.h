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

#include <algorithm>
#include <optional>
#include <set>
#include <vector>

namespace RSM {

/**
 * @brief W3C SCXML 3.6: Deep initial state entry logic (Zero Duplication Principle)
 *
 * Single Source of Truth for ancestor path calculation and optimized entry order.
 * Shared between Interpreter engine and AOT generated code.
 *
 * W3C SCXML 3.6 allows space-separated descendant state IDs in the initial attribute:
 *   <state id="s1" initial="s11p112 s11p122">
 *
 * The processor must enter all specified states by:
 * 1. Calculating ancestor paths for each target
 * 2. Deduplicating common ancestors across targets
 * 3. Entering states in document order (top-to-bottom)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Logic shared between Interpreter and AOT engines
 * - Single Source of Truth: All deep state entry uses this Helper
 * - Helper Pattern: Follows SendHelper, InvokeHelper, HistoryHelper design
 */
class StateEntryHelper {
public:
    /**
     * @brief W3C SCXML 3.6: Calculate ancestor path for deep initial target
     *
     * Returns ancestor path from parent+1 to target (inclusive) in top-to-bottom order.
     *
     * Example:
     *   parent = s1
     *   target = s11p112
     *   hierarchy: s1 → s11 → s11p1 → s11p11 → s11p112
     *   result = [s11, s11p1, s11p11, s11p112]
     *
     * @tparam StateType State enum type (AOT) or std::string (Interpreter)
     * @tparam GetParentFunc Function: StateType -> std::optional<StateType>
     *
     * @param target Deep initial target state
     * @param parent Parent state containing the initial attribute
     * @param getParent Function to get parent of a state
     * @return Ancestor path in document order (top-to-bottom)
     *
     * @throws std::runtime_error if cycle detected or max depth exceeded
     */
    template <typename StateType, typename GetParentFunc>
    static std::vector<StateType> calculateAncestorPath(StateType target, StateType parent, GetParentFunc getParent) {
        std::vector<StateType> ancestors;
        StateType current = target;

        // W3C SCXML: Traverse from target to parent with cycle detection
        const int MAX_DEPTH = 100;  // Prevent infinite loops in invalid SCXML
        std::set<StateType> visited;
        visited.insert(target);
        visited.insert(parent);

        int depth = 0;

        while (depth < MAX_DEPTH) {
            auto parentOpt = getParent(current);
            if (!parentOpt.has_value()) {
                // Reached root without finding parent - broken hierarchy
                break;
            }

            StateType parentState = parentOpt.value();

            if (parentState == parent) {
                // Reached desired parent - stop
                break;
            }

            // W3C SCXML: Detect cycles (invalid SCXML)
            if (visited.find(parentState) != visited.end()) {
                throw std::runtime_error("W3C SCXML 3.6: Cycle detected in state hierarchy. "
                                         "State has circular ancestor path.");
            }
            visited.insert(parentState);

            ancestors.push_back(parentState);
            current = parentState;
            depth++;
        }

        if (depth >= MAX_DEPTH) {
            throw std::runtime_error("W3C SCXML 3.6: Maximum hierarchy depth exceeded. "
                                     "State has ancestor chain > " +
                                     std::to_string(MAX_DEPTH) + " levels.");
        }

        // W3C SCXML 3.13: Reverse to get document order (top-to-bottom)
        std::reverse(ancestors.begin(), ancestors.end());
        ancestors.push_back(target);

        return ancestors;
    }

    /**
     * @brief W3C SCXML 3.6: Optimize entry order across multiple deep targets
     *
     * Deduplicates common ancestors while preserving document order.
     *
     * Example:
     *   paths = [
     *     [s11, s11p1, s11p11, s11p112],  // First target path
     *     [s11, s11p1, s11p12, s11p122]   // Second target path
     *   ]
     *   result = [s11, s11p1, s11p11, s11p112, s11p12, s11p122]
     *
     * Common ancestors (s11, s11p1) entered once, then diverging paths.
     *
     * @tparam StateType State enum type (AOT) or std::string (Interpreter)
     * @param paths Ancestor paths for all deep initial targets
     * @return Optimized entry order with deduplicated ancestors
     */
    template <typename StateType>
    static std::vector<StateType> optimizeEntryOrder(const std::vector<std::vector<StateType>> &paths) {
        std::vector<StateType> optimized;
        std::set<StateType> seen;

        // W3C SCXML 3.13: Preserve document order
        for (const auto &path : paths) {
            for (const auto &state : path) {
                if (seen.find(state) == seen.end()) {
                    optimized.push_back(state);
                    seen.insert(state);
                }
            }
        }

        return optimized;
    }

    /**
     * @brief W3C SCXML 3.6: Enter deep initial targets with optimized traversal
     *
     * Single Source of Truth for deep state entry logic.
     * Shared between Interpreter engine and AOT generated code.
     *
     * Algorithm:
     * 1. Calculate ancestor paths for all targets
     * 2. Optimize entry order (deduplicate common ancestors)
     * 3. Execute entry actions in optimized order
     *
     * Example usage (AOT):
     *   StateEntryHelper::enterDeepTargets<State>(
     *       State::S1,
     *       {State::S11p112, State::S11p122},
     *       [](State s) { return Policy::getParent(s); },
     *       [&](State s) { executeEntryActions(s, engine); }
     *   );
     *
     * Example usage (Interpreter):
     *   StateEntryHelper::enterDeepTargets<std::string>(
     *       "s1",
     *       {"s11p112", "s11p122"},
     *       [this](const std::string& s) { return getParent(s); },
     *       [this](const std::string& s) { executeEntryActions(s); }
     *   );
     *
     * @tparam StateType State enum type (AOT) or std::string (Interpreter)
     * @tparam GetParentFunc Function: StateType -> std::optional<StateType>
     * @tparam EntryFunc Function: (StateType) -> void
     *
     * @param parent Parent state containing the initial attribute
     * @param targets Deep initial target states (space-separated from initial attribute)
     * @param getParent Function to get parent of a state
     * @param executeEntry Function to execute entry actions for a state
     */
    template <typename StateType, typename GetParentFunc, typename EntryFunc>
    static void enterDeepTargets(StateType parent, const std::vector<StateType> &targets, GetParentFunc getParent,
                                 EntryFunc executeEntry) {
        if (targets.empty()) {
            return;
        }

        // W3C SCXML 3.6: Calculate ancestor paths for all targets
        std::vector<std::vector<StateType>> allPaths;

        for (const auto &target : targets) {
            auto path = calculateAncestorPath(target, parent, getParent);
            allPaths.push_back(path);
        }

        // W3C SCXML 3.6: Optimize entry order (deduplicate ancestors)
        auto optimizedOrder = optimizeEntryOrder(allPaths);

        // W3C SCXML 3.8: Execute entry actions in optimized order
        for (const auto &state : optimizedOrder) {
            executeEntry(state);
        }
    }
};

}  // namespace RSM
