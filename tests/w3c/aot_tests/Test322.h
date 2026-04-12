// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "SimpleAotTest.h"
#include "test322_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML B.2.1: _sessionid system variable immutability
 *
 * Tests that the _sessionid system variable cannot be modified after initialization.
 * The test attempts to assign a new value to _sessionid and verifies that the original
 * value remains unchanged, ensuring the immutability of this system variable.
 *
 * W3C SCXML B.2.1: The '_sessionid' system variable is immutable and must not be
 * modified by the state machine. Any attempt to change its value should be ignored.
 */
struct Test322 : public SimpleAotTest<Test322, 322> {
    static constexpr const char *DESCRIPTION = "_sessionid immutability (W3C B.2.1 AOT)";
    using SM = SCE::Generated::test322::test322;
};

// Auto-register
inline static AotTestRegistrar<Test322> registrar_Test322;

}  // namespace SCE::W3C::AotTests
