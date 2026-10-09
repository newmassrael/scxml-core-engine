// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

        // A mode is written only when the caller asked for one. A child built for <invoke>
        // shares its parent's scheduler, so leaving the mode alone is how it inherits MANUAL.
        if (schedulerMode_) {
            if (auto scheduler = eventRaiser_->getScheduler()) {
                scheduler->setMode(*schedulerMode_);
            }
        }
    }

    return stateMachine;
}

}  // namespace SCE
