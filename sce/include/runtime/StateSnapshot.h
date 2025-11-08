// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <map>
#include <memory>
#include <optional>
#include <queue>
#include <set>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Lightweight event snapshot for serialization
 *
 * Simplified event representation for WASM/JSON serialization.
 * Contains only essential event information without complex C++ objects.
 */
struct EventSnapshot {
    std::string name;
    std::string data;  // Serialized event data

    EventSnapshot() = default;

    EventSnapshot(const std::string &n, const std::string &d = "") : name(n), data(d) {}
};

/**
 * @brief Snapshot of state machine execution state for backward stepping
 *
 * Captures complete state machine state at a specific execution step
 * to enable time-travel debugging in the interactive visualizer.
 *
 * W3C SCXML compliance: Preserves all runtime state per W3C SCXML 3.1
 */
struct StateSnapshot {
    // Active configuration (W3C SCXML 3.11)
    std::set<std::string> activeStates;

    // Data model state (W3C SCXML 5.0)
    std::map<std::string, std::string> dataModel;  // Serialized JS values

    // Event queues (W3C SCXML 3.2) - simplified for serialization
    std::vector<EventSnapshot> internalQueue;
    std::vector<EventSnapshot> externalQueue;

    // InteractiveTestRunner UI-added events (separate from engine queues)
    std::vector<EventSnapshot> pendingUIEvents;

    // Event execution history for accurate state restoration via replay
    // W3C SCXML 3.13: Store all processed events to enable time-travel debugging
    std::vector<EventSnapshot> executedEvents;

    // Execution metadata
    int stepNumber;
    std::string lastEventName;
    std::string lastTransitionSource;
    std::string lastTransitionTarget;

    StateSnapshot() : stepNumber(0) {}
};

/**
 * @brief Manages state snapshots for backward stepping capability
 *
 * Maintains a circular buffer of state snapshots with configurable
 * maximum history size to prevent unbounded memory growth.
 */
class SnapshotManager {
public:
    explicit SnapshotManager(size_t maxHistory = 1000) : maxHistory_(maxHistory) {}

    /**
     * @brief Capture current state machine state as snapshot
     *
     * @param activeStates Current active configuration
     * @param dataModel Current data model state (serialized)
     * @param internalQueue Current internal event queue
     * @param externalQueue Current external event queue
     * @param stepNumber Current execution step number
     * @param lastEvent Last processed event name
     * @param transitionSource Source state of last transition
     * @param transitionTarget Target state of last transition
     */
    void captureSnapshot(const std::set<std::string> &activeStates, const std::map<std::string, std::string> &dataModel,
                         const std::vector<EventSnapshot> &internalQueue,
                         const std::vector<EventSnapshot> &externalQueue,
                         const std::vector<EventSnapshot> &pendingUIEvents,
                         const std::vector<EventSnapshot> &executedEvents, int stepNumber,
                         const std::string &lastEvent = "", const std::string &transitionSource = "",
                         const std::string &transitionTarget = "");

    /**
     * @brief Get snapshot at specific step number
     *
     * @param stepNumber Step number to retrieve
     * @return Snapshot if available, nullopt otherwise
     */
    std::optional<StateSnapshot> getSnapshot(int stepNumber) const;

    /**
     * @brief Get most recent snapshot
     *
     * @return Latest snapshot if available, nullopt otherwise
     */
    std::optional<StateSnapshot> getLatestSnapshot() const;

    /**
     * @brief Get current number of stored snapshots
     */
    size_t size() const {
        return snapshots_.size();
    }

    /**
     * @brief Clear all snapshots
     */
    void clear() {
        snapshots_.clear();
    }

    /**
     * @brief Get maximum history size
     */
    size_t maxHistory() const {
        return maxHistory_;
    }

private:
    std::vector<StateSnapshot> snapshots_;
    size_t maxHistory_;
};

}  // namespace SCE
