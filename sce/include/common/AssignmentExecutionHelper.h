// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include <functional>
#include <string>

namespace SCE {

/**
 * @brief Helper class for W3C SCXML assignment execution logic
 *
 * ARCHITECTURE.md: Zero Duplication Principle
 * - Shared assignment execution between the Interpreter and the generated
 *   C++ machines, which call it with the same location and expression
 *
 * §scxml-5.4: <assign> element execution (modify the data model)
 * §scxml-B-2: the assignment replaces the value at the location, and raises
 * error.execution when it cannot
 *
 * Usage Pattern:
 * 1. Validate the location: the Interpreter with AssignHelper::isValidLocation(),
 *    the code generator from the location literal (`read_only_location`). Both
 *    refuse an empty location and the read-only system variables of
 *    §scxml-5.10 before anything is evaluated.
 * 2. Call AssignmentExecutionHelper::executeAssignment() with location and expression
 * 3. Handle error via callback (Interpreter: eventRaiser, AOT: engine.raise)
 */
class AssignmentExecutionHelper {
public:
    /**
     * @brief Execute one `<assign>`: the engine stores the value of @p expr at
     *        @p location itself
     *
     * ARCHITECTURE.md: Zero Duplication - Single Source of Truth for assignment execution
     *
     * One path for every location and every expression,
     * `IScriptEngine::assign`, which runs `<location> = (<expr>)` inside the
     * data model. The value never leaves the engine, so whatever it is — a
     * number, an object, `_event` itself (§scxml-5.10), or the DOM that XML
     * in `_event.data` arrives as — the location holds that same value
     * afterwards.
     *
     * It used to be three paths chosen by the shape of the text. A bare-name
     * location evaluated the expression to a `ScriptValue` and wrote that back
     * with `setVariable`; a `ScriptValue` has no kind for a DOM, a function or
     * an object's identity, so `<assign location="v" expr="_event.data"/>`
     * stored an empty object where the event carried a document. The exact
     * spellings `_event`, `_sessionid`, `_name`, `_ioprocessors` and `_x` were
     * routed around that to a script so that test 329's `Var2 = _event` kept
     * the reference — the same loss, patched for five strings, and missed by
     * `_event.data`, `_event['data']` or `(_event)`. A member-path location
     * was already the script this path now is for all of them.
     *
     * The expression is evaluated exactly once. Measured 2026-08-29 on
     * `<assign location="answers.d15" expr="++v"/>` with `v` at `'1'`: an
     * earlier shape evaluated it before choosing a path and again inside the
     * script, and the recorded value was 3 where §scxml-5.4 and ECMA-262
     * 13.4.4 say 2. A single statement cannot evaluate it twice.
     *
     * @param jsEngine script engine of the session
     * @param sessionId Session identifier
     * @param location Target location (validated by the caller, see above)
     * @param expr Expression to evaluate and assign
     * @param errorCallback Called on error with error message
     * @return true if assignment succeeded, false otherwise
     */
    static bool executeAssignment(IScriptEngine &jsEngine, const std::string &sessionId, const ScriptSource &location,
                                  const ScriptSource &expr, std::function<void(const std::string &)> errorCallback) {
        auto result = jsEngine.assign(sessionId, location, expr).get();
        if (!result.isSuccess()) {
            // Names the expression as well as the location: §scxml-5.4's
            // error is about evaluating the expression at least as often as
            // it is about the location, and the author's text is what both
            // are named by.
            const std::string errorMsg = "Assignment failed: " + location.source() + " = " + expr.source();
            SCE_LOG_ERROR("AssignmentExecutionHelper: {} ({})", errorMsg, result.getErrorMessage());
            errorCallback(errorMsg);
            return false;
        }
        SCE_LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {}", location.source(), expr.source());
        return true;
    }

    /**
     * @brief §scxml-6.2.4 / §scxml-6.4.1: write a generated id to an
     *        `idlocation`
     *
     * The location is a location expression, so this is the assignment
     * executeAssignment() makes: a member path lands, and one that cannot take
     * the id reports through `errorCallback` and answers false, for the caller
     * to raise error.execution and abandon the element (§scxml-5.9.2). The id
     * is quoted here, in the location's own language, because an invoke's is
     * a run-time value.
     */
    static bool storeIdInLocation(IScriptEngine &jsEngine, const std::string &sessionId, const ScriptSource &location,
                                  const std::string &id, std::function<void(const std::string &)> errorCallback) {
        return executeAssignment(jsEngine, sessionId, location, ScriptSource::stringLiteral(id, location.language()),
                                 std::move(errorCallback));
    }
};

}  // namespace SCE
