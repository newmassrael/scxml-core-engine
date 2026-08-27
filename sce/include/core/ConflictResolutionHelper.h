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
     * @param enabledTransitions All enabled transitions (in document order)
     * @param getParent Function to get parent state
     * @return Filtered non-conflicting transition set (optimal transition set)
     *
     * @note No `isParallelState` predicate: the procedure asks nothing about
     *       `<parallel>` states. It once did, to stand in for an exit set that
     *       could not name a sibling region; Appendix D's computeExitSet names
     *       them now, so the intersection below is the entire conflict test.
     */
    template <typename StateType, typename GetParentFn>
    [[nodiscard]] static std::vector<TransitionDescriptor<StateType>>
    removeConflictingTransitions(const std::vector<TransitionDescriptor<StateType>> &enabledTransitions,
                                 GetParentFn getParent) {
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
                // when their EXIT SETS intersect. That is the whole test the
                // appendix states, and it is now the whole test made here.
                //
                // Three rules used to sit beside it -- a target/source equality
                // check and a `<parallel>`-ancestor check in each direction --
                // and none is in the appendix. They stood in for an exit set
                // this engine could not compute: assembled from one region's own
                // chain, a set could not name the sibling regions a transition
                // leaving the `<parallel>` exits, so the intersection came back
                // empty for transitions that plainly conflict and something had
                // to say so. §scxml-D-computeExitSet now reads the CONFIGURATION
                // in every engine, so the intersection answers on its own.
                //
                // Two consequences worth naming, because the removed rules made
                // both of them invisible:
                //  - A transition that exits nothing conflicts with nothing and
                //    can never be preempted. That is exactly a targetless
                //    transition -- the appendix guards computeExitSet with `if
                //    t.target` -- and it is what W3C test 403c means by "this
                //    transition never gets preempted, should fire twice". The
                //    old rules each read a targetless transition as a
                //    self-transition on its own source, which is why they needed
                //    an empty-exit-set gate ahead of them to keep 403c green.
                //  - Two transitions in different regions of one `<parallel>`
                //    have domains in disjoint subtrees, so their exit sets are
                //    disjoint and both survive. §scxml-3.4 requires exactly that.
                if (!hasIntersection(t1.exitSet, t2.exitSet)) {
                    continue;
                }

                // §scxml-D-removeConflictingTransitions: the descendant source
                // wins. The appendix leaves the loop here rather than gathering
                // more removals it would discard.
                if (HierarchicalAlgorithms::isDescendantOf(t1.source, t2.source, getParent)) {
                    transitionsToRemove.push_back(i);
                } else {
                    t1Preempted = true;
                    break;
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
            enabledTransitions, [](State s) { return StatePolicy::getParent(s); });
    }
};

}  // namespace SCE::Core
