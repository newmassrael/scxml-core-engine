// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#ifdef __EMSCRIPTEN__

#include <emscripten/bind.h>
#include <emscripten/val.h>

#include "InteractiveTestRunner.h"

using namespace emscripten;
using namespace SCE::W3C;

/**
 * @brief Emscripten bindings for SCXML interactive visualizer
 *
 * Exposes InteractiveTestRunner to JavaScript for browser-based debugging.
 *
 * JavaScript Usage:
 * @code
 * const Module = await createVisualizer();
 * const runner = new Module.InteractiveTestRunner();
 *
 * // Load SCXML
 * const scxmlContent = "<scxml>...</scxml>";
 * runner.loadSCXML(scxmlContent, false);  // false = content string
 * runner.initialize();
 *
 * // Step through execution
 * runner.raiseEvent("switch_on");
 * runner.stepForward();
 *
 * // Inspect state
 * const states = runner.getActiveStates();
 * const dataModel = runner.getDataModel();
 * const transition = runner.getLastTransition();
 *
 * // Time-travel debugging
 * runner.stepBackward();
 * runner.reset();
 * @endcode
 */
EMSCRIPTEN_BINDINGS(interactive_test_runner) {
    // Bind std::vector<std::string> for getActiveStates()
    register_vector<std::string>("VectorString");

    // Bind InteractiveTestRunner class
    class_<InteractiveTestRunner>("InteractiveTestRunner")
        .constructor<>()

        // SCXML loading and initialization
        .function("loadSCXML", &InteractiveTestRunner::loadSCXML, allow_raw_pointers())

        .function("initialize", &InteractiveTestRunner::initialize)

        // Step control
        .function("stepForward", &InteractiveTestRunner::stepForward)

        .function("stepBackward", &InteractiveTestRunner::stepBackward)

        .function("reset", &InteractiveTestRunner::reset)

        .function("raiseEvent", &InteractiveTestRunner::raiseEvent)

        .function("removeInternalEvent", &InteractiveTestRunner::removeInternalEvent)

        .function("removeExternalEvent", &InteractiveTestRunner::removeExternalEvent)

        .function("pollScheduler", &InteractiveTestRunner::pollScheduler)

        // State introspection
        .function("getActiveStates", &InteractiveTestRunner::getActiveStates)

        .function("getCurrentStep", &InteractiveTestRunner::getCurrentStep)

        .function("isInFinalState", &InteractiveTestRunner::isInFinalState)

        .function("getLastTransition", &InteractiveTestRunner::getLastTransition)

        .function("getEventQueue", &InteractiveTestRunner::getEventQueue)

        .function("getScheduledEvents", &InteractiveTestRunner::getScheduledEvents)

        .function("getDataModel", &InteractiveTestRunner::getDataModel)

        .function("evaluateExpression", &InteractiveTestRunner::evaluateExpression)

        // SCXML structure for visualization
        .function("getSCXMLStructure", &InteractiveTestRunner::getSCXMLStructure)

        .function("getW3CReferences", &InteractiveTestRunner::getW3CReferences)

        .function("preloadFile", &InteractiveTestRunner::preloadFile)
        .function("setBasePath", &InteractiveTestRunner::setBasePath)
        .function("getInvokedChildren", &InteractiveTestRunner::getInvokedChildren)
        .function("getSubSCXMLStructures", &InteractiveTestRunner::getSubSCXMLStructures);
}

#endif  // __EMSCRIPTEN__
