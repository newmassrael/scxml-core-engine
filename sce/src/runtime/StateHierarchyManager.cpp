// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateHierarchyManager.h"
#include "core/LogMacros.h"
#include "model/IStateNode.h"
#include "model/SCXMLModel.h"
#include <algorithm>

namespace SCE {

StateHierarchyManager::StateHierarchyManager(std::shared_ptr<SCXMLModel> model) : model_(std::move(model)) {
    SCE_LOG_DEBUG("StateHierarchyManager: Initialized with SCXML model");
}

std::string StateHierarchyManager::getCurrentState() const {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);

    if (activeStates_.empty()) {
        // Not started, or ended: W3C SCXML Appendix D exitInterpreter leaves an
        // ended run with no active states, so this is an ordinary answer.
        SCE_LOG_DEBUG("No active states");
        return "";
    }

    // SCXML W3C specification: parallel states define the current state context
    if (model_) {
        for (const auto &stateId : activeStates_) {
            auto stateNode = model_->findStateById(stateId);
            if (stateNode && stateNode->getType() == Type::PARALLEL) {
                return stateId;  // Return the parallel state as current state
            }
        }
    }

    // SCXML W3C specification: For compound states, return the most specific (atomic) state
    for (auto it = activeStates_.rbegin(); it != activeStates_.rend(); ++it) {
        const std::string &stateId = *it;
        if (model_) {
            auto stateNode = model_->findStateById(stateId);
            if (stateNode) {
                auto stateType = stateNode->getType();
                if (stateType == Type::ATOMIC || stateType == Type::FINAL) {
                    return stateId;
                }
            }
        }
    }

    // Fallback: return the last (most specific) state in the active configuration
    return activeStates_.back();
}

std::vector<std::string> StateHierarchyManager::getActiveStates() const {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);
    return activeStates_;
}

bool StateHierarchyManager::isStateActive(const std::string &stateId) const {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);
    return activeSet_.find(stateId) != activeSet_.end();
}

void StateHierarchyManager::reset() {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);

    SCE_LOG_DEBUG("Clearing all active states");
    activeStates_.clear();
    activeSet_.clear();
}

void StateHierarchyManager::addStateToConfiguration(const std::string &stateId) {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);

    if (stateId.empty() || activeSet_.find(stateId) != activeSet_.end()) {
        return;  // Already active or empty ID
    }

    activeStates_.push_back(stateId);
    activeSet_.insert(stateId);
}

void StateHierarchyManager::removeStateFromConfiguration(const std::string &stateId) {
    // TSAN FIX: Protect configuration access with mutex
    std::lock_guard<std::mutex> lock(configurationMutex_);

    if (stateId.empty()) {
        return;
    }

    // Remove from vector
    auto it = std::find(activeStates_.begin(), activeStates_.end(), stateId);
    if (it != activeStates_.end()) {
        activeStates_.erase(it);
    }

    // Remove from set
    activeSet_.erase(stateId);

    SCE_LOG_DEBUG("removeStateFromConfiguration - Removed: {}", stateId);
}

}  // namespace SCE
