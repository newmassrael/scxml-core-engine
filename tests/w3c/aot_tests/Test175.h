// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ScheduledAotTest.h"
#include "test175_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: Send delayexpr uses current datamodel value
 *
 * Tests delayed send with expression-based delay calculation.
 * Requires event scheduler polling.
 */
struct Test175 : public ScheduledAotTest<Test175, 175> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: delayexpr evaluates current datamodel value at send execution time";
    using SM = SCE::Generated::test175::test175;
};

// Auto-register
inline static AotTestRegistrar<Test175> registrar_Test175;

}  // namespace SCE::W3C::AotTests
