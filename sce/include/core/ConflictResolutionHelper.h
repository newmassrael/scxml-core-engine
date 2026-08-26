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

#include "core/HierarchicalStateHelper.h"
#include "core/LogMacros.h"
#include "core/ParallelTransitionHelper.h"
#include "core/StatePolicyConcepts.h"
#include <algorithm>
#include <optional>
#include <type_traits>
#include <vector>

namespace SCE::Core {

// ═══════════════════════════════════════════════════════════════════════════════
// Unified Conflict Resolution Algorithms (Single Source of Truth)
// §scxml-D-removeConflictingTransitions: Shared by AOT engine (enum states) and Interpreter (string states)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * @brief Generic conflict resolution algorithms parameterized by state type
 *
 * @details
 * Eliminates duplication between AOT (enum-based StatePolicy) and Interpreter
 * (string-based lambda injection). Both engines delegate to these algorithms.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Single implementation for all state types
 * - Single Source of Truth: All conflict resolution algorithms centralized here
 * - W3C SCXML Perfect Compliance: Full Appendix D.2 algorithm
 */
struct ConflictResolutionAlgorithms {
    /**
     * @brief Transition descriptor for conflict resolution (§scxml-D-removeConflictingTransitions)
     *
     * @tparam StateType State identifier type (enum for AOT, std::string for Interpreter)
     */
    template <typename StateType> struct TransitionDescriptor {
        StateType source{};
        StateType target{};
        std::vector<StateType> exitSet;
        int transitionIndex = 0;
        bool hasActions = false;    // §scxml-3.13: Transition action metadata
        bool isInternal = false;    // §scxml-3.13: Whether transition is type="internal"
        bool isTargetless = false;  // §scxml-3.13: Whether transition has no target attribute
        bool isExternal = false;    // §scxml-3.13: Whether transition exits parallel state

        TransitionDescriptor() = default;

        TransitionDescriptor(StateType src, StateType tgt, int idx = 0, bool actions = false, bool internal = false,
                             bool targetless = false)
            : source(std::move(src)), target(std::move(tgt)), transitionIndex(idx), hasActions(actions),
              isInternal(internal), isTargetless(targetless) {}
    };

    /**
     * @brief Check if two exit sets have non-empty intersection (§scxml-D-removeConflictingTransitions)
     */
    template <typename StateType>
    static bool hasIntersection(const std::vector<StateType> &set1, const std::vector<StateType> &set2) {
        for (const auto &s1 : set1) {
            for (const auto &s2 : set2) {
                if (s1 == s2) {
                    return true;
                }
            }
        }
        return false;
    }

    /**
     * @brief Remove conflicting transitions (§scxml-D-removeConflictingTransitions)
     *
     * @tparam StateType State identifier type
     * @tparam GetParentFn Callable: (const StateType&) -> std::optional<StateType>
     * @tparam IsParallelFn Callable: (const StateType&) -> bool
     * @param enabledTransitions All enabled transitions (in document order)
     * @param getParent Function to get parent state
     * @param isParallelState Function to check if state is parallel
     * @return Filtered non-conflicting transition set (optimal transition set)
     */
    template <typename StateType, typename GetParentFn, typename IsParallelFn>
    [[nodiscard]] static std::vector<TransitionDescriptor<StateType>>
    removeConflictingTransitions(const std::vector<TransitionDescriptor<StateType>> &enabledTransitions,
                                 GetParentFn getParent, IsParallelFn isParallelState) {
        std::vector<TransitionDescriptor<StateType>> filteredTransitions;

        SCE_LOG_DEBUG("ConflictResolution::removeConflictingTransitions: Processing {} transitions",
                      enabledTransitions.size());

        for (const auto &t1 : enabledTransitions) {
            // §scxml-D-selectTransitions: the enabled set is an ORDERED SET, and
            // the same transition reached from two different atomic states is one
            // element of it, not two. A transition written on a `<parallel>` is
            // selected once per region by the ancestor walk, so this is the
            // ordinary case, not an edge one -- W3C test 403b turns on a
            // `<parallel>`-level `<assign>` running exactly once.
            //
            // Selection stops at the first enabled transition of a state, so
            // within one microstep a source contributes at most one transition
            // and (source, target) identifies it. `transitionIndex` deliberately
            // takes no part: each region numbers its own walk, so two regions
            // reporting the same ancestor transition disagree about it.
            const bool alreadySelected = std::any_of(filteredTransitions.begin(), filteredTransitions.end(),
                                                     [&t1](const TransitionDescriptor<StateType> &seen) {
                                                         return seen.source == t1.source && seen.target == t1.target;
                                                     });
            if (alreadySelected) {
                continue;
            }

            bool t1Preempted = false;
            std::vector<size_t> transitionsToRemove;

            for (size_t i = 0; i < filteredTransitions.size(); ++i) {
                const auto &t2 = filteredTransitions[i];

                // §scxml-D-removeConflictingTransitions: two transitions conflict
                // when their EXIT SETS intersect. A transition that exits nothing
                // therefore conflicts with nothing and can never be preempted --
                // which is exactly a targetless transition, whose
                // §scxml-D-computeExitSet contribution the appendix defines as
                // empty.
                //
                // The heuristics below stand in for an exit set this engine could
                // not always compute, and every one of them reads a targetless
                // transition as a self-transition on its own source. Left
                // ungated, a targetless `<transition event="*">` sitting in one
                // region was preempted whenever a sibling region's transition
                // exited the enclosing `<parallel>` -- the case W3C test 403c
                // marks "this transition never gets preempted, should fire twice".
                if (t1.exitSet.empty() || t2.exitSet.empty()) {
                    continue;
                }

                bool hasConflict = false;

                // §scxml-D-removeConflictingTransitions: Check if exit sets intersect (conflict)
                if (hasIntersection(t1.exitSet, t2.exitSet)) {
                    hasConflict = true;
                }

                // §scxml-D-removeConflictingTransitions: Target/source conflict detection
                if (!hasConflict) {
                    if (t1.target == t2.source || t2.target == t1.source) {
                        hasConflict = true;
                    }
                }

                // §scxml-3.13: Parallel state conflict detection
                if (!hasConflict && !(t2.isInternal && t2.source == t2.target)) {
                    for (const auto &exitState : t1.exitSet) {
                        if (isParallelState(exitState)) {
                            if (HierarchicalAlgorithms::isDescendantOf(t2.source, exitState, getParent)) {
                                hasConflict = true;
                                break;
                            }
                        }
                    }
                }

                // Check reverse: t2 exits parallel state that is ancestor of t1's source
                if (!hasConflict && !(t1.isInternal && t1.source == t1.target)) {
                    for (const auto &exitState : t2.exitSet) {
                        if (isParallelState(exitState)) {
                            if (HierarchicalAlgorithms::isDescendantOf(t1.source, exitState, getParent)) {
                                hasConflict = true;
                                break;
                            }
                        }
                    }
                }

                if (hasConflict) {
                    // §scxml-D-removeConflictingTransitions: Preemption rules
                    if (t1.target == t2.source) {
                        transitionsToRemove.push_back(i);
                    } else if (t2.target == t1.source) {
                        t1Preempted = true;
                    } else if (HierarchicalAlgorithms::isDescendantOf(t1.source, t2.source, getParent)) {
                        transitionsToRemove.push_back(i);
                    } else {
                        t1Preempted = true;
                    }
                }
            }

            if (!t1Preempted) {
                for (auto it = transitionsToRemove.rbegin(); it != transitionsToRemove.rend(); ++it) {
                    filteredTransitions.erase(filteredTransitions.begin() + static_cast<long>(*it));
                }
                filteredTransitions.push_back(t1);
            }
        }

        SCE_LOG_DEBUG("ConflictResolution::removeConflictingTransitions: Filtered to {} transitions",
                      filteredTransitions.size());

        return filteredTransitions;
    }
};

// ═══════════════════════════════════════════════════════════════════════════════
// AOT Engine Wrapper (delegates to ConflictResolutionAlgorithms)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * @brief §scxml-D-removeConflictingTransitions Conflict Resolution Helper (AOT wrapper)
 *
 * @details
 * Thin wrapper around ConflictResolutionAlgorithms for AOT engine compatibility.
 * Binds StatePolicy static methods to the generic algorithm interface.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication Principle: Delegates to unified ConflictResolutionAlgorithms
 * - Single Source of Truth: All conflict resolution logic in ConflictResolutionAlgorithms
 * - W3C SCXML Perfect Compliance: Full implementation of Appendix D.2 algorithm
 */
#if __cpp_concepts >= 202002L
template <ParallelStatePolicy StatePolicy> class ConflictResolutionHelper {
#else
template <typename StatePolicy> class ConflictResolutionHelper {
#endif

public:
    using State = typename StatePolicy::State;
    using TransitionDescriptor = ConflictResolutionAlgorithms::TransitionDescriptor<State>;

    /**
     * @brief Compute exit set for a single transition
     *
     * @details
     * §scxml-D-computeExitSet: the active states that are proper descendants of
     * the transition's domain. It is read off the CONFIGURATION, which is why
     * the caller has to hand one over — the same set the microstep exits, and
     * the set `removeConflictingTransitions` below intersects.
     *
     * ARCHITECTURE.md Zero Duplication: Delegates to ParallelTransitionHelper for exit set computation.
     * Single Source of Truth - same algorithm used by AOT engine microstep execution.
     *
     * @param source Source state of transition
     * @param target Target state of transition
     * @param configuration The currently active states
     * @return Exit set (states to be exited)
     *
     * @par Thread Safety
     * Thread-safe and reentrant.
     *
     * @par Performance
     * - Time Complexity: O(|configuration| * depth)
     * - Space Complexity: O(|configuration|)
     *
     * @par Example
     * @code
     * // Given hierarchy: S0 -> { S01 -> S011, S02 }, configuration [S0, S01, S011]
     * // Transition from S011 to S02
     * auto exitSet = ConflictResolutionHelper<Policy>::computeExitSet(
     *     State::S011, State::S02, false, false, {State::S0, State::S01, State::S011});
     * // Returns: [S01, S011] (the active proper descendants of the domain S0)
     * @endcode
     */
    static std::vector<State> computeExitSet(State source, State target, bool isInternal, bool isTargetless,
                                             const std::vector<State> &configuration) {
        // ARCHITECTURE.MD Zero Duplication: Delegate to ParallelTransitionHelper
        // Construct minimal Transition descriptor for exit set computation
        typename ParallelTransitionHelper::Transition<State> trans;
        trans.source = source;
        trans.targets = {target};
        trans.isInternal = isInternal;      // §scxml-3.13: Pass internal transition type
        trans.isTargetless = isTargetless;  // §scxml-3.13: Pass targetless transition flag

        // §scxml-D-computeExitSet: Use shared Helper for exit set computation
        auto exitSetUnordered = ParallelTransitionHelper::computeExitSet<State, StatePolicy>(trans, configuration);

        // Convert unordered_set to vector for conflict resolution algorithm
        std::vector<State> exitSet(exitSetUnordered.begin(), exitSetUnordered.end());

        SCE_LOG_DEBUG("ConflictResolutionHelper::computeExitSet: Transition {} -> {} exits {} states",
                      static_cast<int>(source), static_cast<int>(target), exitSet.size());

        return exitSet;
    }

    /**
     * @brief Check if two exit sets have non-empty intersection
     *
     * @details
     * §scxml-D-removeConflictingTransitions: Two transitions conflict if their exit sets intersect.
     * Exit set intersection means both transitions would exit at least one common state.
     *
     * @param set1 First exit set
     * @param set2 Second exit set
     * @return true if sets have common element, false otherwise
     *
     * @par Thread Safety
     * Thread-safe and reentrant.
     *
     * @par Performance
     * - Time Complexity: O(n * m) where n, m are set sizes
     * - Space Complexity: O(1)
     *
     * @par Example
     * @code
     * std::vector<State> exitSet1 = {State::S011, State::S01};
     * std::vector<State> exitSet2 = {State::S012, State::S01};
     * bool conflict = ConflictResolutionHelper<Policy>::hasIntersection(exitSet1, exitSet2);
     * // Returns: true (both exit S01)
     * @endcode
     */
    static bool hasIntersection(const std::vector<State> &set1, const std::vector<State> &set2) {
        return ConflictResolutionAlgorithms::hasIntersection(set1, set2);
    }

    /**
     * @brief Remove conflicting transitions (§scxml-D-removeConflictingTransitions)
     *
     * Delegates to ConflictResolutionAlgorithms with StatePolicy bindings.
     */
    static std::vector<TransitionDescriptor>
    removeConflictingTransitions(const std::vector<TransitionDescriptor> &enabledTransitions) {
        return ConflictResolutionAlgorithms::removeConflictingTransitions(
            enabledTransitions, [](State s) { return StatePolicy::getParent(s); },
            [](State s) { return StatePolicy::isParallelState(s); });
    }
};

}  // namespace SCE::Core
