#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Transition descriptor for Interpreter engine parallel state conflict resolution
 *
 * @details
 * Represents an enabled transition discovered during event processing in a concurrent region.
 * Used to collect all enabled transitions before applying W3C SCXML Appendix D.2 conflict resolution.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Compatible with ConflictResolutionHelperString::TransitionDescriptor
 * - W3C SCXML Appendix D.2: Optimal transition set selection
 */
struct TransitionDescriptorString {
    std::string source;                // Source state ID
    std::string target;                // Target state ID (empty for targetless transitions)
    std::string event;                 // Event name that triggered this transition
    std::vector<std::string> exitSet;  // States to be exited (computed by region)
    int transitionIndex = 0;           // Document order index for preemption
    bool hasActions = false;           // W3C SCXML 3.13: Whether transition has action nodes
    bool isInternal = false;           // W3C SCXML 3.13: Whether transition is type="internal"
    bool isExternal = false;           // W3C SCXML 3.13: Whether transition exits parallel state

    TransitionDescriptorString() = default;

    TransitionDescriptorString(std::string src, std::string tgt, std::string evt, int idx = 0, bool actions = false,
                               bool internal = false, bool external = false)
        : source(std::move(src)), target(std::move(tgt)), event(std::move(evt)), transitionIndex(idx),
          hasActions(actions), isInternal(internal), isExternal(external) {}
};

// Forward declarations
class IConcurrentRegion;
class IStateNode;
class Event;

/**
 * @brief Result of concurrent region operations
 */
struct ConcurrentOperationResult {
    bool isSuccess = false;
    std::string errorMessage;
    std::string regionId;

    // W3C SCXML 3.4: External transition discovered by region
    // When a region finds a transition to a state outside the region,
    // it returns this information so the parent StateMachine can handle it
    std::string externalTransitionTarget;  // Empty if no external transition
    std::string externalTransitionEvent;   // Event that triggered the external transition
    std::string externalTransitionSource;  // Source state ID (safer than raw pointer)

    // W3C SCXML Appendix D.2: Enabled transitions for conflict resolution
    // Region collects all enabled transitions and returns them to StateMachine
    // StateMachine applies ConflictResolutionHelperString to select optimal transition set
    std::vector<TransitionDescriptorString> enabledTransitions;

    static ConcurrentOperationResult success(const std::string &regionId) {
        ConcurrentOperationResult result;
        result.isSuccess = true;
        result.regionId = regionId;
        return result;
    }

    static ConcurrentOperationResult failure(const std::string &regionId, const std::string &error) {
        ConcurrentOperationResult result;
        result.isSuccess = false;
        result.regionId = regionId;
        result.errorMessage = error;
        return result;
    }

    static ConcurrentOperationResult externalTransition(const std::string &regionId, const std::string &target,
                                                        const std::string &event, const std::string &sourceStateId) {
        ConcurrentOperationResult result;
        result.isSuccess = false;  // Not success because region couldn't handle it
        result.regionId = regionId;
        result.externalTransitionTarget = target;
        result.externalTransitionEvent = event;
        result.externalTransitionSource = sourceStateId;
        result.errorMessage = "External transition - parent must handle";
        return result;
    }
};

/**
 * @brief Status of a concurrent region
 */
enum class ConcurrentRegionStatus {
    INACTIVE,  // Region is not active
    ACTIVE,    // Region is active and running
    FINAL,     // Region has reached a final state
    ERROR      // Region is in an error state
};

/**
 * @brief Configuration for concurrent state behavior (SCXML W3C compliant)
 *
 * SCXML specification mandates strict behavior for parallel states:
 * - Parallel states MUST have at least one region (section 3.4)
 * - ALL regions MUST complete for parallel state completion (section 3.4)
 * - Events MUST be broadcast to all active regions (section 3.4)
 */
struct ConcurrentStateConfig {
    // SCXML W3C compliance: parallel states must have regions
    // No fallback for empty regions - this violates SCXML specification

    // SCXML W3C compliance: ALL regions must complete for state completion
    // No configuration option - this is mandated by specification

    // SCXML W3C compliance: events must be broadcast to all regions
    // No configuration option - this is mandated by specification

    // Reserved for future SCXML-compliant extensions only
    bool _reserved_for_future_scxml_extensions = false;
};

/**
 * @brief Information about a concurrent region
 */
struct ConcurrentRegionInfo {
    std::string id;
    ConcurrentRegionStatus status;
    std::string currentState;
    bool isInFinalState;
    std::vector<std::string> activeStates;  // For compound regions
};

}  // namespace SCE