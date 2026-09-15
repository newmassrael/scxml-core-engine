// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "SimpleAotTest.h"
#include "test487_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Illegal assignment error.execution event
 *
 * Tests that attempting to access a property on undefined raises error.execution event.
 * The test initializes with an assignment expression that tries to access
 * undefined.invalidProperty, which should trigger error.execution.
 *
 * Expected behavior:
 * - Assignment expression: Var1 = undefined.invalidProperty
 * - Runtime evaluation via JSEngine detects illegal access
 * - error.execution event raised and caught by transition
 * - Transitions to pass state upon receiving error.execution
 *
 * Uses Static Hybrid approach: static state machine structure with
 * runtime ECMAScript expression evaluation via JSEngine.
 */
struct Test487 : public SimpleAotTest<Test487, 487> {
    using SM = SCE::Generated::test487::test487;
};

// Auto-register
inline static AotTestRegistrar<Test487> registrar_Test487;

}  // namespace SCE::W3C::AotTests
