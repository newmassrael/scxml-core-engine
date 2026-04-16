// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <memory>
#include <string>

namespace SCE {

class IEventRaiser;

/**
 * @brief Interface for EventRaiser registration and management (SOLID: Interface Segregation)
 *
 * This interface provides a clean abstraction for EventRaiser management,
 * allowing components to register EventRaisers without tight coupling to JSEngine.
 * Follows SOLID principles:
 * - Single Responsibility: Only handles EventRaiser registration
 * - Interface Segregation: Minimal, focused interface
 * - Dependency Inversion: Components depend on abstraction, not implementation
 */
class IEventRaiserRegistry {
public:
    virtual ~IEventRaiserRegistry() = default;

    /**
     * @brief Register EventRaiser for a session
     *
     * @param sessionId Target session identifier
     * @param eventRaiser EventRaiser instance to register
     * @return true if registration successful, false otherwise
     */
    virtual bool registerEventRaiser(const std::string &sessionId, std::shared_ptr<IEventRaiser> eventRaiser) = 0;

    /**
     * @brief Get EventRaiser for a session
     *
     * @param sessionId Target session identifier
     * @return EventRaiser instance or nullptr if not found
     */
    virtual std::shared_ptr<IEventRaiser> getEventRaiser(const std::string &sessionId) const = 0;

    /**
     * @brief Unregister EventRaiser for a session
     *
     * @param sessionId Target session identifier
     * @return true if unregistration successful, false if not found
     */
    virtual bool unregisterEventRaiser(const std::string &sessionId) = 0;

    /**
     * @brief Check if session has a registered EventRaiser
     *
     * @param sessionId Target session identifier
     * @return true if EventRaiser is registered for this session
     */
    virtual bool hasEventRaiser(const std::string &sessionId) const = 0;

    /**
     * @brief Clear all registered EventRaisers
     *
     * Removes all session-to-EventRaiser mappings. Used for test isolation
     * and engine shutdown cleanup.
     */
    virtual void clear() = 0;
};

}  // namespace SCE