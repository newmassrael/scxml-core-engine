// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "SimpleAotTest.h"
#include "test348_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: Send event parameter sets event name
 *
 * Tests that the event parameter in <send> correctly sets the name of the event
 * being sent. The event name specified in the event attribute should be used
 * as-is when dispatching the event to the target.
 */
struct Test348 : public SimpleAotTest<Test348, 348> {
    static constexpr const char *DESCRIPTION = "Send event parameter sets event name (W3C 6.2 AOT)";
    using SM = SCE::Generated::test348::test348;
};

// Auto-register
inline static AotTestRegistrar<Test348> registrar_Test348;

}  // namespace SCE::W3C::AotTests
