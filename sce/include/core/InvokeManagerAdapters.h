#pragma once

#include <memory>
#include <string>
#include <vector>

/**
 * @file InvokeManagerAdapters.h
 * @brief AOT invoke manager adapter (W3C SCXML 6.4)
 *
 * Design principles (same as AOTEventQueue):
 * 1. Minimal interface for InvokeProcessingAlgorithms
 * 2. Engine-specific implementations hidden behind uniform API
 * 3. Zero overhead through inline methods
 *
 * Note: InterpreterInvokeManager is in runtime/InterpreterInvokeManager.h
 * (separated to maintain sce_core's header-only boundary).
 *
 * Required interface for InvokeProcessingAlgorithms:
 * - std::string getFinalizeScript(const std::string& childSessionId)
 * - std::vector<std::shared_ptr<StateMachine>>
 *   getAutoforwardSessions(const std::string& parentSessionId)
 */

namespace SCE::Core {

// Forward declaration
class StateMachine;

/**
 * @brief AOT engine invoke manager adapter
 *
 * Adapts AOT's Policy class (containing activeInvokes_ map) to the unified
 * interface required by InvokeProcessingAlgorithms.
 *
 * Implementation notes:
 * - Policy stores invoke info in activeInvokes_ map (sessionId -> ChildSession)
 * - ChildSession contains: invokeId, finalizeScript, autoforward flag, StateMachine*
 * - Adapter extracts data from Policy's flat map structure
 *
 * @tparam Policy Generated policy class from StaticCodeGenerator
 *         Must provide:
 *         - std::unordered_map<std::string, ChildSession> activeInvokes_;
 *         - struct ChildSession { finalizeScript, autoforward, parentSessionId, stateMachine }
 *
 * @example Usage in generated AOT code:
 * @code
 * SCE::Core::AOTInvokeManager<MyStateMachinePolicy> adapter(policy_);
 * SCE::Core::InvokeProcessingAlgorithms::processFinalize(
 *     getOriginSessionId(event),
 *     adapter,
 *     *this
 * );
 * @endcode
 */
template <typename Policy> class AOTInvokeManager {
public:
    /**
     * @brief Constructor
     * @param policy Reference to Policy instance containing invoke data
     */
    explicit AOTInvokeManager(Policy &policy) : policy_(policy) {}

    /**
     * @brief Get finalize script for child session (W3C SCXML 6.5)
     * @param childSessionId Child session ID that sent event
     * @return Finalize script if exists, empty string otherwise
     */
    std::string getFinalizeScript(const std::string &childSessionId) const {
        // Find invoke by matching child session ID
        for (const auto &[invokeId, session] : policy_.activeInvokes_) {
            if (session.sessionId == childSessionId) {
                return session.finalizeScript;
            }
        }
        return "";
    }

    /**
     * @brief Get child sessions with autoforward enabled (W3C SCXML 6.4.1)
     * @param parentSessionId Parent session ID
     * @return Vector of child StateMachine shared_ptrs with autoforward=true
     */
    std::vector<std::shared_ptr<StateMachine>> getAutoforwardSessions(const std::string &parentSessionId) {
        std::vector<std::shared_ptr<StateMachine>> result;

        for (const auto &[sessionId, session] : policy_.activeInvokes_) {
            if (session.autoforward && session.parentSessionId == parentSessionId && session.stateMachine) {
                result.push_back(session.stateMachine);
            }
        }

        return result;
    }

private:
    Policy &policy_;
};

}  // namespace SCE::Core
