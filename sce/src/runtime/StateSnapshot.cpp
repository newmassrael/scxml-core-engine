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
    snapshot.lastTransitionSource = transitionSource;
    snapshot.lastTransitionTarget = transitionTarget;

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

}  // namespace SCE
