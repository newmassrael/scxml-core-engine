// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "AotTestBase.h"
#include "AotTestRegistry.h"
#include "core/LogMacros.h"
#include "scripting/ScriptEngineProvider.h"
#include <memory>
#include <thread>

namespace SCE::W3C::AotTests {

/**
 * @brief CRTP template for simple AOT tests
 *
 * Simplifies test creation for most common pattern:
 * 1. Create state machine
 * 2. Initialize
 * 3. Check final state is Pass
 *
 * Usage:
 * @code
 * #include "test144_sm.h"
 * struct Test144 : public SimpleAotTest<Test144, 144> {
 *     static constexpr const char* DESCRIPTION = "Event queue ordering";
 *     using SM = SCE::Generated::test144::test144;
 * };
 * REGISTER_AOT_TEST(Test144);
 * @endcode
 */
template <typename Derived, int TestNum> class SimpleAotTest : public AotTestBase {
public:
    static constexpr int TEST_ID = TestNum;

    bool run() override {
        using SM = typename Derived::SM;
        SM sm;
        // W3C SCXML B.1: Inject script engine handle (Path B+ Q1=(d) C++=shared_ptr).
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // ScriptEngineProvider singleton, shared_ptr is a non-owning view.
        if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
            sm.setScriptEngine(::std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                       [](::SCE::IScriptEngine *) {}));
        }
        sm.initialize();
        auto finalState = sm.getCurrentState();
        bool isFinished = sm.isInFinalState();

        // W3C SCXML: Check success state (default: Pass, override with PASS_STATE)
        // Policy-based design: Derived class can define PASS_STATE for custom success states
        bool isPass;
        if constexpr (requires { Derived::PASS_STATE; }) {
            // Manual test or custom success state
            isPass = (finalState == Derived::PASS_STATE);
        } else {
            // Standard test: success = Pass state
            isPass = (finalState == SM::State::Pass);
        }

        SCE_LOG_DEBUG("AOT Test {}: isInFinalState={}, currentState={}, isPass={}", TEST_ID, isFinished,
                      static_cast<int>(finalState), isPass);
        return isFinished && isPass;
    }

    int getTestId() const override {
        return TEST_ID;
    }

    const char *getDescription() const override {
        // Lazy load description from metadata.txt (Single Source of Truth)
        // Cached to avoid repeated file I/O
        if (cachedDescription_.empty()) {
            cachedDescription_ = AotTestBase::loadMetadataDescription(TEST_ID);
        }
        return cachedDescription_.c_str();
    }

private:
    mutable std::string cachedDescription_;

    /**
     * @brief Get test type: pure_static or static_hybrid
     *
     * Uses Policy::NEEDS_SCRIPT_ENGINE to determine if test uses JSEngine
     * for ECMAScript expression evaluation (In(), typeof, _event, etc.)
     */
    const char *getTestType() const override {
        using SM = typename Derived::SM;
        using Policy = typename SM::PolicyType;
        return Policy::NEEDS_SCRIPT_ENGINE ? "static_hybrid" : "pure_static";
    }
};

}  // namespace SCE::W3C::AotTests
