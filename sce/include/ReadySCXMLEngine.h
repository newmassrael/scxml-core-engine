// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Production-ready, high-level SCXML engine interface
 *
 * This is the primary interface users should interact with.
 * All complexity (sessions, threading, initialization) is hidden internally.
 * Ready-to-use with zero configuration required.
 *
 * Example usage:
 * ```cpp
 * auto engine = ReadySCXMLEngine::fromFile("workflow.scxml");
 * engine->start();
 * engine->sendEvent("user_action");
 * if (engine->isInState("completed")) {
 *     // Handle completion
 * }
 * ```
 */
class ReadySCXMLEngine {
public:
    virtual ~ReadySCXMLEngine() = default;

    // === Factory Methods (Hide Complex Construction) ===

    /**
     * @brief Create engine from SCXML file
     * @param scxmlFile Path to SCXML file
     * @return Engine instance or nullptr on error (call lastFactoryError() for details)
     */
    static std::unique_ptr<ReadySCXMLEngine> fromFile(const std::string &scxmlFile);

    /**
     * @brief Create engine from SCXML string content
     * @param scxmlContent SCXML document as string
     * @return Engine instance or nullptr on error (call lastFactoryError() for details)
     */
    static std::unique_ptr<ReadySCXMLEngine> fromString(const std::string &scxmlContent);

    /**
     * @brief Get the error message from the last failed factory call
     * @return Error detail from the most recent fromFile()/fromString() failure
     */
    static const std::string &lastFactoryError();

    // === Core State Machine Operations ===

    /**
     * @brief Start the state machine
     * @return true if started successfully
     */
    virtual bool start() = 0;

    /**
     * @brief Stop the state machine
     */
    virtual void stop() = 0;

    /**
     * @brief Send an event to the state machine
     * @param eventName Name of the event
     * @param eventData Optional event data (JSON string)
     * @return true if event was processed
     */
    virtual bool sendEvent(const std::string &eventName, const std::string &eventData = "") = 0;

    /**
     * @brief Send an external event to the state machine's external event queue
     * @param eventName Name of the event
     * @param eventData Optional event data (JSON string)
     * @return true if event was queued successfully
     */
    virtual bool sendExternalEvent(const std::string &eventName, const std::string &eventData = "") = 0;

    // === State Query Operations ===

    /**
     * @brief Check if state machine is running
     * @return true if running
     */
    virtual bool isRunning() const = 0;

    /**
     * @brief Get current active state
     * @return Current state ID, empty if not started
     */
    virtual std::string getCurrentState() const = 0;

    /**
     * @brief Check if a specific state is currently active
     * @param stateId State ID to check
     * @return true if state is active
     */
    virtual bool isInState(const std::string &stateId) const = 0;

    /**
     * @brief Get all currently active states (for hierarchical/parallel states)
     * @return Vector of active state IDs
     */
    virtual std::vector<std::string> getActiveStates() const = 0;

    // === Simple Variable Access ===

    /**
     * @brief Set a string variable in the state machine's data model
     * @param name Variable name
     * @param value String value
     * @return true if variable was set successfully
     */
    virtual bool setVariable(const std::string &name, const std::string &value) = 0;

    /**
     * @brief Route const char* to the string overload (prevents implicit bool conversion)
     */
    bool setVariable(const std::string &name, const char *value) { return setVariable(name, std::string(value)); }

    /**
     * @brief Set a boolean variable in the state machine's data model
     * @param name Variable name
     * @param value Boolean value (native JavaScript boolean type)
     * @return true if variable was set successfully
     */
    virtual bool setVariable(const std::string &name, bool value) = 0;

    /**
     * @brief Set a numeric variable in the state machine's data model
     * @param name Variable name
     * @param value Double value (native JavaScript number type)
     * @return true if variable was set successfully
     */
    virtual bool setVariable(const std::string &name, double value) = 0;

    /**
     * @brief Set an integer variable in the state machine's data model
     * @param name Variable name
     * @param value Integer value (native JavaScript number type)
     * @return true if variable was set successfully
     */
    virtual bool setVariable(const std::string &name, int64_t value) = 0;

    /**
     * @brief Get a variable from the state machine's data model
     * @param name Variable name
     * @return Variable value as string, empty if not found
     */
    virtual std::string getVariable(const std::string &name) const = 0;

    // === Error Information ===

    /**
     * @brief Get last error message
     * @return Error message, empty if no error
     */
    virtual std::string getLastError() const = 0;

    // === Statistics (Optional) ===

    /**
     * @brief Get basic statistics
     */
    struct Statistics {
        int totalEvents = 0;
        int totalTransitions = 0;
        std::string currentState;
        bool isRunning = false;
    };

    virtual Statistics getStatistics() const = 0;
};

}  // namespace SCE