// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateMachineBuilder.h"

#include <stdexcept>

namespace SCE {

std::shared_ptr<StateMachine> StateMachineBuilder::build() {
    if (!scriptEngine_) {
        throw std::runtime_error("StateMachineBuilder::build: withScriptEngine() must be called before build()");
    }

    // Create StateMachine with engine injection
    auto stateMachine = std::make_shared<StateMachine>(*scriptEngine_, sessionId_);

    // Inject dependencies after construction
    if (!basicHttpAccessUri_.empty()) {
        stateMachine->setBasicHttpAccessUri(basicHttpAccessUri_);
    }

    if (eventDispatcher_) {
        stateMachine->setEventDispatcher(eventDispatcher_);
    }

    if (eventRaiser_) {
        stateMachine->setEventRaiser(eventRaiser_);

        // Apply scheduler mode for parent-child inheritance
        // Get scheduler from EventRaiser and set mode (MANUAL for interactive debugging)
        auto scheduler = eventRaiser_->getScheduler();
        if (scheduler) {
            scheduler->setMode(schedulerMode_);
        }
    }

    return stateMachine;
}

}  // namespace SCE
