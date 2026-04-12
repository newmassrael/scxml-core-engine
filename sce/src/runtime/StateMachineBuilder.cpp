// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateMachineBuilder.h"
#include "scripting/ScriptEngineProvider.h"

namespace SCE {

std::shared_ptr<StateMachine> StateMachineBuilder::build() {
    // Resolve script engine: explicit injection or configured provider default
    IScriptEngine &engine = scriptEngine_ ? *scriptEngine_ : ScriptEngineProvider::getScriptEngine();

    // Create StateMachine with engine injection
    auto stateMachine = std::make_shared<StateMachine>(engine, sessionId_);

    // Inject dependencies after construction
    if (eventDispatcher_) {
        stateMachine->setEventDispatcher(eventDispatcher_);
    }

    if (eventRaiser_) {
        stateMachine->setEventRaiser(eventRaiser_);

        // W3C SCXML 3.13: Apply scheduler mode for parent-child inheritance
        // Get scheduler from EventRaiser and set mode (MANUAL for interactive debugging)
        auto scheduler = eventRaiser_->getScheduler();
        if (scheduler) {
            scheduler->setMode(schedulerMode_);
        }
    }

    return stateMachine;
}

}  // namespace SCE
