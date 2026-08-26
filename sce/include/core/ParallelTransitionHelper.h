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
#include <algorithm>
#include <functional>
#include <optional>
#include <unordered_set>
#include <vector>

namespace SCE::Core {

/**
 * @brief Appendix D's computeExitSet and getTransitionDomain, over a configuration
 *
 * @details
 * The appendix computes an exit set FROM THE CONFIGURATION: for a transition
 * that has a target, it is every active state that is a proper descendant of
 * the transition's domain. Walking only the source's own ancestor chain answers
 * that same set in a machine whose configuration is a single chain, and a
 * strictly smaller one the moment a `<parallel>` is active below the domain --
 * the sibling regions are descendants of the domain too, and the chain never
 * reaches them.
 *
 * The gap is not cosmetic. `removeConflictingTransitions` decides conflict by
 * INTERSECTING these sets, so a chain-shaped exit set makes a transition that
 * tears a whole `<parallel>` down look disjoint from the sibling region's
 * transition on the same event.
 *
 * Lambda-injected so the Interpreter (states are `std::string`) and the AOT
 * engine (states are an enum) share one implementation -- the same shape
 * `ConflictResolutionAlgorithms` uses, and the reason this engine's conflict
 * set and its microstep set are now one procedure rather than two that agree
 * only where no `<parallel>` is active.
 */
struct ExitSetAlgorithms {
    /**
     * @brief Appendix D's getTransitionDomain
     *
     * @param isDomainCandidate `isCompoundStateOrScxmlElement`; a `<parallel>` answers false
     * @return The domain, or std::nullopt when it is the `<scxml>` element --
     *         which has no state identifier in either engine, and which callers
     *         read as "every active state lies below it"
     */
    template <typename StateType, typename GetParentFn, typename IsDomainCandidateFn>
    [[nodiscard]] static std::optional<StateType>
    getTransitionDomain(const StateType &source, const std::vector<StateType> &targets, bool isInternal,
                        GetParentFn getParent, IsDomainCandidateFn isDomainCandidate) {
        const auto containedBy = [&](const StateType &ancestor) {
            return std::all_of(targets.begin(), targets.end(), [&](const StateType &target) {
                return HierarchicalAlgorithms::isDescendantOf(target, ancestor, getParent);
            });
        };

        // §scxml-D-getTransitionDomain: an internal transition whose targets all
        // lie below a compound source has the SOURCE as its domain, so the
        // source stays active and only its active descendants are exited. That
        // is not the same as exiting nothing: a transition rooted at one of
        // those descendants exits it too, and the appendix expects the two to be
        // found in conflict.
        if (isInternal && !targets.empty() && isDomainCandidate(source) && containedBy(source)) {
            return source;
        }

        // §scxml-D-findLCCA over the source and EVERY target at once -- the
        // first legal candidate that contains all of them. Combining pairwise
        // answers can only widen the domain.
        StateType current = source;
        while (true) {
            auto parent = getParent(current);
            if (!parent.has_value()) {
                return std::nullopt;  // Out of proper ancestors: the domain is the <scxml> element.
            }
            current = parent.value();

            if (!isDomainCandidate(current)) {
                continue;  // A <parallel> is not a domain.
            }
            if (containedBy(current)) {
                return current;
            }
        }
    }

    /**
     * @brief Appendix D's computeExitSet: the active proper descendants of the domain
     *
     * @param configuration The states currently active — the appendix's `configuration`
     * @return The exited states, in the configuration's own order
     */
    template <typename StateType, typename GetParentFn, typename IsDomainCandidateFn>
    [[nodiscard]] static std::vector<StateType>
    computeExitSet(const StateType &source, const std::vector<StateType> &targets, bool isInternal, bool isTargetless,
                   const std::vector<StateType> &configuration, GetParentFn getParent,
                   IsDomainCandidateFn isDomainCandidate) {
        std::vector<StateType> exitSet;

        // §scxml-D-computeExitSet: the appendix guards the whole computation
        // with `if t.target`, so a transition without one exits nothing at all
        // and can therefore never be preempted.
        if (isTargetless || targets.empty()) {
            return exitSet;
        }

        const auto domain = getTransitionDomain(source, targets, isInternal, getParent, isDomainCandidate);

        exitSet.reserve(configuration.size());
        for (const auto &state : configuration) {
            if (!domain.has_value()) {
                // The domain is the <scxml> element and every active state is a
                // descendant of it -- the sibling regions of an enclosing
                // `<parallel>` included.
                exitSet.push_back(state);
                continue;
            }
            if (state == domain.value()) {
                continue;  // The domain itself is not exited.
            }
            if (HierarchicalAlgorithms::isDescendantOf(state, domain.value(), getParent)) {
                exitSet.push_back(state);
            }
        }

        return exitSet;
    }
};

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
     * @brief §scxml-D-computeExitSet: the active states this transition exits
     *
     * @details
     * The appendix reads the exit set off the CONFIGURATION -- every active
     * state that is a proper descendant of the transition's domain -- so this
     * needs the configuration and cannot be answered from the hierarchy alone.
     * Walking the source's own ancestor chain, which is what stood here, names
     * the same states only while no `<parallel>` is active below the domain; a
     * transition that tears one down then looked disjoint from the sibling
     * region's transition on the same event, and `removeConflictingTransitions`
     * intersects exactly this set.
     *
     * @tparam StateType State enum or identifier type
     * @tparam PolicyType Policy class with state hierarchy
     * @param transition Transition to compute exit set for
     * @param configuration The currently active states
     * @return Set of states that will be exited
     */
#if __cpp_concepts >= 202002L
    template <typename StateType, ParallelStatePolicy PolicyType>
#else
    template <typename StateType, typename PolicyType>
#endif
    static std::unordered_set<StateType> computeExitSet(const Transition<StateType> &transition,
                                                        const std::vector<StateType> &configuration) {
        using Hierarchy = SCE::Core::HierarchicalStateHelper<PolicyType>;

        // ARCHITECTURE.md Zero Duplication: one appendix procedure, bound here
        // to a StatePolicy and in the Interpreter to lambdas over state IDs.
        const auto exited = ExitSetAlgorithms::computeExitSet(
            transition.source, transition.targets, transition.isInternal, transition.isTargetless, configuration,
            [](const StateType &s) { return PolicyType::getParent(s); },
            [](const StateType &s) { return Hierarchy::isTransitionDomainCandidate(s); });

        std::unordered_set<StateType> exitSet(exited.begin(), exited.end());

        return exitSet;
    }

    // §scxml-D-removeConflictingTransitions lives in ConflictResolutionHelper,
    // where both engines reach it. A second resolver stood here -- a depth sort
    // and a greedy scan, which is not the appendix's ordered-set procedure -- and
    // nothing in the tree called it. It is gone rather than carried forward with
    // the configuration this exit set now needs: two implementations of one
    // appendix procedure can only drift, and the unreachable one drifts unseen.

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

        // §scxml-D-computeExitSet takes a LIST of transitions and unions their
        // exit sets. Each one is the same procedure conflict resolution
        // intersects -- the microstep and the resolver must not be able to
        // disagree about which states a transition exits, and they did while
        // this walked the configuration itself and `computeExitSet` walked the
        // source's ancestor chain.
        for (const auto &trans : transitions) {
            for (const auto &state : computeExitSet<StateType, PolicyType>(trans, activeStates)) {
                if (std::find(statesToExit.begin(), statesToExit.end(), state) == statesToExit.end()) {
                    statesToExit.push_back(state);
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

    // §scxml-D-getTransitionDomain used to be spelled twice below this line --
    // `isInternalToDescendant` and `computeEffectiveLCA`, private helpers whose
    // only callers were the two exit-set walks above. Both now go through
    // `ExitSetAlgorithms::getTransitionDomain`, which is the one place the rule
    // is written and the one the Interpreter can reach as well.
};

}  // namespace SCE::Core
