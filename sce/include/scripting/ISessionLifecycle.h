// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <string>

namespace SCE {

/**
 * @brief Core session lifecycle interface
 *
 * Minimal interface for session creation, destruction, and existence checks.
 * Base for both ISessionManager (with observer support) and IScriptEngine.
 */
class ISessionLifecycle {
public:
    virtual ~ISessionLifecycle() = default;

    /**
     * @brief Create a new session
     * @param sessionId Unique session identifier
     * @param parentSessionId Parent session identifier (optional)
     * @return true if session created successfully
     */
    virtual bool createSession(const std::string &sessionId, const std::string &parentSessionId = "") = 0;

    /**
     * @brief Destroy an existing session
     * @param sessionId Session identifier to destroy
     * @return true if session destroyed successfully
     */
    virtual bool destroySession(const std::string &sessionId) = 0;

    /**
     * @brief Check if a session exists
     * @param sessionId Session identifier to check
     * @return true if session exists
     */
    virtual bool hasSession(const std::string &sessionId) const = 0;
};

}  // namespace SCE
