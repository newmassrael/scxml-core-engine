// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <string>
#include <vector>

#include "runtime/StateSnapshot.h"

namespace SCE::W3C {

/**
 * @brief Detailed difference report between two snapshots
 *
 * Used for comprehensive time-travel debugging verification.
 * Provides field-by-field comparison with human-readable diff messages.
 */
struct SnapshotDiff {
    bool isIdentical;
    std::vector<std::string> differences;

    // Field-specific mismatch flags
    bool activeStatesMismatch;
    bool dataModelMismatch;
    bool internalQueueMismatch;
    bool externalQueueMismatch;
    bool pendingUIEventsMismatch;
    bool scheduledEventsMismatch;
    bool executedEventsMismatch;
    bool activeInvokesMismatch;
    bool stepNumberMismatch;
    bool lastEventNameMismatch;
    bool incomingTransitionMismatch;
    bool outgoingTransitionMismatch;

    SnapshotDiff()
        : isIdentical(true), activeStatesMismatch(false), dataModelMismatch(false), internalQueueMismatch(false),
          externalQueueMismatch(false), pendingUIEventsMismatch(false), scheduledEventsMismatch(false),
          executedEventsMismatch(false), activeInvokesMismatch(false), stepNumberMismatch(false),
          lastEventNameMismatch(false), incomingTransitionMismatch(false), outgoingTransitionMismatch(false) {}

    /**
     * @brief Get formatted diff report for logging
     */
    std::string format() const;
};

/**
 * @brief W3C SCXML 3.13 snapshot comparison utility
 *
 * Provides field-by-field comparison of StateSnapshot objects for
 * time-travel debugging verification.
 *
 * Architecture Compliance:
 * - Zero Duplication: Single implementation for all snapshot comparisons
 * - W3C SCXML 3.13: Complete state comparison (active states, datamodel, queues, invokes)
 */
class SnapshotComparator {
public:
    /**
     * @brief Compare two snapshots field-by-field
     *
     * @param expected Expected snapshot state
     * @param actual Actual snapshot state
     * @param timingToleranceMs Tolerance for scheduledEvents.remainingTimeMs (default 10ms)
     * @return SnapshotDiff with detailed comparison results
     */
    static SnapshotDiff compare(const StateSnapshot &expected, const StateSnapshot &actual, int timingToleranceMs = 10);

    /**
     * @brief Compare two event snapshots
     *
     * W3C SCXML 5.10.1: Compares all event metadata fields
     *
     * @param expected Expected event
     * @param actual Actual event
     * @param ignoreTimestamp If true, ignore timestampNs field (default false)
     * @return true if events are identical
     */
    static bool compareEvents(const EventSnapshot &expected, const EventSnapshot &actual, bool ignoreTimestamp = false);

    /**
     * @brief Compare two scheduled event snapshots
     *
     * W3C SCXML 6.2: Compares all scheduled event fields with timing tolerance
     *
     * @param expected Expected scheduled event
     * @param actual Actual scheduled event
     * @param timingToleranceMs Tolerance for remainingTimeMs (default 10ms)
     * @return true if scheduled events are identical within tolerance
     */
    static bool compareScheduledEvents(const ScheduledEventSnapshot &expected, const ScheduledEventSnapshot &actual,
                                       int timingToleranceMs = 10);

    /**
     * @brief Compare two invoke snapshots recursively
     *
     * W3C SCXML 3.11: Compares invoke state including recursive child state
     *
     * @param expected Expected invoke snapshot
     * @param actual Actual invoke snapshot
     * @param timingToleranceMs Timing tolerance for child state scheduledEvents
     * @return true if invokes are identical
     */
    static bool compareInvokes(const InvokeSnapshot &expected, const InvokeSnapshot &actual,
                               int timingToleranceMs = 10);

private:
    /**
     * @brief Compare active states vectors (W3C SCXML 3.13: document order preserved)
     */
    static bool compareActiveStates(const std::vector<std::string> &expected, const std::vector<std::string> &actual,
                                    std::vector<std::string> &diffs);

    /**
     * @brief Compare data model maps
     */
    static bool compareDataModel(const std::map<std::string, std::string> &expected,
                                 const std::map<std::string, std::string> &actual, std::vector<std::string> &diffs);

    /**
     * @brief Compare event queue vectors
     */
    static bool compareEventQueue(const std::vector<EventSnapshot> &expected, const std::vector<EventSnapshot> &actual,
                                  const std::string &queueName, std::vector<std::string> &diffs,
                                  bool ignoreTimestamp = false);

    /**
     * @brief Compare scheduled events vectors
     */
    static bool compareScheduledEventsVector(const std::vector<ScheduledEventSnapshot> &expected,
                                             const std::vector<ScheduledEventSnapshot> &actual, int timingToleranceMs,
                                             std::vector<std::string> &diffs);

    /**
     * @brief Compare active invokes vectors
     */
    static bool compareActiveInvokesVector(const std::vector<InvokeSnapshot> &expected,
                                           const std::vector<InvokeSnapshot> &actual, int timingToleranceMs,
                                           std::vector<std::string> &diffs);

    /**
     * @brief Compare transition information
     */
    static bool compareTransition(const std::string &expectedSource, const std::string &expectedTarget,
                                  const std::string &expectedEvent, const std::string &actualSource,
                                  const std::string &actualTarget, const std::string &actualEvent,
                                  const std::string &transitionType, std::vector<std::string> &diffs);
};

}  // namespace SCE::W3C
