// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "model/IStateNode.h"
#include "model/SCXMLModel.h"
#include <memory>
#include <string>
#include <vector>

namespace SCE {

class StateHierarchyManager;

/**
 * @brief W3C SCXML 3.12/3.13: State tree geometry calculations
 *
 * Single Source of Truth for all transition domain computations:
 * LCA calculation, exit set computation, document position,
 * descendant checking, and ancestor traversal.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Delegates to HierarchicalAlgorithms for LCA
 * - Single Responsibility: Pure computation over the model tree
 * - No side effects: All methods are const
 */
class TransitionDomainCalculator {
public:
    /**
     * @brief W3C SCXML 3.13: Exit set computation result
     *
     * Returns both the exit set and the LCA to avoid duplicate computation.
     */
    struct ExitSetResult {
        std::vector<std::string> states;  // States to exit (in order)
        std::string lca;                  // Least Common Compound Ancestor

        ExitSetResult() = default;

        ExitSetResult(const std::vector<std::string> &s, const std::string &l) : states(s), lca(l) {}
    };

    /**
     * @brief Construct with required dependencies
     *
     * @param model SCXML model for state tree lookups
     * @param hierarchyManager Non-owning pointer for active state queries
     */
    TransitionDomainCalculator(std::shared_ptr<SCXMLModel> model, StateHierarchyManager *hierarchyManager);

    /**
     * @brief W3C SCXML 3.12: Find Least Common Ancestor of two states
     *
     * Delegates to HierarchicalAlgorithms (Zero Duplication).
     *
     * @param sourceStateId Source state ID
     * @param targetStateId Target state ID
     * @return LCA state ID, or empty string if no common ancestor
     */
    std::string findLCA(const std::string &sourceStateId, const std::string &targetStateId) const;

    /**
     * @brief W3C SCXML 3.13: Compute exit set for a transition
     *
     * @param sourceStateId Source state of transition
     * @param targetStateId Target state of transition
     * @return ExitSetResult with states to exit and computed LCA
     */
    ExitSetResult computeExitSet(const std::string &sourceStateId, const std::string &targetStateId) const;

    /**
     * @brief Build exit set for active descendants of an ancestor state
     *
     * @param ancestorState Ancestor state whose descendants to collect
     * @param excludeParallelChildren If true, skip children of parallel states
     * @return Descendant states sorted deepest-first
     */
    std::vector<std::string> buildExitSetForDescendants(const std::string &ancestorState,
                                                        bool excludeParallelChildren = true) const;

    /**
     * @brief W3C SCXML 3.13: Get document order position for a state
     *
     * Uses depth-first pre-order traversal to assign positions.
     *
     * @param stateId State ID to look up
     * @return Document order position (0-based), or -1 if not found
     */
    int getStateDocumentPosition(const std::string &stateId) const;

    /**
     * @brief W3C SCXML 3.12: Get all proper ancestors of a state
     *
     * @param stateId State to get ancestors for
     * @return Ancestors from immediate parent to root
     */
    std::vector<std::string> getProperAncestors(const std::string &stateId) const;

    /**
     * @brief W3C SCXML 3.12: Check if a state is a descendant of another
     *
     * @param stateId Potential descendant state
     * @param ancestorId Potential ancestor state
     * @return true if stateId is a proper descendant of ancestorId
     */
    bool isDescendant(const std::string &stateId, const std::string &ancestorId) const;

private:
    std::shared_ptr<SCXMLModel> model_;
    StateHierarchyManager *hierarchyManager_;  // Non-owning
};

}  // namespace SCE
