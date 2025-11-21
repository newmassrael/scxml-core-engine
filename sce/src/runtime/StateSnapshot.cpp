// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateSnapshot.h"
#include <algorithm>

namespace SCE {

void SnapshotManager::captureSnapshot(
    const std::set<std::string> &activeStates, const std::map<std::string, std::string> &dataModel,
    const std::vector<EventSnapshot> &internalQueue, const std::vector<EventSnapshot> &externalQueue,
    const std::vector<EventSnapshot> &pendingUIEvents, const std::vector<ScheduledEventSnapshot> &scheduledEvents,
    const std::vector<InvokeSnapshot> &activeInvokes, const std::vector<EventSnapshot> &executedEvents, int stepNumber,
    const std::string &lastEvent, const std::string &transitionSource, const std::string &transitionTarget) {
    StateSnapshot snapshot;
    snapshot.activeStates = activeStates;
    snapshot.dataModel = dataModel;
    snapshot.internalQueue = internalQueue;
    snapshot.externalQueue = externalQueue;
    snapshot.pendingUIEvents = pendingUIEvents;
    snapshot.scheduledEvents = scheduledEvents;
    snapshot.activeInvokes = activeInvokes;
    snapshot.executedEvents = executedEvents;
    snapshot.stepNumber = stepNumber;
    snapshot.lastEventName = lastEvent;

    // W3C SCXML 3.13: Set incoming transition (how we arrived at this state)
    snapshot.incomingTransitionSource = transitionSource;
    snapshot.incomingTransitionTarget = transitionTarget;
    snapshot.incomingTransitionEvent = lastEvent;

    // Outgoing transition will be set later by updateSnapshotOutgoing()

    // Check if snapshot with same stepNumber already exists
    auto it = std::find_if(snapshots_.begin(), snapshots_.end(),
                           [stepNumber](const StateSnapshot &s) { return s.stepNumber == stepNumber; });

    if (it != snapshots_.end()) {
        // Update existing snapshot (for same-step queue changes via raiseEvent)
        *it = std::move(snapshot);
    } else {
        // Add new snapshot
        snapshots_.push_back(std::move(snapshot));

        // Enforce maximum history limit (FIFO)
        if (snapshots_.size() > maxHistory_) {
            snapshots_.erase(snapshots_.begin());
        }
    }
}

std::optional<StateSnapshot> SnapshotManager::getSnapshot(int stepNumber) const {
    for (const auto &snapshot : snapshots_) {
        if (snapshot.stepNumber == stepNumber) {
            return snapshot;
        }
    }
    return std::nullopt;
}

std::optional<StateSnapshot> SnapshotManager::getLatestSnapshot() const {
    if (snapshots_.empty()) {
        return std::nullopt;
    }
    return snapshots_.back();
}

void SnapshotManager::removeSnapshotsAfter(int stepNumber) {
    // Remove all snapshots with stepNumber > specified step
    // This implements history branching for interactive debugging
    snapshots_.erase(std::remove_if(snapshots_.begin(), snapshots_.end(),
                                    [stepNumber](const StateSnapshot &s) { return s.stepNumber > stepNumber; }),
                     snapshots_.end());
}

bool SnapshotManager::hasSnapshot(int stepNumber) const {
    return std::any_of(snapshots_.begin(), snapshots_.end(),
                       [stepNumber](const StateSnapshot &s) { return s.stepNumber == stepNumber; });
}

bool SnapshotManager::updateSnapshotOutgoing(int stepNumber, const std::string &source, const std::string &target,
                                             const std::string &event) {
    // W3C SCXML 3.13: Update outgoing transition for step backward visualization
    // This enables UI to show "cancelled transition" when stepping back
    for (auto &snapshot : snapshots_) {
        if (snapshot.stepNumber == stepNumber) {
            snapshot.outgoingTransitionSource = source;
            snapshot.outgoingTransitionTarget = target;
            snapshot.outgoingTransitionEvent = event;
            return true;
        }
    }
    return false;
}

}  // namespace SCE
