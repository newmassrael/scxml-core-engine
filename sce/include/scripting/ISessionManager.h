// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ISessionLifecycle.h"
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Extended session management with session query capabilities
 *
 * Extends ISessionLifecycle with the queries a session owner needs beyond
 * create/destroy/has. Used by EventRaiserService for session validation.
 */
class ISessionManager : public virtual ISessionLifecycle {
public:
    virtual ~ISessionManager() = default;

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