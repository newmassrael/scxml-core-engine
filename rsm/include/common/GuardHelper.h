#pragma once
#include "common/Logger.h"
#include "scripting/JSEngine.h"
#include <stdexcept>
#include <string>

namespace RSM::GuardHelper {

/**
 * @brief Evaluates a guard expression using JSEngine
 *
 * W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value
 * ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST
 * treat the expression as if it evaluated to 'false'.
 *
 * @param jsEngine Reference to JSEngine instance
 * @param sessionId JSEngine session ID
 * @param guardExpr Guard expression to evaluate (e.g., "typeof Var4 !== 'undefined'")
 * @return bool True if guard evaluates to true, false if evaluation fails or returns false
 */
inline bool evaluateGuard(JSEngine &jsEngine, const std::string &sessionId, const std::string &guardExpr) {
    auto guardResult = jsEngine.evaluateExpression(sessionId, guardExpr).get();

    if (!JSEngine::isSuccess(guardResult)) {
        // W3C SCXML 5.9: Evaluation errors must be treated as false
        LOG_WARN("W3C SCXML 5.9: Guard evaluation failed, treating as false: {}", guardExpr);
        return false;
    }

    return JSEngine::resultToBool(guardResult);
}

}  // namespace RSM::GuardHelper
