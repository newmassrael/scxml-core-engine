// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <memory>
#include <string>

namespace SCE {

class IEventDispatcher;

/**
 * @brief Interface for SCXML session infrastructure registry
 *
 * Provides engine-agnostic session registry services: invoke mappings,
 * file path resolution, event dispatcher management, and session ID generation.
 * Separated from script execution to enable pluggable script engines (W3C SCXML 6.2).
 */
class ISessionRegistry {
public:
    virtual ~ISessionRegistry() = default;

    // === Invoke Session Management (W3C SCXML #_invokeid support) ===

    /**
     * @brief Register invoke mapping for session communication
     * @param parentSessionId Parent session that created the invoke
     * @param invokeId Invoke identifier from the invoke element
     * @param childSessionId Child session created by the invoke
     */
    virtual void registerInvokeMapping(const std::string &parentSessionId, const std::string &invokeId,
                                       const std::string &childSessionId) = 0;

    /**
     * @brief Get child session ID for an invoke ID
     * @param parentSessionId Parent session to search in
     * @param invokeId Invoke identifier to lookup
     * @return Child session ID or empty string if not found
     */
    virtual std::string getInvokeSessionId(const std::string &parentSessionId, const std::string &invokeId) const = 0;

    /**
     * @brief Unregister invoke mapping during session cleanup
     * @param parentSessionId Parent session
     * @param invokeId Invoke identifier to remove
     */
    virtual void unregisterInvokeMapping(const std::string &parentSessionId, const std::string &invokeId) = 0;

    /**
     * @brief Get invoke ID for a child session (reverse lookup)
     * W3C SCXML 5.10 test 338: Enables setting invokeid field on events from invoked children
     * @param childSessionId Child session ID to lookup
     * @return Invoke ID that created this child session, or empty string if not found
     */
    virtual std::string getInvokeIdForChildSession(const std::string &childSessionId) const = 0;

    /**
     * @brief Register parent-child session relationship
     * W3C SCXML 6.4: Enables child-to-parent event routing without script engine coupling
     * @param childSessionId Child session ID
     * @param parentSessionId Parent session ID
     */
    virtual void registerParentChild(const std::string &childSessionId, const std::string &parentSessionId) = 0;

    /**
     * @brief Unregister parent-child session relationship
     * @param childSessionId Child session ID to remove
     */
    virtual void unregisterParentChild(const std::string &childSessionId) = 0;

    /**
     * @brief Get parent session ID for a child session
     * W3C SCXML 6.4: Enables child-to-parent event routing without script engine coupling
     * @param childSessionId Child session ID to lookup
     * @return Parent session ID, or empty string if not found
     */
    virtual std::string getParentSessionId(const std::string &childSessionId) const = 0;

    // === Session File Path Management (W3C SCXML relative path resolution) ===

    /**
     * @brief Register file path for session-based relative path resolution
     * @param sessionId Session identifier
     * @param filePath Absolute path to the SCXML file for this session
     */
    virtual void registerSessionFilePath(const std::string &sessionId, const std::string &filePath) = 0;

    /**
     * @brief Get the source file path for a session
     * @param sessionId Session to get file path for
     * @return Absolute file path or empty string if not found
     */
    virtual std::string getSessionFilePath(const std::string &sessionId) const = 0;

    /**
     * @brief Unregister session file path during cleanup
     * @param sessionId Session identifier to remove
     */
    virtual void unregisterSessionFilePath(const std::string &sessionId) = 0;

    // === Event Dispatcher Management (W3C SCXML 6.2) ===

    /**
     * @brief Register EventDispatcher for session-aware delayed event cancellation
     * @param sessionId Session identifier
     * @param eventDispatcher EventDispatcher instance to register
     */
    virtual void registerEventDispatcher(const std::string &sessionId,
                                         std::shared_ptr<IEventDispatcher> eventDispatcher) = 0;

    /**
     * @brief Unregister EventDispatcher for session cleanup
     * @param sessionId Session identifier
     */
    virtual void unregisterEventDispatcher(const std::string &sessionId) = 0;

    /**
     * @brief Cleanup session resources: cancel delayed events + unregister file path
     * W3C SCXML 6.2: Called during session destruction for proper resource cleanup
     * @param sessionId Session identifier to clean up
     */
    virtual void cleanupSession(const std::string &sessionId) = 0;

    // === Session ID Generation ===

    /**
     * @brief Generate unique session ID
     * @return Unique session ID number
     */
    virtual uint64_t generateSessionId() const = 0;

    /**
     * @brief Generate unique session ID string with prefix
     * @param prefix Prefix for session ID (e.g., "sm_", "session_")
     * @return Unique session ID string
     */
    virtual std::string generateSessionIdString(const std::string &prefix = "session_") const = 0;
};

}  // namespace SCE
