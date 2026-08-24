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
#include "core/StatePolicyConcepts.h"
#include <functional>
#include <optional>
#include <unordered_set>
#include <vector>

namespace SCE::Core {

/**
 * @brief Helper functions for parallel state transition conflict detection
 *
 * §scxml-D-removeConflictingTransitions: optimal enabled transition set
 * - Optimal enabled transition set: Select non-conflicting transitions
 * - Conflict detection: Two transitions conflict if they exit the same state
 *
 * Shared between Interpreter and AOT engines following Zero Duplication Principle.
 */
class ParallelTransitionHelper {
public:
    /**
     * @brief Transition descriptor for conflict detection
     */
    template <typename StateType> struct Transition {
        StateType source;                       // Source state
        std::vector<StateType> targets;         // Target states
        std::unordered_set<StateType> exitSet;  // States exited by this transition

        // §scxml-3.13: Additional metadata for AOT engine compatibility
        int transitionIndex = 0;    // Index for executeTransitionActions
        bool hasActions = false;    // Whether transition has executable content
        bool isInternal = false;    // §scxml-3.13: Whether transition is type="internal"
        bool isTargetless = false;  // §scxml-3.13: Whether transition has no target (consumes event only)

        Transition() = default;

        Transition(StateType src, std::vector<StateType> tgts) : source(src), targets(std::move(tgts)) {}

        // Constructor with full metadata (for AOT engine)
        Transition(StateType src, std::vector<StateType> tgts, int idx, bool actions, bool internal = false,
                   bool targetless = false)
            : source(src), targets(std::move(tgts)), transitionIndex(idx), hasActions(actions), isInternal(internal),
              isTargetless(targetless) {}
    };

    /**
     * @brief Compute exit set for a transition
     *
     * §scxml-3.13: Exit set = all states exited when taking this transition
     * = source state + ancestors up to (but not including) LCA with targets
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param transition Transition to compute exit set for
     * @return Set of states that will be exited
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::unordered_set<StateType> computeExitSet(const Transition<StateType> &transition) {
        std::unordered_set<StateType> exitSet;

        // §scxml-3.13: Targetless internal transitions (consumes event only, no exit/enter)
        // These transitions execute actions but do not change state - empty exit set
        if (transition.isTargetless) {
            return exitSet;  // Empty exit set for targetless transition
        }

        // §scxml-3.13: Internal transition to compound descendant - source stays active
        if (transition.isInternal) {
            bool allTargetsAreDescendants = true;
            for (const auto &target : transition.targets) {
                if (!isInternalToDescendant<StateType, PolicyType>(transition.source, target)) {
                    allTargetsAreDescendants = false;
                    break;
                }
            }

            if (allTargetsAreDescendants) {
                return exitSet;  // Empty set - source and ancestors remain active
            }
        }

        // External transition (or internal transition that behaves as external)
        // §scxml-D-getTransitionDomain: the domain is findLCCA over the source
        // and every target — the candidates are compound `<state>`s and the
        // `<scxml>` element, never a `<parallel>`. This set is what conflict
        // resolution intersects, so answering a `<parallel>` here made a
        // region-root transition look non-conflicting with the sibling
        // region's transition on the same event.
        // The appendix takes findLCCA over the whole list at once — the first
        // legal candidate that contains EVERY target — rather than combining
        // pairwise answers, which can only widen the domain.
        std::optional<StateType> lca = std::nullopt;
        {
            using Hierarchy = SCE::Core::HierarchicalStateHelper<PolicyType>;
            StateType current = transition.source;
            while (true) {
                auto parent = PolicyType::getParent(current);
                if (!parent.has_value()) {
                    break;  // Out of ancestors: the domain is the <scxml> element.
                }
                current = parent.value();

                if (!Hierarchy::isTransitionDomainCandidate(current)) {
                    continue;  // A <parallel> is not a domain.
                }

                bool containsEveryTarget = true;
                for (const auto &target : transition.targets) {
                    if (!Hierarchy::isDescendantOf(target, current)) {
                        containsEveryTarget = false;
                        break;
                    }
                }

                if (containsEveryTarget) {
                    lca = current;
                    break;
                }
            }
        }

        // Collect all states from source up to (but not including) LCA
        auto current = transition.source;
        while (true) {
            exitSet.insert(current);

            auto parent = PolicyType::getParent(current);
            if (!parent.has_value()) {
                break;
            }

            // Stop before LCA
            if (lca.has_value() && parent.value() == lca.value()) {
                break;
            }

            current = parent.value();
        }

        return exitSet;
    }

    /**
     * @brief Check if two transitions conflict
     *
     * §scxml-D-removeConflictingTransitions: two transitions conflict if their exit sets intersect
     * (they would exit the same state, which is invalid).
     *
     * §scxml-3.13: Special case for parallel states - if a transition exits a parallel state,
     * it conflicts with any transition whose source is a descendant of that parallel state,
     * even if their exit sets don't explicitly intersect (because exiting the parallel state
     * implicitly exits all its child regions).
     *
     * @tparam StateType State enum or identifier type
     * @param t1 First transition
     * @param t2 Second transition
     * @return true if transitions conflict
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static bool hasConflict(const Transition<StateType> &t1, const Transition<StateType> &t2) {
        // Check if exit sets intersect
        for (const auto &state : t1.exitSet) {
            if (t2.exitSet.find(state) != t2.exitSet.end()) {
                return true;  // Conflict: both exit the same state
            }
        }

        // §scxml-3.13: Parallel state conflict detection
        // If t1 exits a parallel state, it conflicts with any transition whose source is a descendant of that parallel
        // state
        for (const auto &exitState : t1.exitSet) {
            if (PolicyType::isParallelState(exitState)) {
                if (PolicyType::isDescendantOf(t2.source, exitState)) {
                    return true;  // Conflict: t1 exits parallel ancestor of t2's source
                }
            }
        }

        // Check reverse: t2 exits parallel state that is ancestor of t1's source
        for (const auto &exitState : t2.exitSet) {
            if (PolicyType::isParallelState(exitState)) {
                if (PolicyType::isDescendantOf(t1.source, exitState)) {
                    return true;  // Conflict: t2 exits parallel ancestor of t1's source
                }
            }
        }

        return false;
    }

    /**
     * @brief Select optimal enabled transition set (non-conflicting)
     *
     * §scxml-D-removeConflictingTransitions: from all enabled transitions, select maximal
     * non-conflicting subset. Preemption rule: Transitions in child states
     * have priority over parent states.
     *
     * Algorithm:
     * 1. Sort transitions by state hierarchy depth (deeper first)
     * 2. Greedily select transitions that don't conflict with already selected
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param enabledTransitions All enabled transitions for current event
     * @return Non-conflicting subset of transitions to execute
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::vector<Transition<StateType>>
    selectOptimalTransitions(std::vector<Transition<StateType>> &enabledTransitions) {
        // Compute exit sets for all transitions
        for (auto &transition : enabledTransitions) {
            transition.exitSet = computeExitSet<StateType, PolicyType>(transition);
        }

        // Sort by state hierarchy depth (deeper states first - preemption)
        std::sort(enabledTransitions.begin(), enabledTransitions.end(),
                  [](const Transition<StateType> &a, const Transition<StateType> &b) {
                      return getDepth<StateType, PolicyType>(a.source) > getDepth<StateType, PolicyType>(b.source);
                  });

        // Greedy selection: Pick transitions that don't conflict with already selected
        std::vector<Transition<StateType>> selectedTransitions;

        for (const auto &transition : enabledTransitions) {
            bool conflicts = false;

            // Check if this transition conflicts with any already selected
            for (const auto &selectedTransition : selectedTransitions) {
                if (hasConflict<StateType, PolicyType>(transition, selectedTransition)) {
                    conflicts = true;
                    break;
                }
            }

            if (!conflicts) {
                selectedTransitions.push_back(transition);
            }
        }

        return selectedTransitions;
    }

    /**
     * @brief Get hierarchy depth of a state
     *
     * Depth = number of ancestors (0 for root states)
     * Used for preemption: deeper states have priority
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param state State to get depth for
     * @return Depth (0 = root)
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static int getDepth(StateType state) {
        int depth = 0;
        auto current = state;

        while (true) {
            auto parent = PolicyType::getParent(current);
            if (!parent.has_value()) {
                break;
            }
            depth++;
            current = parent.value();
        }

        return depth;
    }

    /**
     * @brief Compute and sort states to exit for microstep execution
     *
     * ARCHITECTURE.MD: Zero Duplication Principle - Shared exit computation logic
     * §scxml-D-computeExitSet Step 1: Collect unique source states from transitions
     * §scxml-3.13: Sort by reverse document order (deepest/rightmost first)
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with getDocumentOrder()
     * @param transitions Transitions to execute
     * @param activeStates Current active states
     * @return States to exit in reverse document order
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::vector<StateType> computeStatesToExit(const std::vector<Transition<StateType>> &transitions,
                                                      const std::vector<StateType> &activeStates) {
        std::vector<StateType> statesToExit;

        // §scxml-D-computeExitSet: For each transition, compute LCA-based exit set
        // Exit set = all active states that are descendants of LCA (excluding LCA itself)
        for (const auto &trans : transitions) {
            // §scxml-3.13: Targetless transitions do not exit any states
            // These transitions execute actions but do not change state configuration
            if (trans.isTargetless) {
                continue;  // Skip exit computation for targetless transition
            }

            // Handle each target separately (parallel states may have multiple targets)
            if (trans.targets.empty()) {
                continue;  // No target, no exit needed
            }

            for (const auto &target : trans.targets) {
                // §scxml-3.13: Compute effective LCA considering internal transition semantics
                auto lca = computeEffectiveLCA<StateType, PolicyType>(trans.source, target, trans.isInternal);

                if (!lca.has_value()) {
                    // §scxml-D-computeExitSet: the domain is the <scxml>
                    // element, and EVERY active state is a descendant of it —
                    // so the whole configuration exits and the target's entry
                    // chain is walked fresh.
                    //
                    // Walking only the source's own ancestor chain, which is
                    // what stood here, is the same set in a machine whose
                    // configuration is a single chain — and silently wrong the
                    // moment a `<parallel>` is active, because the sibling
                    // regions are descendants of the domain too. That is the
                    // half of a region-root external transition the domain fix
                    // above cannot supply on its own.
                    for (const auto &activeState : activeStates) {
                        bool alreadyInSet =
                            std::find(statesToExit.begin(), statesToExit.end(), activeState) != statesToExit.end();
                        if (!alreadyInSet) {
                            statesToExit.push_back(activeState);
                        }
                    }
                } else {
                    // §scxml-D-computeExitSet: exit every active descendant of
                    // the transition's domain; the domain itself stays.
                    //
                    // The domain can never be the source: the ancestor search
                    // chooses among proper ancestors, which exclude the state
                    // itself, and the one other answer `computeEffectiveLCA`
                    // gives — the source, for an internal transition to a
                    // descendant — is handled by the `isInternal` branch above.
                    // An external self-transition is therefore an ordinary
                    // descendant of its parent domain and needs no case of its
                    // own.
                    for (const auto &activeState : activeStates) {
                        if (activeState == lca.value()) {
                            continue;  // The domain is not exited
                        }

                        bool shouldExit = false;
                        auto current = activeState;

                        while (true) {
                            auto parent = PolicyType::getParent(current);
                            if (!parent.has_value()) {
                                break;  // Reached root without finding the domain
                            }

                            if (parent.value() == lca.value()) {
                                shouldExit = true;
                                break;
                            }
                            current = parent.value();
                        }

                        // Add to exit set if should exit and not already present
                        if (shouldExit) {
                            bool alreadyInSet =
                                std::find(statesToExit.begin(), statesToExit.end(), activeState) != statesToExit.end();
                            if (!alreadyInSet) {
                                statesToExit.push_back(activeState);
                            }
                        }
                    }
                }
            }
        }

        // §scxml-3.13: Sort by REVERSE document order (exit deepest/rightmost first)
        std::sort(statesToExit.begin(), statesToExit.end(), [](StateType a, StateType b) {
            return PolicyType::getDocumentOrder(a) > PolicyType::getDocumentOrder(b);
        });

        return statesToExit;
    }

    /**
     * @brief Sort transitions by source state document order
     *
     * ARCHITECTURE.MD: Zero Duplication Principle - Shared sorting logic
     * §scxml-D-executeTransitionContent Step 3: Execute transition content in document order
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with getDocumentOrder()
     * @param transitions Transitions to sort
     * @return Sorted transitions (by source state document order)
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::vector<Transition<StateType>> sortTransitionsBySource(std::vector<Transition<StateType>> transitions) {
        std::sort(transitions.begin(), transitions.end(),
                  [](const Transition<StateType> &a, const Transition<StateType> &b) {
                      return PolicyType::getDocumentOrder(a.source) < PolicyType::getDocumentOrder(b.source);
                  });

        return transitions;
    }

    /**
     * @brief Sort transitions by target state document order
     *
     * ARCHITECTURE.MD: Zero Duplication Principle - Shared sorting logic
     * §scxml-D-enterStates Step 4-5: Enter target states in document order
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with getDocumentOrder()
     * @param transitions Transitions to sort
     * @return Sorted transitions (by target state document order)
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::vector<Transition<StateType>> sortTransitionsByTarget(std::vector<Transition<StateType>> transitions) {
        std::sort(transitions.begin(), transitions.end(),
                  [](const Transition<StateType> &a, const Transition<StateType> &b) {
                      StateType targetA = a.targets.empty() ? a.source : a.targets[0];
                      StateType targetB = b.targets.empty() ? b.source : b.targets[0];
                      return PolicyType::getDocumentOrder(targetA) < PolicyType::getDocumentOrder(targetB);
                  });

        return transitions;
    }

    /**
     * @brief Sort states for exit by depth and document order
     *
     * ARCHITECTURE.MD: Zero Duplication Principle - Shared exit ordering logic
     * §scxml-3.13: States exit in order (deepest first, then reverse document order)
     * Shared between Interpreter and AOT engines.
     *
     * @tparam StateType State identifier type (string or enum)
     * @tparam GetDepthFunc Callable that returns depth for a state
     * @tparam GetDocOrderFunc Callable that returns document order for a state
     * @param states States to sort
     * @param getDepth Function to get state depth (0 = root)
     * @param getDocOrder Function to get document order
     * @return Sorted states (deepest first, reverse document order for same depth)
     */
    template <typename StateType, typename GetDepthFunc, typename GetDocOrderFunc>
    static std::vector<StateType> sortStatesForExit(std::vector<StateType> states, GetDepthFunc getDepth,
                                                    GetDocOrderFunc getDocOrder) {
        std::sort(states.begin(), states.end(), [&](const StateType &a, const StateType &b) {
            // §scxml-3.13: Primary sort by depth (deepest first)
            int depthA = getDepth(a);
            int depthB = getDepth(b);

            if (depthA != depthB) {
                return depthA > depthB;  // Deeper states exit first
            }

            // §scxml-3.13: Secondary sort by reverse document order
            return getDocOrder(a) > getDocOrder(b);  // Later states exit first
        });

        return states;
    }

    /**
     * @brief Check if a transition is enabled for an event
     *
     * A transition is enabled if:
     * 1. Source state is active
     * 2. Event matches transition's event descriptor
     * 3. Condition evaluates to true (if present)
     *
     * @tparam StateType State enum or identifier type
     * @tparam EventType Event enum or identifier type
     * @param sourceState Source state of transition
     * @param transitionEvent Event descriptor of transition
     * @param currentEvent Current event being processed
     * @param isActive Predicate to check if source state is active
     * @return true if transition is enabled
     */
    template <typename StateType, typename EventType>
    static bool isTransitionEnabled(StateType sourceState, EventType transitionEvent, EventType currentEvent,
                                    std::function<bool(StateType)> isActive) {
        // Check if source state is active
        if (!isActive(sourceState)) {
            return false;
        }

        // Check if event matches (event matching logic is in EventMatchingHelper)
        // For now, simple equality check
        return transitionEvent == currentEvent;
    }

private:
    /**
     * @brief Check if a transition qualifies as internal-to-descendant
     *
     * §scxml-3.13: An internal transition does NOT exit its source state when:
     * 1. The source is a compound state (NOT parallel, NOT atomic)
     * 2. The target is a proper descendant of the source
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param source Source state of the transition
     * @param target Target state to check
     * @return true if source is compound and target is a proper descendant
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static bool isInternalToDescendant(StateType source, StateType target) {
        bool sourceIsCompound = PolicyType::isCompoundState(source) && !PolicyType::isParallelState(source);
        if (!sourceIsCompound) {
            return false;
        }
        return PolicyType::isDescendantOf(target, source) && target != source;
    }

    /**
     * @brief Compute effective LCA for a (source, target) pair considering internal transition semantics
     *
     * §scxml-3.13: For internal transitions where the source is compound and
     * the target is a proper descendant, the effective LCA is the source itself
     * (source stays active). For all other cases, standard LCA via hierarchy traversal.
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param source Source state of the transition
     * @param target Target state of the transition
     * @param isInternal Whether the transition is type="internal"
     * @return Effective LCA state, or nullopt if no common ancestor found
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::optional<StateType> computeEffectiveLCA(StateType source, StateType target, bool isInternal) {
        if (isInternal && isInternalToDescendant<StateType, PolicyType>(source, target)) {
            return source;  // Source is the LCA - don't exit it
        }
        // §scxml-D-getTransitionDomain: findLCCA, not findLCA. The appendix
        // filters the candidate ancestors with `isCompoundStateOrScxmlElement`,
        // so a `<parallel>` is never a domain — and a transition written on a
        // REGION ROOT is precisely the case where the two procedures differ.
        // Asking for a plain LCA answered the enclosing `<parallel>`, which
        // left the sibling regions unexited and their own transitions on the
        // same event unpreempted: measured 2026-08-25 as a configuration
        // holding BOTH children of the sibling region at once.
        return SCE::Core::HierarchicalStateHelper<PolicyType>::findLCCA(source, target);
    }
};

}  // namespace SCE::Core
