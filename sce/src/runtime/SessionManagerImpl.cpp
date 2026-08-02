// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/SessionManagerImpl.h"
#include "core/LogMacros.h"
#include <algorithm>

namespace SCE {

bool SessionManagerImpl::hasSession(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    return sessions_.find(sessionId) != sessions_.end();
}

bool SessionManagerImpl::createSession(const std::string &sessionId, const std::string &parentSessionId) {
    // Validate input parameters
    if (!isValidSessionId(sessionId)) {
        SCE_LOG_ERROR("SessionManagerImpl: Invalid session ID: '{}'", sessionId);
        return false;
    }

    if (!isValidParentSession(parentSessionId)) {
        SCE_LOG_ERROR("SessionManagerImpl: Invalid parent session ID: '{}'", parentSessionId);
        return false;
    }

    std::lock_guard<std::mutex> lock(sessionsMutex_);

    // Check if session already exists
    if (sessions_.find(sessionId) != sessions_.end()) {
        SCE_LOG_DEBUG("SessionManagerImpl: Session '{}' already exists", sessionId);
        return false;
    }

    // Create session info
    SessionInfo sessionInfo(sessionId, parentSessionId);
    sessions_[sessionId] = sessionInfo;

    SCE_LOG_DEBUG("SessionManagerImpl: Created session '{}' with parent '{}' (total sessions: {})", sessionId,
                  parentSessionId.empty() ? "none" : parentSessionId, sessions_.size());

    return true;
}

bool SessionManagerImpl::destroySession(const std::string &sessionId) {
    if (sessionId.empty()) {
        SCE_LOG_ERROR("SessionManagerImpl: Cannot destroy session with empty ID");
        return false;
    }

    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        SCE_LOG_DEBUG("SessionManagerImpl: Session '{}' does not exist for destruction", sessionId);
        return false;
    }

    sessions_.erase(it);

    SCE_LOG_DEBUG("SessionManagerImpl: Destroyed session '{}' (remaining sessions: {})", sessionId, sessions_.size());

    return true;
}

std::vector<std::string> SessionManagerImpl::getActiveSessions() const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);

    std::vector<std::string> activeSessions;
    activeSessions.reserve(sessions_.size());

    for (const auto &pair : sessions_) {
        activeSessions.push_back(pair.first);
    }

    return activeSessions;
}

std::string SessionManagerImpl::getParentSessionId(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second.parentSessionId;
    }

    return "";  // Session not found
}

bool SessionManagerImpl::updateSessionSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                      const std::vector<IOProcessorDescriptor> &ioProcessors) {
    if (sessionId.empty()) {
        SCE_LOG_ERROR("SessionManagerImpl: Cannot update system variables for empty session ID");
        return false;
    }

    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        SCE_LOG_ERROR("SessionManagerImpl: Cannot update system variables for non-existent session: '{}'", sessionId);
        return false;
    }

    // Update session information
    it->second.sessionName = sessionName;
    it->second.ioProcessors = ioProcessors;

    SCE_LOG_DEBUG("SessionManagerImpl: Updated system variables for session '{}': name='{}', {} I/O processors",
                  sessionId, sessionName, ioProcessors.size());

    return true;
}

std::string SessionManagerImpl::getSessionName(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second.sessionName;
    }

    return "";  // Session not found
}

std::vector<IOProcessorDescriptor> SessionManagerImpl::getSessionIOProcessors(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second.ioProcessors;
    }

    return {};  // Session not found
}

// === Private Helper Methods ===

bool SessionManagerImpl::isValidSessionId(const std::string &sessionId) const {
    // Session ID must not be empty and should have reasonable length
    return !sessionId.empty() && sessionId.length() <= 256;
}

bool SessionManagerImpl::isValidParentSession(const std::string &parentSessionId) const {
    // Empty parent session is valid (no parent)
    if (parentSessionId.empty()) {
        return true;
    }

    // If specified, parent session must exist
    return sessions_.find(parentSessionId) != sessions_.end();
}

}  // namespace SCE