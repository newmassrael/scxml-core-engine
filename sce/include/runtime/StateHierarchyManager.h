// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <memory>
#include <mutex>
#include <string>
#include <unordered_set>
#include <vector>

namespace SCE {

class SCXMLModel;

/**
 * @brief The configuration: the states a machine is in, in the order it
 *        entered them
 *
 * W3C SCXML Appendix D keeps one piece of run state about states,
 * `configuration` — the set every microstep exits from and enters into.
 * `SCE::Core::MicrostepAlgorithms` decides what leaves and what arrives; this
 * holds the result, and answers a host asking where the machine is. It enters
 * nothing of its own accord and runs no executable content.
 *
 * Thread-safe: a script engine worker reads the configuration for `In()` while
 * the machine's own thread changes it.
 */
class StateHierarchyManager {
public:
    /**
     * @param model The document the state ids belong to, read to answer
     *        `getCurrentState()`
     */
    explicit StateHierarchyManager(std::shared_ptr<SCXMLModel> model);

    ~StateHierarchyManager() = default;

    /**
     * @brief The state `StateMachine::getCurrentState()` names
     *
     * An active `<parallel>` if there is one, else the most recently entered
     * atomic or final state.
     *
     * @return The state id, or empty when the configuration is empty
     */
    std::string getCurrentState() const;

    /**
     * @brief Every active state, in the order it was entered
     */
    std::vector<std::string> getActiveStates() const;

    /**
     * @brief Whether @p stateId is in the configuration
     */
    bool isStateActive(const std::string &stateId) const;

    /**
     * @brief Empty the configuration
     */
    void reset();

    /**
     * @brief Take one state out of the configuration
     *
     * @param stateId The state to remove; an id that is not active is ignored
     */
    void removeStateFromConfiguration(const std::string &stateId);

    /**
     * @brief Put one state into the configuration, after every state already
     *        there
     *
     * @param stateId The state to add; an empty or already active id is ignored
     */
    void addStateToConfiguration(const std::string &stateId);

private:
    std::shared_ptr<SCXMLModel> model_;
    std::vector<std::string> activeStates_;      // Entry order
    std::unordered_set<std::string> activeSet_;  // The same states, for lookup

    // TSAN FIX: Mutex to protect activeStates_ and activeSet_ from concurrent access
    mutable std::mutex configurationMutex_;
};

}  // namespace SCE
