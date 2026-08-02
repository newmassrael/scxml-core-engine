// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "common/IOProcessorHelper.h"
#include "scripting/ISessionManager.h"
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

namespace SCE {

/**
 * @brief Concrete implementation of session lifecycle management
 *
 * SOLID Architecture: Single Responsibility for session lifecycle management.
 * Separated from JavaScript execution concerns for better testability and maintainability.
 */
class SessionManagerImpl : public ISessionManager {
public:
    SessionManagerImpl() = default;
    ~SessionManagerImpl() override = default;

    // Non-copyable, non-movable for thread safety
    SessionManagerImpl(const SessionManagerImpl &) = delete;
    SessionManagerImpl &operator=(const SessionManagerImpl &) = delete;
    SessionManagerImpl(SessionManagerImpl &&) = delete;
    SessionManagerImpl &operator=(SessionManagerImpl &&) = delete;

    // === Core Session Management ===

    /**
     * @brief Check if a session exists
     * @param sessionId Session identifier to check
     * @return true if session exists
     */
    bool hasSession(const std::string &sessionId) const override;

    /**
     * @brief Create a new session with optional parent relationship
     * @param sessionId New session identifier
     * @param parentSessionId Parent session identifier (optional)
     * @return true if session created successfully
     */
    bool createSession(const std::string &sessionId, const std::string &parentSessionId = "") override;

    /**
     * @brief Destroy an existing session
     * @param sessionId Session identifier to destroy
     * @return true if session destroyed successfully
     */
    bool destroySession(const std::string &sessionId) override;

    // === Extended Session Management ===

    /**
     * @brief Get list of all active sessions
     * @return Vector of session identifiers
     */
    std::vector<std::string> getActiveSessions() const override;

    /**
     * @brief Get parent session ID for a given session
     * @param sessionId Session to get parent for
     * @return Parent session ID or empty string if no parent
     */
    std::string getParentSessionId(const std::string &sessionId) const override;

    // === Session Information Management ===

    /**
     * @brief Update session system variables
     * @param sessionId Target session
     * @param sessionName Human-readable session name
     * @param ioProcessors Entries to publish in `_ioprocessors`
     * @return true if update successful
     */
    bool updateSessionSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                      const std::vector<IOProcessorDescriptor> &ioProcessors);

    /**
     * @brief Get session name for display purposes
     * @param sessionId Session identifier
     * @return Session name or empty string if not found
     */
    std::string getSessionName(const std::string &sessionId) const;

    /**
     * @brief Get I/O processors for a session
     * @param sessionId Session identifier
     * @return Entries published in the session's `_ioprocessors`
     */
    std::vector<IOProcessorDescriptor> getSessionIOProcessors(const std::string &sessionId) const;

private:
    // === Internal Data Structures ===

    struct SessionInfo {
        std::string sessionId;
        std::string parentSessionId;
        std::string sessionName;
        std::vector<IOProcessorDescriptor> ioProcessors;

        SessionInfo() = default;

        SessionInfo(const std::string &id, const std::string &parent = "") : sessionId(id), parentSessionId(parent) {}
    };

    // === Thread-safe Storage ===
    mutable std::mutex sessionsMutex_;
    std::unordered_map<std::string, SessionInfo> sessions_;

    // === Internal Helper Methods ===

    /**
     * @brief Validate session ID format and constraints
     * @param sessionId Session identifier to validate
     * @return true if valid
     */
    bool isValidSessionId(const std::string &sessionId) const;

    /**
     * @brief Check if parent session exists (if specified)
     * @param parentSessionId Parent session to validate
     * @return true if valid or empty
     */
    bool isValidParentSession(const std::string &parentSessionId) const;
};

}  // namespace SCE