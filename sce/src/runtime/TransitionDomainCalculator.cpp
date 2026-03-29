#include "runtime/TransitionDomainCalculator.h"

#include "core/HierarchicalStateHelper.h"
#include "core/LogMacros.h"
#include "runtime/StateHierarchyManager.h"
#include <algorithm>
#include <functional>

namespace SCE {

TransitionDomainCalculator::TransitionDomainCalculator(std::shared_ptr<SCXMLModel> model,
                                                       StateHierarchyManager *hierarchyManager)
    : model_(std::move(model)), hierarchyManager_(hierarchyManager) {}

std::vector<std::string> TransitionDomainCalculator::getProperAncestors(const std::string &stateId) const {
    std::vector<std::string> ancestors;

    if (!model_) {
        return ancestors;
    }

    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        return ancestors;
    }

    IStateNode *current = stateNode->getParent();
    while (current != nullptr) {
        ancestors.push_back(current->getId());
        current = current->getParent();
    }

    return ancestors;
}

bool TransitionDomainCalculator::isDescendant(const std::string &stateId, const std::string &ancestorId) const {
    if (!model_ || stateId.empty() || ancestorId.empty()) {
        return false;
    }

    if (stateId == ancestorId) {
        return false;  // A state is not its own descendant
    }

    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        return false;
    }

    IStateNode *current = stateNode->getParent();
    while (current != nullptr) {
        if (current->getId() == ancestorId) {
            return true;
        }
        current = current->getParent();
    }

    return false;
}

int TransitionDomainCalculator::getStateDocumentPosition(const std::string &stateId) const {
    // W3C SCXML 3.13: Get document order position for state
    // Uses depth-first pre-order traversal to assign positions
    if (!model_) {
        return -1;
    }

    // Helper to recursively assign positions
    int position = 0;
    std::function<int(IStateNode *, const std::string &)> findPosition = [&](IStateNode *node,
                                                                             const std::string &targetId) -> int {
        if (!node) {
            return -1;
        }

        if (node->getId() == targetId) {
            return position;
        }

        position++;

        // Depth-first pre-order: visit children
        const auto &children = node->getChildren();
        for (const auto &child : children) {
            int result = findPosition(child.get(), targetId);
            if (result >= 0) {
                return result;
            }
        }

        return -1;
    };

    // Start from root state
    auto rootState = model_->getRootState();
    if (!rootState) {
        return -1;
    }

    return findPosition(rootState.get(), stateId);
}

std::string TransitionDomainCalculator::findLCA(const std::string &sourceStateId,
                                                 const std::string &targetStateId) const {
    if (!model_) {
        return "";
    }

    // ARCHITECTURE.md: Zero Duplication - delegate to HierarchicalStateHelper
    // W3C SCXML 3.12: Find Least Common Ancestor for hierarchical transitions
    auto getParent = [this](const std::string &stateId) -> std::optional<std::string> {
        auto node = model_->findStateById(stateId);
        if (!node || !node->getParent()) {
            return std::nullopt;
        }
        return node->getParent()->getId();
    };

    // Use shared unified algorithm (Single Source of Truth)
    return SCE::Core::HierarchicalAlgorithms::findLCA(sourceStateId, targetStateId, getParent).value_or("");
}

std::vector<std::string> TransitionDomainCalculator::buildExitSetForDescendants(const std::string &ancestorState,
                                                                                bool excludeParallelChildren) const {
    std::vector<std::string> exitSet;

    if (!hierarchyManager_ || !model_) {
        return exitSet;
    }

    // Get all active states
    auto activeStates = hierarchyManager_->getActiveStates();

    for (const auto &activeState : activeStates) {
        // Skip if this is the ancestor itself
        if (activeState == ancestorState) {
            continue;
        }

        // Defensive: Get state node and skip if not found
        auto activeNode = model_->findStateById(activeState);
        if (!activeNode) {
            SCE_LOG_WARN("buildExitSetForDescendants: Active state '{}' not found in model - skipping", activeState);
            continue;
        }

        // Check if parent is a parallel state - skip if requested
        if (excludeParallelChildren) {
            auto parent = activeNode->getParent();
            if (parent && parent->getType() == Type::PARALLEL) {
                // Skip - parallel state's children are handled by exitParallelState
                continue;
            }
        }

        // Check if activeState is a descendant of ancestorState
        if (ancestorState.empty()) {
            // If ancestor is root (empty), all active states are descendants
            exitSet.push_back(activeState);
        } else {
            // Walk up the ancestor chain to check if we reach ancestorState
            IStateNode *current = activeNode->getParent();
            while (current) {
                if (current->getId() == ancestorState) {
                    // Found ancestor - activeState is a descendant
                    exitSet.push_back(activeState);
                    break;
                }
                current = current->getParent();
            }
        }
    }

    // Sort by depth (deepest first) for correct exit order
    std::sort(exitSet.begin(), exitSet.end(), [this](const std::string &a, const std::string &b) {
        int depthA = 0, depthB = 0;
        auto nodeA = model_->findStateById(a);
        auto nodeB = model_->findStateById(b);

        if (nodeA) {
            IStateNode *current = nodeA->getParent();
            while (current) {
                depthA++;
                current = current->getParent();
            }
        }
        if (nodeB) {
            IStateNode *current = nodeB->getParent();
            while (current) {
                depthB++;
                current = current->getParent();
            }
        }

        return depthA > depthB;  // Deeper states first
    });

    return exitSet;
}

TransitionDomainCalculator::ExitSetResult
TransitionDomainCalculator::computeExitSet(const std::string &sourceStateId, const std::string &targetStateId) const {
    ExitSetResult result;
    result.states.reserve(8);  // Performance: Reserve typical exit set size to avoid reallocation

    if (!model_ || sourceStateId.empty()) {
        return result;
    }

    // If target is empty (targetless transition), exit source only
    if (targetStateId.empty()) {
        result.states.push_back(sourceStateId);
        return result;
    }

    // W3C SCXML 3.13: Find LCA (Lowest Common Ancestor) once
    result.lca = findLCA(sourceStateId, targetStateId);

    // W3C SCXML 3.13: Exit set = "all active states that are proper descendants of LCCA"
    // This must include ALL active descendants, not just the source->LCA chain (test 505)
    // Use helper method to build exit set (reduces code duplication)
    result.states = buildExitSetForDescendants(result.lca, true);

    // W3C SCXML 3.10 (test 579): Ancestor transition (target == LCA)
    // When transitioning to an ancestor state, the target must also be exited and re-entered
    // This ensures onexit/onentry are executed, allowing data changes (e.g., Var1++)
    if (targetStateId == result.lca && hierarchyManager_ && hierarchyManager_->isStateActive(targetStateId)) {
        result.states.push_back(targetStateId);
        SCE_LOG_DEBUG("W3C SCXML: Ancestor transition detected, including target '{}' in exit set", targetStateId);
    }

    // W3C SCXML 3.10 (test 580): History state transition
    // When transitioning to a history state whose parent is active, exit and re-enter the parent
    // This ensures onexit/onentry actions execute (e.g., Var1++ in onexit)
    auto targetNode = model_->findStateById(targetStateId);
    if (targetNode && targetNode->getType() == Type::HISTORY) {
        auto parentNode = targetNode->getParent();
        if (parentNode && hierarchyManager_ && hierarchyManager_->isStateActive(parentNode->getId())) {
            std::string parentId = parentNode->getId();
            // Check if parent is not already in exit set
            if (std::find(result.states.begin(), result.states.end(), parentId) == result.states.end()) {
                result.states.push_back(parentId);
                SCE_LOG_DEBUG(
                    "W3C SCXML 3.10: History state transition, including active parent '{}' in exit set (test 580)",
                    parentId);
            }
        }
    }

    // Note: buildExitSetForDescendants already:
    // - Excludes parallel children (test 404, 504)
    // - Sorts by depth (deepest first)
    // - Handles null checks defensively

    SCE_LOG_DEBUG("W3C SCXML: computeExitSet({} -> {}) = {} states, LCA = '{}'", sourceStateId, targetStateId,
              result.states.size(), result.lca);

    return result;
}

}  // namespace SCE
