// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ISessionLifecycle.h"
#include <string>
#include <vector>

namespace SCE {

// Forward declaration
class ISessionObserver;

/**
 * @brief Extended session management with observer support
 *
 * Extends ISessionLifecycle with observer pattern and session query capabilities.
 * Used by EventRaiserService for session validation.
 */
class ISessionManager : public virtual ISessionLifecycle {
public:
    virtual ~ISessionManager() = default;

    // === Observer Pattern Support ===

    /**
     * @brief Add observer for session lifecycle events
     * @param observer Observer to be notified of session events
     */
    virtual void addObserver(ISessionObserver *observer) = 0;

    /**
     * @brief Remove observer from session lifecycle events
     * @param observer Observer to be removed
     */
    virtual void removeObserver(ISessionObserver *observer) = 0;

    // === Extended Session Management ===

    /**
     * @brief Get list of all active sessions
     * @return Vector of session identifiers
     */
    virtual std::vector<std::string> getActiveSessions() const = 0;

    /**
     * @brief Get parent session ID for a given session
     * @param sessionId Session to get parent for
     * @return Parent session ID or empty string if no parent
     */
    virtual std::string getParentSessionId(const std::string &sessionId) const = 0;
};

}  // namespace SCE