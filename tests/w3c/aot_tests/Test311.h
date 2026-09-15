// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "SimpleAotTest.h"
#include "test311_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief error.execution for invalid assignment location
 *
 * Tests that assignment to a non-existent (empty) location yields an
 * error.execution event. The test uses <assign location="" expr="1"/>
 * to trigger the error condition.
 *
 * Expected behavior:
 * - Enter s0 state
 * - Attempt assign to empty location ""
 * - Raise error.execution event
 * - Transition to pass state
 */
struct Test311 : public SimpleAotTest<Test311, 311> {
    using SM = SCE::Generated::test311::test311;
};

// Auto-register
inline static AotTestRegistrar<Test311> registrar_Test311;

}  // namespace SCE::W3C::AotTests
