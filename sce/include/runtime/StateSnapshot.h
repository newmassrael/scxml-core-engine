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

    // W3C SCXML 5.10.1: Event metadata for _event object restoration
    std::string sendid;      // W3C SCXML 5.10.1: _event.sendid
    std::string origintype;  // W3C SCXML 5.10.1: _event.origintype
    std::string origin;      // W3C SCXML 5.10.1: _event.origin (session ID)
    std::string invokeid;    // W3C SCXML 5.10.1: _event.invokeid

    // W3C SCXML 3.13: Timestamp for FIFO ordering preservation during snapshot restore
    // Stores nanoseconds since epoch for precise queue order restoration
    int64_t timestampNs;

    EventSnapshot() : timestampNs(0) {}

    EventSnapshot(const std::string &n, const std::string &d = "")
        : name(n), data(d), sendid(""), origintype(""), origin(""), invokeid(""), timestampNs(0) {}

    EventSnapshot(const std::string &n, const std::string &d, const std::string &sid, const std::string &otype,
                  const std::string &orig, const std::string &invId, int64_t ts = 0)
        : name(n), data(d), sendid(sid), origintype(otype), origin(orig), invokeid(invId), timestampNs(ts) {}
};

/**
 * @brief Lightweight scheduled event snapshot for serialization
 *
 * Captures scheduled event state for step backward restoration.
 * Contains event metadata without complex C++ objects (IEventTarget).
 *
 * W3C SCXML 6.2: Stores complete send element information for accurate restoration.
 */
struct ScheduledEventSnapshot {
    std::string eventName;
    std::string sendId;
    int64_t originalDelayMs;  // Original delay in milliseconds (W3C SCXML 6.2.4)
    int64_t remainingTimeMs;  // Remaining time at snapshot capture (for accurate restoration)
    std::string sessionId;

    // W3C SCXML 6.2: Complete EventDescriptor fields for restoration
    std::string targetUri;                      // Target URI (empty = external queue, "#_internal" = internal)
    std::string eventType;                      // Event type (scxml, platform, etc.)
    std::string eventData;                      // Event data payload
    std::string content;                        // HTTP body content (W3C SCXML C.2)
    std::map<std::string, std::string> params;  // W3C SCXML 6.2: param name-value pairs for _event.data restoration

    ScheduledEventSnapshot() = default;

    ScheduledEventSnapshot(const std::string &name, const std::string &id, int64_t delayMs, int64_t remainingMs,
                           const std::string &sessId, const std::string &target = "", const std::string &type = "scxml",
                           const std::string &data = "", const std::string &cnt = "",
                           const std::map<std::string, std::string> &prms = {})
        : eventName(name), sendId(id), originalDelayMs(delayMs), remainingTimeMs(remainingMs), sessionId(sessId),
          targetUri(target), eventType(type), eventData(data), content(cnt), params(prms) {}
};

// Forward declaration for recursive child state snapshot
struct StateSnapshot;

/**
 * @brief Snapshot of active invoke state for time-travel debugging
 *
 * W3C SCXML 3.11: Invocations are part of configuration
 * Zero Duplication: Captures invoke state without duplicating InvokeExecutor logic
 *
 * Contains complete invoke state including child state machine configuration
 * to enable accurate restoration during step backward/reset operations.
 */
struct InvokeSnapshot {
    std::string invokeId;        // W3C SCXML invoke ID (e.g., "s0.invoke_2")
    std::string parentStateId;   // Parent state containing this invoke (e.g., "s0")
    std::string childSessionId;  // Child state machine session ID
    std::string type;            // Invoke type (e.g., "http://www.w3.org/TR/scxml")
    std::string scxmlContent;    // Child SCXML content (from src/srcexpr evaluation)
    bool autoForward = false;    // W3C SCXML 6.4: Autoforward flag for event forwarding to child

    // W3C SCXML 3.11: Recursive child state machine configuration
    // Captures complete child state (active states, datamodel, queues, etc.)
    std::shared_ptr<StateSnapshot> childState;

    InvokeSnapshot() = default;

    InvokeSnapshot(const std::string &invId, const std::string &parentId, const std::string &childSessId,
                   const std::string &invType, const std::string &content = "", bool autoFwd = false)
        : invokeId(invId), parentStateId(parentId), childSessionId(childSessId), type(invType), scxmlContent(content),
          autoForward(autoFwd), childState(nullptr) {}
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
    // W3C SCXML 3.13: Use vector to preserve document order for time-travel debugging (Test 570)
    std::vector<std::string> activeStates;

    // Data model state (W3C SCXML 5.0)
    std::map<std::string, std::string> dataModel;  // Serialized JS values

    // Event queues (W3C SCXML 3.2) - simplified for serialization
    std::vector<EventSnapshot> internalQueue;
    std::vector<EventSnapshot> externalQueue;

    // InteractiveTestRunner UI-added events (separate from engine queues)
    std::vector<EventSnapshot> pendingUIEvents;

    // Scheduled events (W3C SCXML 6.2) - delayed send operations
    // Stores complete event info for recreation on step backward
    std::vector<ScheduledEventSnapshot> scheduledEvents;

    // Event execution history for accurate state restoration via replay
    // W3C SCXML 3.13: Store all processed events to enable time-travel debugging
    std::vector<EventSnapshot> executedEvents;

    // W3C SCXML 3.11: Active invocations (part of configuration)
    // Zero Duplication: Enables complete state restoration without side effects
    std::vector<InvokeSnapshot> activeInvokes;

    // Execution metadata
    int stepNumber;
    std::string lastEventName;

    // W3C SCXML 3.13: Dual transition tracking for time-travel debugging
    // Incoming transition: How we arrived at this state (previous step's transition)
    std::string incomingTransitionSource;
    std::string incomingTransitionTarget;
    std::string incomingTransitionEvent;

    // Outgoing transition: Next transition from this state (current step's transition)
    // Enables step backward to display "cancelled transition"
    std::string outgoingTransitionSource;
    std::string outgoingTransitionTarget;
    std::string outgoingTransitionEvent;

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
     * @param activeStates Current active configuration (document order preserved)
     * @param dataModel Current data model state (serialized)
     * @param internalQueue Current internal event queue
     * @param externalQueue Current external event queue
     * @param pendingUIEvents UI-added events (separate from engine queues)
     * @param scheduledEvents Scheduled events for step backward restoration (W3C SCXML 6.2)
     * @param activeInvokes Active invocations (W3C SCXML 3.11)
     * @param executedEvents Event execution history
     * @param stepNumber Current execution step number
     * @param lastEvent Last processed event name
     * @param transitionSource Source state of last transition
     * @param transitionTarget Target state of last transition
     */
    void
    captureSnapshot(const std::vector<std::string> &activeStates, const std::map<std::string, std::string> &dataModel,
                    const std::vector<EventSnapshot> &internalQueue, const std::vector<EventSnapshot> &externalQueue,
                    const std::vector<EventSnapshot> &pendingUIEvents,
                    const std::vector<ScheduledEventSnapshot> &scheduledEvents,
                    const std::vector<InvokeSnapshot> &activeInvokes, const std::vector<EventSnapshot> &executedEvents,
                    int stepNumber, const std::string &lastEvent = "", const std::string &transitionSource = "",
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

    /**
     * @brief Remove all snapshots after specified step number
     *
     * Used for history branching when user modifies execution path
     * (e.g., removing events from queue, adding new events).
     * All snapshots with stepNumber > specified step are removed.
     *
     * @param stepNumber Step number after which to remove snapshots
     */
    void removeSnapshotsAfter(int stepNumber);

    /**
     * @brief Update outgoing transition for a specific snapshot
     *
     * W3C SCXML 3.13: After executing a transition, update the previous snapshot's
     * outgoing transition to enable accurate step backward visualization.
     *
     * This allows UI to display "cancelled transition" when stepping backward.
     *
     * @param stepNumber Step number of snapshot to update
     * @param source Source state of transition
     * @param target Target state of transition
     * @param event Event that triggered transition
     * @return true if snapshot was found and updated, false otherwise
     */
    bool updateSnapshotOutgoing(int stepNumber, const std::string &source, const std::string &target,
                                const std::string &event);

    /**
     * @brief Check if snapshot exists for specified step number
     *
     * @param stepNumber Step number to check
     * @return true if snapshot exists, false otherwise
     */
    bool hasSnapshot(int stepNumber) const;

private:
    std::vector<StateSnapshot> snapshots_;
    size_t maxHistory_;
};

}  // namespace SCE
