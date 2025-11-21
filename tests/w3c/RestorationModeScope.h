#pragma once

#include "runtime/StateMachine.h"
#include <memory>

namespace SCE::W3C {

/**
 * @brief RAII guard for StateMachine restoration mode (W3C SCXML 3.13)
 *
 * Automatically enables restoration mode on construction and disables it on destruction.
 * This ensures proper cleanup even in the presence of exceptions, following the
 * Resource Acquisition Is Initialization (RAII) pattern.
 *
 * Usage:
 * @code
 * {
 *     RestorationModeScope guard(stateMachine.get());
 *     // Restoration mode is enabled here
 *     // ... perform snapshot restoration ...
 *     // Restoration mode automatically disabled when scope exits
 * }
 * @endcode
 *
 * Exception Safety: Strong guarantee - restoration mode will be disabled even if
 * an exception is thrown during snapshot restoration.
 */
class RestorationModeScope {
public:
    /**
     * @brief Constructor - enables restoration mode on all parallel regions
     * @param stateMachine The state machine to guard (must not be null)
     */
    explicit RestorationModeScope(StateMachine *stateMachine) : stateMachine_(stateMachine) {
        if (stateMachine_) {
            stateMachine_->setRestoringSnapshotOnAllRegions(true);
        }
    }

    /**
     * @brief Destructor - disables restoration mode on all parallel regions
     *
     * Guaranteed to execute even if exceptions are thrown, ensuring
     * restoration mode is always properly cleaned up.
     */
    ~RestorationModeScope() {
        if (stateMachine_) {
            stateMachine_->setRestoringSnapshotOnAllRegions(false);
        }
    }

    // Non-copyable: RAII guard should not be copied
    RestorationModeScope(const RestorationModeScope &) = delete;
    RestorationModeScope &operator=(const RestorationModeScope &) = delete;

    // Non-movable: Keep lifetime management simple
    RestorationModeScope(RestorationModeScope &&) = delete;
    RestorationModeScope &operator=(RestorationModeScope &&) = delete;

private:
    StateMachine *stateMachine_;
};

}  // namespace SCE::W3C
