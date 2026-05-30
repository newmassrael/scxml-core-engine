// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/SessionRegistry.h"
#include "common/UniqueIdGenerator.h"
#include "core/LogMacros.h"
#include "events/IEventDispatcher.h"

namespace SCE {

SessionRegistry &SessionRegistry::instance() {
    static SessionRegistry instance;
    return instance;
}

void SessionRegistry::reset() {
    {
        std::lock_guard<std::mutex> lock(invokeMappingsMutex_);
        invokeMappings_.clear();
    }
    {
        std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);
        sessionFilePaths_.clear();
    }
    {
        std::lock_guard<std::mutex> lock(parentChildMutex_);
        parentChildMappings_.clear();
    }
    {
        std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
        eventDispatchers_.clear();
    }
    SCE_LOG_DEBUG("SessionRegistry: Reset complete");
}

// === Invoke Mapping Management ===

void SessionRegistry::registerInvokeMapping(const std::string &parentSessionId, const std::string &invokeId,
                                            const std::string &childSessionId) {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);
    invokeMappings_[parentSessionId][invokeId] = childSessionId;
    SCE_LOG_DEBUG("SessionRegistry: Registered invoke mapping - parent: {}, invoke: {}, child: {}", parentSessionId,
                  invokeId, childSessionId);
}

std::string SessionRegistry::getInvokeSessionId(const std::string &parentSessionId,
                                                 const std::string &invokeId) const {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    auto parentIt = invokeMappings_.find(parentSessionId);
    if (parentIt == invokeMappings_.end()) {
        SCE_LOG_DEBUG("SessionRegistry: No invoke mappings found for parent session: {}", parentSessionId);
        return "";
    }

    auto invokeIt = parentIt->second.find(invokeId);
    if (invokeIt == parentIt->second.end()) {
        SCE_LOG_DEBUG("SessionRegistry: Invoke ID '{}' not found in parent session: {}", invokeId, parentSessionId);
        return "";
    }

    SCE_LOG_DEBUG("SessionRegistry: Found invoke mapping - parent: {}, invoke: {}, child: {}", parentSessionId,
                  invokeId, invokeIt->second);
    return invokeIt->second;
}

void SessionRegistry::unregisterInvokeMapping(const std::string &parentSessionId, const std::string &invokeId) {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    auto parentIt = invokeMappings_.find(parentSessionId);
    if (parentIt != invokeMappings_.end()) {
        parentIt->second.erase(invokeId);

        // Clean up empty parent entries
        if (parentIt->second.empty()) {
            invokeMappings_.erase(parentIt);
        }

        SCE_LOG_DEBUG("SessionRegistry: Unregistered invoke mapping - parent: {}, invoke: {}", parentSessionId,
                      invokeId);
    }
}

std::string SessionRegistry::getInvokeIdForChildSession(const std::string &childSessionId) const {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    // §scxml-5.10 test 338: Reverse lookup childSessionId -> invokeId
    for (const auto &parentEntry : invokeMappings_) {
        for (const auto &invokeEntry : parentEntry.second) {
            if (invokeEntry.second == childSessionId) {
                SCE_LOG_DEBUG("SessionRegistry: Found invokeId '{}' for child session '{}' in parent '{}'",
                              invokeEntry.first, childSessionId, parentEntry.first);
                return invokeEntry.first;
            }
        }
    }

    SCE_LOG_DEBUG("SessionRegistry: No invokeId found for child session: {}", childSessionId);
    return "";
}

void SessionRegistry::registerParentChild(const std::string &childSessionId, const std::string &parentSessionId) {
    if (childSessionId.empty() || parentSessionId.empty()) {
        return;
    }
    std::lock_guard<std::mutex> lock(parentChildMutex_);
    parentChildMappings_[childSessionId] = parentSessionId;
    SCE_LOG_DEBUG("SessionRegistry: Registered parent-child: parent='{}', child='{}'", parentSessionId, childSessionId);
}

void SessionRegistry::unregisterParentChild(const std::string &childSessionId) {
    std::lock_guard<std::mutex> lock(parentChildMutex_);
    parentChildMappings_.erase(childSessionId);
    SCE_LOG_DEBUG("SessionRegistry: Unregistered parent-child for child: {}", childSessionId);
}

std::string SessionRegistry::getParentSessionId(const std::string &childSessionId) const {
    // §scxml-6.4: Direct lookup from parent-child mapping
    {
        std::lock_guard<std::mutex> lock(parentChildMutex_);
        auto it = parentChildMappings_.find(childSessionId);
        if (it != parentChildMappings_.end()) {
            SCE_LOG_DEBUG("SessionRegistry: Found parent '{}' for child session '{}'", it->second, childSessionId);
            return it->second;
        }
    }

    // Fallback: Reverse lookup from invoke mappings (for backward compatibility)
    {
        std::lock_guard<std::mutex> lock(invokeMappingsMutex_);
        for (const auto &parentEntry : invokeMappings_) {
            for (const auto &invokeEntry : parentEntry.second) {
                if (invokeEntry.second == childSessionId) {
                    SCE_LOG_DEBUG("SessionRegistry: Found parent '{}' for child session '{}' (via invoke mapping)",
                                  parentEntry.first, childSessionId);
                    return parentEntry.first;
                }
            }
        }
    }

    SCE_LOG_DEBUG("SessionRegistry: No parent found for child session: {}", childSessionId);
    return "";
}

// === Session File Path Management ===

void SessionRegistry::registerSessionFilePath(const std::string &sessionId, const std::string &filePath) {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);
    sessionFilePaths_[sessionId] = filePath;
    SCE_LOG_DEBUG("SessionRegistry: Registered session file path - session: {}, path: {}", sessionId, filePath);
}

std::string SessionRegistry::getSessionFilePath(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);

    auto it = sessionFilePaths_.find(sessionId);
    if (it == sessionFilePaths_.end()) {
        SCE_LOG_DEBUG("SessionRegistry: No file path found for session: {}", sessionId);
        return "";
    }

    SCE_LOG_DEBUG("SessionRegistry: Found session file path - session: {}, path: {}", sessionId, it->second);
    return it->second;
}

void SessionRegistry::unregisterSessionFilePath(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);

    auto it = sessionFilePaths_.find(sessionId);
    if (it != sessionFilePaths_.end()) {
        sessionFilePaths_.erase(it);
        SCE_LOG_DEBUG("SessionRegistry: Unregistered session file path - session: {}", sessionId);
    }
}

// === Event Dispatcher Management ===

void SessionRegistry::registerEventDispatcher(const std::string &sessionId,
                                              std::shared_ptr<IEventDispatcher> eventDispatcher) {
    if (!eventDispatcher) {
        SCE_LOG_WARN("SessionRegistry: Attempted to register null EventDispatcher for session: {}", sessionId);
        return;
    }

    std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
    eventDispatchers_[sessionId] = eventDispatcher;
    SCE_LOG_DEBUG("SessionRegistry: Registered EventDispatcher for session: {}", sessionId);
}

void SessionRegistry::unregisterEventDispatcher(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
    auto it = eventDispatchers_.find(sessionId);
    if (it != eventDispatchers_.end()) {
        eventDispatchers_.erase(it);
        SCE_LOG_DEBUG("SessionRegistry: Unregistered EventDispatcher for session: {}", sessionId);
    }
}

void SessionRegistry::cleanupSession(const std::string &sessionId) {
    // §scxml-6.2: Cancel delayed events for terminating session
    {
        std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
        auto dispatcherIt = eventDispatchers_.find(sessionId);
        if (dispatcherIt != eventDispatchers_.end()) {
            auto eventDispatcher = dispatcherIt->second.lock();
            if (eventDispatcher) {
                size_t cancelledCount = eventDispatcher->cancelEventsForSession(sessionId);
                SCE_LOG_DEBUG("SessionRegistry: Cancelled {} delayed events for session: {}", cancelledCount,
                              sessionId);
            }
            eventDispatchers_.erase(dispatcherIt);
        }
    }

    // Clean up session file path mapping
    unregisterSessionFilePath(sessionId);
}

// === Session ID Generation ===

uint64_t SessionRegistry::generateSessionId() const {
    return UniqueIdGenerator::generateNumericSessionId();
}

std::string SessionRegistry::generateSessionIdString(const std::string &prefix) const {
    return UniqueIdGenerator::generateSessionId(prefix);
}

}  // namespace SCE
