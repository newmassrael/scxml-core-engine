// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "SnapshotComparator.h"

#include <algorithm>
#include <sstream>

namespace SCE::W3C {

std::string SnapshotDiff::format() const {
    if (isIdentical) {
        return "Snapshots are identical";
    }

    std::ostringstream oss;
    oss << "Snapshot comparison failed:\n";
    for (const auto &diff : differences) {
        oss << "  - " << diff << "\n";
    }
    return oss.str();
}

SnapshotDiff SnapshotComparator::compare(const StateSnapshot &expected, const StateSnapshot &actual,
                                         int timingToleranceMs) {
    SnapshotDiff result;

    // Compare stepNumber
    if (expected.stepNumber != actual.stepNumber) {
        result.stepNumberMismatch = true;
        result.isIdentical = false;
        std::ostringstream oss;
        oss << "stepNumber differs: expected " << expected.stepNumber << ", got " << actual.stepNumber;
        result.differences.push_back(oss.str());
    }

    // Compare lastEventName
    if (expected.lastEventName != actual.lastEventName) {
        result.lastEventNameMismatch = true;
        result.isIdentical = false;
        std::ostringstream oss;
        oss << "lastEventName differs: expected '" << expected.lastEventName << "', got '" << actual.lastEventName
            << "'";
        result.differences.push_back(oss.str());
    }

    // Compare activeStates
    if (!compareActiveStates(expected.activeStates, actual.activeStates, result.differences)) {
        result.activeStatesMismatch = true;
        result.isIdentical = false;
    }

    // Compare dataModel
    if (!compareDataModel(expected.dataModel, actual.dataModel, result.differences)) {
        result.dataModelMismatch = true;
        result.isIdentical = false;
    }

    // Compare internalQueue
    if (!compareEventQueue(expected.internalQueue, actual.internalQueue, "internalQueue", result.differences)) {
        result.internalQueueMismatch = true;
        result.isIdentical = false;
    }

    // Compare externalQueue
    if (!compareEventQueue(expected.externalQueue, actual.externalQueue, "externalQueue", result.differences)) {
        result.externalQueueMismatch = true;
        result.isIdentical = false;
    }

    // Compare pendingUIEvents
    if (!compareEventQueue(expected.pendingUIEvents, actual.pendingUIEvents, "pendingUIEvents", result.differences)) {
        result.pendingUIEventsMismatch = true;
        result.isIdentical = false;
    }

    // Compare scheduledEvents
    if (!compareScheduledEventsVector(expected.scheduledEvents, actual.scheduledEvents, timingToleranceMs,
                                      result.differences)) {
        result.scheduledEventsMismatch = true;
        result.isIdentical = false;
    }

    // Compare executedEvents
    if (!compareEventQueue(expected.executedEvents, actual.executedEvents, "executedEvents", result.differences,
                           true)) {  // Ignore timestamp for executed events
        result.executedEventsMismatch = true;
        result.isIdentical = false;
    }

    // Compare activeInvokes
    if (!compareActiveInvokesVector(expected.activeInvokes, actual.activeInvokes, timingToleranceMs,
                                    result.differences)) {
        result.activeInvokesMismatch = true;
        result.isIdentical = false;
    }

    // Compare incoming transition
    if (!compareTransition(expected.incomingTransitionSource, expected.incomingTransitionTarget,
                           expected.incomingTransitionEvent, actual.incomingTransitionSource,
                           actual.incomingTransitionTarget, actual.incomingTransitionEvent, "incoming",
                           result.differences)) {
        result.incomingTransitionMismatch = true;
        result.isIdentical = false;
    }

    // Compare outgoing transition
    if (!compareTransition(expected.outgoingTransitionSource, expected.outgoingTransitionTarget,
                           expected.outgoingTransitionEvent, actual.outgoingTransitionSource,
                           actual.outgoingTransitionTarget, actual.outgoingTransitionEvent, "outgoing",
                           result.differences)) {
        result.outgoingTransitionMismatch = true;
        result.isIdentical = false;
    }

    return result;
}

bool SnapshotComparator::compareEvents(const EventSnapshot &expected, const EventSnapshot &actual,
                                       bool ignoreTimestamp) {
    if (expected.name != actual.name) {
        return false;
    }
    if (expected.data != actual.data) {
        return false;
    }
    if (expected.sendid != actual.sendid) {
        return false;
    }
    if (expected.origintype != actual.origintype) {
        return false;
    }
    if (expected.origin != actual.origin) {
        return false;
    }
    if (expected.invokeid != actual.invokeid) {
        return false;
    }

    if (!ignoreTimestamp && expected.timestampNs != actual.timestampNs) {
        return false;
    }

    return true;
}

bool SnapshotComparator::compareScheduledEvents(const ScheduledEventSnapshot &expected,
                                                const ScheduledEventSnapshot &actual, int timingToleranceMs) {
    if (expected.eventName != actual.eventName) {
        return false;
    }
    if (expected.sendId != actual.sendId) {
        return false;
    }
    if (expected.sessionId != actual.sessionId) {
        return false;
    }
    if (expected.targetUri != actual.targetUri) {
        return false;
    }
    if (expected.eventType != actual.eventType) {
        return false;
    }
    if (expected.eventData != actual.eventData) {
        return false;
    }
    if (expected.content != actual.content) {
        return false;
    }
    if (expected.params != actual.params) {
        return false;
    }

    // Allow timing tolerance for remainingTimeMs
    int64_t timeDiff = std::abs(expected.remainingTimeMs - actual.remainingTimeMs);
    if (timeDiff > timingToleranceMs) {
        return false;
    }

    // originalDelayMs should match exactly (not timing-sensitive)
    if (expected.originalDelayMs != actual.originalDelayMs) {
        return false;
    }

    return true;
}

bool SnapshotComparator::compareInvokes(const InvokeSnapshot &expected, const InvokeSnapshot &actual,
                                        int timingToleranceMs) {
    if (expected.invokeId != actual.invokeId) {
        return false;
    }
    if (expected.parentStateId != actual.parentStateId) {
        return false;
    }
    if (expected.childSessionId != actual.childSessionId) {
        return false;
    }
    if (expected.type != actual.type) {
        return false;
    }
    if (expected.scxmlContent != actual.scxmlContent) {
        return false;
    }

    // Recursive comparison of child state
    if (expected.childState == nullptr && actual.childState == nullptr) {
        return true;
    }

    if (expected.childState == nullptr || actual.childState == nullptr) {
        return false;
    }

    // Recursive snapshot comparison
    auto childDiff = compare(*expected.childState, *actual.childState, timingToleranceMs);
    return childDiff.isIdentical;
}

bool SnapshotComparator::compareActiveStates(const std::vector<std::string> &expected,
                                             const std::vector<std::string> &actual, std::vector<std::string> &diffs) {
    if (expected == actual) {
        return true;
    }

    // W3C SCXML 3.13: Compare vectors (document order preserved for time-travel debugging)
    // Find missing states (in expected but not in actual)
    std::vector<std::string> missing;
    for (const auto &state : expected) {
        if (std::find(actual.begin(), actual.end(), state) == actual.end()) {
            missing.push_back(state);
        }
    }

    // Find extra states (in actual but not in expected)
    std::vector<std::string> extra;
    for (const auto &state : actual) {
        if (std::find(expected.begin(), expected.end(), state) == expected.end()) {
            extra.push_back(state);
        }
    }

    // Format diff message
    std::ostringstream oss;
    oss << "activeStates differ:";
    if (!missing.empty()) {
        oss << " missing=[";
        for (size_t i = 0; i < missing.size(); ++i) {
            if (i > 0) {
                oss << ", ";
            }
            oss << missing[i];
        }
        oss << "]";
    }
    if (!extra.empty()) {
        oss << " extra=[";
        for (size_t i = 0; i < extra.size(); ++i) {
            if (i > 0) {
                oss << ", ";
            }
            oss << extra[i];
        }
        oss << "]";
    }
    diffs.push_back(oss.str());

    return false;
}

bool SnapshotComparator::compareDataModel(const std::map<std::string, std::string> &expected,
                                          const std::map<std::string, std::string> &actual,
                                          std::vector<std::string> &diffs) {
    if (expected == actual) {
        return true;
    }

    bool identical = true;

    // Check for missing or differing values
    for (const auto &[key, expectedValue] : expected) {
        auto it = actual.find(key);
        if (it == actual.end()) {
            std::ostringstream oss;
            oss << "dataModel['" << key << "'] missing in actual (expected: '" << expectedValue << "')";
            diffs.push_back(oss.str());
            identical = false;
        } else if (it->second != expectedValue) {
            std::ostringstream oss;
            oss << "dataModel['" << key << "'] differs: expected '" << expectedValue << "', got '" << it->second << "'";
            diffs.push_back(oss.str());
            identical = false;
        }
    }

    // Check for extra keys
    for (const auto &[key, actualValue] : actual) {
        if (expected.find(key) == expected.end()) {
            std::ostringstream oss;
            oss << "dataModel['" << key << "'] unexpected in actual (value: '" << actualValue << "')";
            diffs.push_back(oss.str());
            identical = false;
        }
    }

    return identical;
}

bool SnapshotComparator::compareEventQueue(const std::vector<EventSnapshot> &expected,
                                           const std::vector<EventSnapshot> &actual, const std::string &queueName,
                                           std::vector<std::string> &diffs, bool ignoreTimestamp) {
    if (expected.size() != actual.size()) {
        std::ostringstream oss;
        oss << queueName << " size differs: expected " << expected.size() << ", got " << actual.size();
        diffs.push_back(oss.str());
        return false;
    }

    bool identical = true;
    for (size_t i = 0; i < expected.size(); ++i) {
        if (!compareEvents(expected[i], actual[i], ignoreTimestamp)) {
            std::ostringstream oss;
            oss << queueName << "[" << i << "] differs: expected event '" << expected[i].name << "', got '"
                << actual[i].name << "'";
            diffs.push_back(oss.str());
            identical = false;
        }
    }

    return identical;
}

bool SnapshotComparator::compareScheduledEventsVector(const std::vector<ScheduledEventSnapshot> &expected,
                                                      const std::vector<ScheduledEventSnapshot> &actual,
                                                      int timingToleranceMs, std::vector<std::string> &diffs) {
    if (expected.size() != actual.size()) {
        std::ostringstream oss;
        oss << "scheduledEvents size differs: expected " << expected.size() << ", got " << actual.size();
        diffs.push_back(oss.str());
        return false;
    }

    bool identical = true;
    for (size_t i = 0; i < expected.size(); ++i) {
        if (!compareScheduledEvents(expected[i], actual[i], timingToleranceMs)) {
            std::ostringstream oss;
            oss << "scheduledEvents[" << i << "] differs: expected event '" << expected[i].eventName << "', got '"
                << actual[i].eventName << "'";
            diffs.push_back(oss.str());
            identical = false;
        }
    }

    return identical;
}

bool SnapshotComparator::compareActiveInvokesVector(const std::vector<InvokeSnapshot> &expected,
                                                    const std::vector<InvokeSnapshot> &actual, int timingToleranceMs,
                                                    std::vector<std::string> &diffs) {
    if (expected.size() != actual.size()) {
        std::ostringstream oss;
        oss << "activeInvokes size differs: expected " << expected.size() << ", got " << actual.size();
        diffs.push_back(oss.str());
        return false;
    }

    bool identical = true;
    for (size_t i = 0; i < expected.size(); ++i) {
        if (!compareInvokes(expected[i], actual[i], timingToleranceMs)) {
            std::ostringstream oss;
            oss << "activeInvokes[" << i << "] differs: expected invokeId '" << expected[i].invokeId << "', got '"
                << actual[i].invokeId << "'";
            diffs.push_back(oss.str());
            identical = false;
        }
    }

    return identical;
}

bool SnapshotComparator::compareTransition(const std::string &expectedSource, const std::string &expectedTarget,
                                           const std::string &expectedEvent, const std::string &actualSource,
                                           const std::string &actualTarget, const std::string &actualEvent,
                                           const std::string &transitionType, std::vector<std::string> &diffs) {
    // All three fields must match
    if (expectedSource == actualSource && expectedTarget == actualTarget && expectedEvent == actualEvent) {
        return true;
    }

    // Generate detailed diff
    std::ostringstream oss;
    oss << transitionType << "Transition differs:";
    if (expectedSource != actualSource) {
        oss << " source (expected: '" << expectedSource << "', got: '" << actualSource << "')";
    }
    if (expectedTarget != actualTarget) {
        oss << " target (expected: '" << expectedTarget << "', got: '" << actualTarget << "')";
    }
    if (expectedEvent != actualEvent) {
        oss << " event (expected: '" << expectedEvent << "', got: '" << actualEvent << "')";
    }
    diffs.push_back(oss.str());

    return false;
}

}  // namespace SCE::W3C
