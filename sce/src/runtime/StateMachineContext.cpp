// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateMachineContext.h"
#include "core/LogMacros.h"

namespace SCE {

StateMachineContext::StateMachineContext(std::shared_ptr<StateMachine> stateMachine)
    : stateMachine_(std::move(stateMachine)) {
    SCE_LOG_DEBUG("StateMachineContext: Created with StateMachine (shared): {}", (void *)stateMachine_.get());
}

StateMachineContext::~StateMachineContext() {
    SCE_LOG_DEBUG("StateMachineContext: Starting automatic cleanup");

    // Only cleanup StateMachine
    // EventRaiser/EventDispatcher are owned externally (e.g., TestResources)
    if (stateMachine_) {
        // A run still going is exited as a stopped one (§scxml-D-exitInterpreter);
        // stop() is a no-op on one that already ended. The script session and
        // its In() callback are released by the StateMachine's destructor.
        SCE_LOG_DEBUG("StateMachineContext: Calling StateMachine::stop() (isRunning: {})", stateMachine_->isRunning());
        stateMachine_->stop();

        // With shared_ptr ownership, callbacks using weak_ptr are safe
        // No sleep needed - callbacks will check weak_ptr validity
        SCE_LOG_DEBUG("StateMachineContext: Releasing StateMachine (shared_ptr, use_count: {})",
                      stateMachine_.use_count());
        stateMachine_.reset();
    }

    SCE_LOG_DEBUG("StateMachineContext: Automatic cleanup completed");
}

}  // namespace SCE