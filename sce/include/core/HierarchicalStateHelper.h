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

#include "core/LogMacros.h"
#include "core/StatePolicyConcepts.h"
#include <algorithm>
#include <optional>
#include <stdexcept>
#include <type_traits>
#include <vector>

namespace SCE::Core {

// ═══════════════════════════════════════════════════════════════════════════════
// C++17-compatible trait for ParallelStatePolicy detection
// Used in if constexpr to check if a policy supports parallel state operations.
// ═══════════════════════════════════════════════════════════════════════════════
template <typename P, typename = void> struct IsParallelStatePolicyTrait : std::false_type {};

template <typename P>
struct IsParallelStatePolicyTrait<
    P, std::void_t<decltype(P::isParallelState(std::declval<typename P::State>())),
                   decltype(P::getParallelRegions(std::declval<typename P::State>())),
                   decltype(P::isDescendantOf(std::declval<typename P::State>(), std::declval<typename P::State>())),
                   decltype(P::getDocumentOrder(std::declval<typename P::State>())),
                   decltype(P::isFinalState(std::declval<typename P::State>()))>> : std::true_type {};

template <typename P> inline constexpr bool IsParallelStatePolicy = IsParallelStatePolicyTrait<P>::value;

// ═══════════════════════════════════════════════════════════════════════════════
// Unified Hierarchical Algorithms (Single Source of Truth)
// §scxml-3.13: Shared by AOT engine (enum states) and Interpreter (string states)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * @brief Generic hierarchical state algorithms parameterized by state type
 *
 * @details
 * Eliminates duplication between AOT (enum-based StatePolicy) and Interpreter
 * (string-based lambda injection). Both engines delegate to these algorithms.
 *
 * What a microstep enters and exits is not here: Appendix D computes both
 * over the whole configuration and the whole target set, which a walk along
 * one state's ancestor chain cannot, and `ExitSetAlgorithms` /
 * `EntrySetAlgorithms` are those procedures. The entry and exit chains that
 * used to live here answered for one target at a time and are gone with the
 * engines that walked them.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Single implementation for all state types
 * - Single Source of Truth: All hierarchical algorithms centralized here
 */
struct HierarchicalAlgorithms {
    /**
     * @brief Find Least Common Ancestor of two states (§scxml-3.13)
     *
     * @tparam StateType State identifier type (enum for AOT, std::string for Interpreter)
     * @tparam GetParentFn Callable: (const StateType&) -> std::optional<StateType>
     * @return LCA state, or std::nullopt if no common ancestor
     */
    template <typename StateType, typename GetParentFn>
    [[nodiscard]] static std::optional<StateType> findLCA(const StateType &state1, const StateType &state2,
                                                          GetParentFn getParent) {
        // §scxml-D-findLCCA: the Least Common Compound Ancestor is the state that
        // is a proper ancestor of every state in the list and has no descendant
        // with that property. Both engines reach the procedure through here.
        //
        // The candidates come from the proper ancestors, which exclude the
        // state itself, so the two states being equal — an external
        // self-transition — resolves to the parent and never to the state.
        // Answering with the state instead left the exit-set walk, which
        // climbs from the source to just below this answer, without a stopping
        // point: it ran to the document root, the exit set then named the
        // enclosing `<parallel>`, and conflict resolution preempted the
        // sibling regions' own transitions on that same event — where every
        // region is required to take its enabled transition in one microstep.
        if (state1 == state2) {
            return getParent(state1);
        }

        // §scxml-3.13: Build ancestor chain starting from state1's parent (test 504)
        std::vector<StateType> ancestors1;
        ancestors1.reserve(8);

        auto parent1 = getParent(state1);
        if (parent1.has_value()) {
            StateType current = parent1.value();
            while (true) {
                ancestors1.push_back(current);
                auto parent = getParent(current);
                if (!parent.has_value()) {
                    break;
                }
                current = parent.value();
            }
        }

        // Walk up from state2 to find intersection with state1's ancestors
        StateType current = state2;
        while (true) {
            for (const auto &ancestor : ancestors1) {
                if (current == ancestor) {
                    return current;
                }
            }
            auto parent = getParent(current);
            if (!parent.has_value()) {
                break;
            }
            current = parent.value();
        }

        return std::nullopt;
    }

    /**
     * @brief Check if descendant is a child/grandchild of ancestor (§scxml-D-isDescendant)
     */
    template <typename StateType, typename GetParentFn>
    [[nodiscard]] static bool isDescendantOf(const StateType &descendant, const StateType &ancestor,
                                             GetParentFn getParent) {
        StateType current = descendant;
        while (true) {
            auto parent = getParent(current);
            if (!parent.has_value()) {
                return false;
            }
            if (parent.value() == ancestor) {
                return true;
            }
            current = parent.value();
        }
    }
};

/**
 * @brief The same questions, bound to a generated StatePolicy
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication Principle: Shared logic between Interpreter and AOT engines
 * - Static-First Principle: All hierarchy operations are Closed World
 *   (state structure known at compile-time from SCXML parse)
 */
#if __cpp_concepts >= 202002L
template <HierarchyPolicy StatePolicy> class HierarchicalStateHelper {
#else
template <typename StatePolicy> class HierarchicalStateHelper {
#endif

public:
    using State = typename StatePolicy::State;

    /// Whether `state` sits below another state rather than directly under
    /// the `<scxml>` element.
    static bool hasParent(State state) {
        return StatePolicy::getParent(state).has_value();
    }

    static std::optional<State> getParent(State state) {
        return StatePolicy::getParent(state);
    }

    /**
     * @brief §scxml-D-findLCCA: may this state be a domain?
     *
     * The `isCompoundStateOrScxmlElement` filter that procedure applies to the
     * candidate ancestors, spelled over a StatePolicy.
     *
     * A `<parallel>` answers false. Every policy reports it as compound, so the
     * two questions have to be asked together rather than one standing in for
     * the other.
     *
     * The `<scxml>` element is the other legal answer in the appendix and has
     * no State here, which is why a domain search returns nullopt for it
     * rather than naming it.
     */
    static bool isTransitionDomainCandidate(State state) {
        return StatePolicy::isCompoundState(state) && !StatePolicy::isParallelState(state);
    }

    /**
     * @brief Check if one state is a descendant of another
     *
     * A state is a descendant of `ancestor` if `ancestor` appears in its parent
     * chain; a state is not its own descendant.
     */
    static bool isDescendantOf(State descendant, State ancestor) {
        return HierarchicalAlgorithms::isDescendantOf(descendant, ancestor,
                                                      [](State s) { return StatePolicy::getParent(s); });
    }
};

}  // namespace SCE::Core
