// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief A call into a shut-down script engine answers instead of hanging
 *
 * `JSEngine::executeScript` and its four siblings hand their work to
 * `PlatformExecutionHelper::executeAsync` and return the future; every caller
 * in this repository then waits on it with `.get()`. The queued (native)
 * executor runs that work on a worker thread which `shutdown()` joins.
 *
 * Queueing into a joined executor used to be silent: the operation went onto a
 * queue nothing would drain, the promise was never satisfied, and `.get()`
 * blocked forever — no log line, no error, no timeout. Measured 2026-08-19,
 * that is what a pre-push `ctest` reported as `IntegrationTests (Timeout
 * 180.10 sec)`, with the only clue being the name of the test it happened to be
 * sitting in. Two gates after it never ran, so nothing was claimed about them
 * either.
 *
 * The same shape reaches a consumer: an application that calls `shutdown()` and
 * then executes one more script hangs, rather than being told it is shut down.
 *
 * Every case here bounds its own wait with `wait_for` rather than calling
 * `.get()`. A test that reproduces a hang by hanging is not a test — it is the
 * defect again, wearing the harness's timeout.
 */

#include "scripting/JSEngine.h"

#include <chrono>
#include <future>
#include <gtest/gtest.h>
#include <string>

namespace SCE::Tests {
namespace {

/// Generous next to the microseconds an answer takes, and far below any
/// harness timeout: a wait that reaches this has hung, not been slow.
constexpr auto ANSWER_BUDGET = std::chrono::seconds(5);

/// Wait for `future` with a bound. Returns nullopt when the budget elapses,
/// which is the observation this file exists to make.
std::optional<ScriptResult> answer_within(std::future<ScriptResult> &future) {
    if (future.wait_for(ANSWER_BUDGET) != std::future_status::ready) {
        return std::nullopt;
    }
    return future.get();
}

class ShutDownEngineAnswersTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &JSEngine::instance();
        engine_->reset();
    }

    void TearDown() override {
        // Leave the singleton usable for whatever runs next in this binary:
        // a shut-down engine that the next test only `reset()`s is exactly the
        // state these cases are about.
        engine_->reset();
    }

    JSEngine *engine_ = nullptr;
};

}  // namespace

/// The axis: after `shutdown()`, a script call comes back.
TEST_F(ShutDownEngineAnswersTest, ExecuteScriptAfterShutdownAnswers) {
    const std::string session = "shutdown_answers_execute";
    ASSERT_TRUE(engine_->createSession(session, ""));

    engine_->shutdown();

    auto future = engine_->executeScript(session, "1 + 1;");
    auto result = answer_within(future);

    ASSERT_TRUE(result.has_value()) << "executeScript() on a shut-down engine never answered. The worker thread was "
                                       "joined by shutdown(), so the operation sat on a queue nothing drains and the "
                                       "promise was never satisfied — a caller waiting with .get() hangs forever, with "
                                       "no log line and no error. That is the 180-second ctest timeout of 2026-08-19";
    EXPECT_FALSE(result->isSuccess()) << "a shut-down engine must refuse the work, not report success";
}

/// The sibling entry points reach the same executor, so they must answer too.
/// Checked one by one because each is its own one-line forwarder — the kind of
/// list a later edit adds to without noticing this file.
TEST_F(ShutDownEngineAnswersTest, EverySiblingEntryPointAnswersAfterShutdown) {
    const std::string session = "shutdown_answers_siblings";
    ASSERT_TRUE(engine_->createSession(session, ""));

    engine_->shutdown();

    auto evaluate = engine_->evaluateExpression(session, "1 + 1");
    auto evaluated = answer_within(evaluate);
    ASSERT_TRUE(evaluated.has_value()) << "evaluateExpression() on a shut-down engine never answered";
    EXPECT_FALSE(evaluated->isSuccess());

    auto validate = engine_->validateExpression(session, "1 + 1");
    auto validated = answer_within(validate);
    ASSERT_TRUE(validated.has_value()) << "validateExpression() on a shut-down engine never answered";
    EXPECT_FALSE(validated->isSuccess());

    auto assign = engine_->setVariable(session, "x", ScriptValue{static_cast<int64_t>(1)});
    auto assigned = answer_within(assign);
    ASSERT_TRUE(assigned.has_value()) << "setVariable() on a shut-down engine never answered";
    EXPECT_FALSE(assigned->isSuccess());
}

/// The refusal must not become the answer to everything: an engine brought
/// back by `reset()` executes again.
///
/// This is the half that keeps the repair from degenerating into "refuse
/// always", which would pass the cases above while breaking every real caller.
TEST_F(ShutDownEngineAnswersTest, AnEngineResetAfterShutdownExecutesAgain) {
    const std::string session = "shutdown_answers_recovers";

    engine_->shutdown();
    engine_->reset();

    ASSERT_TRUE(engine_->createSession(session, "")) << "reset() must leave the engine able to open a session";

    auto future = engine_->executeScript(session, "var recovered = 41 + 1; recovered;");
    auto result = answer_within(future);

    ASSERT_TRUE(result.has_value()) << "executeScript() after reset() never answered";
    EXPECT_TRUE(result->isSuccess()) << "a reset engine must execute, not keep refusing: " << result->getErrorMessage();
}

}  // namespace SCE::Tests
