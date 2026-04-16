// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test172_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: send eventexpr evaluates datamodel at execution time
 */
struct Test172 : public SimpleAotTest<Test172, 172> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 6.2: send eventexpr evaluates datamodel at execution time";
    using SM = SCE::Generated::test172::test172;
};

// Auto-register
inline static AotTestRegistrar<Test172> registrar_Test172;

}  // namespace SCE::W3C::AotTests
