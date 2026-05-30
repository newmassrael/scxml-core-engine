// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ISessionRegistry.h"
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>

namespace SCE {

/**
 * @brief Singleton session registry for SCXML infrastructure
 *
 * Engine-agnostic registry managing invoke mappings, file paths, event dispatchers,
 * and session ID generation. Extracted from JSEngine to enable pluggable script engines.
 *
 * Thread-safe: All operations protected by internal mutexes.
 */
class SessionRegistry : public ISessionRegistry {
public:
    /**
     * @brief Get the global SessionRegistry instance
     */
    static SessionRegistry &instance();

    /**
     * @brief Reset the registry (for test isolation)
     */
    void reset();

    // === ISessionRegistry Implementation ===

    void registerInvokeMapping(const std::string &parentSessionId, const std::string &invokeId,
                               const std::string &childSessionId) override;
    std::string getInvokeSessionId(const std::string &parentSessionId, const std::string &invokeId) const override;
    void unregisterInvokeMapping(const std::string &parentSessionId, const std::string &invokeId) override;
    std::string getInvokeIdForChildSession(const std::string &childSessionId) const override;
    void registerParentChild(const std::string &childSessionId, const std::string &parentSessionId) override;
    void unregisterParentChild(const std::string &childSessionId) override;
    std::string getParentSessionId(const std::string &childSessionId) const override;

    void registerSessionFilePath(const std::string &sessionId, const std::string &filePath) override;
    std::string getSessionFilePath(const std::string &sessionId) const override;
    void unregisterSessionFilePath(const std::string &sessionId) override;

    void registerEventDispatcher(const std::string &sessionId,
                                 std::shared_ptr<IEventDispatcher> eventDispatcher) override;
    void unregisterEventDispatcher(const std::string &sessionId) override;
    void cleanupSession(const std::string &sessionId) override;

    uint64_t generateSessionId() const override;
    std::string generateSessionIdString(const std::string &prefix = "session_") const override;

private:
    SessionRegistry() = default;
    ~SessionRegistry() = default;

    SessionRegistry(const SessionRegistry &) = delete;
    SessionRegistry &operator=(const SessionRegistry &) = delete;
    SessionRegistry(SessionRegistry &&) = delete;
    SessionRegistry &operator=(SessionRegistry &&) = delete;

    // === Invoke Session Mappings ===
    // W3C SCXML: Maps parent_session_id -> (invoke_id -> child_session_id)
    std::unordered_map<std::string, std::unordered_map<std::string, std::string>> invokeMappings_;
    mutable std::mutex invokeMappingsMutex_;

    // === Session File Paths ===
    // W3C SCXML: Maps session_id -> absolute_file_path for relative path resolution
    std::unordered_map<std::string, std::string> sessionFilePaths_;
    mutable std::mutex sessionFilePathsMutex_;

    // === Parent-Child Session Relationships ===
    // §scxml-6.4: Maps child_session_id -> parent_session_id
    std::unordered_map<std::string, std::string> parentChildMappings_;
    mutable std::mutex parentChildMutex_;

    // === Event Dispatchers ===
    // §scxml-6.2: EventDispatcher registry for automatic delayed event cancellation
    std::unordered_map<std::string, std::weak_ptr<class IEventDispatcher>> eventDispatchers_;
    mutable std::mutex eventDispatchersMutex_;
};

}  // namespace SCE
