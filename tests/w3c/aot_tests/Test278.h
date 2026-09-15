// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test278_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Global datamodel scope
 *
 * Tests that variables defined in state-level datamodel are globally accessible.
 * Variable Var1 defined in state s1's datamodel should be accessible from state s0.
 */
struct Test278 : public SimpleAotTest<Test278, 278> {
    using SM = SCE::Generated::test278::test278;
};

// Auto-register
inline static AotTestRegistrar<Test278> registrar_Test278;

}  // namespace SCE::W3C::AotTests
