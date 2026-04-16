// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test155_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach sums array items into variable (AOT JSEngine)
 */
struct Test155 : public SimpleAotTest<Test155, 155> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.6: foreach executes content once per item";
    using SM = SCE::Generated::test155::test155;
};

// Auto-register
inline static AotTestRegistrar<Test155> registrar_Test155;

}  // namespace SCE::W3C::AotTests
