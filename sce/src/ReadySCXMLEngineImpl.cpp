// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "ReadySCXMLEngine.h"
#include "SCXMLEngine.h"
#include "core/LogMacros.h"
#include <atomic>
#include <filesystem>
#include <fstream>

namespace SCE {

/**
 * @brief Internal implementation of ReadySCXMLEngine
 *
 * Now uses SCXMLEngine's high-level API for a unified architecture:
 * - Automatic session management via SCXMLEngine
 * - Simplified initialization
 * - Direct delegation to SCXMLEngine high-level methods
 * - Consistent error handling across the stack
 */
class ReadySCXMLEngineImpl : public ReadySCXMLEngine {
private:
    std::unique_ptr<SCXMLEngine> scxmlEngine_;
    std::string sessionId_;
    std::string lastError_;
    bool initialized_ = false;

    // Internal helper for setVariable overloads (Zero Duplication)
    template <typename T> bool setVariableImpl(const std::string &name, T value) {
        if (!initialized_ || !scxmlEngine_) {
            lastError_ = "Engine not initialized";
            return false;
        }

        try {
            bool result = scxmlEngine_->setVariableSync(name, value, sessionId_);
            if (!result) {
                lastError_ = scxmlEngine_->getLastStateMachineError(sessionId_);
                SCE_LOG_WARN("ReadySCXMLEngine: Failed to set variable '{}': {}", name, lastError_);
            }
            return result;
        } catch (const std::exception &e) {
            lastError_ = std::string("Variable setting exception: ") + e.what();
            SCE_LOG_ERROR("ReadySCXMLEngine: Variable '{}' exception: {}", name, e.what());
            return false;
        }
    }

public:
    ReadySCXMLEngineImpl() {
        // Create SCXMLEngine instance
        scxmlEngine_ = createSCXMLEngine();
        // Generate unique session ID to prevent conflicts between instances
        static std::atomic<uint64_t> instanceCounter{0};
        sessionId_ = "ready_session_" + std::to_string(instanceCounter.fetch_add(1));
    }

    ~ReadySCXMLEngineImpl() {
        if (initialized_ && scxmlEngine_) {
            // Stop state machine and cleanup session
            scxmlEngine_->stopStateMachine(sessionId_);
            // Destroy session to prevent resource leaks and session ID conflicts
            scxmlEngine_->destroySession(sessionId_);
        }
    }

    bool initialize(const std::string &scxmlContent) {
        try {
            if (!initEngine()) return false;

            if (!scxmlEngine_->loadSCXMLFromString(scxmlContent, sessionId_)) {
                lastError_ = "Failed to load SCXML content: " + scxmlEngine_->getLastStateMachineError(sessionId_);
                SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
                return false;
            }

            initialized_ = true;
            SCE_LOG_INFO("ReadySCXMLEngine: Initialized successfully with session: {}", sessionId_);
            return true;

        } catch (const std::exception &e) {
            lastError_ = std::string("Initialization failed: ") + e.what();
            SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            return false;
        }
    }

    bool initializeFromFile(const std::string &scxmlFile) {
        try {
            if (!initEngine()) return false;

            // W3C SCXML: Use loadSCXMLFromFile to preserve base path for invoke relative resolution
            if (!scxmlEngine_->loadSCXMLFromFile(scxmlFile, sessionId_)) {
                lastError_ = "Failed to load SCXML file: " + scxmlEngine_->getLastStateMachineError(sessionId_);
                SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
                return false;
            }

            initialized_ = true;
            SCE_LOG_INFO("ReadySCXMLEngine: Initialized from file with session: {}", sessionId_);
            return true;

        } catch (const std::exception &e) {
            lastError_ = std::string("Initialization from file failed: ") + e.what();
            SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            return false;
        }
    }

private:
    bool initEngine() {
        if (!scxmlEngine_) {
            lastError_ = "SCXMLEngine is null";
            SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            return false;
        }
        if (!scxmlEngine_->initialize()) {
            lastError_ = "Failed to initialize SCXMLEngine";
            SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            return false;
        }
        return true;
    }

public:

    // === ReadySCXMLEngine Interface Implementation ===

    bool start() override {
        if (!initialized_) {
            lastError_ = "Engine not initialized";
            return false;
        }

        try {
            bool result = scxmlEngine_->startStateMachine(sessionId_);
            if (!result) {
                lastError_ = scxmlEngine_->getLastStateMachineError(sessionId_);
                SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            }
            return result;
        } catch (const std::exception &e) {
            lastError_ = std::string("Start failed: ") + e.what();
            SCE_LOG_ERROR("ReadySCXMLEngine: {}", lastError_);
            return false;
        }
    }

    void stop() override {
        if (initialized_ && scxmlEngine_) {
            try {
                scxmlEngine_->stopStateMachine(sessionId_);
            } catch (const std::exception &e) {
                SCE_LOG_WARN("ReadySCXMLEngine: Exception during stop: {}", e.what());
            }
        }
    }

    bool sendEvent(const std::string &eventName, const std::string &eventData) override {
        if (!initialized_) {
            lastError_ = "Engine not initialized";
            return false;
        }

        if (!scxmlEngine_->isStateMachineRunning(sessionId_)) {
            lastError_ = "State machine is not running";
            return false;
        }

        try {
            bool result = scxmlEngine_->sendEventSync(eventName, sessionId_, eventData);
            if (!result) {
                lastError_ = scxmlEngine_->getLastStateMachineError(sessionId_);
                SCE_LOG_WARN("ReadySCXMLEngine: Event '{}' failed: {}", eventName, lastError_);
            }
            return result;
        } catch (const std::exception &e) {
            lastError_ = std::string("Event processing exception: ") + e.what();
            SCE_LOG_ERROR("ReadySCXMLEngine: Event '{}' exception: {}", eventName, e.what());
            return false;
        }
    }

    bool sendExternalEvent(const std::string &eventName, const std::string &eventData) override {
        if (!initialized_) {
            lastError_ = "Engine not initialized";
            return false;
        }

        if (!scxmlEngine_) {
            lastError_ = "No engine available";
            return false;
        }

        try {
            return scxmlEngine_->raiseExternalEvent(eventName, sessionId_, eventData);
        } catch (const std::exception &e) {
            lastError_ = std::string("External event error: ") + e.what();
            return false;
        }
    }

    bool isRunning() const override {
        return initialized_ && scxmlEngine_ && scxmlEngine_->isStateMachineRunning(sessionId_);
    }

    std::string getCurrentState() const override {
        if (!initialized_ || !scxmlEngine_) {
            return "";
        }
        return scxmlEngine_->getCurrentStateSync(sessionId_);
    }

    bool isInState(const std::string &stateId) const override {
        if (!initialized_ || !scxmlEngine_) {
            return false;
        }
        return scxmlEngine_->isInStateSync(stateId, sessionId_);
    }

    std::vector<std::string> getActiveStates() const override {
        if (!initialized_ || !scxmlEngine_) {
            return {};
        }
        return scxmlEngine_->getActiveStatesSync(sessionId_);
    }

    bool setVariable(const std::string &name, const std::string &value) override {
        return setVariableImpl(name, value);
    }

    bool setVariable(const std::string &name, bool value) override {
        return setVariableImpl(name, value);
    }

    bool setVariable(const std::string &name, double value) override {
        return setVariableImpl(name, value);
    }

    bool setVariable(const std::string &name, int64_t value) override {
        return setVariableImpl(name, value);
    }

    std::string getVariable(const std::string &name) const override {
        if (!initialized_ || !scxmlEngine_) {
            return "";
        }

        try {
            return scxmlEngine_->getVariableSync(name, sessionId_);
        } catch (const std::exception &e) {
            SCE_LOG_WARN("ReadySCXMLEngine: Failed to get variable '{}': {}", name, e.what());
            return "";
        }
    }

    std::string getLastError() const override {
        return lastError_;
    }

    Statistics getStatistics() const override {
        Statistics stats;

        if (initialized_ && scxmlEngine_) {
            // Get real statistics from SCXMLEngine
            auto engineStats = scxmlEngine_->getStatisticsSync(sessionId_);
            stats.totalEvents = engineStats.totalEvents;
            stats.totalTransitions = engineStats.totalTransitions;
            stats.currentState = engineStats.currentState;
            stats.isRunning = engineStats.isRunning;
        }

        return stats;
    }
};

// === Factory Method Implementations ===

// Thread-local storage for factory error messages
static thread_local std::string s_lastFactoryError;

const std::string &ReadySCXMLEngine::lastFactoryError() {
    return s_lastFactoryError;
}

std::unique_ptr<ReadySCXMLEngine> ReadySCXMLEngine::fromFile(const std::string &scxmlFile) {
    s_lastFactoryError.clear();

    if (!std::filesystem::exists(scxmlFile)) {
        s_lastFactoryError = "SCXML file not found: " + scxmlFile;
        SCE_LOG_ERROR("ReadySCXMLEngine: {}", s_lastFactoryError);
        return nullptr;
    }

    auto engine = std::make_unique<ReadySCXMLEngineImpl>();

    // W3C SCXML: Use file-based loading to preserve base path for invoke relative resolution
    if (!engine->initializeFromFile(scxmlFile)) {
        s_lastFactoryError = engine->getLastError();
        SCE_LOG_ERROR("ReadySCXMLEngine: Failed to initialize from file: {}", scxmlFile);
        return nullptr;
    }

    return engine;
}

std::unique_ptr<ReadySCXMLEngine> ReadySCXMLEngine::fromString(const std::string &scxmlContent) {
    s_lastFactoryError.clear();

    if (scxmlContent.empty()) {
        s_lastFactoryError = "Empty SCXML content";
        SCE_LOG_ERROR("ReadySCXMLEngine: {}", s_lastFactoryError);
        return nullptr;
    }

    auto engine = std::make_unique<ReadySCXMLEngineImpl>();

    if (!engine->initialize(scxmlContent)) {
        s_lastFactoryError = engine->getLastError();
        SCE_LOG_ERROR("ReadySCXMLEngine: Failed to initialize with SCXML content");
        return nullptr;
    }

    return engine;
}

}  // namespace SCE