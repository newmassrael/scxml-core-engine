// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "model/SCXMLModel.h"
#include "core/LogMacros.h"
#include "model/ITransitionNode.h"
#include <algorithm>
#include <iostream>
#include <sstream>
#include <unordered_set>

SCE::SCXMLModel::SCXMLModel() : rootState_(nullptr) {
    SCE_LOG_DEBUG("Creating SCXML model");
}

SCE::SCXMLModel::~SCXMLModel() {
    SCE_LOG_DEBUG("Destroying SCXML model");
    // Smart pointers handle resource cleanup
}

void SCE::SCXMLModel::setRootState(std::shared_ptr<SCE::IStateNode> rootState) {
    SCE_LOG_DEBUG("Setting root state: {}", (rootState ? rootState->getId() : "null"));
    rootState_ = rootState;
    // The root is the first top-level state. The parser adds it before naming
    // it; a model assembled by hand may only name it.
    if (rootState && rootState->getParent() == nullptr &&
        std::find(topLevelStates_.begin(), topLevelStates_.end(), rootState) == topLevelStates_.end()) {
        topLevelStates_.insert(topLevelStates_.begin(), rootState);
    }
    // Rebuild the complete state list to include all nested children
    rebuildAllStatesList();
}

std::shared_ptr<SCE::IStateNode> SCE::SCXMLModel::getRootState() const {
    return rootState_;
}

void SCE::SCXMLModel::setName(const std::string &name) {
    name_ = name;
}

const std::string &SCE::SCXMLModel::getName() const {
    return name_;
}

void SCE::SCXMLModel::setInitialState(const std::string &initialState) {
    SCE_LOG_DEBUG("Setting initial state: {}", initialState);

    // §scxml-3.3: Parse space-separated initial state IDs
    initialStates_.clear();
    std::istringstream iss(initialState);
    std::string stateId;

    while (iss >> stateId) {
        initialStates_.push_back(stateId);
    }

    SCE_LOG_DEBUG("Parsed {} initial state(s)", initialStates_.size());
}

const std::vector<std::string> &SCE::SCXMLModel::getInitialStates() const {
    return initialStates_;
}

std::string SCE::SCXMLModel::getInitialState() const {
    // Return first initial state for backward compatibility
    return initialStates_.empty() ? "" : initialStates_[0];
}

void SCE::SCXMLModel::setDatamodel(const std::string &datamodel) {
    SCE_LOG_DEBUG("Setting datamodel: {}", datamodel);
    datamodel_ = datamodel;
}

const std::string &SCE::SCXMLModel::getDatamodel() const {
    return datamodel_;
}

void SCE::SCXMLModel::addContextProperty(const std::string &name, const std::string &type) {
    SCE_LOG_DEBUG("Adding context property: {} ({})", name, type);
    contextProperties_[name] = type;
}

const std::unordered_map<std::string, std::string> &SCE::SCXMLModel::getContextProperties() const {
    return contextProperties_;
}

void SCE::SCXMLModel::addInjectPoint(const std::string &name, const std::string &type) {
    SCE_LOG_DEBUG("Adding inject point: {} ({})", name, type);
    injectPoints_[name] = type;
}

const std::unordered_map<std::string, std::string> &SCE::SCXMLModel::getInjectPoints() const {
    return injectPoints_;
}

void SCE::SCXMLModel::addGuard(std::shared_ptr<SCE::IGuardNode> guard) {
    if (guard) {
        SCE_LOG_DEBUG("Adding guard: {}", guard->getId());
        guards_.push_back(guard);
    }
}

const std::vector<std::shared_ptr<SCE::IGuardNode>> &SCE::SCXMLModel::getGuards() const {
    return guards_;
}

void SCE::SCXMLModel::addState(std::shared_ptr<SCE::IStateNode> state) {
    if (state) {
        SCE_LOG_DEBUG("Adding state: {}", state->getId());
        addedStates_.push_back(state);
        if (state->getParent() == nullptr) {
            topLevelStates_.push_back(state);
        }
        // Rebuild the complete state list to include all nested children
        rebuildAllStatesList();
    }
}

const std::vector<std::shared_ptr<SCE::IStateNode>> &SCE::SCXMLModel::getAllStates() const {
    return allStates_;
}

const std::vector<std::shared_ptr<SCE::IStateNode>> &SCE::SCXMLModel::getTopLevelStates() const {
    return topLevelStates_;
}

SCE::IStateNode *SCE::SCXMLModel::findStateById(const std::string &id) const {
    // Search in map first
    auto it = stateIdMap_.find(id);
    if (it != stateIdMap_.end()) {
        return it->second;
    }

    // If not in map, search all top-level states
    std::set<std::string> visitedStates;  // Track already visited state IDs
    for (const auto &state : allStates_) {
        if (state->getId() == id) {
            return state.get();
        }

        SCE::IStateNode *result = findStateByIdRecursive(state.get(), id, visitedStates);
        if (result) {
            return result;
        }
    }

    return nullptr;
}

SCE::IStateNode *SCE::SCXMLModel::findStateByIdRecursive(SCE::IStateNode *state, const std::string &id,
                                                         std::set<std::string> &visitedStates) const {
    if (!state) {
        return nullptr;
    }

    // Skip already visited states
    if (visitedStates.find(state->getId()) != visitedStates.end()) {
        return nullptr;
    }

    visitedStates.insert(state->getId());

    // Check current state
    if (state->getId() == id) {
        return state;
    }

    // Search child states
    for (const auto &child : state->getChildren()) {
        SCE::IStateNode *result = findStateByIdRecursive(child.get(), id, visitedStates);
        if (result) {
            return result;
        }
    }

    return nullptr;
}

void SCE::SCXMLModel::addDataModelItem(std::shared_ptr<SCE::IDataModelItem> dataItem) {
    if (dataItem) {
        SCE_LOG_DEBUG("Adding data model item: {}", dataItem->getId());
        dataModelItems_.push_back(dataItem);
    }
}

const std::vector<std::shared_ptr<SCE::IDataModelItem>> &SCE::SCXMLModel::getDataModelItems() const {
    return dataModelItems_;
}

bool SCE::SCXMLModel::validateStateRelationships() const {
    SCE_LOG_INFO("Validating state relationships");

    // Validate all states
    for (const auto &state : allStates_) {
        // Validate parent state
        SCE::IStateNode *parent = state->getParent();
        if (parent) {
            // Check if parent actually has this state as a child
            bool foundAsChild = false;
            for (const auto &childState : parent->getChildren()) {
                if (childState.get() == state.get()) {
                    foundAsChild = true;
                    break;
                }
            }

            if (!foundAsChild) {
                SCE_LOG_ERROR("State '{}' has parent '{}' but is not in parent's children list", state->getId(),
                              parent->getId());
                return false;
            }
        }

        // Check if target states of all transitions exist
        for (const auto &transition : state->getTransitions()) {
            const auto targets = transition->getTargets();
            for (const auto &target : targets) {
                SCE::IStateNode *targetState = findStateById(target);
                if (!targetState) {
                    SCE_LOG_ERROR("Transition in state '{}' references non-existent target state '{}'", state->getId(),
                                  target);
                    return false;
                }
            }
        }

        // Check if initial state exists
        if (!state->getInitialState().empty()) {
            if (state->getChildren().empty()) {
                SCE_LOG_WARN("State '{}' has initialState but no children", state->getId());
            } else {
                // §scxml-3.3: Validate space-separated initial state list
                std::istringstream iss(state->getInitialState());
                std::string initialStateId;

                while (iss >> initialStateId) {
                    // Search in entire model (not just direct children)
                    if (!findStateById(initialStateId)) {
                        SCE_LOG_ERROR("State '{}' references non-existent initial state '{}'", state->getId(),
                                      initialStateId);
                        return false;
                    }
                }
            }
        }
    }

    SCE_LOG_INFO("All state relationships are valid");
    return true;
}

std::vector<std::string> SCE::SCXMLModel::findMissingStateIds() const {
    SCE_LOG_INFO("Looking for missing state IDs");

    std::vector<std::string> missingIds;
    std::unordered_set<std::string> existingIds;

    // Collect all state IDs
    for (const auto &state : allStates_) {
        existingIds.insert(state->getId());
    }

    // Check referenced state IDs
    for (const auto &state : allStates_) {
        // Check initial state
        if (!state->getInitialState().empty() && existingIds.find(state->getInitialState()) == existingIds.end()) {
            missingIds.push_back(state->getInitialState());
            SCE_LOG_WARN("Missing state ID referenced as initial state: {}", state->getInitialState());
        }

        // Check transition targets
        for (const auto &transition : state->getTransitions()) {
            const auto targets = transition->getTargets();
            for (const auto &target : targets) {
                if (!target.empty() && existingIds.find(target) == existingIds.end()) {
                    missingIds.push_back(target);
                    SCE_LOG_WARN("Missing state ID referenced as transition target: {}", target);
                }
            }
        }
    }

    // Remove duplicates
    std::sort(missingIds.begin(), missingIds.end());
    missingIds.erase(std::unique(missingIds.begin(), missingIds.end()), missingIds.end());

    SCE_LOG_INFO("Found {} missing state IDs", missingIds.size());
    return missingIds;
}

std::set<std::string> SCE::SCXMLModel::getDataModelVariableNames() const {
    std::set<std::string> variableNames;

    for (const auto &dataItem : dataModelItems_) {
        if (dataItem) {
            variableNames.insert(dataItem->getId());
        }
    }

    return variableNames;
}

void SCE::SCXMLModel::printModelStructure() const {
    SCE_LOG_INFO("Printing model structure");
    SCE_LOG_INFO("SCXML Model Structure:\n");
    SCE_LOG_INFO("======================\n");
    std::string initialStateStr;
    for (size_t i = 0; i < initialStates_.size(); ++i) {
        if (i > 0) {
            initialStateStr += " ";
        }
        initialStateStr += initialStates_[i];
    }
    SCE_LOG_INFO("Initial State(s): {}", initialStateStr);
    SCE_LOG_INFO("Datamodel: {}", datamodel_);

    SCE_LOG_INFO("Context Properties:\n");
    for (const auto &[name, type] : contextProperties_) {
        SCE_LOG_INFO("  {}: {}", name, type);
    }

    SCE_LOG_INFO("\nInject Points:\n");
    for (const auto &[name, type] : injectPoints_) {
        SCE_LOG_INFO("  {}: {}", name, type);
    }

    SCE_LOG_INFO("\nGuards:\n");
    for (const auto &guard : guards_) {
        SCE_LOG_INFO("  {}:", guard->getId());

        if (!guard->getCondition().empty()) {
            SCE_LOG_INFO("    Condition: {}", guard->getCondition());
        }

        if (!guard->getTargetState().empty()) {
            SCE_LOG_INFO("    Target State: {}", guard->getTargetState());
        }

        SCE_LOG_INFO("    Dependencies:\n");
        for (const auto &dep : guard->getDependencies()) {
            SCE_LOG_INFO("      {}", dep);
        }

        if (!guard->getExternalClass().empty()) {
            SCE_LOG_INFO("    External Class: {}", guard->getExternalClass());
        }
    }

    SCE_LOG_INFO("\nState Hierarchy:\n");
    if (rootState_) {
        printStateHierarchy(rootState_.get(), 0);
    }

    SCE_LOG_INFO("Model structure printed");
}

void SCE::SCXMLModel::printStateHierarchy(SCE::IStateNode *state, int depth) const {
    if (!state) {
        return;
    }

    // Generate indentation
    std::string indent(depth * 2, ' ');

    // Print current state information
    SCE_LOG_INFO("{}State: {}", indent, state->getId());

    // Print child states recursively
    for (const auto &child : state->getChildren()) {
        printStateHierarchy(child.get(), depth + 1);
    }
}

void SCE::SCXMLModel::setBinding(const std::string &binding) {
    SCE_LOG_DEBUG("Setting binding mode: {}", binding);
    binding_ = binding;
}

const std::string &SCE::SCXMLModel::getBinding() const {
    return binding_;
}

void SCE::SCXMLModel::addSystemVariable(std::shared_ptr<SCE::IDataModelItem> systemVar) {
    if (systemVar) {
        SCE_LOG_DEBUG("Adding system variable: {}", systemVar->getId());
        systemVariables_.push_back(systemVar);
    }
}

const std::vector<std::shared_ptr<SCE::IDataModelItem>> &SCE::SCXMLModel::getSystemVariables() const {
    return systemVariables_;
}

void SCE::SCXMLModel::addTopLevelScript(std::shared_ptr<SCE::IActionNode> script) {
    if (script) {
        SCE_LOG_DEBUG("Adding top-level script (W3C SCXML 5.8)");
        topLevelScripts_.push_back(script);
    }
}

const std::vector<std::shared_ptr<SCE::IActionNode>> &SCE::SCXMLModel::getTopLevelScripts() const {
    return topLevelScripts_;
}

void SCE::SCXMLModel::rebuildAllStatesList() {
    // Document order is a pre-order walk: each top-level state, in the order
    // `<scxml>` holds them, followed by its subtree. A state added on its own
    // rather than as a top-level state — a model assembled by hand — comes
    // after, so it is still found by id. Each node is listed once: the walk
    // this replaced re-added the second and later top-level states on every
    // rebuild, so they appeared twice.
    std::vector<std::shared_ptr<IStateNode>> ordered;
    std::unordered_set<const IStateNode *> seen;
    std::vector<std::shared_ptr<IStateNode>> pending;
    const auto visit = [&](const std::shared_ptr<IStateNode> &start) {
        pending.push_back(start);
        while (!pending.empty()) {
            const auto state = pending.back();
            pending.pop_back();
            if (!state || !seen.insert(state.get()).second) {
                continue;
            }
            ordered.push_back(state);
            const auto &children = state->getChildren();
            for (auto child = children.rbegin(); child != children.rend(); ++child) {
                pending.push_back(*child);
            }
        }
    };
    if (rootState_) {
        visit(rootState_);
    }
    for (const auto &state : topLevelStates_) {
        visit(state);
    }
    for (const auto &state : addedStates_) {
        visit(state);
    }

    allStates_ = std::move(ordered);

    // Rebuild the state ID map as well
    stateIdMap_.clear();
    for (const auto &state : allStates_) {
        stateIdMap_[state->getId()] = state.get();
    }
}
